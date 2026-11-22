/* FCE Ultra - NES/Famicom Emulator
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
// movie_record.h
//
// v1.12 Scissors Phase F-C: recording-manipulator split.
//
// Pure code move from src/movie.cpp — lines 1016-1186
// (FCEUI_MovieToggleRecording, FCEUI_MovieInsertFrame,
// FCEUI_MovieDeleteFrame, FCEUI_MovieTruncate,
// FCEUI_MovieNextRecordMode/PrevRecordMode/
// RecordModeTruncate/Overwrite/Insert).
//
// The recording-manipulator functions form the public recording-side
// command surface invoked from the EMUCMD_MOVIE_* table (input.cpp)
// and from the ConsoleWindow toolbar / TAS Editor. They are split
// from playback/format sub-modules per plan §5.
//
// Class declarations (MovieRecord / MovieData) stay in movie.h per
// the prior-phase convention; only method bodies move.

#pragma once

#include "movie.h"   // MovieRecord / MovieData declarations