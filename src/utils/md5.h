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
#ifndef _MD5_H
#define _MD5_H

#include "../types.h"
#include "valuearray.h"
#include "../rust/fceux11_rust.h"

typedef ValueArray<uint8,16> MD5DATA;

// R2.1 (docs/internal/refactor_plan_R1_R5_archive.md §Phase R2): layout-assert for MD5DATA — same
// reason as FCEU_Guid in guid.h (16-byte SFORMAT blob in src/state.cpp).
static_assert(sizeof(MD5DATA) == 16,
              "MD5DATA layout changed: 16-byte SFORMAT serialization will "
              "break. Check valuearray.h refactor.");

void md5_starts( struct md5_context *ctx );
void md5_update( struct md5_context *ctx, uint8 *input, uint32 length );
void md5_finish( struct md5_context *ctx, uint8 digest[16] );

/* Uses a static buffer, so beware of how it's used. */
const char *md5_asciistr(MD5DATA& md5);

#endif /* md5.h */
