//! StateRecorder (rewind / savestate history) — placeholder for v1.10 migration.
//!
//! The C++ `StateRecorder` class in `src/state.cpp` remains in place for v1.9.
//! Rust-side state-file management (`state_file.rs`) already replaces the
//! compression and envelope layers that StateRecorder depends on.
//!
//! v1.9 Chronicle: state_file.rs now supports V2 (FCEU11ST) format with
//! per-chunk CRC32. StateRecorder continues to use V1 format internally
//! for its ring buffer snapshots. Full migration to Rust deferred to v1.10.
