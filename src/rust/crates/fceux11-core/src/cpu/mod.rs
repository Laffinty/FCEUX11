//! 6502 CPU module — Rust-first reimplementation.
//!
//! See `docs/plans/cpu-rust-v2.md` for the architecture plan and
//! `src/cpu/../state.rs` for the 64-byte savestate-compatible layout.
//!
//! Public surface (Phase 1 + 3):
//! * [`state::X6502Layout`] — the byte-compatible C++ X6502 struct.
//! * [`state::Flags`] / [`state::IrqSource`] — flag bitflags and IRQ masks.
//! * [`addressing::Bus`] — minimal CPU bus trait; the real [`crate::traits::Cpu`]
//!   integration lands in Phase 5.
//! * [`addressing::CpuState`] — wrapper around the layout + side state.
//! * [`decode::info`] — `OpcodeInfo` for any of the 256 opcodes.
//! * [`execute::step`] / [`execute::run`] — instruction execution.
//! * [`bus::CppBus`] / [`bus::fceux11_cpu_set_bus`] — Phase 3 FFI bridge
//!   that calls into C++ for memory access when wired in via
//!   `FCEUX11_RUST_CPU=ON`.
//! * [`ffi`] — the actual `#[no_mangle] extern "C"` symbols.

pub mod addressing;
pub mod alu;
pub mod bus;
pub mod decode;
pub mod execute;
pub mod ffi;
pub mod state;

pub use addressing::{Bus, CpuState, ModeResult};
pub use decode::{info, OpKind, OpcodeInfo, CYC_TABLE, OP_SIZE};
pub use execute::{run, step, CYCLES_PER_CPU_CYCLE};
pub use state::{Flags, IrqSource, X6502Layout, ZN_TABLE};

/// Quick sanity-check helper: returns true if every opcode's base cycle
/// count is the same as the legacy `CycTable`. Phase 1 ships this so
/// other modules (tests, debug HUDs) can verify the decode table
/// hasn't drifted.
pub fn cycle_count(opcode: u8) -> u8 {
    CYC_TABLE[opcode as usize]
}

/// Returns the size in bytes (operand bytes + opcode) of `opcode`.
/// Returns 0 for unstable / unhandled opcodes.
pub fn size_of(opcode: u8) -> u8 {
    OP_SIZE[opcode as usize]
}