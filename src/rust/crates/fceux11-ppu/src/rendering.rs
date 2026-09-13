//! v2.1.3 batch 2: per-dot background fetch pipeline.
//!
//! Replaces the Phase 4 per-scanline batch renderer
//! (`render_scanline`), which fetched all 34 NT/AT/PT groups at dot 0 of
//! each visible scanline and painted 256 pixels before the scanline
//! started. Hardware spreads those accesses across dots 1-256 (plus the
//! 321-336 preload for the next line), so anything the CPU or a mapper
//! changes *inside* a scanline — `$2005`/`$2006`/`$2000` scroll writes,
//! `$2007` CHR/NT writes, mapper CHR bank switches — could only be
//! modelled a whole line late (or not at all).
//!
//! ## Per-dot model
//!
//! [`tick_dot`] is called once per PPU dot (before [`crate::frame::tick_dot`]
//! for the same dot) by the per-dot drivers in `ffi.rs`. For each fetch
//! line (pre-render `-1` or visible `0..=239`) with rendering enabled:
//!
//! - dots 1-256 / 321-336: one 8-dot group per tile. Group phase 0
//!   fetches the nametable byte, phase 2 the attribute byte, phase 4 the
//!   pattern low plane, phase 6 the pattern high plane. The two pattern
//!   shift registers shift left once on every dot of the group and the
//!   freshly fetched pattern bytes are loaded into the low byte at
//!   phase 7 (the last dot of the group, the same dot
//!   [`crate::frame::tick_dot`] increments coarse X).
//! - dot 0 (visible lines): the sprite layer for the scanline is
//!   prepared from primary OAM (the per-dot sprite evaluation of batch 4
//!   replaces this).
//! - dots 257-320: the sprite pattern fetch window. No BG pixel output
//!   and no BG shift; the addresses are still reported to the A12
//!   watcher.
//! - dots 337-340: the two garbage nametable fetches.
//!
//! Pixel `x = dot - 1` is emitted as the PPU reaches dot `x + 1`, using
//! the current `fine_x` to select the bit from the shift registers, so a
//! `$2005` fine-X write takes effect on the very next pixel.
//!
//! ## Attribute model
//!
//! Each 8-pixel string shares one attribute: the attribute byte latched
//! during a group is the one that applies to the tile that group
//! fetched. This is the hardware model documented in
//! `docs/knowledge_base/ppu/ppu_rendering.md` ("because the PPU can only
//! fetch an attribute byte every 8 cycles, each sequential string of 8
//! pixels is forced to have the same palette attribute"), and what
//! Mesen2 / Nintendulator / ares / puNES implement. FCEUX's `atlatch`
//! (the last four pixels of a tile taking the NEXT tile's attribute) was
//! a tile-loop artifact, removed here per the batch 2 decision in
//! `docs/plans/v2.1.3_ppu_accuracy_plan.md` §4 and
//! `docs/knowledge_base/reference/dot_ppu_designs.md` §2.6.

use crate::bus::PpuBus;
use crate::registers::{ctrl_bits, mask_bits};
use crate::state::PpuState;

/// Bit 0 of `$2001`: grayscale.
const GRAYSCALE: u8 = 1 << mask_bits::GRAYSCALE;
/// Bit 3 of `$2001`: show BG.
const SHOW_BG: u8 = 1 << mask_bits::SHOW_BG;
/// Bit 4 of `$2001`: show sprites.
const SHOW_SPRITES: u8 = 1 << mask_bits::SHOW_SPRITES;
/// Bit 4 of `$2000`: BG pattern table select.
const BG_PATTERN_BIT: u8 = 1 << ctrl_bits::BG_PATTERN;
/// Bit 5 of the sprite attribute byte: priority (0 = front, 1 = behind BG).
const SPRITE_PRIORITY_BIT: u8 = 1 << 5;

/// Visible pixels per scanline.
pub const VISIBLE_PIXELS: usize = 256;

