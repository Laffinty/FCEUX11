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
#include "ppu_bridge_state.h"  // v2.1.1.7 Step B.1: savestate staging
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

// v2.1.1.7 Step B.1 (D1-A): chunk-3 (PPUR/SPRA/PSPL/XOFF/VTGL/RADD/
// TADD/VBUF/PGEN) and chunk-31 (PST0/PST1) no longer serialise the C++
// engine globals. They serialise the bridge-owned staging block in
// ppu_bridge_state.cpp, which FCEUPPU_SaveState() below refreshes from the
// Rust PPU. NTAR / PRAM keep pointing at the C++ authoritative arrays.
// See plan §0.1 conclusion (1) and §B.1 for the D1-A design.

SFORMAT FCEUPPU_STATEINFO[] = {
	{ NTARAM, 0x800, "NTAR" },
	{ PALRAM.data(), 0x20, "PRAM" },
	{ bridge_oam, 0x100, "SPRA" },
	{ bridge_ppu_regs, 0x4, "PPUR" },
	{ &bridge_kook, 1, "KOOK" },
	{ &bridge_ppudead, 1, "DEAD" },
	{ &bridge_ppuspl, 1, "PSPL" },
	{ &bridge_xoffset, 1, "XOFF" },
	{ &bridge_vtoggle, 1, "VTGL" },
	{ &bridge_refresh_addr, 2 | FCEUSTATE_RLSB, "RADD" },
	{ &bridge_temp_addr, 2 | FCEUSTATE_RLSB, "TADD" },
	{ &bridge_vram_buffer, 1, "VBUF" },
	{ &bridge_ppu_gen_latch, 1, "PGEN" },
	{ 0 }
};

SFORMAT FCEU_NEWPPU_STATEINFO[] = {
	{ &bridge_newppu_idle_synch, 1, "IDLS" },
	{ &bridge_newppu_spr_slots[0], 4 | FCEUSTATE_RLSB, "SR_0" },
	{ &bridge_newppu_spr_slots[1], 4 | FCEUSTATE_RLSB, "SR_1" },
	{ &bridge_newppu_spr_slots[2], 4 | FCEUSTATE_RLSB, "SR_2" },
	{ &bridge_newppu_spr_slots[3], 4 | FCEUSTATE_RLSB, "SR_3" },
	// hotfix1 P0-2 (C-02): the eight SFORMAT entries for sprite positions
	// all pointed at found_pos[0], so save/load only ever persisted slot 0
	// and the remaining 7 sprites desynchronised after load. Each label
	// now maps to its own array element (labels preserved for savestate
	// compatibility with v1.15 LTS).
	{ &bridge_newppu_spr_slots[4], 4 | FCEUSTATE_RLSB, "SRx0" },
	{ &bridge_newppu_spr_slots[5], 4 | FCEUSTATE_RLSB, "SRx1" },
	{ &bridge_newppu_spr_slots[6], 4 | FCEUSTATE_RLSB, "SRx2" },
	{ &bridge_newppu_spr_slots[7], 4 | FCEUSTATE_RLSB, "SRx3" },
	{ &bridge_newppu_spr_slots[8], 4 | FCEUSTATE_RLSB, "SRx4" },
	{ &bridge_newppu_spr_slots[9], 4 | FCEUSTATE_RLSB, "SRx5" },
	{ &bridge_newppu_spr_slots[10], 4 | FCEUSTATE_RLSB, "SRx6" },
	{ &bridge_newppu_spr_slots[11], 4 | FCEUSTATE_RLSB, "SRx7" },
	{ &bridge_newppu_spr_slots[12], 4 | FCEUSTATE_RLSB, "SR_4" },
	{ &bridge_newppu_spr_slots[13], 4 | FCEUSTATE_RLSB, "SR_5" },
	{ &bridge_newppu_spr_slots[14], 4 | FCEUSTATE_RLSB, "SR_6" },
	{ &bridge_newppu_ppur_slots[0], 4 | FCEUSTATE_RLSB, "PFVx" },
	{ &bridge_newppu_ppur_slots[1], 4 | FCEUSTATE_RLSB, "PVxx" },
	{ &bridge_newppu_ppur_slots[2], 4 | FCEUSTATE_RLSB, "PHxx" },
	{ &bridge_newppu_ppur_slots[3], 4 | FCEUSTATE_RLSB, "PVTx" },
	{ &bridge_newppu_ppur_slots[4], 4 | FCEUSTATE_RLSB, "PHTx" },
	{ &bridge_newppu_ppur_slots[5], 4 | FCEUSTATE_RLSB, "P_FV" },
	{ &bridge_newppu_ppur_slots[6], 4 | FCEUSTATE_RLSB, "P_Vx" },
	{ &bridge_newppu_ppur_slots[7], 4 | FCEUSTATE_RLSB, "P_Hx" },
	{ &bridge_newppu_ppur_slots[8], 4 | FCEUSTATE_RLSB, "P_VT" },
	{ &bridge_newppu_ppur_slots[9], 4 | FCEUSTATE_RLSB, "P_HT" },
	{ &bridge_newppu_ppur_slots[10], 4 | FCEUSTATE_RLSB, "PFHx" },
	{ &bridge_newppu_ppur_slots[11], 4 | FCEUSTATE_RLSB, "PSxx" },
	{ &bridge_newppu_ppur_slots[12], 4 | FCEUSTATE_RLSB, "PST0" },
	{ &bridge_newppu_ppur_slots[13], 4 | FCEUSTATE_RLSB, "PST1" },
	{ &bridge_newppu_ppur_slots[14], 4 | FCEUSTATE_RLSB, "PST2" },
	{ 0 }
};

void FCEUPPU_SaveState(void) {
	// v2.1.1.7 Step B.1 (D1-A): pull the live Rust PPU state into the
	// bridge staging block before the SFORMAT tables serialise it.
	// TempAddrT / RefreshAddrT are no longer referenced by chunk-3; the
	// scratch copies stay until batch 4 rewires FCEUPPU_LoadState().
	bridge_state_refresh_from_rust();
	TempAddrT = TempAddr;
	RefreshAddrT = RefreshAddr;
}