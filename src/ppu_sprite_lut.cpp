// ppu_sprite_lut.cpp
//
// hotfix2 P0-1 (ARCH-2): generation of the 65536-entry sprite LUT.
//
// Mathematical derivation (matches ppu_rendering.cpp:866 logic):
//
//   ppulut1[ca0]   = sum_y ((ca0 >> (7-y)) & 1) << (y * 4)
//   ppulut2[ca1]   = ppulut1[ca1] << 1
//   pixdata        = ppulut1[ca0] | ppulut2[ca1]
//   J              = ca0 | ca1
//
// For sprite pixel position k (left-to-right, 0..7):
//   The NES pattern byte stores bit 7 as the leftmost pixel and
//   bit 0 as the rightmost — hence when ppulut1[ca] is added to
//   pixdata, chunk k of pixdata reads ca's bit (7-k). The original
//   code's first read (no shifts) writes C[0]; the seventh read
//   (after six `>>= 4`) writes C[7]. Therefore:
//     colour_k = (ca0 >> (7-k)) & 1
//              | ((ca1 >> (7-k)) & 1) << 1
//              = ((input >> (7-k)) & 1)
//              | (((input >> (15-k)) & 1) << 1)
//     where input = (ca0 | (ca1 << 8)).
//
//   visibility mask bit (7-k) of J:
//     J_bit(7-k) = (ca0 >> (7-k)) | (ca1 >> (7-k))
//                = ((input >> (7-k))  & 1)
//                | ((input >> (15-k)) & 1)
//     The original code's `if (J & (0x80 >> k))` lets pixel k through
//     iff this bit is set.
//
// Note: "colour_k == 0" does NOT always imply transparent.
// e.g. ca0 = 0x80 (only MSB) and ca1 = 0 ⇒ colour at k=0 is 1
// (chunk 0 reads ca's bit 7 = 1) yet J's bit 7 (which the original
// checks via `J & 0x80`) is 0. To preserve the original's
// "skip-on-J-bit-cleared" semantics exactly, the LUT stores
// visibility as a separate flag (bit 7 of each byte).
//
// Per-byte layout of each LUT entry:
//   bit 0..1  = colour index for the pixel at this position (0..3)
//   bit 7     = visibility flag (1 = write this pixel)
//   bits 2..6 = unused (always 0)
//
// Iteration order in RefreshSprites is left-to-right; H_FLIP applies
// a single 64-bit byte-reverse (`FCEU_BSWAP64`) which simultaneously
// flips visibility AND colour ordering, so no extra branching is
// needed for the horizontal mirror.

#include "ppu_sprite_lut.h"

namespace fceu11::ppu {

alignas(64) const std::array<uint64_t, 65536> kSpriteIdxLUT = []{
    std::array<uint64_t, 65536> t{};
    for (uint32_t input = 0; input < 65536; ++input) {
        uint64_t packed = 0;
        for (int k = 0; k < 8; ++k) {
            // Pixel k (left-to-right, 0..7). NES pattern byte stores
            // bit 7 = leftmost pixel, bit 0 = rightmost — so the
            // colour index for pixel k in chunk k of pixdata reads
            // ca0's bit (7-k) and ca1's bit (7-k):
            //   colour_k = (ca0 >> (7-k)) & 1
            //            | ((ca1 >> (7-k)) & 1) << 1
            // `input` packs these as ca0 = input[0..7] and
            // ca1 = input[8..15], so the ca1 contribution lives at
            // bit (15-k) of input.
            const uint8_t colour = static_cast<uint8_t>(
                ((input >> (7 - k)) & 1u)
                | (((input >> (15 - k)) & 1u) << 1));
            // visibility flag (bit 7) = J's bit (7-k), where
            // J = ca0 | ca1. Equals "colour != 0"; we keep the
            // explicit OR to avoid relying on the equivalence (the
            // MSVC /O2 path doesn't need the dedup either way).
            const uint8_t visible = static_cast<uint8_t>(
                ((input >> (7 - k)) | (input >> (15 - k))) & 1u);
            const uint8_t byte =
                static_cast<uint8_t>(colour | (visible << 7));
            packed |= static_cast<uint64_t>(byte) << (k * 8);
        }
        t[input] = packed;
    }
    return t;
}();

}  // namespace fceu11::ppu
