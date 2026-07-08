#include "emufile.h"
#include "version.h"
#include "types.h"
#include "utils/endian.h"
#include "palette.h"
#include "input.h"
#include "fceu.h"
#include "netplay.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "driver_callbacks.h"
#include "state.h"
#include "file.h"
#include "video.h"
#include "movie.h"
#include "cart.h"
#include "fds.h"
#include "vsuni.h"
#include "rust/fceux11_rust.h"
#ifdef _S9XLUA_H
#include "fceulua.h"
#endif
#include "utils/guid.h"
#include "utils/memory.h"
#include "utils/xstring.h"
#include "utils/safe_string.h"
#include <sstream>
#include <algorithm>

#ifdef CREATE_AVI
#endif

#ifdef WIN32
#include <windows.h>
#endif

// v1.13 Phase B / Batch B: Removed the `#include "./drivers/Qt/TasEditor/
// TasEditorWindow.h"` drag. The TAS Editor bridge is now wired through
// function-pointer registration (see `fceu11::RegisterTasBridge` in
// movie.h). The TasEditor GUI registers itself in its constructor and
// unregisters in its destructor / closeEvent.

extern int RAMInitOption;
extern int RAMInitSeed;

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cassert>
#include <iomanip>
#include <fstream>
#include <climits>
#include <cstdarg>
#include <zlib.h>

// v1.2 Census §2.3: `using namespace std` removed; std types below are
// explicitly qualified with std::.

#define MOVIE_VERSION           3

extern char FileBase[];
extern bool AutoSS;		//Declared in fceu.cpp, keeps track if a auto-savestate has been made

std::vector<int> subtitleFrames;		//Frame numbers for subtitle messages
std::vector<std::string> subtitleMessages;	//Messages of subtitles

bool subtitlesOnAVI = false;
bool autoMovieBackup = false; //Toggle that determines if movies should be backed up automatically before altering them
bool freshMovie = false;	  //True when a movie loads, false when movie is altered.  Used to determine if a movie has been altered since opening
bool movieFromPoweron = true;

// v1.13 Phase B / Batch B: TAS Editor bridge (function-pointer based,
// set via fceu11::RegisterTasBridge from the TasEditorWindow GUI on
// construction). When `is_recording` is null the movie core treats the
// frame as a normal joystick log; when set, the per-frame dispatcher
// in FCEUMOV_AddInputState routes into TAS via `record_input` instead
// of writing the joy log itself.
namespace {
fceu11::TasBridge g_tas_bridge{nullptr, nullptr, nullptr};
} // anonymous namespace

void fceu11::RegisterTasBridge(const fceu11::TasBridge& b)
{
	g_tas_bridge = b;
}

void fceu11::UnregisterTasBridge()
{
	g_tas_bridge = fceu11::TasBridge{nullptr, nullptr, nullptr};
}

static int _currCommand = 0;

// Function declarations------------------------


//TODO - remove the synchack stuff from the replay gui and require it to be put into the fm2 file
//which the user would have already converted from fcm
//also cleanup the whole emulator version bullshit in replay. we dont support that old stuff anymore

//todo - better error handling for the compressed savestate

//todo - consider a MemoryBackedFile class..
//..a lot of work.. instead lets just read back from the current fcm

//todo - could we, given a field size, over-read from an inputstream and then parse out an integer?
//that would be faster than several reads, perhaps.

//sometimes we accidentally produce movie stop signals while we're trying to do other things with movies..
bool suppressMovieStop=false;

//----movie engine main state
EMOVIEMODE movieMode = MOVIEMODE_INACTIVE;

//this should not be set unless we are in MOVIEMODE_RECORD!
//FILE* fpRecordingMovie = 0;
EMUFILE* osRecordingMovie = NULL;

int currFrameCounter;
uint32 cur_input_display = 0;
int pauseframe = -1;
bool movie_readonly = true;
int input_display = 0;
int frame_display = 0;
int rerecord_display = 0;
bool fullSaveStateLoads = false;	//Option for loading a savestates full contents in read+write mode instead of up to the frame count in the savestate (useful as a recovery option)
int movieRecordMode = 0;			//Option for various movie recording modes such as TRUNCATE (normal), OVERWRITE etc.

// v1.13 Phase B / Batch C: SFORMAT FCEUMOV_STATEINFO[] moved to
// movie_playback.cpp where the rest of the savestate plugin lives
// (chunk 6 / 7 dispatcher, FCEUMOV_WriteState / ReadState / PreLoad /
// PostLoad, CheckTimelines). The state.cpp:118 `extern` resolves to the
// new TU at link time with no source change required.

std::string curMovieFilename;
MovieData currMovieData;
MovieData defaultMovieData;
int currRerecordCount; // Keep the global value

char lagcounterbuf[32] = {0};

void MovieData::clearRecordRange(int start, int len)
{
	for(int i=0;i<len;i++)
	{
		records[i+start].clear();
	}
}

void MovieData::eraseRecords(int at, int frames)
{
	if (at < (int)records.size())
	{
		if (frames == 1)
		{
			// erase 1 frame
			records.erase(records.begin() + at);
		} else
		{
			// erase many frames
			if (at + frames > (int)records.size())
				frames = (int)records.size() - at;
			records.erase(records.begin() + at, records.begin() + (at + frames));
		}
	}
}

