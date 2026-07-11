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

/// \file
/// \brief memory management services provided by FCEU core

// v0.3.6: Suppress deprecation warnings inside the implementation file —
// memory.cpp implements the deprecated FCEU_malloc/FCEU_free/FCEU_dmalloc/FCEU_dfree
// shims. External callers (and the v0.3.6 mapper migrations) should still see
// the warnings. CMakeLists.txt also injects -DFCEUX11_NO_DEPRECATION_WARNINGS
// via add_definitions; guard against redefinition (C4005).
#ifndef FCEUX11_NO_DEPRECATION_WARNINGS
#define FCEUX11_NO_DEPRECATION_WARNINGS
#endif

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <new>
#include "../types.h"
#include "../fceu.h"
#include "memory.h"

void *FCEU_amalloc(size_t size, size_t alignment)
{
	size = (size + alignment - 1) & ~(alignment-1);

	void *ret = nullptr;
	#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MINGW64__)
	ret = _aligned_malloc(size,alignment);
	#else
	ret = aligned_alloc(alignment,size);
	#endif

	if(!ret)
		FCEU_abort("Error allocating memory!");

	return ret;
}

void FCEU_afree(void* ptr)
{
	#if defined(_MSC_VER) || defined(__MINGW32__) || defined(__MINGW64__)
	_aligned_free(ptr);
	#else
	free(ptr);
	#endif
}

static void *_FCEU_malloc(size_t size)
{
	void* ret = malloc(size);

	if(!ret)
		FCEU_abort("Error allocating memory!");

	return ret;
}

static void _FCEU_free(void* ptr)
{
	free(ptr);
}

void *FCEU_gmalloc(size_t size)
{
	void *ret = ::operator new(size, std::nothrow);

	if(!ret)
		FCEU_abort("Error allocating memory!");

	// initialize according to RAMInitOption, default zero
	FCEU_MemoryRand((uint8*)ret,size,true);

	return ret;
}

void *FCEU_malloc(size_t size)
{
	void *ret = _FCEU_malloc(size);
	memset(ret, 0, size);
	return ret;
}

void FCEU_gfree(void *ptr)
{
	::operator delete(ptr);
}

void FCEU_free(void *ptr)
{
	_FCEU_free(ptr);
}

void *FCEU_dmalloc(size_t size)
{
	return FCEU_malloc(size);
}

void FCEU_dfree(void *ptr)
{
	return FCEU_free(ptr);
}

void* FCEU_realloc(void* ptr, size_t size)
{
	void* ret = realloc(ptr,size);
	if(!ret && size != 0)
	{
		// R7.1: realloc failure with size!=0 does NOT free ptr (per C
		// standard) — the caller would leak the original buffer. Match
		// the FCEU_malloc/FCEU_amalloc policy and abort on failure. The
		// size==0 case is implementation-defined (MSVC frees ptr and
		// returns nullptr) and is NOT a failure — return nullptr normally.
		FCEU_abort("Error reallocating memory!");
	}
	return ret;
}

void FCEU_abort(const char* message)
{
	if(message) FCEU_PrintError("%s", message);
	abort();
}
