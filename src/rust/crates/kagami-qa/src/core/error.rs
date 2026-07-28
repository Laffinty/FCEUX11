/// Unified error type for KagamiQA operations.
#[derive(Debug, Clone)]
pub struct QaError {
    pub kind: ErrorKind,
    pub message: String,
}

impl std::fmt::Display for QaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.kind, self.message)
    }
}

impl std::error::Error for QaError {}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    /// Manifest file not found or parse error.
    ManifestError,
    /// Test binary execution failed (not found, crash, etc.).
    TestExecFailed,
    /// Test exceeded timeout.
    Timeout,
    /// Oracle comparison mismatch.
    OracleMismatch,
    /// I/O error.
    IoError,
    /// Operation not supported by this adapter.
    Unsupported,
}

impl QaError {
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Unsupported,
            message: msg.into(),
        }
    }
}

impl From<std::io::Error> for QaError {
    fn from(e: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::IoError,
            message: e.to_string(),
        }
    }
}
