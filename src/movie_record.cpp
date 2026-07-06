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

#include "fceu.h"
#include "utils/safe_string.h"

#include <cstring>
#include <vector>

// Helpers defined in movie.cpp (Phase F-B removed `static` from the
// related playback helpers; the same treatment is applied here for
// record-flow helpers called from this TU). They remain part of the
// movie module's internal surface — no public header exposure.
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
// defined in movie.cpp; the static helpers (RedumpWholeMovieFile /
// OnMovieClosed / GetMovieModeStr) stay in movie.cpp and resolve via
// the existing extern declarations.
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