//! L5 — scheduling and execution.
//!
//! `scheduler` runs manifest entries through the subprocess adapter;
//! `direct` is the shared in-process execution core used by both the
//! CLI `--direct` mode and the C-ABI `direct_entry`;
//! `blargg` is the Track C Task 1 / C-1 harness — re-implements the
//! C++ `blargg_runner.cpp` semantics (single-ROM `BLARGG_RESULT:` line
//! and batch `--manifest` JSON output) on top of [`SutAdapter`];
//! `rom_regression` is the Track C Task 1 / C-2 harness — replaces
//! `tests/rom_regression_test.cpp` (13-ROM CRC32 frame regression);
//! `savestate_regression` is the Track C Task 1 / C-3 harness —
//! replaces `tests/savestate_regression_test.cpp` (12-ROM MD5
//! savestate regression, vrc7 omitted);
//! `mapper_byte_diff` is the Task 1 mapper harness — replaces
//! `tests/core/mapper_byte_diff_test.cpp` (175-case mapper state
//! byte-diff against `fixtures/golden_mapper/*.bin`);
//! `test_helpers` is the shared utilities module (path resolution,
//! CRC32, golden-header ops) — the Rust equivalent of
//! `tests/core/test_helpers.h` for migrated harnesses;
//! `lua` is the Task 1 Lua runner — replaces `tests/lua_runner.cpp`
//! (headless Lua script runner with C-level stdout/stderr capture).

pub mod blargg;
pub mod direct;
pub mod lua;
pub mod mapper_byte_diff;
pub mod rom_regression;
pub mod savestate_regression;
pub mod scheduler;
pub mod test_helpers;
