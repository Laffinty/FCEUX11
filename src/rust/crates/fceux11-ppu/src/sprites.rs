//! Sprite layer preparation + per-pixel lookup (v2.1.3 batch 2).
//!
//! The per-dot sprite pipeline (secondary-OAM eval at dots 65-256,
//! garbled fetch at dots 257-320, per-dot shift registers with x
//! counters, mid-scanline tile reloads) is batch 4 of
//! `docs/plans/v2.1.3_ppu_accuracy_plan.md`. Until then
//! [`prepare_line`] re-scans primary OAM once per visible scanline 鈥?//! called from the BG pipeline at dot 0 鈥?and loads the eight-sprite
//! selection into `PpuState::sprite_shift` / `sprite_attr` /
//! `sprite_x`. [`pick_pixel`] then resolves one pixel on demand, so the
//! background pipeline composes background and sprite pixels at the
//! exact dot it produces them (the batch compositor that wrote whole
//! scanlines into the framebuffer is gone).
//!
//! Implemented behaviors (matching nesdev):
//! - 8x8 and 8x16 sprites, with the 8x16 tile bit 0 selecting the
//!   pattern table and the upper 7 bits selecting the tile pair.
//! - 8-sprites-per-scanline limit in OAM order; 9th+ in-range sprite
//!   sets the overflow flag (state.sprite_overflow).
//! - Priority MUX: attribute bit 5 = 0 renders in front of BG,
//!   = 1 renders behind (only visible where BG is transparent).
//! - Horizontal (attribute bit 6) and vertical (bit 7) flip, including
//!   the 8x16 subtile exchange for vertical flip (nesdev PPU_OAM).
//! - Sprite pixels address the sprite palette region (0x10 plus the
//!   attribute select times 4 plus the 2-bit color).
//!
//! Known simplifications vs. cycle-accurate hardware (batch 4):
//! - No per-dot shift registers: the pattern row is fetched once per
//!   scanline and indexed per pixel.
//! - The left-edge 8-pixel clip ($2001 bits 1-2) is not applied.
//! - OAM de-emphasis / corruption and the OAMADDR quirks are not
//!   modelled.

use crate::state::{MAX_SPRITES_PER_LINE, PpuState};

// $2000 (ctrl) bits.
/// Bit 5 of $2000: sprite size (0 = 8x8, 1 = 8x16).
const SPRITE_SIZE: u8 = 1 << 5;
/// Bit 3 of $2000: sprite pattern table base ($0000 or $1000).
const SPRITE_PATTERN: u8 = 1 << 3;
// $2001 (mask) bits.
/// Bit 4 of $2001: show sprites.
const SHOW_SPRITES: u8 = 1 << 4;

/// Maximum CHR address (pattern tables live in $0000-$1FFF).
const CHR_ADDR_MASK: u16 = 0x1FFF;

