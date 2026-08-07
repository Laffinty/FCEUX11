// ppu_rendering_lut_test.cpp
//
// hotfix2 P0-1 (ARCH-2): byte-for-byte equivalence test for the
// two-stage sprite decode LUT.
//
// Strategy:
//   1. Recompute the original `pixdata` and `J` for every (ca0, ca1)
//      pair using the same maths as ppu_rendering.cpp:866-867.
//   2. For each of the four (SP_BACK × H_FLIP) paths, walk through
//      the 8-pixel sequence and write expected outputs to a scratch
//      buffer.
//   3. Use the new LUT to produce actual outputs.
//   4. Compare buffer bytes; report first mismatch.
//
// Coverage: full 65536 × 4-path grid is exhaustive and finishes in
// < 1 second on a modern CPU.

#include <cstdio>
#include <cstdint>
#include <cstdlib>
#include <array>

#include "ppu_sprite_lut.h"
#include "ppu_rendering.h"  // ppulut1/2/3 declarations
#include "compiler_attrs.h"

namespace {

// Recreate the original SP_BACK × H_FLIP decode against a synthetic
// PALRAM and a single sprite's VB. The PAL table is parameterised so
// each test cell can use a unique palette (avoids accidental aliasing
// of identical palettes masking structural bugs).
struct DecodeCase {
    uint8_t pal[4];
    uint8_t atr;        // SP_BACK, H_FLIP bits
    uint8_t VB_base;
    uint8_t pal_mask;   // GRAYSCALE effect (0x30 or 0xFF)
};

constexpr uint8_t SP_BACK = 0x20;
constexpr uint8_t H_FLIP  = 0x40;

FCEU_ALWAYS_INLINE void decode_original(
    uint8_t ca0, uint8_t ca1, const DecodeCase& c, uint8_t out[8]) noexcept
{
    // Per ppu_rendering.cpp:866-867
    uint32_t pixdata = ppulut1[ca0] | ppulut2[ca1];
    uint8_t J = ca0 | ca1;
    uint8_t atr = c.atr;

    if (!J) {
        for (int i = 0; i < 8; ++i) out[i] = 0xFFu;   // sentinel: untouched
        return;
    }

    for (int i = 0; i < 8; ++i) {
        out[i] = 0xFFu;
    }

    uint8_t palread[4];
    for (int i = 0; i < 4; ++i) {
        palread[i] = c.pal[i] & c.pal_mask;
    }
    uint8_t back_or = (atr & SP_BACK) ? 0x40 : 0x00;

    // Mirrors the four code paths in ppu_rendering.cpp:888-955.
    if (atr & H_FLIP) {
        if (J & 0x80) { out[7] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x40) { out[6] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x20) { out[5] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x10) { out[4] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x08) { out[3] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x04) { out[2] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x02) { out[1] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x01) { out[0] = (palread[pixdata & 3] | back_or); }
    } else {
        if (J & 0x80) { out[0] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x40) { out[1] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x20) { out[2] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x10) { out[3] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x08) { out[4] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x04) { out[5] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x02) { out[6] = (palread[pixdata & 3] | back_or); }
        pixdata >>= 4;
        if (J & 0x01) { out[7] = (palread[pixdata & 3] | back_or); }
    }
}

FCEU_ALWAYS_INLINE void decode_lut(
    uint8_t ca0, uint8_t ca1, const DecodeCase& c, uint8_t out[8]) noexcept
{
    uint8_t atr = c.atr;
    uint8_t J = ca0 | ca1;
    if (!J) {
        for (int i = 0; i < 8; ++i) out[i] = 0xFFu;
        return;
    }

    uint8_t pal_tab_op[4], pal_tab_bk[4];
    for (int i = 0; i < 4; ++i) {
        uint8_t cv = c.pal[i] & c.pal_mask;
        pal_tab_op[i] = cv;
        pal_tab_bk[i] = cv | 0x40;
    }
    const uint8_t* pal_tab = (atr & SP_BACK) ? pal_tab_bk : pal_tab_op;

    uint64_t packed = fceu11::ppu::kSpriteIdxLUT[
        (uint32_t)ca0 | ((uint32_t)ca1 << 8)];
    if (atr & H_FLIP) packed = FCEU_BSWAP64(packed);
    const uint8_t* idx = reinterpret_cast<const uint8_t*>(&packed);

    for (int i = 0; i < 8; ++i) {
        // idx[i] & 0x80 = visibility; idx[i] & 0x03 = 2-bit colour.
        // Mirror the production RefreshSprites semantics: byte is
        // untouched (sentinel 0xFF) when visibility is clear.
        out[i] = (idx[i] & 0x80) ? pal_tab[idx[i] & 0x03] : 0xFFu;
    }
}

// ppulut1/2/3 + makeppulut are external declarations from
// ppu_rendering.h. The test link pulls in ppu_rendering.cpp which
// owns the real definitions. Nothing to re-declare here.

}  // namespace

