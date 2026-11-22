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
// movie_fm2.h
//
// v1.12 Scissors Phase F-A: FM2 format parsing/serialization split.
//
// Pure code move from src/movie.cpp — lines 200, 202-373 (MovieRecord
// FM2 I/O), 392-395 (MovieData::truncateAt), 397-462 (installValue),
// 464-544 (dump), 596-686 (LoadFM2).
//
// Class declarations (MovieRecord / MovieData) stay in movie.h per the
// prior-phase convention; only method bodies move.

#pragma once

#include "movie.h"   // MovieRecord / MovieData declarations