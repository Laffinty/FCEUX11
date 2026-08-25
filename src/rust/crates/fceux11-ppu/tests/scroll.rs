//! Integration tests for `$2005`/`$2006` double-write scroll/addr latches.
//!
//! These tests pin down the v / t scroll counter semantics that the
//! renderer's BG fetch (sl 0..=239 dot 257) and the even/odd toggle
//! ultimately rely on. They run as integration tests (separate crate
//! compilation unit) so the public API surface — [`PpuState`],
//! [`Registers`] — is exercised end-to-end.

use fceux11_ppu::{FlatBus, PpuState};

#[test]
fn scroll_first_write_does_not_change_v() {
    // Per nesdev: only `t` changes on the first $2005 write; `v` is
    // only loaded by the sl 0/257 copy events.
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.write_scroll(0xA5); // coarse_x=0x14, fine_x=5
    assert_eq!(s.registers.v, 0, "v should remain untouched on first $2005");
    assert_eq!(s.registers.t & 0x001F, 0x14);
    assert_eq!(s.registers.fine_x, 5);
    assert!(s.registers.write_toggle);
    let _ = &mut bus;
}

#[test]
fn scroll_second_write_updates_fine_y_and_coarse_y() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.write_scroll(0xFF); // first: arbitrary
    s.registers.write_scroll(0x7B); // second: fine_y=3, coarse_y=0x0F
    let _ = &mut bus;

    // t layout: yyy NN YYYYY XXXXX
    //   fine_y   = bits 14..=12
    //   nametable= bit 11, 10
    //   coarse_y = bits 9..=5
    assert_eq!((s.registers.t >> 12) & 0x07, 3, "fine_y = val & 0x07");
    assert_eq!((s.registers.t >> 5) & 0x1F, 0x0F, "coarse_y = val >> 3");
    assert!(!s.registers.write_toggle);
}

#[test]
fn scroll_fine_x_survives_second_write() {
    // First write sets fine_x; second write must NOT clobber it.
    let mut s = PpuState::new();
    s.registers.write_scroll(0x07); // fine_x = 7, coarse_x = 0
    assert_eq!(s.registers.fine_x, 7);
    s.registers.write_scroll(0x00); // fine_y/coarse_y, leave fine_x alone
    assert_eq!(s.registers.fine_x, 7, "fine_x must survive second write");
}

#[test]
fn addr_double_write_loads_v_on_second_write() {
    let mut s = PpuState::new();
    s.registers.write_addr(0x20);
    assert_eq!((s.registers.t >> 8) & 0x3F, 0x20);
    assert_eq!(s.registers.v, 0, "v only loaded on second $2006 write");
    s.registers.write_addr(0x00);
    assert_eq!(s.registers.v, 0x2000);
    assert_eq!(s.registers.v, s.registers.t);
}

#[test]
fn addr_only_uses_low_six_bits_on_first_write() {
    // $2006 first write only takes bits 5..=0 of the byte into bits 13..=8.
    let mut s = PpuState::new();
    s.registers.write_addr(0xFF); // bits 7,6 ignored on first write
    assert_eq!(
        (s.registers.t >> 8) & 0x3F,
        0x3F,
        "low 6 bits → bits 13..=8"
    );
    assert_eq!((s.registers.t >> 14) & 0x03, 0, "high 2 bits dropped");
}

#[test]
fn coarse_x_rollover_flips_nametable() {
    let mut s = PpuState::new();
    // Build a v value with coarse_x = 31 (just before rollover) and
    // nametable_x = 0.
    s.registers.v = 0x1F;
    s.registers.increment_coarse_x();
    assert_eq!(s.registers.v & 0x001F, 0, "coarse_x rolled over to 0");
    // nametable_x should be flipped to 1 (was 0).
    assert_ne!(s.registers.v & 0x0400, 0, "nametable_x flipped to 1");
}

