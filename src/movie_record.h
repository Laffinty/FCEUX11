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