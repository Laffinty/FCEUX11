/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2001 Aaron Oneal
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

// FCEUX11 v0.3.7 — types.h split: see utils/platform_compat.h, utils/scoped_ptr.h, utils/format.h.

#ifndef __FCEU_TYPES
#define __FCEU_TYPES

#ifdef __cplusplus
	#include <cstdint>
	#if __cpp_lib_span >= 202002L
		#define FCEU11_HAS_STD_SPAN 1
	#endif
	#if __cpp_lib_format >= 201907L
		#define FCEU11_HAS_STD_FORMAT 1
	#endif
#endif

// enables a hack designed for debugging dragon warrior 3 which treats BRK as a 3-byte opcode
//#define BRK_3BYTE_HACK

// enables a hack designed for debugging dragon warrior 3 which treats 0F and 1F NL files both as 1F
//#define DW3_NL_0F_1F_HACK

/// causes the code fragment argument to be compiled in if the build includes debugging
#ifdef FCEUDEF_DEBUGGER
	#define DEBUG(X) X;
#else
	#define DEBUG(X)
#endif

// Integer-width aliases. uint8/16/32 and int8/16/32 are the project's
// canonical spellings (identical to uint8_t etc. at the ABI level).
#ifdef _MSC_VER
typedef unsigned char uint8;
typedef unsigned short uint16;
typedef unsigned int uint32;
typedef signed char int8;
typedef signed short int16;
typedef signed int int32;
#else
#include <sys/types.h>
#include <inttypes.h>
typedef int8_t int8;
typedef int16_t int16;
typedef int32_t int32;
typedef uint8_t uint8;
typedef uint16_t uint16;
typedef uint32_t uint32;
#endif

#ifdef __GNUC__
	typedef unsigned long long uint64;
	typedef uint64 u64;
	typedef long long int64;
	#define INLINE inline
	#define GINLINE inline
#elif defined(_MSC_VER)
	typedef __int64 int64;
	typedef unsigned __int64 uint64;
	#define __restrict__
	#define INLINE __inline
	#define GINLINE			/* Can't declare a function INLINE
					   and global in MSVC.  Bummer. */
	#define PSS_STYLE 2			/* Does MSVC compile for anything
					   other than Windows/DOS targets? */
#endif

// Mapper / cart function-pointer typedefs. Used by every board
// (src/boards/*.cpp) and by src/cart.cpp's PRG/CHR hook registration.
typedef void (*writefunc)(uint32 A, uint8 V);
typedef uint8 (*readfunc)(uint32 A);

// v0.3.7: pull in the three split-out headers so the 41 existing
// #include "types.h" consumers keep working without source changes.
#include "utils/platform_compat.h"
#include "utils/scoped_ptr.h"
#include "utils/format.h"

#include "utils/endian.h"
#include "utils/cache.h"

#endif
