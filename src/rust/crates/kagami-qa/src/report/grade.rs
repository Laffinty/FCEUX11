//! Release-readiness grading (A–E) — machine-computable pass standard.
//!
//! Task 5 (FCEUX11-1.17_计划.md §6): every test run produces a single
//! grade from the migration matrix + manifest. Grades are monotone gates:
//!
//! | Grade | Meaning            | Rule (all must hold to reach this grade)          |
//! |-------|--------------------|---------------------------------------------------|
//! | A     | perfect            | no failures, no regression, nothing skipped        |
//! | B     | release standard   | no blocking fail, no regression, every remaining  |
//! |       |                    | failure is within the frozen baseline              |
//! | C     | acceptable release | no blocking fail, no regression, all failures are  |
//! |       |                    | advisory (documented known-limits)                 |
//! | D     | no release         | any blocking failure or any PASS→FAIL regression   |
//! | E     | core broken        | engine boot test (smoke/headless) failed           |
//!
//! Key design points:
//! - B vs C is decided by the *frozen baseline*: with a baseline whose
//!   transition matrix shows every failure as `fail_to_fail`, the run is B;
//!   a failure not present in the baseline (`new_test` with curr=FAIL) or
//!   the absence of any baseline caps the grade at C. This keeps the
//!   "current v1.16 baseline = C" verdict honest: CI runs without
//!   `--baseline`, so advisory known-limits cannot be proven frozen.
//! - The grade uses the same immutable matrix data as the R4 gate — no
//!   separate judgment channel, no way to game it beyond the anti-gaming
//!   rules already enforced on the manifest.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::manifest::schema::{FailureSeverity, TestManifest};

use super::matrix::{MigrationMatrix, TransitionEntry};

/// Release-readiness grade. Serialises as its letter (`"A"` … `"E"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub enum Grade {
    /// Perfect pass — every test green, nothing skipped.
    A,
    /// Meets release standard — failures are all within the frozen baseline.
    B,
    /// Acceptable release standard — advisory known-limits only.
    #[default]
    C,
    /// Not allowed to release — blocking failure or regression.
    D,
    /// Basic functionality damaged — engine boot test failed.
    E,
}

impl Grade {
    /// Single-letter label (`"A"` … `"E"`).
    pub fn label(&self) -> &'static str {
        match self {
            Grade::A => "A",
            Grade::B => "B",
            Grade::C => "C",
            Grade::D => "D",
            Grade::E => "E",
        }
    }

    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Grade::A => "perfect",
            Grade::B => "release",
            Grade::C => "acceptable",
            Grade::D => "blocked",
            Grade::E => "broken",
        }
    }
}