void MovieData::insertEmpty(int at, int frames)
{
	if (at == -1)
	{
		records.resize(records.size() + frames);
	} else
	{
		records.insert(records.begin() + at, frames, MovieRecord());
	}
}

void MovieData::cloneRegion(int at, int frames)
{
	if (at < 0) return;

	records.insert(records.begin() + at, frames, MovieRecord());

	for(int i = 0; i < frames; i++)
		records[i + at].Clone(records[i + at + frames]);
}
// ----------------------------------------------------------------------------
MovieRecord::MovieRecord()
{
	commands = 0;
	*(uint32*)&joysticks = 0;
	memset(zappers, 0, sizeof(zappers));
}

void MovieRecord::clear()
{
	commands = 0;
	*(uint32*)&joysticks = 0;
	memset(zappers, 0, sizeof(zappers));
}

bool MovieRecord::Compare(MovieRecord& compareRec)
{
	//Joysticks, Zappers, and commands

	if (this->commands != compareRec.commands)
		return false;
	if ((*(uint32*)&(this->joysticks)) != (*(uint32*)&(compareRec.joysticks)))
		return false;
	if (memcmp(this->zappers, compareRec.zappers, sizeof(zappers)))
		return false;

	return true;
}
void MovieRecord::Clone(MovieRecord& sourceRec)
{
	*(uint32*)&joysticks = *(uint32*)(&(sourceRec.joysticks));
	memcpy(this->zappers, sourceRec.zappers, sizeof(zappers));
	this->commands = sourceRec.commands;
}

// MovieRecord::mnemonics + dumpJoy/parseJoy/parse/parseBinary/dumpBinary/dump,
// MovieData::truncateAt, installValue, dump, and LoadFM2 moved to
// src/movie_fm2.cpp (Phase F-A, v1.12 Scissors).


MovieData::MovieData()
	: version(MOVIE_VERSION)
	, emuVersion(FCEU_VERSION_NUMERIC)
	, fds(false)
	, palFlag(false)
	, PPUflag(false)
	, rerecordCount(0)
	, binaryFlag(false)
	, loadFrameCount(-1)
	, fourscore(false)
	, microphone(false)
	, RAMInitOption(0)
	, RAMInitSeed(0)
{
	memset(&romChecksum,0,sizeof(MD5DATA));
}


int FCEUMOV_GetFrame(void)
{
	return currFrameCounter;
}

int FCEUI_GetLagCount(void)
{
	return lagCounter;
}

bool FCEUI_GetLagged(void)
{
	if (lagFlag)
		return true;
	else
		return false;
}
void FCEUI_SetLagFlag(bool value)
{
	lagFlag = (value) ? 1 : 0;
}

bool FCEUMOV_ShouldPause(void)
{
	if(pauseframe && currFrameCounter+1 == pauseframe)
	{
		pauseframe = 0;
		return true;
	}
	else
	{
		return false;
	}
}

EMOVIEMODE FCEUMOV_Mode()
{
	return movieMode;
}

bool FCEUMOV_Mode(EMOVIEMODE modemask)
{
	return (movieMode&modemask)!=0;
}

bool FCEUMOV_Mode(int modemask)
{
	return FCEUMOV_Mode((EMOVIEMODE)modemask);
}

const char *GetMovieModeStr()
{
	if (movieMode == MOVIEMODE_INACTIVE)
		return " (no movie)";
	else if (movieMode == MOVIEMODE_PLAY)
		return " (playing)";
	else if (movieMode == MOVIEMODE_RECORD)
		return " (recording)";
	else if (movieMode == MOVIEMODE_FINISHED)
		return " (finished)";
	else if (movieMode == MOVIEMODE_TASEDITOR)
		return " (taseditor)";
	else
		return ".";
}

static const char *GetMovieReadOnlyStr()
{
	if (movieMode == MOVIEMODE_RECORD)
		return movie_readonly ? " R-O" : "";
	else
		return movie_readonly ? "" : " R+W";
}

static const char *GetMovieRecordModeStr()
{
	switch (movieRecordMode)
	{
	case MOVIE_RECORD_MODE_OVERWRITE:
		return " [W]";
	case MOVIE_RECORD_MODE_INSERT:
		return " [I]";
	default:
		return "";
	}
}

static EMUFILE *openRecordingMovie(const char* fname)
{
	if (osRecordingMovie)
		delete osRecordingMovie;

	osRecordingMovie = FCEUD_UTF8_fstream(fname, "wb");
	if (!osRecordingMovie || osRecordingMovie->fail()) {
		FCEU_PrintError("Error opening movie output file: %s", fname);
		return NULL;
	}
	if ( fname != curMovieFilename.c_str() )
	{
		curMovieFilename.assign(fname);
	}

	return osRecordingMovie;
}

void closeRecordingMovie()
{
	if (osRecordingMovie)
	{
		delete osRecordingMovie;
		osRecordingMovie = 0;
	}
}

