// FCEUX11 v0.3.12 — cache-line and prefetch helpers.
//
// MSVC 19.36 + C++20.  std::hardware_destructive_interference_size is used
// when available; on every x64 Windows target we care about it evaluates to
// 64 bytes, so the fallback is also 64.

#ifndef FCEUX11_UTILS_CACHE_H
#define FCEUX11_UTILS_CACHE_H

#include <cstddef>
#include <new>
#include <immintrin.h>

namespace fceu11 {

inline constexpr std::size_t kCacheLineSize =
#ifdef __cpp_lib_hardware_interference_size
    std::hardware_destructive_interference_size;
#else
    64;
#endif

} // namespace fceu11

// Place an object / array at a cache-line boundary.
// Use the leading `::` so the macro works correctly when expanded
// inside `namespace fceu11 { ... }` blocks (otherwise the qualified
// name `fceu11::kCacheLineSize` resolves to `fceu11::fceu11::...`
// which doesn't exist). All existing call sites in cart.cpp,
// fceu.cpp, bus.cpp etc. are at file scope, where `::fceu11::` and
// `fceu11::` resolve to the same symbol, so this change is
// backward-compatible.
#define FCEUX11_CACHE_ALIGN alignas(::fceu11::kCacheLineSize)

// Mark a mapper register group as hot for cache-line alignment.
#define FCEUX11_MAPPER_HOT FCEUX11_CACHE_ALIGN

// Software prefetch hint.  addr is evaluated once.
#define FCEUX11_PREFETCH(addr) \
    _mm_prefetch(reinterpret_cast<const char*>(addr), _MM_HINT_T0)

#endif // FCEUX11_UTILS_CACHE_H
