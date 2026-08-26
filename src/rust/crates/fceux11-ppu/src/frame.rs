//! `tick_dot` — the dot-level main state machine.
//!
//! Each call advances the PPU by exactly one PPU dot. The current scanline
//! and dot are consulted to determine which side effects fire this tick:
//! VBlank flag set/clear, NMI sampling, scroll increment on visible
//! scanlines, secondary-OAM reload at scanline start, even/odd skip on
//! the pre-render line.
//!
//! All register-level side effects (writes to `$2000`/`$2005`/`$2006`,
//! reads from `$2002`/`$2004`/`$2007`) live on [`crate::registers::Registers`].
//! They are *value* operations — the state machine does not call them
//! itself; it only sets/clears `status` bits based on the timing.
//!
//! Returns [`TickOutcome`] so the test harness (and Phase 3 scheduler)
//! can react to "NMI fired this tick" / "VBL flag transitioned this tick".
//!
//! Timing reference: <https://www.nesdev.org/wiki/PPU_frame_timing>.
//! The dot-level events below match what the C++ new PPU
//! (`src/ppu_rendering.cpp`) implements, modulo the Phase 1 simplifications
//! documented at each `match` arm.

use crate::bus::PpuBus;
#[allow(unused_imports)] // mask_bits / status_bits used only by #[cfg(test)] modules.
use crate::registers::{ctrl_bits, mask_bits, status_bits};
use crate::state::{DOTS_PER_SCANLINE, NTSC_SCANLINES, PpuState};

/// Result of a single `tick_dot` call.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TickOutcome {
    /// NMI asserted this tick (rising edge from PPU side, to be latched
    /// by the CPU in Phase 3).
    pub nmi_asserted: bool,
    /// VBL flag transitioned from 0 → 1 this tick (not set if suppressed).
    pub vbl_entered: bool,
    /// Scanline counter changed this tick.
    pub scanline_changed: bool,
    /// Dot counter wrapped 340 → 0 this tick.
    pub dot_wrapped: bool,
    /// Frame counter incremented this tick (sl 0 dot 0 → first dot of
    /// a new visible frame).
    pub frame_advanced: bool,
    /// Sprite 0 hit was set this tick (Phase 4 will gate it on dot;
    /// Phase 1 reports whenever eval sets it).
    pub sprite0_hit_now: bool,
    /// True if this tick is at the boundary into sl -1 dot 0 — the
    /// pre-render line where even/odd skip is decided.
    pub pre_render_decision: bool,
}

