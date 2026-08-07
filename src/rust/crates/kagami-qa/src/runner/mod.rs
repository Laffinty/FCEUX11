//! L5 — scheduling and execution.
//!
//! `scheduler` runs manifest entries through the subprocess adapter;
//! `direct` is the shared in-process execution core used by both the
//! CLI `--direct` mode and the C-ABI `direct_entry`;
//! `blargg` is the Track C Task 1 / C-1 harness — re-implements the
//! C++ `blargg_runner.cpp` semantics (single-ROM `BLARGG_RESULT:` line
//! and batch `--manifest` JSON output) on top of [`SutAdapter`];
//! `rom_regression` is the Track C Task 1 / C-2 harness — replaces
//! `tests/rom_regression_test.cpp` (12-ROM CRC32 frame regression).

pub mod blargg;
pub mod direct;
pub mod rom_regression;
pub mod scheduler;
