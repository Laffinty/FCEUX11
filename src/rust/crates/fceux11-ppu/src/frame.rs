//! `tick_dot` — the dot-level main state machine.
//!
//! Each call advances the PPU by exactly one PPU dot. The current scanline
//! and dot are consulted to determine which side effects fire this tick:
//! VBlank flag set/clear, NMI sampling, the background fetch cycle's
//! scroll updates (coarse-X increments at the end of each 8-dot fetch
//! group, fine-Y increment at dot 256, horizontal reload at dot 257),
//! sprite evaluation at the start of a visible scanline, even/odd skip on
//! the pre-render line.
//!
//! v2.1.3 batch 2: the fetches themselves, and the pixels they produce,
//! live in [`crate::rendering::tick_dot`], which the per-dot drivers call
//! immediately *before* this function for the same dot. The split keeps
//! the scroll model in the state machine while the data-dependent
//! pipeline stays with the renderer (which holds the NT/CHR windows).
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
use crate::state::{DOTS_PER_SCANLINE, PpuState};

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
    // Step B.5-2b: the raster geometry is region-dependent (NTSC 262
    // lines, PAL/Dendy 312) - see crate::video_system.
    let timings = state.video_system.timings();

    // v2.1.3 batch 2: advance the A12 watcher's dot clock. The fetch
    // addresses themselves are reported by the background pipeline
    // (`crate::rendering::tick_dot`), which the per-dot drivers run just
    // before this function for the same dot and which knows the address
    // each fetch actually places on the bus. With rendering off there
    // are no fetches, so the reporting comes from the CPU access path
    // (`ffi.rs` $2006/$2007/$2001 handlers) — blargg MMC3 test 3 counts
    // those whether or not rendering is on.
    state.a12.advance_tick();
    // Entering the post-render line (sl 240): fetching stops and the bus
    // drops back to `v` (Mesen2 does the same SetBusAddress at scanline
    // 240 cycle 0). Rendering-off mid-frame transitions are reported by
    // the `$2001` write handler in `ffi.rs`.
    if sl == 240 && dot == 0 {
        let v = state.registers.v & 0x3FFF;
        if state.a12.observe(v) {
            _bus.notify_a12_rising();
        }
    }

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
        // NTSC-only: "This behavior is NTSC-specific - PAL frames are
        // always the same number of cycles" (Mesen2 NesPpu.cpp:950-954).
        if timings.odd_frame_skips_dot && state.rendering_enabled() {
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
    }

    // Background fetch-cycle scroll updates (v2.1.3 batch 2, nesdev PPU
    // rendering / PPU frame timing):
    //
    // - coarse X increments at the last dot of every 8-dot fetch group:
    //   dots 8, 16, ..., 256 for the visible tile walk and dots 328 /
    //   336 for the two-tile preload of the next scanline;
    // - fine Y (with carry into coarse Y / nametable Y) increments at
    //   dot 256;
    // - the horizontal bits of t are copied into v at dot 257.
    //
    // The pre-render line runs the same cycle — its preload groups feed
    // the first two tiles of scanline 0 — so it gets the same updates.
    // `crate::rendering::tick_dot` reads `v` at each fetch; keeping the
    // scroll updates here leaves the whole loopy model in the dot state
    // machine instead of the (bridge-side) renderer.
    //
    // The dot-0 `copy_horizontal` this replaced was a batch-renderer
    // artifact: the pipeline's preload groups already leave coarse X at
    // t.coarseX + 2 for the line's first fetched tile.
    if (sl == -1 || (0..=239).contains(&sl)) && state.rendering_enabled() {
        let in_fetch_region = (1..=256).contains(&dot) || (321..=336).contains(&dot);
        if in_fetch_region && dot % 8 == 0 {
            state.registers.increment_coarse_x();
        }
        if dot == 256 {
            state.registers.increment_fine_y();
        }
        if dot == 257 {
            state.registers.copy_horizontal();
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
    // Batch 3c.1 (v2.1.4 plan §2, milestone 3c): restored the documented
    // dot-1 set point. The code below had drifted back to `dot == 0`
    // (commit bb4a9f2, the v2.1.1-era A.2 tracediff trade) while this
    // comment block, the suppression design, the vbl_nmi integration
    // tests, and the Mesen/fceux-original/Nestopia calibration all
    // specify **sl 241 dot 1** — the flag set lands on the second dot
    // of the VBL line, with the suppression window on the dot-0 $2002
    // read. Verified locally against the real blargg suite (177 ROMs,
    // fixtures downloaded): dot=0 fails vbl_02/03/04/06/07/08/10 +
    // ppu_vbl_nmi; see the 3c.1 commit for the after numbers.
    if sl == timings.vbl_set_scanline && dot == 1 && state.ppudead == 0 {
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
        if sl == timings.vbl_set_scanline && dot == 0 {
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

    // v2.1.3 batch 2: sprite-0 hit is latched by
    // `crate::rendering::tick_dot` at the exact dot the hit pixel is
    // output; there is no separate recorded hit dot to replay here.

    // Sprite 0 hit + sprite overflow latched by eval/pipeline: mirror the
    // state flags onto PPU[2] bits 6/5 every tick. With the pre-render
    // clear above this stays consistent with hardware: both flags clear
    // once per frame and re-set only where the pipeline sets them.
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
        if state.scanline == state.video_system.timings().vbl_end_scanline {
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
    let timings = state.video_system.timings();
    let max_ticks = 2 * timings.scanlines as u32 * timings.dots_per_scanline as u32;
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
    /// Step B.5-2b gate: one frame of dots must equal the region table's
    /// `dots_per_frame`, and the VBL flag must be set on the region's
    /// `vbl_set_scanline` (Dendy = 291, NTSC/PAL = 241).
    #[test]
    fn frame_length_and_vbl_line_follow_the_region_table() {
        use crate::video_system::VideoSystem;
        for sys in [VideoSystem::Ntsc, VideoSystem::Pal, VideoSystem::Dendy] {
            let t = sys.timings();
            let mut s = PpuState::new();
            s.ppudead = 0;
            s.video_system = sys;
            let mut bus = FlatBus::new();

            // Rendering stays off, so the NTSC odd-frame dot skip is inert
            // and the frame length is exactly the table value.
            let mut dots = 0u32;
            let mut vbl_line: Option<i16> = None;
            loop {
                let out = tick_dot(&mut s, &mut bus);
                dots += 1;
                if out.vbl_entered && vbl_line.is_none() {
                    vbl_line = Some(s.scanline);
                }
                if out.frame_advanced {
                    break;
                }
                assert!(dots < 2 * t.dots_per_frame, "{sys:?} frame never wrapped");
            }
            assert_eq!(dots, t.dots_per_frame, "{sys:?} frame length");
            assert_eq!(vbl_line, Some(t.vbl_set_scanline), "{sys:?} VBL set line");
        }
    }

    #[test]
    fn nmi_asserted_at_sl_241_dot_1_when_enabled() {
        // Batch 3c.1: restored the documented dot-1 set point (the
        // bb4a9f2 dot-0 trade is reverted; see the VBL set block).
        let mut s = PpuState::new();
        s.ppudead = 0; // post-boot state machine under test
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();
        let out = tick_to(&mut s, &mut bus, 241, 0);
        assert!(!out.nmi_asserted, "NMI must NOT fire on dot 0");
        assert!(!out.vbl_entered, "VBL must NOT set on dot 0");
        let out = tick_to(&mut s, &mut bus, 241, 1);
        assert!(out.nmi_asserted, "NMI fires on the sl 241 dot 1 tick");
        assert!(out.vbl_entered, "VBL sets on the sl 241 dot 1 tick");
    }

    #[test]
    fn vbl_suppression_via_sl_241_dot_0_read_blocks_set() {
        // Batch 3c.1: a $2002 read at (241, 0) — 1 PPU dot before the
        // VBL set at (241, 1) — marks `vbl_suppressed_this_frame` via
        // `apply_a2002_suppression`; the (241, 1) tick checks the
        // flag and skips both `set_vbl_flag` and `nmi_asserted`.
        let mut s = PpuState::new();
        s.ppudead = 0; // post-boot state machine under test
        s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();

        // apply_a2002_suppression matches the CURRENT (sl, dot) and
        // tick_to leaves the state one dot PAST its target, so tick to
        // (240, 340) to actually be at (241, 0) when the read happens.
        tick_to(&mut s, &mut bus, 240, 340);
        assert_eq!((s.scanline, s.dot), (241, 0), "precondition");
        s.apply_a2002_suppression();

        // The (241, 1) tick processes the (suppressed) VBL set.
        let out = tick_dot(&mut s, &mut bus);
        assert!(!out.vbl_entered);
        assert!(!out.nmi_asserted);
        assert_eq!(
            s.registers.status & (1 << status_bits::VBL),
            0,
            "VBL flag should NOT be set after (241, 0) suppression"
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

    /// v2.1.3 batch 2: the horizontal scroll is reloaded at dot 257 only
    /// — the batch renderer's extra dot-0 reload would undo the two
    /// coarse-X increments the preload groups make for the next line.
    #[test]
    fn horizontal_reload_at_dot_257_and_preload_advances_coarse_x() {
        let mut s = PpuState::new();
        s.registers.t = 0x1234; // coarse X = 0x14
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        // Phase 6.1.e follow-up (VBL-first layout): with ppudead=1 +
        // rendering on + odd_frame=false (cold start), the even skip
        // at (sl -1, dot 340) skips (sl 0, dot 0) on frame 1, so the
        // first reachable (sl 0, dot 0) tick lands on frame 2. Setting
        // `odd_frame = true` disables the frame-1 skip so the test can
        // probe (sl 0, dot 0) within frame 1.
        s.odd_frame = true;
        let mut bus = FlatBus::new();
        let _ = tick_to(&mut s, &mut bus, 0, 0);
        // The pre-render line's dot-257 reload set v's horizontal bits
        // from t; its two preload groups then incremented coarse X
        // twice, so the visible line fetches tile 3 at t.coarseX + 2.
        assert_eq!(
            s.registers.v & 0x1F,
            (s.registers.t & 0x1F) + 2,
            "preload groups advance coarse X past the dot-257 reload"
        );
        let _ = tick_to(&mut s, &mut bus, 0, 257);
        assert_eq!(
            s.registers.v & 0x041F,
            s.registers.t & 0x041F,
            "dot 257 reloads the horizontal scroll bits of t into v"
        );
    }

    #[test]
    fn dot_257_copies_horizontal_bits_from_t() {
        let mut s = PpuState::new();
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        s.registers.v = 0x1000; // coarse_x = 0, vertical bits nonzero
        s.registers.t = 0x021F; // h bits: coarse_x = 31, nametable_x = 0
        let mut bus = FlatBus::new();
        tick_to(&mut s, &mut bus, 100, 256);
        tick_dot(&mut s, &mut bus); // dot 256: coarse-X increment
        tick_dot(&mut s, &mut bus); // dot 257: t -> v horizontal copy
        assert_eq!(
            s.registers.v & 0x041F,
            s.registers.t & 0x041F,
            "dot 257 copies the horizontal scroll bits of t into v"
        );
        // The vertical bits must survive a visible scanline untouched:
        // copying them here reset the vertical scroll every line, which
        // made every scanline fetch the same tile row (KIRA.nes: flat
        // backdrop plus sprites only).
        assert_ne!(
            s.registers.v & 0x7BE0,
            s.registers.t & 0x7BE0,
            "a visible scanline must not copy the vertical bits of t into v"
        );
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

    /// Sprite 0's opaque pixel over an opaque BG pixel latches PPU[2]
    /// bit 6 exactly at the dot the pixel is output (pixel x → dot
    /// x + 1); earlier dots of the same scanline must still read clear.
    #[test]
    fn sprite0_hit_latches_at_the_hit_dot() {
        use crate::rendering::{RenderWindows, tick_dot as render_dot};
        use crate::registers::mask_bits as mb;

        let mut s = PpuState::new();
        s.ppudead = 0;
        s.registers
            .write_mask((1 << mb::SHOW_BG) | (1 << mb::SHOW_SPRITES));
        // Sprite 0: y=0 (visible on scanline 0), tile 1, x=10, front.
        s.oam[0..4].copy_from_slice(&[0, 0x01, 0x00, 10]);

        let mut chr = [0u8; 8192];
        chr[0x01 * 16] = 0xFF; // sprite pattern row 0 fully opaque
        for i in 0..16 {
            chr[i] = 0xFF; // BG tile 0 opaque
        }
        let nt = [0u8; 4096];
        let mut palette = [0u8; 32];
        palette[3] = 0x25; // BG colour 3
        palette[0x11] = 0x16; // sprite palette 0, colour 1

        let win = RenderWindows {
            nt: &nt,
            chr: &chr,
            palette: &palette,
            mirror: 0,
        };
        let mut bus = FlatBus::new();
        let bit6 = 1 << status_bits::SPRITE0_HIT;
        let mut fb = [0u8; 256 * 256];

        s.registers.v = 0x2000;
        s.registers.t = 0x2000;
        s.scanline = 0;
        s.dot = 0;
        // Priming pass fills the BG shift registers.
        for _ in 0..crate::state::DOTS_PER_SCANLINE {
            render_dot(&mut s, &mut bus, &win, &mut fb);
            tick_dot(&mut s, &mut bus);
        }
        // Resume at dot 0 with the previous line's preload increments.
        s.registers.v = 0x2000;
        s.registers.increment_coarse_x();
        s.registers.increment_coarse_x();
        s.scanline = 0;
        s.dot = 0;
        s.sprite0_hit = false;

        for dot in 0..256u16 {
            render_dot(&mut s, &mut bus, &win, &mut fb);
            tick_dot(&mut s, &mut bus);
            if dot >= 11 {
                assert_ne!(
                    s.registers.status & bit6,
                    0,
                    "dot {dot}: hit latched at its pixel's dot"
                );
            } else {
                assert_eq!(
                    s.registers.status & bit6,
                    0,
                    "dot {dot}: hit must not be visible yet"
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
