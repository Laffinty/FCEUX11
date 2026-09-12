// ppu_bridge_state.h
//
// FCEUX11 v2.1.1.7 Step B.1 (D1-A) - bridge-owned savestate staging.
//
// The Rust PPU crate owns the live PPU runtime state (registers, OAM,
// the v/t address latches, fine X, the $2005/$2006 write toggle, the
// open-bus data latch, scanline/dot). The chunk-3 / chunk-31 SFORMAT
// tables keep their chunk numbers, labels, sizes and order; only the
// source of values changes: they point at the staging variables
// declared here instead of the C++ engine globals.
//
// Data flow (docs/history/v2.1.1.7_cpp_ppu_removal_archived_2026-09-12.md section B.1):
//
//   Save:  FCEUPPU_SaveState() -> bridge_state_refresh_from_rust()
//                              -> serialise staging (chunk-3 / chunk-31)
//   Load:  deserialise -> staging -> FCEUPPU_LoadState()
//                              -> bridge_state_apply_to_rust()
//                              -> refresh CHR/NT/palette windows
//                              -> push mirror mode
//
// The staging block is never read by emulation logic. It exists so the
// savestate byte layout stays parseable by older builds, and so that a
// loaded state actually restores the Rust PPU instead of re-instating a
// tombstone (the pre-v2.1.1.7 behaviour - see plan section 0.1).
//
// Each helper is one-way:
//   bridge_state_refresh_from_rust():  Rust -> staging  (save path)
//   bridge_state_apply_to_rust():      staging -> Rust  (load path)

#pragma once

#include <cstdint>

// ---------------------------------------------------------------------------
// chunk-3 staging ("FCEUPPU_STATEINFO[]" shape, see ppu_state.cpp).
//
// Names mirror the v1.0 C++ globals that used to back the SFORMAT
// entries. NTAR / PRAM are deliberately absent: those two stay bound to
// the C++ authoritative arrays (NTARAM / PALRAM) - the Rust renderer
// only holds copies of them (section 0.1).
// ---------------------------------------------------------------------------
extern uint8_t  bridge_ppu_regs[4];     // "PPUR" - $2000/$2001/$2002/$2003
extern uint8_t  bridge_oam[0x100];      // "SPRA" - primary OAM
extern int      bridge_kook;            // "KOOK" - compat only, no Rust field
extern int      bridge_ppudead;         // "DEAD" - compat only, no Rust field
extern uint8_t  bridge_ppuspl;          // "PSPL" - compat only, no Rust field
extern uint8_t  bridge_xoffset;         // "XOFF" - Rust registers.fine_x
extern uint8_t  bridge_vtoggle;         // "VTGL" - Rust registers.write_toggle
extern uint16_t bridge_refresh_addr;    // "RADD" - NES v; Rust registers.v
extern uint16_t bridge_temp_addr;       // "TADD" - NES t; Rust registers.t
extern uint8_t  bridge_vram_buffer;     // "VBUF" - Rust registers.vram_buffer
extern uint8_t  bridge_ppu_gen_latch;   // "PGEN" - Rust registers.data_bus

// ---------------------------------------------------------------------------
// chunk-31 staging ("FCEU_NEWPPU_STATEINFO[]" shape).
//
// The legacy newppu pipeline is gone, so only two of its fields still
// have a Rust counterpart: the raster position (PST0 = scanline,
// PST1 = dot). Every other slot is written as a neutral constant so the
// byte positions stay stable. Slot indices follow the SFORMAT order in
// ppu_state.cpp.
// ---------------------------------------------------------------------------
extern uint8_t bridge_newppu_idle_synch;      // "IDLS"
extern int32_t bridge_newppu_spr_slots[15];   // SR_0..SR_3, SRx0..SRx7, SR_4..SR_6
extern int32_t bridge_newppu_ppur_slots[15];  // PFVx .. PST2

// Slot indices inside bridge_newppu_ppur_slots[].
enum {
	bridge_newppu_pst0 = 12,  // "PST0" - Rust PPU scanline
	bridge_newppu_pst1 = 13,  // "PST1" - Rust PPU dot
	bridge_newppu_pst2 = 14   // "PST2" - neutral placeholder
};

// ---------------------------------------------------------------------------
// Interface
// ---------------------------------------------------------------------------

// Save path: pull the live Rust PPU state into the staging variables.
// No-op (keeping the previous staging contents) when the bridge has no
// Rust state yet, or when the FFI rejects the block.
void bridge_state_refresh_from_rust();

// Load path: push the staging variables back into the Rust PPU, then
// refresh the CHR / NT / palette window copies and re-push the mirror
// mode. The Rust renderer reads copies, not the C++ arrays, so skipping
// the refresh would render a stale frame after a load.
void bridge_state_apply_to_rust();