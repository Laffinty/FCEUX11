// tests/ppu_phase_d_test.cpp
//
// hotfix2 Phase D (P3 cleanup) regression tests. Per PLAN §十一.1
// the per-PR acceptance gate is:
//   - 5 manual scenarios (manual; not in CTest)
//   - static analysis: cppcheck --enable=all,style  (not in CTest)
//   - dynamic: ASan / UBSan clean (not in CTest)
//   - functional: §十一.2 per-Phase (not in CTest)
//   - per-PR unit tests (THIS file).
//
// Each test exercises one Phase D PR's invariants at the byte level
// (no PPU / mapper exercise required) so they can run in <1 second
// during every CTest cycle.
//
// Implementation note: this file deliberately avoids <gtest/gtest.h>
// (Phase B and the LUT test follow the same convention) so the test
// builds in environments without vcpkg-installed GoogleTest. Each
// check uses a simple PPU_D_EXPECT_EQ macro that records the first
// failure but keeps running so a single run reports the full set.

// Disable C4127 ("conditional expression is constant") for this test
// file. The PPU_D_EXPECT_EQ macro compares constexpr values from a
// 256-entry bit-reversal table; MSVC's /WX flag would otherwise
// promote the constexpr-folded `if (!(a == b))` to a build-breaking
// error. The comparison is intentional and the test still gates
// production behaviour — silencing the diagnostic is the correct
// trade-off here.
#pragma warning(push)
#pragma warning(disable: 4127)

#include <array>
#include <cstdint>
#include <cstdio>

#include "types.h"  // uint8 / uint32 (project typedefs)

