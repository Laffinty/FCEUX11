//! Integration tests for VBlank/NMI timing.
//!
//! These tests exercise the dot-level timing model of `fceux11-ppu` end
//! to end. They use the [`tick_to`] helper to fast-forward to the
//! exact (sl, dot) position and assert the state-machine reactions.
//!
//! Reference: `docs/plans/v2.1_ppu_rust_refactor_plan.md` §6.6 and
//! <https://www.nesdev.org/wiki/NMI>.

use fceux11_ppu::{FlatBus, PpuState, Registers, ctrl_bits, frame::tick_to, status_bits};

#[test]
fn vbl_flag_set_at_sl_241_dot_1_with_nmi_enable() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    let mut bus = FlatBus::new();

    let out = tick_to(&mut s, &mut bus, 241, 1);
    assert!(out.vbl_entered, "VBL flag transitioned to set this tick");
    assert!(
        s.registers.status & (1 << status_bits::VBL) != 0,
        "VBL flag is set after sl 241 dot 1"
    );
}

#[test]
fn vbl_flag_clear_at_pre_render_dot_1() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    let mut bus = FlatBus::new();
    // Pre-set the VBL flag to prove the state machine clears it.
    // Phase 5.1 geometry: the pre-render line is sl -1 (hardware 261);
    // sl 261 no longer occurs inside a frame.
    s.registers.set_vbl_flag();
    let _ = tick_to(&mut s, &mut bus, -1, 1);
    assert_eq!(
        s.registers.status & (1 << status_bits::VBL),
        0,
        "VBL flag should be cleared at the pre-render line dot 1"
    );
}

#[test]
fn nmi_not_asserted_when_nmi_disable_bit_off() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.registers.write_ctrl(0); // NMI off
    let mut bus = FlatBus::new();
    let out = tick_to(&mut s, &mut bus, 241, 1);
    assert!(!out.nmi_asserted, "NMI must not fire with NMI_ENABLE = 0");
    assert!(
        out.vbl_entered,
        "VBL flag must still set even when NMI is gated off"
    );
}

#[test]
fn nmi_asserted_only_at_sl_241_dot_1_when_enabled() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    let mut bus = FlatBus::new();

    // Tick to sl 241 dot 0 — NMI should not fire yet.
    let out = tick_to(&mut s, &mut bus, 241, 0);
    assert!(!out.nmi_asserted);
    assert!(!out.vbl_entered);

    // One more tick (sl 241 dot 1) — NMI should fire.
    let next = tick_to(&mut s, &mut bus, 241, 1);
    assert!(next.nmi_asserted);
    assert!(next.vbl_entered);
}

#[test]
fn status_read_at_sl_241_dot_0_suppresses_vbl_flag_set() {
    // The plan §6.6 suppression window: a $2002 read at sl 241 dot 0
    // (one PPU dot before the PPU-programmer-reference VBL set at
    // sl 241 dot 1) suppresses the VBL flag set + NMI for this frame.
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    let mut bus = FlatBus::new();

    // Tick to sl 241 dot 0 and call apply_a2002_suppression.
    tick_to(&mut s, &mut bus, 241, 0);
    s.apply_a2002_suppression();

    // Advance into sl 241 dot 1 — VBL should NOT be set, NMI should NOT fire.
    let out = tick_to(&mut s, &mut bus, 241, 1);
    assert!(!out.vbl_entered);
    assert!(!out.nmi_asserted);
    assert_eq!(
        s.registers.status & (1 << status_bits::VBL),
        0,
        "VBL flag should NOT be set after (241, 0) suppression"
    );
}

#[test]
fn status_read_at_sl_241_dot_1_clears_vbl_flag() {
    // Reading $2002 at sl 241 dot 1 returns VBL=1 (set this same tick)
    // and clears it for subsequent reads.
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    let mut bus = FlatBus::new();
    let _ = tick_to(&mut s, &mut bus, 241, 1);
    let first = s.registers.read_status();
    assert_ne!(first & (1 << status_bits::VBL), 0, "VBL=1 on first read");
    let second = s.registers.read_status();
    assert_eq!(
        second & (1 << status_bits::VBL),
        0,
        "VBL=0 after the read cleared it"
    );
}

