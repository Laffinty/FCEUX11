//! FFI surface for vNESU11 — extern "C" stubs (Phase 0).
//!
//! These functions are exported with `#[unsafe(no_mangle)]` (Rust 2024
//! convention) so C++ can call them once `vnesu11.lib` is linked. Phase 0
//! ships them as no-ops or minimal stubs sufficient for `VNESU11_CORE=ON`
//! link + startup. Real logic arrives in Phase 1-6.
//!
//! See `02_architecture.md` §5 for the full surface spec.

use core::ffi::{c_int, c_void};
use crate::cpu::regs::CpuRegsLayout;
use crate::mapper::{MapperMetaVtable, ReadRangeHandler, WriteRangeHandler};
use crate::soc::{VNesSoc, VNesSocOpaque};

// =========================================================================
// Lifecycle
// =========================================================================

/// Create a new SoC. Phase 0: returns a `Box::into_raw` of a default-constructed
/// `VNesSoc` wrapped in a heap-allocated `VNesSocOpaque`. Phase 1+ will wire
/// RAM init patterns + mapper attach.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_create() -> *mut VNesSocOpaque {
    let soc = Box::new(VNesSoc::default());
    let soc_raw: *mut VNesSoc = Box::into_raw(soc);
    let opaque = Box::new(VNesSocOpaque(soc_raw));
    Box::into_raw(opaque)
}

/// Free the SoC. Phase 0: inverse of `vnesu11_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_destroy(soc: *mut VNesSocOpaque) {
    if soc.is_null() {
        return;
    }
    let opaque = Box::from_raw(soc);
    if !opaque.0.is_null() {
        drop(Box::from_raw(opaque.0));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_power_on(_soc: *mut VNesSocOpaque) {
    // Phase 0: stub.
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_reset(_soc: *mut VNesSocOpaque) {
    // Phase 0: stub.
}

// =========================================================================
// Mapper handler registration
// =========================================================================

/// Register a per-range read handler. Phase 2+ will use this for actual reads;
/// Phase 0 stores the handler so future reads can resolve correctly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_read_handler(
    soc: *mut VNesSocOpaque,
    start: u16,
    end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,
    ctx: *mut c_void,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if soc_ref.mapper.read_count >= crate::mapper::MAX_RANGES {
        return -2;
    }
    let i = soc_ref.mapper.read_count;
    soc_ref.mapper.read_ranges[i] = ReadRangeHandler { start, end, fn_ptr, ctx };
    soc_ref.mapper.read_count += 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_write_handler(
    soc: *mut VNesSocOpaque,
    start: u16,
    end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16, u8),
    ctx: *mut c_void,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if soc_ref.mapper.write_count >= crate::mapper::MAX_RANGES {
        return -2;
    }
    let i = soc_ref.mapper.write_count;
    soc_ref.mapper.write_ranges[i] = WriteRangeHandler { start, end, fn_ptr, ctx };
    soc_ref.mapper.write_count += 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_clear_mapper_handlers(soc: *mut VNesSocOpaque) {
    if let Some(s) = into_mut(soc) {
        s.mapper.clear();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_attach_mapper_meta(
    soc: *mut VNesSocOpaque,
    mapper: *mut c_void,
    vtable: *const MapperMetaVtable,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if vtable.is_null() {
        return -2;
    }
    soc_ref.mapper_meta = Some(crate::soc::MapperMetaSlot {
        mapper_ctx: mapper,
        meta: *vtable,
    });
    0
}

// =========================================================================
// System type (Phase 6 — stub for now)
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_system_type(
    _soc: *mut VNesSocOpaque,
    _system_type: u32,
) -> c_int {
    // 0=iNES 1=FDS 2=NSF 3=VS — Phase 0 stub.
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_external_irq(
    _soc: *mut VNesSocOpaque,
    _source: u32,
    _on: bool,
) {
    // FDS disk IRQ — Phase 0 stub.
}

// =========================================================================
// Emulation
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_emulate_frame(
    _soc: *mut VNesSocOpaque,
    _skip: c_int,
    _xbuf: *mut u8,
    _sbuf: *mut i16,
    _sbuf_cap: usize,
    _sbuf_written: *mut usize,
) -> c_int {
    // Phase 0: stub. Phase 1+ wires real emulation.
    -1
}

// =========================================================================
// Debugger / Lua / savestate peek/poke
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_peek(
    _soc: *const VNesSocOpaque,
    _addr: u16,
) -> u8 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_poke(
    _soc: *mut VNesSocOpaque,
    _addr: u16,
    _val: u8,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_peek_regs(
    _soc: *const VNesSocOpaque,
    out: *mut CpuRegsLayout,
) {
    if !out.is_null() {
        *out = CpuRegsLayout::default();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_poke_regs(
    _soc: *mut VNesSocOpaque,
    _regs: *const CpuRegsLayout,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_ppu_peek(
    _soc: *const VNesSocOpaque,
    _addr: u16,
) -> u8 {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_joypad_set_button(
    _soc: *mut VNesSocOpaque,
    _pad: u8,
    _btn: u32,
    _pressed: bool,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_joypad_set_strobe(
    _soc: *mut VNesSocOpaque,
    _strobe: bool,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_lua_mem_hook(_active: bool) {}

// =========================================================================
// Savestate (Phase 0 stub — real impl in Phase 0 + Phase 6 per
// docs/wip_2.0_plan/savestate_tags.md)
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_save_cpu_state(
    _soc: *const VNesSocOpaque,
    _sink: *mut c_void,
    _write_fn: extern "C" fn(*mut c_void, *const u8, usize),
) -> c_int {
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_load_cpu_state(
    _soc: *mut VNesSocOpaque,
    _source: *mut c_void,
    _read_fn: extern "C" fn(*mut c_void, *mut u8, usize) -> usize,
) -> c_int {
    -1
}

// =========================================================================
// Helpers
// =========================================================================

/// Returns `Some(&mut soc)` if the pointer is valid (non-null + non-null inner).
unsafe fn into_mut(soc: *mut VNesSocOpaque) -> Option<&'static mut VNesSoc> {
    if soc.is_null() {
        return None;
    }
    let inner = (*soc).0;
    if inner.is_null() {
        return None;
    }
    Some(&mut *inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_destroy_round_trip() {
        let soc = unsafe { vnesu11_create() };
        assert!(!soc.is_null());
        unsafe { vnesu11_destroy(soc); }
    }

    #[test]
    fn null_safety() {
        unsafe { vnesu11_destroy(core::ptr::null_mut()); }
        let r = unsafe { vnesu11_set_read_handler(
            core::ptr::null_mut(), 0, 0, dummy_read, core::ptr::null_mut()
        ) };
        assert_eq!(r, -1);
    }

    unsafe extern "C" fn dummy_read(_ctx: *mut core::ffi::c_void, _addr: u16) -> u8 { 0 }
}