static int run_grid(const DecodeCase& c)
{
    uint8_t expected[8];
    uint8_t actual[8];
    int mismatches = 0;
    int first_mismatch_input = -1;
    int first_mismatch_pos = -1;
    uint8_t first_expected = 0;
    uint8_t first_actual = 0;

    for (uint32_t ca0 = 0; ca0 < 256; ++ca0) {
        for (uint32_t ca1 = 0; ca1 < 256; ++ca1) {
            decode_original((uint8_t)ca0, (uint8_t)ca1, c, expected);
            decode_lut((uint8_t)ca0, (uint8_t)ca1, c, actual);
            for (int i = 0; i < 8; ++i) {
                // Expected == 0xFF means the original code skipped the
                // pixel (J bit clear). In real rendering, sprlinebuf
                // starts as 0x80, which we treat as "untouched". For
                // the LUT path, idx[i] = 0 also means "skipped". Both
                // code paths produce the same sentinel when comparing
                // their *delta* over a pre-initialised buffer.
                if (expected[i] != actual[i]) {
                    ++mismatches;
                    if (first_mismatch_input == -1) {
                        first_mismatch_input = (int)(ca0 | (ca1 << 8));
                        first_mismatch_pos = i;
                        first_expected = expected[i];
                        first_actual = actual[i];
                    }
                }
            }
        }
    }

    if (mismatches != 0) {
        std::printf(
            "FAIL: atr=0x%02X pal_mask=0x%02X — %d mismatches; first "
            "input=0x%04X pos=%d expected=0x%02X actual=0x%02X\n",
            c.atr, c.pal_mask, mismatches,
            first_mismatch_input, first_mismatch_pos,
            first_expected, first_actual);
    } else {
        std::printf("OK atr=0x%02X pal_mask=0x%02X — 65536 inputs matched\n",
                    c.atr, c.pal_mask);
    }
    return mismatches;
}

int main()
{
    makeppulut();

    // Cover all four (SP_BACK × H_FLIP) combinations × both
    // GRAYSCALE modes with a representative palette.
    const DecodeCase cases[] = {
        // pal[0..3]    atr           VB   mask
        { {0x00, 0x10, 0x20, 0x30}, 0x00, 0x10, 0xFF },
        { {0x00, 0x10, 0x20, 0x30}, H_FLIP, 0x10, 0xFF },
        { {0x00, 0x10, 0x20, 0x30}, SP_BACK, 0x10, 0xFF },
        { {0x00, 0x10, 0x20, 0x30}, uint8_t(SP_BACK | H_FLIP), 0x10, 0xFF },
        // GRAYSCALE = 1 (mask 0x30 collapses palettes)
        { {0x00, 0x10, 0x20, 0x30}, 0x00, 0x10, 0x30 },
        { {0x00, 0x10, 0x20, 0x30}, H_FLIP, 0x10, 0x30 },
        { {0x00, 0x10, 0x20, 0x30}, SP_BACK, 0x10, 0x30 },
        { {0x00, 0x10, 0x20, 0x30}, uint8_t(SP_BACK | H_FLIP), 0x10, 0x30 },
        // A pathological palette (alternating 0 and full) to maximise
        // value-distinguishing bits.
        { {0xFF, 0x00, 0xFF, 0x00}, 0x00, 0x10, 0xFF },
        { {0xFF, 0x00, 0xFF, 0x00}, uint8_t(SP_BACK | H_FLIP), 0x10, 0xFF },
    };

    int total_fail = 0;
    for (const auto& c : cases) total_fail += run_grid(c);

    if (total_fail != 0) {
        std::printf("\nFAIL: %d total mismatches across grid\n", total_fail);
        return 1;
    }
    std::printf("\nPASS: %u grids verified\n",
                static_cast<unsigned>(sizeof(cases) / sizeof(cases[0])));
    return 0;
}
