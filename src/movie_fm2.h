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