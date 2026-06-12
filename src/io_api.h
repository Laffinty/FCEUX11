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

// FCEUX11 v0.3.9 — io_api.h (physical split of driver.h per plan
// v3 §5 v0.3.9). This header owns the input / output subsystem surface:
//   - File I/O helpers (UTF-8 path handling, archive open/scan)
//   - User input devices (joypad, zapper, powerpad, Famicom expansion)
//   - Audio output (rate / volume / quality)
//   - Video output (palette, NTSC hue/tint, render-plane toggles)
//   - AVI recording
//   - Movie / Lua / savestate driver commands
//   - Directory override (IoDir)
//
// Depends on `types.h` (which transitively pulls git.h via FCEUGI::input)
// and `file.h` (for FCEUFILE / ArchiveScanRecord / EMUFILE_FILE). Notably
// does NOT depend on core_api.h, net_api.h, or diag_api.h — the four
// siblings are peer headers, not layered.

#ifndef __FCEU_IO_API_H_
#define __FCEU_IO_API_H_

#include "types.h"
#include "git.h"
#include "file.h"

#include <cstdio>
#include <string>
#include <iosfwd>

// ---- File / path I/O ----------------------------------------------------
// UTF-8 fopen helpers — the standard fopen() on Windows mangles non-ASCII
// paths. The Qt driver routes these through the platform's wide-char
// APIs; the SDL driver maps them to SDL_RWops.
FILE *FCEUD_UTF8fopen(const char *fn, const char *mode);
inline FILE *FCEUD_UTF8fopen(const std::string &n, const char *mode) { return FCEUD_UTF8fopen(n.c_str(),mode); }
EMUFILE_FILE* FCEUD_UTF8_fstream(const char *n, const char *m);
inline EMUFILE_FILE* FCEUD_UTF8_fstream(const std::string &n, const char *m) { return FCEUD_UTF8_fstream(n.c_str(),m); }

// C-linkage variant used by the Rust FFI layer (fceux11-formats).
#ifdef __cplusplus
extern "C"
#endif
FILE *FCEUI_UTF8fopen_C(const char *n, const char *m);

// Archive (zip / 7z) open — used by file.cpp's ROM loader.
FCEUFILE* FCEUD_OpenArchiveIndex(ArchiveScanRecord& asr, std::string& fname, int innerIndex);
FCEUFILE* FCEUD_OpenArchiveIndex(ArchiveScanRecord& asr, std::string& fname, int innerIndex, int* userCancel);
FCEUFILE* FCEUD_OpenArchive(ArchiveScanRecord& asr, std::string& fname, std::string* innerFilename);
FCEUFILE* FCEUD_OpenArchive(ArchiveScanRecord& asr, std::string& fname, std::string* innerFilename, int* userCancel);
ArchiveScanRecord FCEUD_ScanArchive(std::string fname);

// ---- Input devices ------------------------------------------------------
// `attrib` is a per-port driver-specific cookie (e.g. poll function,
// gamepad index). The driver is responsible for retaining it.
void FCEUI_SetInput(int port, ESI type, void *ptr, int attrib);
void FCEUI_SetInputFC(ESIFC type, void *ptr, int attrib);
// Tells the emulator whether a fourscore is attached.
void FCEUI_SetInputFourscore(bool attachFourscore);
bool FCEUI_GetInputFourscore();
bool FCEUI_GetInputMicrophone();

// Apply a built-in input preset (0=gamepad, 1=zapper, …).
void FCEUI_UseInputPreset(int preset);

// Driver-side batched input update — called once per frame from the
// video/sound driver after polling hardware. `fourscore` selects the
// 4-player mode, `microphone` toggles Famicom mic capture.
void FCEUD_SetInput(bool fourscore, bool microphone, ESI port0, ESI port1, ESIFC fcexp);

// ---- NTSC hue / tint (NTSC filter UI knobs) ----------------------------
void FCEUI_NTSCSELHUE(void);
void FCEUI_NTSCSELTINT(void);
void FCEUI_NTSCDEC(void);
void FCEUI_NTSCINC(void);
void FCEUI_GetNTSCTH(int *tint, int *hue);
void FCEUI_SetNTSCTH(bool en, int tint, int hue);

// ---- Palette ------------------------------------------------------------
// Video interface — palette setters / getters.
void FCEUD_SetPalette(uint8 index, uint8 r, uint8 g, uint8 b);
void FCEUD_GetPalette(uint8 i, uint8 *r, uint8 *g, uint8 *b);
bool FCEUI_GetUserPaletteAvail(void);
void FCEUI_SetUserPalette(uint8 *pal, int nEntries);

// ---- Base directory ----------------------------------------------------
// Base directory (save states, snapshots, etc. are saved in subdirs of
// this directory). Set once at startup from the OS-specific paths.
void FCEUI_SetBaseDirectory(std::string const & dir);
const char *FCEUI_GetBaseDirectory(void);

