// Baseline persistence and drift detection.
//
// A "baseline" is a snapshot of a previous run's results. By comparing
// the current run against a saved baseline, the runner can populate the
// transition_matrix (FAIL_TO_PASS / PASS_TO_FAIL / …) and detect
// unapproved baseline drift.
//
// Governance (plan §1.2 constraints 6-7):
//   - Baseline updates require the same review as code changes.
//   - AI agents shall NOT modify expected values in the manifest.
//   - Any drift is reported with `approved: false` until reviewed.

use std::collections::BTreeMap;
use std::path::Path;

use super::matrix::{BaselineDrift, PreviousRun};

/// Load a previous-run snapshot from disk.
/// Returns None if the file doesn't exist (first run, no baseline yet).
pub fn load_baseline(path: &Path) -> Option<PreviousRun> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save the current run as a baseline for future comparison.
/// Overwrites any existing baseline at the given path.
pub fn save_baseline(path: &Path, baseline: &PreviousRun) -> Result<(), String> {
    let json = serde_json::to_string_pretty(baseline)
        .map_err(|e| format!("Failed to serialise baseline: {}", e))?;
    std::fs::write(path, &json)
        .map_err(|e| format!("Failed to write baseline '{}': {}", path.display(), e))?;
    Ok(())
}

/// Build a PreviousRun snapshot from the current results.
pub fn snapshot_from_results(
    run_id: &str,
    results: &BTreeMap<String, bool>,
) -> PreviousRun {
    PreviousRun {
        run_id: run_id.to_string(),
        generated_at: chrono_now(),
        results: results.clone(),
    }
}

/// Detect baseline drift by comparing the current run's results against
/// the previous run's results.
///
/// Drift categories produced:
///   - PASS→FAIL (regression): previously passed, now fails → `approved: false`
///   - FAIL→PASS (progress): previously failed, now passes → `approved: false`
///     (needs review to confirm it's a genuine fix, not a fluke)
///   - NEW_TEST: test exists in current but not in previous baseline
///   - REMOVED_TEST: test was in previous baseline but no longer in current
///
/// If no previous baseline is provided, returns an empty vector (first run).
pub fn detect_drift(
    _manifest: &BTreeMap<String, crate::manifest::schema::TestManifest>,
    previous: Option<&PreviousRun>,
    current_results: &BTreeMap<String, bool>,
) -> Vec<BaselineDrift> {
    let prev = match previous {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut drifts = Vec::new();

    // Compare current results against previous results.
    for (test_id, &current_passed) in current_results {
        match prev.results.get(test_id) {
            Some(&prev_passed) => {
                if prev_passed && !current_passed {
                    // Regression: PASS → FAIL
                    drifts.push(BaselineDrift {
                        id: test_id.clone(),
                        field: "passed".into(),
                        old: "PASS".into(),
                        new: "FAIL".into(),
                        approved: false,
                    });
                } else if !prev_passed && current_passed {
                    // Progress: FAIL → PASS (needs review)
                    drifts.push(BaselineDrift {
                        id: test_id.clone(),
                        field: "passed".into(),
                        old: "FAIL".into(),
                        new: "PASS".into(),
                        approved: false,
                    });
                }
            }
            None => {
                // New test not in previous baseline.
                drifts.push(BaselineDrift {
                    id: test_id.clone(),
                    field: "existence".into(),
                    old: "absent".into(),
                    new: "present".into(),
                    approved: false,
                });
            }
        }
    }

    // Detect tests removed from current run.
    for test_id in prev.results.keys() {
        if !current_results.contains_key(test_id) {
            drifts.push(BaselineDrift {
                id: test_id.clone(),
                field: "existence".into(),
                old: "present".into(),
                new: "absent".into(),
                approved: false,
            });
        }
    }

    drifts
}

/// Produce an ISO 8601 timestamp (same algorithm as matrix::generate_run_id).
fn chrono_now() -> String {
    use std::time::SystemTime;

    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let days = (secs / 86400) as i64;
    let time = secs % 86400;
    let hours = time / 3600;
    let mins = (time % 3600) / 60;
    let secs_rem = time % 60;

    let (y, m, d) = days_since_epoch_to_ymd(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, mins, secs_rem
    )
}

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

    #[test]
    fn save_and_load_baseline_round_trip() {
        let mut results = BTreeMap::new();
        results.insert("test_a".into(), true);
        results.insert("test_b".into(), false);
        let snap = PreviousRun {
            run_id: "20260727-test".into(),
            generated_at: "2026-07-27T00:00:00Z".into(),
            results,
        };

        let tmp = tempfile::NamedTempFile::new().unwrap();
        save_baseline(tmp.path(), &snap).unwrap();

        let loaded = load_baseline(tmp.path()).unwrap();
        assert_eq!(loaded.run_id, "20260727-test");
        assert_eq!(loaded.results.len(), 2);
        assert!(loaded.results["test_a"]);
        assert!(!loaded.results["test_b"]);
    }

    #[test]
    fn load_nonexistent_returns_none() {
        assert!(load_baseline(Path::new("/nonexistent/baseline.json")).is_none());
    }

    #[test]
    fn drift_detection_returns_empty_when_no_previous() {
        let current = BTreeMap::new();
        let drifts = detect_drift(&BTreeMap::new(), None, &current);
        assert!(drifts.is_empty());
    }

    #[test]
    fn drift_detection_finds_regression() {
        let mut prev_results = BTreeMap::new();
        prev_results.insert("test_a".into(), true);
        prev_results.insert("test_b".into(), true);
        let prev = PreviousRun {
            run_id: "run-1".into(),
            generated_at: "2026-07-27T00:00:00Z".into(),
            results: prev_results,
        };

        let mut current_results = BTreeMap::new();
        current_results.insert("test_a".into(), true);   // still PASS
        current_results.insert("test_b".into(), false);  // regressed: PASS→FAIL

        let drifts = detect_drift(&BTreeMap::new(), Some(&prev), &current_results);
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].id, "test_b");
        assert_eq!(drifts[0].old, "PASS");
        assert_eq!(drifts[0].new, "FAIL");
        assert!(!drifts[0].approved);
    }

    #[test]
    fn drift_detection_finds_progress() {
        let mut prev_results = BTreeMap::new();
        prev_results.insert("test_a".into(), false);
        let prev = PreviousRun {
            run_id: "run-1".into(),
            generated_at: "2026-07-27T00:00:00Z".into(),
            results: prev_results,
        };

        let mut current_results = BTreeMap::new();
        current_results.insert("test_a".into(), true);  // FAIL→PASS

        let drifts = detect_drift(&BTreeMap::new(), Some(&prev), &current_results);
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].id, "test_a");
        assert_eq!(drifts[0].old, "FAIL");
        assert_eq!(drifts[0].new, "PASS");
    }

    #[test]
    fn drift_detection_finds_new_test() {
        let prev_results = BTreeMap::new();
        let prev = PreviousRun {
            run_id: "run-1".into(),
            generated_at: "2026-07-27T00:00:00Z".into(),
            results: prev_results,
        };

        let mut current_results = BTreeMap::new();
        current_results.insert("test_new".into(), true);

        let drifts = detect_drift(&BTreeMap::new(), Some(&prev), &current_results);
        assert_eq!(drifts.len(), 1);
        assert_eq!(drifts[0].field, "existence");
    }
}
