// movie_playback.cpp
//
// v1.12 Scissors Phase F-B: savestate-plugin split.
//
// Pure code move from src/movie.cpp — lines 910-1105.
// See movie_playback.h.

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
#include "movie_playback.h"

#include "emufile.h"

#include "fceu.h"
#include "driver.h"
#include "driver_callbacks.h"
#include "state.h"

#include <cstdint>

// These helpers are defined in movie.cpp but called from FCEUMOV_ReadState
// below. Drop-static promotions: declarations here resolve cross-TU
// through the existing movie.cpp definitions (their `static` keyword
// was removed as part of Phase F-B). They remain part of the movie
// module's internal surface (no public header exposure).
void closeRecordingMovie();
void RedumpWholeMovieFile(bool justToggledRecording = false);
void FinishPlayback();

// ----------------------------------------------------------------------------
// Savestate plugin: write side
// ----------------------------------------------------------------------------

int FCEUMOV_WriteState(EMUFILE* os)
{
	//we are supposed to dump the movie data into the savestate
	if(movieMode == MOVIEMODE_RECORD || movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_FINISHED)
		return currMovieData.dump(os, true);
	else return 0;
}

// returns
int CheckTimelines(MovieData& stateMovie, MovieData& currMovie)
{
	// end_frame = min(urrMovie.records.size(), stateMovie.records.size(), currFrameCounter)
	int end_frame = currMovie.records.size();
	if (end_frame > (int)stateMovie.records.size())
		end_frame = stateMovie.records.size();
	if (end_frame > currFrameCounter)
		end_frame = currFrameCounter;

	for (int x = 0; x < end_frame; x++)
	{
		if (!stateMovie.records[x].Compare(currMovie.records[x]))
			return x;
	}
	// no mismatch found
	return -1;
}

// ----------------------------------------------------------------------------
// Savestate plugin: load side
//
// Set by FCEUMOV_ReadState once it has successfully projected the
// savestate's MovieData into currMovieData (and validated timeline
// compatibility). FCEUMOV_PreLoad clears it; FCEUMOV_PostLoad returns
// it so state.cpp can decide whether to commit the load.
// ----------------------------------------------------------------------------

static bool load_successful = false;

