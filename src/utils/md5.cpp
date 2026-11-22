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
/// \file
/// \brief MD5 wrapper — delegates to Rust.
///
/// Phase 1 (v0.2.2): Rust module provides memory-safe equivalent,
/// validated against the `md-5` crate (RustCrypto).

#include <string.h>
#include "../types.h"
#include "md5.h"
#include "../rust/fceux11_rust.h"

void md5_starts( struct md5_context *ctx )
{
	fceux11_rust_md5_starts(ctx);
}

void md5_update( struct md5_context *ctx, uint8 *input, uint32 length )
{
	fceux11_rust_md5_update(ctx, input, length);
}

void md5_finish( struct md5_context *ctx, uint8 digest[16] )
{
	fceux11_rust_md5_finish(ctx, digest);
}

/* Uses a static buffer, so beware of how it's used. */
const char *md5_asciistr(MD5DATA& md5)
{
	return fceux11_rust_md5_asciistr(md5.data);
}
