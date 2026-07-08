// movie_record.cpp
//
// v1.12 Scissors Phase F-C: recording-manipulator split.
//
// Pure code move from src/movie.cpp — lines 1016-1186.
// See movie_record.h.

/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 1998 BERO
 *  Copyright (C) 2003 Xodnizel
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */

#include "types.h"
#include "movie.h"
#include "movie_record.h"

#include "emufile.h"
#include "fceu.h"
#include "driver.h"
#include "driver_callbacks.h"
#include "file.h"   // `extern bool bindSavestate;`
#include "input.h"  // lagCounter / lagCounterDisplay / lagFlag (move globals)
#include "utils/safe_string.h"
#include "utils/memory.h"   // memset
#include "video.h"  // DrawTextTrans / ClipSidesOffset / FCEU_TextScanlineOffsetFromBottom
#include "drawing.h"

// v1.13 Phase B / Batch D-D.6: MovieData / MovieRecord constructors need
// MOVIE_VERSION (defined in movie.cpp via #define, but we keep the macro
// here too — historically it lived at the top of movie.cpp; carrying
// it over here too avoids pulling in version.h gratuitously when the
// macro is already in scope through the next TU). FCEU_VERSION_NUMERIC
// comes from version.h transitively.
#define MOVIE_VERSION           3

// v1.13 Phase B / Batch D-D.3: extra includes for the HUD overlay
// helpers. drawing.h is also pulled in transitively by video.h.

#include <cstring>
#include <vector>

// v1.13 Phase B / Batch D-D.2: definitions of `osRecordingMovie` (declared
// extern in movie.h:284) and `bindSavestate` (driver/settings) live in
// movie.cpp / drivers layer; `AutoSS` lives in fceu.cpp. Re-declare here
// to resolve the cross-TU symbol references inside the session helpers
// moved into this file. (bindSavestate's true type is `bool` per file.h:12.)
extern EMUFILE* osRecordingMovie;
extern bool AutoSS;

// v1.13 Phase B / Batch D-D.5: the per-command staging int was promoted
// from TU-static (movie.cpp) to TU-external so the moved
// FCEUMOV_ClearCommands (movie_io.cpp) and the still-resident
// FCEUMOV_AddCommand (movie.cpp) plus MovieAddInputState_Record (this
// TU, D-D.4) share it without going through a function call.
extern int _currCommand;

// v1.13 Phase B / Batch D-D.3: HUD overlay helpers reference int
// frame_display / rerecord_display file-scope globals from fceu.cpp
// (not in any header). lagCounter / lagCounterDisplay / lagFlag come
// in via input.h:309-311.
extern int frame_display;
extern int rerecord_display;

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.2: helper forward-declarations for the
// pre-existing v1.12 manipulator block (which still references these
// by name — the v1.13 §0.2.1 pure-move principle keeps the manipulator
// call sites unchanged). Definitions are also in this TU (see end of
// file) but C++ requires a forward decl before first use.
// ----------------------------------------------------------------------------
void RedumpWholeMovieFile(bool justToggledRecording = false);
void OnMovieClosed();
const char* GetMovieModeStr();

// ----------------------------------------------------------------------------
// Recording-side manipulators.
//
// These functions form the user-facing recording command surface:
// they toggle playback↔record mode, insert/delete/truncate frames, and
// select the active record-mode (TRUNCATE / OVERWRITE / INSERT). Most
// dispatch through `movieMode` checks against the file-static state
// defined in movie.cpp; the helpers listed above (now in this TU) are
// no longer cross-TU references.
// ----------------------------------------------------------------------------

void FCEUI_MovieToggleRecording()
{
	char message[260] = "";

	if (movieMode == MOVIEMODE_INACTIVE)
		FCEU_strlcpy(message, sizeof(message), "Cannot toggle Recording");
	else if (currFrameCounter > (int)currMovieData.records.size())
	{
		movie_readonly = !movie_readonly;
		if (movie_readonly)
			FCEU_strlcpy(message, sizeof(message), "Movie is now Read-Only (finished)");
		else
			FCEU_strlcpy(message, sizeof(message), "Movie is now Read+Write (finished)");
	} else if (movieMode == MOVIEMODE_PLAY || (movieMode == MOVIEMODE_FINISHED && currFrameCounter == (int)currMovieData.records.size()))
	{
		FCEU_strlcpy(message, sizeof(message), "Movie is now Read+Write");
		movie_readonly = false;
		FCEUMOV_IncrementRerecordCount();
		movieMode = MOVIEMODE_RECORD;
		RedumpWholeMovieFile(true);
	} else if (movieMode == MOVIEMODE_RECORD)
	{
		FCEU_strlcpy(message, sizeof(message), "Movie is now Read-Only");
		movie_readonly = true;
		movieMode = MOVIEMODE_PLAY;
		RedumpWholeMovieFile(true);
		if (currFrameCounter >= (int)currMovieData.records.size())
		{
			extern int closeFinishedMovie;
			if (closeFinishedMovie)
			{
				movieMode = MOVIEMODE_INACTIVE;
				OnMovieClosed();
			} else
				movieMode = MOVIEMODE_FINISHED;
		}
	} else
		FCEU_strlcpy(message, sizeof(message), "Nothing to do in this mode");

	safe_strcat(message, sizeof(message), GetMovieModeStr());

	FCEU_DispMessage("%s",0,message);
}

