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
