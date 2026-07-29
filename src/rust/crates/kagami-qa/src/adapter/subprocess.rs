use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::core::{ErrorKind, QaConfig, QaError};
use crate::manifest::schema::TestManifest;
use crate::oracle::regression::check_expected;
use super::trait_def::{SutAdapter, TestResult};

/// Adapter that runs tests as subprocesses (wrapping existing CTest binaries).
pub struct SubprocessAdapter {
    bin_dir: PathBuf,
    default_working_dir: PathBuf,
}

impl SubprocessAdapter {
    pub fn new(bin_dir: impl Into<PathBuf>) -> Self {
        Self {
            bin_dir: bin_dir.into(),
            default_working_dir: std::env::current_dir().unwrap_or_default(),
        }
    }

    pub fn with_working_dir(bin_dir: impl Into<PathBuf>, working_dir: impl Into<PathBuf>) -> Self {
        Self {
            bin_dir: bin_dir.into(),
            default_working_dir: working_dir.into(),
        }
    }
}

impl SutAdapter for SubprocessAdapter {
    fn init(&self, _config: &QaConfig) -> Result<(), QaError> {
        Ok(())
    }

    fn run_test(&self, test: &TestManifest) -> Result<TestResult, QaError> {
        let start = Instant::now();
        let bin_path = if test.input.binary.contains('/') || test.input.binary.contains('\\') {
            // Absolute or relative path — use as-is.
            PathBuf::from(&test.input.binary)
        } else {
            // Bare binary name — try bin_dir first, fall back to PATH.
            // P5 S1-fix: On Windows, tests.json uses bare names like
            // "fceux11_cpu_test" but the actual file is "fceux11_cpu_test.exe".
            // Try both name and name+EXE_EXTENSION before falling back to PATH.
            let name = &test.input.binary;
            let candidate = self.bin_dir.join(name);
            if candidate.exists() {
                candidate
            } else {
                let candidate_exe = self.bin_dir.join(format!("{}.{}", name, std::env::consts::EXE_EXTENSION));
                if candidate_exe.exists() {
                    candidate_exe
                } else {
                    PathBuf::from(name)
                }
            }
        };

        let cwd = test
            .input
            .working_dir
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_working_dir.clone());

        let output = Command::new(&bin_path)
            .args(&test.input.args)
            .current_dir(&cwd)
            .output()
            .map_err(|e| QaError {
                kind: ErrorKind::TestExecFailed,
                message: format!(
                    "Failed to execute test '{}' ({}): {}",
                    test.id, bin_path.display(), e
                ),
            })?;

        let duration = start.elapsed();
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let duration_ms = duration.as_millis() as u64;

        // Pass/fail decision is delegated to oracle::regression::check_expected
        // so that schema-declared `expected.stdout_contains` actually takes effect.
        // Stage-2 §四·五 Phase 0.5 / PR 0.5-1.
        let probe = TestResult {
            test_id: test.id.clone(),
            // placeholder; check_expected only reads exit_code + stdout
            passed: false,
            exit_code,
            stdout: stdout.clone(),
            stderr: stderr.clone(),
            duration_ms,
            migration_note: None,
        };
        let passed = check_expected(&probe, &test.expected);

        Ok(TestResult {
            test_id: test.id.clone(),
            passed,
            exit_code,
            stdout,
            stderr,
            duration_ms,
            migration_note: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::schema::{
        ExpectedResult, FailureSeverity, OracleType, TestInput, TestLayer,
    };

    #[test]
    fn test_binary_not_found() {
        let adapter = SubprocessAdapter::new("/nonexistent");
        let test = TestManifest {
            id: "fake".into(),
            description: "fake".into(),
            oracle_type: OracleType::A,
            layer: TestLayer::Core,
            input: TestInput {
                binary: "nonexistent_binary.exe".into(),
                ..Default::default()
            },
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: None,
            },
            timeout_seconds: 30,
            tags: vec![],
            failure_means: FailureSeverity::Blocking,
            provenance: "test".into(),
        };
        let result = adapter.run_test(&test);
        assert!(result.is_err());
    }

    #[test]
    #[cfg(windows)]
    fn test_existing_binary_echo() {
        // Use absolute path to cmd.exe to avoid bin_dir resolution.
        let adapter = SubprocessAdapter::new(".");
        let cmd = std::path::PathBuf::from(std::env::var("COMSPEC").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".into()));
        let test = TestManifest {
            id: "echo_test".into(),
            description: "simple echo".into(),
            oracle_type: OracleType::A,
            layer: TestLayer::Core,
            input: TestInput {
                binary: cmd.to_string_lossy().to_string(),
                args: vec!["/c".into(), "exit 0".into()],
                ..Default::default()
            },
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: None,
            },
            timeout_seconds: 30,
            tags: vec![],
            failure_means: FailureSeverity::Blocking,
            provenance: "test".into(),
        };
        let result = adapter.run_test(&test).unwrap();
        assert!(result.passed);
        assert_eq!(result.exit_code, 0);
    }

    /// Stage-2 §四·五 PR 0.5-1 acceptance: a test that exits 0 but whose
    /// stdout does NOT contain the expected string MUST be marked FAIL.
    /// Previously `SubprocessAdapter` only checked exit_code, silently
    /// passing such tests — defeating schema-declared `stdout_contains`.
    #[test]
    #[cfg(windows)]
    fn test_stdout_contains_mismatch_marks_fail() {
        let adapter = SubprocessAdapter::new(".");
        let cmd = std::path::PathBuf::from(
            std::env::var("COMSPEC").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".into()),
        );
        let test = TestManifest {
            id: "stdout_mismatch".into(),
            description: "exit 0 but stdout must NOT contain 'NEVER_MATCHES_42'".into(),
            oracle_type: OracleType::A,
            layer: TestLayer::Core,
            input: TestInput {
                // `cmd /c exit 0` produces empty stdout.
                binary: cmd.to_string_lossy().to_string(),
                args: vec!["/c".into(), "exit 0".into()],
                ..Default::default()
            },
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: Some("NEVER_MATCHES_42".into()),
            },
            timeout_seconds: 30,
            tags: vec![],
            failure_means: FailureSeverity::Blocking,
            provenance: "test".into(),
        };
        let result = adapter.run_test(&test).unwrap();
        assert!(!result.passed, "exit_code 0 + missing stdout_contains must FAIL");
        assert_eq!(result.exit_code, 0);
    }
}