// Callers shall set the approriate movieMode before calling this
void RedumpWholeMovieFile(bool justToggledRecording = false)
{
	bool recording = (movieMode == MOVIEMODE_RECORD);
	assert((NULL != osRecordingMovie) == (recording != justToggledRecording) && "osRecordingMovie should be consistent with movie mode!");

	if (NULL == openRecordingMovie(curMovieFilename.c_str()))
		return;

	currMovieData.dump(osRecordingMovie, false/*currMovieData.binaryFlag*/, recording);
	if (recording)
		osRecordingMovie->fflush();
	else
		closeRecordingMovie();
}

/// Stop movie playback.
static void StopPlayback()
{
	assert(movieMode != MOVIEMODE_RECORD && NULL == osRecordingMovie);

	movieMode = MOVIEMODE_INACTIVE;
	FCEU_DispMessageOnMovie("Movie playback stopped.");
}

// Stop movie playback without closing the movie.
void FinishPlayback()
{
	assert(movieMode != MOVIEMODE_RECORD);

	extern int closeFinishedMovie;
	if (closeFinishedMovie)
		StopPlayback();
	else
	{
		movieMode = MOVIEMODE_FINISHED;
		FCEU_DispMessage("Movie finished playing.",0);
	}
}

/// Stop movie recording
static void StopRecording()
{
	assert(movieMode == MOVIEMODE_RECORD);

	movieMode = MOVIEMODE_INACTIVE;
	RedumpWholeMovieFile(true);
	FCEU_DispMessage("Movie recording stopped.",0);
}

void OnMovieClosed()
{
	assert(movieMode == MOVIEMODE_INACTIVE);

	curMovieFilename.clear();			//No longer a current movie filename
	freshMovie = false;					//No longer a fresh movie loaded
	if (bindSavestate) AutoSS = false;	//If bind movies to savestates is true, then there is no longer a valid auto-save to load

	if (auto* fn = fceu11::g_driver().set_main_window_text) fn(nullptr);
}

bool bogorf;

void fceu11::StopMovie()
{
	if (suppressMovieStop)
		return;

	if (movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_FINISHED)
		StopPlayback();
	else if (movieMode == MOVIEMODE_RECORD)
		StopRecording();

	OnMovieClosed();
}

void poweron(bool shouldDisableBatteryLoading)
{
	//// make a for-movie-recording power-on clear the game's save data, too
	//extern char lastLoadedGameName [2048];
	//extern int disableBatteryLoading, suppressAddPowerCommand;
	//suppressAddPowerCommand=1;
	//if(shouldDisableBatteryLoading) disableBatteryLoading=1;
	//suppressMovieStop=true;
	//{
	//	//we need to save the pause state through this process
	//	int oldPaused = EmulationPaused;

	//	// NOTE:  this will NOT write an FCEUNPCMD_POWER into the movie file
	//	FCEUGI* gi = fceu11::LoadGame(lastLoadedGameName, 0);
	//	assert(gi);
	//	PowerNES();

	//	EmulationPaused = oldPaused;
	//}
	//suppressMovieStop=false;
	//if(shouldDisableBatteryLoading) disableBatteryLoading=0;
	//suppressAddPowerCommand=0;

	extern int disableBatteryLoading;
	if(!bogorf) disableBatteryLoading = 1;
	PowerNES();
	if(!bogorf) disableBatteryLoading = 0;
}

void FCEUMOV_CreateCleanMovie()
{
	currMovieData = MovieData();
	currMovieData.palFlag = fceu11::GetCurrentVidSystem(0,0)!=0;
	currMovieData.romFilename = FileBase;
	if ( GameInfo )
	{
		currMovieData.romChecksum = GameInfo->MD5;
	}
	currMovieData.guid.newGuid();
	currMovieData.fourscore = fceu11::GetInputFourscore();
	currMovieData.microphone = fceu11::GetInputMicrophone();
	// v0.3.8: currMovieData.ports[] is `int[3]`; .type is the typed
	// fceu11::InputDevice / InputDeviceFC form �?cast to int.
	currMovieData.ports[0] = static_cast<int>(joyports[0].type);
	currMovieData.ports[1] = static_cast<int>(joyports[1].type);
	currMovieData.ports[2] = static_cast<int>(portFC.type);
	currMovieData.fds = isFDS;
	currMovieData.PPUflag = (newppu != 0);
	currMovieData.RAMInitOption = RAMInitOption;
	currMovieData.RAMInitSeed = RAMInitSeed;
}
void FCEUMOV_ClearCommands()
{
	_currCommand = 0;
}

bool FCEUMOV_FromPoweron()
{
	return movieFromPoweron;
}

// v1.13 Phase B / Batch C: MovieData::loadSavestateFrom + dumpSavestateTo
// bodies moved to movie_playback.cpp alongside the rest of the savestate
// plugin. Declared as static members in movie.h; the body-only relocation
// is C++-legal and matches the v1.12 cross-TU static-style pattern
// documented at movie_record.cpp:39-46.

