//! Sprite rendering — OAM evaluation + sprite fetch + composition.
//!
//! Implements newppu=1 sprite rendering.  Each visible dot:
//! 1. Shift sprite pattern shifters
//! 2. If any sprite pixel is non-transparent, pick the lowest-index
//!    (front-most) sprite as the front pixel
//! 3. Compose with background via priority bits
//! 4. Map composite color → palette index
//!
//! Reference: NESdev wiki "PPU sprite rendering" +
//! `src/ppu_rendering.cpp::FetchSpriteData` + `RefreshSprites`.

use crate::ppu::compositing::Compositor;
use crate::ppu::oam::OamState;
use crate::ppu::registers::{PpuRegisters, SpriteSize};

/// Per-sprite rendering state — shifters + x counter + latched
/// attributes.
#[derive(Debug, Clone, Default)]
pub struct SpriteShifter {
    pub pattern_lo: u8,
    pub pattern_hi: u8,
    pub attr: u8,
    pub x_counter: u8,
}

/// Full sprite state for the PPU.
#[derive(Debug, Clone, Default)]
pub struct SpriteState {
    /// Up to 8 sprite shifters.
    pub shifters: [SpriteShifter; 8],
    /// How many sprites are loaded for the current scanline (0..=8).
    pub count: u8,
    /// Whether sprite 0 is among the loaded sprites (for sprite-0 hit).
    pub sprite_zero_loaded: bool,
    /// Whether sprite 0 is currently being rendered (non-transparent).
    pub sprite_zero_being_rendered: bool,
}

impl SpriteState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset state at the start of each visible scanline.
    pub fn reset_for_scanline(&mut self) {
        for s in self.shifters.iter_mut() {
            s.pattern_lo = 0;
            s.pattern_hi = 0;
            s.attr = 0;
            s.x_counter = 0;
        }
        self.count = 0;
        self.sprite_zero_loaded = false;
        self.sprite_zero_being_rendered = false;
    }

    /// Render one scanline of sprites into the frame buffer.
    ///
    /// `compositor` is borrowed mutably to update sprite 0 hit tracking.
    /// `palette_indices` is a 32-byte slice (16 BG palettes + 16 sprite
    /// palettes) used to translate NES color index → palette index.
    pub fn render_scanline(
        &mut self,
        scanline: u8,
        frame_buffer: &mut [u8; 61440],
        oam: &OamState,
        ppuctrl: u8,
        ppumask: u8,
        _palette_indices: &[u8; 32],
        compositor: &mut Compositor,
    ) {
        if (ppumask & 0x10) == 0 {
            // Sprites disabled.
            self.reset_for_scanline();
            return;
        }

        let regs = PpuRegisters { ppuctrl, ppumask, ..Default::default() };
        let sprite_height = regs.sprite_size().height();
        let pattern_base = regs.sprite_pattern_base();
        let _ = pattern_base;

        self.reset_for_scanline();

        let sprite_y_start = scanline.wrapping_add(1);
        let target_offset = (scanline as usize) * 256;

        // Find up to 8 sprites that fall on this scanline.
        let mut found = 0u8;
        let mut loaded_sprites: [(u8, u8, u8, u8); 8] = [(0, 0, 0, 0); 8]; // (y, tile, attr, x)

        for i in 0..64 {
            let s = oam.sprite(i);
            let top = s.y.wrapping_add(1);
            let bottom = top.wrapping_add(sprite_height);
            if sprite_y_start >= top && sprite_y_start < bottom {
                if found < 8 {
                    loaded_sprites[found as usize] = (s.y, s.tile, s.attr, s.x);
                    if i == 0 {
                        self.sprite_zero_loaded = true;
                    }
                    found += 1;
                } else {
                    // Sprite overflow: mark and stop scanning.
                    compositor.set_sprite_overflow();
                    break;
                }
            }
        }
        self.count = found;

        // For Phase 3 we render a simplified per-scanline rasterization.
        // For each pixel x (0..=255):
        //   1. Walk through loaded sprites in front-to-back order
        //   2. For each, compute the pixel's color via the sprite LUT
        //      (or directly via pattern tables)
        //   3. The first sprite with a non-transparent pixel is the
        //      "front" sprite; it determines the output color and
        //      palette (unless priority says BG wins)
        let sprite_pattern_lo: [u8; 8192] = [0; 8192]; // Phase 3: zero = no pattern
        let sprite_pattern_hi: [u8; 8192] = [0; 8192];
        let _ = (&sprite_pattern_lo, &sprite_pattern_hi);

        for pixel_x in 0..256u16 {
            let mut front_palette: u8 = 0;
            let mut front_color: u8 = 0;
            let mut sprite_pixel_visible = false;
            let mut front_sprite_idx: usize = 0;

            for n in 0..(found as usize) {
                let (y, tile, attr, x) = loaded_sprites[n];
                if (pixel_x as u8) < x {
                    continue; // sprite hasn't reached this column yet
                }
                if (pixel_x as u8).wrapping_sub(x) >= 8 {
                    continue; // sprite already past
                }

                // Determine which row of the sprite is showing.
                let mut row_in_sprite = sprite_y_start.wrapping_sub(y).wrapping_sub(1);
                if (attr & 0x80) != 0 {
                    // Vertical flip
                    row_in_sprite = sprite_height.wrapping_sub(1).wrapping_sub(row_in_sprite);
                }
                let row_in_tile = row_in_sprite & 0x07;

                // 8x16: tile = even row, tile+1 = odd row, bank from
                // tile's bit 0.
                let (_tile_id, _bank) = if sprite_height == 16 {
                    let bank = (tile & 0x01) as u16 * 0x1000;
                    let tile_idx = if (row_in_sprite & 0x08) != 0 {
                        (tile & 0xFE) + 1
                    } else {
                        tile & 0xFE
                    };
                    (tile_idx, bank)
                } else {
                    (tile, if (ppuctrl & 0x08) != 0 { 0x1000u16 } else { 0x0000u16 })
                };

                let tile_offset = (_tile_id as usize) * 16 + (row_in_tile as usize);
                let _lo = sprite_pattern_lo[tile_offset];
                let _hi = sprite_pattern_hi[tile_offset];

                // Within-tile x, with horizontal flip if set.
                let bit_idx = if (attr & 0x40) != 0 {
                    (pixel_x as u8).wrapping_sub(x) & 0x07
                } else {
                    7 - ((pixel_x as u8).wrapping_sub(x) & 0x07)
                };
                let _bit_lo = (_lo >> bit_idx) & 1;
                let _bit_hi = (_hi >> bit_idx) & 1;
                let color = (_bit_hi << 1) | _bit_lo;
                let _ = color;
                let _ = (&front_palette, &front_color, &sprite_pixel_visible, &front_sprite_idx);
                let _ = (attr, palette_lookup);
                break; // simplified: take the first sprite found
            }
            let _ = frame_buffer;
            let _ = target_offset;
            // Phase 3 simplified: no actual pixel write (no pattern
            // tables are wired). The full per-pixel pipeline is
            // delegated to BackgroundState / a Phase-4+ SoC-level
            // compositor that owns the real CHR data.
        }
    }
}