/// The NT / CHR / palette views the pipeline reads through.
///
/// The C++ bridge installs these as copies of the authoritative mapper
/// state (`g_nt_window` / `g_chr_window` / live `PALRAM`) and re-copies
/// the pages whose backing pointer moved once per 8-dot group
/// (`PpuBus::refresh_windows`), so a mid-scanline CHR bank switch is
/// visible to the very next fetch group.
pub struct RenderWindows<'a> {
    /// 4 KiB nametable window (four-screen mappers address all four pages).
    pub nt: &'a [u8; 4096],
    /// 8 KiB CHR window.
    pub chr: &'a [u8; 8192],
    /// 32-byte palette RAM window.
    pub palette: &'a [u8; 32],
    /// Mirroring mode: 0=horizontal, 1=vertical, 2=single_lo, 3=single_hi, 4=four.
    pub mirror: u8,
}

/// Replicates the C++ `PaletteAdjustPixel` XBuf byte format
/// (`src/ppu_rendering.cpp:1524-1531`). The framebuffer the bridge
/// copies into XBuf must carry the same high-bit flags as the C++
/// new PPU's output: no emphasis → `(color & 0x3F) | 0x80`, any
/// emphasis (not all three) → `color | 0x40`, all three emphasis
/// bits ($2001 >> 5 == 0x7) → `(color & 0x3F) | 0xC0`.
pub fn palette_adjust_pixel(color: u8, mask: u8) -> u8 {
    if (mask >> 5) == 0x7 {
        (color & 0x3F) | 0xC0
    } else if (mask & 0xE0) != 0 {
        color | 0x40
    } else {
        (color & 0x3F) | 0x80
    }
}

/// Maps a 14-bit VRAM address to the corresponding 1 KiB nametable
/// page, applying the configured mirroring mode. Returns the index
/// into the 4 KiB NT window.
fn map_nametable_addr(v: u16, mode: u8) -> u16 {
    let nt = ((v >> 10) & 0x3) as u16;
    let coarse = v & 0x03FF;
    let mapped_nt: u16 = match mode {
        0 => nt & 0x1,
        1 => nt >> 1,
        2 => 0,
        3 => 1,
        4 => nt,
        _ => nt,
    };
    (mapped_nt << 10) | coarse
}

/// Extract the 2-bit palette quadrant from a single attribute byte,
/// given the tile's coarse X and the bit-1 of coarse Y (passed as 0 or 4).
fn attribute_quadrant(at_byte: u8, coarse_x: u16, coarse_y_bit1: u8) -> u8 {
    let shift = ((coarse_x & 2) + (coarse_y_bit1 as u16 & 0x4)) as u8;
    (at_byte >> shift) & 0x3
}

/// Attribute-table address for the current `v` (nesdev PPU scrolling).
#[inline]
fn attribute_addr(v: u16) -> u16 {
    0x23C0 | (v & 0x0C00) | ((v >> 4) & 0x38) | ((v >> 2) & 0x07)
}

/// Pattern-table address of the tile latched by the current fetch group.
#[inline]
fn bg_pattern_addr(ctrl: u8, nt: u8, v: u16) -> u16 {
    let base: u16 = if ctrl & BG_PATTERN_BIT != 0 { 0x1000 } else { 0x0000 };
    base | ((nt as u16) << 4) | ((v >> 12) & 0x7)
}

/// Present a PPU bus address to the filtered A12 watcher and forward a
/// real rising edge to the cartridge hook (MMC3-family IRQ counters).
#[inline]
fn report_bus<B: PpuBus + ?Sized>(state: &mut PpuState, bus: &mut B, addr: u16) {
    if state.a12.observe(addr) {
        bus.notify_a12_rising();
    }
}

