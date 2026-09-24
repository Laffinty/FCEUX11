use std::path::PathBuf;

/// Runner configuration.
#[derive(Debug, Clone)]
pub struct QaConfig {
    /// Path to the tests.json manifest file.
    pub manifest_path: PathBuf,
    /// Directory containing test binaries (CTest build output).
    pub bin_dir: PathBuf,
    /// Working directory for test execution.
    pub working_dir: PathBuf,
    /// Output path for the migration matrix JSON report.
    pub output_path: PathBuf,
    /// Default timeout per test in seconds.
    pub timeout_seconds: u64,
}
