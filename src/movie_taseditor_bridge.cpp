// movie_taseditor_bridge.cpp
//
// v1.13 Phase B / Batch D-D.4: the MOVIEMODE_TASEDITOR branch of the
// per-frame dispatcher FCEUMOV_AddInputState relocated here from
// src/movie.cpp. The TAS EDITOR branch is the only one that calls
// into the function-pointer bridge (registered via
// fceu11::RegisterTasBridge in the TasEditorWindow constructor); the
// PLAY and RECORD branches stay / move with their respective files.
//
// movie.cpp keeps a thin FCEUMOV_AddInputState() dispatcher (~25 lines
// with the inline `currFrameCounter++; memcpy(&cur_input_display,joy,4);`
// trailer) that switches on movieMode and forwards to the helper in
// this file.

#include "movie.h"

#include "fceu.h"
#include "input.h"
#include "fds.h"
#include "vsuni.h"

#include "core_api.h"

// v1.13 Phase B / Batch D-D.4 + B: TAS Editor branch helper.
// Returns true iff the dispatcher should fall through to the post-frame
// trailer (currFrameCounter++ / cur_input_display update).
//
// Hot-path: called once per emulated frame when movieMode ==
// MOVIEMODE_TASEDITOR. The TAS bridge call is gated by a non-null
// is_recording check; when the TasEditor GUI is closed (and
// UnregisterTasBridge has been called) the helper behaves as a pure
// replay-only path.

// v1.13 Phase B / Batch D-D.5 + D-D.4: g_tas_bridge (B) and
// _currCommand (D-D.5) live in movie.cpp as TU-externals (each
// promoted from anonymous / TU-static so a helper in this TU can
// read/write them).
extern fceu11::TasBridge g_tas_bridge;
extern int _currCommand;

namespace {

inline void MovieReplayEmulatedCommands(MovieRecord& mr)
{
	if (mr.command_power())
		PowerNES();
	if (mr.command_reset())
		ResetNES();
	if (mr.command_fds_insert())
		FCEU_FDSInsert();
	if (mr.command_fds_select())
		FCEU_FDSSelect();
	if (mr.command_vs_insertcoin())
		FCEU_VSUniCoin(0);
	if (mr.command_vs_insertcoin2())
		FCEU_VSUniCoin(1);
	if (mr.command_vs_service())
		FCEU_VSUniService();
}

inline void MoviePadEmptyFramesToFrame(int frame)
{
	// if movie length is less or equal to currFrame, pad it with empty frames
	if (((int)currMovieData.records.size() - 1) < (frame + 1))
		currMovieData.insertEmpty(-1, (frame + 1) - ((int)currMovieData.records.size() - 1));
}

} // anonymous namespace

void MovieAddInputState_TasEditor()
{
	MoviePadEmptyFramesToFrame(currFrameCounter);

	MovieRecord* mr = &currMovieData.records[currFrameCounter];
	if (g_tas_bridge.is_recording && g_tas_bridge.is_recording(g_tas_bridge.ctx))
	{
		// record commands and buttons
		mr->commands |= _currCommand;
		joyports[0].log(mr);
		joyports[1].log(mr);
		if (g_tas_bridge.record_input)
			g_tas_bridge.record_input(g_tas_bridge.ctx);
	}
	// replay buttons
	joyports[0].load(mr);
	joyports[1].load(mr);
	// replay commands
	MovieReplayEmulatedCommands(*mr);
	_currCommand = 0;
}