void FCEUI_MovieInsertFrame()
{
	char message[260] = "";

	if (movieMode == MOVIEMODE_INACTIVE)
		FCEU_strlcpy(message, sizeof(message), "No movie to insert a frame.");
	else if (movie_readonly)
		FCEU_strlcpy(message, sizeof(message), "Cannot modify movie in Read-Only mode.");
	else if (currFrameCounter > (int)currMovieData.records.size())
		FCEU_strlcpy(message, sizeof(message), "Cannot insert a frame here.");
	else if (movieMode == MOVIEMODE_RECORD || movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_FINISHED)
	{
		FCEU_strlcpy(message, sizeof(message), "1 frame inserted");
		safe_strcat(message, sizeof(message), GetMovieModeStr());
		std::vector<MovieRecord>::iterator iter = currMovieData.records.begin();
		currMovieData.records.insert(iter + currFrameCounter, MovieRecord());
		FCEUMOV_IncrementRerecordCount();
		RedumpWholeMovieFile();
	} else
	{
		FCEU_strlcpy(message, sizeof(message), "Nothing to do in this mode");
		safe_strcat(message, sizeof(message), GetMovieModeStr());
	}

	FCEU_DispMessage("%s",0,message);
}

void FCEUI_MovieDeleteFrame()
{
	char message[260] = "";

	if (movieMode == MOVIEMODE_INACTIVE)
		FCEU_strlcpy(message, sizeof(message), "No movie to delete a frame.");
	else if (movie_readonly)
		FCEU_strlcpy(message, sizeof(message), "Cannot modify movie in Read-Only mode.");
	else if (currFrameCounter >= (int)currMovieData.records.size())
		FCEU_strlcpy(message, sizeof(message), "Nothing to delete past movie end.");
	else if (movieMode == MOVIEMODE_RECORD || movieMode == MOVIEMODE_PLAY)
	{
		FCEU_strlcpy(message, sizeof(message), "1 frame deleted");
		std::vector<MovieRecord>::iterator iter = currMovieData.records.begin();
		currMovieData.records.erase(iter + currFrameCounter);
		FCEUMOV_IncrementRerecordCount();
		RedumpWholeMovieFile();

		if (movieMode != MOVIEMODE_RECORD && currFrameCounter >= (int)currMovieData.records.size())
		{
			extern int closeFinishedMovie;
			if (closeFinishedMovie)
			{
				movieMode = MOVIEMODE_INACTIVE;
				OnMovieClosed();
			} else
				movieMode = MOVIEMODE_FINISHED;
		}
		safe_strcat(message, sizeof(message), GetMovieModeStr());
	} else
	{
		FCEU_strlcpy(message, sizeof(message), "Nothing to do in this mode");
		safe_strcat(message, sizeof(message), GetMovieModeStr());
	}

	FCEU_DispMessage("%s",0,message);
}

void FCEUI_MovieTruncate()
{
	char message[260] = "";

	if (movieMode == MOVIEMODE_INACTIVE)
		FCEU_strlcpy(message, sizeof(message), "No movie to truncate.");
	else if (movie_readonly)
		FCEU_strlcpy(message, sizeof(message), "Cannot modify movie in Read-Only mode.");
	else if (currFrameCounter >= (int)currMovieData.records.size())
		FCEU_strlcpy(message, sizeof(message), "Nothing to truncate past movie end.");
	else if (movieMode == MOVIEMODE_RECORD || movieMode == MOVIEMODE_PLAY)
	{
		FCEU_strlcpy(message, sizeof(message), "Movie truncated");
		currMovieData.truncateAt(currFrameCounter);
		FCEUMOV_IncrementRerecordCount();
		RedumpWholeMovieFile();

		if (movieMode != MOVIEMODE_RECORD)
		{
			extern int closeFinishedMovie;
			if (closeFinishedMovie)
			{
				movieMode = MOVIEMODE_INACTIVE;
				OnMovieClosed();
			}
			else
				movieMode = MOVIEMODE_FINISHED;
		}
		safe_strcat(message, sizeof(message), GetMovieModeStr());
	} else
	{
		FCEU_strlcpy(message, sizeof(message), "Nothing to do in this mode");
		safe_strcat(message, sizeof(message), GetMovieModeStr());
	}

	FCEU_DispMessage("%s",0,message);
}

