//! Shared in-process (direct) execution core.
//!
//! Drives every ROM-based test in the manifest through an in-process
//! adapter (`load` → `step` × N → `read_oracle_probe` → `reset`) and
//! returns one `TestResult` per ROM test.
//!
//! Task 4 (FCEUX11-1.17_计划.md §5.3 step 2): this is the single
//! execution core shared by both callers that previously duplicated the
//! loop — the CLI `--direct` mode (`cli::run_direct`) and the C-ABI
//! entry point (`direct_entry::kagami_qa_direct_main`, consumed by
//! `tests/kagami_direct_main.cpp`). Keeping one implementation here means
//! frame-stepping / `$6000` probing behaviour cannot drift between them.
//!
//! ## Task 4 watchdog (FCEUX11-1.17_计划.md §5.3 step 7)
//!
//! Direct mode runs in-process, so a single broken test must not take
//! down the whole run:
//! - **Panic isolation**: each test runs under `catch_unwind`; a Rust
//!   panic is recorded as that test's FAIL and the remaining tests
//!   continue.
//! - **Timeout detection**: elapsed time per test is measured against the
//!   manifest `timeout_seconds`; a test that overruns its budget is
//!   scored FAIL with a `timeout (direct)` note (consistent with the
//!   subprocess path, which kills on timeout).
//! - **Honest limit**: a hard crash in the C++ core (segfault) or a
//!   stuck FFI call that never returns cannot be preempted from within
//!   the process. Full isolation for those cases requires running each
//!   direct test in its own subprocess (planned as an optional extension;
//!   costs frame-level interactive debugging).

use std::collections::BTreeMap;
use std::time::Instant;

use crate::adapter::trait_def::{InputSpec, SutAdapter, TestResult};
use crate::manifest::schema::TestManifest;

/// Run every ROM-driven test in the manifest through a direct adapter.
///
/// Only entries with `input.rom` set are driven here — in-process mode
/// targets Oracle B hardware tests; script-driven entries remain on the
/// subprocess path. Every outcome (including load / step / probe errors,
/// panics and timeouts) is recorded as a `TestResult` so the caller can
/// count verdicts and blocking failures without re-interpreting raw
/// adapter state.
pub fn run_direct_rom_tests(
    adapter: &mut dyn SutAdapter,
    manifest: &BTreeMap<String, TestManifest>,
) -> Vec<TestResult> {
    let mut results = Vec::new();

    for (id, test) in manifest {
        if test.input.rom.is_none() {
            continue;
        }
        let id = id.clone();
        let test = test.clone();

        // Task 4 step 7 ②: panic isolation. A panicking adapter/test is
        // recorded as FAIL; the run continues with the remaining tests.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_one(adapter, &id, &test)
        }));

        let mut result = match outcome {
            Ok(r) => r,
            Err(_) => direct_fail(
                &id,
                "panic: test panicked, isolated by catch_unwind (Task 4 watchdog)",
            ),
        };

        // Task 4 step 7 ①: per-test timeout detection. Direct mode cannot
        // preempt a stuck FFI call, but a test that overruns its manifest
        // budget must still be scored FAIL (parity with subprocess kill).
        let budget_ms = test.timeout_seconds.saturating_mul(1000);
        if test.timeout_seconds > 0 && result.duration_ms > budget_ms {
            result.passed = false;
            result.migration_note = Some(format!(
                "timeout (direct): exceeded {}s, elapsed {}ms",
                test.timeout_seconds, result.duration_ms
            ));
        }

        results.push(result);
    }

    results
}

