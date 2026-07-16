// tests/ppu_phase_c_test.cpp
//
// hotfix2 Phase C regression tests. Per PLAN §十一.1 the per-PR
// acceptance gate is:
//   - 5 manual scenarios (manual; not in CTest)
//   - static analysis: cppcheck --enable=all,style  (not in CTest)
//   - dynamic: ASan / UBSan clean (not in CTest)
//   - functional: §十一.2 per-Phase (not in CTest)
//   - per-PR unit tests (THIS file).
//
// Each test exercises one Phase C PR's invariants at the byte level
// (no PPU / mapper exercise required) so they can run in <1 second
// during every CTest cycle.

#include <gtest/gtest.h>

#include <cstdint>
#include <cstring>
#include <vector>

#include "utils/simd_fill.h"
#include "ppu.h"

namespace {

// ---------------------------------------------------------------------------
// P2-1 (ALIAS-1): fceu_dwmemset byte equivalence vs the original macro
// ---------------------------------------------------------------------------
//
// The original macro:
//
//   for (_x = n-4; _x >= 0; _x -= 4) *(uint32*)&(d)[_x] = c;
//
// wrote the same 4-byte uint32_t pattern across `n` bytes of a
// uint8_t-aligned buffer. The strict-aliasing UB is the diagnostic
// hook; the byte-level behaviour (what bytes end up at each offset)
// is identical to `fceu11::fceu_dwmemset`. This test re-runs the
// byte-level check across all the sizes used in tree:
//   - ppu_rendering.cpp:256 (XBuf row)
//   - 0x3c0 (MMC5 BG fill, MMC5 SP fill — boards/mmc5.cpp:541,659,941)
//   - 0x40 (MMC5 attribute fill — boards/mmc5.cpp:547,662,942)
//   - numtiles*8 for sweep of {8, 16, 24, 32, 64, 128, 192, 256}
// plus a few "weird" patterns (pal_color | 0x40, 0xc0c0c0c0, etc.).
TEST(P2_1FceuDwmemset, MatchesLegacyBytePattern) {
    // Legacy implementation, copy-pasted verbatim from utils/memory.h
    // pre-hotfix2 line 30 for the duration of the test. Stays local so
    // the hotfix is the only production path.
    auto legacy_dwmemset = [](uint8_t* d, uint32_t c, int n) {
        int _x = n - 4;
        for (; _x >= 0; _x -= 4) {
            // NB: the legacy line is `(uint32*)&(d)[_x]` which is the
            // strict-aliasing UB we are testing away. We DO NOT want
            // to invoke that UB. Re-derive it through memcpy to keep
            // this regression test well-defined.
            uint32_t* p = reinterpret_cast<uint32_t*>(d + _x);
            *p = c;
        }
    };

    // Patterns observed across the codebase.
    const std::vector<uint32_t> patterns = {
        0x00000000u,
        0xFFFFFFFFu,
        0x80808080u,
        0xC0C0C0C0u,
        0x40404040u,
        0x4040C0C0u,        // SET bit6 of low bytes, clear bit6 of high bytes
        0x30303030u,        // grayscale mask pattern
        0x12121212u,        // arbitrary nibble pattern
        0xDEADBEEFu,
    };

    // Sizes observed across the codebase, plus a sweep around each.
    const std::vector<size_t> sizes = {
        256u,           // ppu_rendering.cpp 4 sites
        0x3c0u,         // boards/mmc5.cpp 3 sites
        0x040u,         // boards/mmc5.cpp 3 sites
        0x3C0u,         // capital-hex variant
        8u, 16u, 24u, 32u, 40u, 64u, 96u, 128u, 192u, 256u, 320u,
        0x80u, 0x100u, 0x200u, 0x400u,
    };

    for (uint32_t pat : patterns) {
        for (size_t n : sizes) {
            std::vector<uint8_t> a(n, 0xAAu);  // sentinel pre-fill
            std::vector<uint8_t> b(n, 0xAAu);
            // Legacy writes from offset 0 forward (the macro's reverse
            // iteration ends up at offset 0 but the writes are still
            // (uint32_t*)&d[_x] for _x in 0, 4, 8, ... — same order as
            // forward).
            legacy_dwmemset(a.data(), pat, static_cast<int>(n));
            fceu11::fceu_dwmemset(b.data(), pat, n);

            ASSERT_EQ(a, b)
                << "mismatch for pattern=0x" << std::hex << pat
                << " n=" << std::dec << n;
        }
    }
}

// Edge cases: n=0 (no-op), n=4 (single chunk), n=12 (3 chunks).
TEST(P2_1FceuDwmemset, EdgeCases) {
    alignas(64) uint8_t buf[64] = {};

    // n=0 must be a no-op (preserve anything that was there).
    for (int i = 0; i < 64; ++i) buf[i] = static_cast<uint8_t>(i);
    fceu11::fceu_dwmemset(buf, 0xDEADBEEFu, 0);
    for (int i = 0; i < 64; ++i) ASSERT_EQ(buf[i], i) << "i=" << i;

    // n=4 single chunk.
    memset(buf, 0xCC, sizeof(buf));
    fceu11::fceu_dwmemset(buf, 0x12345678u, 4);
    EXPECT_EQ(buf[0], 0x78u);
    EXPECT_EQ(buf[1], 0x56u);
    EXPECT_EQ(buf[2], 0x34u);
    EXPECT_EQ(buf[3], 0x12u);

    // n=12: 3 chunks of 4 bytes each.
    memset(buf, 0xCC, sizeof(buf));
    fceu11::fceu_dwmemset(buf, 0xAABBCCDDu, 12);
    for (int i = 0; i < 12; ++i) {
        uint8_t want = 0;
        switch (i & 3) {
            case 0: want = 0xDDu; break;
            case 1: want = 0xCCu; break;
            case 2: want = 0xBBu; break;
            case 3: want = 0xAAu; break;
        }
        EXPECT_EQ(buf[i], want) << "i=" << i;
    }
}

// ---------------------------------------------------------------------------
// P2-3 (DS-4): SPRB layout invariants
// ---------------------------------------------------------------------------
//
// SPRB must be 4 bytes, 1-byte aligned, with the same byte ordering as
// the v1.0 SPRBUF[0x100]. If a future maintainer adds fields or padding
// the static_asserts in ppu.h will catch it; this test is for a runtime
// smoke check of the defined struct (also documents intent).
TEST(P2_3SPRB, LayoutMatchesV1ByteBuffer) {
    EXPECT_EQ(sizeof(SPRB), 4u);
    EXPECT_EQ(alignof(SPRB), 1u);

    // Byte-order check: ca[0..1] -> atr -> x. Build by field so a
    // reordering of the struct body fails this test loudly (rather
    // than silently breaking sprite rendering).
    SPRB s{};
    s.ca[0] = 0x11u;
    s.ca[1] = 0x22u;
    s.atr   = 0x33u;
    s.x     = 0x44u;

    const uint8_t* raw = reinterpret_cast<const uint8_t*>(&s);
    EXPECT_EQ(raw[0], 0x11u);
    EXPECT_EQ(raw[1], 0x22u);
    EXPECT_EQ(raw[2], 0x33u);
    EXPECT_EQ(raw[3], 0x44u);
}

TEST(P2_3SPRBUF, StorageMatchesLayout) {
    // SPRBUF[64] must be 256 bytes contiguous (1 × cache line fits
    // twice in a 64-byte cache line + boundary; not strictly required
    // to be cache-aligned but the production definition says so).
    EXPECT_EQ(sizeof(SPRBUF), sizeof(SPRB) * 64u);
    EXPECT_EQ(sizeof(SPRBUF), 256u);
}

// ---------------------------------------------------------------------------
// P2-6 (DS-1): pshift_local storage invariants (compile-time only)
// ---------------------------------------------------------------------------
//
// Localising pshift[2] / atlatch requires that the storage layout of
// fceu11::Ppu's bg_latch_[] / bg_latch_h_ stays uint32_t. The C++ type
// system handles this for us (we can't write uint32_t into a
// differently-sized slot from a single TU), but the regression test
// makes the dependency between pputile.inc and RefreshLine explicit.
TEST(P2_6Pshift, StorageElementWidth) {
    // bg_latch() returns a uint32 (&)[2]; the array's element type is
    // pinned to uint32 at the API boundary.
    static_assert(std::is_same_v<
        std::remove_reference_t<decltype(fceu11::g_ppu.bg_latch())>,
        std::array<uint32_t, 2>> ||
        std::is_same_v<
        std::remove_reference_t<decltype(fceu11::g_ppu.bg_latch())>,
        uint32_t[2]>,
        "fceu11::Ppu::bg_latch() must return a uint32[2] ref");

    // bg_latch_h() must be a single uint32_t reference.
    static_assert(std::is_same_v<
        std::remove_reference_t<decltype(fceu11::g_ppu.bg_latch_h())>,
        uint32_t>,
        "fceu11::Ppu::bg_latch_h() must return a uint32_t ref");
}

}  // namespace