#[test]
fn nmi_enable_off_to_on_after_vbl_does_not_retrigger() {
    // The NMI is a level + edge source per nesdev. Phase 1 doesn't model
    // the edge latching exactly; we just assert that flipping NMI_ENABLE
    // off→on *after* the sl 241 dot 1 NMI doesn't fire a fresh NMI on
    // the same frame.
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    let mut bus = FlatBus::new();
    let first = tick_to(&mut s, &mut bus, 241, 1);
    assert!(first.nmi_asserted);
    // Flip NMI off then back on later in the frame.
    s.registers.write_ctrl(0);
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    // Tick to sl 260 dot 0; no NMI should fire on this advance.
    let later = tick_to(&mut s, &mut bus, 260, 0);
    assert!(
        !later.nmi_asserted,
        "NMI re-enable after VBL is not edge-triggered"
    );
}

#[test]
fn even_frame_skips_one_dot_at_pre_render_boundary() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    // Enable rendering so even/odd logic fires.
    s.registers.write_mask(0x08 | 0x10); // show BG + show sprites
    let mut bus = FlatBus::new();

    // Phase 6.1.e follow-up: cold start is (sl 241, dot 0). Drive to
    // the pre-render line dot 340 — the even/odd decision tick.
    let pre = tick_to(&mut s, &mut bus, -1, 340);
    assert!(pre.pre_render_decision);
    // Skip fired → state advances to (0, 1), not (0, 0). This is an
    // intra-frame transition (-1 → 0), not the frame wrap.
    assert_eq!(s.scanline, 0);
    assert_eq!(s.dot, 1);
    assert!(s.odd_frame, "odd_frame toggled to true after even skip");
}

#[test]
fn odd_frame_runs_full_pre_render() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.odd_frame = true;
    s.registers.write_mask(0x08 | 0x10);
    let mut bus = FlatBus::new();

    let pre = tick_to(&mut s, &mut bus, -1, 340);
    assert!(pre.pre_render_decision);
    // No skip: state lands at (0, 0).
    assert_eq!((s.scanline, s.dot), (0, 0));
    assert!(!s.odd_frame);
}

#[test]
fn rendering_off_freezes_odd_frame() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    // mask stays 0 — rendering off.
    s.odd_frame = false;
    let mut bus = FlatBus::new();
    let pre = tick_to(&mut s, &mut bus, -1, 340);
    assert!(pre.pre_render_decision);
    assert_eq!((s.scanline, s.dot), (0, 0));
    assert!(
        !s.odd_frame,
        "odd_frame must not toggle when rendering is off"
    );
}

#[test]
fn suppression_flag_resets_at_frame_boundary() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine under test (Phase 6.4)
    s.vbl_suppressed_this_frame = true;
    let mut bus = FlatBus::new();
    // Tick to the last dot of the pre-render line (-1, 340) — the
    // boundary tick that fires the suppression reset, then advances
    // into the same frame's sl 0 (no even/odd skip: rendering off).
    // Phase 6.1.e follow-up: this is an intra-frame transition
    // (-1 → 0). The frame wrap goes sl 240 → sl 241 (separate path).
    let _ = tick_to(&mut s, &mut bus, -1, 340);
    assert!(!s.vbl_suppressed_this_frame);
    assert_eq!(s.scanline, 0);
    assert_eq!(s.dot, 0);
}

#[test]
fn registers_default_is_no_vbl_no_nmi() {
    // Sanity: a fresh Registers struct has VBL=0; reading $2002 returns 0.
    let mut r = Registers::new();
    assert_eq!(r.read_status(), 0);
}

// ===========================================================================
// Plan §0.8 step 1D.2: PpuState::apply_a2002_suppression unit tests.
//
// NESdev PPU frame timing suppression window:
//   (sl 240, dot 340) 1 dot before VBL set
//     -> mark vbl_suppressed_this_frame = true
//        (frame state machine skips VBL set + NMI assert for the frame)
//   (sl 241, dot 0 or 1) same dot or 1 dot later than VBL set
//     -> nmi_pending = false  (read pulls /NMI back up before CPU samples)
//   other dots -> no-op
// ===========================================================================

