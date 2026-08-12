//! Mapper adapter integration tests (Phase 5 stage 1).
//!
//! Per `docs/wip_2.0_plan/phase_5_mapper_adapter.md` §3.1:
//!
//! ```text
//! crates/vnesu11/tests/mapper_tests.rs
//! ├── test_null_mapper          // 无 handler 注册时读 $8000 返回 open bus
//! ├── test_single_range         // 注册一个区间，读命中/未命中
//! ├── test_overlapping_ranges   // 重叠区间按注册顺序优先
//! ├── test_range_capacity       // 超 MAX_RANGES 返回错误
//! └── test_clear_handlers       // 清空后再读回 open bus
//! ```
//!
//! Also covers the FFI registration surface (`vnesu11_set_read_handler` /
//! `vnesu11_set_write_handler` / `vnesu11_attach_mapper_meta`) with null
//! safety + capacity return codes.

use vnesu11::mapper::{MapperMetaVtable, MapperRangeTable, ReadRangeHandler, WriteRangeHandler};
use vnesu11::soc::VNesSoc;
use std::sync::atomic::{AtomicU16, AtomicU8, Ordering};

/// Tiny helper: build a fresh `VNesSoc`.
fn soc() -> VNesSoc {
    VNesSoc::default()
}

// ====================================================================
// MapperRangeTable — direct (table-level) tests
// ====================================================================

#[test]
fn test_null_mapper_reads_open_bus() {
    let mut s = soc();
    s.open_bus = 0x42;
    // No handler registered → $8000 falls through to open bus.
    assert_eq!(s.cpu_read(0x8000), 0x42);
    // Writes to unmapped region are dropped, but they latch the bus
    // (the value written becomes the open-bus value for later reads).
    s.cpu_write(0x8000, 0xAA);
    assert_eq!(s.cpu_read(0x8000), 0xAA, "write latches open bus");
}

#[test]
fn test_single_range_hit_and_miss() {
    static GOT: AtomicU16 = AtomicU16::new(0);
    unsafe extern "C" fn stub(_ctx: *mut core::ffi::c_void, addr: u16) -> u8 {
        GOT.store(addr, Ordering::SeqCst);
        0xAB
    }
    let mut s = soc();
    s.mapper.read_ranges[0] = ReadRangeHandler {
        start: 0x8000,
        end: 0xBFFF,
        fn_ptr: stub,
        ctx: core::ptr::null_mut(),
    };
    s.mapper.read_count = 1;

    // Inside the range → handler fires.
    assert_eq!(s.cpu_read(0x8000), 0xAB);
    assert_eq!(s.cpu_read(0xBFFF), 0xAB);
    assert_eq!(GOT.load(Ordering::SeqCst), 0xBFFF);
    // Outside the range → open bus.
    s.open_bus = 0x77;
    assert_eq!(s.cpu_read(0xC000), 0x77);
}

#[test]
fn test_overlapping_ranges_first_registered_wins() {
    static FIRST: AtomicU8 = AtomicU8::new(0);
    static SECOND: AtomicU8 = AtomicU8::new(0);
    unsafe extern "C" fn first(_ctx: *mut core::ffi::c_void, _addr: u16) -> u8 {
        FIRST.store(1, Ordering::SeqCst);
        0x11
    }
    unsafe extern "C" fn second(_ctx: *mut core::ffi::c_void, _addr: u16) -> u8 {
        SECOND.store(1, Ordering::SeqCst);
        0x22
    }
    let mut s = soc();
    // Two overlapping ranges — registration order decides.
    s.mapper.read_ranges[0] = ReadRangeHandler {
        start: 0x8000, end: 0xFFFF, fn_ptr: first, ctx: core::ptr::null_mut(),
    };
    s.mapper.read_ranges[1] = ReadRangeHandler {
        start: 0x9000, end: 0xAFFF, fn_ptr: second, ctx: core::ptr::null_mut(),
    };
    s.mapper.read_count = 2;

    assert_eq!(s.cpu_read(0x8500), 0x11, "first range wins");
    assert_eq!(FIRST.load(Ordering::SeqCst), 1);
    assert_eq!(SECOND.load(Ordering::SeqCst), 0, "second must not fire");
    // Outside both → open bus.
    s.open_bus = 0x55;
    assert_eq!(s.cpu_read(0x7000), 0x55);
}

