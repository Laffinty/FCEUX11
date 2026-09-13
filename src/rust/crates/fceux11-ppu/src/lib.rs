//! fceux11-ppu — Native Rust cycle-accurate 2C02 PPU state machine.
//!
//! Phase 1 of the v2.1 PPU refactor plan (`docs/plans/v2.1_ppu_rust_refactor_plan.md`).
//! Phase 2 of the same plan adds the FFI surface (`ffi` module) and the
//! minimal NROM renderer (`render` module) so the C++ side can drive the
//! Rust PPU through `ppu_rust_bridge.cpp`.
//!
//! Modules:
//! - [`a12`]: filtered A12 watcher (v2.1.3 batch 1; batch 2 feeds it the
//!   real fetch addresses from [`rendering`]).
//! - [`bus`]: `PpuBus` trait + the `FlatBus` test stub used by the integration tests.
//! - [`registers`]: `$2000`-`$2007`, `$4014`, scroll latches, open-bus buffer.
//! - [`state`]: `PpuState` aggregate (registers + OAM + secondary OAM + frame counters).
//! - [`frame`]: `tick_dot` — the dot-level main state machine (timing events,
//!   the fetch cycle's scroll updates, sprite evaluation).
//! - [`ffi`]: C-ABI surface (`fceux11_ppu_*` exports).
//! - [`render`]: NROM BG/sprite/palette pipeline (Phase 2 stub; Phase 4 full).
//! - [`rendering`]: v2.1.3 batch 2 per-dot BG fetch + pixel pipeline.
//! - [`sprites`]: sprite layer preparation + per-pixel lookup.
//! - [`luts`]: precomputed ppulut1/2/3 lookup tables (Phase 4).

pub mod a12;
pub mod bus;
pub mod ffi;
pub mod frame;
pub mod luts;
pub mod registers;
pub mod render;
pub mod rendering;
pub mod scheduler;
pub mod snapshot;
pub mod sprites;
pub mod state;
pub mod video_system;

pub use bus::{FlatBus, PpuBus};
pub use frame::{TickOutcome, tick_dot};
pub use registers::{Registers, ctrl_bits, mask_bits, status_bits};
pub use scheduler::{
    NTSC_CPU_CYCLES_PER_FRAME, NesScheduler, PAL_CPU_CYCLES_PER_FRAME, PPU_DOTS_PER_CPU_CYCLE,
};
pub use state::PpuState;
pub use video_system::{VideoSystem, VideoSystemTimings};
