/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2001 Aaron Oneal
 *  Copyright (C) 2002 Xodnizel
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

// FCEUX11 v0.3.9 — core_api.h (physical split of driver.h per plan
// v3 §5 v0.3.9). This header owns the emulator's core lifecycle, state,
// frame, and control surface. It depends on `types.h`, `git.h`, and
// `fceu11_core_types.h` (for the printf format macros from utils/format.h,
// transitively pulled in by types.h).
//
// The legacy `src/driver.h` becomes a thin shim that re-includes this
// and the three siblings (io_api.h / net_api.h / diag_api.h) so the 33
// existing #include "driver.h" call sites compile without source edits.

#ifndef __FCEU_CORE_API_H_
#define __FCEU_CORE_API_H_

#include "types.h"
#include "git.h"

#include <cstdio>

// Format-string attribute macros come in via types.h -> utils/format.h.
// FCEU_printf/FCEUI_DispMessage are part of the core messaging surface
// (used by every subsystem) so they live here, not in diag_api.h.
void FCEU_printf( __FCEU_PRINTF_FORMAT const char *format, ...)  __FCEU_PRINTF_ATTRIBUTE( 1, 2 );
#define FCEUI_printf FCEU_printf

// Game lifecycle: the five "user always calls these" entry points.
FCEUGI *FCEUI_LoadGame(const char *name, int OverwriteVidMode, bool silent = false);
// Same as FCEUI_LoadGame, but can load from a tempfile inside an archive.
// `name` is the logical path to open; `archiveFilename` is the archive
// which contains `name`.
FCEUGI *FCEUI_LoadGameVirtual(const char *name, int OverwriteVidMode, bool silent = false);
bool FCEUI_Initialize();
void FCEUI_Emulate(uint8 **, int32 **, int32 *, int);
void FCEUI_CloseGame(void);
// Deallocates all allocated memory. Call after FCEUI_Emulate() returns.
void FCEUI_Kill(void);

// NES-level control. Emulator "soft" reset and power-cycle buttons.
void FCEUI_ResetNES(void);
void FCEUI_PowerNES(void);
// CPU interrupt injection — used by the debugger path to step over BRK.
void FCEUI_NMI(void);
void FCEUI_IRQ(void);

// CPU/PPU debug-time memory ops.
void FCEUI_MemDump(uint16 a, int32 len, void (*callb)(uint16 a, uint8 v));
uint8 FCEUI_MemSafePeek(uint16 A);
void FCEUI_MemPoke(uint16 a, uint8 v, int hl);
uint16 FCEUI_Disassemble(void *XA, uint16 a, char *stringo);
void FCEUI_GetIVectors(uint16 *reset, uint16 *irq, uint16 *nmi);
uint32 FCEUI_CRC32(uint32 crc, uint8 *buf, uint32 len);

// Video system (region / vsystem / scanline window).
void FCEUI_SetVidSystem(int a);
// `region`: 0 = NTSC, 1 = PAL, 2 = Dendy.
void FCEUI_SetRegion(int region, int notify = 1);
int  FCEUI_GetRegion(void);
// Returns currently emulated video system (0=NTSC, 1=PAL).
int FCEUI_GetCurrentVidSystem(int *slstart, int *slend);
// First and last scanlines to render, for ntsc and pal emulation.
void FCEUI_SetRenderedLines(int ntscf, int ntscl, int palf, int pall);

// State (savestate) save / load / select-slot.
void FCEUI_SaveState(const char *fname, bool display_message=true);
void FCEUI_LoadState(const char *fname, bool display_message=true);
int  FCEUI_SelectState(int, int);
extern void FCEUI_SelectStateNext(int);

// Snapshot (screen image) save.
int32 FCEUI_GetDesiredFPS(void);
void FCEUI_SaveSnapshot(void);
void FCEUI_SaveSnapshotAs(void);

// Frame advance and pause / frame-step.
void FCEUI_FrameAdvance(void);
void FCEUI_FrameAdvanceEnd(void);
int  FCEUI_EmulationPaused();
int  FCEUI_EmulationFrameStepped();
void FCEUI_ClearEmulationFrameStepped();
void FCEUI_SetEmulationPaused(int val);
void FCEUI_ToggleEmulationPause();
void FCEUI_PauseForDuration(int secs);
int  FCEUI_PauseFramesRemaining();

#ifdef FRAMESKIP
/* Should be called from FCEUD_BlitScreen(). Specifies how many frames
   to skip until FCEUD_BlitScreen() is called. FCEUD_BlitScreenDummy()
   will be called instead of FCEUD_BlitScreen() when a frame is skipped. */
void FCEUI_FrameSkip(int x);
#endif

// Enable / disable Game Genie decoder.
void FCEUI_SetGameGenie(bool a);

// Display message (on-screen OSD, e.g. "State 3 saved").
void FCEU_DispMessage( __FCEU_PRINTF_FORMAT const char *format, int disppos, ...) __FCEU_PRINTF_ATTRIBUTE( 1, 3 );
#define FCEUI_DispMessage FCEU_DispMessage

// Displayer-agnostic error / log hooks. Called by the core; the driver
// (Qt, SDL, …) provides the implementation.
void FCEUD_PrintError(const char *s);
void FCEUD_Message(const char *s);

