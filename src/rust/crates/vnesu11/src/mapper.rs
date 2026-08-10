//! Mapper adapter — per-range handler registration.
//!
//! **Audit S4**: Real mapper mechanism is `SetReadHandler(start, end, fn)`,
//! NOT a single `cpu_read` vtable. MMC3 alone registers 5+ ranges.
//!
//! The range table is searched linearly (typically 4-16 entries), hot in
//! cache; benchmark target ≤ 10ns per read.

use core::ffi::c_void;

/// Maximum number of registered ranges per direction (read/write).
/// MMC3 needs 5+, so 64 is generous (vs. upstream 1 MiB ARead[] table).
pub const MAX_RANGES: usize = 64;

/// Per-range read handler. Registered via `vnesu11_set_read_handler`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ReadRangeHandler {
    pub start: u16,
    pub end: u16,
    /// `fn(ctx, addr) -> byte` — equivalent to upstream `readfunc fn`.
    pub fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,
    pub ctx: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct WriteRangeHandler {
    pub start: u16,
    pub end: u16,
    pub fn_ptr: unsafe extern "C" fn(*mut c_void, u16, u8),
    pub ctx: *mut c_void,
}

/// Internal range table for mapper regions.
#[derive(Clone)]
pub struct MapperRangeTable {
    pub read_ranges: [ReadRangeHandler; MAX_RANGES],
    pub write_ranges: [WriteRangeHandler; MAX_RANGES],
    pub read_count: usize,
    pub write_count: usize,
}

impl Default for MapperRangeTable {
    fn default() -> Self {
        // Phase 0: empty tables. The function-pointer slots are never
        // called while `count == 0` (linear scan stops before), so we use
        // a sentinel function that never executes. This avoids the
        // `core::mem::zeroed()` UB warning on function-pointer fields.
        unsafe extern "C" fn never_called(_ctx: *mut c_void, _addr: u16) -> u8 { 0 }
        unsafe extern "C" fn never_called_w(_ctx: *mut c_void, _addr: u16, _val: u8) {}
        let sentinel_r: unsafe extern "C" fn(*mut c_void, u16) -> u8 = never_called;
        let sentinel_w: unsafe extern "C" fn(*mut c_void, u16, u8) = never_called_w;

        let read_entry = ReadRangeHandler {
            start: 0, end: 0,
            fn_ptr: sentinel_r,
            ctx: core::ptr::null_mut(),
        };
        let write_entry = WriteRangeHandler {
            start: 0, end: 0,
            fn_ptr: sentinel_w,
            ctx: core::ptr::null_mut(),
        };

        Self {
            read_ranges: [read_entry; MAX_RANGES],
            write_ranges: [write_entry; MAX_RANGES],
            read_count: 0,
            write_count: 0,
        }
    }
}

impl MapperRangeTable {
    pub fn clear(&mut self) {
        self.read_count = 0;
        self.write_count = 0;
    }

    /// Linear-scan read. Returns `None` if no range matches (open bus).
    #[inline(always)]
    pub fn read(&self, addr: u16) -> Option<u8> {
        let n = self.read_count;
        for i in 0..n {
            let h = &self.read_ranges[i];
            if addr >= h.start && addr <= h.end {
                return Some(unsafe { (h.fn_ptr)(h.ctx, addr) });
            }
        }
        None
    }

    #[inline(always)]
    pub fn write(&mut self, addr: u16, val: u8) -> bool {
        let n = self.write_count;
        for i in 0..n {
            let h = &self.write_ranges[i];
            if addr >= h.start && addr <= h.end {
                unsafe { (h.fn_ptr)(h.ctx, addr, val); }
                return true;
            }
        }
        false
    }
}

/// Meta vtable for non-range mapper operations (mirroring, audio, IRQ,
/// savestate). Distinct from the per-range handler table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MapperMetaVtable {
    pub mirroring:   unsafe extern "C" fn(*mut c_void) -> u8,
    pub fill_audio:  unsafe extern "C" fn(*mut c_void, *mut i16, usize),
    pub tick_irq:    unsafe extern "C" fn(*mut c_void, *mut bool),
    pub save_state:  unsafe extern "C" fn(*mut c_void, *mut u8, usize, *mut usize) -> i32,
    pub load_state:  unsafe extern "C" fn(*mut c_void, *const u8, usize) -> i32,
}

impl Default for MapperMetaVtable {
    fn default() -> Self {
        // Phase 0: a sentinel vtable whose function pointers are never
        // invoked (meta ops are gated by `mapper_meta.is_some()` in higher
        // layers). Avoids the `core::mem::zeroed()` UB warning on fn
        // pointer fields.
        unsafe extern "C" fn noop_mirror(_ctx: *mut c_void) -> u8 { 0 }
        unsafe extern "C" fn noop_audio(_ctx: *mut c_void, _out: *mut i16, _n: usize) {}
        unsafe extern "C" fn noop_irq(_ctx: *mut c_void, _out: *mut bool) {}
        unsafe extern "C" fn noop_save(_ctx: *mut c_void, _out: *mut u8, _cap: usize, _w: *mut usize) -> i32 { -1 }
        unsafe extern "C" fn noop_load(_ctx: *mut c_void, _in: *const u8, _len: usize) -> i32 { -1 }
        Self {
            mirroring: noop_mirror,
            fill_audio: noop_audio,
            tick_irq: noop_irq,
            save_state: noop_save,
            load_state: noop_load,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU16, AtomicU8, Ordering};

    static READ_GOT_ADDR: AtomicU16 = AtomicU16::new(0);
    static READ_GOT_VAL: AtomicU8 = AtomicU8::new(0);

    unsafe extern "C" fn stub_read(_ctx: *mut c_void, addr: u16) -> u8 {
        READ_GOT_ADDR.store(addr, Ordering::SeqCst);
        0xAB
    }
    unsafe extern "C" fn stub_read2(_ctx: *mut c_void, addr: u16) -> u8 {
        READ_GOT_VAL.store(addr as u8, Ordering::SeqCst);
        0xCD
    }

    #[test]
    fn no_handler_returns_none() {
        let table = MapperRangeTable::default();
        assert!(table.read(0x8000).is_none());
    }

    #[test]
    fn hit_first_range() {
        let mut table = MapperRangeTable::default();
        table.read_ranges[0] = ReadRangeHandler {
            start: 0x8000, end: 0xFFFF,
            fn_ptr: stub_read, ctx: core::ptr::null_mut(),
        };
        table.read_count = 1;
        let v = table.read(0x8000);
        assert_eq!(v, Some(0xAB));
        assert_eq!(READ_GOT_ADDR.load(Ordering::SeqCst), 0x8000);
    }

    #[test]
    fn overlapping_ranges_first_wins() {
        let mut table = MapperRangeTable::default();
        table.read_ranges[0] = ReadRangeHandler {
            start: 0x8000, end: 0xBFFF,
            fn_ptr: stub_read, ctx: core::ptr::null_mut(),
        };
        table.read_ranges[1] = ReadRangeHandler {
            start: 0x8000, end: 0xFFFF,
            fn_ptr: stub_read2, ctx: core::ptr::null_mut(),
        };
        table.read_count = 2;
        assert_eq!(table.read(0x9000), Some(0xAB)); // first
        assert_eq!(table.read(0xC000), Some(0xCD)); // second
    }

    #[test]
    fn clear_resets() {
        let mut table = MapperRangeTable::default();
        table.read_count = 5;
        table.write_count = 3;
        table.clear();
        assert_eq!(table.read_count, 0);
        assert_eq!(table.write_count, 0);
    }
}
