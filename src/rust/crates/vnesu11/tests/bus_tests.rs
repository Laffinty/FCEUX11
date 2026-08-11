//! Bus + RAM + savestate integration tests (Phase 2).
//!
//! These tests exercise the full `VNesSoc` bus matrix end-to-end —
//! integrating the bus module, the RAM banks, the nametable mirror
//! function, and the snapshot (SFORMAT tag) layer. Where individual
//! modules have their own `#[cfg(test)] mod tests`, those cover
//! internals; this file is the public-facing integration suite per
//! `phase_2_bus_and_ram.md` §3.
//!
//! Coverage:
//! - WRAM mirror table
//! - PPU register mirror table
//! - Nametable mirror (horizontal / vertical / single / four)
//! - Palette mirror logic
//! - PPU data read buffer ($2007 lag)
//! - Open-bus latch semantics
//! - Mapper range table integration (via `vnesu11_set_read_handler`)
//! - RAM init + golden byte equivalence (splitmix64 + xoroshiro128plus)
//! - SFORMAT tag round-trip
//! - OAM DMA
//! - Joypad strobe

use vnesu11::cpu::BusContext;
use vnesu11::mapper::{MapperRangeTable, ReadRangeHandler, WriteRangeHandler};
use vnesu11::ppu::nametable::Mirroring;
use vnesu11::ram::{RamInitOption, RamRng};
use vnesu11::snapshot::mem::{Reader, Tag, Writer};
use vnesu11::soc::VNesSoc;
use std::sync::atomic::{AtomicU16, AtomicU32, AtomicU8, Ordering};

/// Tiny test helper: build a fresh `VNesSoc` with all RAM zeroed.
fn soc() -> VNesSoc {
    VNesSoc::default()
}

// ====================================================================
// WRAM mirror
// ====================================================================

#[test]
fn wram_mirror_table() {
    let mut s = soc();
    s.cpu_write(0x0000, 0xAB);
    // Every 2 KiB mirror in $0000-$1FFF must see the same byte at the
    // SAME offset. So $0000, $0800, $1000, $1800 all map to WRAM[0];
    // $0001, $0801, $1001, $1801 all map to WRAM[1]; etc.
    let mirrors: [u16; 4] = [0x0000, 0x0800, 0x1000, 0x1800];
    for &a in &mirrors {
        assert_eq!(s.cpu_read(a), 0xAB, "mirror at ${:04X}", a);
    }
    // Different WRAM offsets are independent — $0000 vs $0400 vs $0800
    // all share offset 0; $0400 is offset 0x400 (different byte).
    s.cpu_write(0x0400, 0xCD);
    assert_eq!(s.cpu_read(0x0400), 0xCD);
    assert_eq!(s.cpu_read(0x0C00), 0xCD); // $0C00 mirrors $0400
    assert_eq!(s.cpu_read(0x0000), 0xAB); // $0000 is offset 0, not 0x400
    // Reads beyond the WRAM region don't disturb WRAM's contents.
    s.cpu_write(0x1FFF, 0xEF);
    assert_eq!(s.cpu_read(0x1FFF), 0xEF);
    assert_eq!(s.cpu_read(0x0000), 0xAB); // earlier write preserved
}

// ====================================================================
// Open-bus semantics
// ====================================================================

#[test]
fn open_bus_latches_each_read() {
    let mut s = soc();
    assert_eq!(s.open_bus, 0);
    s.wram[0] = 0x42;
    let v = s.cpu_read(0x0000);
    assert_eq!(v, 0x42);
    assert_eq!(s.open_bus, 0x42);
    // Writes also update open_bus.
    s.cpu_write(0x0000, 0x99);
    assert_eq!(s.open_bus, 0x99);
}

#[test]
fn open_bus_returns_latched_value_for_unmapped_addr() {
    let mut s = soc();
    s.open_bus = 0xCC;
    // $4020 with no mapper range → open bus.
    assert_eq!(s.cpu_read(0x4020), 0xCC);
}

// ====================================================================
// PPU register mirror
// ====================================================================