#[test]
fn test_clear_handlers_restores_open_bus() {
    unsafe extern "C" fn stub(_ctx: *mut core::ffi::c_void, _addr: u16) -> u8 { 0xAB }
    unsafe extern "C" fn stub_w(_ctx: *mut core::ffi::c_void, _addr: u16, _val: u8) {}
    let mut s = soc();
    s.mapper.read_ranges[0] = ReadRangeHandler {
        start: 0x8000, end: 0xFFFF, fn_ptr: stub, ctx: core::ptr::null_mut(),
    };
    s.mapper.write_ranges[0] = WriteRangeHandler {
        start: 0x8000, end: 0xFFFF, fn_ptr: stub_w, ctx: core::ptr::null_mut(),
    };
    s.mapper.read_count = 1;
    s.mapper.write_count = 1;

    assert_eq!(s.cpu_read(0x8000), 0xAB);
    s.mapper.clear();
    assert_eq!(s.mapper.read_count, 0);
    assert_eq!(s.mapper.write_count, 0);
    s.open_bus = 0x99;
    assert_eq!(s.cpu_read(0x8000), 0x99);
    // The write is dropped (no handler), but it latches the open bus.
    s.cpu_write(0x8000, 0x01);
    assert_eq!(s.cpu_read(0x8000), 0x01, "unmapped write latches open bus");
}

#[test]
fn test_write_range_receives_addr_and_value() {
    static W_ADDR: AtomicU16 = AtomicU16::new(0);
    static W_VAL: AtomicU8 = AtomicU8::new(0);
    unsafe extern "C" fn stub_w(
        _ctx: *mut core::ffi::c_void,
        addr: u16,
        val: u8,
    ) {
        W_ADDR.store(addr, Ordering::SeqCst);
        W_VAL.store(val, Ordering::SeqCst);
    }
    let mut s = soc();
    s.mapper.write_ranges[0] = WriteRangeHandler {
        start: 0x8000,
        end: 0xFFFF,
        fn_ptr: stub_w,
        ctx: core::ptr::null_mut(),
    };
    s.mapper.write_count = 1;
    s.cpu_write(0x8001, 0xCD);
    assert_eq!(W_ADDR.load(Ordering::SeqCst), 0x8001);
    assert_eq!(W_VAL.load(Ordering::SeqCst), 0xCD);
}

#[test]
fn test_chr_read_unregistered_returns_zero() {
    // PPU-side CHR reads return 0 until a CHR page is registered via
    // the mapper adapter (Phase 6: `vnesu11_chr_set_page`).
    let s = soc();
    assert_eq!(s.ppu_read(0x0000), 0);
    assert_eq!(s.ppu_read(0x1FFF), 0);
}

// ====================================================================
// FFI registration surface
// ====================================================================

use vnesu11::ffi::{
    vnesu11_attach_mapper_meta, vnesu11_clear_mapper_handlers, vnesu11_create,
    vnesu11_destroy, vnesu11_set_read_handler, vnesu11_set_write_handler,
};

unsafe extern "C" fn ffi_read(_ctx: *mut core::ffi::c_void, addr: u16) -> u8 {
    let _ = addr;
    0xFE
}
unsafe extern "C" fn ffi_write(_ctx: *mut core::ffi::c_void, _addr: u16, _val: u8) {}

/// Register via FFI, then confirm the SoC's table routes reads.
#[test]
fn test_ffi_set_read_handler_routes_bus() {
    let soc = unsafe { vnesu11_create() };
    assert!(!soc.is_null());
    let rc = unsafe {
        vnesu11_set_read_handler(soc, 0x8000, 0xFFFF, ffi_read, core::ptr::null_mut())
    };
    assert_eq!(rc, 0, "registration must succeed");
    // Read through the FFI-owned SoC.
    let v = unsafe { (*soc).0.as_mut().unwrap() }.cpu_read(0x8000);
    assert_eq!(v, 0xFE);
    unsafe { vnesu11_destroy(soc); }
}

/// FFI null-pointer handling: `-1` for a null SoC.
#[test]
fn test_ffi_null_soc_returns_minus_one() {
    let rc = unsafe {
        vnesu11_set_read_handler(core::ptr::null_mut(), 0, 0, ffi_read, core::ptr::null_mut())
    };
    assert_eq!(rc, -1);
    let rc = unsafe {
        vnesu11_set_write_handler(core::ptr::null_mut(), 0, 0, ffi_write, core::ptr::null_mut())
    };
    assert_eq!(rc, -1);
    unsafe { vnesu11_clear_mapper_handlers(core::ptr::null_mut()); } // no-op, no crash
}