bool MovieData::loadSaveramFrom(std::vector<uint8>* buf)
{
	// v0.3.10: boundary conversion at the std::vector<u8> -> std::vector<std::byte>.
	std::vector<std::byte> tmp(reinterpret_cast<const std::byte*>(buf->data()),
	                            reinterpret_cast<const std::byte*>(buf->data()) + buf->size());
	EMUFILE_MEMORY ms(&tmp);

	bool hasBattery = !!ms.read32le();
	if(hasBattery != !!currCartInfo->battery)
	{
		FCEU_PrintError("movie battery load mismatch 1");
		return false;
	}

	for (size_t i=0;i<currCartInfo->SaveGame.size();i++)
	{
		int len = ms.read32le();

		if( (currCartInfo->SaveGame[i].bufptr == nullptr) && (len!=0) )
		{
			FCEU_PrintError("movie battery load mismatch 2");
			return false;
		}

		if(currCartInfo->SaveGame[i].buflen != static_cast<size_t>(len))
		{
			FCEU_PrintError("movie battery load mismatch 3");
			return false;
		}

		ms.fread(std::span<std::byte>(reinterpret_cast<std::byte*>(currCartInfo->SaveGame[i].bufptr), len));
	}

	return true;
}

void MovieData::dumpSaveramTo(std::vector<uint8>* buf, int compressionLevel)
{
	// v0.3.10: write into std::vector<std::byte>, flush back at the end.
	std::vector<std::byte> tmp;
	EMUFILE_MEMORY ms(&tmp);

	ms.write32le(currCartInfo->battery?1:0);
	for(size_t i=0;i<currCartInfo->SaveGame.size();i++)
	{
		if (!currCartInfo->SaveGame[i].bufptr)
		{
			ms.write32le((u32)0);
			continue;
		}
		ms.write32le( static_cast<uint32>(currCartInfo->SaveGame[i].buflen) );
		ms.fwrite(std::span<const std::byte>(reinterpret_cast<const std::byte*>(currCartInfo->SaveGame[i].bufptr), currCartInfo->SaveGame[i].buflen));
	}

	buf->assign(reinterpret_cast<const uint8_t*>(tmp.data()),
	            reinterpret_cast<const uint8_t*>(tmp.data()) + tmp.size());
}


//begin playing an existing movie
bool fceu11::LoadMovie(const char *fname, bool _read_only, int _pauseframe)
{
	if(!FCEU_IsValidUI(FCEUI_PLAYMOVIE))
		return true;	//adelikat: file did not fail to load, so let's return true here, just do nothing

	assert(fname);

	//mbg 6/10/08 - we used to call StopMovie here, but that cleared curMovieFilename and gave us crashes...
	if(movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_FINISHED)
		StopPlayback();
	else if(movieMode == MOVIEMODE_RECORD)
		StopRecording();
	//--------------

	currMovieData = MovieData();

	curMovieFilename.assign(fname);
	FCEUFILE *fp = FCEU_fopen(fname,0,"rb",0);
	if (!fp) return false;
	if(fp->isArchive() && !_read_only) {
		FCEU_PrintError("Cannot open a movie in read+write from an archive.");
		return true;	//adelikat: file did not fail to load, so return true (false is only for file not exist/unable to open errors
	}

	LoadFM2(currMovieData, fp->stream, fp->size, false);
	LoadSubtitles(currMovieData);
	delete fp;

	RAMInitOption = currMovieData.RAMInitOption;
	RAMInitSeed = currMovieData.RAMInitSeed;

	freshMovie = true;	//Movie has been loaded, so it must be unaltered
	if (bindSavestate) AutoSS = false;	//If bind savestate to movie is true, then their isn't a valid auto-save to load, so flag it
	cur_input_display = 0; //clear previous input display
	//fully reload the game to reinitialize everything before playing any movie
	poweron(true);

	if(currMovieData.savestate.size())
	{
		//WE NEED TO LOAD A SAVESTATE
		movieFromPoweron = false;
		bool success = MovieData::loadSavestateFrom(&currMovieData.savestate);
		if(!success) return true;	//adelikat: I guess return true here?  False is only for a bad movie filename, if it got this far the file was good?
	}
	else if(currMovieData.saveram.size())
	{
		movieFromPoweron = true;
		bool success = MovieData::loadSaveramFrom(&currMovieData.saveram);
		if(!success) return true;	//adelikat: I guess return true here?  False is only for a bad movie filename, if it got this far the file was good?
	}
	else {
		movieFromPoweron = true;
	}

	//if there is no savestate, we won't have this crucial piece of information at the start of the movie.
	//so, we have to include it with the movie
	if(currMovieData.palFlag)
		fceu11::SetVidSystem(1);
	else
		fceu11::SetVidSystem(0);



	//force the input configuration stored in the movie to apply
	FCEUD_SetInput(currMovieData.fourscore, currMovieData.microphone, (ESI)currMovieData.ports[0], (ESI)currMovieData.ports[1], (ESIFC)currMovieData.ports[2]);

	//stuff that should only happen when we're ready to positively commit to the replay
	currFrameCounter = 0;
	pauseframe = _pauseframe;
	movie_readonly = _read_only;
	movieMode = MOVIEMODE_PLAY;
	if (movieMode != MOVIEMODE_TASEDITOR)
		currRerecordCount = currMovieData.rerecordCount;

	if(movie_readonly)
		FCEU_DispMessage("Replay started Read-Only.",0);
	else
		FCEU_DispMessage("Replay started Read+Write.",0);

	if (auto* fn = fceu11::g_driver().set_main_window_text) fn(nullptr);

	#ifdef CREATE_AVI
	if(LoggingEnabled)
	{
	    FCEU_DispMessage("Video recording enabled.\n",0);
	    LoggingEnabled = 2;
	}
	#endif

	return true;
}


