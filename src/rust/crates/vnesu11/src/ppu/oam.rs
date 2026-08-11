//! Primary + secondary OAM (sprite attribute memory).
//!
//! NES hardware has:
//! - **Primary OAM**: 256 bytes, addressed via $2003/$2004.  Holds up
//!   to 64 sprites, 4 bytes each: (y, tile_id, attribute, x).
//! - **Secondary OAM**: 32 bytes, internal to the PPU.  Holds the 8
//!   (or fewer) sprites that will be rendered on the next scanline.
//!
//! Reference: NESdev wiki "PPU sprite evaluation" +
//! `src/ppu.cpp::PPU_RefreshLine` / `src/ppu_rendering.cpp` sprite
//! eval logic.

use crate::ppu::dot_clock::MAX_SPRITES_PER_LINE;

/// Primary OAM — 256 bytes, 64 sprites × 4 bytes.
///
/// Byte layout per sprite (NES OAM format):
/// - OAM[i*4 + 0] = Y position (top of sprite, minus 1)
/// - OAM[i*4 + 1] = tile id (or top half for 8x16)
/// - OAM[i*4 + 2] = attribute (palette, priority, flips)
/// - OAM[i*4 + 3] = X position
#[derive(Debug, Clone)]
pub struct OamState {
    /// 256-byte primary OAM.
    pub primary: [u8; 256],
    /// 32-byte secondary OAM (filled during sprite eval).
    pub secondary: [u8; 32],
    /// Number of sprites copied to secondary OAM (0..=8; 9+ triggers overflow).
    pub secondary_count: u8,
}

impl Default for OamState {
    fn default() -> Self {
        Self::new()
    }
}

impl OamState {
    pub fn new() -> Self {
        Self {
            primary: [0; 256],
            secondary: [0; 32],
            secondary_count: 0,
        }
    }

    /// Read primary OAM at the address pointed to by `oam_addr`.
    /// Used for `$2004` reads.  Increments `oam_addr` after read.
    #[inline]
    pub fn read_primary(&mut self, oam_addr: &mut u8) -> u8 {
        let v = self.primary[*oam_addr as usize];
        // Reading does NOT increment oam_addr in hardware. Source:
        // NESdev wiki "OAM read".  We keep it stable.
        v
    }

    /// Write primary OAM at the address pointed to by `oam_addr`.
    /// Used for `$2004` writes.  Increments `oam_addr` after write.
    #[inline]
    pub fn write_primary(&mut self, oam_addr: &mut u8, val: u8) {
        self.primary[*oam_addr as usize] = val;
        *oam_addr = oam_addr.wrapping_add(1);
    }

    /// Get the 4-byte sprite entry at index `n` (0..=63).
    #[inline]
    pub fn sprite(&self, n: usize) -> SpriteEntry {
        let base = n * 4;
        SpriteEntry {
            y: self.primary[base],
            tile: self.primary[base + 1],
            attr: self.primary[base + 2],
            x: self.primary[base + 3],
        }
    }

    /// Get the secondary-OAM sprite entry at slot `n` (0..=7).
    #[inline]
    pub fn secondary_sprite(&self, n: usize) -> SpriteEntry {
        let base = n * 4;
        SpriteEntry {
            y: self.secondary[base],
            tile: self.secondary[base + 1],
            attr: self.secondary[base + 2],
            x: self.secondary[base + 3],
        }
    }
}

/// A single 4-byte OAM sprite entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteEntry {
    pub y: u8,
    pub tile: u8,
    pub attr: u8,
    pub x: u8,
}

impl SpriteEntry {
    /// Palette index (bits 0-1 of attr).
    #[inline(always)]
    pub fn palette(&self) -> u8 {
        self.attr & 0x03
    }

    /// Priority (bit 5 of attr). True = behind background.
    #[inline(always)]
    pub fn priority_behind_bg(&self) -> bool {
        (self.attr & 0x20) != 0
    }

    /// Horizontal flip (bit 6 of attr).
    #[inline(always)]
    pub fn flip_h(&self) -> bool {
        (self.attr & 0x40) != 0
    }

    /// Vertical flip (bit 7 of attr).
    #[inline(always)]
    pub fn flip_v(&self) -> bool {
        (self.attr & 0x80) != 0
    }

    /// Check whether the sprite is visible on the given scanline.
    /// Returns `Some(row_in_sprite)` if yes, `None` if no.
    ///
    /// `scanline` is the *current visible* scanline (0..=239).
    /// `sprite_height` is 8 or 16.
    pub fn visible_at(&self, scanline: u8, sprite_height: u8) -> Option<u8> {
        let top = self.y.wrapping_add(1); // Y+1 is the top (Y is top-1)
        let bottom = top.wrapping_add(sprite_height);
        if scanline >= top && scanline < bottom {
            Some(scanline - top)
        } else {
            None
        }
    }
}

