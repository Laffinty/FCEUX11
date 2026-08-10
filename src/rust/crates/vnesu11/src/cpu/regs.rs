//! CpuRegsLayout — 64-byte mirror of the C++ `X6502` struct.
//!
//! **Audit-critical (S1)**: Field order, alignment, and size MUST match
//! `src/x6502struct.h` exactly. Verified at compile time via the
//! `static_assert!` block below + at test time via `tests/layout_check.rs`.
//!
//! Reference: `src/x6502struct.h`:
//! ```c
//! typedef struct alignas(64) X6502 {
//!   int32 tcount;      // 0
//!   uint16 PC;         // 4
//!   uint8 A,X,Y,S,P,mooPI; // 6..11
//!   uint8 jammed;      // 12
//!   int32 count;       // 16
//!   uint32 IRQlow;     // 20
//!   uint8 DB;          // 24
//!   int preexec;       // 28
//!   #ifdef FCEUDEF_DEBUGGER
//!     void (*CPUHook)(...);    // 32
//!     uint8 (*ReadHook)(...);  // 40
//!     void (*WriteHook)(...);  // 48
//!   #endif
//! } X6502;
//! ```

use std::mem::{offset_of, size_of};

/// 64-byte mirror of `X6502` (no FCEUDEF_DEBUGGER hooks).
///
/// When the `debugger` Cargo feature is enabled, the `cpu_hook`/`read_hook`/
/// `write_hook` fields become populated. The 64-byte size is constant in both
/// cases (the hooks slot in between 32 and 56, leaving 8 bytes of tail
/// padding to reach 64).
#[repr(C, align(64))]
#[derive(Clone, Copy)]
pub struct CpuRegsLayout {
    pub tcount: i32,       // 0
    pub PC: u16,           // 4
    pub A: u8,             // 6
    pub X: u8,             // 7
    pub Y: u8,             // 8
    pub S: u8,             // 9
    pub P: u8,             // 10
    pub moo_pi: u8,        // 11
    pub jammed: u8,        // 12
    // (padding 13-15 for count's 4-byte alignment)
    pub count: i32,        // 16
    pub irq_low: u32,      // 20
    pub db: u8,            // 24
    // (padding 25-27 for preexec's 4-byte alignment)
    pub preexec: i32,      // 28
    // FCEUDEF_DEBUGGER hooks (32/40/48) — present in both feature configs so
    // the struct remains 64 bytes; the hooks are nulled out by default.
    pub cpu_hook: *mut core::ffi::c_void,    // 32
    pub read_hook: *mut core::ffi::c_void,   // 40
    pub write_hook: *mut core::ffi::c_void,  // 48
    // (padding 56-63 to satisfy alignas(64))
}

// =========================================================================
// AUDIT S1 — Compile-time layout assertions.
// These are NOT optional. Any drift in C++ x6502struct.h must be reflected
// here AND the audit must be re-run.
// =========================================================================
const _: () = {
    assert!(offset_of!(CpuRegsLayout, tcount)    == 0);
    assert!(offset_of!(CpuRegsLayout, PC)        == 4);
    assert!(offset_of!(CpuRegsLayout, A)         == 6);
    assert!(offset_of!(CpuRegsLayout, X)         == 7);
    assert!(offset_of!(CpuRegsLayout, Y)         == 8);
    assert!(offset_of!(CpuRegsLayout, S)         == 9);
    assert!(offset_of!(CpuRegsLayout, P)         == 10);
    assert!(offset_of!(CpuRegsLayout, moo_pi)    == 11);
    assert!(offset_of!(CpuRegsLayout, jammed)    == 12);
    assert!(offset_of!(CpuRegsLayout, count)     == 16);
    assert!(offset_of!(CpuRegsLayout, irq_low)   == 20);
    assert!(offset_of!(CpuRegsLayout, db)        == 24);
    assert!(offset_of!(CpuRegsLayout, preexec)   == 28);
    assert!(offset_of!(CpuRegsLayout, cpu_hook)  == 32);
    assert!(offset_of!(CpuRegsLayout, read_hook) == 40);
    assert!(offset_of!(CpuRegsLayout, write_hook) == 48);
    assert!(size_of::<CpuRegsLayout>() == 64);
    assert!(align_of::<CpuRegsLayout>() == 64);
};

impl Default for CpuRegsLayout {
    fn default() -> Self {
        Self {
            tcount: 0,
            PC: 0,
            A: 0,
            X: 0,
            Y: 0,
            S: 0,
            P: 0,
            moo_pi: 0,
            jammed: 0,
            count: 0,
            irq_low: 0,
            db: 0,
            preexec: 0,
            cpu_hook: core::ptr::null_mut(),
            read_hook: core::ptr::null_mut(),
            write_hook: core::ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_is_64_bytes() {
        assert_eq!(size_of::<CpuRegsLayout>(), 64);
    }

    #[test]
    fn alignment_is_64() {
        assert_eq!(align_of::<CpuRegsLayout>(), 64);
    }
}
