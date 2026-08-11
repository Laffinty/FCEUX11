//! Sprite LUT — 512 KiB precomputed lookup table for sprite pixel
//! decoding.
//!
//! In newppu, sprite pattern fetch can be replaced with a simple
//! 256×256×8 lookup: for every (Y offset, tile_id, row) combination,
//! precompute the 8 pixel values.  This avoids the runtime
//! computation per sprite pixel.
//!
//! # Memory layout
//!
//! Total: 256 Y values × 256 tile ids × 8 rows × 1 byte = **524,288
//! bytes** = 512 KiB.
//!
//! Index formula:
//!   `lut[y * 2048 + tile * 8 + row]` = the 8-bit pattern byte
//!   (low bit of color for each pixel in the row)
//!
//! For 8x16 sprites, the high bit selects the tile bank; Phase 3
//! computes the bank offset at fetch time.
//!
//! # LTCG safety
//!
//! We use `LazyLock` (Rust 2024 std lib) to defer the 512 KiB
//! allocation until first use.  This avoids:
//! 1. BSS segment bloat (linker reserves 512 KiB)
//! 2. Static init order / `c2.dll` materialization issues
//!    (audit / LTCG / hotfix precedent for splitting large tables)
//!
//! Reference: `src/ppu_sprite_lut.cpp` (FCEUX upstream).

use std::sync::LazyLock;

/// 512 KiB sprite LUT, lazily initialized.
pub static SPRITE_LUT: LazyLock<Box<SpriteLut>> = LazyLock::new(|| Box::new(SpriteLut::build()));

/// Sprite LUT type.  256 Y values × 256 tile ids × 8 rows = 524,288 bytes.
#[repr(C, align(64))]
pub struct SpriteLut {
    /// Indexed `[y * 2048 + tile * 8 + row]`.
    data: [u8; SPRITE_LUT_SIZE],
}

/// Size of the sprite LUT in bytes.
pub const SPRITE_LUT_SIZE: usize = 256 * 256 * 8;

impl SpriteLut {
    /// Build the LUT.  For Phase 3, we use a **placeholder** fill: each
    /// byte is computed from `(y, tile, row)` deterministically but
    /// does NOT match real CHR ROM contents (we don't have those at
    /// init time).  The Phase 4+ SoC layer feeds real CHR data via
    /// `fill_from_chr()`.
    ///
    /// This placeholder matches the size + shape expected by the
    /// sprite rendering pipeline so callers can use the API
    /// correctly.  Phase 6 shadow-run tests pin the byte values
    /// against the upstream `ppu_sprite_lut.cpp` table.
    fn build() -> Self {
        let mut lut = Self { data: [0; SPRITE_LUT_SIZE] };
        // Placeholder: a deterministic pattern that exercises every
        // combination.  Real CHR-fed fill happens at SoC load time.
        for y in 0..256usize {
            for tile in 0..256usize {
                for row in 0..8usize {
                    let idx = y * 2048 + tile * 8 + row;
                    // Reproducible placeholder: bit 0 = (y XOR tile XOR row) bit 0.
                    lut.data[idx] = ((y ^ tile ^ row) & 0x01) as u8;
                }
            }
        }
        lut
    }

    /// Get the placeholder pattern byte for a (y, tile, row) tuple.
    #[inline(always)]
    pub fn get(&self, y: u8, tile: u8, row: u8) -> u8 {
        self.data[y as usize * 2048 + tile as usize * 8 + row as usize]
    }

    /// Mutable accessor for filling the LUT with real CHR data
    /// (Phase 4 SoC layer).
    #[inline]
    pub fn fill_from_chr(&mut self, chr: &[u8]) {
        // Phase 4+ real implementation.  For Phase 3, copy CHR
        // pattern bytes into the LUT.
        let copy_len = chr.len().min(SPRITE_LUT_SIZE);
        self.data[..copy_len].copy_from_slice(&chr[..copy_len]);
    }

    /// Total size in bytes (for sanity checks + snapshot).
    pub fn size_bytes() -> usize {
        SPRITE_LUT_SIZE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lut_size_matches_expectation() {
        assert_eq!(SpriteLut::size_bytes(), 256 * 256 * 8);
        assert_eq!(SpriteLut::size_bytes(), 524_288);
    }

    #[test]
    fn lut_is_64_byte_aligned() {
        // align(64) on the struct + u8 array means the data array
        // starts on a 64-byte boundary.  Important for SIMD / cache.
        assert_eq!(std::mem::align_of::<SpriteLut>(), 64);
    }

    #[test]
    fn lut_lazy_init_via_static() {
        // First access triggers init.
        let _ = SPRITE_LUT.get(0, 0, 0);
        let _ = SPRITE_LUT.get(255, 255, 7);
    }

    #[test]
    fn lut_placeholder_is_deterministic() {
        // Same input → same output, regardless of access order.
        let v1 = SPRITE_LUT.get(0x42, 0x37, 0x05);
        let v2 = SPRITE_LUT.get(0x42, 0x37, 0x05);
        assert_eq!(v1, v2);
        // Placeholder formula: (y XOR tile XOR row) bit 0.
        let expected = (0x42u8 ^ 0x37u8 ^ 0x05u8) & 0x01;
        assert_eq!(v1, expected);
    }

    #[test]
    fn lut_indexing_covers_full_table() {
        // Sanity: probe the four corners of the 3D LUT.
        let corners = [(0u8, 0u8, 0u8), (255, 0, 0), (0, 255, 0), (0, 0, 7), (255, 255, 7)];
        for &(y, tile, row) in &corners {
            let _ = SPRITE_LUT.get(y, tile, row);
        }
    }
}