#[test]
fn suppression_flag_set_at_sl_240_dot_340() {
    let mut s = PpuState::new();
    s.ppudead = 0; // post-boot state machine
    s.scanline = 240;
    s.dot = 340;
    assert!(!s.vbl_suppressed_this_frame, "precondition: flag is clear");
    s.apply_a2002_suppression();
    assert!(
        s.vbl_suppressed_this_frame,
        "1 dot before VBL set marks the frame's VBL+NMI for suppression"
    );
    // Tick to (241, 1) and confirm the state machine does NOT set
    // VBL or assert NMI this tick.
    let mut bus = FlatBus::new();
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    let out = tick_to(&mut s, &mut bus, 241, 1);
    assert!(
        !out.vbl_entered,
        "VBL must NOT enter when suppression flag is set"
    );
    assert!(
        !out.nmi_asserted,
        "NMI must NOT assert when suppression flag is set"
    );
    assert_eq!(
        s.registers.status & (1 << status_bits::VBL),
        0,
        "VBL flag must remain clear through the suppressed set dot"
    );
}

#[test]
fn nmi_pending_cleared_at_sl_241_dot_1() {
    let mut s = PpuState::new();
    s.ppudead = 0;
    s.scanline = 241;
    s.dot = 1;
    // Simulate the per-dot interleave having just latched NMI
    // (this is what ffi.rs::fceux11_ppu_tick_dots does on
    // outcome.nmi_asserted).
    s.nmi_pending = true;
    s.apply_a2002_suppression();
    assert!(
        !s.nmi_pending,
        "read at (241, 1) cancels the pending NMI latch          (pulls /NMI back up before CPU samples)"
    );
}

#[test]
fn a2002_read_at_sl_241_dot_0_cancels_nmi_only() {
    // Plan §0.8 step 2 (Option B) follow-up: the original c872db7
    // fix marked (241, 0) as a "set vbl_suppressed_this_frame" dot
    // based on the NESdev PPU programmer reference's "the dot
    // before it is set (scanline 241, dot 0)" wording. That
    // broke `blargg_ppu_read_buffer` and
    // `blargg_vbl_05_nmi_timing` (kagami_qa_direct_smoke 5P/7F
    // -> 3P/9F).
    //
    // The project's C++ engine baseline at `src/ppu.cpp:625-629`
    // implements VBL set at the (sl 240 -> sl 241) boundary
    // (i.e. (241, 0) IS the VBL set dot), and treats reads at
    // (241, 0) / (241, 1) as NMI-cancel-only without marking the
    // VBL set as suppressed. To match the C++ engine working
    // config, (241, 0) is now in the NMI-cancel-only arm.
    //
    // (240, 340) remains the only "set vbl_suppressed_this_frame"
    // dot (1 clock before the VBL set on the previous scanline
    // tick).
    let mut s = PpuState::new();
    s.ppudead = 0;
    s.scanline = 241;
    s.dot = 0;
    s.nmi_pending = true;
    s.apply_a2002_suppression();
    assert!(
        !s.vbl_suppressed_this_frame,
        "(241, 0) is the VBL set dot per C++ engine (src/ppu.cpp:625-629) - must NOT mark vbl_suppressed_this_frame"
    );
    assert!(
        !s.nmi_pending,
        "(241, 0) must cancel the pending NMI (the read pulls /NMI back up before the CPU samples it)"
    );
}

#[test]
fn nmi_pending_cleared_at_sl_241_dot_2() {
    // (241, 2) is the "1 dot later than VBL set" position. The
    // NESdev PPU programmer reference states:
    //   "NMI is also suppressed when this occurs, and may even be
    //    suppressed by reads landing on the following dot or two."
    // So (241, 1) and (241, 2) both clear `nmi_pending` without
    // affecting `vbl_suppressed_this_frame` (VBL was already set
    // at (241, 1) and `Registers::read_status` will clear it on
    // read; the NMI line is just pulled back up before the CPU
    // samples it). The previous implementation missed (241, 2)
    // entirely.
    let mut s = PpuState::new();
    s.ppudead = 0;
    s.scanline = 241;
    s.dot = 2;
    s.nmi_pending = true;
    s.apply_a2002_suppression();
    assert!(
        !s.nmi_pending,
        "(241, 2) is the '1 dot later' window — must cancel pending NMI"
    );
    assert!(
        !s.vbl_suppressed_this_frame,
        "(241, 2) is after VBL set — must NOT mark the frame as suppressed"
    );
}

#[test]
fn a2002_read_outside_suppression_window_is_noop() {
    let mut s = PpuState::new();
    s.ppudead = 0;
    // Pick a dot well away from the VBL set boundary.
    s.scanline = 100;
    s.dot = 50;
    s.nmi_pending = false;
    s.apply_a2002_suppression();
    assert!(!s.vbl_suppressed_this_frame);
    assert!(!s.nmi_pending);
}
