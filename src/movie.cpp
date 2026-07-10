#include "emufile.h"
#include "version.h"
#include "types.h"
#include "utils/endian.h"
#include "palette.h"
#include "input.h"
#include "fceu.h"
#include "netplay.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "driver_callbacks.h"
#include "state.h"
#include "file.h"
#include "video.h"
#include "movie.h"
#include "cart.h"
#include "fds.h"
#include "vsuni.h"
#include "rust/fceux11_rust.h"
#ifdef _S9XLUA_H
#include "fceulua.h"
#endif
#include "utils/guid.h"
#include "utils/memory.h"
#include "utils/xstring.h"
#include "utils/safe_string.h"
#include <sstream>
#include <algorithm>

#ifdef CREATE_AVI
#endif

#ifdef WIN32
#include <windows.h>
#endif

// =========================================================================
// v1.13 Phase B master migration note for src/movie.cpp:
//   movie.cpp was 1203 lines pre-Phase B. After B + C + D.1..D.5 the file
//   carries only the per-frame dispatcher + the public FCEUI_* / fceu11::
//   surface that has no specialized TU. The following carve-outs explain
//   what's left in this file and where every migrated block went:
//
//   Batch B (commit 32cd40e)
//     - Dropped the #include "./drivers/Qt/TasEditor/TasEditorWindow.h"
//       drag (now exposed via fceu11::RegisterTasBridge in movie.h).
//
//   Batch C (commit eeabe86)
//     - SFORMAT FCEUMOV_STATEINFO[] + MovieData::loadSavestateFrom +
//       dumpSavestateTo bodies → movie_playback.cpp (savestate plugin).
//
//   Batch D-D.1 (commit 0b7b3fe)
//     - LoadSubtitles / ProcessSubtitles / FCEU_DisplaySubtitles + the
//       three subtitle globals → movie_subtitles.cpp (NEW).
//
//   Batch D-D.2 (commit bfe9d14)
//     - openRecordingMovie / closeRecordingMovie / RedumpWholeMovieFile
//       + StopPlayback / FinishPlayback / StopRecording / OnMovieClosed
//       + GetMovieModeStr → movie_record.cpp (recording session IO).
//
//   Batch D-D.3 (commit 1459430)
//     - FCEU_DrawMovies / FCEU_DrawLagCounter + GetMovieReadOnlyStr +
//       GetMovieRecordModeStr + lagcounterbuf → movie_record.cpp (HUD).
//
//   Batch D-D.6 (commit 85fddc6)
//     - MovieData record-array helpers (clearRecordRange, eraseRecords,
//       insertEmpty, cloneRegion) + MovieRecord ctor/clear/Compare/Clone
//       + MovieData ctor → movie_record.cpp (record helpers).
//
//   Batch D-D.5-extension (movie_io.cpp, NEW):
//     - poweron / FCEUMOV_CreateCleanMovie / FCEUMOV_ClearCommands /
//       FCEUMOV_FromPoweron + MovieData::loadSaveramFrom / dumpSaveramTo
//       + fceu11::LoadMovie / SaveMovie / MoviePlayFromBeginning.
//
//   Batch D-D.4 (commit d843d46, movie_taseditor_bridge.cpp, NEW):
//     - MOVIEMODE_TASEDITOR branch of FCEUMOV_AddInputState + the
//       g_tas_bridge function-pointer dispatch.
//
//   Batch D-D.5-extension (movie_settings.cpp, NEW):
//     - FCEUMOV_AddCommand / IncrementRerecordCount / MovieToggle*
//       family / Get/Set Movie Toggle Read-Only / MovieGetInfo +
//       FCEUI_CreateMovieFile / FCEUI_MakeBackupMovie.
//
//   The remainder of this file: mode queries + lag queries + ShouldPause
//   + GetMovieName + the per-frame dispatcher + the PLAY branch helper
//   (MovieAddInputState_Playback) + fceu11::StopMovie + the bridge
//   storage + register/unregister free fns.
// =========================================================================