// Cheat engine (Game Genie / Pro Action Rocky) — the core owns the
// decoder and storage; the UI is in drivers/Qt.
int FCEUI_DecodePAR(const char *code, int *a, int *v, int *c, int *type);
int FCEUI_DecodeGG(const char *str, int *a, int *v, int *c);
int FCEUI_AddCheat(const char *name, uint32 addr, uint8 val, int compare, int type);
int FCEUI_DelCheat(uint32 which);
int FCEUI_ToggleCheat(uint32 which);
int FCEUI_GlobalToggleCheat(int global_enable);
int32 FCEUI_CheatSearchGetCount(void);
void FCEUI_CheatSearchGetRange(uint32 first, uint32 last, int (*callb)(uint32 a, uint8 last, uint8 current));
void FCEUI_CheatSearchGet(int (*callb)(uint32 a, uint8 last, uint8 current, void *data), void *data);
void FCEUI_CheatSearchBegin(void);
void FCEUI_CheatSearchEnd(int type, uint8 v1, uint8 v2);
void FCEUI_ListCheats(int (*callb)(const char *name, uint32 a, uint8 v, int compare, int s, int type, void *data), void *data);
int  FCEUI_GetCheat(uint32 which, std::string *name, uint32 *a, uint8 *v, int *compare, int *s, int *type);
int  FCEUI_SetCheat(uint32 which, const std::string *name, int32 a, int32 v, int compare, int s, int type);
void FCEUI_CheatSearchShowExcluded(void);
void FCEUI_CheatSearchSetCurrentAsOriginal(void);

// Audio filter.
void FCEUI_SetLowPass(int q);

// NSF (NES Sound Format) player.
void FCEUI_NSFSetVis(int mode);
int  FCEUI_NSFChange(int amount);
int  FCEUI_NSFGetInfo(uint8 *name, uint8 *artist, uint8 *copyright, int maxlen);

// VS Unisystem DIP switches and coin/service.
void FCEUI_VSUniToggleDIPView(void);
void FCEUI_VSUniToggleDIP(int w);
uint8 FCEUI_VSUniGetDIPs(void);
void FCEUI_VSUniSetDIP(int w, int state);
void FCEUI_VSUniCoin(void);
void FCEUI_VSUniCoin2(void);
void FCEUI_VSUniService(void);

// Famicom Disk System.
void FCEUI_FDSInsert(void);
void FCEUI_FDSSelect(void);

// Datach (Bandai) barcode reader.
int FCEUI_DatachSet(uint8 *rcode);

// Emulator command dispatcher — driver supplies a poll callback that
// returns 1 if the named command is currently "pressed".
typedef int TestCommandState(int cmd);
void FCEUI_HandleEmuCommands(TestCommandState* testfn);

// EFCEUI is the canonical "what command did the user just trigger" enum,
// the same namespace used by TestCommandState::cmd above.
enum EFCEUI
{
	FCEUI_STOPAVI, FCEUI_QUICKSAVE, FCEUI_QUICKLOAD, FCEUI_SAVESTATE, FCEUI_LOADSTATE,
	FCEUI_NEXTSAVESTATE,FCEUI_PREVIOUSSAVESTATE,FCEUI_VIEWSLOTS,
	FCEUI_STOPMOVIE, FCEUI_RECORDMOVIE, FCEUI_PLAYMOVIE,
	FCEUI_OPENGAME, FCEUI_CLOSEGAME,
	FCEUI_TASEDITOR,
	FCEUI_RESET, FCEUI_POWER, FCEUI_PLAYFROMBEGINNING, FCEUI_EJECT_DISK, FCEUI_SWITCH_DISK, FCEUI_INSERT_COIN, FCEUI_INPUT_BARCODE,
	FCEUI_TOGGLERECORDINGMOVIE, FCEUI_TRUNCATEMOVIE, FCEUI_INSERT1FRAME, FCEUI_DELETE1FRAME
};

// Checks whether an EFCEUI is valid right now (e.g. "save state" is
// invalid when no game is loaded).
bool FCEU_IsValidUI(EFCEUI ui);

// Emulation speed (slowest..fastest) and turbo.
enum EMUSPEED_SET
{
	EMUSPEED_SLOWEST=0,
	EMUSPEED_SLOWER,
	EMUSPEED_NORMAL,
	EMUSPEED_FASTER,
	EMUSPEED_FASTEST
};
void FCEUD_SetEmulationSpeed(int cmd);
void FCEUD_TurboOn(void);
void FCEUD_TurboOff(void);
void FCEUD_TurboToggle(void);

// Status icon / menu visibility toggles.
int  FCEUD_ShowStatusIcon(void);
void FCEUD_ToggleStatusIcon(void);
void FCEUD_HideMenuToggle(void);

/// signals the driver to perform a file open GUI operation
void FCEUD_CmdOpen(void);

/// called when the emulator closes a game
void FCEUD_OnCloseGame(void);

// Debugger hook surface — the driver wires these to the UI's
// breakpoint / trace / nametable viewers.
void FCEUD_DebugBreakpoint(int bp_num);
void FCEUD_TraceInstruction(uint8 *opcode, int size);
void FCEUD_FlushTrace();
void FCEUD_UpdateNTView(int scanline, bool drawall);
void FCEUD_UpdatePPUView(int scanline, int drawall);
bool FCEUD_PauseAfterPlayback();
void FCEUD_VideoChanged();

// Lua TAS editor bridge. Only declared when the Lua engine has been
// included in this translation unit (the s9xlua.h header is the
// canonical sentinel — same convention as the pre-split driver.h).
#ifdef _S9XLUA_H
void TaseditorAutoFunction(void);
void TaseditorManualFunction(void);
#endif

#endif //__FCEU_CORE_API_H_