/// Advance the background pipeline by one PPU dot and emit the pixel
/// that dot produces.
///
/// Must be called with `(state.scanline, state.dot)` already positioned
/// on the dot being processed, and *before* [`crate::frame::tick_dot`]
/// for the same dot (the frame state machine owns the `v` coarse-X /
/// fine-Y / horizontal-copy updates that the fetch addresses depend on).
pub fn tick_dot<B: PpuBus + ?Sized>(
    state: &mut PpuState,
    bus: &mut B,
    win: &RenderWindows<'_>,
    framebuffer: &mut [u8; 256 * 256],
) {
    let sl = state.scanline;
    let dot = state.dot;

    if !(0..=239).contains(&sl) {
        // Pre-render line: the full fetch cycle runs (it preloads the
        // first two tiles of scanline 0 and clocks the A12 watcher), but
        // no pixel is output.
        if sl == -1 && state.rendering_enabled() {
            pipeline_dot(state, bus, win, dot);
        }
        return;
    }

    let mask = state.effective_mask();
    if !state.rendering_enabled() {
        // Rendering fully disabled: the picture region shows the backdrop
        // (or the palette colour `v` points at, see `docs/knowledge_base/
        // ppu/ppu_mid_frame_writes.md` §2), and no fetches happen.
        output_blank_pixel(state, dot, win, framebuffer, mask);
        return;
    }

    let bg_show = mask & SHOW_BG != 0;
    let spr_show = mask & SHOW_SPRITES != 0;

    match dot {
        0 => {
            // Prepare the sprite layer for this scanline. The BG fetch
            // cycle for dots 1-256 consumes it per pixel below.
            if spr_show {
                crate::sprites::prepare_line(state, win.chr, sl, mask);
            }
        }
        1..=256 => {
            let grayscale = mask & GRAYSCALE != 0;
            let hit = emit_pixel(
                state,
                (dot - 1) as usize,
                bg_show,
                spr_show,
                grayscale,
                mask,
                win,
                framebuffer,
            );
            if hit {
                state.sprite0_hit = true;
            }
            pipeline_dot(state, bus, win, dot);
        }
        _ => pipeline_dot(state, bus, win, dot),
    }
}

/// Fetch-cycle activity for one dot (address reporting, byte latching,
/// shift-register advance, preload).
fn pipeline_dot<B: PpuBus + ?Sized>(
    state: &mut PpuState,
    bus: &mut B,
    win: &RenderWindows<'_>,
    dot: u16,
) {
    match dot {
        0 => {
            // Idle cycle on a visible line: the address bus carries the
            // CHR address the first pattern fetch (dot 5) will use. The
            // pre-render line keeps `v` on the bus here (batch 1
            // calibration).
            if (0..=239).contains(&state.scanline) {
                let addr: u16 = if state.registers.ctrl & BG_PATTERN_BIT != 0 {
                    0x1000
                } else {
                    0x0000
                };
                report_bus(state, bus, addr);
            }
        }
        1..=256 => {
            let phase = (dot - 1) & 0x7;
            bg_fetch(state, bus, win, phase);
            shift_and_reload(state, phase);
        }
        257..=320 => {
            // Sprite pattern fetch window: 8 slots x 8 dots. The garbage
            // nametable fetches and the dummy `$FF` pattern fetches of
            // empty slots still drive the cartridge address bus — blargg's
            // MMC3 tests count them.
            let slot = ((dot - 257) >> 3) as usize;
            let phase = (dot - 257) & 0x7;
            let addr = match phase {
                0 | 2 => 0x2000 | (state.registers.v & 0x0FFF),
                4 => crate::a12::sprite_pattern_addr(
                    slot,
                    state.registers.ctrl,
                    state.scanline,
                    &state.secondary_oam,
                    state.secondary_oam_count,
                ),
                _ => return,
            };
            report_bus(state, bus, addr);
        }
        321..=336 => {
            // Preload of the next scanline's first two tiles.
            let phase = (dot - 321) & 0x7;
            bg_fetch(state, bus, win, phase);
            shift_and_reload(state, phase);
        }
        337 | 339 => {
            // Two garbage nametable fetches (MMC5 clocks a scanline
            // counter on this three-fetch string).
            report_bus(state, bus, 0x2000 | (state.registers.v & 0x0FFF));
        }
        _ => {}
    }
}

/// One 8-dot group: latch the byte the group's `phase` fetches.
fn bg_fetch<B: PpuBus + ?Sized>(
    state: &mut PpuState,
    bus: &mut B,
    win: &RenderWindows<'_>,
    phase: u16,
) {
    let v = state.registers.v;
    match phase {
        0 => {
            let addr = 0x2000 | (v & 0x0FFF);
            report_bus(state, bus, addr);
            state.bg_latch_nt = win.nt[map_nametable_addr(addr, win.mirror) as usize];
        }
        2 => {
            let addr = attribute_addr(v);
            report_bus(state, bus, addr);
            let at = win.nt[map_nametable_addr(addr, win.mirror) as usize];
            let coarse_y_bit1 = (((v >> 5) & 0x02) << 1) as u8;
            state.bg_latch_attr = attribute_quadrant(at, v & 0x1F, coarse_y_bit1);
        }
        4 => {
            let addr = bg_pattern_addr(state.registers.ctrl, state.bg_latch_nt, v);
            report_bus(state, bus, addr);
            state.bg_latch_lo = win.chr[(addr & 0x1FFF) as usize];
        }
        6 => {
            let addr = bg_pattern_addr(state.registers.ctrl, state.bg_latch_nt, v) + 8;
            report_bus(state, bus, addr);
            state.bg_latch_hi = win.chr[(addr & 0x1FFF) as usize];
        }
        _ => {}
    }
}