extern int RAMInitOption;
extern int RAMInitSeed;

// Cross-TU forward decls (helpers now in movie_record.cpp / movie_io.cpp
// / movie_taseditor_bridge.cpp / movie_settings.cpp).
void StopPlayback();
void StopRecording();
void FinishPlayback();
void OnMovieClosed();
void RedumpWholeMovieFile(bool justToggledRecording = false);
EMUFILE *openRecordingMovie(const char* fname);
const char* GetMovieModeStr();                // movie_record.cpp
const char* GetMovieReadOnlyStr();            // movie_record.cpp
const char* GetMovieRecordModeStr();          // movie_record.cpp
void MovieAddInputState_TasEditor();          // movie_taseditor_bridge.cpp
void MovieAddInputState_Record();             // movie_record.cpp

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cassert>
#include <iomanip>
#include <fstream>
#include <climits>
#include <cstdarg>
#include <zlib.h>

// v1.2 Census §2.3: `using namespace std` removed; std types below are
// explicitly qualified with std::.

// v1.13 Purify H: #define → constexpr
inline constexpr int MOVIE_VERSION = 3;

extern char FileBase[];
extern bool AutoSS;		//Declared in fceu.cpp, keeps track if a auto-savestate has been made

// v1.13 Phase B / Batch D-D.1: subtitlesOnAVI + subtitleFrames / subtitleMessages /
// LoadSubtitles / ProcessSubtitles / FCEU_DisplaySubtitles relocated to
// src/movie_subtitles.cpp. The `extern bool subtitlesOnAVI` in movie.h:280
// resolves to the new TU at link time.
bool autoMovieBackup = false; //Toggle that determines if movies should be backed up automatically before altering them
bool freshMovie = false;	  //True when a movie loads, false when movie is altered.  Used to determine if a movie has been altered since opening
bool movieFromPoweron = true;

// v1.13 Phase B / Batch B + D-D.4: TAS Editor bridge. Set via
// fceu11::RegisterTasBridge from the TasEditorWindow GUI; consumed by
// MovieAddInputState_TasEditor in movie_taseditor_bridge.cpp.
fceu11::TasBridge g_tas_bridge{nullptr, nullptr, nullptr};

void fceu11::RegisterTasBridge(const fceu11::TasBridge& b)
{
	g_tas_bridge = b;
}

void fceu11::UnregisterTasBridge()
{
	g_tas_bridge = fceu11::TasBridge{nullptr, nullptr, nullptr};
}

// v1.13 Phase B / Batch D-D.5: promoted from TU-static for cross-TU
// read/write from movie_io.cpp / movie_settings.cpp / movie_record.cpp.
int _currCommand = 0;

//TODO - remove the synchack stuff from the replay gui and require it to be put into the fm2 file
//which the user would have already converted from fcm
//also cleanup the whole emulator version bullshit in replay. we dont support that old stuff anymore

//todo - better error handling for the compressed savestate

//todo - consider a MemoryBackedFile class..
//..a lot of work.. instead lets just read back from the current fcm

//todo - could we, given a field size, over-read from an inputstream and then parse out an integer?
//that would be faster than several reads, perhaps.

//sometimes we accidentally produce movie stop signals while we're trying to do other things with movies..
bool suppressMovieStop=false;

//----movie engine main state
EMOVIEMODE movieMode = MOVIEMODE_INACTIVE;

//this should not be set unless we are in MOVIEMODE_RECORD!
//FILE* fpRecordingMovie = 0;
EMUFILE* osRecordingMovie = NULL;

int currFrameCounter;
uint32 cur_input_display = 0;
int pauseframe = -1;
bool movie_readonly = true;
int input_display = 0;
int frame_display = 0;
int rerecord_display = 0;
bool fullSaveStateLoads = false;	//Option for loading a savestates full contents in read+write mode instead of up to the frame count in the savestate (useful as a recovery option)
int movieRecordMode = 0;			//Option for various movie recording modes such as TRUNCATE (normal), OVERWRITE etc.

