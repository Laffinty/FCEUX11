//! Background + sprite multiplexing, sprite 0 hit, sprite overflow.
//!
//! Reference: `src/ppu_rendering.cpp::CheckSpriteHit` +
//! `RefreshLine`/`RefreshSprites` + NESdev wiki "PPU sprite priority".

use crate::ppu::registers::{PPUSTATUS_SPRITE_OVERFLOW, PPUSTATUS_SPRITE_ZERO_HIT};

/// Compositor — owns the priority multiplexing logic + sprite 0 hit
/// tracking + sprite overflow tracking for the current scanline.
#[derive(Debug, Default, Clone)]
pub struct Compositor {
    /// Sprite 0 is loaded for the current scanline.
    sprite_zero_loaded: bool,
    /// Sprite 0 has been seen rendering (non-transparent pixel).
    sprite_zero_being_rendered: bool,
    /// Sprite 0 hit already triggered this frame (sticky until read).
    sprite_0hit_pending: bool,
    /// Sprite overflow already triggered this frame.
    sprite_overflow_pending: bool,
    /// Cached palette indices (16 BG palettes + 16 sprite palettes).
    palette_indices: [u8; 32],
}

impl Compositor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Per-scanline reset.
    pub fn reset_for_scanline(&mut self) {
        self.sprite_zero_loaded = false;
        self.sprite_zero_being_rendered = false;
    }

    /// Mark sprite 0 as loaded for the current scanline.
    #[inline]
    pub fn sprite_zero_loaded(&mut self) {
        self.sprite_zero_loaded = true;
    }

    /// Mark that sprite overflow was detected (9th sprite).
    #[inline]
    pub fn set_sprite_overflow(&mut self) {
        if !self.sprite_overflow_pending {
            self.sprite_overflow_pending = true;
        }
    }

    /// Notify the compositor of sprite 0 hit occurrence at pixel
    /// `(x, y)`.  Returns true if the hit should be latched (and the
    /// PPUSTATUS bit should be set).
    ///
    /// Conditions for sprite 0 hit:
    /// 1. Sprite 0 is loaded for this scanline.
    /// 2. Both BG and sprite 0 have non-transparent pixels at the same
    ///    dot.
    /// 3. The dot is at x >= 8 (left 8 pixels are clipped).
    pub fn check_sprite_zero_hit(
        &mut self,
        x: u8,
        y: u8,
        bg_opaque: bool,
        sprite_opaque: bool,
    ) -> bool {
        if self.sprite_zero_loaded
            && bg_opaque
            && sprite_opaque
            && x >= 8
            && !self.sprite_0hit_pending
        {
            self.sprite_0hit_pending = true;
            self.sprite_zero_being_rendered = true;
            let _ = y;
            true
        } else {
            false
        }
    }

    /// Compose a single pixel: given a BG color (0..=3) + BG palette
    /// (0..=3), and a sprite color (0..=3) + sprite palette (0..=3)
    /// + sprite priority-behind-bg flag, return the final color index
    /// and palette index.
    ///
    /// Returns `(palette_idx, color_idx)` where each is 0..=3.
    #[inline]
    pub fn compose(
        bg_color: u8,
        bg_palette: u8,
        sprite_color: u8,
        sprite_palette: u8,
        sprite_priority_behind_bg: bool,
    ) -> (u8, u8) {
        if sprite_color == 0 {
            // Sprite transparent: use BG.
            (bg_palette, bg_color)
        } else if bg_color == 0 {
            // BG transparent: use sprite.
            (sprite_palette, sprite_color)
        } else if sprite_priority_behind_bg {
            // Both opaque, sprite behind BG.
            (bg_palette, bg_color)
        } else {
            // Both opaque, sprite in front.
            (sprite_palette, sprite_color)
        }
    }

    /// Read-and-clear the sprite 0 hit pending flag.  Returns true
    /// if the flag was set.
    #[inline]
    pub fn consume_sprite_zero_hit(&mut self) -> bool {
        let v = self.sprite_0hit_pending;
        self.sprite_0hit_pending = false;
        v
    }

    /// Read-and-clear the sprite overflow pending flag.
    #[inline]
    pub fn consume_sprite_overflow(&mut self) -> bool {
        let v = self.sprite_overflow_pending;
        self.sprite_overflow_pending = false;
        v
    }

    /// Read the palette indices (16 BG + 16 sprite).
    pub fn palette_indices(&self) -> &[u8; 32] {
        &self.palette_indices
    }

    /// Set a palette entry.  Used during palette RAM writes.
    #[inline]
    pub fn set_palette_index(&mut self, idx: usize, val: u8) {
        if idx < 32 {
            self.palette_indices[idx] = val;
        }
    }

    /// Build the PPUSTATUS bits to OR into the status register.
    ///
    /// Called after each scanline to update the VBlank-visible bits.
    pub fn status_bits(&self) -> u8 {
        let mut bits = 0u8;
        if self.sprite_0hit_pending {
            bits |= PPUSTATUS_SPRITE_ZERO_HIT;
        }
        if self.sprite_overflow_pending {
            bits |= PPUSTATUS_SPRITE_OVERFLOW;
        }
        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_sprite_transparent_uses_bg() {
        let (pal, color) = Compositor::compose(2, 1, 0, 3, false);
        assert_eq!(pal, 1);
        assert_eq!(color, 2);
    }

    #[test]
    fn compose_bg_transparent_uses_sprite() {
        let (pal, color) = Compositor::compose(0, 0, 3, 2, false);
        assert_eq!(pal, 2);
        assert_eq!(color, 3);
    }

    #[test]
    fn compose_both_opaque_priority() {
        // Both opaque, sprite in front.
        let (pal, color) = Compositor::compose(2, 1, 3, 2, false);
        assert_eq!(pal, 2);
        assert_eq!(color, 3);
        // Both opaque, sprite behind BG.
        let (pal, color) = Compositor::compose(2, 1, 3, 2, true);
        assert_eq!(pal, 1);
        assert_eq!(color, 2);
    }

    #[test]
    fn sprite_zero_hit_requires_opaque_both() {
        let mut c = Compositor::new();
        c.sprite_zero_loaded();
        // BG transparent → no hit
        assert!(!c.check_sprite_zero_hit(10, 0, false, true));
        // Sprite transparent → no hit
        assert!(!c.check_sprite_zero_hit(10, 0, true, false));
        // Both opaque at x < 8 → no hit (clipped)
        assert!(!c.check_sprite_zero_hit(5, 0, true, true));
        // Both opaque at x >= 8 → hit
        assert!(c.check_sprite_zero_hit(10, 0, true, true));
    }

    #[test]
    fn sprite_zero_hit_is_sticky() {
        let mut c = Compositor::new();
        c.sprite_zero_loaded();
        c.check_sprite_zero_hit(10, 0, true, true);
        // Second call: hit already pending → returns false.
        assert!(!c.check_sprite_zero_hit(11, 0, true, true));
        // Read-and-clear: returns true, then false.
        assert!(c.consume_sprite_zero_hit());
        assert!(!c.consume_sprite_zero_hit());
    }

    #[test]
    fn sprite_overflow_is_sticky() {
        let mut c = Compositor::new();
        c.set_sprite_overflow();
        assert!(c.consume_sprite_overflow());
        assert!(!c.consume_sprite_overflow());
    }

    #[test]
    fn status_bits_reflects_state() {
        let mut c = Compositor::new();
        c.sprite_zero_loaded();
        c.set_sprite_overflow();
        c.check_sprite_zero_hit(10, 0, true, true);
        let bits = c.status_bits();
        assert_ne!(bits & PPUSTATUS_SPRITE_ZERO_HIT, 0);
        assert_ne!(bits & PPUSTATUS_SPRITE_OVERFLOW, 0);
    }
}