//begin recording a new movie
//TODO - BUG - the record-from-another-savestate doesnt work.
void fceu11::SaveMovie(const char *fname, EMOVIE_FLAG flags, std::wstring author)
{
	if(!FCEU_IsValidUI(FCEUI_RECORDMOVIE))
		return;

	assert(fname);

	fceu11::StopMovie();

	if (NULL == openRecordingMovie(fname))
		return;

	currFrameCounter = 0;
	LagCounterReset();
	FCEUMOV_CreateCleanMovie();
	if(author != L"") currMovieData.comments.push_back(L"author " + author);

	if(flags & MOVIE_FLAG_FROM_POWERON)
	{
		movieFromPoweron = true;
		poweron(true);
	}
	else if(flags & MOVIE_FLAG_FROM_SAVERAM)
	{
		movieFromPoweron = true;
		MovieData::dumpSaveramTo(&currMovieData.saveram,Z_NO_COMPRESSION); //i guess with this there's a chance someone could hack the file, at least, so maybe it's helpfu
		bogorf = true;
		poweron(false);
		bogorf = false;
	}
	else //from savestate
	{
		movieFromPoweron = false;
		MovieData::dumpSavestateTo(&currMovieData.savestate,Z_BEST_COMPRESSION);
	}

	FCEUMOV_ClearCommands();

	//we are going to go ahead and dump the header. from now on we will only be appending frames
	currMovieData.dump(osRecordingMovie, false);

	movieMode = MOVIEMODE_RECORD;
	movie_readonly = false;
	if (movieMode != MOVIEMODE_TASEDITOR)
		currRerecordCount = 0;

	FCEU_DispMessage("Movie recording started.",0);
}


//the main interaction point between the emulator and the movie system.
//either dumps the current joystick state or loads one state from the movie
void FCEUMOV_AddInputState()
{
	if (movieMode == MOVIEMODE_TASEDITOR)
	{
		// if movie length is less or equal to currFrame, pad it with empty frames
		if (((int)currMovieData.records.size() - 1) < (currFrameCounter + 1))
			currMovieData.insertEmpty(-1, (currFrameCounter + 1) - ((int)currMovieData.records.size() - 1));

		MovieRecord* mr = &currMovieData.records[currFrameCounter];
		if (g_tas_bridge.is_recording && g_tas_bridge.is_recording(g_tas_bridge.ctx))
		{
			// record commands and buttons
			mr->commands |= _currCommand;
			joyports[0].log(mr);
			joyports[1].log(mr);
			if (g_tas_bridge.record_input)
				g_tas_bridge.record_input(g_tas_bridge.ctx);
		}
		// replay buttons
		joyports[0].load(mr);
		joyports[1].load(mr);
		// replay commands
		if (mr->command_power())
			PowerNES();
		if (mr->command_reset())
			ResetNES();
		if (mr->command_fds_insert())
			FCEU_FDSInsert();
		if (mr->command_fds_select())
			FCEU_FDSSelect();
		if (mr->command_vs_insertcoin())
			FCEU_VSUniCoin(0);
		if (mr->command_vs_insertcoin2())
			FCEU_VSUniCoin(1);
		if (mr->command_vs_service())
			FCEU_VSUniService();
		_currCommand = 0;
	} else
	if (movieMode == MOVIEMODE_PLAY)
	{
		//stop when we run out of frames
		if (currFrameCounter >= (int)currMovieData.records.size())
		{
			FinishPlayback();
			//tell all drivers to poll input and set up their logical states
			for(int port=0;port<2;port++)
				joyports[port].driver->Update(port,joyports[port].ptr,joyports[port].attrib);
			portFC.driver->Update(portFC.ptr,portFC.attrib);
		} else
		{
			MovieRecord* mr = &currMovieData.records[currFrameCounter];

			//reset and power cycle if necessary
			if(mr->command_power())
				PowerNES();
			if(mr->command_reset())
				ResetNES();
			if(mr->command_fds_insert())
				FCEU_FDSInsert();
			if(mr->command_fds_select())
				FCEU_FDSSelect();
			if (mr->command_vs_insertcoin())
				FCEU_VSUniCoin(0);
			if (mr->command_vs_insertcoin2())
				FCEU_VSUniCoin(1);
			if (mr->command_vs_service())
				FCEU_VSUniService();

			joyports[0].load(mr);
			joyports[1].load(mr);
		}

		//if we are on the last frame, then pause the emulator if the player requested it
		if ( static_cast<size_t>(currFrameCounter) == currMovieData.records.size()-1)
		{
			if(FCEUD_PauseAfterPlayback())
			{
				fceu11::ToggleEmulationPause();
			}
		}

		//pause the movie at a specified frame
		if (FCEUMOV_ShouldPause() && fceu11::IsEmulationPaused()==0)
		{
			fceu11::ToggleEmulationPause();
			FCEU_DispMessage("Paused at specified movie frame",0);
		}

	} else if (movieMode == MOVIEMODE_RECORD)
	{
		MovieRecord mr;

		joyports[0].log(&mr);
		joyports[1].log(&mr);
		mr.commands = _currCommand;
		_currCommand = 0;

		//aquanull: now it supports other recording modes that don't necessarily truncate further frame data
		//If the user chooses it can be delayed to here
		if (currFrameCounter < (int)currMovieData.records.size())
			switch (movieRecordMode)
			{
			case MOVIE_RECORD_MODE_OVERWRITE:
				currMovieData.records[currFrameCounter].Clone(mr);
				break;
			case MOVIE_RECORD_MODE_INSERT:
				//FIXME: this could be very insufficient
				currMovieData.records.insert(currMovieData.records.begin() + currFrameCounter, mr);
				break;
			//case MOVIE_RECORD_MODE_TRUNCATE:
			default:
				//Adelikat: in normal mode, this is done at the time of loading a savestate in read+write mode
				currMovieData.truncateAt(currFrameCounter);
				currMovieData.records.push_back(mr);
				break;
			}
		else
			currMovieData.records.push_back(mr);

		mr.dump(&currMovieData, osRecordingMovie, currFrameCounter);	// to disk
	}

	currFrameCounter++;

	extern uint8 joy[4];
	memcpy(&cur_input_display,joy,4);
}


