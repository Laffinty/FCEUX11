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

#ifndef __FCEU_TYPES
#define __FCEU_TYPES

#include <stdlib.h>
#include <new>
#include <memory>  // v0.3.6: std::unique_ptr for fceuScopedPtr alias

// v0.3.3: C++20 feature-test macros
#ifdef __cplusplus
  #include <cstdint>
  #if __cpp_lib_span >= 202002L
    #define FCEU11_HAS_STD_SPAN 1
  #endif
  #if __cpp_lib_format >= 201907L
    #define FCEU11_HAS_STD_FORMAT 1
  #endif
#endif

//enables a hack designed for debugging dragon warrior 3 which treats BRK as a 3-byte opcode
//#define BRK_3BYTE_HACK

//enables a hack designed for debugging dragon warrior 3 which treats 0F and 1F NL files both as 1F
//#define DW3_NL_0F_1F_HACK

///causes the code fragment argument to be compiled in if the build includes debugging
#ifdef FCEUDEF_DEBUGGER
#define DEBUG(X) X;
#else
#define DEBUG(X)
#endif

#ifdef _MSC_VER
typedef unsigned char uint8;
typedef unsigned short uint16;
typedef unsigned int uint32;
typedef signed char int8;
typedef signed short int16;
typedef signed int int32;
#define dup _dup
#define stat _stat
#define mkdir _mkdir
#define alloca _alloca
#define FCEUX_fstat _fstat
#if _MSC_VER < 1500
#define vsnprintf _vsnprintf
#endif
#define W_OK 2
#define R_OK 2
#define X_OK 1
#define F_OK 0
#define PATH_MAX 260
#else

// MinGW-w64 GCC 16+ provides alloca via <malloc.h> included from <stdlib.h>.
// Manual definition is no longer needed and triggers -Wbuiltin-macro-redefined.


//#include <typeinfo>

#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>
#include <inttypes.h>
typedef int8_t int8;
typedef int16_t int16;
typedef int32_t int32;

typedef uint8_t uint8;
typedef uint16_t uint16;
typedef uint32_t uint32;

#define FCEUX_fstat fstat
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
					   and global in MSVC.  Bummer.
					*/
 #define PSS_STYLE 2			/* Does MSVC compile for anything
					   other than Windows/DOS targets?
					*/

 #if _MSC_VER >= 1300
  // v0.3.1: Removed /wd4244 /wd4996 pragma suppression (now handled by /W4 /WX- at CMake level)
#endif

 #if _MSC_VER < 1400
  #define vsnprintf _vsnprintf
 #endif
#endif

#if PSS_STYLE==2

#define PSS "\\"
#define PS '\\'

#elif PSS_STYLE==1

#define PSS "/"
#define PS '/'

#elif PSS_STYLE==3

#define PSS "\\"
#define PS '\\'

#elif PSS_STYLE==4

#define PSS ":"
#define PS ':'

#else

#error PSS_STYLE undefined or invalid; see "types.h" for possible values, and add as compile-time option.

#endif



typedef void (*writefunc)(uint32 A, uint8 V);
typedef uint8 (*readfunc)(uint32 A);

#ifndef CTASSERT
#define CTASSERT(x)  typedef char __assert ## y[(x) ? 1 : -1];
#endif

#define __FCEU_STRINGIZE2(x) #x
#define __FCEU_STRINGIZE(x) __FCEU_STRINGIZE2(x)

#define  FCEU_CPP_HAS_STD(x)  ( defined(__cplusplus) && (__cplusplus >= x) )

#ifdef   __has_cpp_attribute
#define  FCEU_HAS_CPP_ATTRIBUTE(x)  __has_cpp_attribute(x)
#else
#define  FCEU_HAS_CPP_ATTRIBUTE(x)  0
#endif

#define  FCEU_UNUSED(x)   (void)(x)

#if FCEU_CPP_HAS_STD(201603L) || FCEU_HAS_CPP_ATTRIBUTE(maybe_unused)
#define  FCEU_MAYBE_UNUSED  [[maybe_unused]]
#else
#define  FCEU_MAYBE_UNUSED
#endif

#if defined(_MSC_VER)
	// Microsoft compiler won't catch format issues, but VS IDE can catch on analysis mode
	#define  __FCEU_PRINTF_FORMAT  _In_z_ _Printf_format_string_
	#define  __FCEU_PRINTF_ATTRIBUTE( fmt, va )

#elif defined(__GNUC__) || FCEU_HAS_CPP_ATTRIBUTE(format)
	// GCC will perform printf format type checks, useful for catching format errors.
	#define  __FCEU_PRINTF_FORMAT
	#define  __FCEU_PRINTF_ATTRIBUTE( fmt, va )  __attribute__((__format__(__printf__, fmt, va)))

#else
	#define  __FCEU_PRINTF_FORMAT
	#define  __FCEU_PRINTF_ATTRIBUTE( fmt, va )
#endif

// Scoped pointer ensures that memory pointed to by this object gets cleaned up
// and deallocated when this object goes out of scope. Helps prevent memory leaks
// on temporary memory allocations in functions with early outs.
//
// v0.3.6: fceuScopedPtr<T> deprecated in favor of std::unique_ptr<T> (with
// custom deleters in utils/memory.h for malloc/free-backed allocations).
// The legacy fceuAllocType enum is preserved as a no-op typedef for any
// out-of-tree call sites that still reference it.
enum fceuAllocType
{
	FCEU_ALLOC_TYPE_NEW = 0,
	FCEU_ALLOC_TYPE_NEW_ARRAY,
	FCEU_ALLOC_TYPE_MALLOC
};

// v0.3.6: legacy fceuScopedPtr replaced by std::unique_ptr. The new RAII
// helper for malloc-backed buffers lives in utils/memory.h (FceuMallocPtr).
// Existing call sites should migrate: `fceuScopedPtr<T> x = new T;` becomes
// `std::unique_ptr<T> x(new T);`, and raw `fceuScopedPtr<T> x;` becomes
// `std::unique_ptr<T> x;` (use x.reset(p) for assignment).
#if !defined(FCEUX11_NO_DEPRECATION_WARNINGS)
template <typename T>
using fceuScopedPtr [[deprecated("fceuScopedPtr is deprecated since v0.3.6; use std::unique_ptr<T> (or FceuMallocPtr for malloc-backed buffers)")]] = std::unique_ptr<T>;
#else
template <typename T>
using fceuScopedPtr = std::unique_ptr<T>;
#endif

#include "utils/endian.h"

#endif
