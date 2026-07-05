// ppu_rendering.cpp
//
// v1.12 Scissors Phase E-C (scope v1): main per-frame loop split —
// **placeholder file, body consolidation deferred**.
//
// This file currently has no function bodies. The rendering pipeline
// (FCEUPPU_Loop + FCEUX_PPU_Loop + DoLine + helpers + BGData +
// sprite eval + runppu + PaletteAdjustPixel) remains in ppu.cpp
// until a follow-up patch with full MSVC build verification.
//
// See ppu_rendering.h for the future-scope list and rationale.

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
#include "ppu_rendering.h"

// No function bodies yet. Future E-C.2 / v1.13 work adds:
//   FCEUPPU_Loop, FCEUX_PPU_Loop, DoLine, runppu, PaletteAdjustPixel,
//   BGData, RefreshLine, ResetRL, EndRL, CheckSpriteHit, Fixit1/2,
//   FetchSpriteData, RefreshSprites, CopySprites, etc.
//
// The current scaffolding is intentionally empty (zero non-comment
// lines of code in the body) to satisfy the v1.12 plan §0.6
// "include aggregator shell" anti-pattern: this file must hold real
// content (>50 lines, >60% of moved code) once the migration lands.
// Until then, it exists as a registration point and documentation.