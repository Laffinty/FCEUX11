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
// FCEUX11 — v1.6 Resonance §3: ExpansionAudio interface + EXPSOUND adapter.
//
// Non-inline adapter parts. The simple routing helpers live in the header as
// static inline functions so the compiler can fold them at call sites. The
// C-style NeoFill callback (used by the Rust filter FFI) is defined here so
// it has a single stable address.

#include "expansion_audio.h"
#include "sound.h"   // GameExpSound

void FCEU11_ExpNeoFillCallback(int32_t* wave, int count) {
    FCEU11_ExpNeoFill(&GameExpSound, wave, count);
}
