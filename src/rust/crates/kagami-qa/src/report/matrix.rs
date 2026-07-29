// KagamiQA report module — SWE-bench isomorphic migration matrix.
//
// Implements the full §八 JSON report format:
//   run_id + engine → summary → transition_matrix → oracle_breakdown
//   → baseline_drift → details
//
// The transition_matrix is the core field: it compares the current run
// against a previous baseline and classifies every test into one of
// FAIL_TO_PASS / PASS_TO_PASS / PASS_TO_FAIL / FAIL_TO_FAIL.
// AI agents read this field to determine "did the patch help or hurt?"

use serde::Serialize;
use std::collections::BTreeMap;

use crate::adapter::trait_def::TestResult;

// ---------------------------------------------------------------------------
// Engine metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EngineInfo {
    pub version: String,
    pub toolchain: String,
    pub git_rev: String,
}

impl Default for EngineInfo {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            toolchain: "msvc-19.x".to_string(),
            git_rev: option_env!("FCEUX11_GIT_REV").unwrap_or("unknown").to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MatrixSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

// ---------------------------------------------------------------------------
// Transition matrix — the core field for AI agents and human reviewers.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TransitionMatrix {
    /// Tests that went from FAIL (baseline) to PASS (current): progress!
    pub fail_to_pass: Vec<TransitionEntry>,
    /// Tests that stayed PASS → PASS: stable baseline, no regression.
    pub pass_to_pass: Vec<TransitionEntry>,
    /// Tests that went from PASS (baseline) to FAIL (current): REGRESSION!
    pub pass_to_fail: Vec<TransitionEntry>,
    /// Tests that stayed FAIL → FAIL: known failures, not yet fixed.
    pub fail_to_fail: Vec<TransitionEntry>,
    /// Tests whose `test_id` does NOT appear in the baseline at all.
    /// Separated from the 4 transition buckets because it is semantically
    /// incomparable: there is no "previous" to compare against.
    /// Stage-2 §四·五 PR 0.5-3 / 0.5-d.
    pub new_test: Vec<TransitionEntry>,
}

