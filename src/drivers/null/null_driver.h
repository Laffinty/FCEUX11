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
// F11QA P1 — Null Driver (headless test harness foundation).
//
// Provides globals that the core engine expects (dendy, pal_emulation)
// without linking Qt or SDL. Registers all-nullptr DriverCallbacks so
// that every FCEUD_* callback is a safe no-op.
//
// Usage from a headless test binary:
//   null_driver_init();          // install null callbacks
//   fceu11::Initialize();        // bring up the engine
//   ... run frames / load ROM ...
//   fceu11::Kill();              // tear down

#pragma once

extern int dendy;
extern int pal_emulation;

/// Register all-nullptr DriverCallbacks and initialise driver globals.
/// Call once before fceu11::Initialize().
void null_driver_init();
