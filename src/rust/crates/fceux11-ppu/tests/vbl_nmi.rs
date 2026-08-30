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
    // (one PPU dot before the Mesen/fceux VBL set boundary)
    // suppresses the VBL flag set + NMI for this frame.
    let mut s = PpuState::new();
    s.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
    let mut bus = FlatBus::new();

    // Tick to sl 241 dot 0.
    let _ = tick_to(&mut s, &mut bus, 241, 0);

    // CPU reads $2002 here. The legacy C++ flag is `fceu11_ppu_mark_vbl_set_suppressed`;
    // here we toggle the state flag directly (Phase 2 will wire it through FFI).
    let v = s.registers.read_status();
    assert_eq!(
        v & (1 << status_bits::VBL),
        0,
        "VBL not yet set at sl 241 dot 0"
    );
    s.vbl_suppressed_this_frame = true;

    // Advance into sl 241 dot 1 — VBL should NOT be set, NMI should NOT fire.
    let out = tick_to(&mut s, &mut bus, 241, 1);
    assert!(!out.vbl_entered);
    assert!(!out.nmi_asserted);
    assert_eq!(
        s.registers.status & (1 << status_bits::VBL),
        0,
        "VBL flag should NOT be set after suppression"
    );
}

#[test]
fn status_read_at_sl_241_dot_1_clears_vbl_flag() {
    // Reading $2002 at sl 241 dot 1 returns VBL=1 (set this same tick)
    // and clears it for subsequent reads.
    let mut s = PpuState::new();
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
    // Enable rendering so even/odd logic fires.
    s.registers.write_mask(0x08 | 0x10); // show BG + show sprites
    let mut bus = FlatBus::new();

    // Start at (-1, 0) with odd_frame=false (even). Advance to (-1, 340).
    let pre = tick_to(&mut s, &mut bus, -1, 340);
    assert!(pre.pre_render_decision);
    // Skip fired → state at (0, 1), not (0, 0).
    assert_eq!(s.scanline, 0);
    assert_eq!(s.dot, 1);
    assert!(s.odd_frame, "odd_frame toggled to true after even skip");
}

#[test]
fn odd_frame_runs_full_pre_render() {
    let mut s = PpuState::new();
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
    s.vbl_suppressed_this_frame = true;
    let mut bus = FlatBus::new();
    // Tick to the last dot of the pre-render line (-1, 340) — the
    // boundary tick that fires the suppression reset, then advances
    // into the next frame's sl 0 (no even/odd skip: rendering off).
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