/// Advance the PPU state by exactly one PPU dot.
///
/// Events fired at the *current* (sl, dot) position run before the dot
/// counter advances. This matches the real hardware: as the PPU enters
/// a given dot, the events for that dot are processed (e.g. sl 241 dot 1
/// set the VBL flag as we enter dot 1).
pub fn tick_dot<B: PpuBus + ?Sized>(state: &mut PpuState, _bus: &mut B) -> TickOutcome {
    let mut out = TickOutcome::default();
    let sl = state.scanline;
    let dot = state.dot;

    // -----------------------------------------------------------------
    // Events that fire as we enter (sl, dot)
    // -----------------------------------------------------------------

    // Pre-render scanline: even/odd skip decision at dot 340. We model
    // it as: when we *enter* sl -1 dot 340, look at odd_frame and the
    // rendering mask to decide whether the next scanline should be
    // skipped (i.e. sl becomes 0 at dot 1 instead of dot 0).
    //
    // Per nesdev: when rendering is OFF, odd_frame is frozen and does
    // NOT toggle on the pre-render line.
    let mut skip_one_dot = false;
    if sl == -1 && dot == 340 {
        out.pre_render_decision = true;
        if state.rendering_enabled() {
            if !state.odd_frame {
                // Even frame + rendering: skip one dot of the next
                // scanline. We'll advance twice below so (0, 0) never
                // fires.
                skip_one_dot = true;
                state.odd_frame = true;
            } else {
                // Odd frame + rendering: no skip; just toggle for next.
                state.odd_frame = false;
            }
        }
        // Rendering off: odd_frame frozen.
    }

    // Visible scanline starts: sprite eval (sl 0..239 at dot 0).
    // Real hardware evaluates sprites during dots 0..=63 of each visible
    // line; we collapse it into dot 0 for the Phase 1 minimal model.
    if (0..=239).contains(&sl) && dot == 0 {
        let sprite_height = if state.registers.ctrl & (1 << ctrl_bits::SPRITE_SIZE) != 0 {
            16
        } else {
            8
        };
        state.eval_sprites(sprite_height);
        // Reset scroll for the start of the visible line: copy t → v (h bits).
        state.registers.copy_horizontal();
    }

    // Visible scanline scroll increment: BG fetch clock at dot 256,
    // vertical copy at dot 257, fine_y increment at dot 256.
    if (0..=239).contains(&sl) && state.rendering_enabled() {
        if dot == 256 {
            state.registers.increment_coarse_x();
            // Phase 4: increment fine_y (and coarse_y / nametable_y
            // on roll-over) at the end of each visible scanline so
            // the next render_scanline call uses the next row of
            // pattern data. Mirrors the C++ new PPU's
            // `increment_fine_y` at the end of RefreshLine.
            state.registers.increment_fine_y();
        }
        if dot == 257 {
            state.registers.copy_vertical();
        }
    }

    // sl 241 dot 1: VBL flag set + NMI check. This is the central
    // timing event Phase 1 exists to model.
    if sl == 241 && dot == 1 {
        if state.vbl_suppressed_this_frame {
            // Suppression flag from the sl 241 dot 0 $2002 read:
            // do not set the flag, do not assert NMI.
            out.vbl_entered = false;
            out.nmi_asserted = false;
        } else {
            state.registers.set_vbl_flag();
            out.vbl_entered = true;
            if state.nmi_enabled() {
                out.nmi_asserted = true;
            }
        }
    }

    // Post-render (sl 261 dot 1 for NTSC): VBL flag clear.
    if sl == 261 && dot == 1 {
        state.registers.clear_vbl_flag();
    }

    // Frame boundary: sl 261 dot 340 rolls into sl -1 dot 0; we
    // also reset the suppression flag here for the *next* frame.
    if sl == 261 && dot == 340 {
        state.vbl_suppressed_this_frame = false;
    }

    // Pre-render line: at dot 280, copy t's vertical bits to v so
    // the first visible scanline starts with the right scroll
    // position. The C++ ppu_rendering.cpp::DoLine calls
    // `copy_vertical` at sl -1 dot 280.
    if sl == -1 && dot == 280 && state.rendering_enabled() {
        state.registers.copy_vertical();
    }

    // Sprite 0 hit latched by eval: copy the latched flag onto PPU[2]
    // bit 6. Phase 1 just mirrors `state.sprite0_hit` onto `status`.
    if state.sprite0_hit {
        state.registers.set_sprite0_hit();
        out.sprite0_hit_now = true;
    } else {
        state.registers.clear_sprite0_hit();
    }

    // -----------------------------------------------------------------
    // Advance (sl, dot) by one. For the even-frame skip path we advance
    // twice so the (0, 0) tick is skipped entirely (per nesdev).
    // -----------------------------------------------------------------
    if skip_one_dot {
        advance(state, &mut out); // (-1, 340) → (0, 0)
        advance(state, &mut out); // (0, 0) → (0, 1)
    } else {
        advance(state, &mut out);
    }

    out
}

/// Move (sl, dot) forward by one PPU dot. Called at the end of
/// [`tick_dot`].
fn advance(state: &mut PpuState, out: &mut TickOutcome) {
    let next_dot = state.dot + 1;
    if next_dot >= DOTS_PER_SCANLINE {
        // Wrap within scanline: dot 340 → 0 of next scanline.
        out.dot_wrapped = true;
        state.dot = 0;
        let next_sl = state.scanline + 1;
        if next_sl >= NTSC_SCANLINES {
            // Wrap frame: sl 261 → sl -1.
            state.scanline = -1;
            out.frame_advanced = true;
        } else {
            state.scanline = next_sl;
        }
        out.scanline_changed = true;
    } else {
        state.dot = next_dot;
    }
}