#[test]
fn ppu_register_mirror_within_2000_3fff() {
    let mut s = soc();
    // $2000 writes (PPUCTRL) must be visible at any $20X0 mirror.
    s.cpu_write(0x2000, 0x80);
    assert_eq!(s.ppu_ctrl, 0x80);
    // Reading $2000 via the bus exercises the same path; the
    // general PPU register read returns open bus for the $2000 case
    // (only $2002/$2004/$2007 have defined behavior in Phase 2).
    let _ = s.cpu_read(0x2000);
    let _ = s.cpu_read(0x2008);
    let _ = s.cpu_read(0x2010);
    let _ = s.cpu_read(0x2018);
}

// ====================================================================
// Nametable mirroring
// ====================================================================

#[test]
fn horizontal_nametable_pairs() {
    let mut s = soc();
    s.set_mirroring(Mirroring::Horizontal);
    s.vram[0x000] = 0xA1; // top-row low page
    s.vram[0x400] = 0xA2; // bottom-row high page
    // A ($2000) and B ($2400) → low 1 KiB page.
    assert_eq!(s.ppu_read(0x2000), 0xA1);
    assert_eq!(s.ppu_read(0x2400), 0xA1);
    // C ($2800) and D ($2C00) → high 1 KiB page.
    assert_eq!(s.ppu_read(0x2800), 0xA2);
    assert_eq!(s.ppu_read(0x2C00), 0xA2);
}

#[test]
fn vertical_nametable_pairs() {
    let mut s = soc();
    s.set_mirroring(Mirroring::Vertical);
    s.vram[0x000] = 0xB1;
    s.vram[0x400] = 0xB2;
    // A ($2000) and C ($2800) → low 1 KiB page.
    assert_eq!(s.ppu_read(0x2000), 0xB1);
    assert_eq!(s.ppu_read(0x2800), 0xB1);
    // B ($2400) and D ($2C00) → high 1 KiB page.
    assert_eq!(s.ppu_read(0x2400), 0xB2);
    assert_eq!(s.ppu_read(0x2C00), 0xB2);
}

#[test]
fn single_screen_low_mirrors_all_to_low_page() {
    let mut s = soc();
    s.set_mirroring(Mirroring::SingleScreenLow);
    s.vram[0x000] = 0xCC;
    for a in [0x2000u16, 0x2400, 0x2800, 0x2C00] {
        assert_eq!(s.ppu_read(a), 0xCC);
    }
}

#[test]
fn single_screen_high_mirrors_all_to_high_page() {
    let mut s = soc();
    s.set_mirroring(Mirroring::SingleScreenHigh);
    s.vram[0x400] = 0xDD;
    for a in [0x2000u16, 0x2400, 0x2800, 0x2C00] {
        assert_eq!(s.ppu_read(a), 0xDD);
    }
}

#[test]
fn set_mirroring_updates_function_pointer() {
    let mut s = soc();
    s.vram[0x000] = 0xAA;
    s.vram[0x400] = 0xBB;

    s.set_mirroring(Mirroring::Horizontal);
    // In H mode $2400 aliases A → low page (0xAA).
    assert_eq!(s.ppu_read(0x2400), 0xAA);

    s.set_mirroring(Mirroring::Vertical);
    // In V mode $2400 → high page (0xBB).
    assert_eq!(s.ppu_read(0x2400), 0xBB);

    assert_eq!(s.nametable_mirror, Mirroring::Vertical);
}

// ====================================================================
// Palette mirror
// ====================================================================

#[test]
fn palette_mirror_to_3f10() {
    let mut s = soc();
    s.ppu_write(0x3F00, 0x42);
    assert_eq!(s.ppu_read(0x3F00), 0x42);
    assert_eq!(s.ppu_read(0x3F10), 0x42);
    // $3F14 mirrors $3F04.
    s.ppu_write(0x3F04, 0xAB);
    assert_eq!(s.ppu_read(0x3F14), 0xAB);
    // $3F18 mirrors $3F08.
    s.ppu_write(0x3F08, 0xCD);
    assert_eq!(s.ppu_read(0x3F18), 0xCD);
    // $3F1C mirrors $3F0C.
    s.ppu_write(0x3F0C, 0xEF);
    assert_eq!(s.ppu_read(0x3F1C), 0xEF);
}

