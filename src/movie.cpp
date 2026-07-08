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

// v1.13 Phase B / Batch D-D.2: cross-TU forward decls for session-lifecycle
// helpers now defined in movie_record.cpp. fceu11::StopMovie (below),
// fceu11::SaveMovie (around line 600), fceu11::OnMovieClosed, and
// fceu11::MoviePlayFromBeginning (around line 970) call into these.
void StopPlayback();
void StopRecording();
void FinishPlayback();
void OnMovieClosed();
void RedumpWholeMovieFile(bool justToggledRecording = false);
EMUFILE *openRecordingMovie(const char* fname);
const char* GetMovieModeStr();
const char* GetMovieReadOnlyStr();        // moved to movie_record.cpp in D-D.3
const char* GetMovieRecordModeStr();      // moved to movie_record.cpp in D-D.3

// v1.13 Phase B / Batch D-D.4: per-frame dispatcher helpers — the
// PLAY branch stays in movie.cpp; the TASEDITOR branch lives in
// movie_taseditor_bridge.cpp; the RECORD branch in movie_record.cpp.
void MovieAddInputState_TasEditor();       // movie_taseditor_bridge.cpp
void MovieAddInputState_Record();          // movie_record.cpp

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

// v1.13 Phase B / Batch D-D.1: subtitlesOnAVI + subtitleFrames / subtitleMessages /
// LoadSubtitles / ProcessSubtitles / FCEU_DisplaySubtitles relocated to
// src/movie_subtitles.cpp. The `extern bool subtitlesOnAVI` in movie.h:280
// resolves to the new TU at link time.
bool autoMovieBackup = false; //Toggle that determines if movies should be backed up automatically before altering them
bool freshMovie = false;	  //True when a movie loads, false when movie is altered.  Used to determine if a movie has been altered since opening
bool movieFromPoweron = true;

// v1.13 Phase B / Batch B + D-D.4: TAS Editor bridge (function-pointer
// based, set via fceu11::RegisterTasBridge from the TasEditorWindow GUI
// on construction). g_tas_bridge moved out of anonymous namespace
// (Batch D-D.4) so the TASEDITTOR branch helper MovieAddInputState_TasEditor
// (now in movie_taseditor_bridge.cpp) can read it without re-defining
// the structure / losing the bridge registration.
fceu11::TasBridge g_tas_bridge{nullptr, nullptr, nullptr};

void fceu11::RegisterTasBridge(const fceu11::TasBridge& b)
{
	g_tas_bridge = b;
}

void fceu11::UnregisterTasBridge()
{
	g_tas_bridge = fceu11::TasBridge{nullptr, nullptr, nullptr};
}

// v1.13 Phase B / Batch D-D.5: _currCommand lost its `static` qualifier
// (internal linkage) so that fceu11::SaveMovie (in movie_io.cpp) and
// FCEUMOV_AddCommand (still in movie.cpp until D-D.4) plus
// FCEUMOV_ClearCommands (in movie_io.cpp) can read/write it across TUs
// without a function-call indirection.
int _currCommand = 0;

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

// v1.13 Phase B / Batch D-D.3: lagcounterbuf relocated to
// src/movie_record.cpp alongside FCEU_DrawLagCounter (its only writer
// and reader). The buffer is purely private to FCEU_DrawLagCounter,
// so promoting it from file-scope in movie.cpp to anonymous namespace
// in movie_record.cpp is a no-op.

// v1.13 Phase B / Batch D-D.6: MovieData record-array helpers
// (clearRecordRange / eraseRecords / insertEmpty / cloneRegion) and
// MovieRecord ctor / clear / Compare / Clone bodies, plus the
// MovieData ctor, relocated to src/movie_record.cpp. These are all
// pure record-array / per-frame utilities that co-locate cleanly with
// the recording-side manipulators and session IO already moved there.
//
// The MovieData static members loadSavestateFrom / dumpSavestateTo
// were already moved in Batch C; this batch covers the remaining
// non-static helpers and the constructors. Declared in movie.h
// (lines 114-274); only the bodies move, and call sites are
// unchanged (the v1.13 §0.2.1 pure-move principle).

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

// v1.13 Phase B / Batch D-D.2: GetMovieModeStr relocated to
// src/movie_record.cpp. Call site at line ~960 (MovieToggleReadOnly)
// resolves through the cross-TU forward decl at the top of this file.
//
// v1.13 Phase B / Batch D-D.3: GetMovieReadOnlyStr / GetMovieRecordModeStr
// also relocated to src/movie_record.cpp (HUD + recording helpers
// co-located). Forward decls at top of movie.cpp keep the remaining
// (RECORD branch in FCEUMOV_AddInputState until D-D.4) call sites
// resolving.

