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

/// Plan §0.8 step 1C: env-gated PPU phase trace. When
/// `FCEUX11_PPU_PHASE_TRACE=1`, the three timing-sensitive events
/// (VBL flag set, NMI assert, pre-render VBL clear) emit a single
/// `eprintln!` line. The companion bridge-side trace at
/// `ppu_rust_bridge.cpp:646-650` already covers the $2002 read path;
/// together they let us replay the Rust PPU's cycle-level events
/// against the C++ engine's E1 P2002_READ stream for diffing.
///
/// The check is cached in an `AtomicU8` (one-shot env read per
/// process) so the per-dot fast path stays a single load+compare.
fn phase_trace_on() -> bool {
    use std::sync::atomic::{AtomicU8, Ordering};
    static CACHE: AtomicU8 = AtomicU8::new(2); // 0=off, 1=on, 2=uninit
    match CACHE.load(Ordering::Relaxed) {
        0 => false,
        1 => true,
        _ => {
            let on = std::env::var("FCEUX11_PPU_PHASE_TRACE")
                .map(|v| v == "1")
                .unwrap_or(false);
            CACHE.store(if on { 1 } else { 0 }, Ordering::Relaxed);
            on
        }
    }
}

#[allow(unused_macros)]
macro_rules! phase_trace {
    ($($arg:tt)*) => {
        if $crate::frame::phase_trace_on() {
            eprintln!($($arg)*);
        }
    };
}

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
    //
    // Phase 6.4: gate eval on rendering_enabled() — the C++ engine only
    // runs sprite evaluation inside its rendering path, so with $2001
    // rendering bits clear (incl. every boot frame before the game
    // enables rendering) the overflow flag must never set. The Rust
    // model evaluated unconditionally: with OAM all-zero at boot, every
    // visible scanline 0..=7 saw 64 in-range sprites and latched
    // status bit 5, so ALL $2002 reads returned 0x20 instead of 0x00 —
    // the root divergence behind blargg ppu_open_bus (Failed #2) /
    // ppu_read_buffer (value=0x80), A/B verified via trace-diff
    // (§6.3.a.4 follow-up; first divergence = access #3, first $2002
    // read after power). The vbl_nmi ROMs kept passing only because
    // their wait loops branch on bit 7 alone.
    if (0..=239).contains(&sl) && dot == 0 && state.rendering_enabled() {
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
    //
    // Phase 6.1.e.v3 (2026-08-30): reverted an earlier dot-0
    // experiment (commit 946ee31). The Mesen reference + the C++
    // engine (ppu_rendering.cpp:1671-1672 "Working config: VBL at
    // cycle 0" comment) reference a different convention; blargg
    // ppu_vbl_nmi 02-vbl_set_time is calibrated to the dot-1 set
    // point that Mesen, fceux original, and Nestopia all use. A
    // $2002 read at sl 241 dot 0 — 1 PPU dot before the set — marks
    // `vbl_suppressed_this_frame`; the (241, 1) tick checks the
    // flag and skips the VBL set + NMI assert. Reads at (241, 1)
    // (handled by the bridge via `take_nmi_pending`) consume an
    // already-set flag's NMI — that path is orthogonal to this block.
    //
    // Golden baseline (commit b06388c^, pre-6.1.e): nestest frames
    // 3-7 + savestate hash kept at the dot-1 timing values.
    // (See `docs/history/v2.1_phase6_batch_compat.md` §6.1.e.v3.)
    if sl == 241 && dot == 1 && state.ppudead == 0 {
        if state.vbl_suppressed_this_frame {
            // Suppression flag from the (sl 240, dot 340) $2002 read
            // (NESdev PPU frame timing: read 1 PPU clock before the
            // VBL set dot suppresses VBL+NMI for the entire frame).
            out.vbl_entered = false;
            out.nmi_asserted = false;
            phase_trace!(
                "R3 PPU_VBL_SET sl=241 dot=1 suppressed=1 reason=early_read"
            );
        } else {
            state.registers.set_vbl_flag();
            out.vbl_entered = true;
            if state.nmi_enabled() {
                out.nmi_asserted = true;
                phase_trace!(
                    "R3 PPU_NMI_ASSERT sl=241 dot=1 vbl_set=1 nmi_enabled=1"
                );
            } else {
                phase_trace!(
                    "R3 PPU_VBL_SET sl=241 dot=1 suppressed=0 vbl_set=1 nmi_enabled=0"
                );
            }
        }
    }

    // Post-render: VBL flag clears at the start of the pre-render line
    // (sl -1, dot 1 for NTSC — hardware scanline 261). Phase 5.1: this
    // used to live at sl 261, but the frame also STARTS at sl -1, so
    // the pre-render line was visited twice per frame (263 scanlines =
    // 89683 dots) while the CPU budget is 89342 dots (262 lines) — a
    // one-scanline phase drift per frame that broke the savestate /
    // nestest gates. The wrap now goes 240 → 241 directly (VBL-first
    // layout, Phase 6.1.e follow-up).
    //
    // Phase 6.1.e follow-up: in VBL-first layout, the ppudead path
    // sets VBL at (sl 241, dot 0) (1 dot earlier than normal); the
    // natural (sl -1, dot 1) clear fires 20 scanlines + 1 dot later,
    // matching the observable VBL-window semantic the C++ engine
    // approximates with its own `PPU_status = 0` mid-frame clear.
    if sl == -1 && dot == 1 {
        state.registers.clear_vbl_flag();
        // Phase 6.6 (Session A): hardware clears sprite 0 hit and
        // sprite overflow at dot 1 of the pre-render line (nesdev
        // PPU frame timing). The Rust model previously never cleared
        // either flag, so a single hit/overflow latched the status
        // bit for the rest of the run — games polling "wait for hit
        // clear, then wait for set" saw stale values.
        state.sprite0_hit = false;
        state.sprite0_hit_dot = crate::state::NO_SPRITE0_HIT_DOT;
        state.sprite_overflow = false;
        phase_trace!("R3 PPU_VBL_CLEAR sl=-1 dot=1");
    }

    // Phase 6.4: ppudead — the process's FIRST frame mirrors the C++
    // new-PPU layout (ppu_rendering.cpp:1626-1655): VBL flag set at
    // frame dot 0 (`PPU_status |= 0x80` before any runppu), VBL window
    // = the first 20 scanlines (`runppu(20*kLineTime)` then
    // `PPU_status = 0`), and the normal sl-241 set / sl-1 clear are
    // skipped for this frame. The C++ decrements ppudead after the
    // frame's full 262 lines.
    //
    // Phase 6.1.e follow-up (VBL-block-phase alignment, §6.4.3): the
    // frame is VBL-first, so the frame start sits at (sl 241, dot 0)
    // instead of (sl -1, dot 0). The VBL flag set moves to (sl 241,
    // dot 0) so it lands at frame dot 0 (matching C++ ppudead start);
    // the natural sl -1 dot 1 VBL clear (already present for normal
    // frames) fires 20 scanlines + 1 dot later — same observable
    // 6820-dot VBL window as C++ ppudead. The decrement moves to the
    // new frame-end (sl 240, dot 340).
    if state.ppudead > 0 {
        if sl == 241 && dot == 0 {
            state.registers.set_vbl_flag();
            out.vbl_entered = true;
            if state.nmi_enabled() {
                out.nmi_asserted = true;
                phase_trace!(
                    "R3 PPU_VBL_SET_PPUDEAD sl=241 dot=0 vbl_set=1 nmi_enabled=1"
                );
            } else {
                phase_trace!(
                    "R3 PPU_VBL_SET_PPUDEAD sl=241 dot=0 vbl_set=1 nmi_enabled=0"
                );
            }
        }
        if sl == 240 && dot == 340 {
            state.ppudead -= 1;
            phase_trace!("R3 PPU_PPUDEAD_DECR sl=240 dot=340 remaining={}", state.ppudead);
        }
    }

    // Frame boundary: the last dot of the pre-render line (-1, 340)
    // resets the suppression flag for the *next* frame (and decides
    // the even/odd skip above).
    if sl == -1 && dot == 340 {
        let was_suppressed = state.vbl_suppressed_this_frame;
        state.vbl_suppressed_this_frame = false;
        phase_trace!("R3 PPU_VBL_SUPPRESS_RESET sl=-1 dot=340 was={}", was_suppressed);
    }

    // Pre-render line: at dot 280, copy t's vertical bits to v so
    // the first visible scanline starts with the right scroll
    // position. The C++ ppu_rendering.cpp::DoLine calls
    // `copy_vertical` at sl -1 dot 280.
    if sl == -1 && dot == 280 && state.rendering_enabled() {
        state.registers.copy_vertical();
    }

    // Phase 6.6 (Session A): per-pixel sprite 0 hit timing. The batch
    // render records the dot (pixel x + 1) at which the first hit
    // pixel outputs on this scanline; latch the flag when the state
    // machine reaches it. Until then the mirror block below keeps
    // PPU[2] bit 6 clear, so a $2002 read mid-scanline only sees the
    // hit from the correct pixel onward.
    if (0..=239).contains(&sl)
        && state.sprite0_hit_dot != crate::state::NO_SPRITE0_HIT_DOT
        && dot >= state.sprite0_hit_dot
    {
        state.sprite0_hit = true;
        state.sprite0_hit_dot = crate::state::NO_SPRITE0_HIT_DOT;
    }

    // Sprite 0 hit + sprite overflow latched by eval: mirror the state
    // flags onto PPU[2] bits 6/5 every tick. With the pre-render clear
    // above this stays consistent with hardware: both flags clear once
    // per frame and re-set only where the pipeline sets them.
    if state.sprite0_hit {
        state.registers.set_sprite0_hit();
        out.sprite0_hit_now = true;
    } else {
        state.registers.clear_sprite0_hit();
    }
    if state.sprite_overflow {
        state.registers.set_sprite_overflow();
    } else {
        state.registers.clear_sprite_overflow();
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
        // Phase 6.1.e follow-up (VBL-block-phase alignment,
        // `docs/history/v2.1_phase6_batch_compat.md` §6.4.3): the
        // frame is VBL-first — visit sequence per frame is
        // `[241..=260, -1, 0..=240]` (262 scanlines × 341 dots =
        // 89342 dots per frame). Two non-monotonic transitions
        // happen on dot-340 wrap:
        //
        //   sl 260 → sl -1 (VBL-block end → pre-render, intra-frame)
        //   sl 240 → sl 241 (post-render → next frame's VBL-block
        //                    start, FRAME WRAP — `frame_advanced`)
        //
        // The previous `next_sl >= NTSC_SCANLINES - 1` (= 261) wrap
        // was pre-render-first (visit `-1, 0..=260`), which placed
        // the VBL block at the END of each frame and gave the VBL
        // flag a different visible window vs the C++ engine's
        // `[VBL 20][pre-render + visible 242]` layout — a 20-scanline
        // constant phase offset behind C++ that broke
        // `rust_ppu_vbl_nmi_timing_test` 02-vbl_set_time and several
        // blargg ppu_open_bus / ppu_read_buffer cases.
        if state.scanline == 260 {
            // VBL-block end → pre-render (intra-frame, no wrap).
            state.scanline = -1;
        } else if state.scanline == 240 {
            // Post-render → next frame's VBL-block start (FRAME WRAP).
            state.scanline = 241;
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
    // Phase 6.1.e follow-up (VBL-first layout): the even/odd skip at
    // (sl -1, dot 340) sits ~7161 ticks into a frame (after the
    // 20-scanline VBL block), and a test that starts with
    // ppudead=1 + rendering on converges only when the second frame's
    // (sl -1, dot 340) ticks no-skip — that's 2 full frames deep
    // (~96504 ticks for the standard `tick_to(0, 0)` reachability
    // probe). The previous `(NTSC_SCANLINES + 2) * DOTS_PER_SCANLINE`
    // guard (~89904 ticks) was sized for the pre-render-first layout
    // where the skip sits 340 ticks in; allow up to 2 frames.
    let max_ticks = 2 * NTSC_SCANLINES as u32 * DOTS_PER_SCANLINE as u32;
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
        // Phase 6.1.e follow-up (VBL-first layout): frame start is
        // (sl 241, dot 0).
        assert_eq!((s.scanline, s.dot), (241, 0));
        tick_dot(&mut s, &mut bus);
        assert_eq!((s.scanline, s.dot), (241, 1));
    }

    #[test]
    fn dot_wraps_into_next_scanline() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        // tick_to(-1, 340) fires the (-1, 340) tick and advances.
        // VBL-first layout: from sl -1 the next scanline is sl 0,
        // NOT sl 241 (sl 241 is the wrap target of the previous
        // frame's sl 240 → 241 boundary).
        let out = tick_to(&mut s, &mut bus, -1, 340);
        assert!(out.dot_wrapped);
        assert!(out.scanline_changed);
        assert_eq!((s.scanline, s.dot), (0, 0));
    }

    #[test]
    fn even_frame_skips_one_dot_after_pre_render() {
        // odd_frame starts false (even); with rendering on, the
        // pre-render dot 340 → next scanline dot 1 (skip).
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
    fn vbl_flag_clears_at_pre_render_dot_1() {
        let mut s = PpuState::new();
        s.ppudead = 0; // post-boot state machine under test
        let mut bus = FlatBus::new();
        // Manually set the VBL flag to prove the state machine clears it.
        // Phase 5.1 geometry: the pre-render line is sl -1 (hardware 261);
        // sl 261 no longer occurs inside a frame.
        s.registers.set_vbl_flag();
        tick_to(&mut s, &mut bus, -1, 1);
        assert_eq!(
            s.registers.status & (1 << status_bits::VBL),
            0,
            "VBL flag should be cleared at the pre-render line dot 1"
        );
    }

    #[test]
    fn vbl_set_at_sl_241_dot_1_with_nmi_enable() {
        // Phase 6.1.e.v3: VBL set is at sl 241 dot 1 (PPU programmer
        // reference, Mesen reference; blargg ppu_vbl_nmi
        // 02-vbl_set_time calibration per §6.6.ter.5).
        let mut s = PpuState::new();
        s.ppudead = 0; // post-boot state machine under test
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
        s.ppudead = 0; // post-boot state machine under test
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();
        let out = tick_to(&mut s, &mut bus, 241, 1);
        assert!(out.nmi_asserted, "NMI should fire on sl 241 dot 1 tick");
    }

    #[test]
    fn vbl_suppression_via_sl_240_dot_340_read_blocks_set() {
        // A $2002 read at (240, 340) — 1 PPU dot before the
        // (PPU-programmer-reference) VBL set at (241, 1) — marks
        // `vbl_suppressed_this_frame` via
        // `apply_a2002_suppression`; the (241, 1) tick checks the
        // flag and skips both `set_vbl_flag` and `nmi_asserted`.
        let mut s = PpuState::new();
        s.ppudead = 0; // post-boot state machine under test
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();

        // Tick to (240, 340) and apply the suppression window as
        // the bridge would on a $2002 read.
        tick_to(&mut s, &mut bus, 240, 340);
        s.apply_a2002_suppression();

        // Advance into (241, 1) — VBL should NOT be set, NMI should NOT fire.
        let out = tick_dot(&mut s, &mut bus);
        assert!(!out.vbl_entered);
        assert!(!out.nmi_asserted);
        assert_eq!(
            s.registers.status & (1 << status_bits::VBL),
            0,
            "VBL flag should NOT be set after (240, 340) suppression"
        );
    }

    #[test]
    fn suppression_resets_at_frame_boundary() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        s.vbl_suppressed_this_frame = true;
        // Tick to the last dot of the pre-render line (-1, 340) →
        // suppression reset; the same tick advances into sl 0 (no
        // even/odd skip: rendering is off in this test). The frame
        // wrap (sl 240 → sl 241) is separate from this intra-frame
        // dot-340 transition — sl -1 still proceeds to sl 0 within
        // the same frame.
        tick_to(&mut s, &mut bus, -1, 340);
        assert!(!s.vbl_suppressed_this_frame);
        assert_eq!(s.scanline, 0, "advanced into the same frame's sl 0");
        assert_eq!(s.dot, 0);
    }

    #[test]
    fn scroll_copy_horizontal_at_visible_scanline_start() {
        let mut s = PpuState::new();
        // Set t = some non-zero scroll value, leave v = 0.
        s.registers.t = 0x1234;
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        // Phase 6.1.e follow-up (VBL-first layout): with ppudead=1 +
        // rendering on + odd_frame=false (cold start), the even skip
        // at (sl -1, dot 340) skips (sl 0, dot 0) on frame 1, so the
        // first reachable (sl 0, dot 0) tick lands on frame 2 (after
        // ppudead decrements and odd_frame toggles). Setting
        // `odd_frame = true` here disables the frame-1 skip so the test
        // can probe (sl 0, dot 0) directly within frame 1, matching
        // the pre-Phase-6.1.e behaviour the test was written against.
        s.odd_frame = true;
        let mut bus = FlatBus::new();
        let _ = tick_to(&mut s, &mut bus, 0, 0);
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

    // Phase 6.1.e follow-up (VBL-block-phase alignment, §6.4.3):
    // the frame wrap goes sl 240 → sl 241 (VBL-first layout).
    #[test]
    fn frame_wraps_from_sl_240_to_sl_241() {
        let mut s = PpuState::new();
        s.ppudead = 0; // post-boot state machine under test
        let mut bus = FlatBus::new();
        // Drive to the last dot of the frame's last visible scanline.
        let out = tick_to(&mut s, &mut bus, 240, 340);
        assert!(out.dot_wrapped, "dot 340 → dot 0 wrap fires");
        assert!(out.scanline_changed, "scanline transition fires");
        assert!(
            out.frame_advanced,
            "frame wrap detected — sl 240 → next frame's sl 241"
        );
        assert_eq!(s.scanline, 241, "wrapped to next frame's VBL-block start");
        assert_eq!(s.dot, 0);
    }

    #[test]
    fn frame_starts_at_sl_241_dot_0() {
        // Cold-state: PpuState::new() must place (sl, dot) at (241, 0)
        // (VBL-block-first layout, §6.4.3).
        let s = PpuState::new();
        assert_eq!((s.scanline, s.dot), (241, 0));
    }

    #[test]
    fn ppudead_sets_vbl_at_sl_241_dot_0_and_decrements_at_sl_240() {
        // Phase 6.1.e follow-up: ppudead frame mirrors C++
        // ppu_rendering.cpp:1626-1655 (VBL set at frame dot 0,
        // VBL block, normal frame, decrement at frame end).
        let mut s = PpuState::new();
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        // PpuState::new() initialises ppudead = 1.
        let mut bus = FlatBus::new();

        // (sl 241, dot 0): ppudead VBL set + NMI assert.
        let first = tick_to(&mut s, &mut bus, 241, 0);
        assert!(first.vbl_entered, "VBL set at frame start of ppudead");
        assert!(first.nmi_asserted, "NMI fires at frame start of ppudead");
        assert_eq!(s.ppudead, 1, "ppudead still pending for the rest of frame");

        // (sl 240, dot 340): ppudead decrement + frame wrap.
        // `tick_to(240, 340)` fires the (240, 340) tick — which
        // decrements ppudead — and advances to (sl 241, dot 0) of
        // the NEXT frame (frame_advanced = true).
        let out = tick_to(&mut s, &mut bus, 240, 340);
        assert_eq!(
            s.ppudead, 0,
            "ppudead decremented at frame end (sl 240 dot 340)"
        );
        assert!(out.frame_advanced, "frame wrapped at sl 240 → sl 241");
        assert_eq!((s.scanline, s.dot), (241, 0), "next frame starts at (241, 0)");
    }

    // -----------------------------------------------------------------
    // Phase 6.6 (Session A): per-pixel sprite 0 hit + per-frame flag
    // clears.
    // -----------------------------------------------------------------

    /// Render scanline 0's sprite pass over an opaque BG row and tick
    /// through the scanline: PPU[2] bit 6 must stay clear until the
    /// recorded hit dot (pixel x + 1) and set from that dot onward.
    #[test]
    fn sprite0_hit_latches_at_recorded_dot() {
        use crate::rendering::palette_adjust_pixel;
        use crate::sprites::render_sprites_for_scanline;

        let mut s = PpuState::new();
        s.ppudead = 0;
        s.registers
            .write_mask((1 << mask_bits::SHOW_BG) | (1 << mask_bits::SHOW_SPRITES));
        // Sprite 0: y=0 (visible on scanline 0), tile 1, x=10, front.
        s.oam[0..4].copy_from_slice(&[0, 0x01, 0x00, 10]);
        let mut bus = FlatBus::new();
        bus.chr[0x01 * 16] = 0xFF; // pattern row 0 fully opaque

        let mask = s.registers.mask;
        let mut palette = [0u8; 32];
        palette[1] = 0x16;
        let mut fb = [0u8; 256 * 256];
        // Pre-fill the BG row with an opaque (non-backdrop) pixel the
        // BG pass would have written.
        for x in 0..256 {
            fb[x] = palette_adjust_pixel(0x25, mask);
        }
        let chr_window = bus.chr;
        render_sprites_for_scanline(&mut s, &mut bus, Some(&chr_window), &mut fb, &palette, mask, 0);
        assert_eq!(s.sprite0_hit_dot, 11, "hit recorded at pixel 10 → dot 11");
        assert!(!s.sprite0_hit, "flag must NOT latch during the render pass");

        // Drive the state machine to the start of scanline 0 and tick
        // through it: tick i fires the events of dot i.
        s.scanline = 0;
        s.dot = 0;
        let bit6 = 1 << status_bits::SPRITE0_HIT;
        for dot in 0u16..=12 {
            let _ = tick_dot(&mut s, &mut bus);
            if dot < 11 {
                assert_eq!(
                    s.registers.status & bit6,
                    0,
                    "dot {dot}: hit must not be visible yet"
                );
            } else {
                assert_ne!(
                    s.registers.status & bit6,
                    0,
                    "dot {dot}: hit latched at its recorded dot"
                );
            }
        }
    }

    /// Sprite 0 hit and sprite overflow clear at the pre-render line
    /// dot 1 (hardware PPU frame timing), including the status bits.
    #[test]
    fn sprite0_hit_and_overflow_clear_at_pre_render_dot_1() {
        let mut s = PpuState::new();
        s.ppudead = 0;
        s.sprite0_hit = true;
        s.sprite_overflow = true;
        s.registers.set_sprite0_hit();
        s.registers.set_sprite_overflow();
        let mut bus = FlatBus::new();
        tick_to(&mut s, &mut bus, -1, 1);
        assert!(!s.sprite0_hit, "hit latch cleared at pre-render dot 1");
        assert!(!s.sprite_overflow, "overflow cleared at pre-render dot 1");
        assert_eq!(
            s.registers.status & (1 << status_bits::SPRITE0_HIT),
            0,
            "status bit 6 cleared"
        );
        assert_eq!(
            s.registers.status & (1 << status_bits::SPRITE_OVERFLOW),
            0,
            "status bit 5 cleared"
        );
    }

    /// The status overflow bit must follow the state flag every tick —
    /// a stale set bit with `sprite_overflow == false` clears on the
    /// next tick (previously the bit latched for the whole run).
    #[test]
    fn overflow_status_bit_mirrors_state_flag() {
        let mut s = PpuState::new();
        s.ppudead = 0;
        s.registers.set_sprite_overflow();
        let mut bus = FlatBus::new();
        let _ = tick_dot(&mut s, &mut bus);
        assert_eq!(
            s.registers.status & (1 << status_bits::SPRITE_OVERFLOW),
            0,
            "stale overflow bit cleared by the state mirror"
        );
    }
}
