// P1: Oracle A (regression-equivalence) is simple exit-code comparison.
// The subprocess adapter checks `exit_code == expected.exit_code`.
// P2+ will add frame hash / WAV diff / mapper state comparison here.

use crate::adapter::trait_def::TestResult;
use crate::manifest::schema::ExpectedResult;

/// Check whether a test result matches the expected outcome.
/// P1: exit-code only.
pub fn check_expected(result: &TestResult, expected: &ExpectedResult) -> bool {
    if result.exit_code != expected.exit_code {
        return false;
    }
    if let Some(needle) = &expected.stdout_contains {
        if !result.stdout.contains(needle.as_str()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(exit_code: i32, stdout: &str) -> TestResult {
        TestResult {
            test_id: "t".into(),
            passed: true,
            exit_code,
            stdout: stdout.into(),
            stderr: String::new(),
            duration_ms: 0,
            migration_note: None,
        }
    }

    #[test]
    fn exit_code_match() {
        let r = make_result(0, "");
        let e = ExpectedResult {
            exit_code: 0,
            stdout_contains: None,
        };
        assert!(check_expected(&r, &e));
    }

    #[test]
    fn exit_code_mismatch() {
        let r = make_result(1, "");
        let e = ExpectedResult {
            exit_code: 0,
            stdout_contains: None,
        };
        assert!(!check_expected(&r, &e));
    }

    #[test]
    fn stdout_contains_match() {
        let r = make_result(0, "PASS: all tests");
        let e = ExpectedResult {
            exit_code: 0,
            stdout_contains: Some("PASS".into()),
        };
        assert!(check_expected(&r, &e));
    }

    #[test]
    fn stdout_contains_mismatch() {
        let r = make_result(0, "FAIL: test");
        let e = ExpectedResult {
            exit_code: 0,
            stdout_contains: Some("PASS".into()),
        };
        assert!(!check_expected(&r, &e));
    }
}