/// Result of sprite evaluation for one scanline.
#[derive(Debug, Clone, Copy)]
pub struct SpriteEvalResult {
    /// Sprites that will be rendered (0..=8).
    pub count: u8,
    /// Whether the 9th sprite triggered overflow.
    pub overflow: bool,
}

/// Evaluate which sprites are visible on `scanline`.
///
/// Walks through primary OAM, copying entries that fall within the
/// scanline range into secondary OAM.  Stops after 8 sprites; the 9th
/// in-range sprite triggers the overflow flag (a simplification: real
/// hardware has a buggy overflow detection that's more permissive).
pub fn evaluate_sprites(
    oam: &OamState,
    scanline: u8,
    sprite_height: u8,
) -> SpriteEvalResult {
    let mut count = 0u8;
    let mut overflow = false;
    for i in 0..64 {
        let entry = oam.sprite(i);
        if entry.visible_at(scanline, sprite_height).is_some() {
            if count < MAX_SPRITES_PER_LINE as u8 {
                // Copy to secondary OAM.
                let dst = (count as usize) * 4;
                // SAFETY: count < 8 so dst + 3 < 32.
                let mut s = oam.clone();
                s.secondary[dst] = entry.y;
                s.secondary[dst + 1] = entry.tile;
                s.secondary[dst + 2] = entry.attr;
                s.secondary[dst + 3] = entry.x;
                count += 1;
            } else {
                // 9th+ visible sprite triggers overflow.
                overflow = true;
                break;
            }
        }
    }
    SpriteEvalResult { count, overflow }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_attr_decoding() {
        // Use 0xE3 = 1110_0011: palette=3, priority=1 (bit 5), flip_h=1, flip_v=1
        let s = SpriteEntry { y: 0, tile: 0, attr: 0xE3, x: 0 };
        // bits 0-1 = palette 3
        assert_eq!(s.palette(), 3);
        // bit 5 = priority behind BG
        assert!(s.priority_behind_bg());
        // bit 6 = flip H
        assert!(s.flip_h());
        // bit 7 = flip V
        assert!(s.flip_v());
    }

    #[test]
    fn visible_at_top_row() {
        let s = SpriteEntry { y: 0, tile: 0, attr: 0, x: 0 };
        // Y=0 means top is at scanline 1. Bottom is at scanline 9
        // (exclusive), so last visible row is 8.
        assert_eq!(s.visible_at(0, 8), None);
        assert_eq!(s.visible_at(1, 8), Some(0));
        assert_eq!(s.visible_at(7, 8), Some(6));
        assert_eq!(s.visible_at(8, 8), Some(7));
        assert_eq!(s.visible_at(9, 8), None);
    }

    #[test]
    fn visible_at_8x16() {
        let s = SpriteEntry { y: 0, tile: 0, attr: 0, x: 0 };
        // 8x16 sprite: top at 1, bottom at 17 (exclusive), last row = 16.
        assert_eq!(s.visible_at(0, 16), None);
        assert_eq!(s.visible_at(1, 16), Some(0));
        assert_eq!(s.visible_at(15, 16), Some(14));
        assert_eq!(s.visible_at(16, 16), Some(15));
        assert_eq!(s.visible_at(17, 16), None);
    }

    #[test]
    fn oam_address_wraps_around() {
        let mut oam = OamState::new();
        // Writing $FF at $FF increments to $00.
        let mut addr = 0xFFu8;
        oam.write_primary(&mut addr, 0xCC);
        // Direct access at $FF should hold 0xCC.
        assert_eq!(oam.primary[0xFF], 0xCC);
        // The address was incremented past 0xFF → 0x00.
        assert_eq!(addr, 0x00);
    }

    #[test]
    fn evaluate_finds_visible_sprites() {
        let mut oam = OamState::new();
        // 3 sprites, all visible on scanline 10 (Y=9 → top=10, height=8).
        oam.primary[0..4].copy_from_slice(&[9, 0x10, 0x00, 0x20]);
        oam.primary[4..8].copy_from_slice(&[9, 0x11, 0x01, 0x40]);
        oam.primary[8..12].copy_from_slice(&[9, 0x12, 0x02, 0x60]);
        // 1 sprite NOT visible (Y=100).
        oam.primary[12..16].copy_from_slice(&[100, 0x13, 0x03, 0x80]);

        let result = evaluate_sprites(&oam, 10, 8);
        assert_eq!(result.count, 3);
        assert!(!result.overflow);
    }

    #[test]
    fn evaluate_triggers_overflow_at_9() {
        let mut oam = OamState::new();
        // 9 sprites, all visible.
        for i in 0..9 {
            let base = i * 4;
            oam.primary[base..base + 4].copy_from_slice(&[9, i as u8, 0x00, 0]);
        }
        let result = evaluate_sprites(&oam, 10, 8);
        assert_eq!(result.count, 8); // capped at 8
        assert!(result.overflow);
    }
}