//TODO
void FCEUMOV_AddCommand(int cmd)
{
	// do nothing if not recording a movie
	if(movieMode != MOVIEMODE_RECORD && movieMode != MOVIEMODE_TASEDITOR)
		return;

	// translate "FCEU NetPlay" command to "FCEU Movie" command
	switch (cmd)
	{
		case FCEUNPCMD_RESET: cmd = MOVIECMD_RESET; break;
		case FCEUNPCMD_POWER: cmd = MOVIECMD_POWER; break;
		case FCEUNPCMD_FDSINSERT: cmd = MOVIECMD_FDS_INSERT; break;
		case FCEUNPCMD_FDSSELECT: cmd = MOVIECMD_FDS_SELECT; break;
		case FCEUNPCMD_VSUNICOIN: cmd = MOVIECMD_VS_INSERTCOIN; break;
		case FCEUNPCMD_VSUNICOIN2: cmd = MOVIECMD_VS_INSERTCOIN2; break;
		case FCEUNPCMD_VSUNISERVICE: cmd = MOVIECMD_VS_SERVICE; break;
		// all other netplay commands (e.g. FCEUNPCMD_VSUNIDIP0) are not supported by movie recorder for now
		default: return;
	}

	_currCommand |= cmd;
}

void FCEU_DrawMovies(uint8 *XBuf)
{
	// not the best place, but just working
	assert((NULL != osRecordingMovie) == (movieMode == MOVIEMODE_RECORD));

	if (frame_display)
	{
		char counterbuf[32] = {0};
		int color = 0x20;
		
		if (movieMode == MOVIEMODE_PLAY)
		{
			snprintf(counterbuf, sizeof(counterbuf), "%d/%d%s%s", currFrameCounter, (int)currMovieData.records.size(), GetMovieRecordModeStr(), GetMovieReadOnlyStr());
		} else if (movieMode == MOVIEMODE_RECORD)
		{
			if (movieRecordMode == MOVIE_RECORD_MODE_TRUNCATE)
				snprintf(counterbuf, sizeof(counterbuf), "%d%s%s (record)", currFrameCounter, GetMovieRecordModeStr(), GetMovieReadOnlyStr()); // nearly classic
			else
				snprintf(counterbuf, sizeof(counterbuf), "%d/%d%s%s (record)", currFrameCounter, (int)currMovieData.records.size(), GetMovieRecordModeStr(), GetMovieReadOnlyStr());
		} else if (movieMode == MOVIEMODE_FINISHED)
		{
			snprintf(counterbuf, sizeof(counterbuf),"%d/%d%s%s (finished)",currFrameCounter,(int)currMovieData.records.size(), GetMovieRecordModeStr(), GetMovieReadOnlyStr());
			color = 0x17; //Show red to get attention
		} else if (movieMode == MOVIEMODE_TASEDITOR)
		{
			snprintf(counterbuf, sizeof(counterbuf),"%d",currFrameCounter);
		} else
			snprintf(counterbuf, sizeof(counterbuf),"%d (no movie)",currFrameCounter);

		if (counterbuf[0])
			DrawTextTrans(ClipSidesOffset+XBuf+FCEU_TextScanlineOffsetFromBottom(30)+1, 256, (uint8*)counterbuf, color+0x80);
	}
	if (rerecord_display && movieMode != MOVIEMODE_INACTIVE)
	{
		char counterbuf[32] = {0};
		snprintf(counterbuf, sizeof(counterbuf), "%d", currMovieData.rerecordCount);

		if (counterbuf[0])
			DrawTextTrans(ClipSidesOffset+XBuf+FCEU_TextScanlineOffsetFromBottom(50)+1, 256, (uint8*)counterbuf, 0x28+0x80);
	}
}