bool FCEUMOV_ReadState(EMUFILE* is, uint32 size)
{
	load_successful = false;

	if (!movie_readonly)
	{
		if (currMovieData.loadFrameCount >= 0)
		{
			if (auto* fn = fceu11::g_driver().message_box) {
				fn("Movie Replay", "This movie is a TAS Editor project file.\nIt can be modified in TAS Editor only.\n\nOpen it in TAS Editor now?", 4);
			} else {
				FCEU_printf("This movie is a TAS Editor project file! It can be modified in TAS Editor only.\nMovie is now Read-Only.\n");
			}
			movie_readonly = true;
		}
		if (FCEU_isFileInArchive(curMovieFilename.c_str()))
		{
			//a little rule: cant load states in read+write mode with a movie from an archive.
			//so we are going to switch it to readonly mode in that case
			FCEU_PrintError("Cannot loadstate in Read+Write with movie from archive. Movie is now Read-Only.");
			movie_readonly = true;
		}
	}

	MovieData tempMovieData = MovieData();
	std::ios::pos_type curr = is->ftell();
	if(!LoadFM2(tempMovieData, is, size, false)) {
		is->fseek((uint32)curr+size,SEEK_SET);
		extern bool FCEU_state_loading_old_format;
		if(FCEU_state_loading_old_format) {
			if(movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_RECORD || movieMode == MOVIEMODE_FINISHED) {
				//fceu11::StopMovie();  //No reason to stop the movie, nothing destructive has happened yet.
				FCEU_PrintError("You have tried to use an old savestate while playing a movie. This is unsupported (since the old savestate has old-format movie data in it which can't be converted on the fly)");
			}
		}
		return false;
	}

	//----------------
	//complex TAS logic for loadstate
	//fully conforms to the savestate logic documented in the Laws of TAS
	//http://tasvideos.org/LawsOfTAS/OnSavestates.html
	//----------------


	if(movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_RECORD || movieMode == MOVIEMODE_FINISHED)
	{
		//handle moviefile mismatch
		if(tempMovieData.guid != currMovieData.guid)
		{
			if (auto* fn = fceu11::g_driver().message_box) {
				std::string msg = "There is a mismatch between savestate's movie and current movie.\ncurrent: " + currMovieData.guid.toString() + "\nsavestate: " + tempMovieData.guid.toString() + "\n\nThis means that you have loaded a savestate belonging to a different movie than the one you are playing now.\n\nContinue loading this savestate anyway?";
				int result = fn("Error loading savestate", msg.c_str(), 1);
				if(result == 2)
				{
					if (!backupSavestates)
					{
						FCEU_PrintError("Unable to restore backup, movie playback stopped.");
						fceu11::StopMovie();
					}

					return false;
				}
			} else {
				if (!backupSavestates)
				{
					FCEU_PrintError("Mismatch between savestate's movie and current movie.\ncurrent: %s\nsavestate: %s\nUnable to restore backup, movie playback stopped.\n",currMovieData.guid.toString().c_str(),tempMovieData.guid.toString().c_str());
					fceu11::StopMovie();
				}
				else
				FCEU_PrintError("Mismatch between savestate's movie and current movie.\ncurrent: %s\nsavestate: %s\n",currMovieData.guid.toString().c_str(),tempMovieData.guid.toString().c_str());

				return false;
			}
		}

		if (movie_readonly)
		{
			if (movieMode == MOVIEMODE_RECORD)
			{
				movieMode = MOVIEMODE_PLAY;
				RedumpWholeMovieFile(true);
				closeRecordingMovie();
			}

			// currFrameCounter at this point represents the savestate framecount
			int frame_of_mismatch = CheckTimelines(tempMovieData, currMovieData);
			if (frame_of_mismatch >= 0)
			{
				// Wrong timeline, do apprioriate logic here
				if (!backupSavestates)	//If backups are disabled we can just resume normally since we can't restore so stop movie and inform user
				{
					FCEU_PrintError("Error: Savestate not in the same timeline as movie!\nFrame %d branches from current timeline\nUnable to restore backup, movie playback stopped.", frame_of_mismatch);
					fceu11::StopMovie();
				} else
					FCEU_PrintError("Error: Savestate not in the same timeline as movie!\nFrame %d branches from current timeline", frame_of_mismatch);
				return false;
			} else if ((int)tempMovieData.records.size() < currFrameCounter)
			{
				// this is post-movie savestate and must be checked further
				if (tempMovieData.records.size() < currMovieData.records.size())
				{
					// this savestate doesn't contain enough input to be checked
					//TODO: turn frame counter to red to get attention
					if (!backupSavestates)	//If backups are disabled we can just resume normally since we can't restore so stop movie and inform user
					{
						FCEU_PrintError("Error: Savestate taken from a frame (%d) after the final frame in the savestate movie (%zi) cannot be verified against current movie (%zi). This is not permitted.\nUnable to restore backup, movie playback stopped.", currFrameCounter, tempMovieData.records.size() - 1, currMovieData.records.size() - 1);
						fceu11::StopMovie();
					} else
						FCEU_PrintError("Savestate taken from a frame (%d) after the final frame in the savestate movie (%zi) cannot be verified against current movie (%zi). This is not permitted.", currFrameCounter, tempMovieData.records.size() - 1, currMovieData.records.size() - 1);
					return false;
				}
			}

			// Finally, this is a savestate file for this movie
			// We'll allow loading post-movie savestates that were made after finishing current movie
			if (currFrameCounter < (int)currMovieData.records.size())
				movieMode = MOVIEMODE_PLAY;
			else
				FinishPlayback();
		} else
		{
			//Read+Write mode
			closeRecordingMovie();

			if (currFrameCounter > (int)tempMovieData.records.size())
			{
				//This is a post movie savestate, handle it differently
				//Replace movie contents but then switch to movie finished mode
				currMovieData = tempMovieData;
				movieMode = MOVIEMODE_PLAY;
				FCEUMOV_IncrementRerecordCount();
				RedumpWholeMovieFile();
				FinishPlayback();
			} else
			{
				//truncate before we copy, just to save some time, unless the user selects a full copy option
				if (!fullSaveStateLoads)
					//we can only assume this here since we have checked that the frame counter is not greater than the movie data
					tempMovieData.truncateAt(currFrameCounter);

				currMovieData = tempMovieData;
				movieMode = MOVIEMODE_RECORD;
				FCEUMOV_IncrementRerecordCount();
				RedumpWholeMovieFile(true);
			}
		}
	}

	load_successful = true;

	return true;
}

void FCEUMOV_PreLoad(void)
{
	load_successful=0;
}

bool FCEUMOV_PostLoad(void)
{
	if(movieMode == MOVIEMODE_INACTIVE || movieMode == MOVIEMODE_TASEDITOR)
		return true;
	else
		return load_successful;
}