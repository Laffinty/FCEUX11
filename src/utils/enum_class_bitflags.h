// FCEUX11 — Bitflag helpers for scoped enumerations (v0.3.8)
//
// Per docs/v0.3.x_Construction_Plan_v3.md §5 v0.3.8 task 5: introduce a
// FCEU_ENUM_CLASS_BITFLAGS(E) macro (akin to magic_enum's bit-flags
// pattern) that auto-generates the bitwise operator set for a scoped
// enumeration so callers can write idiomatic
//   CpuFlag f = CpuFlag::N | CpuFlag::Z;
//   if (has(f, CpuFlag::N)) ...
// without writing one-off operator overloads per enum.
//
// The macro is SFINAE-restricted to scoped enumerations: plain `enum`
// types whose values implicitly convert to int already accept the
// builtin bitwise operators (with poorly-typed semantics) — applying
// this macro to such types is almost always a programming error, so
// we reject them with a clear static_assert.
//
// Intentional non-goal (per plan §5 v0.3.8 final sentence about PPU[0..2]
// not being enum-class-ified): the macro is NOT consumed by the
// x6502.cpp hot path's _P|=Z_FLAG / _P&=~C_FLAG patterns (~600 sites
// across the CPU dispatch macros). Those patterns continue to use the
// pre-existing #define N_FLAG..C_FLAG masks in src/x6502.h, and CpuFlag
// declared below is provided strictly as the macro's correctness
// witness (exercised in tests/enum_class_bitflags_test.cpp).
//
// Migration path for future code: when a NEW enum class needs OR-able
// flag semantics (anticipated for v0.3.13 input plugins, v0.3.14 OpenGL
// render flags, and v0.4.0 mapper concepts), declare it as
//   enum class MyFlag : uint8_t { A=0x01, B=0x02, ... };
//   FCEU_ENUM_CLASS_BITFLAGS(MyFlag);
// at namespace scope (the macro emits free functions so it must NOT
// sit inside a class body).

#ifndef FCEU11_ENUM_CLASS_BITFLAGS_H
#define FCEU11_ENUM_CLASS_BITFLAGS_H

#include <cstdint>
#include <type_traits>

#define FCEU_ENUM_CLASS_BITFLAGS(E)                                            \
    static_assert(std::is_enum_v<E>,                                           \
        "FCEU_ENUM_CLASS_BITFLAGS requires an enum type");                     \
    static_assert(!std::is_convertible_v<E, int>,                              \
        "FCEU_ENUM_CLASS_BITFLAGS requires a scoped enum "                     \
        "(use 'enum class', not plain 'enum')");                               \
    [[nodiscard]] constexpr E operator|(E a, E b) noexcept {                   \
        using U = std::underlying_type_t<E>;                                   \
        return static_cast<E>(static_cast<U>(a) | static_cast<U>(b));          \
    }                                                                          \
    [[nodiscard]] constexpr E operator&(E a, E b) noexcept {                   \
        using U = std::underlying_type_t<E>;                                   \
        return static_cast<E>(static_cast<U>(a) & static_cast<U>(b));          \
    }                                                                          \
    [[nodiscard]] constexpr E operator^(E a, E b) noexcept {                   \
        using U = std::underlying_type_t<E>;                                   \
        return static_cast<E>(static_cast<U>(a) ^ static_cast<U>(b));          \
    }                                                                          \
    [[nodiscard]] constexpr E operator~(E a) noexcept {                        \
        using U = std::underlying_type_t<E>;                                   \
        return static_cast<E>(~static_cast<U>(a));                             \
    }                                                                          \
    constexpr E& operator|=(E& a, E b) noexcept { a = a | b; return a; }       \
    constexpr E& operator&=(E& a, E b) noexcept { a = a & b; return a; }       \
    constexpr E& operator^=(E& a, E b) noexcept { a = a ^ b; return a; }       \
    [[nodiscard]] constexpr bool has(E v, E f) noexcept {                      \
        using U = std::underlying_type_t<E>;                                   \
        return (static_cast<U>(v) & static_cast<U>(f)) == static_cast<U>(f);   \
    }                                                                          \
    constexpr void set(E& v, E f) noexcept { v = v | f; }                      \
    constexpr void clear(E& v, E f) noexcept { v = v & ~f; }                   \
    /* Trailing semicolon eater: lets call sites end with ';' as expected. */ \
    using FCEU_ENUM_CLASS_BITFLAGS__##E##_marker = void

namespace fceu11 {

// 6502 status-register (P) flag bits. Provided here as the canonical
// FCEU_ENUM_CLASS_BITFLAGS test subject; the live CPU emulator in
// src/x6502.cpp continues to use the equivalent #define N_FLAG..C_FLAG
// masks from src/x6502.h, exactly as plan §5 v0.3.8 specifies
// ("PPU[0..2] 的位标志不做 enum class 化（保持 uint8 + constexpr 掩码）"
// — same rationale applied to the P register).
//
// Bit layout matches the 6502 hardware ordering (MSB to LSB):
//   N V U B  D I Z C
enum class CpuFlag : uint8_t {
    N = 0x80,  // Negative
    V = 0x40,  // Overflow
    U = 0x20,  // Unused (always reads as 1)
    B = 0x10,  // Break
    D = 0x08,  // Decimal mode (unused on 2A03)
    I = 0x04,  // IRQ disable
    Z = 0x02,  // Zero
    C = 0x01,  // Carry
};

FCEU_ENUM_CLASS_BITFLAGS(CpuFlag);

} // namespace fceu11

#endif // FCEU11_ENUM_CLASS_BITFLAGS_H