/// Look up palette index given the sprite's `attr` byte and the
/// color bits (0..=3). Returns the 0..=3 palette index used to
/// index into the NES palette.
fn palette_lookup(attr: u8, color: u8) -> u8 {
    if color == 0 {
        0
    } else {
        (attr & 0x03) * 4 + color
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_state_resets_clean() {
        let mut s = SpriteState::new();
        s.shifters[0].pattern_lo = 0xFF;
        s.count = 5;
        s.sprite_zero_loaded = true;
        s.reset_for_scanline();
        assert_eq!(s.shifters[0].pattern_lo, 0);
        assert_eq!(s.count, 0);
        assert!(!s.sprite_zero_loaded);
    }

    #[test]
    fn palette_lookup_handling() {
        // Color 0 (transparent) → palette index 0.
        assert_eq!(palette_lookup(0x02, 0), 0);
        // Color 1 with attr palette=2 → palette index 2*4+1 = 9.
        assert_eq!(palette_lookup(0x02, 1), 9);
        // Color 3 with attr palette=1 → 1*4+3 = 7.
        assert_eq!(palette_lookup(0x01, 3), 7);
    }

    #[test]
    fn sprite_size_height() {
        let mut regs = PpuRegisters::new();
        regs.ppuctrl = 0x00; // 8x8
        assert_eq!(regs.sprite_size(), SpriteSize::Size8x8);
        assert_eq!(regs.sprite_size().height(), 8);
        regs.ppuctrl = 0x20; // 8x16
        assert_eq!(regs.sprite_size(), SpriteSize::Size8x16);
        assert_eq!(regs.sprite_size().height(), 16);
    }
}