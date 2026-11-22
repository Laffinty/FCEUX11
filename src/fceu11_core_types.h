/* FCE Ultra - NES/Famicom Emulator
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
// FCEUX11 — Core type aliases under fceu11:: namespace (v0.3.8)
//
// Per v0.3.x construction plan v3 §5 v0.3.8 task 4 ("175 mapper
// MapIRQHook → using"): strongly type the global mapper-IRQ dispatch hook
// without changing its linkage. The global symbol `::MapIRQHook` keeps its
// pre-v0.3.x C-linkage contract used by 35 mapper .cpp files in
// src/boards/; this header only adds a typed alias so the extern
// declaration in src/x6502.h can verify type identity at compile time
// (see static_assert in src/x6502.cpp).
//
// Per plan §6.1 phase 1: new symbols enter fceu11::; old symbols
// (global `MapIRQHook`) stay at global namespace. Full migration to
// fceu11::MapIRQHook qualified call sites is deferred to v0.4.0.

#ifndef FCEU11_CORE_TYPES_H
#define FCEU11_CORE_TYPES_H

namespace fceu11 {

// Mapper-IRQ hook function-pointer type. The hook is invoked from
// src/x6502.cpp's main run loop on every CPU cycle to give the active
// mapper an opportunity to drive its own IRQ counter (MMC3 scanline
// counter, VRC6/VRC7 timer, FDS sound interrupt, etc.).
//
// Argument: the number of CPU cycles that just executed. Mappers
// integrate this into their internal counter to detect IRQ trigger
// conditions. NULL when no IRQ-driving mapper is active.
using MapIRQHook = void(*)(int);

} // namespace fceu11

#endif // FCEU11_CORE_TYPES_H
