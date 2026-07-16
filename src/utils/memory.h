/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2002 Xodnizel
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

/*        Various macros for faster memory stuff
		(at least that's the idea)
*/

#ifndef FCEU11_MEMORY_H
#define FCEU11_MEMORY_H

#include <memory>

#include "utils/simd_fill.h"   // hotfix2 P2-1 (ALIAS-1): see simd_fill.h

// hotfix2 P2-1 (ALIAS-1): the legacy macro body declared a
// strict-aliasing UB (`*(uint32*)&(uint8_t[])` write on a buffer whose
// dynamic type is `uint8_t`). The macro is now a thin redirect to the
// type-safe `fceu11::fceu_dwmemset` (defined in utils/simd_fill.h) which
// uses memcpy semantics internally and dispatches to AVX2
// `_mm256_storeu_si256` on capable hosts.
#define FCEU_dwmemset(d,c,n) fceu11::fceu_dwmemset((d),(c),(n))

// v0.3.6: deprecation annotation. Suppress with -DFCEUX11_NO_DEPRECATION_WARNINGS
// (provided until v0.4.0, per the v0.3.x plan §6.3).
#if !defined(FCEUX11_DEPRECATED)
#  if defined(FCEUX11_NO_DEPRECATION_WARNINGS)
#    define FCEUX11_DEPRECATED(msg)
#  elif defined(__cplusplus) && __cplusplus >= 201402L
#    define FCEUX11_DEPRECATED(msg) [[deprecated(msg)]]
#  elif defined(_MSC_VER)
#    define FCEUX11_DEPRECATED(msg) __declspec(deprecated(msg))
#  else
#    define FCEUX11_DEPRECATED(msg)
#  endif
#endif

//returns a buffer initialized to 0
//v0.3.6: deprecated — use std::make_unique_for_overwrite<uint8_t[]>(n) or
//std::pmr::get_default_resource()->allocate(n)
FCEUX11_DEPRECATED("FCEU_malloc is deprecated since v0.3.6; use std::make_unique_for_overwrite<uint8_t[]>(n) or std::pmr::get_default_resource()")
void *FCEU_malloc(size_t size);

//returns a buffer, with jumbled initial contents
//used by boards for WRAM etc, initialized to 0 (default) or other via RAMInitOption
void *FCEU_gmalloc(size_t size);

//free memory allocated with FCEU_gmalloc
void FCEU_gfree(void *ptr);

//returns an aligned buffer, initialized to 0
//the alignment will default to the largest thing you could ever sensibly want for massively aligned cache friendly buffers
void *FCEU_amalloc(size_t size, size_t alignment = 256);

//frees memory allocated with FCEU_amalloc
void FCEU_afree(void* ptr);

//free memory allocated with FCEU_malloc
//v0.3.6: deprecated — match it with std::unique_ptr<T, FceuMallocDeleter>
FCEUX11_DEPRECATED("FCEU_free is deprecated since v0.3.6; use std::unique_ptr<T, FceuMallocDeleter>")
void FCEU_free(void *ptr);

//reallocate memory allocated with FCEU_malloc
void* FCEU_realloc(void* ptr, size_t size);

//don't use these. change them if you find them.
//v0.3.6: deprecated — FCEU_dmalloc/dfree are merged with FCEU_malloc/free
FCEUX11_DEPRECATED("FCEU_dmalloc is deprecated since v0.3.6; merged with FCEU_malloc. Use std::make_unique_for_overwrite<uint8_t[]>(n) or std::pmr::get_default_resource()")
void *FCEU_dmalloc(size_t size);

//don't use these. change them if you find them.
//v0.3.6: deprecated — FCEU_dmalloc/dfree are merged with FCEU_malloc/free
FCEUX11_DEPRECATED("FCEU_dfree is deprecated since v0.3.6; merged with FCEU_free. Use std::unique_ptr<T, FceuMallocDeleter>")
void FCEU_dfree(void *ptr);

//aborts the process for fatal errors
void FCEU_abort(const char* message = nullptr);

// v0.3.6: RAII deleter for FCEU_gmalloc-allocated buffers.
// FCEU_gmalloc uses malloc() internally; the deleter must use the matching
// deallocator (FCEU_gfree → free) — never replace with delete/delete[].
struct FceuMallocDeleter {
	void operator()(uint8_t* p) const noexcept { if (p) FCEU_gfree(p); }
};
using FceuMallocPtr = std::unique_ptr<uint8_t[], FceuMallocDeleter>;

// v0.3.6: RAII helper for FCEU_gmalloc. Returns a unique_ptr that owns the
// buffer and calls FCEU_gfree() on destruction. Replaces 98 raw FCEU_gmalloc
// call sites in src/boards/*.cpp.
inline FceuMallocPtr FCEU_gmalloc_unique(size_t size) {
	return FceuMallocPtr(static_cast<uint8_t*>(FCEU_gmalloc(size)));
}

#endif // FCEU11_MEMORY_H
