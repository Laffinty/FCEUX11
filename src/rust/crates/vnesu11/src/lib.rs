//! vNESU11 — FCEUX11 unified CPU/PPU/APU virtual SoC core (v2.0 wip)
//!
//! See `docs/wip_2.0_plan/` for the full engineering plan.
//!
//! # Phase status
//! - Phase 0: crate skeleton + FFI surface + CpuRegsLayout (DONE).
//! - Phase 1: 6502 interpreter (DONE).
//! - Phase 2: bus matrix + private RAM + RamRng (THIS PHASE).
//! - Phase 3-8: pending.

#![allow(unsafe_op_in_unsafe_fn)]
#![allow(non_snake_case)]

pub mod apu;
pub mod bus;
pub mod cpu;
pub mod dma;
pub mod ffi;
pub mod irq;
pub mod joypad;
pub mod mapper;
pub mod ppu;
pub mod ram;
pub mod snapshot;
pub mod soc;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_root_compiles() {
        assert_eq!(2 + 2, 4);
    }
}