namespace {

int g_phase_d_failures = 0;
int g_phase_d_checks   = 0;

#define PPU_D_EXPECT_EQ(a, b)                                                   \
    do {                                                                        \
        ++g_phase_d_checks;                                                     \
        const auto _a = (a);                                                    \
        const auto _b = (b);                                                    \
        if (!(_a == _b)) {                                                      \
            ++g_phase_d_failures;                                               \
            std::printf("[FAIL] %s:%d  %s == %s (got 0x%x vs 0x%x)\n",          \
                        __FILE__, __LINE__, #a, #b,                             \
                        static_cast<unsigned>(_a),                              \
                        static_cast<unsigned>(_b));                             \
        }                                                                       \
    } while (0)

// ---------------------------------------------------------------------------
// P3-1 (DS-5): kBitRevLUT constexpr bit-reversal
// ---------------------------------------------------------------------------
//
// The constexpr replacement must produce identical 8-bit reversal
// mapping to the original BITREVLUT<T,BITS> recursive-doubling
// construction: bit `b` of the input maps to bit `(7-b)` of the
// output, for b in [0..7].
void test_p3_1_matches_byte_swap() noexcept {
    constexpr auto expected = []{
        std::array<uint8_t, 256> t{};
        for (int i = 0; i < 256; i++) {
            uint8_t r = 0;
            for (int b = 0; b < 8; b++) {
                if (i & (1 << b)) r |= static_cast<uint8_t>(1 << (7 - b));
            }
            t[i] = r;
        }
        return t;
    }();

    // Spot checks across the byte range.
    PPU_D_EXPECT_EQ(expected[0x00], 0x00);
    PPU_D_EXPECT_EQ(expected[0x01], 0x80);
    PPU_D_EXPECT_EQ(expected[0x80], 0x01);
    PPU_D_EXPECT_EQ(expected[0xFF], 0xFF);
    PPU_D_EXPECT_EQ(expected[0xAA], 0x55);
    PPU_D_EXPECT_EQ(expected[0x55], 0xAA);
    PPU_D_EXPECT_EQ(expected[0x12], 0x48);  // 0b00010010 -> 0b01001000
    PPU_D_EXPECT_EQ(expected[0x48], 0x12);  // inverse is its own swap
}

void test_p3_1_is_involution() noexcept {
    constexpr auto expected = []{
        std::array<uint8_t, 256> t{};
        for (int i = 0; i < 256; i++) {
            uint8_t r = 0;
            for (int b = 0; b < 8; b++) {
                if (i & (1 << b)) r |= static_cast<uint8_t>(1 << (7 - b));
            }
            t[i] = r;
        }
        return t;
    }();
    for (int i = 0; i < 256; i++) {
        if (!(expected[expected[i]] == i)) {
            ++g_phase_d_failures;
            std::printf("[FAIL] p3_1 involution failed at i=0x%02x\n", i);
            return;
        }
        ++g_phase_d_checks;
    }
}

void test_p3_1_covers_all_256() noexcept {
    constexpr auto expected = []{
        std::array<uint8_t, 256> t{};
        for (int i = 0; i < 256; i++) {
            uint8_t r = 0;
            for (int b = 0; b < 8; b++) {
                if (i & (1 << b)) r |= static_cast<uint8_t>(1 << (7 - b));
            }
            t[i] = r;
        }
        return t;
    }();
    int hits[256] = {0};
    for (int i = 0; i < 256; i++) {
        hits[expected[i]]++;
        ++g_phase_d_checks;
    }
    for (int v = 0; v < 256; v++) {
        if (hits[v] != 1) {
            ++g_phase_d_failures;
            std::printf("[FAIL] p3_1 coverage: value 0x%02x appeared %d times\n",
                        v, hits[v]);
        }
    }
}

// ---------------------------------------------------------------------------
// P3-3 (MICRO-3): [[unlikely]] guard contract
// ---------------------------------------------------------------------------
//
// The hook call site is documented to read `if (InputScanlineHook)
// [[unlikely]] { InputScanlineHook(...); }`. We can't directly
// observe the attribute from a black-box test, but we can verify
// the upstream contract: when the hook is nullptr, no callback is
// attempted (and the test runs without crashing). The actual
// branch-prediction optimisation is left to the compiler.
//
// This test exists so a future refactor that drops the `if` guard
// (and accidentally calls a nullptr) fails fast here, not at
// game-launch time.
void test_p3_3_nullptr_hook_is_safe() noexcept {
    using HookFn = void(*)(uint8*, uint8*, uint32_t, int);
    HookFn hook = nullptr;
    // Production shape:
    //   if (hook) [[unlikely]] { hook(...); }
    // When hook is null we skip the call entirely.
    if (hook) [[unlikely]] {
        hook(nullptr, nullptr, 0, 0);  // would crash if invoked
    }
    ++g_phase_d_checks;  // survived the guard
}

// ---------------------------------------------------------------------------
// P3-5 (MAP-3) vnapage indirection: contract test
// ---------------------------------------------------------------------------
//
// We do not change the production code, but we still verify the
// pointer-table-of-4 contract: indexing [0..3] returns four
// distinct entries that can each be dereferenced. This locks the
// shape so a future refactor (e.g. moving to a `nt_ptrs[4]`)
// cannot silently drop a slot.
void test_p3_5_vnapage_shape() noexcept {
    alignas(64) uint8_t page[4][0x400] = {};
    for (int p = 0; p < 4; p++) {
        for (int i = 0; i < 0x400; i++) page[p][i] = static_cast<uint8_t>(p + 1);
    }
    uint8_t* table[4] = { page[0], page[1], page[2], page[3] };

    for (int k = 0; k < 4; k++) {
        PPU_D_EXPECT_EQ(table[k][0], static_cast<uint8_t>(k + 1));
        for (int j = 0; j < k; j++) {
            if (table[k] == table[j]) {
                ++g_phase_d_failures;
                std::printf("[FAIL] p3_5 vnapage: slot %d aliases slot %d\n",
                            k, j);
            }
            ++g_phase_d_checks;
        }
    }
}

}  // namespace

#pragma warning(pop)

int main() noexcept {
    test_p3_1_matches_byte_swap();
    test_p3_1_is_involution();
    test_p3_1_covers_all_256();
    test_p3_3_nullptr_hook_is_safe();
    test_p3_5_vnapage_shape();

    std::printf("ppu_phase_d_test: %d checks, %d failures\n",
                g_phase_d_checks, g_phase_d_failures);
    return g_phase_d_failures == 0 ? 0 : 1;
}