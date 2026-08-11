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
use crate::ram::RamInitOption;
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
    // Phase 0: stub. Phase 2 ships `vnesu11_set_ram_init` + a real
    // power-on path that consumes it.
}

/// Phase 2: configure the RAM-init option + seed. Must be called before
/// `vnesu11_power_on`. Equivalent to C++ setting `RAMInitOption` /
/// `RAMInitSeed` in `src/drivers/Qt/ConsoleEmuControl.cpp:475-501`.
///
/// `option`: 0=Checker (default), 1=AllOnes, 2=AllZeros, 3=Random.
/// `seed`: 32-bit seed for `splitmix64` (Random mode only).
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_ram_init(
    soc: *mut VNesSocOpaque,
    option: u32,
    seed: u32,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    // SAFETY: any 0..=3 is valid; out-of-range falls back to Checker.
    soc_ref.ram_init_option = unsafe { RamInitOption::from_raw_unchecked(option) };
    soc_ref.ram_init_seed = seed;
    0
}

/// Phase 2: power on with the previously-set `RamInitOption` + seed.
/// Mirrors `PowerNES` in `src/fceu.cpp:1000-1025`.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_power_on_with_init(soc: *mut VNesSocOpaque) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    let option = soc_ref.ram_init_option;
    let seed = soc_ref.ram_init_seed;
    soc_ref.power_on(option, seed);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_reset(soc: *mut VNesSocOpaque) {
    // Phase 2: minimal — re-init CPU, leave RAM alone.
    if let Some(s) = into_mut(soc) {
        let mut bus = crate::soc::VNesBusContext::new(s);
        s.cpu.reset(&mut bus);
        s.open_bus = 0;
        s.ppu_w = false;
        s.ppu_t = 0;
        s.ppu_v = 0;
        s.ppu_x = 0;
        s.ppu_read_buffer = 0;
    }
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
// Phase 2: save/load the four private RAM banks (WRAM/VRAM/OAM/Palette)
// =========================================================================
//
// The C++ side calls these through the FFI to integrate vNESU11 into the
// `FCEUSS_SaveMS` / `FCEUSS_LoadMS` flow. The V2-chunked byte stream
// matches the layout produced by `state.cpp`'s `SFCPU`/`FCEU_NEWPPU_STATEINFO`
// groups, so a vNESU11 savestate round-trips byte-for-byte with the C++
// reader.

/// Save the four RAM banks into a heap buffer. Caller takes ownership
/// of the returned `*mut u8` (free with `vnesu11_free_buffer`).
///
/// `out_len`: optional pointer to receive the byte count; pass null if
/// not needed.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`. The
/// returned `*mut u8` must be freed with `vnesu11_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_save_ram_state(
    soc: *const VNesSocOpaque,
    out_len: *mut usize,
) -> *mut u8 {
    let soc_ref = match into_const(soc) {
        Some(s) => s,
        None => return core::ptr::null_mut(),
    };
    let mut w = crate::snapshot::mem::Writer::with_capacity(8192);
    soc_ref.ram_banks.save_state(&mut w);
    let bytes = w.into_bytes();
    let n = bytes.len();
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    if !out_len.is_null() {
        *out_len = n;
    }
    ptr
}

/// Load the four RAM banks from a V2 byte stream. Returns 0 on success,
/// negative on error.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`. `bytes`
/// must point to a buffer of at least `len` readable bytes (typically
/// a buffer previously returned by `vnesu11_save_ram_state`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_load_ram_state(
    soc: *mut VNesSocOpaque,
    bytes: *const u8,
    len: usize,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if bytes.is_null() || len == 0 {
        return -2;
    }
    let slice = core::slice::from_raw_parts(bytes, len);
    let mut r = crate::snapshot::mem::Reader::new(slice);
    match soc_ref.ram_banks.load_state(&mut r) {
        Ok(()) => {
            soc_ref.sync_ram_banks_to_views();
            0
        }
        Err(_) => -3,
    }
}

/// Free a buffer returned by `vnesu11_save_ram_state`.
///
/// # Safety
/// `ptr` must either be null or a pointer returned by
/// `vnesu11_save_ram_state`; `len` must be the value written to
/// `out_len` at allocation time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr, len));
    }
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

unsafe fn into_const(soc: *const VNesSocOpaque) -> Option<&'static VNesSoc> {
    if soc.is_null() {
        return None;
    }
    let inner = (*soc).0;
    if inner.is_null() {
        return None;
    }
    Some(&*inner)
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
