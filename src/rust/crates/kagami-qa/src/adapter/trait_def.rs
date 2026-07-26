use crate::core::{QaConfig, QaError};
use crate::manifest::schema::TestManifest;

/// Result of executing a single test.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub test_id: String,
    pub passed: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    /// Optional migration note (e.g. setup error description).
    pub migration_note: Option<String>,
}

/// System-Under-Test adapter trait.
///
/// Abstracts how the QA runner invokes tests against the FCEUX11 emulator.
/// P1: Subprocess adapter (wraps CTest binaries).
/// P2+: In-process adapter (calls core via FFI).
pub trait SutAdapter {
    /// Initialize the adapter.
    fn init(&self, config: &QaConfig) -> Result<(), QaError>;

    /// Run a single test and return its result.
    fn run_test(&self, test: &TestManifest) -> Result<TestResult, QaError>;
}
