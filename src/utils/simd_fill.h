// simd_fill.h
//
// hotfix2 P2-1 (ALIAS-1): byte-pattern fill helpers that replace the
// legacy FCEU_dwmemset macro. The macro had two problems:
//   1. Strict-aliasing UB — it casts a `uint8_t*` to `uint32_t*` and
//      stores through that pointer, which is UB on common ABIs (the
//      underlying dynamic type is `uint8_t`, and effective type rules
//      require `uint32_t`-aligned storage). The hotfix1 P2-5 (H-06)
//      followup fixed the in-TU call sites by switching to memcpy,
//      but the macro itself stayed unchanged.
//   2. Slow path — the reverse-counting `for` loop on a buffer of
//      `_x = n-4; _x >= 0; _x -= 4` produces N/4 4-byte stores.
//      AVX2 (`__AVX2__`) lets us collapse that to N/32 32-byte stores
//      (≈5-8× fewer memory transactions) for buffers ≥ 32 bytes.
//
// API: `fceu_dwmemset(dst, pattern, n_bytes)` — replicate the 4 bytes
// of `pattern` across `n_bytes` bytes. `n_bytes` MUST be a multiple of
// 4 (callers already guarantee this: 0x3c0, 0x40, 256, numtiles*8).
//
// Runtime AVX2 dispatch is gated by `fceu11::simd::have_avx2()`, which
// defaults to the compile-time `__AVX2__` switch in `compiler_attrs.h`.
// When the build is `-DFCEUX11_ASAN=ON` (or any sanitizer flag) MSVC
// still runs the AVX2 path correctly; the `<immintrin.h>` intrinsics
// do not conflict with /fsanitize=address. Older GCC/Clang
// configurations without `__AVX2__` defined fall through to the
// scalar memcpy loop automatically.

#pragma once

#include <cstdint>
#include <cstddef>
#include <cstring>

#include "compiler_attrs.h"

namespace fceu11 {

// hotfix2 P2-1: 4-byte-pattern byte fill, no aliasing UB. Inlined
// across all call sites via the static inline keyword + header-only
// definition so the compiler can constant-fold / loop-unroll /
// vectorise as it sees fit.
FCEU_ALWAYS_INLINE
inline void fceu_dwmemset(uint8_t* dst, uint32_t pattern, size_t n_bytes) noexcept {
    // n_bytes is always a positive multiple of 4 in all callers
    // (boards/mmc5.cpp uses 0x3c0 / 0x40 / 0x3C0 / 0x040; ppu_rendering.cpp
    // uses numtiles*8 / 256 / tcount*8). Branch on this once.
    if (n_bytes == 0) return;

#if defined(__AVX2__)
    if (fceu11::simd::have_avx2() && n_bytes >= 64) {
        // Broadcast the 4-byte pattern across a 256-bit lane. The
        // 8 lanes × 4 bytes per lane = 32 contiguous bytes per store
        // — matching `pattern` repeated 8 times. Using storeu (unaligned)
        // so XBuf row offsets (256 stride from `FCEU_amalloc` base, but
        // arbitrary within the buffer) stay correct.
        const __m256i v = _mm256_set1_epi32(static_cast<int>(pattern));
        size_t i = 0;
        for (; i + 32 <= n_bytes; i += 32) {
            _mm256_storeu_si256(reinterpret_cast<__m256i*>(dst + i), v);
        }
        // Tail (0..31 bytes — never >28 since n_bytes is a multiple of 4
        // and we've consumed 32-byte chunks first).
        for (; i < n_bytes; i += 4) {
            std::memcpy(dst + i, &pattern, 4);
        }
        return;
    }
#endif

    // Scalar fallback: 4-byte stride via memcpy. Guaranteed not to
    // trigger strict-aliasing warnings / runtime UB; compilers tend
    // to fuse back-to-back memcpy calls on `dst` into wider stores
    // anyway (LCG/SLP detects the pattern).
    for (size_t i = 0; i < n_bytes; i += 4) {
        std::memcpy(dst + i, &pattern, 4);
    }
}

}  // namespace fceu11
