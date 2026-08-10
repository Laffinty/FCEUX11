//! vNESU11 — FCEUX11 unified CPU/PPU/APU virtual SoC core (v2.0 wip)
//!
//! See `docs/wip_2.0_plan/` for the full engineering plan.
//! This file is the crate root. All modules are stubs in Phase 0 (no real
//! CPU/PPU/APU logic yet); see phase_0_foundation.md for the DoD.

// Allow `unsafe` blocks inside `unsafe fn` (Rust 2024 edition requirement).
// The crate uses many FFI boundaries where `unsafe fn` makes intent clearer
// than wrapping every dereference in another block. Per-operation safety
// is still documented at each call site.
#![allow(unsafe_op_in_unsafe_fn)]

// CpuRegsLayout fields (PC, A, X, Y, S, P, moo_pi, ...) MUST keep their
// C++ spellings (`PC` not `pc`) to match `src/x6502struct.h` field-for-field
// (AUDIT S1). Same for `IRQlow` → `irq_low`, `X6502::count` → `count`,
// `X6502::tcount` → `tcount`, etc. — the snake_case lint is intentionally
// disabled here.
#![allow(non_snake_case)]

// `missing_debug_implementations` is deferred to a later phase once the
// placeholder types are replaced with real types that warrant Debug.

pub mod cpu;
pub mod ffi;
pub mod mapper;
pub mod soc;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_root_compiles() {
        // Smoke test: just confirm the crate links without panic.
        assert_eq!(2 + 2, 4);
    }
}
