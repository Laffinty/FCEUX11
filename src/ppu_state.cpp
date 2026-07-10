// ppu_state.cpp
//
// v1.12 Scissors Phase E-A: PPU savestate bookkeeping split.
//
// Pure code move from src/ppu.cpp lines 1976-2038. See ppu_state.h.

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
#include "ppu.h"
#include "ppu_state.h"
#include "state.h"
#include "utils/memory.h"

#include <cstdint>

// ----------------------------------------------------------------------------
// Savestate scratch addresses.
//
// These mirror `TempAddr` / `RefreshAddr` (declared as reference aliases in
// ppu_class.h) at the moment of FCEUPPU_SaveState and restore them on
// FCEUPPU_LoadState. They live here in ppu_state.cpp because the only
// sites that touch them are the Save/LoadState hooks in this file.
// ----------------------------------------------------------------------------
static uint16 TempAddrT, RefreshAddrT;

void FCEUPPU_LoadState(int version) {
	TempAddr = TempAddrT;
	RefreshAddr = RefreshAddrT;
}

SFORMAT FCEUPPU_STATEINFO[] = {
	{ NTARAM, 0x800, "NTAR" },
	{ PALRAM.data(), 0x20, "PRAM" },
	{ SPRAM, 0x100, "SPRA" },
	{ PPU, 0x4, "PPUR" },
	{ &kook, 1, "KOOK" },
	{ &ppudead, 1, "DEAD" },
	{ &PPUSPL, 1, "PSPL" },
	{ &XOffset, 1, "XOFF" },
	{ &vtoggle, 1, "VTGL" },
	{ &RefreshAddrT, 2 | FCEUSTATE_RLSB, "RADD" },
	{ &TempAddrT, 2 | FCEUSTATE_RLSB, "TADD" },
	{ &VRAMBuffer, 1, "VBUF" },
	{ &PPUGenLatch, 1, "PGEN" },
	{ 0 }
};

SFORMAT FCEU_NEWPPU_STATEINFO[] = {
	{ &idleSynch, 1, "IDLS" },
	{ &spr_read.num, 4 | FCEUSTATE_RLSB, "SR_0" },
	{ &spr_read.count, 4 | FCEUSTATE_RLSB, "SR_1" },
	{ &spr_read.fetch, 4 | FCEUSTATE_RLSB, "SR_2" },
	{ &spr_read.found, 4 | FCEUSTATE_RLSB, "SR_3" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx0" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx1" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx2" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx3" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx4" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx5" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx6" },
	{ &spr_read.found_pos[0], 4 | FCEUSTATE_RLSB, "SRx7" },
	{ &spr_read.ret, 4 | FCEUSTATE_RLSB, "SR_4" },
	{ &spr_read.last, 4 | FCEUSTATE_RLSB, "SR_5" },
	{ &spr_read.mode, 4 | FCEUSTATE_RLSB, "SR_6" },
	{ &ppur.fv, 4 | FCEUSTATE_RLSB, "PFVx" },
	{ &ppur.v, 4 | FCEUSTATE_RLSB, "PVxx" },
	{ &ppur.h, 4 | FCEUSTATE_RLSB, "PHxx" },
	{ &ppur.vt, 4 | FCEUSTATE_RLSB, "PVTx" },
	{ &ppur.ht, 4 | FCEUSTATE_RLSB, "PHTx" },
	{ &ppur._fv, 4 | FCEUSTATE_RLSB, "P_FV" },
	{ &ppur._v, 4 | FCEUSTATE_RLSB, "P_Vx" },
	{ &ppur._h, 4 | FCEUSTATE_RLSB, "P_Hx" },
	{ &ppur._vt, 4 | FCEUSTATE_RLSB, "P_VT" },
	{ &ppur._ht, 4 | FCEUSTATE_RLSB, "P_HT" },
	{ &ppur.fh, 4 | FCEUSTATE_RLSB, "PFHx" },
	{ &ppur.s, 4 | FCEUSTATE_RLSB, "PSxx" },
	{ &ppur.status.sl, 4 | FCEUSTATE_RLSB, "PST0" },
	{ &ppur.status.cycle, 4 | FCEUSTATE_RLSB, "PST1" },
	{ &ppur.status.end_cycle, 4 | FCEUSTATE_RLSB, "PST2" },
	{ 0 }
};

void FCEUPPU_SaveState(void) {
	TempAddrT = TempAddr;
	RefreshAddrT = RefreshAddr;
}