void FCEU_DrawLagCounter(uint8 *XBuf)
{
	if (lagCounterDisplay)
	{
		// If currently lagging - display red, else display green
		uint8 color = (lagFlag) ? (0x16+0x80) : (0x2A+0x80);
		snprintf(lagcounterbuf, sizeof(lagcounterbuf), "%d", lagCounter);
		if(lagcounterbuf[0])
			DrawTextTrans(ClipSidesOffset + XBuf + FCEU_TextScanlineOffsetFromBottom(40) + 1, 256, (uint8*)lagcounterbuf, color);
	}
}

// FCEUMOV_WriteState + CheckTimelines + FCEUMOV_ReadState +
// FCEUMOV_PreLoad + FCEUMOV_PostLoad moved to src/movie_playback.cpp
// (Phase F-B, v1.12 Scissors).


void FCEUMOV_IncrementRerecordCount()
{
#ifdef _S9XLUA_H
	if(!FCEU_LuaRerecordCountSkip())
		if (movieMode != MOVIEMODE_TASEDITOR)
			currRerecordCount++;
		else
			currMovieData.rerecordCount++;
#else
	if (movieMode != MOVIEMODE_TASEDITOR)
		currRerecordCount++;
	else
		currMovieData.rerecordCount++;
#endif
	if (movieMode != MOVIEMODE_TASEDITOR)
		currMovieData.rerecordCount = currRerecordCount;
}

void fceu11::MovieToggleFrameDisplay(void)
{
	frame_display=!frame_display;
}

void FCEUI_MovieToggleRerecordDisplay()
{
	rerecord_display ^= 1;
}

void FCEUI_ToggleInputDisplay(void)
{
	switch(input_display)
	{
	case 0:
		input_display = 1;
		break;
	case 1:
		input_display = 2;
		break;
	case 2:
		input_display = 4;
		break;
	default:
		input_display = 0;
		break;
	}
}

int FCEUI_GetMovieLength()
{
	return currMovieData.records.size();
}

int FCEUI_GetMovieRerecordCount()
{
	return currMovieData.rerecordCount;
}

bool fceu11::GetMovieToggleReadOnly()
{
	return movie_readonly;
}

void fceu11::SetMovieToggleReadOnly(bool which)
{
	if (which)	//If set to readonly
	{
		if (!movie_readonly)	//If not already set
		{
			movie_readonly = true;
			FCEU_DispMessage("Movie is now Read-Only.",0);
		}
		else					//Else restate message
			FCEU_DispMessage("Movie is Read-Only.",0);
	}
	else		//If set to read+write
	{
		if (movie_readonly)		//If not already set
		{
			movie_readonly = false;
			FCEU_DispMessage("Movie is now Read+Write.",0);
		}
		else					//Else restate message
			FCEU_DispMessage("Movie is Read+Write.",0);
	}
}

//auqnull: What's the point to toggle Read-Only without a movie loaded?
void fceu11::MovieToggleReadOnly()
{
	char message[260];

	movie_readonly = !movie_readonly;
	if (movie_readonly)
		FCEU_strlcpy(message, sizeof(message), "Movie is now Read-Only");
	else
		FCEU_strlcpy(message, sizeof(message), "Movie is now Read+Write");
	
	safe_strcat(message, sizeof(message), GetMovieModeStr());
	FCEU_DispMessage("%s",0,message);
}

// FCEUI_MovieToggleRecording / InsertFrame / DeleteFrame / Truncate /
// NextRecordMode / PrevRecordMode / RecordMode{Truncate,Overwrite,Insert}
// moved to src/movie_record.cpp (Phase F-C, v1.12 Scissors).


void fceu11::MoviePlayFromBeginning(void)
{
	if (movieMode == MOVIEMODE_TASEDITOR)
	{
	} else if (movieMode != MOVIEMODE_INACTIVE)
	{
		if (movieMode == MOVIEMODE_RECORD)
		{
			movieMode = MOVIEMODE_PLAY;
			RedumpWholeMovieFile(true);
		}
		if (currMovieData.savestate.empty())
		{
			movie_readonly = true;
			movieMode = MOVIEMODE_PLAY;
			cur_input_display = 0; //clear previous input display
			poweron(true);
			currFrameCounter = 0;
			FCEU_DispMessage("Movie is now Read-Only. Playing from beginning.",0);
		}
		else
		{
			// movie starting from savestate - reload movie file
			std::string str = curMovieFilename;
			fceu11::StopMovie();
			if (fceu11::LoadMovie(str.c_str(), 1, 0))
			{
				movieMode = MOVIEMODE_PLAY;
				movie_readonly = true;
				FCEU_DispMessage("Movie is now Read-Only. Playing from beginning.",0);
			}
			//currMovieData.loadSavestateFrom(&currMovieData.savestate); //TODO: make something like this work instead so it doesn't have to reload
		}
	}
	if (auto* fn = fceu11::g_driver().set_main_window_text) fn(nullptr);
}

std::string fceu11::GetMovieName(void)
{
	return curMovieFilename;
}