/// Tick until reaching `(target_sl, target_dot)`, then fire the event at
/// that position. The returned [`TickOutcome`] is the outcome of the
/// `(target_sl, target_dot)` tick; the state after the call is
/// `(target_sl, target_dot + 1)` (or the next scanline if `target_dot`
/// was the last dot in the scanline).
///
/// If the current position is already at the target, exactly one tick
/// fires (the event-at-target tick).
///
/// Panics in debug builds if the loop fails to converge — typically
/// because the requested target is unreachable from the current state
/// (e.g. the wrap boundary ate it).
pub fn tick_to<B: PpuBus + ?Sized>(
    state: &mut PpuState,
    bus: &mut B,
    target_sl: i16,
    target_dot: u16,
) -> TickOutcome {
    let mut guard = 0u32;
    let max_ticks = (NTSC_SCANLINES as u32 + 2) * DOTS_PER_SCANLINE as u32;
    while (state.scanline, state.dot) != (target_sl, target_dot) {
        tick_dot(state, bus);
        guard += 1;
        assert!(
            guard < max_ticks,
            "tick_to({target_sl}, {target_dot}) didn't converge (stuck at sl={} dot={})",
            state.scanline,
            state.dot
        );
    }
    // Fire the event at the target position.
    tick_dot(state, bus)
}