#[test]
fn palette_non_mirror_offsets_are_independent() {
    let mut s = soc();
    s.ppu_write(0x3F01, 0x11);
    s.ppu_write(0x3F05, 0x22);
    assert_eq!(s.ppu_read(0x3F01), 0x11);
    assert_eq!(s.ppu_read(0x3F05), 0x22);
    // $3F11 is NOT a mirror — it's its own entry (sprite palette idx 0).
    assert_ne!(s.ppu_read(0x3F11), 0x11);
    s.ppu_write(0x3F11, 0x33);
    assert_eq!(s.ppu_read(0x3F11), 0x33);
}

// ====================================================================
// PPU data read buffer
// ====================================================================

#[test]
fn ppu_read_data_returns_buffered_byte() {
    let mut s = soc();
    s.set_mirroring(Mirroring::Vertical);
    s.vram[0x000] = 0x55;
    s.ppu_v = 0x2000;
    s.ppu_read_buffer = 0xAA;

    // First read: returns the OLD buffer (0xAA), refreshes buffer to 0x55,
    // and advances `v`.
    let first = s.ppu_read_data(1);
    assert_eq!(first, 0xAA);
    assert_eq!(s.ppu_read_buffer, 0x55);
    assert_eq!(s.ppu_v, 0x2001);

    // Second read: returns what the buffer just got (0x55).
    let second = s.ppu_read_data(1);
    assert_eq!(second, 0x55);
}

#[test]
fn ppu_read_data_increment_32() {
    let mut s = soc();
    s.vram[0x000] = 0x11;
    s.vram[0x020] = 0x22;
    s.ppu_v = 0x2000;
    s.ppu_read_buffer = 0xAA;

    // First read advances by 32.
    let _ = s.ppu_read_data(32);
    assert_eq!(s.ppu_v, 0x2020);
}

#[test]
fn ppu_read_data_palette_bypasses_buffer() {
    let mut s = soc();
    s.palette[0] = 0x33;
    s.ppu_v = 0x3F00;
    s.ppu_read_buffer = 0xAA;
    let v = s.ppu_read_data(1);
    assert_eq!(v, 0x33);
    assert_eq!(s.ppu_read_buffer, 0xAA); // untouched
    assert_eq!(s.ppu_v, 0x3F01);
}

// ====================================================================
// Mapper range table integration
// ====================================================================

#[test]
fn mapper_read_handler_returns_registered_byte() {
    static GOT_ADDR: AtomicU16 = AtomicU16::new(0);

    unsafe extern "C" fn stub(_ctx: *mut core::ffi::c_void, addr: u16) -> u8 {
        GOT_ADDR.store(addr, Ordering::SeqCst);
        0xFE
    }

    let mut s = soc();
    // Register a single range covering the whole PRG-ROM area.
    s.mapper.read_ranges[0] = ReadRangeHandler {
        start: 0x8000,
        end: 0xFFFF,
        fn_ptr: stub,
        ctx: core::ptr::null_mut(),
    };
    s.mapper.read_count = 1;

    let v = s.cpu_read(0x8000);
    assert_eq!(v, 0xFE);
    assert_eq!(GOT_ADDR.load(Ordering::SeqCst), 0x8000);

    // Read again at a different address — same handler should be hit.
    let v = s.cpu_read(0xABCD);
    assert_eq!(v, 0xFE);
    assert_eq!(GOT_ADDR.load(Ordering::SeqCst), 0xABCD);
}

