use serde::Serialize;

use crate::adapter::trait_def::TestResult;

/// Migration matrix — the core JSON report output.
/// SWE-bench isomorphic: FAIL_TO_PASS / PASS_TO_PASS / PASS_TO_FAIL.
#[derive(Debug, Serialize)]
pub struct MigrationMatrix {
    pub report_version: u32,
    pub runner: String,
    pub generated_at: String,
    pub summary: MatrixSummary,
    pub results: Vec<MatrixEntry>,
}

#[derive(Debug, Serialize)]
pub struct MatrixSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
pub struct MatrixEntry {
    pub test_id: String,
    pub passed: bool,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub oracle_type: String,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration_note: Option<String>,
}

/// Build a migration matrix from raw test results.
/// `oracle_type` and `layer` are taken from the manifest entry.
pub fn build_matrix(
    results: Vec<TestResult>,
    oracle_types: &std::collections::BTreeMap<String, String>,
    layers: &std::collections::BTreeMap<String, String>,
) -> MigrationMatrix {
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;

    let entries: Vec<MatrixEntry> = results
        .into_iter()
        .map(|r| {
            let oracle_type = oracle_types
                .get(&r.test_id)
                .cloned()
                .unwrap_or_else(|| "?".into());
            let layer = layers.get(&r.test_id).cloned().unwrap_or_else(|| "?".into());
            MatrixEntry {
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
        runner: "kagami-qa-p1".into(),
        generated_at: chrono_now(),
        summary: MatrixSummary {
            total,
            passed,
            failed,
        },
        results: entries,
    }
}

/// Produce an ISO 8601 timestamp. Avoids pulling in `chrono` by formatting
/// the system time directly.
fn chrono_now() -> String {
    use std::time::SystemTime;

    let dur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Naive ISO 8601 without timezone: YYYY-MM-DDTHH:MM:SS
    let days = secs / 86400;
    let time = secs % 86400;
    let hours = time / 3600;
    let mins = (time % 3600) / 60;
    let secs_rem = time % 60;

    // Days since Unix epoch → approximate calendar date.
    // This is a simplification; for exact dates, use the `time` crate.
    // We compute year/month/day from days-since-epoch.
    let (y, m, d) = days_since_epoch_to_ymd(days as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, mins, secs_rem
    )
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
/// Civil date algorithm (Howard Hinnant).
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
    fn empty_results() {
        let m = build_matrix(vec![], &Default::default(), &Default::default());
        assert_eq!(m.summary.total, 0);
        assert_eq!(m.summary.passed, 0);
        assert_eq!(m.results.len(), 0);
    }

    #[test]
    fn mixed_results() {
        let results = vec![
            TestResult {
                test_id: "pass_test".into(),
                passed: true,
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: 100,
                migration_note: None,
            },
            TestResult {
                test_id: "fail_test".into(),
                passed: false,
                exit_code: 1,
                stdout: String::new(),
                stderr: "error".into(),
                duration_ms: 200,
                migration_note: Some("regression".into()),
            },
        ];
        let mut oracle_types = std::collections::BTreeMap::new();
        oracle_types.insert("pass_test".into(), "A".into());
        oracle_types.insert("fail_test".into(), "B".into());
        let mut layers = std::collections::BTreeMap::new();
        layers.insert("pass_test".into(), "core".into());
        layers.insert("fail_test".into(), "core".into());

        let m = build_matrix(results, &oracle_types, &layers);
        assert_eq!(m.summary.total, 2);
        assert_eq!(m.summary.passed, 1);
        assert_eq!(m.summary.failed, 1);
        assert_eq!(m.results[0].oracle_type, "A");
        assert_eq!(m.results[1].oracle_type, "B");
        assert!(m.results[1].migration_note.is_some());
    }
}
