// ppu_rendering.h
//
// v1.12 Scissors Phase E-C (scope v1): main per-frame loop split —
// **placeholder header for future migration**.
//
// This file is currently a placeholder. The rendering pipeline
// (FCEUPPU_Loop + FCEUX_PPU_Loop + DoLine + helpers + BGData +
// sprite eval + runppu + PaletteAdjustPixel) is large and has many
// cross-TU dependencies (MMC5* hacks, deemp / deempcnt, maxsprites,
// VRAMBuffer / PPUGenLatch, PPU_hook, GameHBIRQHook, BGData struct,
// file-static helpers). Moving them safely requires either:
//   (a) Promoting ~13 file-static globals to extern (with explicit
//       ownership in ppu_rendering.h), or
//   (b) Moving the entire pipeline as one batch (intra-TU once
//       consolidated in ppu_rendering.cpp).
//
// Plan §4.4 calls for this batch in E-C; the v1.12 Scissors plan
// approved at commit `wip` allocates this work. For safety with the
// verification-constrained Phase E/F/G run, we ship the placeholder
// header and leave the rendering TU consolidation for a follow-up
// patch with full MSVC build verification.
//
// Future scope (deferred to E-C.2 or v1.13):
//   - Move FCEUPPU_Loop + (*PPU_MASTER) + DoLine + DoLine's helpers
//     (RefreshLine / ResetRL / EndRL / CheckSpriteHit / Fixit1 /
//     Fixit2 / FCEUPPU_LineUpdate) + sprite eval (FetchSpriteData /
//     RefreshSprites / CopySprites) + runppu + PaletteAdjustPixel +
//     FCEUX_PPU_Loop + BGData / bgdata + kLineTime / kFetchTime +
//     pputime / totpputime / framectr + `int test` (dead) to
//     ppu_rendering.cpp.
//   - Add `__forceinline` declarations for runppu / PaletteAdjustPixel /
//     BGData::Record::Read per plan §4.3.
//   - Update ppu.h (umbrella) to #include "ppu_rendering.h".

#pragma once

#include "ppu_class.h"   // fceu11::Ppu, PPU/NTARAM/vnapage/PPUCHRRAM/PPUNTARAM aliases

// No declarations yet. Future E-C.2 / v1.13 work adds:
//   extern void DoLine(void);                    // static → extern
//   extern void RefreshLine(int lastpixel);      // static → extern
//   extern void ResetRL(uint8 *target);          // static → extern
//   extern void EndRL(void);                     // static → extern
//   extern void CheckSpriteHit(int p);           // static → extern
//   extern void Fixit1(void);                    // static → extern
//   extern void Fixit2(void);                    // static → extern
//   extern void FetchSpriteData(void);           // static → extern
//   extern void RefreshSprites(void);            // static → extern
//   extern void CopySprites(uint8 *target);      // static → extern
//   extern int FCEUPPU_Loop(int skip);
//   extern int FCEUX_PPU_Loop(int skip);
//   extern void runppu(int x);                   // __forceinline candidate
//   extern int PaletteAdjustPixel(int pixel);    // __forceinline candidate