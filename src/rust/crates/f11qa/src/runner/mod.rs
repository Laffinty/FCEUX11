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
// `direct` is the C-ABI FFI runner; only available with the
// "direct-adapter" feature (see adapter/mod.rs for rationale).
#[cfg(feature = "direct-adapter")]
pub mod direct;
// `lua` declares FFI to fceux11-lua + kagami_bridge C symbols that only
// the CMake `direct-adapter` build links against. Plain cargo / cargo test
// excludes it so the linker never sees the unresolved externs.
#[cfg(feature = "direct-adapter")]
pub mod lua;
pub mod mapper_byte_diff;
pub mod rom_regression;
pub mod savestate_regression;
pub mod scheduler;
pub mod test_helpers;

// =========================================================================
// A-2 (2026-11-19, FCEUX11-遗留问题盘点与优化计划-2026-11-18.md §二 A-2):
// direct-mode honesty guard.
//
// Direct mode only drives manifest entries that carry `input.rom`; the
// artifact `tests/tests.json` has 123 cases and every one of them is
// subprocess-style (`input.binary`), so a direct run executed 0 cases
// and still exited 0 — `f11qa_direct_smoke` was judged "Passed" while
// nothing ran (v2.0.5 r13 ⑥ vacuous-gate observation).
//
// This function is the refusal: a non-empty manifest that executed zero
// tests must not pass. It is deliberately a *pure* function living
// OUTSIDE the `direct-adapter`-gated `direct` module so its unit tests
// run under the gate command `cargo test -p f11qa --lib` (which builds
// without the feature — a lock test behind the feature gate would never
// execute there; discipline 10). Both direct entry points call it:
// `direct_entry::kagami_qa_direct_main` (src/lib.rs) and the CLI
// `--direct` path (src/cli/run_direct.rs).
//
// The error message is a fixed English string; the ctest entry
// `f11qa_direct_smoke` (tests/CMakeLists.txt) matches the
// "executed 0 of" fragment via FAIL_REGULAR_EXPRESSION, so keep that
// wording stable.
pub fn direct_mode_honesty_guard(manifest_len: usize, executed: usize) -> Result<(), String> {
    if manifest_len > 0 && executed == 0 {
        Err(format!(
            "f11qa direct mode executed 0 of {} manifest cases (no input.rom entries); refusing to pass",
            manifest_len
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod direct_honesty_guard_tests {
    use super::direct_mode_honesty_guard;

    /// (a) The shipped artifact shape: 123 subprocess-style cases, none
    /// driven in-process — the vacuous pass A-2 closes. The guard must
    /// refuse and carry the ctest-matched keywords.
    #[test]
    fn nonempty_manifest_with_zero_executed_is_refused() {
        let err = direct_mode_honesty_guard(123, 0)
            .err()
            .expect("123 manifest cases / 0 executed must be refused");
        assert!(
            err.contains("executed 0 of"),
            "message must carry the ctest-matched keyword, got: {}",
            err
        );
        assert!(
            err.contains("refusing to pass"),
            "message must state the refusal, got: {}",
            err
        );
        assert!(
            err.contains("no input.rom entries"),
            "message must state the cause, got: {}",
            err
        );
    }

    /// (b) A manifest that actually drove (some of) its cases is honest.
    #[test]
    fn nonempty_manifest_with_executed_cases_is_ok() {
        assert!(direct_mode_honesty_guard(2, 2).is_ok());
        assert!(direct_mode_honesty_guard(10, 1).is_ok());
    }

    /// (c) An empty manifest is not this guard's concern (the blargg
    /// batch path already refuses empty manifests separately); do not
    /// misfire on it.
    #[test]
    fn empty_manifest_is_not_this_guards_concern() {
        assert!(direct_mode_honesty_guard(0, 0).is_ok());
    }
}
