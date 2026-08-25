//! Integration tests for sprite evaluation into secondary OAM.
//!
//! These tests verify the minimal Phase 1 sprite eval model:
//! 1. Only sprites whose Y range contains the current scanline are
//!    copied into secondary OAM.
//! 2. At most 8 sprites per scanline (overflow flag for the 9th+).
//! 3. Secondary OAM is reset at the start of every visible scanline
//!    (sl 0..=239 dot 0).
//!
//! The full dot-by-dot sprite eval quirks (sloppy sprite overflow
// detection, priority rotation, sprite 0 hit dot window) are
//! deferred to Phase 4. See `docs/plans/v2.1_ppu_rust_refactor_plan.md` §6.4.

use fceux11_ppu::{FlatBus, PpuState, Registers, frame::tick_to, mask_bits};

/// Lay a single 4-byte sprite entry into primary OAM at slot `i`.
fn set_sprite(s: &mut PpuState, i: usize, y: u8, tile: u8, attr: u8, x: u8) {
    s.oam[i * 4] = y;
    s.oam[i * 4 + 1] = tile;
    s.oam[i * 4 + 2] = attr;
    s.oam[i * 4 + 3] = x;
}

#[test]
fn eval_picks_only_in_range_sprites() {
    let mut s = PpuState::new();
    s.scanline = 100;
    // Y values: 96 (in), 110 (out), 100 (in), 50 (out), 95 (in).
    set_sprite(&mut s, 0, 96, 0x10, 0x00, 10);
    set_sprite(&mut s, 1, 110, 0x11, 0x00, 20);
    set_sprite(&mut s, 2, 100, 0x12, 0x00, 30);
    set_sprite(&mut s, 3, 50, 0x13, 0x00, 40);
    set_sprite(&mut s, 4, 95, 0x14, 0x00, 50);

    s.eval_sprites(8);
    assert_eq!(s.secondary_oam_count, 3);
    // Sprite 0 (Y=96) at offset 0.
    assert_eq!(s.secondary_oam[0], 96);
    assert_eq!(s.secondary_oam[1], 0x10);
    // Sprite 2 (Y=100) at offset 4.
    assert_eq!(s.secondary_oam[4], 100);
    // Sprite 4 (Y=95) at offset 8.
    assert_eq!(s.secondary_oam[8], 95);
}

#[test]
fn eval_caps_at_8_and_sets_overflow() {
    let mut s = PpuState::new();
    s.scanline = 50;
    for i in 0..10 {
        set_sprite(&mut s, i, 48, i as u8, 0x00, 0);
    }
    s.eval_sprites(8);
    assert_eq!(s.secondary_oam_count, 8);
    assert!(
        s.sprite_overflow,
        "9th+ in-range sprite must trigger overflow"
    );
    assert!(s.registers.status & 0x20 != 0, "overflow bit set in PPU[2]");
}

#[test]
fn eval_resets_secondary_oam_each_scanline() {
    // Verify that tick_to(sl N, 0) re-runs eval and clears any prior
    // content. Phase 1 collapses the per-dot eval into sl N dot 0.
    let mut s = PpuState::new();
    s.registers.write_mask(1 << mask_bits::SHOW_BG); // rendering on
    // Lay 3 sprites for sl 100.
    set_sprite(&mut s, 0, 96, 0x10, 0x00, 0);
    set_sprite(&mut s, 1, 100, 0x11, 0x00, 0);
    set_sprite(&mut s, 2, 95, 0x12, 0x00, 0);
    let mut bus = FlatBus::new();
    let _ = tick_to(&mut s, &mut bus, 100, 0);
    assert_eq!(s.secondary_oam_count, 3, "sl 100 dot 0 eval'd 3 sprites");
    assert!(!s.sprite_overflow);

    // Now move to sl 200 (different scanline) — sprite 0 (Y=96) is out
    // of range, others are also out for sl 200.
    let _ = tick_to(&mut s, &mut bus, 200, 0);
    assert_eq!(
        s.secondary_oam_count, 0,
        "sl 200 dot 0 eval clears prior content"
    );
    assert!(s.secondary_oam.iter().all(|&b| b == 0));
}

#[test]
fn sprite_y_ff_is_treated_as_off_scanline() {
    // $FF Y is the legacy "off scanline" sentinel.
    let mut s = PpuState::new();
    s.scanline = 100;
    set_sprite(&mut s, 0, 0xFF, 0x10, 0x00, 0);
    s.eval_sprites(8);
    assert_eq!(s.secondary_oam_count, 0);
    assert!(!s.sprite_overflow);
}

#[test]
fn sprite_height_16_doubles_y_range() {
    // With sprite height=16, a sprite at Y=95 should match scanlines
    // 95..=110 inclusive.
    let mut s = PpuState::new();
    s.scanline = 110;
    set_sprite(&mut s, 0, 95, 0x10, 0x00, 0);
    s.eval_sprites(16);
    assert_eq!(s.secondary_oam_count, 1);

    s.scanline = 111;
    s.eval_sprites(16);
    assert_eq!(
        s.secondary_oam_count, 0,
        "scanline 111 is past the 16-line range"
    );
}

#[test]
fn registers_helpers_touch_status_bits() {
    // Ensure the Registers accessors used by eval land in PPU[2] bits 5
    // (overflow) and 6 (sprite 0 hit) without disturbing bits 7 (VBL).
    let mut r = Registers::new();
    r.set_vbl_flag();
    r.set_sprite_overflow();
    r.set_sprite0_hit();
    assert_ne!(r.status & 0x80, 0, "VBL bit preserved");
    assert_ne!(r.status & 0x20, 0, "overflow bit set");
    assert_ne!(r.status & 0x40, 0, "sprite0 bit set");
    r.clear_sprite_overflow();
    assert_eq!(r.status & 0x20, 0);
    assert_ne!(r.status & 0x40, 0, "sprite0 still set");
}
