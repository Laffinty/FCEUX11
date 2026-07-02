/* FCE Ultra - NES/Famicom Emulator
*
* Copyright notice for this file:
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

/// \file
/// \brief NSF player bridge layer — v1.10 Cryptex.
/// Runtime handlers live in nsf_runtime.cpp; UI in nsf_ui.cpp; loader in nsf_load.cpp.

#include "types.h"
#include "utils/safe_string.h"
#include "fceu.h"
#include "nsf.h"
#include "ines.h"
#include "utils/memory.h"
#include "file.h"
#include "driver.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>

// ── Global configuration ─────────────────────────────────────────────────
uint8 *NSFDATA = 0;
int NSFMaxBank;
int32 NSFSize;
uint8 BSon;
uint16 PlayAddr;
uint16 InitAddr;
uint16 LoadAddr;
NSF_HEADER NSFHeader;

// ── Forward declarations from nsf_runtime.cpp ────────────────────────────
extern void NSF_init(void);
extern void DoNSFFrame(void);
extern void nsf_allocate_exwram(void);
extern void nsf_free_exwram(void);
extern void nsf_runtime_create(void);
extern void nsf_runtime_destroy(void);
extern uint8 SongReload;
extern int32 CurrentSong;

// ── Forward declarations from nsf_ui.cpp ─────────────────────────────────
extern void DrawNSF(uint8 *XBuf);

// ── Forward declarations for sound chip cleanup ──────────────────────────
void NSFMMC5_Close(void);

// ── Game interface (lifecycle) ───────────────────────────────────────────
void NSFGI(GI h) {
	switch (h) {
	case GI_CLOSE:
		nsf_runtime_destroy();
		if (NSFDATA) { free(NSFDATA); NSFDATA = 0; }
		nsf_free_exwram();
		if (NSFHeader.SoundChip & 1) { /* NSFVRC6_Init(); */ }
		else if (NSFHeader.SoundChip & 2) { /* NSFVRC7_Init(); */ }
		else if (NSFHeader.SoundChip & 4) { /* FDSSoundReset(); */ }
		else if (NSFHeader.SoundChip & 8) { NSFMMC5_Close(); }
		else if (NSFHeader.SoundChip & 0x10) { /* NSFN106_Init(); */ }
		else if (NSFHeader.SoundChip & 0x20) { /* NSFAY_Init(); */ }
		break;
	case GI_RESETM2:
	case GI_POWER:
		NSF_init();
		break;
	}
}

// ── NSFLoad (thin wrapper around NSFLoadCore in nsf_load.cpp) ────────────
extern int NSFLoadCore(const char *name, FCEUFILE *fp);

int NSFLoad(const char *name, FCEUFILE *fp) {
	FCEU_fseek(fp, 0, SEEK_SET);
	int result = NSFLoadCore(name, fp);
	if (result != LOADER_OK) return result;

	GameInterface = NSFGI;
	FCEU_strlcpy(LoadedRomFName, sizeof(LoadedRomFName), name);
	nsf_allocate_exwram();
	nsf_runtime_create();
	return LOADER_OK;
}

// ── Thin wrappers (Rust FFI) ─────────────────────────────────────────────
int fceu11::NSFChange(int amount) {
	return fceux11_rust_nsf_change_song(::CurrentSong, amount, NSFHeader.TotalSongs, &::SongReload);
}

int fceu11::NSFGetInfo(uint8 *name, uint8 *artist, uint8 *copyright, int maxlen) {
	return fceux11_rust_nsf_get_info((FceuNsfHeader*)&NSFHeader, name, artist, copyright, maxlen);
}
