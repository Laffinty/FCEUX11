//! Phase 6.2: sprite pixel composition (post-BG pass).
//!
//! The per-cycle sprite pipeline (secondary-OAM eval at dots 65-256,
//! garbled fetch at dots 257-320, per-dot shift registers with x
//! counters, mid-scanline tile reloads) is deferred to a follow-up
//! sub-task. For the batch renderer we instead re-scan primary OAM
//! once per visible scanline, fetch each in-range sprite's pattern
//! row directly from CHR, and composite the pixels over the BG
//! output that `rendering::render_scanline` just wrote.
//!
//! Implemented behaviors (matching nesdev):
//! - 8x8 and 8x16 sprites, with the 8x16 tile bit 0 selecting the
//!   pattern table and the upper 7 bits selecting the tile pair.
//! - 8-sprites-per-scanline limit in OAM order; 9th+ in-range sprite
//!   sets the overflow flag (state.sprite_overflow).
//! - Priority MUX: attribute bit 5 = 0 renders in front of BG,
//!   = 1 renders behind (only visible where BG is transparent).
//! - Sprite 0 hit: set when sprite 0's opaque pixel overlaps an
//!   opaque BG pixel at x 1..=254, with front priority.
//!
//! Known simplifications vs. cycle-accurate hardware:
//! - No per-dot shift registers: the pattern row is fetched once per
//!   scanline and indexed per pixel.
//! - No flip handling is wired through the batch path (attr bits 6/7
//!   are latched but horizontal/vertical flip are not applied yet).
//! - The left-edge 8-pixel clip ($2001 bits 1-2) is not applied.
//! - Palette lookup uses `palette[color_2bit | quadrant << 2]`
//!   (0x00-0x0F region), matching the BG writer's convention.

use crate::bus::PpuBus;
use crate::rendering::palette_adjust_pixel;
use crate::state::{MAX_SPRITES_PER_LINE, PpuState};

// $2000 (ctrl) bits.
/// Bit 5 of $2000: sprite size (0 = 8x8, 1 = 8x16).
const SPRITE_SIZE: u8 = 1 << 5;
/// Bit 3 of $2000: sprite pattern table base ($0000 or $1000).
const SPRITE_PATTERN: u8 = 1 << 3;
// $2001 (mask) bits.
/// Bit 3 of $2001: show background.
const SHOW_BG: u8 = 1 << 3;
/// Bit 4 of $2001: show sprites.
const SHOW_SPRITES: u8 = 1 << 4;
/// Bit 0 of $2001: grayscale.
const GRAYSCALE: u8 = 1 << 0;
// OAM attribute bits.
/// Bit 5 of the attribute byte: priority (0 = front, 1 = behind BG).
const SPRITE_PRIORITY_BIT: u8 = 1 << 5;

/// Maximum CHR address (pattern tables live in $0000-$1FFF).
const CHR_ADDR_MASK: u16 = 0x1FFF;