// v1.13 Phase B / Batch D-D.3: FCEU_DrawMovies / FCEU_DrawLagCounter /
// GetMovieRecordModeStr / GetMovieReadOnlyStr relocated to
// src/movie_record.cpp. All four are video-side rendering state that
// co-locates cleanly with the rest of the recording-session output.
//
// The two str() helpers were `static` in this TU; they lose the
// `static` qualifier (internal linkage) and gain external linkage so
// the FCEUMOV_AddInputState RECORD branch — which still lives in
// movie.cpp until D-D.4 — can keep calling them via forward decls at
// the top of movie_record.cpp (added in this commit). After D-D.4 the
// RECORD branch moves too and the forward decls drop.

// v1.13 Phase B / Batch D-D.2: openRecordingMovie / closeRecordingMovie /
// RedumpWholeMovieFile / StopPlayback / FinishPlayback / StopRecording /
// OnMovieClosed relocated to src/movie_record.cpp (recording-session
// IO + playback-lifecycle). The cross-TU decls already in
// movie_record.cpp:43-45 (RedumpWholeMovieFile / OnMovieClosed /
// GetMovieModeStr) become redundant since the definitions now live in
// the same TU; they are pruned there as part of this commit.
// movie_playback.cpp and movie.cpp add equivalent forward decls.

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

// v1.13 Phase B / Batch D-D.5: poweron body relocated to src/movie_io.cpp
// (see that file for the definition; the booger-flag dance around
// disableBatteryLoading stays accessible via fceu11::SaveMovie's
// cross-TU extern declared at the top of movie_io.cpp).

// v1.13 Phase B / Batch D-D.5: FCEUMOV_CreateCleanMovie /
// FCEUMOV_ClearCommands / FCEUMOV_FromPoweron relocated to
// src/movie_io.cpp.

// v1.13 Phase B / Batch C: MovieData::loadSavestateFrom + dumpSavestateTo
// bodies moved to movie_playback.cpp (Batch C); loadSaveramFrom /
// dumpSaveramTo / fceu11::LoadMovie / SaveMovie bodies relocated to
// src/movie_io.cpp (Batch D-D.5).


//the main interaction point between the emulator and the movie system.
//either dumps the current joystick state or loads one state from the movie
// v1.13 Phase B / Batch D-D.4: the TASEDITOR branch (uses
// g_tas_bridge function-pointer dispatch from Batch B) is now in
// movie_taseditor_bridge.cpp; the RECORD branch is in movie_record.cpp;
// the PLAY branch stays here as a thin inline dispatch. FCEUMOV_AddInputState
// itself becomes a thin per-frame router.

void MovieAddInputState_Playback();

// fceu11::SetVidSystem (loader-side) is not needed here; the bridge /
// playback / record helpers each contain their own MovieReplayEmulatedCommands
// or hot-path equivalent. The current dispatcher is intentionally tiny —
// keeping FCEUMOV_AddInputState() in TU-local switch keeps the per-frame
// call site readable for the regression suite.
void FCEUMOV_AddInputState()
{
	if (movieMode == MOVIEMODE_TASEDITOR)
		MovieAddInputState_TasEditor();
	else if (movieMode == MOVIEMODE_PLAY)
		MovieAddInputState_Playback();
	else if (movieMode == MOVIEMODE_RECORD)
		MovieAddInputState_Record();

	currFrameCounter++;

	extern uint8 joy[4];
	memcpy(&cur_input_display,joy,4);
}

// v1.13 Phase B / Batch D-D.4: PLAY branch kept here. The TASEDITOR
// branch (with g_tas_bridge) is in movie_taseditor_bridge.cpp; the
// RECORD branch (which writes to osRecordingMovie + truncates /
// overwrites / inserts based on movieRecordMode) is in movie_record.cpp.
void MovieAddInputState_Playback()
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

// v1.13 Phase B / Batch D-D.3: FCEU_DrawMovies / FCEU_DrawLagCounter
// bodies relocated to src/movie_record.cpp. See comment block above
// the GetMovieRecordModeStr helper for the migration rationale.

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


// v1.13 Phase B / Batch D-D.5: MoviePlayFromBeginning relocated to
// src/movie_io.cpp.

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

// v1.13 Phase B / Batch D-D.1: LoadSubtitles / ProcessSubtitles /
// FCEU_DisplaySubtitles relocated to src/movie_subtitles.cpp. The 3
// fns' declarations in movie.h are unchanged.

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

