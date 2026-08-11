//! Savestate module — SFORMAT-tag-driven serialization.
//!
//! Phase 2 implements the RAM/NRAM/SPRAM/PALR/PRNG/RADO chunks. CPU/PPU/
//! APU chunks land in their respective phases (Phase 6 ties it all
//! together).

pub mod mem;