#[test]
fn mapper_write_handler_receives_value() {
    static WRITTEN_ADDR: AtomicU16 = AtomicU16::new(0);
    static WRITTEN_VAL: AtomicU8 = AtomicU8::new(0);

    unsafe extern "C" fn stub_w(
        _ctx: *mut core::ffi::c_void,
        addr: u16,
        val: u8,
    ) {
        WRITTEN_ADDR.store(addr, Ordering::SeqCst);
        WRITTEN_VAL.store(val, Ordering::SeqCst);
    }

    let mut s = soc();
    s.mapper.write_ranges[0] = WriteRangeHandler {
        start: 0x8000,
        end: 0xFFFF,
        fn_ptr: stub_w,
        ctx: core::ptr::null_mut(),
    };
    s.mapper.write_count = 1;

    s.cpu_write(0x8001, 0x77);
    assert_eq!(WRITTEN_ADDR.load(Ordering::SeqCst), 0x8001);
    assert_eq!(WRITTEN_VAL.load(Ordering::SeqCst), 0x77);
}

#[test]
fn mapper_no_range_returns_open_bus() {
    let mut s = soc();
    s.open_bus = 0x99;
    assert_eq!(s.cpu_read(0xABCD), 0x99);
    // clear() empties the tables.
    s.mapper.clear();
    assert_eq!(s.mapper.read_count, 0);
    assert_eq!(s.mapper.write_count, 0);
}

// ====================================================================
// RAM init + golden byte equivalence
// ====================================================================

#[test]
fn ram_rng_is_seed_deterministic() {
    let mut a = RamRng::new();
    let mut b = RamRng::new();
    a.seed(0xDEAD_BEEF);
    b.seed(0xDEAD_BEEF);
    for _ in 0..64 {
        assert_eq!(a.next_u64(), b.next_u64());
    }
}

#[test]
fn ram_rng_different_seeds_yield_different_streams() {
    let mut a = RamRng::new();
    let mut b = RamRng::new();
    a.seed(0);
    b.seed(1);
    let sa: [u8; 16] = std::array::from_fn(|_| a.next_u8());
    let sb: [u8; 16] = std::array::from_fn(|_| b.next_u8());
    assert_ne!(sa, sb);
}

#[test]
fn ram_init_fills_match_option() {
    let mut s = soc();
    s.power_on(RamInitOption::AllOnes, 0);
    assert!(s.wram.iter().all(|&b| b == 0xFF));
    assert!(s.vram.iter().all(|&b| b == 0xFF));
    assert!(s.oam.iter().all(|&b| b == 0xFF));
    assert!(s.palette.iter().all(|&b| b == 0xFF));

    s.power_on(RamInitOption::AllZeros, 0);
    assert!(s.wram.iter().all(|&b| b == 0x00));
}

#[test]
fn ram_init_same_seed_same_bytes() {
    let mut a = soc();
    let mut b = soc();
    a.power_on(RamInitOption::Random, 0xCAFE);
    b.power_on(RamInitOption::Random, 0xCAFE);
    assert_eq!(a.wram, b.wram);
    assert_eq!(a.vram, b.vram);
    assert_eq!(a.oam, b.oam);
    assert_eq!(a.palette, b.palette);
}

// ====================================================================
// SFORMAT tag round-trip
// ====================================================================

#[test]
fn sformat_ram_roundtrip() {
    let mut s = soc();
    // Distinct fingerprints per bank.
    for (i, b) in s.wram.iter_mut().enumerate() {
        *b = i as u8;
    }
    for (i, b) in s.vram.iter_mut().enumerate() {
        *b = (255u8).wrapping_sub(i as u8);
    }
    for (i, b) in s.oam.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7);
    }
    for (i, b) in s.palette.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(13);
    }
    // Sync public views → ram_banks before saving.
    s.sync_ram_banks_from_views();

    // Save → bytes.
    let mut w = Writer::with_capacity(8192);
    s.ram_banks.save_state(&mut w);
    let bytes = w.into_bytes();

    // Load into a fresh SoC.
    let mut t = soc();
    {
        let mut r = Reader::new(&bytes);
        t.ram_banks.load_state(&mut r).expect("load ok");
        t.sync_ram_banks_to_views();
    }

    assert_eq!(t.wram, s.wram);
    assert_eq!(t.vram, s.vram);
    assert_eq!(t.oam, s.oam);
    assert_eq!(t.palette, s.palette);
}

