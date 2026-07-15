// compiler_attrs.h
//
// Cross-compiler macros for hot-path attributes. Per hotfix2 PLAN §16.6
// (审计新增 2026-07-15): `[[gnu::always_inline]]` / `__attribute__((hot))`
// / `[[clang::preserve_none]]` are GCC/Clang extensions, MSVC does not
// recognise them. The macros here normalise everything so `src/`
// hot-path code stays portable across MSVC / Clang / GCC.

#ifndef FCEU11_COMPILER_ATTRS_H
#define FCEU11_COMPILER_ATTRS_H

// Force-inline. The hot PPU rendering inner loops must not be split
// across translation units at -O2; MSVC equivalent is __forceinline.
#if defined(__GNUC__) || defined(__clang__)
  #define FCEU_ALWAYS_INLINE inline __attribute__((always_inline))
#elif defined(_MSC_VER)
  #define FCEU_ALWAYS_INLINE __forceinline
#else
  #define FCEU_ALWAYS_INLINE inline
#endif

// Hint that this function is on a hot path. GCC/Clang lay the function
// out in .text.hot; MSVC has no equivalent and relies on /O2 + /LTCG +
// PGO. Leave MSVC macro empty so MSVC users see no spurious expansion.
#if defined(__GNUC__) || defined(__clang__)
  #define FCEU_HOT __attribute__((hot))
#elif defined(_MSC_VER)
  #define FCEU_HOT
#else
  #define FCEU_HOT
#endif

// Cold path hint (kept for symmetry; the marking is mirrored via
// __attribute__((cold)) by GCC/Clang, MSVC ignores).
#if defined(__GNUC__) || defined(__clang__)
  #define FCEU_COLD __attribute__((cold))
#elif defined(_MSC_VER)
  #define FCEU_COLD
#else
  #define FCEU_COLD
#endif

// Branch-prediction hints. C++20 [[likely]]/[[unlikely]] is supported
// on MSVC 19.30+; for older compilers fall back to builtin_expect.
#if defined(__GNUC__) || defined(__clang__)
  #define FCEU_LIKELY(x)   (__builtin_expect(!!(x), 1))
  #define FCEU_UNLIKELY(x) (__builtin_expect(!!(x), 0))
#elif defined(_MSC_VER)
  // MSVC 19.30+ supports [[likely]]/[[unlikely]] natively. For older
  // versions of MSVC the hint is dropped silently.
  #define FCEU_LIKELY(x)   (x)
  #define FCEU_UNLIKELY(x) (x)
#else
  #define FCEU_LIKELY(x)   (x)
  #define FCEU_UNLIKELY(x) (x)
#endif

// AVX2 intrinsic header — centralising it here keeps MSVC and Clang
// happy (they include via the same <immintrin.h>; GCC prefers
// <x86intrin.h> but accepts <immintrin.h> too).
#if defined(__AVX2__)
  #include <immintrin.h>
#endif

// Compile-time-known 64-bit byte-reverse. Used by P0-1 to flip
// pixel byte order for H_FLIP without indexed loads from packed
// data. MSVC's `_byteswap_uint64` is the same intrinsic as
// `__builtin_bswap64` on GCC/Clang; they all map to a single
// `bswap`/`movbe` instruction at -O2.
#if defined(__GNUC__) || defined(__clang__)
  #define FCEU_BSWAP64(x) __builtin_bswap64(x)
#elif defined(_MSC_VER)
  #include <intrin.h>
  #define FCEU_BSWAP64(x) _byteswap_uint64(x)
#else
  // Fallback: shift-and-or byte-reverse. Compiler will still
  // collapse to a single bswap at -O2 on x86 targets.
  inline uint64_t FCEU_BSWAP64(uint64_t v) noexcept {
      v = ((v & 0xFFFFFFFF00000000ULL) >> 32) | ((v & 0x00000000FFFFFFFFULL) << 32);
      v = ((v & 0xFFFF0000FFFF0000ULL) >> 16) | ((v & 0x0000FFFF0000FFFFULL) << 16);
      v = ((v & 0xFF00FF00FF00FF00ULL) >> 8)  | ((v & 0x00FF00FF00FF00FFULL) << 8);
      return v;
  }
#endif

// Runtime AVX2 availability (P0-5 / P2-1). Defined as `false` until
// fceu11::simd::have_avx2() is called from main() once. Default to
// compile-time __AVX2__ to keep early builds without dispatcher
// branching; flipped off by simd::probe_cpu_features() in main().
namespace fceu11::simd {
inline bool have_avx2() noexcept {
#if defined(__AVX2__)
    return true;  // conservative — runtime probe done in main()
#else
    return false;
#endif
}
}  // namespace fceu11::simd

#endif  // FCEU11_COMPILER_ATTRS_H