/// Compute the release grade for a run and the reasons it did not reach
/// higher grades (empty for A).
pub fn compute_grade(
    matrix: &MigrationMatrix,
    manifest: &BTreeMap<String, TestManifest>,
) -> (Grade, Vec<String>) {
    let mut reasons = Vec::new();

    // ------------------------------------------------------------------
    // E — engine boot test failed (smoke_test / headless_smoke_test, or
    // any entry tagged `engine-boot`).
    // ------------------------------------------------------------------
    let boot_failed: Vec<&str> = matrix
        .details
        .iter()
        .filter(|d| !d.passed && is_engine_boot_test(&d.test_id, manifest))
        .map(|d| d.test_id.as_str())
        .collect();
    if !boot_failed.is_empty() {
        reasons.push(format!(
            "engine boot test(s) failed: {}",
            boot_failed.join(", ")
        ));
        return (Grade::E, reasons);
    }

    // ------------------------------------------------------------------
    // D — any blocking failure, or any PASS→FAIL regression.
    // ------------------------------------------------------------------
    let blocking_failed: Vec<&str> = matrix
        .details
        .iter()
        .filter(|d| {
            !d.passed
                && manifest
                    .get(&d.test_id)
                    .map(|t| t.failure_means == FailureSeverity::Blocking)
                    .unwrap_or(false)
        })
        .map(|d| d.test_id.as_str())
        .collect();
    if !blocking_failed.is_empty() {
        reasons.push(format!(
            "{} blocking test(s) failed: {}",
            blocking_failed.len(),
            blocking_failed.join(", ")
        ));
        return (Grade::D, reasons);
    }
    if !matrix.transition_matrix.pass_to_fail.is_empty() {
        reasons.push(format!(
            "{} PASS→FAIL regression(s): {}",
            matrix.transition_matrix.pass_to_fail.len(),
            matrix
                .transition_matrix
                .pass_to_fail
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        return (Grade::D, reasons);
    }

    // ------------------------------------------------------------------
    // A — perfect pass (no failures at all, nothing skipped).
    // ------------------------------------------------------------------
    if matrix.summary.failed == 0 && matrix.summary.skipped == 0 {
        return (Grade::A, reasons);
    }

    // ------------------------------------------------------------------
    // B vs C — advisory known-limits within the frozen baseline?
    // ------------------------------------------------------------------
    let new_fails: Vec<&TransitionEntry> = matrix
        .transition_matrix
        .new_test
        .iter()
        .filter(|e| e.curr == "FAIL")
        .collect();
    if !new_fails.is_empty() {
        reasons.push(format!(
            "{} new failing test(s) not in frozen baseline: {}",
            new_fails.len(),
            new_fails
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        return (Grade::C, reasons);
    }

    // `test_set_diff` is only present when a baseline was supplied. Its
    // absence means we cannot prove the remaining advisory failures are
    // within the frozen set — conservative cap at C.
    let baseline_supplied = matrix.test_set_diff.is_some();
    if matrix.summary.failed > 0 {
        if baseline_supplied {
            reasons.push(format!(
                "{} advisory known-limit failure(s), all within frozen baseline",
                matrix.summary.failed
            ));
            return (Grade::B, reasons);
        }
        reasons.push(format!(
            "{} advisory known-limit failure(s); no baseline supplied to verify they are within the frozen set",
            matrix.summary.failed
        ));
        return (Grade::C, reasons);
    }

    // Unreachable in practice (failed == 0 handled by A); kept as a
    // conservative fallback.
    (Grade::C, reasons)
}

/// Is this test an engine-boot gate? (`smoke_test` / `headless_smoke_test`
/// by id, or any entry carrying the `engine-boot` tag.)
fn is_engine_boot_test(id: &str, manifest: &BTreeMap<String, TestManifest>) -> bool {
    if id == "smoke_test" || id == "headless_smoke_test" {
        return true;
    }
    manifest
        .get(id)
        .map(|t| t.tags.iter().any(|tag| tag == "engine-boot"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::trait_def::TestResult;
    use crate::manifest::schema::{
        ExpectedResult, OracleType, TestInput, TestLayer,
    };
    use crate::report::matrix::{build_matrix, PreviousRun};

    fn make_result(id: &str, passed: bool) -> TestResult {
        TestResult {
            test_id: id.into(),
            passed,
            exit_code: if passed { 0 } else { 1 },
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 100,
            migration_note: None,
        }
    }

    fn make_manifest_entry(id: &str, blocking: bool, tags: &[&str]) -> TestManifest {
        TestManifest {
            id: id.into(),
            description: String::new(),
            oracle_type: OracleType::A,
            layer: TestLayer::Core,
            input: TestInput::default(),
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: None,
            },
            timeout_seconds: 60,
            tags: tags.iter().map(|s| s.to_string()).collect(),
            failure_means: if blocking {
                FailureSeverity::Blocking
            } else {
                FailureSeverity::Advisory
            },
            provenance: "test".into(),
        }
    }

    fn manifest_with(entries: Vec<TestManifest>) -> BTreeMap<String, TestManifest> {
        entries.into_iter().map(|t| (t.id.clone(), t)).collect()
    }

    fn build(
        results: Vec<(String, bool)>,
        prev: Option<BTreeMap<String, bool>>,
    ) -> MigrationMatrix {
        let results: Vec<TestResult> = results
            .into_iter()
            .map(|(id, passed)| make_result(&id, passed))
            .collect();
        let previous = prev.map(|results| PreviousRun {
            run_id: "prev".into(),
            generated_at: "2026-08-08T00:00:00Z".into(),
            results,
        });
        build_matrix(results, &Default::default(), &Default::default(), previous.as_ref(), vec![])
    }

    #[test]
    fn all_green_no_baseline_is_a() {
        let matrix = build(vec![("a".into(), true), ("b".into(), true)], None);
        let manifest = manifest_with(vec![
            make_manifest_entry("a", true, &[]),
            make_manifest_entry("b", true, &[]),
        ]);
        let (grade, reasons) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::A);
        assert!(reasons.is_empty());
    }

    #[test]
    fn advisory_fails_without_baseline_is_c() {
        let matrix = build(vec![("a".into(), true), ("known_limit".into(), false)], None);
        let manifest = manifest_with(vec![
            make_manifest_entry("a", true, &[]),
            make_manifest_entry("known_limit", false, &["known-limit"]),
        ]);
        let (grade, reasons) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::C);
        assert!(!reasons.is_empty());
    }

    #[test]
    fn advisory_fails_within_frozen_baseline_is_b() {
        // Baseline: known_limit was already FAIL → current FAIL = fail_to_fail.
        let mut prev = BTreeMap::new();
        prev.insert("a".into(), true);
        prev.insert("known_limit".into(), false);
        let matrix = build(vec![("a".into(), true), ("known_limit".into(), false)], Some(prev));
        let manifest = manifest_with(vec![
            make_manifest_entry("a", true, &[]),
            make_manifest_entry("known_limit", false, &["known-limit"]),
        ]);
        let (grade, _) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::B);
    }

    #[test]
    fn new_failing_test_is_c_even_with_baseline() {
        // Baseline did not contain "brand_new" → it lands in new_test(FAIL).
        let mut prev = BTreeMap::new();
        prev.insert("a".into(), true);
        let matrix = build(
            vec![("a".into(), true), ("brand_new".into(), false)],
            Some(prev),
        );
        let manifest = manifest_with(vec![
            make_manifest_entry("a", true, &[]),
            make_manifest_entry("brand_new", false, &[]),
        ]);
        let (grade, reasons) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::C);
        assert!(reasons.iter().any(|r| r.contains("new failing")));
    }

    #[test]
    fn blocking_failure_is_d() {
        let matrix = build(vec![("a".into(), true), ("blocked".into(), false)], None);
        let manifest = manifest_with(vec![
            make_manifest_entry("a", true, &[]),
            make_manifest_entry("blocked", true, &[]),
        ]);
        let (grade, reasons) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::D);
        assert!(reasons.iter().any(|r| r.contains("blocking")));
    }

    #[test]
    fn regression_is_d() {
        // Baseline: a was PASS, now FAIL → pass_to_fail. The entry is
        // ADVISORY so the regression rule (not the blocking rule) is what
        // triggers grade D — regression alone is release-blocking.
        let mut prev = BTreeMap::new();
        prev.insert("a".into(), true);
        let matrix = build(vec![("a".into(), false)], Some(prev));
        let manifest = manifest_with(vec![make_manifest_entry("a", false, &[])]);
        let (grade, reasons) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::D);
        assert!(reasons.iter().any(|r| r.contains("regression")));
    }

    #[test]
    fn engine_boot_failure_is_e() {
        let matrix = build(vec![("smoke_test".into(), false)], None);
        let manifest = manifest_with(vec![make_manifest_entry("smoke_test", true, &[])]);
        let (grade, reasons) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::E);
        assert!(reasons.iter().any(|r| r.contains("engine boot")));
    }

    #[test]
    fn engine_boot_tag_is_respected() {
        let matrix = build(vec![("boot_gate".into(), false)], None);
        let manifest = manifest_with(vec![make_manifest_entry("boot_gate", false, &["engine-boot"])]);
        let (grade, _) = compute_grade(&matrix, &manifest);
        assert_eq!(grade, Grade::E);
    }

    #[test]
    fn labels_and_names() {
        assert_eq!(Grade::A.label(), "A");
        assert_eq!(Grade::A.name(), "perfect");
        assert_eq!(Grade::C.label(), "C");
        assert_eq!(Grade::C.name(), "acceptable");
        assert_eq!(Grade::E.name(), "broken");
    }
}
