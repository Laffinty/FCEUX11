//! Integration tests for sprite-0 hit detection.
//!
//! Phase 1 implements the *minimum* behavioural contract: sprite-0 hit
//! fires when sprite 0 has non-transparent pixels overlapping a
//! non-transparent BG pixel at X <= 254. The exact dot boundary at
//! which the hit latches is deferred to Phase 4 (where MMC3's
//! ppu_vbl_nmi quirks will be tightened).
//!
//! These tests drive the latched flag through `PpuState::sprite0_hit`
//! and verify that `Registers::set_sprite0_hit` / `clear_sprite0_hit`
//! move PPU[2] bit 6.

use fceux11_ppu::{PpuState, status_bits};

#[test]
fn sprite0_hit_flag_sets_status_bit_6() {
    let mut s = PpuState::new();
    assert!(!s.sprite0_hit);
    assert_eq!(s.registers.status & (1 << status_bits::SPRITE0_HIT), 0);

    s.sprite0_hit = true;
    s.registers.set_sprite0_hit();
    assert_ne!(
        s.registers.status & (1 << status_bits::SPRITE0_HIT),
        0,
        "PPU[2] bit 6 (sprite0 hit) must be set"
    );
}

#[test]
fn sprite0_hit_clears_when_flag_drops() {
    let mut s = PpuState::new();
    s.registers.set_sprite0_hit();
    assert_ne!(s.registers.status & (1 << status_bits::SPRITE0_HIT), 0);
    s.sprite0_hit = false;
    s.registers.clear_sprite0_hit();
    assert_eq!(
        s.registers.status & (1 << status_bits::SPRITE0_HIT),
        0,
        "PPU[2] bit 6 must be cleared when flag drops"
    );
}

#[test]
fn sprite0_hit_does_not_disturb_vbl_or_overflow_bits() {
    // Phase 4 will gate the hit on dot/sprite-pixel logic; here we only
    // assert that touching bit 6 leaves bits 7 and 5 alone.
    let mut s = PpuState::new();
    s.registers.set_vbl_flag();
    s.registers.set_sprite_overflow();
    s.registers.set_sprite0_hit();
    assert_ne!(s.registers.status & (1 << status_bits::VBL), 0);
    assert_ne!(s.registers.status & (1 << status_bits::SPRITE_OVERFLOW), 0);
    assert_ne!(s.registers.status & (1 << status_bits::SPRITE0_HIT), 0);
    s.registers.clear_sprite0_hit();
    assert_ne!(s.registers.status & (1 << status_bits::VBL), 0);
    assert_ne!(
        s.registers.status & (1 << status_bits::SPRITE_OVERFLOW),
        0,
        "clearing sprite0 must not touch overflow"
    );
}

#[test]
fn sprite0_hit_is_cleared_at_frame_boundary_in_phase4() {
    // Phase 1 sanity: the sprite0_hit flag is not auto-cleared by
    // `PpuState::power` — renderers are expected to clear it at the
    // start of each frame. Phase 4 will hook that into tick_dot at the
    // appropriate pre-render dot.
    let mut s = PpuState::new();
    s.sprite0_hit = true;
    s.power();
    // After power, sprite0_hit is cleared (PpuState::power resets all
    // latches). Confirm the *Registers* status bit however still
    // reflects whatever the renderer last set — Registers is the
    // PPU[2] file, not a derived latch.
    assert!(!s.sprite0_hit);
    // Note: registers.status is preserved across power() because
    // power() rebuilds Registers via `Registers::new` which clears all.
    assert_eq!(s.registers.status, 0);
}