/// Load the eight-sprite selection for one visible scanline.
///
/// Re-scans primary OAM in order (the secondary-OAM copy and the
/// dot-by-dot evaluation timing are batch 4), fetches each in-range
/// sprite's pattern row from `chr`, and stores the result in
/// [`PpuState::sprite_shift`] / [`PpuState::sprite_attr`] /
/// [`PpuState::sprite_x`] for [`pick_pixel`]. Also maintains
/// `sprite0_in_range`, `sprite_overflow` and `secondary_oam_count`.
pub fn prepare_line(state: &mut PpuState, chr: &[u8; 8192], sl: i16, mask: u8) {
    if !(0..=239).contains(&sl) || (mask & SHOW_SPRITES) == 0 {
        return;
    }

    let sprite_height: i16 = if state.registers.ctrl & SPRITE_SIZE != 0 {
        16
    } else {
        8
    };
    let sprite_pattern_base: u16 = if state.registers.ctrl & SPRITE_PATTERN != 0 {
        0x1000
    } else {
        0x0000
    };

    let mut n_sprites = 0usize;
    let mut sprite_overflow = false;
    let mut sprite0_in_range = false;
    let mut shift = [[0u8; 2]; MAX_SPRITES_PER_LINE];
    let mut attrs = [0u8; MAX_SPRITES_PER_LINE];
    let mut xs = [0u8; MAX_SPRITES_PER_LINE];

    for i in 0..64usize {
        let y = state.oam[i * 4];
        let tile = state.oam[i * 4 + 1];
        let attr = state.oam[i * 4 + 2];
        let x = state.oam[i * 4 + 3];
        // Sprite Y of $FF means "off scanline" (legacy convention).
        let in_range = y < 0xFF && (y as i16) <= sl && sl < (y as i16) + sprite_height;
        if in_range && n_sprites < MAX_SPRITES_PER_LINE {
            let fine_y = (sl - y as i16) as u16;
            // OAM attribute bit 7 = vertical flip: mirror the row inside the
            // sprite. In 8x16 mode this also exchanges the two subtiles --
            // nesdev PPU_OAM: the odd-numbered tile of a vertically flipped
            // sprite is drawn on top -- the pair addressing below handles the exchange
            let row = if attr & 0x80 != 0 {
                (sprite_height as u16) - 1 - fine_y
            } else {
                fine_y
            };
            let pat_addr = if sprite_height == 16 {
                // 8x16: tile bit 0 selects the pattern table, the
                // upper 7 bits select the tile pair.
                let base: u16 = if tile & 1 != 0 { 0x1000 } else { 0x0000 };
                // Tile pair: rows 0..7 use the even tile, rows 8..15 the odd
                // one (PPU_OAM). Adding row directly would read the previous
                // tile hi-plane for the lower half. Vertical flip exchanges
                // the two tiles here for free.
                base + ((tile as u16 & 0xFE) + (row >> 3)) * 16 + (row & 7)
            } else {
                sprite_pattern_base + tile as u16 * 16 + row
            };
            attrs[n_sprites] = attr;
            xs[n_sprites] = x;
            shift[n_sprites][0] = chr[(pat_addr & CHR_ADDR_MASK) as usize];
            shift[n_sprites][1] = chr[((pat_addr + 8) & CHR_ADDR_MASK) as usize];
            if i == 0 {
                sprite0_in_range = true;
            }
            n_sprites += 1;
        } else if in_range {
            // 9th+ in-range sprite: overflow (nesdev). Keep scanning so
            // OAM order is preserved for the eight that were kept.
            sprite_overflow = true;
        }
    }

    state.sprite_overflow = sprite_overflow;
    state.sprite0_in_range = sprite0_in_range;
    state.sprite_shift = shift;
    state.sprite_attr = attrs;
    state.sprite_x = xs;
    state.secondary_oam_count = n_sprites as u8;
}