impl Default for TransitionMatrix {
    fn default() -> Self {
        Self {
            fail_to_pass: Vec::new(),
            pass_to_pass: Vec::new(),
            pass_to_fail: Vec::new(),
            fail_to_fail: Vec::new(),
            new_test: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TransitionEntry {
    pub id: String,
    pub prev: String, // "PASS" or "FAIL"
    pub curr: String, // "PASS" or "FAIL"
}

// ---------------------------------------------------------------------------
// Oracle breakdown — separate accounting for A (regression) vs B (hardware).
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct OracleBreakdown {
    #[serde(rename = "A_regression")]
    pub a_regression: OracleStats,
    #[serde(rename = "B_hardware")]
    pub b_hardware: OracleStats,
}

#[derive(Debug, Default, Serialize)]
pub struct OracleStats {
    pub pass: usize,
    pub fail: usize,
}

// ---------------------------------------------------------------------------
// Baseline drift — detects when an expected value changed without review.
// Each drift entry is a red flag (plan §1.2 constraint 6).
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct BaselineDrift {
    pub id: String,
    pub field: String,
    pub old: String,
    pub new: String,
    /// false = unreviewed drift → red alert
    pub approved: bool,
}

// ---------------------------------------------------------------------------
// Per-test detail
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TestDetail {
    pub test_id: String,
    pub passed: bool,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub oracle_type: String,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_note: Option<String>,
}

// ---------------------------------------------------------------------------
// Top-level report
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct MigrationMatrix {
    pub report_version: u32,
    pub run_id: String,
    pub engine: EngineInfo,
    pub summary: MatrixSummary,
    pub transition_matrix: TransitionMatrix,
    pub oracle_breakdown: OracleBreakdown,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub baseline_drift: Vec<BaselineDrift>,
    pub details: Vec<TestDetail>,
}

// ---------------------------------------------------------------------------
// Previous-run snapshot (used for transition comparison).
// ---------------------------------------------------------------------------

/// Minimal snapshot of a previous run: test_id → passed.
/// Serialised alongside the full report so the next run can diff against it.
#[derive(Debug, Serialize, serde::Deserialize)]
pub struct PreviousRun {
    pub run_id: String,
    pub generated_at: String,
    pub results: BTreeMap<String, bool>, // test_id → passed
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Build a full MigrationMatrix from test results, optionally comparing
/// against a previous baseline to populate the transition_matrix.
pub fn build_matrix(
    results: Vec<TestResult>,
    oracle_types: &BTreeMap<String, String>,
    layers: &BTreeMap<String, String>,
    previous: Option<&PreviousRun>,
    drifts: Vec<BaselineDrift>,
) -> MigrationMatrix {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    let skipped = 0; // P1 doesn't implement skip; reserved for future use.

    // ---------- transition_matrix ----------
    let mut transition = TransitionMatrix::default();
    if let Some(prev) = previous {
        for r in &results {
            // Stage-2 §四·五 PR 0.5-3: tests not in baseline must NOT fall
            // into fail_to_pass (or any other 4-bucket).
            // Previously `unwrap_or(false)` made a brand-new passing test
            // count as fail_to_pass — a direct gaming vector.
            // New behaviour: route them to a 5th bucket `new_test`.
            let in_baseline = prev.results.contains_key(&r.test_id);
            let prev_passed = prev.results.get(&r.test_id).copied().unwrap_or(false);
            let curr_passed = r.passed;
            let entry = TransitionEntry {
                id: r.test_id.clone(),
                prev: if prev_passed { "PASS" } else { "FAIL" }.into(),
                curr: if curr_passed { "PASS" } else { "FAIL" }.into(),
            };
            if !in_baseline {
                transition.new_test.push(entry);
            } else {
                match (prev_passed, curr_passed) {
                    (false, true) => transition.fail_to_pass.push(entry),
                    (true, true) => transition.pass_to_pass.push(entry),
                    (true, false) => transition.pass_to_fail.push(entry),
                    (false, false) => transition.fail_to_fail.push(entry),
                }
            }
        }
    }

    // ---------- oracle_breakdown ----------
    let mut a_stats = OracleStats::default();
    let mut b_stats = OracleStats::default();
    for r in &results {
        match oracle_types.get(&r.test_id).map(String::as_str) {
            Some("A") => {
                if r.passed {
                    a_stats.pass += 1;
                } else {
                    a_stats.fail += 1;
                }
            }
            Some("B") => {
                if r.passed {
                    b_stats.pass += 1;
                } else {
                    b_stats.fail += 1;
                }
            }
            _ => {} // unknown oracle type — skip breakdown
        }
    }

    // ---------- details ----------
    let details: Vec<TestDetail> = results
        .into_iter()
        .map(|r| {
            let oracle_type = oracle_types
                .get(&r.test_id)
                .cloned()
                .unwrap_or_else(|| "?".into());
            let layer = layers
                .get(&r.test_id)
                .cloned()
                .unwrap_or_else(|| "?".into());
            TestDetail {
                test_id: r.test_id,
                passed: r.passed,
                exit_code: r.exit_code,
                duration_ms: r.duration_ms,
                oracle_type,
                layer,
                migration_note: r.migration_note,
            }
        })
        .collect();

    MigrationMatrix {
        report_version: 1,
        run_id: generate_run_id(),
        engine: EngineInfo::default(),
        summary: MatrixSummary {
            total,
            passed,
            failed,
            skipped,
        },
        transition_matrix: transition,
        oracle_breakdown: OracleBreakdown {
            a_regression: a_stats,
            b_hardware: b_stats,
        },
        baseline_drift: drifts,
        details,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a unique run ID: YYYYMMDD-HHMMSS-xxxxxx
fn generate_run_id() -> String {
    use std::time::SystemTime;

    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let nsecs = dur.subsec_nanos();

    // Days since Unix epoch → calendar date (same algorithm as chrono_now).
    let days = (secs / 86400) as i64;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let mins = (time_of_day % 3600) / 60;
    let secs_rem = time_of_day % 60;

    let (y, m, d) = days_since_epoch_to_ymd(days);

    // Use nanoseconds modulo 0xFFFFFF as a pseudo-random suffix.
    let suffix = nsecs & 0xFF_FFFF;

    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}-{:06x}",
        y, m, d, hours, mins, secs_rem, suffix
    )
}

/// Civil date from days since Unix epoch (Howard Hinnant algorithm).
fn days_since_epoch_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::trait_def::TestResult;

    fn make_result(id: &str, passed: bool, exit_code: i32) -> TestResult {
        TestResult {
            test_id: id.into(),
            passed,
            exit_code,
            stdout: String::new(),
            stderr: String::new(),
            duration_ms: 100,
            migration_note: None,
        }
    }

    // ---- Empty results ----

    #[test]
    fn empty_results() {
        let m = build_matrix(vec![], &Default::default(), &Default::default(), None, vec![]);
        assert_eq!(m.summary.total, 0);
        assert_eq!(m.summary.passed, 0);
        assert_eq!(m.summary.failed, 0);
        assert_eq!(m.details.len(), 0);
        assert!(m.transition_matrix.fail_to_pass.is_empty());
        assert!(m.baseline_drift.is_empty());
    }

    // ---- Oracle breakdown ----

    #[test]
    fn oracle_breakdown_splits_a_and_b() {
        let results = vec![
            make_result("a_pass", true, 0),
            make_result("a_fail", false, 1),
            make_result("b_pass", true, 0),
            make_result("b_fail", false, 1),
        ];
        let mut ot = BTreeMap::new();
        ot.insert("a_pass".into(), "A".into());
        ot.insert("a_fail".into(), "A".into());
        ot.insert("b_pass".into(), "B".into());
        ot.insert("b_fail".into(), "B".into());

        let m = build_matrix(results, &ot, &Default::default(), None, vec![]);
        assert_eq!(m.oracle_breakdown.a_regression.pass, 1);
        assert_eq!(m.oracle_breakdown.a_regression.fail, 1);
        assert_eq!(m.oracle_breakdown.b_hardware.pass, 1);
        assert_eq!(m.oracle_breakdown.b_hardware.fail, 1);
    }

    // ---- Transition matrix with baseline ----

    #[test]
    fn transition_matrix_with_baseline() {
        // Previous run: a was PASS, b was FAIL, c was PASS
        let mut prev_results = BTreeMap::new();
        prev_results.insert("a".into(), true);
        prev_results.insert("b".into(), false);
        prev_results.insert("c".into(), true);
        let prev = PreviousRun {
            run_id: "prev".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            results: prev_results,
        };

        // Current run: a still PASS, b now PASS (fixed!), c now FAIL (regression!)
        let results = vec![
            make_result("a", true, 0),  // PASS → PASS
            make_result("b", true, 0),  // FAIL → PASS (progress!)
            make_result("c", false, 1), // PASS → FAIL (regression!)
        ];
        let mut ot = BTreeMap::new();
        ot.insert("a".into(), "A".into());
        ot.insert("b".into(), "B".into());
        ot.insert("c".into(), "A".into());

        let m = build_matrix(results, &ot, &Default::default(), Some(&prev), vec![]);

        assert_eq!(m.transition_matrix.pass_to_pass.len(), 1);
        assert_eq!(m.transition_matrix.pass_to_pass[0].id, "a");

        assert_eq!(m.transition_matrix.fail_to_pass.len(), 1);
        assert_eq!(m.transition_matrix.fail_to_pass[0].id, "b");
        assert_eq!(m.transition_matrix.fail_to_pass[0].prev, "FAIL");
        assert_eq!(m.transition_matrix.fail_to_pass[0].curr, "PASS");

        assert_eq!(m.transition_matrix.pass_to_fail.len(), 1);
        assert_eq!(m.transition_matrix.pass_to_fail[0].id, "c");
        assert_eq!(m.transition_matrix.pass_to_fail[0].prev, "PASS");
        assert_eq!(m.transition_matrix.pass_to_fail[0].curr, "FAIL");

        assert!(m.transition_matrix.fail_to_fail.is_empty());
    }

    // ---- Transition: new test not in baseline ----
    //
    // Stage-2 §四·五 PR 0.5-3 (0.5-d): tests not in baseline must NOT be
    // bucketed into fail_to_pass or pass_to_fail. They are semantically
    // incomparable (no "previous" exists) and must land in the new `new_test`
    // bucket. Both PASS and FAIL outcomes are tested below.

    #[test]
    fn new_test_passing_goes_to_new_test_bucket() {
        let empty_prev = BTreeMap::new();
        let prev = PreviousRun {
            run_id: "prev".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            results: empty_prev,
        };
        // Brand-new test that PASSES in current run.
        let results = vec![make_result("brand_new_pass", true, 0)];
        let mut ot = BTreeMap::new();
        ot.insert("brand_new_pass".into(), "A".into());

        let m = build_matrix(results, &ot, &Default::default(), Some(&prev), vec![]);

        // CRITICAL: must NOT count toward fail_to_pass (anti-gaming guard).
        assert!(
            m.transition_matrix.fail_to_pass.is_empty(),
            "new passing test leaked into fail_to_pass"
        );
        assert!(
            m.transition_matrix.pass_to_fail.is_empty(),
            "new passing test must not be in pass_to_fail"
        );
        assert_eq!(m.transition_matrix.new_test.len(), 1);
        assert_eq!(m.transition_matrix.new_test[0].id, "brand_new_pass");
        assert_eq!(m.transition_matrix.new_test[0].curr, "PASS");
    }

    #[test]
    fn new_test_failing_also_goes_to_new_test_bucket() {
        let empty_prev = BTreeMap::new();
        let prev = PreviousRun {
            run_id: "prev".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            results: empty_prev,
        };
        // Brand-new test that FAILS in current run.
        let results = vec![make_result("brand_new_fail", false, 1)];
        let mut ot = BTreeMap::new();
        ot.insert("brand_new_fail".into(), "A".into());

        let m = build_matrix(results, &ot, &Default::default(), Some(&prev), vec![]);

        // A naive `unwrap_or(true)` swap would land this in pass_to_fail
        // (false regression alarm). Verify it does NOT.
        assert!(
            m.transition_matrix.pass_to_fail.is_empty(),
            "new failing test leaked into pass_to_fail"
        );
        assert!(
            m.transition_matrix.fail_to_fail.is_empty(),
            "new failing test must not be in fail_to_fail (no prior existed)"
        );
        assert_eq!(m.transition_matrix.new_test.len(), 1);
        assert_eq!(m.transition_matrix.new_test[0].id, "brand_new_fail");
        assert_eq!(m.transition_matrix.new_test[0].curr, "FAIL");
    }

    #[test]
    fn new_test_bucket_keeps_baseline_buckets_clean() {
        // Mixed scenario: baseline has 'a' (was PASS, still PASS); current
        // run also has a brand-new 'b' that PASSES.
        let mut prev_results = BTreeMap::new();
        prev_results.insert("a".into(), true);
        let prev = PreviousRun {
            run_id: "prev".into(),
            generated_at: "2026-01-01T00:00:00Z".into(),
            results: prev_results,
        };
        let results = vec![
            make_result("a", true, 0),  // PASS → PASS  (in baseline)
            make_result("b", true, 0),  // new PASS    (not in baseline)
        ];
        let mut ot = BTreeMap::new();
        ot.insert("a".into(), "A".into());
        ot.insert("b".into(), "A".into());

        let m = build_matrix(results, &ot, &Default::default(), Some(&prev), vec![]);

        assert_eq!(m.transition_matrix.pass_to_pass.len(), 1);
        assert_eq!(m.transition_matrix.pass_to_pass[0].id, "a");
        assert_eq!(m.transition_matrix.new_test.len(), 1);
        assert_eq!(m.transition_matrix.new_test[0].id, "b");
        // All other buckets stay clean.
        assert!(m.transition_matrix.fail_to_pass.is_empty());
        assert!(m.transition_matrix.pass_to_fail.is_empty());
        assert!(m.transition_matrix.fail_to_fail.is_empty());
    }

    // ---- Baseline drift ----

    #[test]
    fn baseline_drift_reported() {
        let drifts = vec![BaselineDrift {
            id: "regression_mapper_byte_mmc5".into(),
            field: "expected.golden_hash".into(),
            old: "a1b2c3".into(),
            new: "d4e5f6".into(),
            approved: false,
        }];
        let m = build_matrix(vec![], &Default::default(), &Default::default(), None, drifts);
        assert_eq!(m.baseline_drift.len(), 1);
        assert!(!m.baseline_drift[0].approved);
    }

    // ---- run_id format ----

    #[test]
    fn run_id_is_generated() {
        let m = build_matrix(vec![], &Default::default(), &Default::default(), None, vec![]);
        assert!(!m.run_id.is_empty());
        // Format: YYYYMMDD-HHMMSS-xxxxxx = 8+1+6+1+6 = 22 chars
        assert!(m.run_id.contains('-'));
        assert_eq!(m.run_id.len(), 22);
    }
}
