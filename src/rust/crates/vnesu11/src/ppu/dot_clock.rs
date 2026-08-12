//! PPU dot clock — 341×262 frame timing (NTSC).
//!
//! The 2C02 PPU runs at 5.369318 MHz, with one CPU cycle = 3 PPU dots.
//! Each frame is **341 dots × 262 scanlines**:
//!
//! - scanline **-1** (pre-render): NMI clear + v/t/w reset
//! - scanlines **0..=239**: visible (256 visible dots + 85 hblank)
//! - scanline **240**: post-render (idle)
//! - scanline **241**: VBlank set at dot 1 → NMI
//! - scanlines **242..=260**: VBlank idle
//! - scanline **261**: pre-render of next frame (odd-frame skips 1 dot)
//!
//! These constants come straight from `src/ppu_rendering.cpp:680-790`
//! (newppu DoLine) and `src/ppu.cpp` (FCEUPPU_Loop).  Magic numbers
//! are pinned with a "source:" comment so any drift gets caught at
//! review time.

/// Dots per scanline (NTSC PPU master clock / 4 = pixel clock, but the
/// PPU actually advances at full master clock which is 341 dots per
/// scanline).
///
/// Source: `src/ppu_rendering.cpp` (FCEUPPU_Loop, scanline 256 + 85 / 256
/// + 69 magic numbers → implies 341 dots).
pub const DOTS_PER_SCANLINE: u16 = 341;

/// Scanlines per frame (NTSC): pre-render (-1) + 240 visible + 1 post + 20
/// vblank = 262, but the count is -1..=261 inclusive (= 263 lines).
///
/// Source: `src/ppu.cpp::totalscanlines = 262`, FCEUX `scanlines_per_frame`.
pub const SCANLINES_PER_FRAME: i16 = 262;

/// Visible scanline range (inclusive).
pub const VISIBLE_SCANLINE_FIRST: i16 = 0;
pub const VISIBLE_SCANLINE_LAST: i16 = 239;

/// VBlank set scanline + dot. At scanline 241, dot 1, `PPU[2] |= 0x80`.
/// Source: `src/ppu.cpp` (FCEUPPU_Loop body, `if(sl == 241 && ...)`).
pub const VBLANK_SCANLINE: i16 = 241;
pub const VBLANK_SET_DOT: u16 = 1;

/// Pre-render scanline (negative index).
pub const PRELINE: i16 = -1;

/// CPU cycle budget constants (复刻 `ppu_rendering.cpp::DoLine`).
///
/// These are the cycle budgets that the new PPU asks `X6502_Run` to
/// consume before yielding control back to the PPU for next-segment
/// work.  Magic numbers must NOT be re-derived — they encode specific
/// timing assumptions that match real-hardware behavior on the budget
/// model.
///
/// Source: `src/ppu_rendering.cpp:690-790`.
pub const CPU_BUDGET_VISIBLE: i32 = 256;
pub const CPU_BUDGET_SPRITE_EVAL: i32 = 69;
pub const CPU_BUDGET_BOUNDARY_FIX: i32 = 16;
pub const CPU_BUDGET_GB_HBLANK: i32 = 85;
/// Per-scanline total for post-render (sl=240) and VBlank-set (sl=241)
/// lines: matches C++ DoLine branch `X6502_Run(256+69) + X6502_Run(16)`
/// (scanline 240 / 241 / every VBlank line share this total).
///
/// **Phase 6 P2 shadow fix (2026-08-12)**: scanline 241 was previously
/// budgeted at 1 cycle, which broke the NMI handler entry sequence
/// (needs 7 cycles). C++ allocates 341 here so the NMI push + vector
/// completes cleanly within the scanline.
pub const CPU_BUDGET_VBLANK_LINE: i32 = 341;
/// Total hblank budget following a visible scanline: 6 (start HBLANK)
/// + 63 (main hblank) + 16 (final sprite eval slot) = 85. Matches
/// `ppu_rendering.cpp::DoLine` visible-scanline epilogue.
///
/// **Phase 6 P2 shadow fix (2026-08-12)**: previously the visible
/// scanline emitted only `CPU_BUDGET_VISIBLE = 256`, leaving the CPU
/// 85 cycles short per scanline. Over 240 scanlines that was a
/// ~20k-cycle deficit per frame and the shadow PC drifted.
pub const CPU_BUDGET_HBLANK_TOTAL: i32 = 85;

/// Sprites per scanline (hardware limit; 9th sprite sets overflow flag).
pub const MAX_SPRITES_PER_LINE: usize = 8;

/// Sprite 0 hit x-clipping. Hits before x=8 are masked (left 8 pixels).
/// Source: NESdev wiki "PPU sprite evaluation".
pub const SPRITE_ZERO_HIT_X_MIN: u8 = 8;

/// Iterate over all visible scanline x positions (0..=255).
#[inline(always)]
pub fn for_each_visible_x<F: FnMut(u8)>(mut f: F) {
    let mut x: u8 = 0;
    while x < 255 {
        f(x);
        x = x.wrapping_add(1);
    }
    f(255);
}

/// Compute dot index within a scanline, given absolute dot counter.
#[inline(always)]
pub fn dot_in_scanline(abs_dot: u32) -> u16 {
    (abs_dot % DOTS_PER_SCANLINE as u32) as u16
}

/// Convert `(scanline, dot)` to absolute dot counter since frame start.
///
/// Used for sprite 0 hit timing debug.
#[inline]
pub fn abs_dot(scanline: i16, dot: u16) -> u32 {
    ((scanline + 1) as u32) * DOTS_PER_SCANLINE as u32 + dot as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_layout_constants_match_newppu() {
        // Frame is 262 scanlines * 341 dots = 89,282 dots per frame.
        assert_eq!(DOTS_PER_SCANLINE, 341);
        assert_eq!(SCANLINES_PER_FRAME, 262);
        // Visible: 240 lines.
        assert_eq!(VISIBLE_SCANLINE_LAST - VISIBLE_SCANLINE_FIRST + 1, 240);
        // VBlank set at (241, 1) per `src/ppu.cpp::FCEUPPU_Loop`.
        assert_eq!(VBLANK_SCANLINE, 241);
        assert_eq!(VBLANK_SET_DOT, 1);
        // Pre-render scanline is negative.
        assert_eq!(PRELINE, -1);
    }

    #[test]
    fn cpu_budget_constants_match_pp_rendering() {
        // These MUST stay byte-exact with the magic numbers in
        // ppu_rendering.cpp:DoLine.
        assert_eq!(CPU_BUDGET_VISIBLE, 256);
        assert_eq!(CPU_BUDGET_SPRITE_EVAL, 69);
        assert_eq!(CPU_BUDGET_BOUNDARY_FIX, 16);
        assert_eq!(CPU_BUDGET_GB_HBLANK, 85);
    }

    #[test]
    fn abs_dot_wraps_correctly() {
        // At frame start (scanline = -1, dot = 0), abs_dot = 0.
        assert_eq!(abs_dot(-1, 0), 0);
        // End of pre-render scanline (scanline = -1, dot = 340) = 340.
        assert_eq!(abs_dot(-1, 340), 340);
        // End of scanline 0 (scanline = 0, dot = 340) = 681.
        assert_eq!(abs_dot(0, 340), 681);
        // End of frame (scanline = 261, dot = 340) = (261+1) * 341 + 340 = 89682.
        assert_eq!(abs_dot(261, 340), (261 + 1) * 341 + 340);
    }
}