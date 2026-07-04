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

// v0.3.10 P4.1 — Core lifecycle / NES control APIs migrate into the
// `fceu11::` namespace per plan v3 §5 v0.3.10. The global `FCEUI_*` names
// are retained as inline function-reference aliases for source-level
// compatibility (every existing caller compiles unchanged). Definitions
// in `fceu.cpp` / `input.cpp` / `x6502.cpp` are rewritten as
// `fceu11::X(...)`; the aliases below pin the legacy `FCEUI_*` symbol so
// existing call sites still resolve. Removal of the aliases is scheduled
// for v0.4.0.
namespace fceu11 {
    // Game lifecycle: the five "user always calls these" entry points.
    FCEUGI *LoadGame(const char *name, int OverwriteVidMode, bool silent = false);
    // Same as LoadGame, but can load from a tempfile inside an archive.
    // `name` is the logical path to open; `archiveFilename` is the archive
    // which contains `name`.
    FCEUGI *LoadGameVirtual(const char *name, int OverwriteVidMode, bool silent = false);
    bool Initialize();
    void Emulate(uint8 **xbuf, int32 **sbuf, int32 *ssize, int skip);
    void CloseGame();
    // Deallocates all allocated memory. Call after Emulate() returns.
    void Kill();

    // NES-level control. Emulator "soft" reset and power-cycle buttons.
    void ResetNES();
    void PowerNES();
    // CPU interrupt injection — used by the debugger path to step over BRK.
    void NMI();
    void IRQ();
} // namespace fceu11

inline FCEUGI *FCEUI_LoadGame(const char *name, int OverwriteVidMode, bool silent = false) { return fceu11::LoadGame(name, OverwriteVidMode, silent); }
inline FCEUGI *FCEUI_LoadGameVirtual(const char *name, int OverwriteVidMode, bool silent = false) { return fceu11::LoadGameVirtual(name, OverwriteVidMode, silent); }
inline bool FCEUI_Initialize() { return fceu11::Initialize(); }
inline void FCEUI_Emulate(uint8 **xbuf, int32 **sbuf, int32 *ssize, int skip) { fceu11::Emulate(xbuf, sbuf, ssize, skip); }
inline void FCEUI_CloseGame() { fceu11::CloseGame(); }
inline void FCEUI_Kill() { fceu11::Kill(); }
inline void FCEUI_ResetNES() { fceu11::ResetNES(); }
inline void FCEUI_PowerNES() { fceu11::PowerNES(); }
inline void FCEUI_NMI() { fceu11::NMI(); }
inline void FCEUI_IRQ() { fceu11::IRQ(); }

// CPU/PPU debug-time memory ops.
void FCEUI_MemDump(uint16 a, int32 len, void (*callb)(uint16 a, uint8 v));
uint8 FCEUI_MemSafePeek(uint16 A);
void FCEUI_MemPoke(uint16 a, uint8 v, int hl);
uint16 FCEUI_Disassemble(void *XA, uint16 a, char *stringo);
void FCEUI_GetIVectors(uint16 *reset, uint16 *irq, uint16 *nmi);
uint32 FCEUI_CRC32(uint32 crc, uint8 *buf, uint32 len);

// v0.3.10 P4.1 (continued) — video region / state / snapshot / pause /
// misc / NSF / VSUni / FDS / Datach surfaces also migrate into the
// `fceu11::` namespace. Pattern as above: definitions move to
// `fceu11::Foo(...)`; the legacy `FCEUI_Foo` symbols are preserved via
// the inline reference aliases at the end of this block.
namespace fceu11 {
    // Video system (region / vsystem / scanline window).
    void SetVidSystem(int a);
    // `region`: 0 = NTSC, 1 = PAL, 2 = Dendy.
    void SetRegion(int region, int notify = 1);
    int  GetRegion();
    // Returns currently emulated video system (0=NTSC, 1=PAL).
    int  GetCurrentVidSystem(int *slstart, int *slend);
    // First and last scanlines to render, for ntsc and pal emulation.
    void SetRenderedLines(int ntscf, int ntscl, int palf, int pall);

    // State (savestate) save / load / select-slot.
    void SaveStateFile(const char *fname, bool display_message = true);
    void LoadStateFile(const char *fname, bool display_message = true);
    int  SelectStateSlot(int w, int show);
    void SelectStateNext(int n);

    // Snapshot (screen image) save.
    int32 GetDesiredFPS();
    void  SaveSnapshot();
    void  SaveSnapshotAs();

    // Frame advance and pause / frame-step.
    void FrameAdvance();
    void FrameAdvanceEnd();
    int  IsEmulationPaused();
    int  EmulationFrameStepped();
    void ClearEmulationFrameStepped();
    void SetEmulationPaused(int val);
    void ToggleEmulationPause();
    void PauseForDuration(int secs);
    int  PauseFramesRemaining();

    // Misc toggles / audio filter.
    void SetGameGenie(bool a);
    void SetLowPass(int q);

