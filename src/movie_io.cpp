// movie_io.cpp
//
// v1.13 Phase B / Batch D-D.5: movie IO cluster relocated here from
// src/movie.cpp. Owns the user-facing load/save/play-from-beginning
// surface plus the supporting helpers (poweron / CreateCleanMovie /
// ClearCommands / FromPoweron / loadSaveramFrom / dumpSaveramTo).
//
// Split rationale: the cluster is co-located on the same call paths
// (fceu11::LoadMovie uses poweron + MovieData::loadSaveramFrom +
// MovieData::loadSavestateFrom; fceu11::SaveMovie uses the saveram +
// savestate dumpers). Grouping them into one TU lets the per-call
// dependency surface stay clean (state.h, cart.h, zlib.h, rust/) and
// keeps movie.cpp narrow.

#include "movie.h"

#include "emufile.h"
#include "version.h"
#include "types.h"

#include "utils/endian.h"
#include "utils/memory.h"
#include "utils/safe_string.h"
#include "utils/xstring.h"

#include "fceu.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "netplay.h"
#include "driver_callbacks.h"
#include "state.h"
#include "file.h"
#include "video.h"
#include "cart.h"
#include "fds.h"
#include "vsuni.h"
#include "palette.h"
#include "input.h"
#include "rust/fceux11_rust.h"

#ifdef _S9XLUA_H
#include "fceulua.h"
#endif

#include <sstream>
#include <algorithm>
#include <cstdio>
#include <cstring>
#include <cassert>
#include <iomanip>
#include <fstream>
#include <climits>
#include <cstdarg>
#include <zlib.h>

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.5: cross-TU fwd decls for the helpers now
// living in movie_record.cpp / movie_playback.cpp that this TU's body
// references (StopPlayback / StopRecording / openRecordingMovie /
// RedumpWholeMovieFile / SFORMAT FCEUMOV_STATEINFO etc. resolve via
// movie_record.cpp / movie_playback.cpp).
// ----------------------------------------------------------------------------
void StopPlayback();
void StopRecording();
EMUFILE *openRecordingMovie(const char* fname);
void RedumpWholeMovieFile(bool justToggledRecording = false);
void LoadSubtitles(MovieData &moviedata);

// v1.13 Phase B / Batch D-D.5: externs for the file-scope state this TU
// writes to (movieMode, currFrameCounter, currRerecordCount, ...).
// These come from the existing extern-block in movie.h plus several
// file-scope globals from movie.cpp — see movie.cpp for the originals.
extern int pauseframe;
extern uint32_t cur_input_display;
extern bool bogorf;
extern int RAMInitOption;
extern int RAMInitSeed;
extern char FileBase[];
extern bool movieFromPoweron;
extern int currRerecordCount;
extern int LoggingEnabled;
extern EMUFILE* osRecordingMovie;
extern bool AutoSS;

// ----------------------------------------------------------------------------
// Power-on + clean-movie helpers
// ----------------------------------------------------------------------------

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
	// fceu11::InputDevice / InputDeviceFC form cast to int.
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
	extern int _currCommand;
	_currCommand = 0;
}

bool FCEUMOV_FromPoweron()
{
	return movieFromPoweron;
}

// ----------------------------------------------------------------------------
// Saveram load/dump — invoked from LoadMovie / SaveMovie
// (MovieData::loadSavestateFrom / dumpSavestateTo moved to
// movie_playback.cpp in Batch C).
// ----------------------------------------------------------------------------

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


// ----------------------------------------------------------------------------
// Load / Save / Play-from-beginning
// ----------------------------------------------------------------------------

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
	extern int LoggingEnabled;
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
