// ppu_core.h
//
// v1.12 Scissors Phase E-B (scope v1): PPU lifecycle + accessor split.
//
// Pure code move from src/ppu.cpp — lines 295-301, 369-370, 1757-1828,
// 2040-2048.
//
// Scope of this batch:
//   - newppu_get_scanline / newppu_get_dot / newppu_hacky_emergency_reset
//   - PPU_hook, GameHBIRQHook, GameHBIRQHook2 (function-pointer definitions)
//   - FCEUPPU_SetVideoSystem / PPU_ResetHooks / Reset
//   - FCEUPPU_PeekAddress
//
// NOT moved in this batch (stay in ppu.cpp):
//   - FCEUPPU_Init: calls makeppulut(), which writes file-static ppulut*[]
//     arrays; the ppulut globals stay in ppu.cpp until E-C.
//   - FCEUPPU_Power: assigns file-static DECLFR/DECLFW function pointers
//     (A2002/A200x/B2000/B2001/.../B4014) into the ARead[]/BWrite[]
//     memory-read/write tables; those static register handlers don't
//     move until the register-port half of the split (a follow-up
//     batch).
//
// Register-port handlers (DECLFR/DECLFW A2002-B4014) and old render
// helpers (DoLine / RefreshLine / FCEUPPU_LineUpdate) STAY in ppu.cpp
// for this batch. They form a tightly-coupled unit (B2007 reads via
// the same VRAMBuffer/PPUGenLatch that A2007 writes; B2001 writes
// `deemp` that FCEUPPU_Loop reads). Splitting them requires promoting
// ~13 file-static globals to extern and adding `extern void DoLine();`
// declarations — a larger surface that warrants its own gate.
//
// Cross-TU promotions in this batch:
//   - scanlines_per_frame (file-static → extern), written by
//     FCEUPPU_SetVideoSystem in ppu_core, read by FCEUPPU_Loop in
//     ppu.cpp (moves to ppu_rendering.cpp in E-C).

#pragma once

#include "ppu_class.h"   // fceu11::Ppu, PPUREGS / PPUSTATUS / SPRITE_READ structs

// ----------------------------------------------------------------------------
// Cross-TU promotion (file-static → extern).
// ----------------------------------------------------------------------------

// Total scanlines per frame (NTSC=262, PAL=312). Written by
// FCEUPPU_SetVideoSystem (ppu_core), read by FCEUPPU_Loop /
// FCEUX_PPU_Loop (ppu.cpp; moves to ppu_rendering.cpp in E-C).
extern unsigned int scanlines_per_frame;

// Phase E-A: PPU globals now referenced from this TU (via
// newppu_get_* / FCEUPPU_Reset).
extern PPUREGS ppur;
extern uint8 PPUSPL;
extern uint8 idleSynch;
extern int ppudead;
extern int kook;
extern bool new_ppu_reset;

// Phase E-A: PPU_status is a #define macro in ppu.cpp pointing at
// PPU[2]; the macro doesn't expand across TU. Drop-static promotion
// of the underlying register file already lives in ppu_class.h.
#include "ppu.h"   // PPU[4] reference alias + PPU_status macro