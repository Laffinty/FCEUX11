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

pub mod audio;
pub mod bios;
/// Research-only measurement tool for the S4 compatibility triage: run real
/// cartridges and report whether each is wedged, and whether it is armed for a
/// DMA start timing this core never fires. Every test in it is `#[ignore]`d, so
/// it adds no gate; it exists to print numbers.
pub mod cartprobe;
pub mod decoupling;
pub mod ffi;
pub mod frame;
pub mod gate;
pub mod overlay;
pub mod rtc;
pub mod save;
pub mod swi;
/// Research-only measurement tool for the v2.0.1 wait-state work. Every test
/// in it is `#[ignore]`d, so it adds no gate; it exists to print numbers.
pub mod waitprobe;

/// Install our SWI implementation into a freshly created core.
///
/// Called from [`ffi::gba_init`]. Kept separate from the FFI surface so the
/// registration is testable from Rust without going through C.
pub fn install_swi_hook(gba: &mut gba_core::gba::Gba) {
    gba.cpu.swi_hook = Some(swi::dispatch);
}

/// # Crash diagnostics -- TEMPORARY (v2.0.1 S4)
///
/// The release profile sets `panic = "abort"`
/// (`src/rust/Cargo.toml`), so a panic anywhere in the core ends the process.
/// The default hook prints `file:line` to stderr first, and on this machine
/// that is exactly what does **not** happen: the S4 triage captured a
/// fast-fail 7 with an empty stderr. An empty stderr is the interesting fact --
/// it means whatever aborted did not go through a Rust panic either.
///
/// This records the panic to a file so the next occurrence names itself. The
/// path comes from `FCEUX11_DIAG_LOG`, which the C++ `main()` sets to the
/// directory the executable lives in, so both halves of the diagnosis land in
/// one `fceux_diag.log` beside the binary.
///
/// The previous hook is kept and called afterwards, so normal panic reporting
/// is unchanged for anyone reading a console.
pub fn install_diagnostic_hooks() {
    use std::io::Write;
    use std::sync::Once;

    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let path = std::env::var("FCEUX11_DIAG_LOG")
                .unwrap_or_else(|_| "fceux_diag.log".to_string());
            // Opened per call and flushed immediately: this runs while the
            // process is already unwinding toward abort, so a buffered stream
            // would lose exactly the text it exists to preserve.
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                let _ = writeln!(
                    file,
                    "\n=== RUST PANIC (thread {:?}) ===\n{}\n",
                    std::thread::current().id(),
                    info
                );
                let _ = file.flush();
            }
            previous(info);
        }));
    });
}
