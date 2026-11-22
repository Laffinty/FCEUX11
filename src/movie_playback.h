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
// movie_playback.h
//
// v1.12 Scissors Phase F-B: savestate-plugin split.
//
// Pure code move from src/movie.cpp — lines 910-1105 (FCEUMOV_WriteState,
// CheckTimelines, FCEUMOV_ReadState, FCEUMOV_PreLoad, FCEUMOV_PostLoad,
// and the load_successful file-static).
//
// The savestate-plugin functions form the playback hook surface
// called by state.cpp at save/load time. They are split from the
// recording/format sub-modules per plan §5.
//
// Class declarations (MovieRecord / MovieData) stay in movie.h per the
// prior-phase convention; only method bodies move.

#pragma once

#include "movie.h"   // MovieRecord / MovieData declarations