/// Tick `n` times, returning each outcome in order.
pub fn tick_n<B: PpuBus + ?Sized>(state: &mut PpuState, bus: &mut B, n: u32) -> Vec<TickOutcome> {
    (0..n).map(|_| tick_dot(state, bus)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::FlatBus;

    #[test]
    fn tick_advances_dot_within_scanline() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        assert_eq!((s.scanline, s.dot), (-1, 0));
        tick_dot(&mut s, &mut bus);
        assert_eq!((s.scanline, s.dot), (-1, 1));
    }

    #[test]
    fn dot_wraps_into_next_scanline() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        // tick_to(-1, 340) fires the (-1, 340) tick and advances.
        let out = tick_to(&mut s, &mut bus, -1, 340);
        assert!(out.dot_wrapped);
        assert!(out.scanline_changed);
        assert_eq!((s.scanline, s.dot), (0, 0));
    }

    #[test]
    fn even_frame_skips_one_dot_after_pre_render() {
        // odd_frame starts false (even); with rendering on, the
        // pre-render dot 340 → sl 0 dot 1 (skip).
        let mut s = PpuState::new();
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        let mut bus = FlatBus::new();
        let out = tick_to(&mut s, &mut bus, -1, 340);
        assert!(out.pre_render_decision);
        // After the skip, state should be at (0, 1), not (0, 0).
        assert_eq!(s.scanline, 0);
        assert_eq!(s.dot, 1);
        assert!(s.odd_frame, "odd_frame toggled to true after even skip");
    }

    #[test]
    fn odd_frame_runs_full_pre_render() {
        // odd_frame starts true → no skip; advance goes (-1, 340) → (0, 0).
        let mut s = PpuState::new();
        s.odd_frame = true;
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        let mut bus = FlatBus::new();
        let out = tick_to(&mut s, &mut bus, -1, 340);
        assert!(out.pre_render_decision);
        assert_eq!((s.scanline, s.dot), (0, 0));
        assert!(!s.odd_frame, "odd_frame toggled back to false on odd frame");
    }

    #[test]
    fn rendering_off_skips_even_odd_logic() {
        let mut s = PpuState::new();
        // mask stays 0; rendering off; odd_frame should NOT toggle.
        s.odd_frame = false;
        let mut bus = FlatBus::new();
        let _ = tick_to(&mut s, &mut bus, -1, 340);
        // No skip: state lands at (0, 0) (the (-1, 340) tick already
        // fired and advanced into sl 0).
        assert_eq!((s.scanline, s.dot), (0, 0));
        assert!(!s.odd_frame, "rendering off → odd_frame frozen");
    }

    #[test]
    fn vbl_flag_clears_at_sl_261_dot_1() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        // Manually set the VBL flag to prove the state machine clears it.
        s.registers.set_vbl_flag();
        tick_to(&mut s, &mut bus, 261, 1);
        assert_eq!(
            s.registers.status & (1 << status_bits::VBL),
            0,
            "VBL flag should be cleared at sl 261 dot 1"
        );
    }

    #[test]
    fn vbl_set_at_sl_241_dot_1_with_nmi_enable() {
        let mut s = PpuState::new();
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();
        tick_to(&mut s, &mut bus, 241, 1);
        assert_ne!(
            s.registers.status & (1 << status_bits::VBL),
            0,
            "VBL flag should be set at sl 241 dot 1"
        );
    }

    #[test]
    fn nmi_not_asserted_when_nmi_disabled() {
        let mut s = PpuState::new();
        s.registers.write_ctrl(0); // NMI off
        let mut bus = FlatBus::new();
        let out = tick_to(&mut s, &mut bus, 241, 1);
        assert!(!out.nmi_asserted);
    }

    #[test]
    fn nmi_asserted_at_sl_241_dot_1_when_enabled() {
        let mut s = PpuState::new();
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();
        let out = tick_to(&mut s, &mut bus, 241, 1);
        assert!(out.nmi_asserted, "NMI should fire on sl 241 dot 1 tick");
    }

    #[test]
    fn vbl_suppression_via_sl_241_dot0_read_blocks_set() {
        // The plan §6.6 suppression window: a $2002 read at sl 241 dot 0
        // suppresses the VBL flag set + NMI for this frame.
        let mut s = PpuState::new();
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();

        // Tick to sl 241 dot 0.
        tick_to(&mut s, &mut bus, 241, 0);
        // Read $2002 at sl 241 dot 0 — this is the suppressing read.
        let _ = s.registers.read_status();
        s.vbl_suppressed_this_frame = true;

        // Advance into sl 241 dot 1 — VBL should NOT be set, NMI should NOT fire.
        let out = tick_dot(&mut s, &mut bus);
        assert!(!out.vbl_entered);
        assert!(!out.nmi_asserted);
        assert_eq!(
            s.registers.status & (1 << status_bits::VBL),
            0,
            "VBL flag should NOT be set after suppression"
        );
    }

    #[test]
    fn suppression_resets_at_frame_boundary() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        s.vbl_suppressed_this_frame = true;
        // Tick to sl 261 dot 340 → suppression reset, then wrap to next frame.
        tick_to(&mut s, &mut bus, 261, 340);
        assert!(!s.vbl_suppressed_this_frame);
        assert_eq!(s.scanline, -1, "wrapped into next frame's pre-render");
        assert_eq!(s.dot, 0);
    }

    #[test]
    fn scroll_copy_horizontal_at_visible_scanline_start() {
        let mut s = PpuState::new();
        // Set t = some non-zero scroll value, leave v = 0.
        s.registers.t = 0x1234;
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        let mut bus = FlatBus::new();
        // Tick to sl 0 dot 0 — copy_horizontal should run.
        tick_to(&mut s, &mut bus, 0, 0);
        assert_eq!(s.registers.v & 0x041F, 0x1234 & 0x041F);
    }

    #[test]
    fn coarse_x_increments_at_dot_256_during_render() {
        let mut s = PpuState::new();
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        s.registers.v = 0x1000; // coarse_x = 0
        let mut bus = FlatBus::new();
        tick_to(&mut s, &mut bus, 100, 256);
        tick_dot(&mut s, &mut bus); // dot 256 → increment
        assert_eq!(s.registers.v & 0x001F, 1);
    }
}
