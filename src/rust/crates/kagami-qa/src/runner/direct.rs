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
fn run_one(adapter: &mut dyn SutAdapter, id: &str, test: &TestManifest) -> TestResult {
    let start = Instant::now();
    let spec = InputSpec::from_manifest(test);

    let mut result = match adapter.load(&spec) {
        Ok(()) => {
            let mut step_ok = true;
            for _f in 0..spec.frames {
                if adapter.step().is_err() {
                    step_ok = false;
                    break;
                }
            }
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
            if self.step_sleep_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(self.step_sleep_ms));
            }
            Ok(())
        }
        fn read_oracle_probe(&self, _addr: u32) -> Result<u8, QaError> {
            Ok(self.val)
        }
        fn reset(&mut self) -> Result<(), QaError> {
            Ok(())
        }
    }

    #[test]
    fn pass_and_fail_are_recorded() {
        let manifest = manifest_of(vec![
            make_test("pass_test", 30),
            make_test("fail_test", 30),
        ]);
        let mut adapter = FixedAdapter { val: 0x01, step_sleep_ms: 0 };
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
        let mut adapter = FixedAdapter { val: 0x00, step_sleep_ms: 250 };
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
        let mut adapter = FixedAdapter { val: 0x00, step_sleep_ms: 0 };
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
        let mut adapter = FixedAdapter { val: 0x00, step_sleep_ms: 0 };
        let results = run_direct_rom_tests(&mut adapter, &manifest);
        assert!(results.is_empty());
    }
}