// Directory override slot for the per-category path table indexed by
// `fceu11::IoDir` (roms / nv / states / fdsrom / snaps / cheats / movies /
// memw / bbot / macro / input / lua / avi).
//
// v0.3.8: FCEUIOD_* are now `fceu11::IoDir` enumerators. The legacy
// global names (FCEUIOD_ROMS..FCEUIOD_AVI, FCEUIOD__COUNT) are kept as
// `inline constexpr int` aliases so the 50+ array-index sites in
// src/file.cpp (e.g. `odirs[FCEUIOD_STATES]`) keep working without
// per-site `static_cast<size_t>(...)` decoration — IoDir doesn't
// implicitly convert to size_t. Removal of the int aliases is planned
// for v0.4.0; new code should use fceu11::IoDir directly together with
// an explicit cast at array boundaries.
namespace fceu11 {
    enum class IoDir : uint8_t
    {
        Roms    = 0,   // ROM files
        Nv      = 1,   // non-volatile save data
        States  = 2,   // savestates
        FdsRom  = 3,   // disksys.rom (FDS BIOS)
        Snaps   = 4,   // screenshots
        Cheats  = 5,   // cheats
        Movies  = 6,   // .fm2 files
        MemW    = 7,   // memory watch files
        BBot    = 8,   // basicbot (obsolete)
        Macro   = 9,   // old TASEdit v0.1 macro files (obsolete)
        Input   = 10,  // input presets
        Lua     = 11,  // lua scripts
        Avi     = 12,  // default file for avi output
        Count   = 13,  // sentinel (also == array length of odirs[])
    };
} // namespace fceu11

inline constexpr int FCEUIOD_ROMS    = static_cast<int>(fceu11::IoDir::Roms);
inline constexpr int FCEUIOD_NV      = static_cast<int>(fceu11::IoDir::Nv);
inline constexpr int FCEUIOD_STATES  = static_cast<int>(fceu11::IoDir::States);
inline constexpr int FCEUIOD_FDSROM  = static_cast<int>(fceu11::IoDir::FdsRom);
inline constexpr int FCEUIOD_SNAPS   = static_cast<int>(fceu11::IoDir::Snaps);
inline constexpr int FCEUIOD_CHEATS  = static_cast<int>(fceu11::IoDir::Cheats);
inline constexpr int FCEUIOD_MOVIES  = static_cast<int>(fceu11::IoDir::Movies);
inline constexpr int FCEUIOD_MEMW    = static_cast<int>(fceu11::IoDir::MemW);
inline constexpr int FCEUIOD_BBOT    = static_cast<int>(fceu11::IoDir::BBot);
inline constexpr int FCEUIOD_MACRO   = static_cast<int>(fceu11::IoDir::Macro);
inline constexpr int FCEUIOD_INPUT   = static_cast<int>(fceu11::IoDir::Input);
inline constexpr int FCEUIOD_LUA     = static_cast<int>(fceu11::IoDir::Lua);
inline constexpr int FCEUIOD_AVI     = static_cast<int>(fceu11::IoDir::Avi);
inline constexpr int FCEUIOD__COUNT  = static_cast<int>(fceu11::IoDir::Count);

void FCEUI_SetDirOverride(int which, char *n);

// ---- Audio output -------------------------------------------------------
// Sets up sound code to render sound at the specified rate, in samples
// per second. Only sample rates of 44100, 48000, and 96000 are currently
// supported. If `Rate` equals 0, sound is disabled.
void FCEUI_Sound(int Rate);
void FCEUI_SetSoundQuality(int quality);
void FCEUI_SetSoundVolume(uint32 volume);
void FCEUI_SetTriangleVolume(uint32 volume);
void FCEUI_SetSquare1Volume(uint32 volume);
void FCEUI_SetSquare2Volume(uint32 volume);
void FCEUI_SetNoiseVolume(uint32 volume);
void FCEUI_SetPCMVolume(uint32 volume);
void FCEUD_SoundToggle(void);
void FCEUD_SoundVolumeAdjust(int);

// ---- Video rendering toggles -------------------------------------------
// 0 to keep 8-sprites limitation, 1 to remove it.
void FCEUI_DisableSpriteLimitation(int a);
void FCEUI_SetRenderPlanes(bool sprites, bool bg);
void FCEUI_GetRenderPlanes(bool& sprites, bool& bg);

// Should input aids (crosshairs, lightgun reticles, etc.) be drawn?
// Used in fullscreen mode where the menu is hidden.
bool FCEUD_ShouldDrawInputAids();

// ---- Wave (audio-only) recording ---------------------------------------
bool FCEUI_BeginWaveRecord(const char *fn);
int  FCEUI_EndWaveRecord(void);

// ---- AVI recording -----------------------------------------------------
int  FCEUI_AviBegin(const char* fname);
void FCEUI_AviEnd(void);
void FCEUI_AviVideoUpdate(const unsigned char* buffer);
void FCEUI_AviSoundUpdate(void* soundData, int soundLen);
bool FCEUI_AviIsRecording();
bool FCEUI_AviEnableHUDrecording();
void FCEUI_SetAviEnableHUDrecording(bool enable);
bool FCEUI_AviDisableMovieMessages();
void FCEUI_SetAviDisableMovieMessages(bool disable);

void FCEUD_AviRecordTo(void);
void FCEUD_AviStop(void);

// ---- Driver commands (record/replay, save/load state, lua) ------------
// `fname` overrides the default save state filename code if non-NULL.
void FCEUD_SaveStateAs(void);
void FCEUD_LoadStateFrom(void);
void FCEUD_MovieRecordTo(void);
void FCEUD_MovieReplayFrom(void);
void FCEUD_LuaRunFrom(void);

#endif //__FCEU_IO_API_H_
