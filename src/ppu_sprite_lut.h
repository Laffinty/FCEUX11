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
// ppu_sprite_lut.h
//
// hotfix2 P0-1 (ARCH-2): two-stage sprite decode LUT.
//
// Stage-1 (compile-time, this header's companion .cpp):
//   65536-entry table indexed by `ca0 | (ca1 << 8)`. Each entry
//   holds 8 packed bytes — one per pixel position (left to right) —
//   where byte k = the 2-bit palette index for pixel k, or 0 if
//   that pixel is transparent (both pattern bits are 0).
//
// Stage-2 (per-sprite, runtime):
//   a 4-entry palette-color table is built at RefreshSprites entry
//   for each sprite's `VB` (palette index base), so the hot loop
//   reads colours from a 16-byte L1-resident table instead of
//   recomputing READPAL per pixel.
//
// The LUT is independent of PALRAM / GRAYSCALE / SP_BACK / H_FLIP;
// SP_BACK becomes a separate "back palette" table and H_FLIP becomes
// a 64-bit byte-reverse of the packed index — neither touches the
// LUT contents.

#ifndef FCEU11_PPU_SPRITE_LUT_H
#define FCEU11_PPU_SPRITE_LUT_H

#include <array>
#include <cstdint>

namespace fceu11::ppu {

// kSpriteIdxLUT[ca0 | (ca1 << 8)] → packed uint64_t.
// Reads as 8 bytes (little-endian memory order) where byte k is
// the 2-bit palette index for the k-th pixel from the left, or
// 0 if that pixel is transparent.
//
// Layout:
//   bit-pos  0.. 7  = pixel 0 (leftmost) index
//   bit-pos  8..15  = pixel 1 index
//   ...
//   bit-pos 56..63  = pixel 7 (rightmost) index
//
// Total memory: 65536 * 8 = 512 KiB. alignas(64) puts each entry
// on its own cache line group to avoid false sharing across
// alternate lookups (not critical here, but free with one cache
// line per few entries anyway).
alignas(64) extern const std::array<uint64_t, 65536> kSpriteIdxLUT;

}  // namespace fceu11::ppu

#endif  // FCEU11_PPU_SPRITE_LUT_H
