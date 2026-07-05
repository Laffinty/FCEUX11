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

#pragma once

// File-static in ppu.cpp; referenced by the SFORMAT arrays in
// ppu_state.cpp. Promote to extern so the tables can take their
// addresses across TU. Definitions stay in ppu.cpp (unchanged) until
// Phase E-C migrates them to ppu_rendering.cpp alongside FCEUPPU_Loop.
extern int ppudead;
extern int kook;