// ppu_rendering.h
//
// v1.13 Purify Phase A: rendering pipeline split.
//
// Hosts the PPU rendering pipeline (DoLine + BG/sprite fetch + pixel
// composite + main loops + runppu + BGData + PaletteAdjustPixel).
// All declarations here are externs consumed by ppu.cpp (registration
// via ARead/BWrite + lifecycle) and by the GUI / debugger TUs.
//
// Cross-TU design notes:
//   - File-static globals that were scoped to ppu.cpp and referenced
//     from inside the rendering pipeline (Pline/Plinef/firsttile/tofix,
//     sphitx/sphitdata/spork, rendersprites/renderbg, numsprites/SpriteBlurp,
//     deemp/deempcnt, maxsprites, kLineTime/kFetchTime, pputime/totpputime,
//     framectr, ppulut1/2/3, PPU_MASTER) migrate to ppu_rendering.cpp
//     and stay file-static (no external link). Read-only consumers in
//     ppu.cpp (e.g. FCEUPPU_Power / FCEUPPU_Init that need makeppulut)
//     get explicit extern function declarations below.
//   - PPUREGS / PALRAM / UPALRAM / PPUSPL / ppur / SPRITE_READ /
//     scanlines_per_frame are already declared in ppu_state.h / ppu_core.h
//     and are NOT re-declared here.
//   - Compat aliases (PPU/NTARAM/vnapage/...) live in ppu_class.h.

#pragma once

#include <array>
#include "types.h"
#include "ppu_class.h"   // fceu11::Ppu, g_ppu, PPU/NTARAM/vnapage aliases

// Palette lookup tables consumed by pputile.inc (RefreshLine) and
// RefreshSprites. Initialized once by makeppulut() from FCEUPPU_Init.
// Definition lives in ppu_rendering.cpp; alignas(64) preserved for
// cache alignment (v1.13 §1.3 Batch B).
extern alignas(64) std::array<uint32, 256> ppulut1;
extern alignas(64) std::array<uint32, 256> ppulut2;
extern alignas(64) std::array<uint32, 128> ppulut3;

// Sprite hit / per-scanline sprite presence. Promoted from file-static
// in ppu.cpp because RefreshSprites (line 1438+, stays in ppu.cpp for
// now) reads/writes them. Definition migrates to ppu_rendering.cpp in
// Batch C.
extern int32_t sphitx;
extern uint8_t sphitdata;
extern int spork;

// Per-scanline timestamp (debugger-visible; non-static so ConsoleDebugger
// can read it). Promoted from ppu.cpp line 862 because ResetRL /
// RefreshLine (now in ppu_rendering.cpp) write/read it, and FCEUX_PPU_Loop
// (still in ppu.cpp until Batch D) writes it too.
extern int linestartts;

// MMC5 H-blank callback. Defined in src/boards/mmc5.cpp. DoLine (now in
// ppu_rendering.cpp) and RefreshSprites / FCEUPPU_Loop / FCEUX_PPU_Loop
// (still in ppu.cpp) call it conditionally on MMC5Hack.
extern void MMC5_hb(int);

// Render-plane toggles. Promoted from file-static in ppu.cpp (was at
// lines 899) because CopySprites / FCEUPPU_Loop (still in ppu.cpp) read
// them. Definition migrates to ppu_rendering.cpp.
extern bool rendersprites;
extern bool renderbg;

// ResetRL / DoLine: file-static helpers called from FCEUPPU_Loop
// (still in ppu.cpp until Batch D). Declared extern here so the
// FCEUPPU_Loop body keeps compiling.
extern void ResetRL(uint8 *target);
extern void DoLine(void);

// User-configurable BG fill color. Definition in ppu.cpp; DoLine reads
// it. Promoted to extern in Batch C.
extern uint8_t gNoBGFillColor;

// MMC5 BG VRAM address resolver. Definition in ppu.cpp; pputile.inc
// (included from RefreshLine in ppu_rendering.cpp) calls it via the
// MMC5BGVRAMADR(vadr) macro when MMC5 + CL mode.
extern uint8_t* MMC5BGVRAMADR(uint32 A);

namespace fceu11 {
// Render-plane toggles (set/get from GUI; live with rendering code).
void SetRenderPlanes(bool sprites, bool bg);
void GetRenderPlanes(bool& sprites, bool& bg);
} // namespace fceu11

// ----------------------------------------------------------------------------
// Lifecycle / main loop
// ----------------------------------------------------------------------------
// FCEUPPU_Loop / FCEUX_PPU_Loop are already declared in ppu.h (transitive
// include via ppu_rendering.h). FCEUPPU_LineUpdate is declared in ppu.h
// too. Definitions moved from ppu.cpp to ppu_rendering.cpp (Batch D).

// makeppulut initializes ppulut1/2/3 (file-static in ppu_rendering.cpp).
// Called once from FCEUPPU_Init (lives in ppu.cpp) and once from
// FCEUPPU_Power. Declared extern so ppu.cpp can invoke it.
extern void makeppulut(void);

// ----------------------------------------------------------------------------
// Old-PPU per-scanline pipeline (called from FCEUPPU_Loop)
// ----------------------------------------------------------------------------
extern void FetchSpriteData(void);
extern void RefreshSprites(void);
extern void CopySprites(uint8 *target);

// ----------------------------------------------------------------------------
// Public sprite-limiter toggle
// ----------------------------------------------------------------------------
extern void FCEUI_DisableSpriteLimitation(int a);

// ----------------------------------------------------------------------------
// New-PPU rendering helpers (called from FCEUX_PPU_Loop and BGData::Record)
// ----------------------------------------------------------------------------
// runppu and PaletteAdjustPixel are hot-path. Bodies live in
// ppu_rendering.cpp; all callers (BGData::Record::Read, FCEUPPU_Loop,
// FCEUX_PPU_Loop) also live in ppu_rendering.cpp, so MSVC /Ob2 (Release
// default) inlines these without explicit __forceinline hints.
extern void runppu(int x);
extern int  PaletteAdjustPixel(int pixel);

// Master PPU frame entry function pointer. Default init in ppu_rendering.cpp
// (PPU_MASTER = FCEUPPU_Loop). Other TUs may temporarily repoint it
// (legacy per-board hacks); the declaration must be visible.
extern int (*PPU_MASTER)(int skip);

// newppu: legacy switch between old (FCEUPPU_Loop) and new (FCEUX_PPU_Loop)
// PPU engines. Definition in ppu.cpp; both engines read it.
extern int newppu;

// deemp: color deemphasis bits. Definition (file-static) in
// ppu_rendering.cpp; B2001 mask-write handler in ppu.cpp writes it.
// Promoted to extern in Batch D so the early update path still works.
extern uint8_t deemp;