//! GBAEUX11 -- GBA-side logic layer for FCEUX11 (v2.0 S0').
//!
//! The vendored hardware core is `gba-core` (an independent path-dependency
//! crate). This module owns everything that belongs to *us*:
//!
//!   * the SWI semantics the real GBA BIOS would otherwise provide in
//!     machine code (see [`swi`]), and
//!   * the C ABI the C++ side calls (see [`ffi`]).
//!
//! # Why a module and not a crate
//!
//! The v2.0 plan originally called for a separate `f11gba` crate. That is not
//! buildable on this toolchain: rustc 1.96 fat LTO cannot load the bitcode of
//! a second archive in the dependency chain (see the plan's R14 and the r8
//! entry in its change log). Six controlled experiments showed the only
//! variable is whether the GBA code is its own crate -- not its size, its
//! dependencies, or whether it contains generics. Hosting it as a plain module
//! of the root crate has no rlib boundary to cross and was verified to place
//! all four C ABI symbols in `fceux11_rust.lib`.
//!
//! The seam to the core is a function pointer installed in
//! `gba_core::cpu::arm7tdmi::Arm7tdmi::swi_hook`, so this module and the core
//! stay one-directional: the root crate depends on `gba-core`, never the
//! reverse.

pub mod bios;
pub mod ffi;
pub mod frame;
pub mod overlay;
pub mod save;
pub mod swi;

/// Install our SWI implementation into a freshly created core.
///
/// Called from [`ffi::gba_init`]. Kept separate from the FFI surface so the
/// registration is testable from Rust without going through C.
pub fn install_swi_hook(gba: &mut gba_core::gba::Gba) {
    gba.cpu.swi_hook = Some(swi::dispatch);
}