bool fceu11::MovieGetInfo(FCEUFILE* fp, MOVIE_INFO& info, bool skipFrameCount)
{
	MovieData md;
	if(!LoadFM2(md, fp->stream, fp->size, skipFrameCount))
		return false;

	info.movie_version = md.version;
	info.poweron = md.savestate.size()==0;
	info.reset = false; //Soft-reset isn't used from starting movies anymore, so this will be false, better for FCEUFILE to have that info (as |1| on the first frame indicates it
	info.pal = md.palFlag;
	info.ppuflag = md.PPUflag;
	info.RAMInitOption = md.RAMInitOption;
	info.RAMInitSeed = md.RAMInitSeed;
	info.nosynchack = true;
	info.num_frames = md.records.size();
	info.md5_of_rom_used = md.romChecksum;
	info.emu_version_used = md.emuVersion;
	info.name_of_rom_used = md.romFilename;
	info.rerecord_count = md.rerecordCount;
	info.comments = md.comments;
	info.subtitles = md.subtitles;

	return true;
}

//This function creates an array of frame numbers and corresponding strings for displaying subtitles
void LoadSubtitles(MovieData &moviedata)
{
	subtitleFrames.resize(0);
	subtitleMessages.resize(0);
	extern std::vector<std::string> subtitles;
	for(uint32 i=0; i < moviedata.subtitles.size() ; i++)
	{
		std::string& subtitle = moviedata.subtitles[i];
		size_t splitat = subtitle.find_first_of(' ');
		std::string key, value;

		//If we can't split them, then don't process this one
		if(splitat == std::string::npos)
		{
		}
		//Else split the subtitle into the int and string arrays
		else
		{
			key = subtitle.substr(0,splitat);
			value = subtitle.substr(splitat+1);
			subtitleFrames.push_back(atoi(key.c_str()));
			subtitleMessages.push_back(value);
		}
	}

}

//Every frame, this will be called to determine if a subtitle should be displayed, which one, and then to display it
void ProcessSubtitles(void)
{
	if (movieMode == MOVIEMODE_INACTIVE) return;

	for(uint32 i=0;i<currMovieData.subtitles.size();i++)
	{
		if (currFrameCounter == subtitleFrames[i])
			FCEU_DisplaySubtitles("%s",subtitleMessages[i].c_str());
	}
}

void FCEU_DisplaySubtitles(const char *format, ...)
{
	va_list ap;

	va_start(ap,format);
	vsnprintf(subtitleMessage.errmsg,sizeof(subtitleMessage.errmsg),format,ap);
	va_end(ap);

	subtitleMessage.howlong = 400;
	subtitleMessage.isMovieMessage = subtitlesOnAVI;
	subtitleMessage.linesFromBottom = 0;
}

void FCEUI_CreateMovieFile(std::string fn)
{
	MovieData md = currMovieData;							//Get current movie data
	EMUFILE* outf = FCEUD_UTF8_fstream(fn, "wb");		//open/create file
	md.dump(outf,false);									//dump movie data
	delete outf;											//clean up, delete file object
}

void FCEUI_MakeBackupMovie(bool dispMessage)
{
	//This function generates backup movie files
	std::string currentFn;					//Current movie fillename
	std::string backupFn;					//Target backup filename
	std::string tempFn;						//temp used in back filename creation
	std::stringstream stream;
	int x;								//Temp variable for string manip
	bool exist = false;					//Used to test if filename exists
	bool overflow = false;				//Used for special situation when backup numbering exceeds limit

	currentFn = curMovieFilename;		//Get current moviefilename
	backupFn = curMovieFilename;		//Make backup filename the same as current moviefilename
	x = backupFn.find_last_of(".");		 //Find file extension
	backupFn = backupFn.substr(0,x);	//Remove extension
	tempFn = backupFn;					//Store the filename at this point
	for (unsigned int backNum=0;backNum<999;backNum++) //999 = arbituary limit to backup files
	{
		stream.str("");					 //Clear stream
		if (backNum > 99)
			stream << "-" << backNum;	 //assign backNum to stream
		else if (backNum <=99 && backNum >= 10)
			stream << "-0";				//Make it 010, etc if two digits
		else
			stream << "-00" << backNum;	 //Make it 001, etc if single digit
		backupFn.append(stream.str());	 //add number to bak filename
		backupFn.append(".bak");		 //add extension

		exist = CheckFileExists(backupFn.c_str());	//Check if file exists

		if (!exist)
			break;						//Yeah yeah, I should use a do loop or something
		else
		{
			backupFn = tempFn;			//Before we loop again, reset the filename

			if (backNum == 999)			//If 999 exists, we have overflowed, let's handle that
			{
				backupFn.append("-001.bak"); //We are going to simply overwrite 001.bak
				overflow = true;		//Flag that we have exceeded limit
				break;					//Just in case
			}
		}
	}
	FCEUI_CreateMovieFile(backupFn);

	//TODO, decide if fstream successfully opened the file and print error message if it doesn't

	if (dispMessage)	//If we should inform the user
	{
		if (overflow)
			FCEU_DispMessage("Backup overflow, overwriting %s",0,backupFn.c_str()); //Inform user of overflow
		else
			FCEU_DispMessage("%s created",0,backupFn.c_str()); //Inform user of backup filename
	}
}

