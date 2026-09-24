// FCEUX11 v0.3.8 — FCEU_ENUM_CLASS_BITFLAGS macro unit tests
//
// Verifies the macro-generated bitwise operators and helpers
// (operator|/&/^/~, |=/&=/^=, has/set/clear) for fceu11::CpuFlag.
// Pure compile-time / constexpr test — no NES core initialization
// needed. Follows the project's existing printf-asserts test style
// (see tests/expected_api_test.cpp).

#include <cstdio>
#include <cstdint>
#include <type_traits>

#include "utils/enum_class_bitflags.h"

using fceu11::CpuFlag;

// 1. operator| combines two flags
static bool test_or_basic() {
    printf("[test] operator| ... ");
    constexpr CpuFlag both = CpuFlag::N | CpuFlag::Z;
    if constexpr (static_cast<uint8_t>(both) != 0x82) {
        printf("FAIL (got 0x%02x, expected 0x82)\n", static_cast<uint8_t>(both));
        return false;
    }
    printf("OK\n");
    return true;
}

// 2. operator| chains
static bool test_or_chain() {
    printf("[test] operator| chain ... ");
    constexpr CpuFlag all_three = CpuFlag::N | CpuFlag::V | CpuFlag::C;
    if constexpr (static_cast<uint8_t>(all_three) != 0xC1) {
        printf("FAIL (got 0x%02x, expected 0xC1)\n", static_cast<uint8_t>(all_three));
        return false;
    }
    printf("OK\n");
    return true;
}

// 3. operator& masks
static bool test_and() {
    printf("[test] operator& ... ");
    constexpr CpuFlag v = (CpuFlag::N | CpuFlag::Z | CpuFlag::C) & CpuFlag::Z;
    if constexpr (static_cast<uint8_t>(v) != 0x02) {
        printf("FAIL (got 0x%02x)\n", static_cast<uint8_t>(v));
        return false;
    }
    printf("OK\n");
    return true;
}

// 4. operator~ inverts (within the underlying-type width)
static bool test_not() {
    printf("[test] operator~ ... ");
    constexpr CpuFlag inv = ~CpuFlag::N;       // ~0x80 = 0x7F (uint8_t)
    if constexpr (static_cast<uint8_t>(inv) != 0x7F) {
        printf("FAIL (got 0x%02x, expected 0x7F)\n", static_cast<uint8_t>(inv));
        return false;
    }
    printf("OK\n");
    return true;
}

// 5. operator^ toggles
static bool test_xor() {
    printf("[test] operator^ ... ");
    constexpr CpuFlag a = CpuFlag::N | CpuFlag::Z;        // 0x82
    constexpr CpuFlag b = CpuFlag::N | CpuFlag::C;        // 0x81
    constexpr CpuFlag x = a ^ b;                          // 0x03
    if constexpr (static_cast<uint8_t>(x) != 0x03) {
        printf("FAIL (got 0x%02x, expected 0x03)\n", static_cast<uint8_t>(x));
        return false;
    }
    printf("OK\n");
    return true;
}

// 6. has() probes
static bool test_has() {
    printf("[test] has ... ");
    constexpr CpuFlag v = CpuFlag::N | CpuFlag::C;
    if (!has(v, CpuFlag::N)) { printf("FAIL (N missing)\n"); return false; }
    if (!has(v, CpuFlag::C)) { printf("FAIL (C missing)\n"); return false; }
    if ( has(v, CpuFlag::Z)) { printf("FAIL (Z present)\n"); return false; }
    // Multi-flag query: has(v, N|C) must require BOTH set.
    if (!has(v, CpuFlag::N | CpuFlag::C)) {
        printf("FAIL (N|C not detected)\n");
        return false;
    }
    if ( has(v, CpuFlag::N | CpuFlag::Z)) {
        printf("FAIL (N|Z falsely detected)\n");
        return false;
    }
    printf("OK\n");
    return true;
}

// 7. set() / clear() mutate
static bool test_set_clear() {
    printf("[test] set/clear ... ");
    CpuFlag v = CpuFlag::N;
    set(v, CpuFlag::Z);
    if (static_cast<uint8_t>(v) != (0x80 | 0x02)) {
        printf("FAIL set (got 0x%02x)\n", static_cast<uint8_t>(v));
        return false;
    }
    clear(v, CpuFlag::N);
    if (static_cast<uint8_t>(v) != 0x02) {
        printf("FAIL clear (got 0x%02x)\n", static_cast<uint8_t>(v));
        return false;
    }
    printf("OK\n");
    return true;
}

// 8. Compound-assignment operators
static bool test_compound_assign() {
    printf("[test] |= &= ^= ... ");
    CpuFlag v = CpuFlag::N;
    v |= CpuFlag::Z;
    if (static_cast<uint8_t>(v) != 0x82) { printf("FAIL |=\n"); return false; }
    v &= CpuFlag::Z;
    if (static_cast<uint8_t>(v) != 0x02) { printf("FAIL &=\n"); return false; }
    v ^= CpuFlag::C;
    if (static_cast<uint8_t>(v) != 0x03) { printf("FAIL ^=\n"); return false; }
    printf("OK\n");
    return true;
}

// 9. Underlying type and width invariants
static bool test_underlying_type() {
    printf("[test] underlying_type / size ... ");
    static_assert(std::is_same_v<std::underlying_type_t<CpuFlag>, uint8_t>,
                  "CpuFlag underlying type must be uint8_t");
    static_assert(sizeof(CpuFlag) == 1, "CpuFlag must be 1 byte");
    printf("OK (uint8_t, 1 byte)\n");
    return true;
}

int main() {
    int failures = 0;
    failures += !test_or_basic();
    failures += !test_or_chain();
    failures += !test_and();
    failures += !test_not();
    failures += !test_xor();
    failures += !test_has();
    failures += !test_set_clear();
    failures += !test_compound_assign();
    failures += !test_underlying_type();

    if (failures == 0) {
        printf("\nAll FCEU_ENUM_CLASS_BITFLAGS tests passed.\n");
        return 0;
    }
    printf("\n%d FCEU_ENUM_CLASS_BITFLAGS tests FAILED.\n", failures);
    return 1;
}
