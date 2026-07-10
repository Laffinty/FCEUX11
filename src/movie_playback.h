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