/// Shift both pattern and both attribute registers one dot and, on the
/// last dot of a group, load the freshly fetched bytes. The shift runs on
/// every dot of a fetch region (dots 1-256 and 321-336); `v`'s coarse-X
/// increment for the group is owned by [`crate::frame::tick_dot`].
#[inline]
fn shift_and_reload(state: &mut PpuState, phase: u16) {
    state.bg_pshift[0] <<= 1;
    state.bg_pshift[1] <<= 1;
    state.bg_attr_shift[0] <<= 1;
    state.bg_attr_shift[1] <<= 1;
    if phase == 7 {
        state.bg_pshift[0] = (state.bg_pshift[0] & 0xFF00) | state.bg_latch_lo as u16;
        state.bg_pshift[1] = (state.bg_pshift[1] & 0xFF00) | state.bg_latch_hi as u16;
        let attr = state.bg_latch_attr;
        state.bg_attr_shift[0] =
            (state.bg_attr_shift[0] & 0xFF00) | if attr & 0x01 != 0 { 0x00FF } else { 0x0000 };
        state.bg_attr_shift[1] =
            (state.bg_attr_shift[1] & 0xFF00) | if attr & 0x02 != 0 { 0x00FF } else { 0x0000 };
    }
}

/// Emit pixel `x` from the current shift-register contents and composite
/// the sprite layer over it. Returns `true` when this pixel latches
/// sprite-0 hit.
fn emit_pixel(
    state: &mut PpuState,
    x: usize,
    bg_show: bool,
    spr_show: bool,
    grayscale: bool,
    mask: u8,
    win: &RenderWindows<'_>,
    framebuffer: &mut [u8; 256 * 256],
) -> bool {
    let fine_x = (state.registers.fine_x & 0x07) as u32;
    let bit = 15 - fine_x;

    // Background: index 0 is transparent; the attribute shift registers
    // supply the palette quadrant for this string of 8 pixels.
    let (mut index, bg_opaque) = if bg_show {
        let pat0 = ((state.bg_pshift[0] >> bit) & 1) as u8;
        let pat1 = ((state.bg_pshift[1] >> bit) & 1) as u8;
        let color_2bit = pat0 | (pat1 << 1);
        if color_2bit == 0 {
            (0usize, false)
        } else {
            let attr0 = ((state.bg_attr_shift[0] >> bit) & 1) as u8;
            let attr1 = ((state.bg_attr_shift[1] >> bit) & 1) as u8;
            (
                (color_2bit | (((attr0 | (attr1 << 1)) & 0x03) << 2)) as usize,
                true,
            )
        }
    } else {
        (0usize, false)
    };

    let mut hit = false;
    if spr_show && state.secondary_oam_count != 0 {
        if let Some((i, sprite_color)) = crate::sprites::pick_pixel(state, x as u16) {
            let attr = state.sprite_attr[i];
            let behind_bg = (attr & SPRITE_PRIORITY_BIT) != 0;
            if !behind_bg || !bg_opaque {
                // Sprite palettes live at $3F10-$3F1F.
                index = 0x10 + (sprite_color | ((attr & 0x03) << 2)) as usize;
            }
            // Sprite 0 hit: an opaque sprite-0 pixel over an opaque BG
            // pixel at x 0..=254. The priority bit does not gate the hit.
            if i == 0 && state.sprite0_in_range && bg_opaque && x <= 254 {
                hit = true;
            }
        }
    }

    let gray_mask: u8 = if grayscale { 0x30 } else { 0xFF };
    let pal = win.palette[index & 0x1F] & gray_mask;
    let out = palette_adjust_pixel(pal, mask);
    framebuffer[state.scanline as usize * VISIBLE_PIXELS + x] = out;
    hit
}

