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
use crate::ppu::registers::PpuRegisters;

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
    /// `pattern_lo` / `pattern_hi` are the SoC CHR caches (both pattern
    /// tables, 8192 bytes each — index `(tile_id * 16) + row`). The
    /// background pass runs first and leaves the encoded BG pixel
    /// `(bg_palette << 2) | bg_color` in `frame_buffer`; this pass
    /// composes sprite pixels on top and writes the result back.
    ///
    /// `compositor` is borrowed mutably to update sprite 0 hit tracking.
    pub fn render_scanline(
        &mut self,
        scanline: u8,
        frame_buffer: &mut [u8; 61440],
        oam: &OamState,
        ppuctrl: u8,
        ppumask: u8,
        pattern_lo: &[u8; 8192],
        pattern_hi: &[u8; 8192],
        compositor: &mut Compositor,
    ) {
        if (ppumask & 0x10) == 0 {
            // Sprites disabled.
            self.reset_for_scanline();
            return;
        }

        let regs = PpuRegisters { ppuctrl, ppumask, ..Default::default() };
        let sprite_height = regs.sprite_size().height();

        self.reset_for_scanline();

        let target_offset = (scanline as usize) * 256;

        // Find up to 8 sprites that fall on this scanline (OAM order =
        // front-to-back; sprite 0 is front-most).
        let mut found = 0u8;
        let mut loaded_sprites: [(u8, u8, u8, u8); 8] = [(0, 0, 0, 0); 8]; // (y, tile, attr, x)

        for i in 0..64 {
            let s = oam.sprite(i);
            if let Some(_row) = s.visible_at(scanline, sprite_height) {
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
        if self.sprite_zero_loaded {
            compositor.sprite_zero_loaded();
        }

        for pixel_x in 0..256u16 {
            // BG pixel already rendered by the background pass.
            let bg_encoded = frame_buffer[target_offset + pixel_x as usize];
            let bg_color = bg_encoded & 0x03;
            let bg_palette = (bg_encoded >> 2) & 0x03;

            // Walk loaded sprites front-to-back; the first one with a
            // non-transparent pixel at this column wins the pixel.
            let mut sprite_color: u8 = 0;
            let mut sprite_palette: u8 = 0;
            let mut sprite_priority: bool = false;
            let mut front_found = false;
            let mut front_is_sprite_zero = false;

            for n in 0..(found as usize) {
                let (y, tile, attr, x) = loaded_sprites[n];
                let dx = (pixel_x as u8).wrapping_sub(x);
                if (pixel_x as u8) < x || dx >= 8 {
                    continue; // sprite hasn't reached this column yet / already past
                }

                // Row within the sprite (NES: top row is at scanline Y+1).
                let mut row_in_sprite = scanline.wrapping_sub(y).wrapping_sub(1);
                if (attr & 0x80) != 0 {
                    // Vertical flip.
                    row_in_sprite = sprite_height.wrapping_sub(1).wrapping_sub(row_in_sprite);
                }
                let row_in_tile = row_in_sprite & 0x07;

                // Tile id + pattern bank.
                // 8x16: tile = even row, tile+1 = odd row, bank from
                // tile's bit 0.
                let (tile_idx, bank) = if sprite_height == 16 {
                    let table = (tile & 0x01) as u16; // bank from tile bit 0
                    let t = if (row_in_sprite & 0x08) != 0 {
                        (tile & 0xFE) + 1
                    } else {
                        tile & 0xFE
                    };
                    (t as u16, table)
                } else {
                    // 8x8: bank from PPUCTRL bit 3 (sprite pattern table).
                    let table = if (ppuctrl & 0x08) != 0 { 1 } else { 0 };
                    (tile as u16, table)
                };

                // pattern_lo/hi cover both pattern tables (512 tiles),
                // so table 1 tiles live at (tile + 256) * 16 + row.
                let tile_offset = ((tile_idx + bank * 256) as usize) * 16 + (row_in_tile as usize);
                let lo = pattern_lo[tile_offset];
                let hi = pattern_hi[tile_offset];

                // Within-tile x, with horizontal flip if set.
                let bit_idx = if (attr & 0x40) != 0 {
                    dx & 0x07
                } else {
                    7 - (dx & 0x07)
                };
                let bit_lo = (lo >> bit_idx) & 1;
                let bit_hi = (hi >> bit_idx) & 1;
                let color = (bit_hi << 1) | bit_lo;

                if color != 0 {
                    sprite_color = color;
                    sprite_palette = attr & 0x03;
                    sprite_priority = (attr & 0x20) != 0;
                    front_found = true;
                    front_is_sprite_zero = n == 0;
                    break;
                }
            }

            if front_found {
                // Compose sprite over BG (priority rules in Compositor).
                let (out_pal, out_color) = Compositor::compose(
                    bg_color,
                    bg_palette,
                    sprite_color,
                    sprite_palette,
                    sprite_priority,
                );
                frame_buffer[target_offset + pixel_x as usize] = (out_pal << 2) | out_color;
                // Sprite 0 hit: opaque sprite 0 pixel + opaque BG pixel
                // at x >= 8 (compositor enforces the clip + stickiness).
                if front_is_sprite_zero {
                    compositor.check_sprite_zero_hit(pixel_x as u8, scanline, bg_color != 0, true);
                }
            }
        }
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
    fn sprite_size_height() {
        use crate::ppu::registers::SpriteSize;
        let mut regs = PpuRegisters::new();
        regs.ppuctrl = 0x00; // 8x8
        assert_eq!(regs.sprite_size(), SpriteSize::Size8x8);
        assert_eq!(regs.sprite_size().height(), 8);
        regs.ppuctrl = 0x20; // 8x16
        assert_eq!(regs.sprite_size(), SpriteSize::Size8x16);
        assert_eq!(regs.sprite_size().height(), 16);
    }
}