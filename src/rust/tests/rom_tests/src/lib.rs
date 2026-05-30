//! ROM regression test runner for the Rust side of FCEUX11.
//!
//! This crate is a placeholder for future pure-Rust core integration tests.
//! It re-exports the utility and media crates so that ROM-level tests can
//! verify Rust behaviour independently of the C++ core.
//!
//! As of v0.2.15 the C++ core still owns emulation, so the actual frame
//! hashing happens in `src/tests/rom_regression_test.cpp`.  Once the CPU /
//! PPU / APU are migrated to Rust, the hashing logic will move here.

pub use fceux11_utils;
pub use fceux11_media;

#[cfg(test)]
mod tests {
    use super::*;

    /// Sanity check: ensure the workspace crates are reachable.
    #[test]
    fn test_workspace_deps_reachable() {
        // crc32 module should be present
        let _ = fceux11_utils::crc32::fceux11_rust_crc32;
    }

    /// Placeholder for future nestest runner.
    #[test]
    fn test_nestest_placeholder() {
        // TODO(v0.3.x): Load nestest.nes in a pure-Rust bus, run to $C66E,
        // and assert $02 == 0x00 && $03 == 0x00.
    }
}
