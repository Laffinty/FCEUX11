// ppu_core.h
//
// v1.12 Scissors Phase E-B (scope v1): PPU lifecycle + accessor split.
// v2.1.1.7 Step C (M4, 2026-09-12): the C++ PPU engine was deleted
// (src/ppu.cpp / ppu_rendering.cpp / ppu_core.cpp). This header now only
// carries the cross-TU declarations that survive:
//
//   * scanlines_per_frame - written by FCEUPPU_SetVideoSystem
//     (src/ppu_shared.cpp), still read by the legacy frame-length arithmetic
//     in the drivers;
//   * the tombstone mirrors the savestate staging (src/ppu_state.cpp) and the
//     debugger take addresses of. Their definitions live in
//     src/ppu_legacy_stub.cpp; the live state is Rust-owned.
//
// The engine-side declarations (newppu_get_*, PPU_hook, FCEUPPU_Reset,
// FCEUPPU_SetVideoSystem, PPU_ResetHooks, FCEUPPU_PeekAddress, FCEUPPU_Power,
// FCEUPPU_Init) moved into src/ppu_shared.cpp and are declared in src/ppu.h.

#pragma once

#include "types.h"
#include "ppu_class.h"   // fceu11::Ppu, PPUREGS / PPUSTATUS / SPRITE_READ structs

// Total scanlines per frame (NTSC=262, PAL=312). Written by
// FCEUPPU_SetVideoSystem (src/ppu_shared.cpp).
extern unsigned int scanlines_per_frame;

// ---------------------------------------------------------------------------
// Tombstone mirrors (definitions in src/ppu_legacy_stub.cpp).
//
// The retired C++ engine was their only writer. They stay zero-initialised so
// anything that still takes their address sees the historical power-on bytes;
// reading them for behaviour would be a bug (plan section 0.1).
// ---------------------------------------------------------------------------
extern PPUREGS ppur;
extern struct SPRITE_READ spr_read;
extern uint8 idleSynch;
extern bool new_ppu_reset;
extern uint8 PPUSPL;
extern int ppudead;
extern int kook;

// PPU_status was a #define macro in the deleted ppu.cpp; the alias in
// ppu_class.h (PPU[2]) is the only surviving spelling.
#include "ppu.h"