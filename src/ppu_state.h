// ppu_state.h
//
// v1.12 Scissors Phase E-A: PPU savestate bookkeeping split.
//
// Pure code move from src/ppu.cpp lines 1976-2038:
//   - TempAddrT / RefreshAddrT (file-static savestate scratch)
//   - FCEUPPU_LoadState, FCEUPPU_SaveState
//   - FCEUPPU_STATEINFO[], FCEU_NEWPPU_STATEINFO[] (SFORMAT arrays)
//
// Cross-TU symbols promoted from static to extern (so the SFORMAT
// tables below can reference them):
//   - ppudead, kook (file-static in ppu.cpp; referenced here in SFORMAT).
//
// Other PPU globals referenced by SFORMAT tables (originally file-scope
// in ppu.cpp; extern decls added here so ppu_state.cpp can take their
// addresses across TU).

#pragma once

#include "ppu_class.h"   // fceu11::Ppu, PUREGS struct definition

// File-static in ppu.cpp; referenced by the SFORMAT arrays in
// ppu_state.cpp. Promote to extern so the tables can take their
// addresses across TU. Definitions stay in ppu.cpp (unchanged) until
// Phase E-C migrates them to ppu_rendering.cpp alongside FCEUPPU_Loop.
extern int ppudead;
extern int kook;

// File-scope globals in ppu.cpp referenced by FCEU_NEWPPU_STATEINFO[].
// Extern decls are required now that ppu_state.cpp is a separate TU.
extern uint8 idleSynch;       // line 152 of ppu.cpp
struct SPRITE_READ;
extern SPRITE_READ spr_read;  // line 149 of ppu.cpp
extern struct PPUREGS ppur;   // line 293 of ppu.cpp (PPUREGS defined in ppu.cpp)
extern uint8 PPUSPL;          // line 399 of ppu.cpp
#include <array>
extern alignas(64) std::array<uint8_t, 0x20> PALRAM;   // line 409 of ppu.cpp
extern std::array<uint8_t, 3> UPALRAM;                 // line 410 of ppu.cpp