#[test]
fn fine_y_increment_carries_into_coarse_y_at_eight() {
    // Start with fine_y=7 (3 bits). increment_fine_y → fine_y wraps to 0,
    // coarse_y+1.
    let mut s = PpuState::new();
    // Build v with fine_y=7, coarse_y=0, nametable_y=0.
    s.registers.v = 7 << 12;
    s.registers.increment_fine_y();
    assert_eq!((s.registers.v >> 12) & 0x07, 0, "fine_y wrapped to 0");
    assert_eq!((s.registers.v >> 5) & 0x1F, 1, "coarse_y incremented");
}

#[test]
fn fine_y_carries_into_nametable_at_coarse_y_30() {
    // Per nesdev PPU_scrolling: when fine_y overflows at fine_y=7 and
    // coarse_y is 29 (i.e. the last visible scanline band), the
    // overflow resets coarse_y to 0 and flips nametable_y in the same
    // increment. coarse_y=30 is never observed externally.
    let mut s = PpuState::new();
    s.registers.v = (29u16 << 5) | (7u16 << 12); // coarse_y=29, fine_y=7
    s.registers.increment_fine_y();
    assert_eq!((s.registers.v >> 12) & 0x07, 0, "fine_y wrapped to 0");
    assert_eq!(
        (s.registers.v >> 5) & 0x1F,
        0,
        "coarse_y wraps 29 → 0 in one tick (30 is internal)"
    );
    assert_eq!(
        (s.registers.v >> 11) & 0x01,
        1,
        "nametable_y flipped to 1 in the same tick"
    );

    // Sanity: starting from coarse_y=0, nametable_y=1, after a full
    // fine_y cycle (8 increments) the state lands back at the same
    // coarse_y and fine_y, nametable_y unchanged.
    s.registers.v = 1u16 << 11;
    for _ in 0..8 {
        s.registers.increment_fine_y();
    }
    assert_eq!((s.registers.v >> 12) & 0x07, 0);
    assert_eq!((s.registers.v >> 5) & 0x1F, 1, "coarse_y carried to 1");
    assert_eq!((s.registers.v >> 11) & 0x01, 1, "nametable_y unchanged");
}

#[test]
fn copy_horizontal_copies_t_h_bits_only() {
    let mut s = PpuState::new();
    // Build distinct t and v values: t has coarse_x set; v has coarse_y set.
    s.registers.t = (3 << 12) | (15 << 5); // fine_y=3, coarse_y=15
    s.registers.v = (5 << 12) | (10 << 5) | 0x07; // fine_y=5, coarse_y=10, coarse_x=7
    s.registers.copy_horizontal();
    // After copy: v's coarse_x and nametable_x come from t; everything
    // else is unchanged.
    assert_eq!(s.registers.v & 0x001F, 0, "coarse_x = t.coarse_x");
    assert_eq!(s.registers.v & 0x0400, 0, "nametable_x = t.nametable_x");
    // Vertical bits preserved.
    assert_eq!((s.registers.v >> 12) & 0x07, 5, "fine_y preserved");
    assert_eq!((s.registers.v >> 5) & 0x1F, 10, "coarse_y preserved");
}

#[test]
fn copy_vertical_copies_t_v_bits_only() {
    let mut s = PpuState::new();
    s.registers.t = (5 << 12) | (10 << 5); // fine_y=5, coarse_y=10
    s.registers.v = (3 << 12) | (7 << 5) | 0x1F | 0x0400; // v's horizontal bits set
    s.registers.copy_vertical();
    // After copy: v's vertical bits come from t; horizontal bits preserved.
    assert_eq!((s.registers.v >> 12) & 0x07, 5);
    assert_eq!((s.registers.v >> 5) & 0x1F, 10);
    // Horizontal bits preserved.
    assert_eq!(s.registers.v & 0x001F, 0x1F, "coarse_x preserved");
    assert_eq!(s.registers.v & 0x0400, 0x0400, "nametable_x preserved");
}
