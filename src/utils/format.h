// FCEUX11 — Compile-time feature test + format-string attribute macros (v0.3.7)
// Split out of src/types.h per plan v3 §5 v0.3.7. Collects the C++
// standard library version probe, the [[attribute]] probe, the
// printf-format SAL annotation (MSVC) and the GCC printf-format
// attribute. CTASSERT is the classic compile-time assertion macro used
// across src/. __FCEU_STRINGIZE is the two-level stringize trick that
// expands macro arguments before stringizing.

#ifndef __FCEU_FORMAT_H
#define __FCEU_FORMAT_H

// FCEU_CPP_HAS_STD(x) — true when compiling C++ and the standard is at
// or above the given value. x is in the form YYYYMML (e.g. 201603L for
// C++17 features tested via the C++20 <version> header; the L suffix
// matches the conventional feature-test macro layout).
#define  FCEU_CPP_HAS_STD(x)  ( defined(__cplusplus) && (__cplusplus >= x) )

// FCEU_HAS_CPP_ATTRIBUTE(x) — 1 if the [[x]] attribute is known to the
// compiler (works on MSVC 2017+, GCC 5+, and any C++20 compiler).
// Falls back to 0 on compilers that don't implement __has_cpp_attribute.
#ifdef   __has_cpp_attribute
	#define  FCEU_HAS_CPP_ATTRIBUTE(x)  __has_cpp_attribute(x)
#else
	#define  FCEU_HAS_CPP_ATTRIBUTE(x)  0
#endif

// __FCEU_STRINGIZE — two-level stringize so that macro arguments are
// expanded before being turned into a string literal. Example:
//   #define X 42
//   __FCEU_STRINGIZE(X)   → "42"
//   #X                    → "X"  (wrong)
#define  __FCEU_STRINGIZE2(x) #x
#define  __FCEU_STRINGIZE(x)  __FCEU_STRINGIZE2(x)

// FCEU_UNUSED(x) — silence "unused parameter" warnings when a
// parameter must be present for ABI / virtual-function reasons.
//   void f(int x) { FCEU_UNUSED(x); }   // x intentionally unused
#define  FCEU_UNUSED(x)   (void)(x)

// FCEU_MAYBE_UNUSED — apply to declarations to mark "may be unused"
// in a portable way. C++17 introduced [[maybe_unused]]; C++20
// compilers recognise it. FCEU_HAS_CPP_ATTRIBUTE covers the case where
// the C++ standard is below 201603L but the compiler still supports
// the attribute extension.
#if FCEU_CPP_HAS_STD(201603L) || FCEU_HAS_CPP_ATTRIBUTE(maybe_unused)
	#define  FCEU_MAYBE_UNUSED  [[maybe_unused]]
#else
	#define  FCEU_MAYBE_UNUSED
#endif

// __FCEU_PRINTF_FORMAT / __FCEU_PRINTF_ATTRIBUTE — printf format
// string annotation. MSVC uses the SAL form (the IDE catches format
// mismatches under /analyze); GCC and other compilers that support
// the gnu::format attribute use the __format__(__printf__, fmt, va)
// form, which makes the compiler itself reject mismatched format
// strings at warning level /W4.
// v0.3.6.6: removed the __clang__ defensive branch (project is
// MSVC-only per plan v3 §3.1; clang would no longer compile this
// header anyway). GCC is still allowed for non-default build
// experiments; the FCEU_HAS_CPP_ATTRIBUTE(format) branch is the
// safety net for compilers that have the attribute but not the
// __GNUC__ predefined macro.
#if defined(_MSC_VER)
	// Microsoft compiler won't catch format issues at /W4, but the
	// VS IDE catches them in analysis mode via the SAL annotation.
	#define  __FCEU_PRINTF_FORMAT  _In_z_ _Printf_format_string_
	#define  __FCEU_PRINTF_ATTRIBUTE( fmt, va )

#elif defined(__GNUC__) || FCEU_HAS_CPP_ATTRIBUTE(format)
	// GCC will perform printf format type checks.
	#define  __FCEU_PRINTF_FORMAT
	#define  __FCEU_PRINTF_ATTRIBUTE( fmt, va )  __attribute__((__format__(__printf__, fmt, va)))

#else
	#define  __FCEU_PRINTF_FORMAT
	#define  __FCEU_PRINTF_ATTRIBUTE( fmt, va )
#endif

// CTASSERT(x) — compile-time assertion. Declares a typedef of an
// array of size (x) ? 1 : -1; the -1 size causes a compile error if x
// is false. Use at file scope only; do not use inside a function body.
#ifndef CTASSERT
	#define CTASSERT(x)  typedef char __ctassert_##__LINE__[(x) ? 1 : -1]
#endif

#endif // __FCEU_FORMAT_H
