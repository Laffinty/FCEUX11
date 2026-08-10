//! Per-field layout validation — AUDIT S1.
//!
//! These tests are the runtime counterpart to the `static_assert!`s in
//! `src/cpu/regs.rs`. They confirm that field offsets match the
//! `src/x6502struct.h` definition bit-for-bit, and that the struct
//! remains 64-byte aligned (savestate + debugger hooks).
//!
//! If any of these tests fail, **do not edit the test** — the C++ struct
//! in `src/x6502struct.h` has drifted, and the savestate binary format
//! is now broken. Coordinate with the C++ maintainers before changing.

use std::mem::{align_of, offset_of, size_of};
use vnesu11::cpu::regs::CpuRegsLayout;

#[test]
fn cpu_regs_size_64() {
    assert_eq!(size_of::<CpuRegsLayout>(), 64,
        "CpuRegsLayout must be 64 bytes (matches X6502 struct in src/x6502struct.h)");
}

#[test]
fn cpu_regs_align_64() {
    assert_eq!(align_of::<CpuRegsLayout>(), 64,
        "CpuRegsLayout must be 64-byte aligned (matches alignas(64) on X6502)");
}

#[test]
fn cpu_regs_field_offsets() {
    // Every offset matches the C++ struct order (see AUDIT S1).
    assert_eq!(offset_of!(CpuRegsLayout, tcount),    0, "tcount");
    assert_eq!(offset_of!(CpuRegsLayout, PC),        4, "PC");
    assert_eq!(offset_of!(CpuRegsLayout, A),         6, "A");
    assert_eq!(offset_of!(CpuRegsLayout, X),         7, "X");
    assert_eq!(offset_of!(CpuRegsLayout, Y),         8, "Y");
    assert_eq!(offset_of!(CpuRegsLayout, S),         9, "S");
    assert_eq!(offset_of!(CpuRegsLayout, P),         10, "P");
    assert_eq!(offset_of!(CpuRegsLayout, moo_pi),    11, "mooPI");
    assert_eq!(offset_of!(CpuRegsLayout, jammed),    12, "jammed");
    assert_eq!(offset_of!(CpuRegsLayout, count),     16, "count");
    assert_eq!(offset_of!(CpuRegsLayout, irq_low),   20, "IRQlow");
    assert_eq!(offset_of!(CpuRegsLayout, db),        24, "DB");
    assert_eq!(offset_of!(CpuRegsLayout, preexec),   28, "preexec");
    // FCEUDEF_DEBUGGER hooks (must be present even when feature is off —
    // the struct is always 64 bytes via alignas(64)).
    assert_eq!(offset_of!(CpuRegsLayout, cpu_hook),  32, "CPUHook");
    assert_eq!(offset_of!(CpuRegsLayout, read_hook), 40, "ReadHook");
    assert_eq!(offset_of!(CpuRegsLayout, write_hook),48, "WriteHook");
}

#[test]
fn cpu_regs_default_is_zero() {
    let r = CpuRegsLayout::default();
    assert_eq!(r.tcount, 0);
    assert_eq!(r.PC, 0);
    assert_eq!(r.A, 0);
    assert_eq!(r.X, 0);
    assert_eq!(r.Y, 0);
    assert_eq!(r.S, 0);
    assert_eq!(r.P, 0);
    assert_eq!(r.moo_pi, 0);
    assert_eq!(r.jammed, 0);
    assert_eq!(r.count, 0);
    assert_eq!(r.irq_low, 0);
    assert_eq!(r.db, 0);
    assert_eq!(r.preexec, 0);
    assert!(r.cpu_hook.is_null());
    assert!(r.read_hook.is_null());
    assert!(r.write_hook.is_null());
}