    // NSF (NES Sound Format) player.
    void NSFSetVis(int mode);
    int  NSFChange(int amount);
    int  NSFGetInfo(uint8 *name, uint8 *artist, uint8 *copyright, int maxlen);

    // VS Unisystem DIP switches and coin/service.
    void  VSUniToggleDIP(int w);
    uint8 VSUniGetDIPs();
    void  VSUniSetDIP(int w, int state);
    void  VSUniCoin();
    void  VSUniCoin2();
    void  VSUniService();

    // Famicom Disk System.
    void FDSInsert();
    void FDSSelect();

    // Datach (Bandai) barcode reader.
    int DatachSet(uint8 *rcode);
} // namespace fceu11

inline void  FCEUI_SetVidSystem(int a) { fceu11::SetVidSystem(a); }
inline void  FCEUI_SetRegion(int region, int notify = 1) { fceu11::SetRegion(region, notify); }
inline int   FCEUI_GetRegion() { return fceu11::GetRegion(); }
inline int   FCEUI_GetCurrentVidSystem(int *slstart, int *slend) { return fceu11::GetCurrentVidSystem(slstart, slend); }
inline void  FCEUI_SetRenderedLines(int ntscf, int ntscl, int palf, int pall) { fceu11::SetRenderedLines(ntscf, ntscl, palf, pall); }
inline void  FCEUI_SaveState(const char *fname, bool display_message = true) { fceu11::SaveStateFile(fname, display_message); }
inline void  FCEUI_LoadState(const char *fname, bool display_message = true) { fceu11::LoadStateFile(fname, display_message); }
inline int   FCEUI_SelectState(int w, int show) { return fceu11::SelectStateSlot(w, show); }
inline void  FCEUI_SelectStateNext(int n) { fceu11::SelectStateNext(n); }
inline int32 FCEUI_GetDesiredFPS() { return fceu11::GetDesiredFPS(); }
inline void  FCEUI_SaveSnapshot() { fceu11::SaveSnapshot(); }
inline void  FCEUI_SaveSnapshotAs() { fceu11::SaveSnapshotAs(); }
inline void  FCEUI_FrameAdvance() { fceu11::FrameAdvance(); }
inline void  FCEUI_FrameAdvanceEnd() { fceu11::FrameAdvanceEnd(); }
inline int   FCEUI_EmulationPaused() { return fceu11::IsEmulationPaused(); }
inline int   FCEUI_EmulationFrameStepped() { return fceu11::EmulationFrameStepped(); }
inline void  FCEUI_ClearEmulationFrameStepped() { fceu11::ClearEmulationFrameStepped(); }
inline void  FCEUI_SetEmulationPaused(int val) { fceu11::SetEmulationPaused(val); }
inline void  FCEUI_ToggleEmulationPause() { fceu11::ToggleEmulationPause(); }
inline void  FCEUI_PauseForDuration(int secs) { fceu11::PauseForDuration(secs); }
inline int   FCEUI_PauseFramesRemaining() { return fceu11::PauseFramesRemaining(); }
inline void  FCEUI_SetGameGenie(bool a) { fceu11::SetGameGenie(a); }
inline void  FCEUI_SetLowPass(int q) { fceu11::SetLowPass(q); }
inline void  FCEUI_NSFSetVis(int mode) { fceu11::NSFSetVis(mode); }
inline int   FCEUI_NSFChange(int amount) { return fceu11::NSFChange(amount); }
inline int   FCEUI_NSFGetInfo(uint8 *name, uint8 *artist, uint8 *copyright, int maxlen) { return fceu11::NSFGetInfo(name, artist, copyright, maxlen); }
inline void  FCEUI_VSUniToggleDIP(int w) { fceu11::VSUniToggleDIP(w); }
inline uint8 FCEUI_VSUniGetDIPs() { return fceu11::VSUniGetDIPs(); }
inline void  FCEUI_VSUniSetDIP(int w, int state) { fceu11::VSUniSetDIP(w, state); }
inline void  FCEUI_VSUniCoin() { fceu11::VSUniCoin(); }
inline void  FCEUI_VSUniCoin2() { fceu11::VSUniCoin2(); }
inline void  FCEUI_VSUniService() { fceu11::VSUniService(); }
inline void  FCEUI_FDSInsert() { fceu11::FDSInsert(); }
inline void  FCEUI_FDSSelect() { fceu11::FDSSelect(); }
inline int   FCEUI_DatachSet(uint8 *rcode) { return fceu11::DatachSet(rcode); }

#ifdef FRAMESKIP
/* Specifies how many frames to skip until the next frame is rendered. */
void FCEUI_FrameSkip(int x);
#endif

// Display message (on-screen OSD, e.g. "State 3 saved").
void FCEU_DispMessage( __FCEU_PRINTF_FORMAT const char *format, int disppos, ...) __FCEU_PRINTF_ATTRIBUTE( 1, 3 );
#define FCEUI_DispMessage FCEU_DispMessage

// Displayer-agnostic error / log hooks. Called by the core; the driver
// (Qt, SDL, …) provides the implementation.
void FCEUD_PrintError(const char *s);
void FCEUD_Message(const char *s);

