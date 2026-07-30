use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

        let mut child = Command::new(&bin_path)
            .args(&test.input.args)
            .current_dir(&cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| QaError {
                kind: ErrorKind::TestExecFailed,
                message: format!(
                    "Failed to spawn test '{}' ({}): {}",
                    test.id, bin_path.display(), e
                ),
            })?;

        // Drain stdout/stderr on side threads so the main thread can poll
        // try_wait without losing buffered output (subprocess IPC buffers
        // fill up to ~64 KiB on Windows; beyond that the child blocks).
        let mut stdout_handle = child.stdout.take();
        let mut stderr_handle = child.stderr.take();
        let stdout_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(ref mut s) = stdout_handle {
                let _ = s.read_to_end(&mut buf);
            }
            buf
        });
        let stderr_thread = thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(ref mut s) = stderr_handle {
                let _ = s.read_to_end(&mut buf);
            }
            buf
        });

        // Stage-2 §四·五 PR 0.5-2: timeout_seconds enforcement.
        // Polls try_wait at 50 ms granularity; on timeout, kills the child
        // and reports a synth timeout exit code (-2) with a migration_note.
        // Previously Command::output() blocked forever (no timeout), which
        // masked deadlocked tests as runner hangs.
        let timeout = Duration::from_secs(test.timeout_seconds);
        let exit_status_opt: Option<std::process::ExitStatus> = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) => {
                    if start.elapsed() > timeout {
                        // Kill + reap; on Windows this returns Ok(Some(_))
                        // when the kill lands, but we deliberately don't
                        // use the synthesized status — we want the timeout
                        // marker to survive into the report.
                        let _ = child.kill();
                        let _ = child.wait();
                        break None;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                Err(e) => {
                    return Err(QaError {
                        kind: ErrorKind::TestExecFailed,
                        message: format!(
                            "Failed waiting on test '{}': {}",
                            test.id, e
                        ),
                    });
                }
            }
        };

        let stdout_bytes = stdout_thread.join().unwrap_or_default();
        let stderr_bytes = stderr_thread.join().unwrap_or_default();
        let duration = start.elapsed();
        let duration_ms = duration.as_millis() as u64;

        let (exit_code, migration_note) = match exit_status_opt {
            Some(status) => (status.code().unwrap_or(-1), None),
            None => (
                -2i32,
                Some(format!(
                    "timeout: test exceeded {}s (Stage-2 §四·五 PR 0.5-2)",
                    test.timeout_seconds
                )),
            ),
        };

        let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
        let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

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
            migration_note: migration_note.clone(),
        };
        let passed = check_expected(&probe, &test.expected);

        Ok(TestResult {
            test_id: test.id.clone(),
            passed,
            exit_code,
            stdout,
            stderr,
            duration_ms,
            migration_note,
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

    /// Stage-2 §四·五 PR 0.5-2 acceptance: a test whose subprocess sleeps
    /// longer than `timeout_seconds` MUST be killed and reported as FAIL
    /// with `migration_note = Some(timeout: …)`, instead of blocking the
    /// runner indefinitely. Polling granularity is 50 ms so a 2-second
    /// sleep with a 1-second timeout must reliably trip the kill.
    #[test]
    #[cfg(windows)]
    fn test_timeout_kills_hanging_subprocess() {
        let adapter = SubprocessAdapter::new(".");
        // Use PowerShell Start-Sleep — guaranteed to actually sleep the
        // requested time on Windows. `timeout` via cmd is unreliable
        // when stdout is redirected via `> NUL` under heavy CI load.
        let ps = std::path::PathBuf::from(
            std::env::var("WINDIR").map(|w| format!(r"{}\System32\WindowsPowerShell\v1.0\powershell.exe", w))
                .unwrap_or_else(|_| r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe".into()),
        );
        // Skip the test cleanly if powershell is unavailable (e.g. CI image).
        if !ps.exists() {
            eprintln!("skipping test_timeout_kills_hanging_subprocess — powershell not found");
            return;
        }
        let test = TestManifest {
            id: "hang_test".into(),
            description: "sleep 30s — must trip timeout".into(),
            oracle_type: OracleType::A,
            layer: TestLayer::Core,
            input: TestInput {
                binary: ps.to_string_lossy().to_string(),
                args: vec![
                    "-NoProfile".into(),
                    "-Command".into(),
                    "Start-Sleep -Seconds 30".into(),
                ],
                ..Default::default()
            },
            expected: ExpectedResult {
                exit_code: 0,
                stdout_contains: None,
            },
            timeout_seconds: 1,
            tags: vec![],
            failure_means: FailureSeverity::Blocking,
            provenance: "test".into(),
        };
        let start = std::time::Instant::now();
        let result = adapter.run_test(&test).unwrap();
        let elapsed = start.elapsed();
        // The kill should fire within ~1 s + 50 ms polling jitter.
        assert!(
            elapsed < Duration::from_secs(3),
            "run_test should not block past timeout (took {:?})",
            elapsed
        );
        assert!(!result.passed, "hanging test must FAIL");
        assert_eq!(result.exit_code, -2, "timeout exits with synth -2");
        assert!(
            result
                .migration_note
                .as_deref()
                .unwrap_or("")
                .contains("timeout"),
            "migration_note must mention timeout, got: {:?}",
            result.migration_note
        );
        assert!(
            result.migration_note.as_deref().unwrap_or("").contains("1s"),
            "migration_note must quote the timeout value, got: {:?}",
            result.migration_note
        );
    }
}