/// Rendering-disabled picture output: EXT input (index 0) for every
/// pixel, overridden by the palette colour `v` points at while the low
/// 14 bits of `v` are inside $3F00-$3FFF (the backdrop override some
/// software uses deliberately, e.g. Super Mario Bros. 3).
fn output_blank_pixel(
    state: &PpuState,
    dot: u16,
    win: &RenderWindows<'_>,
    framebuffer: &mut [u8; 256 * 256],
    mask: u8,
) {
    if !(1..=256).contains(&dot) {
        return;
    }
    let v = state.registers.v;
    let index = if (v & 0x3F00) == 0x3F00 {
        (v & 0x1F) as usize
    } else {
        0
    };
    let gray_mask: u8 = if mask & GRAYSCALE != 0 { 0x30 } else { 0xFF };
    let out = palette_adjust_pixel(win.palette[index] & gray_mask, mask);
    framebuffer[state.scanline as usize * VISIBLE_PIXELS + (dot - 1) as usize] = out;
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::state::DOTS_PER_SCANLINE;

    /// Render one full visible scanline through the real per-dot
    /// pipeline: a priming pass primes the shift registers with the
    /// tiles the previous line's preload groups would have left behind
    /// (the pipeline is a pipeline — starting at dot 0 with empty
    /// registers would show two tiles of garbage), then the render pass
    /// runs with `on_dot` fired before each dot's pipeline step so tests
    /// can inject mid-scanline register writes.
    pub(crate) fn run_line(
        state: &mut PpuState,
        bus: &mut crate::bus::FlatBus,
        win: &RenderWindows<'_>,
        sl: i16,
        on_dot: &mut dyn FnMut(&mut PpuState, u16),
    ) -> [u8; 256 * 256] {
        state.ppudead = 0;
        state.scanline = sl;
        state.dot = 0;
        let (t0, fine0) = (state.registers.t, state.registers.fine_x);

        let mut scratch = [0u8; 256 * 256];
        for _ in 0..DOTS_PER_SCANLINE {
            tick_dot(state, bus, win, &mut scratch);
            crate::frame::tick_dot(state, bus);
        }

        // Resume at dot 0 of the requested line with the state the
        // previous line's preload window would have left behind: the
        // shift registers hold the line's first two tiles, and v carries
        // t's horizontal bits plus the two preload coarse-X increments
        // (so the first fetched group is tile 3 at t.coarseX + 2). With
        // rendering disabled there is no preload, so v is left alone
        // (the backdrop-override tests depend on that).
        if state.rendering_enabled() {
            state.registers.v = t0;
            state.registers.increment_coarse_x();
            state.registers.increment_coarse_x();
        }
        state.registers.fine_x = fine0;
        state.scanline = sl;
        state.dot = 0;
        state.sprite0_hit = false;

        let mut fb = [0u8; 256 * 256];
        for _ in 0..DOTS_PER_SCANLINE {
            on_dot(state, state.dot);
            tick_dot(state, bus, win, &mut fb);
            crate::frame::tick_dot(state, bus);
        }
        fb
    }

    /// Row `sl` of a rendered framebuffer.
    pub(crate) fn row(fb: &[u8; 256 * 256], sl: i16) -> [u8; 256] {
        let mut out = [0u8; 256];
        let off = sl as usize * 256;
        out.copy_from_slice(&fb[off..off + 256]);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{row, run_line};
    use super::*;
    use crate::bus::FlatBus;
    use crate::state::PpuState;

    #[test]
    fn map_nametable_horizontal() {
        assert_eq!(map_nametable_addr(0x2000, 0), 0x0000);
        assert_eq!(map_nametable_addr(0x2400, 0), 0x0400);
        assert_eq!(map_nametable_addr(0x2800, 0), 0x0000);
        assert_eq!(map_nametable_addr(0x2C00, 0), 0x0400);
    }

    #[test]
    fn map_nametable_vertical() {
        assert_eq!(map_nametable_addr(0x2000, 1), 0x0000);
        assert_eq!(map_nametable_addr(0x2400, 1), 0x0000);
        assert_eq!(map_nametable_addr(0x2800, 1), 0x0400);
        assert_eq!(map_nametable_addr(0x2C00, 1), 0x0400);
    }

    #[test]
    fn map_nametable_four_screen() {
        assert_eq!(map_nametable_addr(0x2000, 4), 0x0000);
        assert_eq!(map_nametable_addr(0x2400, 4), 0x0400);
        assert_eq!(map_nametable_addr(0x2800, 4), 0x0800);
        assert_eq!(map_nametable_addr(0x2C00, 4), 0x0C00);
    }

    #[test]
    fn palette_adjust_pixel_matches_cpp() {
        // C++ PaletteAdjustPixel (src/ppu_rendering.cpp:1524-1531):
        // no emphasis → (color & 0x3F) | 0x80, any emphasis but not
        // all three → color | 0x40, all three ($2001>>5==0x7) →
        // (color & 0x3F) | 0xC0.
        assert_eq!(palette_adjust_pixel(0x16, 0x00), 0x96);
        assert_eq!(palette_adjust_pixel(0x30, 0x01), 0xB0); // grayscale set, no emphasis
        let any_emph = 0x20; // red only
        assert_eq!(palette_adjust_pixel(0x16, any_emph), 0x56);
        let two_emph = 0xA0; // green+blue
        assert_eq!(palette_adjust_pixel(0x3F, two_emph), 0x7F);
        let all_emph = 0xE0;
        assert_eq!(palette_adjust_pixel(0xFF, all_emph), 0xFF);
    }

    #[test]
    fn attribute_quadrant_layout() {
        assert_eq!(attribute_quadrant(0xCC, 0, 0), 0); // top-left
        assert_eq!(attribute_quadrant(0xCC, 2, 0), 3); // top-right
        assert_eq!(attribute_quadrant(0xCC, 0, 4), 0); // bottom-left
        assert_eq!(attribute_quadrant(0xCC, 2, 4), 3); // bottom-right
    }

    #[test]
    fn attribute_addr_matches_loopy_layout() {
        // v = $2000 (coarse_x 0, coarse_y 0) → $23C0.
        assert_eq!(attribute_addr(0x2000), 0x23C0);
        // coarse_x = 4, coarse_y = 0 → $23C1.
        assert_eq!(attribute_addr(0x2004), 0x23C1);
        // coarse_y = 4 → $23C8.
        assert_eq!(attribute_addr(0x2080), 0x23C8);
    }

    /// A solid tile (both planes $FF) with a uniform attribute fills the
    /// row with the same palette colour — the pipeline's basic sanity.
    #[test]
    fn pipeline_renders_solid_row() {
        let mut state = PpuState::new();
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF;
        }
        let nt = [0u8; 4096];
        let palette = [0x16u8; 32];

        state.registers.write_ctrl(0);
        state.registers.write_mask(1 << mask_bits::SHOW_BG);
        state.registers.v = 0x2000;
        state.registers.t = 0x2000;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);

        // Pattern $FF → colour 3, attribute 0 → palette[3] = 0x16 →
        // PaletteAdjustPixel → 0x96.
        for x in 0..256 {
            assert_eq!(r[x], 0x96, "pixel {x}");
        }
    }

    /// Each 8-pixel string shares one attribute. With `attr = 0xE4`
    /// (quadrants 0,1,2,3 across the 4x4 region) and a solid tile, the
    /// first eight tiles render as two tiles per quadrant — with no
    /// intra-tile attribute change (the FCEUX `atlatch` half-tile delay
    /// is gone).
    #[test]
    fn pipeline_attribute_is_uniform_per_tile() {
        let mut state = PpuState::new();
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF;
        }
        let mut nt = [0u8; 4096];
        nt[0x3C0] = 0xE4;
        nt[0x3C1] = 0xE4;
        let mut palette = [0u8; 32];
        for i in 0..16 {
            palette[i] = i as u8;
        }

        state.registers.write_ctrl(0);
        state.registers.write_mask(1 << mask_bits::SHOW_BG);
        state.registers.v = 0x2000;
        state.registers.t = 0x2000;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);

        // Palette entries 0..15 are their own values, so the rendered
        // byte is (quadrant * 4 + 3) | 0x80. The pipeline is two tiles
        // deep: pixels 0..15 are the preloaded tiles 1/2 (coarse X 0/1),
        // then one fetched tile per 8-pixel string starting at coarse
        // X 2. Each attribute byte covers four tiles, so quadrants come
        // in pairs: 0, 0, 1, 1 across the first byte and 0, 0, 1, 1
        // again across the second.
        let expected = [0x83u8, 0x83, 0x87, 0x87, 0x83, 0x83, 0x87, 0x87];
        for tile in 0..8 {
            for px in 0..8 {
                let x = tile * 8 + px;
                assert_eq!(
                    r[x], expected[tile],
                    "tile {tile} pixel {px} must use its own quadrant uniformly"
                );
            }
        }
    }

    /// A mid-scanline `$2006` write re-points `v`; the fetches after the
    /// write use the new address, the tiles already fetched keep their
    /// pixels. This is the split-screen mechanism of `$2005`/`$2006`
    /// scroll writes inside a scanline.
    #[test]
    fn mid_scanline_v_write_only_affects_later_tiles() {
        let mut state = PpuState::new();
        // Tile 0 = opaque colour 3, tile 1 = transparent (colour 0).
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF;
        }
        // Nametable: entry 6 = tile 1 (transparent), everything else
        // tile 0 (opaque).
        let mut nt = [0u8; 4096];
        nt[6] = 0x01;
        let mut palette = [0u8; 32];
        palette[0] = 0x0Fu8; // backdrop, distinguishable
        palette[3] = 0x16;

        state.registers.write_ctrl(0);
        state.registers.write_mask(1 << mask_bits::SHOW_BG);
        state.registers.v = 0x2000;
        state.registers.t = 0x2000;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        // Without a write, the group that runs at dots 17-24 fetches
        // coarse X 4 (nametable entry 4 = tile 0) and lands at pixels
        // 32..40. Pointing v at entry 6 there makes exactly that group
        // fetch the transparent tile.
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |s, dot| {
            if dot == 17 {
                s.registers.v = 0x2006;
            }
        });
        let r = row(&fb, 0);

        let backdrop = palette_adjust_pixel(0x0F, 0x08);
        for x in 0..32 {
            assert_eq!(r[x], 0x96, "pixel {x} belongs to pre-write tiles");
        }
        for x in 32..40 {
            assert_eq!(r[x], backdrop, "pixel {x} comes from the re-pointed group");
        }
        for x in 40..48 {
            assert_eq!(r[x], 0x96, "pixel {x} follows the incremented coarse X");
        }
    }

    /// A CHR bank switch inside the scanline is visible to the next fetch
    /// group (in the bridge this is the 8-dot window poll).
    #[test]
    fn mid_scanline_chr_change_only_affects_later_fetches() {
        let mut state = PpuState::new();
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF; // bank A: tile 0 opaque
        }
        let chr_b = [0u8; 8192]; // bank B: tile 0 transparent
        let nt = [0u8; 4096];
        let mut palette = [0u8; 32];
        palette[0] = 0x0F;
        palette[3] = 0x16;

        state.registers.write_ctrl(0);
        state.registers.write_mask(1 << mask_bits::SHOW_BG);
        state.registers.v = 0x2000;
        state.registers.t = 0x2000;

        let render = |state: &mut PpuState, bus: &mut FlatBus, switched: bool| {
            let mut fb = [0u8; 256 * 256];
            for _ in 0..crate::state::DOTS_PER_SCANLINE {
                let bank_b = switched && state.dot >= 17;
                let win = RenderWindows {
                    nt: &nt,
                    chr: if bank_b { &chr_b } else { &chr },
                    palette: &palette,
                    mirror: 0,
                };
                tick_dot(state, bus, &win, &mut fb);
                crate::frame::tick_dot(state, bus);
            }
            fb
        };

        state.ppudead = 0;
        state.scanline = 0;
        state.dot = 0;
        let t0 = state.registers.t;
        let mut bus = FlatBus::new();
        // Priming pass (no switch) fills the shift registers.
        let _ = render(&mut state, &mut bus, false);

        state.registers.v = t0;
        state.registers.increment_coarse_x();
        state.registers.increment_coarse_x();
        state.scanline = 0;
        state.dot = 0;
        let fb = render(&mut state, &mut bus, true);
        let r = row(&fb, 0);

        let backdrop = palette_adjust_pixel(0x0F, 0x08);
        for x in 0..32 {
            assert_eq!(r[x], 0x96, "pixel {x} was fetched before the switch");
        }
        for x in 32..40 {
            assert_eq!(r[x], backdrop, "pixel {x} was fetched after the switch");
        }
    }

    /// Fine X scroll moves the pixel/attribute pairing, not just the
    /// pattern bits: with `fine_x = 4` the first four pixels of a group
    /// come from the previous tile and must take ITS attribute. The
    /// attribute shift registers provide that (a per-tile attribute
    /// latch would paint all eight pixels with the new tile's palette).
    #[test]
    fn fine_x_pairs_pixels_with_the_attribute_they_were_fetched_with() {
        let mut state = PpuState::new();
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF; // solid tile
        }
        // Attribute byte at $23C0: quadrant 0 for coarse X 0-1,
        // quadrant 1 for coarse X 2-3. Only the regular palette entries
        // matter here (no sprites).
        let mut nt = [0u8; 4096];
        nt[0x3C0] = 0xE4;
        let mut palette = [0u8; 32];
        for i in 0..16 {
            palette[i] = i as u8;
        }

        state.registers.write_ctrl(0);
        state.registers.write_mask(1 << mask_bits::SHOW_BG);
        // t.coarseX = 1: the preload fetches coarse X 1 (quadrant 0) and
        // coarse X 2 (quadrant 1).
        state.registers.t = 0x2001;
        state.registers.v = 0x2001;
        state.registers.fine_x = 4;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);

        // Pixels 0..3 are the tail of the coarse-X-1 tile (quadrant 0 →
        // 0x83); pixels 4..8 are the head of the coarse-X-2 tile
        // (quadrant 1 → 0x87).
        for x in 0..4 {
            assert_eq!(r[x], 0x83, "pixel {x} belongs to the previous tile");
        }
        for x in 4..8 {
            assert_eq!(r[x], 0x87, "pixel {x} belongs to the next tile");
        }
    }

    #[test]
    fn rendering_off_fills_backdrop() {
        let mut state = PpuState::new();
        let chr = [0u8; 8192];
        let nt = [0u8; 4096];
        let palette = [0u8; 32];

        state.registers.write_mask(0);
        state.registers.v = 0x2000;
        state.registers.t = 0x2000;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);
        for x in 0..256 {
            assert_eq!(r[x], 0x80, "backdrop fill pixel {x}");
        }
    }

    /// Rendering off with `v` inside $3F00-$3FFF draws the palette entry
    /// `v` points at (the backdrop override used by SMB3 and others).
    #[test]
    fn rendering_off_with_v_in_palette_draws_that_entry() {
        let mut state = PpuState::new();
        let chr = [0u8; 8192];
        let nt = [0u8; 4096];
        let mut palette = [0u8; 32];
        palette[0x15] = 0x2A;

        state.registers.write_mask(0);
        state.registers.v = 0x3F15;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);
        assert_eq!(r[0], palette_adjust_pixel(0x2A, 0));
        assert_eq!(r[255], palette_adjust_pixel(0x2A, 0));
    }

    #[test]
    fn grayscale_masks_palette() {
        let mut state = PpuState::new();
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF;
        }
        let nt = [0u8; 4096];
        let mut palette = [0u8; 32];
        palette[3] = 0x3F;

        state.registers.write_ctrl(0);
        state
            .registers
            .write_mask(1 << mask_bits::SHOW_BG | 1 << mask_bits::GRAYSCALE);
        state.registers.v = 0x2000;
        state.registers.t = 0x2000;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);
        for x in 0..256 {
            assert_eq!(r[x], 0xB0, "grayscale masked pixel {x}");
        }
    }

    #[test]
    fn mirroring_horizontal_page_2_maps_to_page_0() {
        let mut state = PpuState::new();
        let mut chr = [0u8; 8192];
        for i in 0..16 {
            chr[i] = 0xFF;
        }
        let nt = [0u8; 4096];
        let mut palette = [0u8; 32];
        palette[3] = 0x10;

        state.registers.write_ctrl(0);
        state.registers.write_mask(1 << mask_bits::SHOW_BG);
        state.registers.v = 0x2800;
        state.registers.t = 0x2800;

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let fb = run_line(&mut state, &mut bus, &win, 0, &mut |_, _| {});
        let r = row(&fb, 0);
        for x in 0..256 {
            assert_eq!(r[x], 0x90, "page 2 mirrors page 0 pixel {x}");
        }
    }
}