void FCEUI_MovieNextRecordMode()
{
	movieRecordMode = (movieRecordMode + 1) % MOVIE_RECORD_MODE_MAX;
}

void FCEUI_MoviePrevRecordMode()
{
	movieRecordMode = (movieRecordMode + MOVIE_RECORD_MODE_MAX - 1) % MOVIE_RECORD_MODE_MAX;
}

void FCEUI_MovieRecordModeTruncate()
{
	movieRecordMode = MOVIE_RECORD_MODE_TRUNCATE;
}

void FCEUI_MovieRecordModeOverwrite()
{
	movieRecordMode = MOVIE_RECORD_MODE_OVERWRITE;
}

void FCEUI_MovieRecordModeInsert()
{
	movieRecordMode = MOVIE_RECORD_MODE_INSERT;
}

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.2: Recording-session IO helpers +
// playback/recording lifecycle helpers relocated here from src/movie.cpp.
// All of these touch osRecordingMovie / movieMode / curMovieFilename /
// etc. — i.e. exclusively recording-side state. Moving them co-locates
// the "save a movie file to disk" and "open a movie recording" flows
// with the existing recording manipulators above.
//
// `bindSavestate` / `AutoSS` / `g_driver()` continue to resolve via
// the extern declarations in fceu.h / driver.h / driver_callbacks.h
// pulled in at the top of this file.
// ----------------------------------------------------------------------------

EMUFILE *openRecordingMovie(const char* fname)
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
void RedumpWholeMovieFile(bool justToggledRecording)
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
void StopPlayback()
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
void StopRecording()
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

const char* GetMovieModeStr()
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

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.3: HUD overlays relocated here from
// src/movie.cpp. All four helpers are called from src/video.cpp
// (the per-frame video renderer); pulling them together with the
// recording-session IO block they format makes for a coherent
// "movie output" TU.
//
// GetMovieReadOnlyStr / GetMovieRecordModeStr lose their v1.12
// `static` qualifier and gain external linkage — the remaining
// FCEUMOV_AddInputState RECORD branch in movie.cpp calls them via
// forward decls at the top of movie.cpp until D-D.4 moves that branch.
// ----------------------------------------------------------------------------

const char *GetMovieReadOnlyStr()
{
	if (movieMode == MOVIEMODE_RECORD)
		return movie_readonly ? " R-O" : "";
	else
		return movie_readonly ? "" : " R+W";
}

const char *GetMovieRecordModeStr()
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

// v1.13 Phase B / Batch D-D.3: lagcounterbuf file-scope storage,
// formerly at the top of movie.cpp. Demoted to anonymous namespace
// here — its only writer/reader is FCEU_DrawLagCounter below, both
// in this TU.
namespace {
char lagcounterbuf[32] = {0};
} // anonymous namespace

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

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.6: MovieData record-array helpers +
// MovieRecord per-frame utilities + MovieData / MovieRecord ctors
// relocated here from src/movie.cpp.
//
// The 5 MovieData static methods loadSavestateFrom / dumpSavestateTo /
// loadSaveramFrom / dumpSaveramTo stay in movie_playback.cpp /
// movie.cpp respectively (already moved by earlier batches and
// covered by separate audit). The methods moved in this batch are
// all NON-static and touch only `this->records` / `this->joysticks` /
// `this->zappers` / `this->commands` / the MovieData member POD —
// no cross-module externs needed other than what movie.h already
// provides (MD5DATA, FCEU_VERSION_NUMERIC, MOVIE_VERSION).
// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------
// v1.13 Phase B / Batch D-D.4: per-frame RECORD branch helper. The main
// FCEUMOV_AddInputState() in movie.cpp dispatches here on
// movieMode == MOVIEMODE_RECORD.
// ----------------------------------------------------------------------------
void MovieAddInputState_Record()
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