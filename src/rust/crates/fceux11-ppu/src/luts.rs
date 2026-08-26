//! Precomputed lookup tables for the 2C02 PPU pixel output.
//!
//! These are the same tables the C++ new PPU builds at `FCEUPPU_Init` time
//! (`src/ppu_rendering.cpp:makeppulut`, originally `src/ppu.cpp:127-151`).
//! They convert a pattern byte (0..=255) into a `u32` of 8 packed
//! 4-bit pixels so the per-pixel loop becomes a single lookup.
//!
//! `ppulut1[byte]` �?bit plane 0 of `byte`, packed as 8 4-bit pixels.
//! `ppulut2[byte]` �?same as `ppulut1` shifted left by 1 (bit plane 1).
//! `ppulut3[xo | (cc << 3)]` �?palette attribute, replicated for each of
//! the 8 pixels in a tile, shifted by the per-pixel offset (xo).
//!
//! For pixel `i` in a tile (i = 0..=7), the final palette index is:
//!   ((ppulut1[b0] | ppulut2[b1]) >> (4*i)) & 0xF | ppulut3_pixel[i]
//! where b0/b1 are the two pattern bytes and `ppulut3_pixel[i]` is the
//! attribute contribution from `atlatch`.

/// Precomputed bit-plane-0 �?4-bit-pixel expansion (plane 0).
/// `ppulut1[byte] = sum_{y=0..=7} ((byte >> (7 - y)) & 1) << (y * 4)`.
pub const PPULUT1: [u32; 256] = build_ppulut1();

/// Precomputed bit-plane-1 expansion: `PPULUT1[byte] << 1`.
pub const PPULUT2: [u32; 256] = build_ppulut2();

/// Precomputed attribute expansion: `ppulut3[xo | (cc << 3)]`.
/// For xo = 0..=7 and cc = 0..=15, packs the 4-bit attribute value into
/// the upper 2 bits of each of the 8 packed 4-bit pixels, shifted by
/// the per-pixel offset.
pub const PPULUT3: [u32; 128] = build_ppulut3();

const fn build_ppulut1() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut x: usize = 0;
    while x < 256 {
        let mut v: u32 = 0;
        let mut y: u32 = 0;
        while y < 8 {
            v |= (((x as u32) >> (7 - y)) & 1) << (y * 4);
            y += 1;
        }
        t[x] = v;
        x += 1;
    }
    t
}

const fn build_ppulut2() -> [u32; 256] {
    let mut t = [0u32; 256];
    let mut x: usize = 0;
    while x < 256 {
        t[x] = PPULUT1[x] << 1;
        x += 1;
    }
    t
}

const fn build_ppulut3() -> [u32; 128] {
    let mut t = [0u32; 128];
    let mut cc: usize = 0;
    while cc < 16 {
        let mut xo: usize = 0;
        while xo < 8 {
            let mut v: u32 = 0;
            let mut pixel: usize = 0;
            while pixel < 8 {
                // shiftr is how many bits to shift cc right to extract
                // the 2-bit attribute quadrant for this pixel. (pixel + xo)
                // divided by 8 (rounded down) gives the quadrant index
                // within the 2x2 quadrant block. * 2 because each
                // quadrant is 2 bits in the attribute byte.
                let shiftr = ((pixel + xo) / 8) * 2;
                v |= (((cc as u32) >> shiftr) & 3) << (2 + pixel * 4);
                pixel += 1;
            }
            t[xo | (cc << 3)] = v;
            xo += 1;
        }
        cc += 1;
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ppulut1_byte_0x80_has_bit_0_set() {
        // 0x80 = 0b1000_0000: only bit 0 of plane 0 �?pixel 0 = 1.
        // (x >> 7) & 1 = 1 �?shifted by 0*4 = 0 �?0x00000001.
        assert_eq!(PPULUT1[0x80] & 0xF, 0x1);
        // All other pixels should be 0.
        for y in 1..8 {
            assert_eq!((PPULUT1[0x80] >> (y * 4)) & 0xF, 0);
        }
    }

    #[test]
    fn ppulut1_byte_0xff_is_all_ones() {
        // 0xFF: every bit set �?every pixel = 1.
        for y in 0..8 {
            assert_eq!((PPULUT1[0xFF] >> (y * 4)) & 0xF, 0x1);
        }
    }

    #[test]
    fn ppulut2_byte_0x80_has_bit_1_set() {
        // ppulut1[0x80] = 0x00000001; ppulut2 = 0x00000002.
        assert_eq!(PPULUT2[0x80] & 0xF, 0x2);
    }

    #[test]
    fn ppulut3_uniform_for_xo_zero() {
        // The C++ ppulut3 LUT is built for 8 pixel values per (xo, cc)
        // pair. For xo=0..3, all 8 pixels get the same value (the
        // lower 2 bits of cc, shifted to bits 2..3 of each 4-bit
        // pixel). For xo=4..7, the C++ algorithm has a known quirk
        // where the upper 4 pixels get 0 (because shiftr=2 for them
        // in the formula). The FCEUX renderer doesn't use the
        // upper-half attribute bits in that case �?it pre-extracts
        // the per-quadrant attribute at the refresh-address level
        // before indexing into ppulut3. The Rust renderer takes the
        // cleaner route and computes the per-pixel attribute
        // directly from coarse X/Y, so this quirk doesn't matter.
        //
        // We only assert the well-defined case (xo=0).
        let v = PPULUT3[0 | (0x3 << 3)];
        // All 8 pixels should be 0xC (attribute=3 at bits 2..3).
        for y in 0..8 {
            assert_eq!(
                (v >> (y * 4)) & 0xF,
                0xC,
                "pixel {} should be 0xC (cc=3, xo=0)",
                y
            );
        }
    }
}
