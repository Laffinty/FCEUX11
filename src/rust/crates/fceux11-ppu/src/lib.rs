//! fceux11-ppu — Native Rust cycle-accurate 2C02 PPU state machine.
//!
//! Phase 1 of the v2.1 PPU refactor plan (`docs/plans/v2.1_ppu_rust_refactor_plan.md`).
//! Phase 2 of the same plan adds the FFI surface (`ffi` module) and the
//! minimal NROM renderer (`render` module) so the C++ side can drive the
//! Rust PPU through `ppu_rust_bridge.cpp`.
//!
//! Modules:
//! - [`bus`]: `PpuBus` trait + the `FlatBus` test stub used by the integration tests.
//! - [`registers`]: `$2000`-`$2007`, `$4014`, scroll latches, open-bus buffer.
//! - [`state`]: `PpuState` aggregate (registers + OAM + secondary OAM + frame counters).
//! - [`frame`]: `tick_dot` — the dot-level main state machine.
//! - [`ffi`]: C-ABI surface (`fceux11_ppu_*` exports).
//! - [`render`]: NROM BG/sprite/palette pipeline (Phase 2 stub; Phase 4 full).
//! - [`rendering`]: Phase 4 per-scanline BG fetch + pixel output.
//! - [`luts`]: precomputed ppulut1/2/3 lookup tables (Phase 4).

pub mod bus;
pub mod ffi;
pub mod frame;
pub mod luts;
pub mod render;
pub mod registers;
pub mod rendering;
pub mod scheduler;
pub mod state;

pub use bus::{FlatBus, PpuBus};
pub use frame::{TickOutcome, tick_dot};
pub use registers::{Registers, ctrl_bits, mask_bits, status_bits};
pub use scheduler::{NesScheduler, NTSC_CPU_CYCLES_PER_FRAME, PAL_CPU_CYCLES_PER_FRAME, PPU_DOTS_PER_CPU_CYCLE};
pub use state::PpuState;