// Cheat engine (Game Genie / Pro Action Rocky) — the core owns the
// decoder and storage; the UI is in drivers/Qt.

// VS Unisystem DIP-switch UI view toggle (no definition in the v0.3.x
// tree — historical declaration kept for the Qt input.cpp commented-out
// stub at line ~1143).
void FCEUI_VSUniToggleDIPView(void);

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

// Debugger hook surface — the driver wires these to the UI's
// breakpoint / trace / nametable viewers.
void FCEUD_DebugBreakpoint(int bp_num);
void FCEUD_TraceInstruction(uint8 *opcode, int size);
void FCEUD_FlushTrace();
void FCEUD_UpdateNTView(int scanline, bool drawall);
void FCEUD_UpdatePPUView(int scanline, int drawall);
bool FCEUD_PauseAfterPlayback();
void FCEUD_VideoChanged();
uint64 FCEUD_GetTime(void);
uint64 FCEUD_GetTimeFreq(void);

// Lua TAS editor bridge. Only declared when the Lua engine has been
// included in this translation unit (the s9xlua.h header is the
// canonical sentinel — same convention as the pre-split driver.h).
#ifdef _S9XLUA_H
void TaseditorAutoFunction(void);
void TaseditorManualFunction(void);
#endif

namespace fceu11 {
    int DecodePAR(const char *code, int *a, int *v, int *c, int *type);
    int DecodeGG(const char *str, int *a, int *v, int *c);
    int AddCheat(const char *name, uint32 addr, uint8 val, int compare, int type);
    int DelCheat(uint32 which);
    int ToggleCheat(uint32 which);
    int GlobalToggleCheat(int global_enable);
    int32 CheatSearchGetCount(void);
    void CheatSearchGetRange(uint32 first, uint32 last, int (*callb)(uint32 a, uint8 last, uint8 current));
    void CheatSearchGet(int (*callb)(uint32 a, uint8 last, uint8 current, void *data), void *data);
    void CheatSearchBegin(void);
    void CheatSearchEnd(int type, uint8 v1, uint8 v2);
    void ListCheats(int (*callb)(const char *name, uint32 a, uint8 v, int compare, int s, int type, void *data), void *data);
    int GetCheat(uint32 which, std::string *name, uint32 *a, uint8 *v, int *compare, int *s, int *type);
    int SetCheat(uint32 which, const std::string *name, int32 a, int32 v, int compare, int s, int type);
    void CheatSearchShowExcluded(void);
    void CheatSearchSetCurrentAsOriginal(void);
} // namespace fceu11

inline int FCEUI_DecodePAR(const char *code, int *a, int *v, int *c, int *type) { return fceu11::DecodePAR(code, a, v, c, type); }
inline int FCEUI_DecodeGG(const char *str, int *a, int *v, int *c) { return fceu11::DecodeGG(str, a, v, c); }
inline int FCEUI_AddCheat(const char *name, uint32 addr, uint8 val, int compare, int type) { return fceu11::AddCheat(name, addr, val, compare, type); }
inline int FCEUI_DelCheat(uint32 which) { return fceu11::DelCheat(which); }
inline int FCEUI_ToggleCheat(uint32 which) { return fceu11::ToggleCheat(which); }
inline int FCEUI_GlobalToggleCheat(int global_enable) { return fceu11::GlobalToggleCheat(global_enable); }
inline int32 FCEUI_CheatSearchGetCount(void) { return fceu11::CheatSearchGetCount(); }
inline void FCEUI_CheatSearchGetRange(uint32 first, uint32 last, int (*callb)(uint32 a, uint8 last, uint8 current)) { fceu11::CheatSearchGetRange(first, last, callb); }
inline void FCEUI_CheatSearchGet(int (*callb)(uint32 a, uint8 last, uint8 current, void *data), void *data) { fceu11::CheatSearchGet(callb, data); }
inline void FCEUI_CheatSearchBegin(void) { fceu11::CheatSearchBegin(); }
inline void FCEUI_CheatSearchEnd(int type, uint8 v1, uint8 v2) { fceu11::CheatSearchEnd(type, v1, v2); }
inline void FCEUI_ListCheats(int (*callb)(const char *name, uint32 a, uint8 v, int compare, int s, int type, void *data), void *data) { fceu11::ListCheats(callb, data); }
inline int FCEUI_GetCheat(uint32 which, std::string *name, uint32 *a, uint8 *v, int *compare, int *s, int *type) { return fceu11::GetCheat(which, name, a, v, compare, s, type); }
inline int FCEUI_SetCheat(uint32 which, const std::string *name, int32 a, int32 v, int compare, int s, int type) { return fceu11::SetCheat(which, name, a, v, compare, s, type); }
inline void FCEUI_CheatSearchShowExcluded(void) { fceu11::CheatSearchShowExcluded(); }
inline void FCEUI_CheatSearchSetCurrentAsOriginal(void) { fceu11::CheatSearchSetCurrentAsOriginal(); }

#endif //__FCEU_CORE_API_H_
