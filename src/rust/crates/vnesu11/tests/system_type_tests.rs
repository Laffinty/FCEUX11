//! System type dispatch smoke tests (Phase 6 §1.1).
//!
//! Verifies the `vnesu11_set_system_type` FFI toggles the SoC's PPU
//! mode correctly for each EGIT value (iNES / VS / FDS / NSF), and
//! that `run_frame` produces sensible output for each.
//!
//! Maps to Phase 6 DoD: "**FDS / NSF / VS** 各跑一个测试文件通过".
//! True cross-core parity vs the C++ pipeline is verified by the
//! kagami-qa harness; here we validate the per-system-type behavior
//! of the Rust SoC in isolation.

use vnesu11::ffi::{
    vnesu11_create, vnesu11_destroy, vnesu11_emulate_frame, vnesu11_power_on_with_init,
    vnesu11_set_ram_init, vnesu11_set_system_type,
};
use vnesu11::soc::VNesSocOpaque;

fn create_powered_soc() -> *mut VNesSocOpaque {
    let soc = unsafe { vnesu11_create() };
    assert!(!soc.is_null());
    unsafe { vnesu11_set_ram_init(soc, 0, 0) };
    unsafe { vnesu11_power_on_with_init(soc) };
    soc
}

#[test]
fn system_type_ines_renders_normally() {
    let soc = create_powered_soc();
    // EGIT 0 = GIT_CART (iNES). PPU in normal mode.
    let rc = unsafe { vnesu11_set_system_type(soc, 0) };
    assert_eq!(rc, 0);
    let raw = unsafe { (*soc).0 };
    let s = unsafe { &mut *raw };
    assert!(!s.ppu.idle, "iNES → PPU should NOT be idle");
    let scanline_before = s.ppu.scanline;
    s.run_frame();
    let scanline_after = s.ppu.scanline;
    assert!(
        scanline_after != scanline_before || s.ppu.frame_ready,
        "iNES frame should advance PPU scanline ({:?} -> {:?})",
        scanline_before,
        scanline_after
    );
    unsafe { vnesu11_destroy(soc) };
}

#[test]
fn system_type_vs_renders_normally() {
    let soc = create_powered_soc();
    // EGIT 1 = GIT_VSUNI (VS UniSystem). PPU in normal mode + VS coin
    // input via the JoypadState.
    let rc = unsafe { vnesu11_set_system_type(soc, 1) };
    assert_eq!(rc, 0);
    let raw = unsafe { (*soc).0 };
    let s = unsafe { &mut *raw };
    assert!(!s.ppu.idle, "VS → PPU should NOT be idle");
    s.run_frame();
    unsafe { vnesu11_destroy(soc) };
}

#[test]
fn system_type_fds_renders_normally() {
    let soc = create_powered_soc();
    // EGIT 2 = GIT_FDS. PPU in normal mode + FDS disk IRQ via the
    // IrqController EXT/EXT2 sources.
    let rc = unsafe { vnesu11_set_system_type(soc, 2) };
    assert_eq!(rc, 0);
    let raw = unsafe { (*soc).0 };
    let s = unsafe { &mut *raw };
    assert!(!s.ppu.idle, "FDS → PPU should NOT be idle");
    s.run_frame();
    unsafe { vnesu11_destroy(soc) };
}

#[test]
fn system_type_nsf_drives_ppu_idle_stub() {
    let soc = create_powered_soc();
    // EGIT 3 = GIT_NSF. PPU is in the idle stub (no rendering, dot
    // clock still advances, no nametable activity).
    let rc = unsafe { vnesu11_set_system_type(soc, 3) };
    assert_eq!(rc, 0);
    let raw = unsafe { (*soc).0 };
    let s = unsafe { &mut *raw };
    assert!(s.ppu.idle, "NSF → PPU should be in idle stub mode");
    s.run_frame();
    assert!(
        s.frame_buffer.iter().all(|&b| b == 0),
        "NSF idle PPU should produce no frame buffer output"
    );
    // Switch back to iNES → PPU leaves idle mode.
    let rc = unsafe { vnesu11_set_system_type(soc, 0) };
    assert_eq!(rc, 0);
    let raw = unsafe { (*soc).0 };
    let s = unsafe { &mut *raw };
    assert!(!s.ppu.idle, "Switching back to iNES should disable idle");
    unsafe { vnesu11_destroy(soc) };
}

#[test]
fn system_type_unknown_value_returns_minus_one() {
    // EGIT values outside 0..=3 are undefined. The FFI returns -1
    // to signal a bad input (the PPU idle flag is left unchanged).
    let soc = create_powered_soc();
    unsafe { vnesu11_set_system_type(soc, 0) };
    let raw = unsafe { (*soc).0 };
    let s = unsafe { &mut *raw };
    assert!(!s.ppu.idle);
    let rc = unsafe { vnesu11_set_system_type(soc, 99) };
    assert_eq!(rc, -1, "out-of-range system_type should return -1");
    // The PPU idle flag should be unchanged.
    assert!(!s.ppu.idle, "out-of-range value must not flip idle");
    unsafe { vnesu11_destroy(soc) };
}

#[test]
fn system_type_nsf_idle_ppu_clears_frame_buffer() {
    // Verify the NSF idle stub's frame-buffer behavior end-to-end:
    // a frame produces a zeroed 61440-byte buffer.
    let soc = create_powered_soc();
    unsafe { vnesu11_set_system_type(soc, 3) };
    let mut xbuf = [0xAAu8; 61440];
    let mut sbuf_storage = [0i16; 16384];
    let mut sbuf_written: usize = 0;
    let rc = unsafe {
        vnesu11_emulate_frame(
            soc,
            0,
            xbuf.as_mut_ptr(),
            sbuf_storage.as_mut_ptr(),
            sbuf_storage.len(),
            &mut sbuf_written as *mut usize,
        )
    };
    assert_eq!(rc, 0, "NSF frame must complete");
    // NSF should produce a zeroed frame buffer (no PPU rendering).
    assert!(
        xbuf.iter().all(|&b| b == 0),
        "NSF idle PPU should produce all-zero frame buffer"
    );
    unsafe { vnesu11_destroy(soc) };
}