/// Exceeding MAX_RANGES returns `-2` (capacity).
#[test]
fn test_ffi_range_capacity_returns_minus_two() {
    let soc = unsafe { vnesu11_create() };
    assert!(!soc.is_null());
    // Fill the read table to capacity.
    for i in 0..vnesu11::mapper::MAX_RANGES as u16 {
        let rc = unsafe { vnesu11_set_read_handler(soc, 0x8000 + i, 0x8000 + i, ffi_read, core::ptr::null_mut()) };
        assert_eq!(rc, 0, "registration {} must fit", i);
    }
    // One more → capacity error.
    let rc = unsafe { vnesu11_set_read_handler(soc, 0xFFFF, 0xFFFF, ffi_read, core::ptr::null_mut()) };
    assert_eq!(rc, -2, "capacity exceeded must return -2");
    unsafe { vnesu11_destroy(soc); }
}

/// clear_mapper_handlers empties both tables.
#[test]
fn test_ffi_clear_mapper_handlers() {
    let soc = unsafe { vnesu11_create() };
    assert!(!soc.is_null());
    unsafe { vnesu11_set_read_handler(soc, 0x8000, 0xFFFF, ffi_read, core::ptr::null_mut()); }
    unsafe { vnesu11_set_write_handler(soc, 0x8000, 0xFFFF, ffi_write, core::ptr::null_mut()); }
    let soc_ref = unsafe { (*soc).0.as_mut().unwrap() };
    assert_eq!(soc_ref.mapper.read_count, 1);
    assert_eq!(soc_ref.mapper.write_count, 1);
    unsafe { vnesu11_clear_mapper_handlers(soc); }
    let soc_ref = unsafe { (*soc).0.as_mut().unwrap() };
    assert_eq!(soc_ref.mapper.read_count, 0);
    assert_eq!(soc_ref.mapper.write_count, 0);
    unsafe { vnesu11_destroy(soc); }
}

/// `vnesu11_attach_mapper_meta` stores the meta vtable; a null vtable
/// is rejected with `-2`.
#[test]
fn test_ffi_attach_mapper_meta() {
    unsafe extern "C" fn mirror(_ctx: *mut core::ffi::c_void) -> u8 { 1 }
    unsafe extern "C" fn audio(_ctx: *mut core::ffi::c_void, _out: *mut i16, _n: usize) {}
    unsafe extern "C" fn tick_irq(_ctx: *mut core::ffi::c_void, _out: *mut bool) {}
    unsafe extern "C" fn save(
        _ctx: *mut core::ffi::c_void, _out: *mut u8, _cap: usize, _w: *mut usize,
    ) -> i32 { 0 }
    unsafe extern "C" fn load(
        _ctx: *mut core::ffi::c_void, _in: *const u8, _len: usize,
    ) -> i32 { 0 }

    let vtable = MapperMetaVtable {
        mirroring: mirror,
        fill_audio: audio,
        tick_irq: tick_irq,
        save_state: save,
        load_state: load,
    };

    let soc = unsafe { vnesu11_create() };
    assert!(!soc.is_null());
    let rc = unsafe { vnesu11_attach_mapper_meta(soc, core::ptr::null_mut(), &vtable) };
    assert_eq!(rc, 0);
    let soc_ref = unsafe { (*soc).0.as_mut().unwrap() };
    assert!(soc_ref.mapper_meta.is_some());
    // Null vtable → -2.
    let rc = unsafe { vnesu11_attach_mapper_meta(soc, core::ptr::null_mut(), core::ptr::null()) };
    assert_eq!(rc, -2);
    // Null SoC → -1.
    let rc = unsafe { vnesu11_attach_mapper_meta(core::ptr::null_mut(), core::ptr::null_mut(), &vtable) };
    assert_eq!(rc, -1);
    unsafe { vnesu11_destroy(soc); }
}

/// `MapperRangeTable` clone + default stability (FFI consumers hold the
/// table across LoadGame boundaries).
#[test]
fn test_mapper_range_table_clone_stable() {
    let t = MapperRangeTable::default();
    assert_eq!(t.read_count, 0);
    assert_eq!(t.write_count, 0);
    let t2 = t.clone();
    assert_eq!(t2.read_count, 0);
}