std::string curMovieFilename;
MovieData currMovieData;
MovieData defaultMovieData;
int currRerecordCount; // Keep the global value

// See master migration note at the top of this file for the full
// Phase B / Batch D-D.X carve-out map. (The mid-file inline markers
// for Batches C, D-D.1..D-D.6 were consolidated into that single
// block.) The remaining markers below point at the latest Phase B
// state changes that don't yet have a fresh commit annotation line.

// v1.13 Phase B / Batch D-D.2: openRecordingMovie / closeRecordingMovie /
// RedumpWholeMovieFile / StopPlayback / FinishPlayback / StopRecording /
// OnMovieClosed relocated to src/movie_record.cpp (recording-session
// IO + playback-lifecycle). The cross-TU decls already in
// movie_record.cpp:43-45 (RedumpWholeMovieFile / OnMovieClosed /
// GetMovieModeStr) become redundant since the definitions now live in
// the same TU; they are pruned there as part of this commit.
// movie_playback.cpp and movie.cpp add equivalent forward decls.

bool bogorf;

void fceu11::StopMovie()
{
	if (suppressMovieStop)
		return;

	if (movieMode == MOVIEMODE_PLAY || movieMode == MOVIEMODE_FINISHED)
		StopPlayback();
	else if (movieMode == MOVIEMODE_RECORD)
		StopRecording();

	OnMovieClosed();
}

// v1.13 Phase B / Batch D-D.5: poweron body relocated to src/movie_io.cpp
// (see that file for the definition; the booger-flag dance around
// disableBatteryLoading stays accessible via fceu11::SaveMovie's
// cross-TU extern declared at the top of movie_io.cpp).

// v1.13 Phase B / Batch D-D.5: FCEUMOV_CreateCleanMovie /
// FCEUMOV_ClearCommands / FCEUMOV_FromPoweron relocated to
// src/movie_io.cpp.

// v1.13 Phase B / Batch C: MovieData::loadSavestateFrom + dumpSavestateTo
// bodies moved to movie_playback.cpp (Batch C); loadSaveramFrom /
// dumpSaveramTo / fceu11::LoadMovie / SaveMovie bodies relocated to
// src/movie_io.cpp (Batch D-D.5).


//the main interaction point between the emulator and the movie system.
//either dumps the current joystick state or loads one state from the movie
// v1.13 Phase B / Batch D-D.4: the TASEDITOR branch (uses
// g_tas_bridge function-pointer dispatch from Batch B) is now in
// movie_taseditor_bridge.cpp; the RECORD branch is in movie_record.cpp;
// the PLAY branch stays here as a thin inline dispatch. FCEUMOV_AddInputState
// itself becomes a thin per-frame router.

void MovieAddInputState_Playback();

// fceu11::SetVidSystem (loader-side) is not needed here; the bridge /
// playback / record helpers each contain their own MovieReplayEmulatedCommands
// or hot-path equivalent. The current dispatcher is intentionally tiny —
// keeping FCEUMOV_AddInputState() in TU-local switch keeps the per-frame
// call site readable for the regression suite.
void FCEUMOV_AddInputState()
{
	if (movieMode == MOVIEMODE_TASEDITOR)
		MovieAddInputState_TasEditor();
	else if (movieMode == MOVIEMODE_PLAY)
		MovieAddInputState_Playback();
	else if (movieMode == MOVIEMODE_RECORD)
		MovieAddInputState_Record();

	currFrameCounter++;

	extern uint8 joy[4];
	memcpy(&cur_input_display,joy,4);
}

// v1.13 Phase B / Batch D-D.4: PLAY branch kept here. The TASEDITOR
// branch (with g_tas_bridge) is in movie_taseditor_bridge.cpp; the
// v1.13 Phase B / Batch D-D.4-extension: MovieAddInputState_Playback
// relocated to src/movie_settings.cpp (small public-API TU). The main
// dispatcher below routes PLAY there.