/// Phase 6.2 per-scanline batch sprite composition.
///
/// Re-scans primary OAM for the current scanline (the secondary-OAM
/// copy is left to the cycle-accurate path; here we bypass its
/// start-of-scanline timing entirely), fetches pattern rows, and
/// composites sprite pixels over the BG pixels already present in
/// `framebuffer` for row `sl`.
///
/// `palette` is the 32-byte PALRAM window; `mask` is the $2001 value.
pub fn render_sprites_for_scanline<B: PpuBus + ?Sized>(
    state: &mut PpuState,
    bus: &mut B,
    framebuffer: &mut [u8; 256 * 256],
    palette: &[u8; 32],
    mask: u8,
    sl: i16,
) {
    if !((0..=239).contains(&sl)) {
        return;
    }
    if (mask & SHOW_SPRITES) == 0 {
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
    let bg_show = (mask & SHOW_BG) != 0;
    let gray_mask: u8 = if mask & GRAYSCALE != 0 { 0x30 } else { 0xFF };

    // Step 1: scan primary OAM for in-range sprites (max 8, OAM order).
    let mut spr_tile = [0u8; MAX_SPRITES_PER_LINE];
    let mut spr_attr = [0u8; MAX_SPRITES_PER_LINE];
    let mut spr_x = [0u8; MAX_SPRITES_PER_LINE];
    let mut spr_pat_lo = [0u8; MAX_SPRITES_PER_LINE];
    let mut spr_pat_hi = [0u8; MAX_SPRITES_PER_LINE];
    let mut n_sprites = 0usize;
    let mut sprite_overflow = false;
    let mut sprite0_in_range = false;

    for i in 0..64usize {
        let y = state.oam[i * 4];
        let tile = state.oam[i * 4 + 1];
        let attr = state.oam[i * 4 + 2];
        let x = state.oam[i * 4 + 3];
        // Sprite Y of $FF means "off scanline" (legacy convention).
        let in_range = y < 0xFF && (y as i16) <= sl && sl < (y as i16) + sprite_height;
        if in_range && n_sprites < MAX_SPRITES_PER_LINE {
            let fine_y = (sl - y as i16) as u16;
            let pat_addr = if sprite_height == 16 {
                // 8x16: tile bit 0 selects the pattern table, the
                // upper 7 bits select the tile pair.
                let base: u16 = if tile & 1 != 0 { 0x1000 } else { 0x0000 };
                base + (tile as u16 & 0xFE) * 16 + fine_y
            } else {
                sprite_pattern_base + tile as u16 * 16 + fine_y
            };
            spr_tile[n_sprites] = tile;
            spr_attr[n_sprites] = attr;
            spr_x[n_sprites] = x;
            spr_pat_lo[n_sprites] = bus.peek_chr(pat_addr & CHR_ADDR_MASK);
            spr_pat_hi[n_sprites] = bus.peek_chr((pat_addr + 8) & CHR_ADDR_MASK);
            if i == 0 {
                sprite0_in_range = true;
            }
            n_sprites += 1;
        } else if in_range {
            sprite_overflow = true;
            // 9th+ in-range sprite: overflow per nesdev. The status
            // bit is set elsewhere (rendering.rs mirror); here we
            // just record the flag for future status updates.
            // Don't break — keep scanning to maintain OAM order in
            // case future code wants sprite count beyond 8.
        }
    }

    // Persist for the snapshot / frame state machine.
    state.sprite_overflow = sprite_overflow;
    state.sprite0_in_range = sprite0_in_range;

    // Pre-fill the per-sprite shift register / attribute / x arrays.
    // The Phase 6.2 batch renderer doesn't actually use these (we
    // re-fetch pattern bytes inline below), but `state.sprite_shift`
    // is read by `fceux11_ppu_get_*` debug accessors and must stay
    // self-consistent for the savestate round-trip.
    for i in 0..MAX_SPRITES_PER_LINE {
        state.sprite_shift[i][0] = if i < n_sprites { spr_pat_lo[i] } else { 0 };
        state.sprite_shift[i][1] = if i < n_sprites { spr_pat_hi[i] } else { 0 };
        state.sprite_attr[i] = if i < n_sprites { spr_attr[i] } else { 0 };
        state.sprite_x[i] = if i < n_sprites { spr_x[i] } else { 0 };
    }
    state.secondary_oam_count = n_sprites as u8;

    let row_off = (sl as usize) * 256;

    // Step 2 + 3 + 4: per-pixel sprite lookup + priority MUX + sprite0 hit.
    for x in 0..256u16 {
        let x_usize = x as usize;

        // Pick the highest-priority sprite visible at this pixel.
        // (i = 0 is highest priority per OAM ordering.)
        let mut chosen: Option<(usize, u8)> = None; // (sprite idx, color_2bit)
        for i in 0..n_sprites {
            let sprite_x_pos = spr_x[i] as u16;
            if x < sprite_x_pos {
                continue;
            }
            // Sprite covers 8 pixels starting at sprite_x_pos. Pixels
            // past sprite_x_pos + 8 belong to the NEXT tile of the
            // same sprite (in the cycle-accurate engine); for the
            // Phase 6.2 batch model we simply say "this sprite has
            // already finished its first tile" and skip it. The next
            // tile would be loaded with the pattern bytes for the
            // following 8 rows; the Phase 6.2 batch simplifies to "one
            // tile per scanline" (fine_y is the only row dimension).
            if x - sprite_x_pos >= 8 {
                continue;
            }
            // Bit position within the 8-bit pattern: 7 - (x - sprite_x_pos).
            let bit = 7u16 - (x - sprite_x_pos);
            let pat0 = (spr_pat_lo[i] >> bit) & 1;
            let pat1 = (spr_pat_hi[i] >> bit) & 1;
            let color_2bit = pat0 | (pat1 << 1);
            if color_2bit == 0 {
                // Transparent sprite pixel — try the next sprite.
                continue;
            }
            chosen = Some((i, color_2bit));
            break;
        }

        let bg_out = framebuffer[row_off + x_usize];
        // BG transparency: the framebuffer stores the final palette
        // RAM index (with emphasis/grayscale tags in the upper bits),
        // so a "transparent" BG pixel is the backdrop entry — low
        // color bits and attribute quadrant both 0.
        let bg_color_2bit = bg_out & 0x3;
        let bg_palette = (bg_out >> 2) & 0x3;
        let bg_is_transparent = !bg_show || (bg_color_2bit == 0 && bg_palette == 0);

        let new_pixel = match chosen {
            None => bg_out,
            Some((i, color_2bit)) => {
                let attr = spr_attr[i];
                let attr_quadrant = attr & 0x3;
                let priority_behind_bg = (attr & SPRITE_PRIORITY_BIT) != 0;
                let sprite_color_4bit = color_2bit | (attr_quadrant << 2);
                let sprite_wins = !priority_behind_bg || bg_is_transparent;
                if sprite_wins {
                    let pal_idx = palette[sprite_color_4bit as usize] & gray_mask;
                    palette_adjust_pixel(pal_idx, mask)
                } else {
                    bg_out
                }
            }
        };

        if new_pixel != bg_out {
            framebuffer[row_off + x_usize] = new_pixel;
        }

        // Sprite 0 hit: sprite 0's opaque pixel over an opaque BG
        // pixel, front priority only, x in 1..=254 (hardware skips
        // the left/right edge columns).
        if let Some((i, _)) = chosen {
            if sprite0_in_range
                && i == 0
                && x >= 1
                && x <= 254
                && !bg_is_transparent
                && (spr_attr[i] & SPRITE_PRIORITY_BIT) == 0
            {
                state.sprite0_hit = true;
            }
        }
    }

    // `spr_tile` is only used for pattern addressing above; keep the
    // binding alive for future mid-scanline reload work.
    let _ = spr_tile;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::FlatBus;
    use crate::state::PpuState;

    const SHOW_SPRITES_ONLY: u8 = SHOW_SPRITES;
    const SHOW_BG_AND_SPRITES: u8 = SHOW_BG | SHOW_SPRITES;

    fn set_sprite(oam: &mut [u8; 256], idx: usize, y: u8, tile: u8, attr: u8, x: u8) {
        oam[idx * 4] = y;
        oam[idx * 4 + 1] = tile;
        oam[idx * 4 + 2] = attr;
        oam[idx * 4 + 3] = x;
    }

    /// Render one scanline into a fresh framebuffer and return row `sl`.
    fn render_row(
        state: &mut PpuState,
        bus: &mut FlatBus,
        palette: &[u8; 32],
        mask: u8,
        sl: i16,
    ) -> [u8; 256] {
        let mut fb = [0u8; 256 * 256];
        render_sprites_for_scanline(state, bus, &mut fb, palette, mask, sl);
        let row_off = (sl as usize) * 256;
        let mut row = [0u8; 256];
        row.copy_from_slice(&fb[row_off..row_off + 256]);
        row
    }

    /// Same as [`render_row`] but with a pre-filled BG row (the caller
    /// supplies the pixel value the BG pass would have written).
    fn render_row_over_bg(
        state: &mut PpuState,
        bus: &mut FlatBus,
        palette: &[u8; 32],
        mask: u8,
        sl: i16,
        bg_pixel: u8,
    ) -> [u8; 256] {
        let mut fb = [0u8; 256 * 256];
        let row_off = (sl as usize) * 256;
        for x in 0..256 {
            fb[row_off + x] = bg_pixel;
        }
        render_sprites_for_scanline(state, bus, &mut fb, palette, mask, sl);
        let mut row = [0u8; 256];
        row.copy_from_slice(&fb[row_off..row_off + 256]);
        row
    }

    #[test]
    fn single_sprite_in_front_renders() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);

        let mut bus = FlatBus::new();
        // Tile $01 row 0, all bits set → the whole 8-pixel row is opaque.
        bus.chr[0x01 * 16] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;

        let row = render_row(&mut state, &mut bus, &palette, SHOW_SPRITES_ONLY, 0);
        let expected = palette_adjust_pixel(0x16, SHOW_SPRITES_ONLY);
        // Sprite x=10 covers x 10..17 (bit 7 at x=10 … bit 0 at x=17).
        assert_eq!(row[10], expected, "sprite 0 first column (bit 7)");
        assert_eq!(row[17], expected, "sprite 0 last column (bit 0)");
        assert_eq!(row[9], 0, "pixel before sprite stays BG");
        assert_eq!(row[18], 0, "pixel after sprite stays BG");
    }

    #[test]
    fn sprite_behind_renders_when_bg_transparent() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG_AND_SPRITES);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x20, 10); // attr bit 5: behind BG

        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        // Backdrop entry 0 → the BG pass writes the "transparent"
        // backdrop pixel (low 6 bits all zero).
        palette[0] = 0x00;
        let bg_pixel = palette_adjust_pixel(0x00, SHOW_BG_AND_SPRITES);

        let row = render_row_over_bg(
            &mut state,
            &mut bus,
            &palette,
            SHOW_BG_AND_SPRITES,
            0,
            bg_pixel,
        );
        let expected = palette_adjust_pixel(0x16, SHOW_BG_AND_SPRITES);
        assert_eq!(
            row[10], expected,
            "behind-sprite visible where BG is backdrop"
        );
    }

    #[test]
    fn sprite_priority_bit_routes_to_bg() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG_AND_SPRITES);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x20, 10); // behind BG

        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        // Opaque BG pixel: a non-backdrop palette entry (low bits set).
        palette[5] = 0x25;
        let bg_pixel = palette_adjust_pixel(0x25, SHOW_BG_AND_SPRITES);

        let row = render_row_over_bg(
            &mut state,
            &mut bus,
            &palette,
            SHOW_BG_AND_SPRITES,
            0,
            bg_pixel,
        );
        // BG opaque + sprite behind → BG pixel stays.
        assert_eq!(row[10], bg_pixel, "opaque BG hides behind-priority sprite");
        assert_eq!(
            row[17], bg_pixel,
            "opaque BG hides behind-priority sprite (bit 0)"
        );
    }

    #[test]
    fn sprite_zero_hit_only_with_front_priority_and_opaque_bg() {
        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        palette[5] = 0x25;
        let opaque_bg = palette_adjust_pixel(0x25, SHOW_BG_AND_SPRITES);
        let backdrop_bg = palette_adjust_pixel(0x00, SHOW_BG_AND_SPRITES);

        // Front sprite 0 + opaque BG → hit.
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG_AND_SPRITES);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;
        let _ = render_row_over_bg(
            &mut state,
            &mut bus,
            &palette,
            SHOW_BG_AND_SPRITES,
            0,
            opaque_bg,
        );
        assert!(state.sprite0_hit, "front sprite 0 over opaque BG sets hit");

        // Behind-priority sprite 0 + opaque BG → no hit.
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG_AND_SPRITES);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x20, 10);
        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;
        let _ = render_row_over_bg(
            &mut state,
            &mut bus,
            &palette,
            SHOW_BG_AND_SPRITES,
            0,
            opaque_bg,
        );
        assert!(!state.sprite0_hit, "behind-priority sprite 0 does not hit");

        // Front sprite 0 + backdrop BG → no hit.
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG_AND_SPRITES);
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);
        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;
        let _ = render_row_over_bg(
            &mut state,
            &mut bus,
            &palette,
            SHOW_BG_AND_SPRITES,
            0,
            backdrop_bg,
        );
        assert!(
            !state.sprite0_hit,
            "sprite 0 over transparent BG does not hit"
        );
    }

    #[test]
    fn eight_sprite_limit_observed() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        // 10 sprites in a row at x = 10 + 8*i so each occupies a
        // distinct pixel range; sprites 8 and 9 exceed the 8-sprite
        // per-line limit and must be dropped.
        for i in 0..10usize {
            set_sprite(&mut state.oam, i, 0, 0x01, 0x00, (10 + 8 * i) as u8);
        }

        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        let expected = palette_adjust_pixel(0x16, SHOW_SPRITES_ONLY);

        let row = render_row(&mut state, &mut bus, &palette, SHOW_SPRITES_ONLY, 0);
        assert_eq!(row[10], expected, "sprite 0 rendered");
        assert_eq!(row[66], expected, "sprite 7 rendered");
        assert_eq!(row[73], expected, "sprite 7 last column rendered");
        assert_eq!(row[74], 0, "sprite 8 dropped by 8-sprite limit");
        assert_eq!(row[82], 0, "sprite 9 dropped by 8-sprite limit");
        assert!(state.sprite_overflow, "9th+ in-range sprite sets overflow");
        assert_eq!(state.secondary_oam_count, 8);
    }

    #[test]
    fn sprite_outside_scanline_y_range_excluded() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        set_sprite(&mut state.oam, 0, 10, 0x01, 0x00, 0);

        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        let expected = palette_adjust_pixel(0x16, SHOW_SPRITES_ONLY);

        // Sprite Y=10 covers scanlines 10..17.
        let row = render_row(&mut state, &mut bus, &palette, SHOW_SPRITES_ONLY, 10);
        assert_eq!(row[0], expected, "sprite Y=10 visible at sl=10");
        assert_eq!(row[7], expected, "sprite Y=10 last column at sl=10");

        let row = render_row(&mut state, &mut bus, &palette, SHOW_SPRITES_ONLY, 0);
        assert_eq!(row[0], 0, "sprite Y=10 not visible at sl=0");

        let row = render_row(&mut state, &mut bus, &palette, SHOW_SPRITES_ONLY, 18);
        assert_eq!(row[0], 0, "sprite Y=10 not visible at sl=18 (past Y+8)");
    }

    #[test]
    fn eight_by_sixteen_sprites_select_pattern_table_via_tile_bit0() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_SPRITES_ONLY);
        state.registers.ctrl |= SPRITE_SIZE; // 8x16 sprites
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10); // tile bit 0 = 1 → $1000

        let mut bus = FlatBus::new();
        // 8x16 tile $01: bit 0 = 1 → pattern table $1000, tile pair
        // index (0x01 & 0xFE) = 0. Row 0 (fine_y = 0) plane 0 is at
        // $1000, plane 1 at $1008.
        bus.chr[0x1000] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        let expected = palette_adjust_pixel(0x16, SHOW_SPRITES_ONLY);

        let row = render_row(&mut state, &mut bus, &palette, SHOW_SPRITES_ONLY, 0);
        assert_eq!(row[10], expected, "8x16 tile $01 fetches from $1000");
    }

    #[test]
    fn sprite_hidden_when_show_sprites_cleared() {
        let mut state = PpuState::new();
        state.registers.write_mask(SHOW_BG); // sprites off
        set_sprite(&mut state.oam, 0, 0, 0x01, 0x00, 10);

        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF;

        let mut palette = [0u8; 32];
        palette[1] = 0x16;

        let row = render_row(&mut state, &mut bus, &palette, SHOW_BG, 0);
        assert!(
            row.iter().all(|&p| p == 0),
            "no sprite pixels when SHOW_SPRITES is cleared"
        );
    }
}