/// Highest-priority opaque sprite pixel at framebuffer column `x`.
///
/// Returns `(sprite index, 2-bit pattern colour)` for the first sprite in
/// OAM order that is opaque at `x`; `None` when every selected sprite is
/// transparent there. Index 0 is OAM sprite 0, which is what the
/// sprite-0 hit test keys on.
#[inline]
pub fn pick_pixel(state: &PpuState, x: u16) -> Option<(usize, u8)> {
    let n = (state.secondary_oam_count as usize).min(MAX_SPRITES_PER_LINE);
    for i in 0..n {
        let sprite_x_pos = state.sprite_x[i] as u16;
        if x < sprite_x_pos || x - sprite_x_pos >= 8 {
            continue;
        }
        // OAM attribute bit 6 = horizontal flip: the leftmost pixel then
        // comes from pattern bit 0 (nesdev PPU_OAM).
        let bit = if state.sprite_attr[i] & 0x40 != 0 {
            x - sprite_x_pos
        } else {
            7 - (x - sprite_x_pos)
        };
        let pat0 = (state.sprite_shift[i][0] >> bit) & 1;
        let pat1 = (state.sprite_shift[i][1] >> bit) & 1;
        let color_2bit = pat0 | (pat1 << 1);
        if color_2bit != 0 {
            return Some((i, color_2bit));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PpuState;

    const SHOW_SPRITES_ONLY: u8 = SHOW_SPRITES;

    fn set_sprite(oam: &mut [u8; 256], idx: usize, y: u8, tile: u8, attr: u8, x: u8) {
        oam[idx * 4] = y;
        oam[idx * 4 + 1] = tile;
        oam[idx * 4 + 2] = attr;
        oam[idx * 4 + 3] = x;
    }

    /// Prepare scanline `sl` and return the composed sprite layer as
    /// `(sprite index, 2-bit colour)` per framebuffer column.
    fn pick_row(
        state: &mut PpuState,
        chr: &[u8; 8192],
        mask: u8,
        sl: i16,
    ) -> [Option<(usize, u8)>; 256] {
        prepare_line(state, chr, sl, mask);
        std::array::from_fn(|x| pick_pixel(state, x as u16))
    }

    #[test]
    fn prepare_line_selects_in_range_sprites_in_oam_order() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 45, 0x01, 0x00, 10);
        set_sprite(&mut state.oam, 1, 48, 0x02, 0x01, 20);
        set_sprite(&mut state.oam, 2, 50, 0x03, 0x02, 30);
        set_sprite(&mut state.oam, 3, 80, 0x04, 0x03, 40); // out of range

        let chr = [0u8; 8192];
        prepare_line(&mut state, &chr, 50, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 3);
        assert_eq!(state.sprite_x[0], 10);
        assert_eq!(state.sprite_x[1], 20);
        assert_eq!(state.sprite_x[2], 30);
        assert!(state.sprite0_in_range, "OAM sprite 0 is in range");
        assert!(!state.sprite_overflow);
    }

    #[test]
    fn eight_sprite_limit_observed_and_overflow_flagged() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        for i in 0..10usize {
            set_sprite(&mut state.oam, i, 0, 0x01, 0x00, (10 + 8 * i) as u8);
        }
        let chr = [0u8; 8192];
        prepare_line(&mut state, &chr, 0, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 8);
        assert!(state.sprite_overflow, "9th+ in-range sprite sets overflow");
        // Sprite 8 exists but is not part of the selection.
        assert_eq!(state.sprite_x[7], 66);
    }

    #[test]
    fn sprite_outside_scanline_y_range_excluded() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 10, 0x01, 0x00, 0);
        let chr = [0u8; 8192];
        prepare_line(&mut state, &chr, 17, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 1, "sl 17 is inside Y=10..17");
        prepare_line(&mut state, &chr, 18, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 0, "sl 18 is past Y+8");
        prepare_line(&mut state, &chr, 9, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 0, "sl 9 precedes Y=10");
    }

    #[test]
    fn sprite_hidden_when_show_sprites_cleared() {
        const SHOW_BG: u8 = 1 << 3;
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG); // sprites off
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0xFF;
        prepare_line(&mut state, &chr, 0, SHOW_BG);
        assert_eq!(
            state.secondary_oam_count, 0,
            "no sprite selection while SHOW_SPRITES is clear"
        );
    }

    #[test]
    fn pick_pixel_returns_none_for_transparent_pattern() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        let chr = [0u8; 8192]; // both planes zero 鈫?fully transparent
        let picks = pick_row(&mut state, &chr, SHOW_SPRITES_ONLY, 0);
        assert!(picks.iter().all(|p| p.is_none()));
    }

    #[test]
    fn sprite_in_front_selects_its_pattern_bits() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0xFF; // plane 0 all ones 鈫?colour 1
        let picks = pick_row(&mut state, &chr, SHOW_SPRITES_ONLY, 0);
        for x in 10..18 {
            assert_eq!(picks[x], Some((0, 1)), "sprite covers x {x}");
        }
        assert_eq!(picks[9], None, "pixel before the sprite");
        assert_eq!(picks[18], None, "pixel after the sprite");
    }

    #[test]
    fn pick_pixel_prefers_the_lower_oam_index() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        // Sprite 0 covers x 10..18 with colour 1, sprite 1 overlaps at
        // x 12..20 with colour 2; OAM order gives sprite 0 priority.
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        set_sprite(&mut state.oam, 1, 0, 0x02, 0x00, 12);
        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0xFF; // colour 1
        chr[0x02 * 16] = 0xFE; // colour: bit 0 clear, others set
        let picks = pick_row(&mut state, &chr, SHOW_SPRITES_ONLY, 0);
        assert_eq!(picks[12], Some((0, 1)), "sprite 0 wins inside its span");
        assert_eq!(picks[18], Some((1, 1)), "sprite 1 shows past sprite 0");
    }

    #[test]
    fn sprite_horizontal_flip_mirrors_pattern() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x40, 10);
        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0x80; // only pattern bit 7 set
        let picks = pick_row(&mut state, &chr, SHOW_SPRITES_ONLY, 0);
        assert_eq!(picks[10], None, "unflipped bit 7 moved to the right edge");
        assert_eq!(
            picks[17],
            Some((0, 1)),
            "flipped bit 7 shows at the right edge"
        );
    }

    #[test]
    fn sprite_vertical_flip_mirrors_rows() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x80, 10);
        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0x80; // row 0
        chr[0x01 * 16 + 7] = 0x01; // row 7
        let picks = pick_row(&mut state, &chr, SHOW_SPRITES_ONLY, 0);
        assert_eq!(picks[10], None, "vertical flip: sl 0 uses pattern row 7");
        assert_eq!(
            picks[17],
            Some((0, 1)),
            "row 7 bit 0 is the rightmost pixel"
        );
    }

    #[test]
    fn eight_by_sixteen_sprites_select_pattern_table_via_tile_bit0() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        state.registers.ctrl |= SPRITE_SIZE; // 8x16 sprites
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10); // tile bit 0 = 1 鈫?$1000
        let mut chr = [0u8; 8192];
        chr[0x1000] = 0xFF; // tile $00/$01 pair, row 0 plane 0
        prepare_line(&mut state, &chr, 0, SHOW_SPRITES_ONLY);
        assert_eq!(
            state.sprite_shift[0][0], 0xFF,
            "8x16 tile $01 must read the $1000 pattern table"
        );
    }

    #[test]
    fn eight_by_sixteen_lower_half_uses_the_odd_tile() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        state.registers.ctrl |= SPRITE_SIZE;
        set_sprite(&mut state.oam, 0, 0, 0x02, 0x00, 10);
        let mut chr = [0u8; 8192];
        chr[0x20] = 0x00; // tile $02 row 0
        chr[0x30] = 0xFF; // tile $03 row 0 (rows 8..15 of the pair)
        prepare_line(&mut state, &chr, 8, SHOW_SPRITES_ONLY);
        assert_eq!(
            state.sprite_shift[0][0], 0xFF,
            "sl 8 of an 8x16 sprite must read the odd tile of the pair"
        );
    }

    #[test]
    fn sprite_uses_sprite_palette_region() {
        // The composition (rendering.rs) reads palette[0x10 + ...]; the
        // sprite layer only supplies the 2-bit colour plus the attribute
        // select, so this pins the attribute plumbing.
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x02, 10); // palette select 2
        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0xFF; // plane 0
        chr[0x01 * 16 + 8] = 0xFF; // plane 1
        let picks = pick_row(&mut state, &chr, SHOW_SPRITES_ONLY, 0);
        let (idx, color) = picks[10].expect("opaque sprite pixel");
        assert_eq!(idx, 0);
        assert_eq!(color, 3, "both planes set 鈫?2-bit colour 3");
        assert_eq!(
            state.sprite_attr[0] & 0x03,
            0x02,
            "palette select preserved"
        );
    }

    #[test]
    fn prepare_line_is_inert_outside_visible_lines() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        let chr = [0u8; 8192];
        prepare_line(&mut state, &chr, 240, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 0);
        prepare_line(&mut state, &chr, -1, SHOW_SPRITES_ONLY);
        assert_eq!(state.secondary_oam_count, 0);
    }
}