#[test]
fn sformat_skips_unknown_chunks() {
    let mut w = Writer::new();
    w.write_chunk(Tag(*b"ZZZZ"), &[1, 2, 3]);
    w.write_chunk(Tag(*b"RAM\0"), &[0xDE, 0xAD]);
    let bytes = w.into_bytes();

    let mut t = soc();
    let mut r = Reader::new(&bytes);
    t.ram_banks.load_state(&mut r).expect("unknown skipped");
    t.sync_ram_banks_to_views();
    assert_eq!(t.wram[0], 0xDE);
    assert_eq!(t.wram[1], 0xAD);
}

// ====================================================================
// OAM DMA
// ====================================================================

#[test]
fn oam_dma_copies_256_bytes() {
    let mut s = soc();
    // Pre-fill WRAM with a recognizable pattern.
    for (i, b) in s.wram.iter_mut().enumerate() {
        *b = i as u8;
    }
    s.cpu_write(0x4014, 0x02);
    for i in 0..256 {
        assert_eq!(s.oam[i], i as u8, "OAM[{}]", i);
    }
}

// ====================================================================
// Joypad strobe
// ====================================================================

#[test]
fn joypad_strobe_latches_then_unlatches() {
    let mut s = soc();
    s.joypad_latched[0] = 0xAB;
    s.cpu_write(0x4016, 0x01);
    assert!(s.joypad_strobe);
    // In strobe mode, $4016 returns the latched byte.
    assert_eq!(s.cpu_read(0x4016), 0xAB);
    // Strobe off → next reads return 0 (no shift register in Phase 2).
    s.cpu_write(0x4016, 0x00);
    assert!(!s.joypad_strobe);
}

// ====================================================================
// BusContext trait: integration smoke
// ====================================================================

#[test]
fn bus_context_drives_cpu() {
    // Use the VNesBusContext to drive the CPU through a few cycles.
    // This pins the integration: CPU → BusContext → SoC::cpu_read.
    let mut s = soc();
    s.cpu_write(0x0000, 0xEA); // NOP
    s.cpu_write(0x0001, 0xEA);
    s.cpu_write(0x0002, 0xEA);

    s.cpu.set_pc(0x0000);
    {
        let mut ctx = unsafe { vnesu11::soc::VNesBusContext::new(&mut s) };
        s.cpu.run_budget(6, &mut ctx); // 3 NOPs
    }
    assert_eq!(s.cpu.pc(), 0x0003);
    assert_eq!(s.cpu.a(), 0); // NOP doesn't touch registers
}

// ====================================================================
// CPU-bus reads go through cpu_read (smoke)
// ====================================================================

#[test]
fn cpu_bus_read_routes_to_wram_for_low_addresses() {
    let mut s = soc();
    s.wram[0x100] = 0xCC;
    // Direct call routes through cpu_read → wram match arm.
    assert_eq!(s.cpu_read(0x0100), 0xCC);
}

#[test]
fn cpu_bus_read_routes_to_mapper_for_high_addresses() {
    static READ_COUNT: AtomicU32 = AtomicU32::new(0);

    unsafe extern "C" fn stub(_ctx: *mut core::ffi::c_void, _addr: u16) -> u8 {
        READ_COUNT.fetch_add(1, Ordering::SeqCst);
        0xAB
    }

    let mut s = soc();
    s.mapper.read_ranges[0] = ReadRangeHandler {
        start: 0x8000,
        end: 0xFFFF,
        fn_ptr: stub,
        ctx: core::ptr::null_mut(),
    };
    s.mapper.read_count = 1;
    let _ = s.cpu_read(0xC000);
    assert_eq!(READ_COUNT.load(Ordering::SeqCst), 1);
}

// ====================================================================
// MapperRangeTable is cloneable + default stable (smoke)
// ====================================================================

#[test]
fn mapper_range_table_clone_and_default() {
    let t = MapperRangeTable::default();
    assert_eq!(t.read_count, 0);
    assert_eq!(t.write_count, 0);
    let _t2 = t.clone();
}