/// Drive a single ROM test through the adapter, returning its verdict
/// with the measured duration.
///
/// v1.17 H-1: honours `spec.reset_after` (manifest `reset_after` field):
/// - `-1` (default): step all `frames`, then probe. Unchanged behaviour.
/// - `0`: load → reset → step all `frames` → probe.
/// - `N > 0`: load → step N → reset → step (frames - N) → probe.
/// `N >= frames` collapses to load → step N (= frames) → reset → probe 0
/// (the post-reset probe happens before any post-reset stepping — same as
/// the C++ `blargg_runner --reset-after` semantics; the post-reset value
/// reflects state immediately after the soft reset, not after running).
fn run_one(adapter: &mut dyn SutAdapter, id: &str, test: &TestManifest) -> TestResult {
    let start = Instant::now();
    let spec = InputSpec::from_manifest(test);

    let mut result = match adapter.load(&spec) {
        Ok(()) => {
            // Plan the step sequence honouring reset_after.
            let frames = spec.frames;
            let reset_at = if spec.reset_after < 0 {
                // -1 → no mid-run reset
                None
            } else if spec.reset_after == 0 {
                // 0 → reset immediately, then step all frames
                Some(0)
            } else if spec.reset_after as u64 >= frames as u64 {
                // reset_after >= frames → reset after stepping all frames,
                // then probe post-reset value (rare; mainly useful for ROMs
                // whose $6000 only updates right after a reset)
                Some(frames)
            } else {
                Some(spec.reset_after as u32)
            };

            let step_ok = step_with_optional_reset(adapter, frames, reset_at);
            if step_ok {
                match adapter.read_oracle_probe(spec.probe_addr) {
                    Ok(val) => {
                        let is_pass = val == 0x00;
                        TestResult {
                            test_id: id.to_string(),
                            passed: is_pass,
                            exit_code: if is_pass { 0 } else { 1 },
                            stdout: format!(
                                "BLARGG_RESULT: rom={} value=0x{:02X} status={}",
                                id,
                                val,
                                if is_pass { "PASS" } else { "FAIL" }
                            ),
                            stderr: String::new(),
                            duration_ms: 0,
                            migration_note: Some("direct-adapter".into()),
                        }
                    }
                    Err(e) => direct_fail(id, format!("probe_error: {:?}", e)),
                }
            } else {
                direct_fail(id, "step_error")
            }
        }
        Err(e) => direct_fail(id, format!("load_error: {:?}", e)),
    };

    let _ = adapter.reset();
    result.duration_ms = start.elapsed().as_millis() as u64;
    result
}

/// Step `frames` total frames, inserting one soft reset at `reset_at` if
/// Some(N). `reset_at == 0` resets before any stepping; `reset_at ==
/// frames` resets after all stepping (then the caller probes post-reset).
/// Returns false on the first step error.
fn step_with_optional_reset(
    adapter: &mut dyn SutAdapter,
    frames: u32,
    reset_at: Option<u32>,
) -> bool {
    if let Some(0) = reset_at {
        if adapter.reset().is_err() {
            return false;
        }
    }
    for f in 0..frames {
        if adapter.step().is_err() {
            return false;
        }
        if Some(f + 1) == reset_at {
            if adapter.reset().is_err() {
                return false;
            }
        }
    }
    true
}

