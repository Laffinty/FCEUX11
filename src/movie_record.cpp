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
#include "utils/safe_string.h"

#include <cstring>
#include <vector>

// v1.13 Phase B / Batch D-D.2: definitions of `osRecordingMovie` (declared
// extern in movie.h:284) and `bindSavestate` (driver/settings) live in
// movie.cpp / drivers layer; `AutoSS` lives in fceu.cpp. Re-declare here
// to resolve the cross-TU symbol references inside the session helpers
// moved into this file. (bindSavestate's true type is `bool` per file.h:12.)
extern EMUFILE* osRecordingMovie;
extern bool AutoSS;

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