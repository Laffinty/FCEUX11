// FCEUX11 — Platform / POSIX compatibility shims (v0.3.7)
// Split out of src/types.h per plan v3 §5 v0.3.7. POSIX-style names
// (dup / stat / mkdir / alloca) are remapped to MSVC underscore-prefixed
// equivalents. The header also introduces the first two fceu11:: symbols
// (kPathSep, kPathSepStr) while keeping the legacy PSS/PS macros as
// direct string/char literals so the existing 12+ call sites (file.cpp,
// config.cpp) keep working unchanged — the C preprocessor requires
// literal operands for string-literal concatenation ("a" PSS "b" needs
// PSS to be a literal, not a namespace-qualified array). Full migration
// to fceu11::kPathSep(Str) is planned for v0.3.10 (API modernisation
// track).

#ifndef __FCEU_PLATFORM_COMPAT_H
#define __FCEU_PLATFORM_COMPAT_H

#include <cstdint>

namespace fceu11 {
	// Path separator constants — new API in v0.3.7.
	// The value is selected at compile time from the PSS_STYLE macro
	// (still injected by src/CMakeLists.txt -DPSS_STYLE=N for now). When
	// PSS_STYLE is 1 (POSIX) kPathSep becomes '/'; otherwise (2/3 = Windows,
	// 4 = classic Mac) it becomes '\\'. v0.3.10 will drop the PSS_STYLE
	// macro plumbing entirely in favour of std::filesystem::path::preferred_separator.
#if defined(PSS_STYLE) && PSS_STYLE == 1
	inline constexpr char kPathSep       = '/';
	inline constexpr char kPathSepStr[]  = "/";
#else
	inline constexpr char kPathSep       = '\\';
	inline constexpr char kPathSepStr[]  = "\\";
#endif
} // namespace fceu11

#ifdef _MSC_VER
	// POSIX-style name → MSVC name shims. Without these shims, every
	// call site that uses POSIX names would have to be rewritten with
	// #ifdef _MSC_VER, which the v0.2.30 codebase did not do. Keep
	// this block as a pure rename; do not add logic.
	#ifndef dup
		#define dup _dup
	#endif
	#ifndef stat
		#define stat _stat
	#endif
	#ifndef mkdir
		#define mkdir _mkdir
	#endif
	#ifndef alloca
		#define alloca _alloca
	#endif
	#ifndef FCEUX_fstat
		#define FCEUX_fstat _fstat
	#endif
	#if _MSC_VER < 1500
		#ifndef vsnprintf
			#define vsnprintf _vsnprintf
		#endif
	#endif

	// POSIX unistd.h access-mode macros. MSVC <io.h> uses _access(mode)
	// where the mode bits are 0/2/4/6 (read/write/rw/exist), not the
	// POSIX 0/1/2/3 (exist/write/read/exec) bit layout. The shims
	// preserve POSIX semantics at the FCEUX11 boundary.
	#ifndef W_OK
		#define W_OK 2
	#endif
	#ifndef R_OK
		#define R_OK 4
	#endif
	#ifndef X_OK
		#define X_OK 4   // MSVC <io.h> has no execute bit; treat read as execute
	#endif
	#ifndef F_OK
		#define F_OK 0
	#endif

	// MSVC <sys/stat.h> has no PATH_MAX. The standard Windows constant
	// is MAX_PATH (260). Keep the POSIX name to avoid touching call
	// sites that #include <limits.h> conditionally.
	#ifndef PATH_MAX
		#define PATH_MAX 260
	#endif
#endif // _MSC_VER

// PSS / PS — legacy path-separator macros from pre-v0.2.x FCEUX.
// MUST be string/char literals (not fceu11:: references) so they
// participate in C preprocessor string-literal concatenation. The
// existing call sites (src/file.cpp, src/drivers/Qt/config.cpp) rely
// on forms like `"%s" PSS "%s"` and `path + PS + name`; replacing
// PSS with `fceu11::kPathSepStr` (a const char[]) breaks concatenation
// because the preprocessor does not see the array as a string literal.
//
// v0.3.7 marks PSS/PS as legacy; new code should use
// fceu11::kPathSep (char) or fceu11::kPathSepStr (const char[]).
// The macros remain in place for v0.3.x and will be removed in v0.4.0
// (per plan v3 §6.3 deprecation flow).
#if defined(PSS_STYLE) && PSS_STYLE == 1
	#define PSS "/"
	#define PS  '/'
#else
	#define PSS "\\"
	#define PS  '\\'
#endif

#endif // __FCEU_PLATFORM_COMPAT_H