/// Build a FAIL result for a direct-mode execution failure.
fn direct_fail(id: &str, note: impl Into<String>) -> TestResult {
    TestResult {
        test_id: id.to_string(),
        passed: false,
        exit_code: 1,
        stdout: String::new(),
        stderr: String::new(),
        duration_ms: 0,
        migration_note: Some(note.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::QaError;
    use crate::manifest::schema::{
        ExpectedResult, FailureSeverity, OracleType, TestInput, TestLayer,
    };

    fn make_test(id: &str, timeout_seconds: u64) -> TestManifest {
        TestManifest {
            id: id.into(),
            description: String::new(),
            oracle_type: OracleType::B,
            layer: TestLayer::Core,
            input: TestInput {
                rom: Some("fixtures/test.nes".into()),
                probe_addr: Some(0x6000),
                args: vec!["--frames".into(), "10".into()],
                ..Default::default()
            },
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: None,
            },
            timeout_seconds,
            tags: vec!["direct-test".into()],
            failure_means: FailureSeverity::Blocking,
            provenance: "test".into(),
        }
    }

    fn make_test_with_reset(id: &str, frames: u32, reset_after: i64) -> TestManifest {
        let mut t = make_test(id, 30);
        t.input.args = vec!["--frames".into(), frames.to_string()];
        t.input.reset_after = reset_after;
        t
    }

    fn manifest_of(tests: Vec<TestManifest>) -> BTreeMap<String, TestManifest> {
        tests.into_iter().map(|t| (t.id.clone(), t)).collect()
    }

    /// Adapter whose `load` always panics — exercises panic isolation.
    struct PanicOnLoadAdapter;

    impl SutAdapter for PanicOnLoadAdapter {
        fn init(&self, _config: &crate::core::QaConfig) -> Result<(), QaError> {
            Ok(())
        }
        fn run_test(&self, _test: &TestManifest) -> Result<TestResult, QaError> {
            unimplemented!("direct path never calls run_test")
        }
        fn load(&mut self, _input: &InputSpec) -> Result<(), QaError> {
            panic!("boom: load panicked (watchdog test)")
        }
    }

    /// Adapter returning a fixed probe value, optionally slow per step.
    struct FixedAdapter {
        val: u8,
        step_sleep_ms: u64,
        /// Counts of `step()` and `reset()` calls — used by reset_after tests.
        step_count: u32,
        reset_count: u32,
    }

    impl FixedAdapter {
        fn new(val: u8) -> Self {
            Self { val, step_sleep_ms: 0, step_count: 0, reset_count: 0 }
        }
    }

    impl SutAdapter for FixedAdapter {
        fn init(&self, _config: &crate::core::QaConfig) -> Result<(), QaError> {
            Ok(())
        }
        fn run_test(&self, _test: &TestManifest) -> Result<TestResult, QaError> {
            unimplemented!("direct path never calls run_test")
        }
        fn load(&mut self, _input: &InputSpec) -> Result<(), QaError> {
            Ok(())
        }
        fn step(&mut self) -> Result<(), QaError> {
            self.step_count += 1;
            if self.step_sleep_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(self.step_sleep_ms));
            }
            Ok(())
        }
        fn read_oracle_probe(&self, _addr: u32) -> Result<u8, QaError> {
            Ok(self.val)
        }
        fn reset(&mut self) -> Result<(), QaError> {
            self.reset_count += 1;
            Ok(())
        }
    }

    #[test]
    fn pass_and_fail_are_recorded() {
        let manifest = manifest_of(vec![
            make_test("pass_test", 30),
            make_test("fail_test", 30),
        ]);
        let mut adapter = FixedAdapter::new(0x01);
        // First run: val 0x01 → both FAIL.
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| !r.passed));

        // Second run: val 0x00 → both PASS.
        adapter.val = 0x00;
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn panic_is_isolated_and_run_continues() {
        let manifest = manifest_of(vec![
            make_test("a", 30),
            make_test("b", 30),
        ]);
        let mut adapter = PanicOnLoadAdapter;
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        // Both tests must be recorded as FAIL (not a process abort).
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| !r.passed));
        assert!(
            results
                .iter()
                .all(|r| r.migration_note.as_deref().unwrap_or("").contains("panic")),
            "each panic must be noted, got: {:?}",
            results.iter().map(|r| r.migration_note.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn timeout_overrun_is_scored_fail() {
        // timeout_seconds = 1; each step sleeps 250ms × 10 frames = 2.5s.
        let manifest = manifest_of(vec![make_test("slow_test", 1)]);
        let mut adapter = FixedAdapter { val: 0x00, step_sleep_ms: 250, step_count: 0, reset_count: 0 };
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(results.len(), 1);
        let r = &results[0];
        assert!(!r.passed, "overrun must be scored FAIL despite 0x00 probe");
        assert!(
            r.migration_note.as_deref().unwrap_or("").contains("timeout"),
            "note must mention timeout, got: {:?}",
            r.migration_note
        );
        assert!(r.duration_ms >= 1000, "duration should reflect actual run");
    }

    #[test]
    fn fast_test_within_budget_stays_pass() {
        let manifest = manifest_of(vec![make_test("fast_test", 30)]);
        let mut adapter = FixedAdapter::new(0x00);
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
        assert!(results[0].migration_note.as_deref() == Some("direct-adapter"));
    }

    #[test]
    fn non_rom_entries_are_skipped() {
        let mut t = make_test("script_test", 30);
        t.input.rom = None;
        let manifest = manifest_of(vec![t]);
        let mut adapter = FixedAdapter::new(0x00);
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        assert!(results.is_empty());
    }

    // ------------------------------------------------------------------
    // v1.17 H-1: reset_after behaviour
    // ------------------------------------------------------------------

    /// Default reset_after = -1 → no mid-run reset. step() runs all frames,
    /// and the only `reset()` call is the cleanup at the end of `run_one`.
    #[test]
    fn reset_after_default_minus_one_no_mid_run_reset() {
        let manifest = manifest_of(vec![make_test_with_reset("a", 10, -1)]);
        let mut adapter = FixedAdapter::new(0x00);
        let _ = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(adapter.step_count, 10, "all 10 frames stepped");
        assert_eq!(adapter.reset_count, 1, "only cleanup reset at end");
    }

    /// reset_after = 0 → load → reset → step all frames → probe.
    /// 1 mid-run reset + 1 cleanup reset = 2 total resets.
    #[test]
    fn reset_after_zero_resets_before_stepping() {
        let manifest = manifest_of(vec![make_test_with_reset("a", 10, 0)]);
        let mut adapter = FixedAdapter::new(0x00);
        let _ = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(adapter.step_count, 10, "all 10 frames still stepped");
        assert_eq!(adapter.reset_count, 2, "1 mid-run + 1 cleanup reset");
    }

    /// reset_after = N > 0 → step N, reset, step (frames - N), probe.
    /// mid-run reset + cleanup reset.
    #[test]
    fn reset_after_mid_value_inserts_reset() {
        let manifest = manifest_of(vec![make_test_with_reset("a", 10, 4)]);
        let mut adapter = FixedAdapter::new(0x00);
        let _ = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(adapter.step_count, 10, "all 10 frames stepped (4+6)");
        assert_eq!(adapter.reset_count, 2, "1 mid-run + 1 cleanup reset");
    }

    /// reset_after > frames → step all frames, then reset, then probe.
    /// mid-run reset is scheduled after the last step so it acts as a
    /// post-run reset. The probe happens after this reset, not before.
    #[test]
    fn reset_after_larger_than_frames_resets_at_end() {
        let manifest = manifest_of(vec![make_test_with_reset("a", 5, 100)]);
        let mut adapter = FixedAdapter::new(0x00);
        let _ = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(adapter.step_count, 5, "all 5 frames stepped");
        assert_eq!(adapter.reset_count, 2, "1 mid-run reset (after step 5) + 1 cleanup");
    }

    /// reset_after exactly == frames → same as reset_after > frames.
    #[test]
    fn reset_after_equal_to_frames() {
        let manifest = manifest_of(vec![make_test_with_reset("a", 5, 5)]);
        let mut adapter = FixedAdapter::new(0x00);
        let _ = run_direct_rom_tests(&mut adapter, &manifest);
        assert_eq!(adapter.step_count, 5);
        assert_eq!(adapter.reset_count, 2);
    }

    /// Multiple tests with mixed reset_after values run independently —
    /// each test gets its own step/reset accounting.
    #[test]
    fn mixed_reset_after_across_tests() {
        let manifest = manifest_of(vec![
            make_test_with_reset("no_reset", 10, -1),
            make_test_with_reset("reset_zero", 10, 0),
            make_test_with_reset("reset_mid", 10, 3),
        ]);
        let mut adapter = FixedAdapter::new(0x00);
        let _ = run_direct_rom_tests(&mut adapter, &manifest);
        // 3 tests × 10 frames each = 30 steps
        assert_eq!(adapter.step_count, 30);
        // reset counts: 1 (default cleanup only) + 2 (zero) + 2 (mid) = 5
        assert_eq!(adapter.reset_count, 5);
    }
}
