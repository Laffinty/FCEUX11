// ppu_core.cpp
//
// v1.12 Scissors Phase E-B (scope v1): PPU lifecycle + accessor split.
//
// Pure code move from src/ppu.cpp — lines 295-301, 369-370, 1757-1828,
// 2040-2048. See ppu_core.h.

/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 1998 BERO
 *  Copyright (C) 2003 Xodnizel
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

#include "types.h"
#include "fceu.h"
#include "ppu.h"
#include "ppu_core.h"

// ----------------------------------------------------------------------------
// Accessor helpers. Pure read-only wrappers around the new-PPU state
// register (ppur). Return the current scanline and dot counter so
// external callers (debugger, frame limiter) can introspect timing.
// ----------------------------------------------------------------------------

int newppu_get_scanline() { return ppur.status.sl; }
int newppu_get_dot() { return ppur.status.cycle; }
void newppu_hacky_emergency_reset()
{
	if(ppur.status.end_cycle == 0)
		ppur.reset();
}

// ----------------------------------------------------------------------------
// PPU hook pointers.
//
// These are the "raster / sprite-overflow" callbacks invoked from the
// mapper scanline hooks (GameHBIRQHook/2) and the per-dot PPU trace
// (PPU_hook). The defaults are NULL; mmc5.cpp and other board code
// assign them at reset time. Definitions live here because the only
// writers are PPU_ResetHooks (this TU) and the board-side assignments.
// ----------------------------------------------------------------------------

void (*GameHBIRQHook)(void), (*GameHBIRQHook2)(void);
void (*PPU_hook)(uint32 A);

// ----------------------------------------------------------------------------
// PPU timing / scanline configuration.
//
// FCEUPPU_SetVideoSystem adjusts the frame height for NTSC/PAL/Dendy
// modes. scanlines_per_frame is exported (declared extern in ppu_core.h)
// because the main scanline loops in ppu.cpp read it once per frame.
// ----------------------------------------------------------------------------

void FCEUPPU_SetVideoSystem(int w) {
	if (w) {
		scanlines_per_frame = dendy ? 262: 312;
		FSettings.FirstSLine = FSettings.UsrFirstSLine[1];
		FSettings.LastSLine = FSettings.UsrLastSLine[1];
		//paldeemphswap = 1; // dendy has pal ppu, and pal ppu has these swapped
	} else {
		scanlines_per_frame = 262;
		FSettings.FirstSLine = FSettings.UsrFirstSLine[0];
		FSettings.LastSLine = FSettings.UsrLastSLine[0];
		//paldeemphswap = 0;
	}
}

void PPU_ResetHooks() {
	FFCEUX_PPURead = FFCEUX_PPURead_Default;
}

void FCEUPPU_Reset(void) {
	VRAMBuffer = PPU[0] = PPU[1] = PPU_status = PPU[3] = 0;
	PPUSPL = 0;
	PPUGenLatch = 0;
	RefreshAddr = TempAddr = 0;
	vtoggle = 0;
	ppudead = 2;
	kook = 0;
	idleSynch = 1;

	new_ppu_reset = true; // delay reset of ppur/spr_read until it's ready to start a new frame
}

// NOTE: FCEUPPU_Power stays in ppu.cpp. It assigns file-static
// DECLFR/DECLFW function pointers (A2002/A200x/B2000/B2001/.../B4014)
// into the ARead[]/BWrite[] memory-read/write tables; those static
// register handlers don't move until the register-port half of the
// split (a follow-up batch). The Power hook's other side effects
// (FCEU_MemoryRand, FCEUPPU_Reset) are visible cross-TU.

// ----------------------------------------------------------------------------
// FCEUPPU_PeekAddress — read the current PPU address without side effects.
//
// Used by the debugger (debug.cpp) and the cheat/trace tools to peek at
// the next address the PPU will read or write. When newppu is active,
// the address is derived from ppur's state-register counters; otherwise
// it's the legacy RefreshAddr latch.
// ----------------------------------------------------------------------------

uint32 FCEUPPU_PeekAddress()
{
	if (newppu)
	{
		return ppur.get_2007access() & 0x3FFF;
	}

	return RefreshAddr & 0x3FFF;
}