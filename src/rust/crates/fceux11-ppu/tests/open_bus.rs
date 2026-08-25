//! Integration tests for the open-bus / VRAM-buffer behaviour of
//! `$2007` reads and writes.
//!
//! Per nesdev: every non-palette `$2007` access reads through a
//! buffered byte. The first read returns the *prior* buffer (or
//! whatever was last clocked in), and updates the buffer with the
//! fresh bus value. Writes update the buffer with the written value.
//! Palette reads do not update the buffer; their return is real bus
//! data (with palette mirroring collapsing to the canonical 0x3F00..0x3F0F
//! range).

use fceux11_ppu::{FlatBus, PpuBus, PpuState, Registers};

#[test]
fn read_data_returns_buffered_byte_then_real_value() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    bus.write(0x2000, 0xAB);
    s.registers.v = 0x2000;
    let first = s.registers.read_data(&mut bus, s.registers.ctrl);
    assert_eq!(first, 0, "first read returns prior buffer (0)");
    let second = s.registers.read_data(&mut bus, s.registers.ctrl);
    assert_eq!(second, 0xAB, "second read returns what was buffered");
}

#[test]
fn write_data_updates_buffer_for_non_palette() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.v = 0x2000;
    s.registers.write_data(&mut bus, s.registers.ctrl, 0x55);
    assert_eq!(bus.read(0x2000), 0x55, "value reaches the bus");
    assert_eq!(s.registers.vram_buffer, 0x55, "buffer updated to write");
    // Next read returns the buffer.
    let r = s.registers.read_data(&mut bus, s.registers.ctrl);
    assert_eq!(r, 0x55);
}

#[test]
fn palette_read_does_not_update_buffer() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    bus.write(0x3F00, 0x12);
    s.registers.vram_buffer = 0xCD; // sentinel from prior frame
    s.registers.v = 0x3F00;
    let v = s.registers.read_data(&mut bus, s.registers.ctrl);
    assert_eq!(v, 0x12, "palette returns real bus value");
    assert_eq!(
        s.registers.vram_buffer, 0xCD,
        "palette read must NOT touch the buffer"
    );
}

#[test]
fn palette_mirrors_collapse() {
    // $3F00 / $3F04 / $3F08 / $3F0C — same palette entry.
    // $3F10 / $3F14 / $3F18 / $3F1C — alias to $3F00 / $3F04 / $3F08 / $3F0C.
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    bus.write(0x3F00, 0x42);
    bus.write(0x3F04, 0x43);
    bus.write(0x3F10, 0x99); // write goes to canonical 0x3F00 = 0x42's mirror? no — read aliases but writes...

    // The behavior of palette *writes* in the real PPU is also mirrored,
    // but our read_data consults the bus at the *mirrored* address.
    // Read from $3F10 should return whatever was at $3F00 (alias).
    s.registers.v = 0x3F10;
    let v = s.registers.read_data(&mut bus, s.registers.ctrl);
    assert_eq!(v, 0x42, "$3F10 aliases $3F00");
}

#[test]
fn vram_buffer_initially_zero_after_power() {
    let s = PpuState::new();
    assert_eq!(s.registers.vram_buffer, 0);
}

#[test]
fn read_data_increments_v_by_one_by_default() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.v = 0x2000;
    s.registers.read_data(&mut bus, s.registers.ctrl);
    assert_eq!(s.registers.v, 0x2001);
}

#[test]
fn read_data_increments_v_by_thirty_two_when_ctrl_bit2_set() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.v = 0x2000;
    // ctrl bit 2 = VRAM_INCREMENT.
    let ctrl = 1 << 2;
    s.registers.read_data(&mut bus, ctrl);
    assert_eq!(s.registers.v, 0x2020, "v += 32 when ctrl bit 2 is set");
}

#[test]
fn write_data_also_increments_v() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.v = 0x2050;
    s.registers.write_data(&mut bus, s.registers.ctrl, 0x00);
    assert_eq!(s.registers.v, 0x2051);
}

#[test]
fn registers_new_initializes_buffer_to_zero() {
    let r = Registers::new();
    assert_eq!(r.vram_buffer, 0);
}
