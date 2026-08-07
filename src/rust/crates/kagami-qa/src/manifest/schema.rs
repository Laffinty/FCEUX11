use serde::{Deserialize, Serialize};

/// A single test case entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestManifest {
    pub id: String,
    pub description: String,
    pub oracle_type: OracleType,
    pub layer: TestLayer,
    #[serde(default)]
    pub input: TestInput,
    pub expected: ExpectedResult,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub failure_means: FailureSeverity,
    pub provenance: String,
}

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OracleType {
    /// Regression-equivalence (exit-code based, unit tests).
    #[serde(rename = "A")]
    A,
    /// Hardware-consistency (output/ROM based).
    #[serde(rename = "B")]
    B,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestLayer {
    #[serde(rename = "core")]
    Core,
    #[serde(rename = "boards")]
    Boards,
    #[serde(rename = "driver")]
    Driver,
    #[serde(rename = "lua")]
    Lua,
    #[serde(rename = "script")]
    Script,
    #[serde(rename = "benchmark")]
    Benchmark,
}

/// How to invoke the test.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestInput {
    /// Binary name (resolved relative to bin_dir).
    pub binary: String,
    /// Command-line arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional working directory override.
    #[serde(default)]
    pub working_dir: Option<String>,
    /// P2: ROM path for Oracle B / ROM-based tests.
    #[serde(default)]
    pub rom: Option<String>,
    /// P2: Probe address for Oracle B $6000 protocol (default 0x6000).
    #[serde(default)]
    pub probe_addr: Option<u32>,
    /// P3: Lua script path for Lua script channel (run via fceux11_lua_runner).
    #[serde(default)]
    pub script_path: Option<String>,
    /// v1.17 H-1: if > 0, the runner steps this many frames, then issues a
    /// soft reset, then steps the remaining frames. -1 = no mid-run reset
    /// (default). 0 = reset immediately after load. This is a sibling
    /// parameter to `probe_addr` / `frames` — same scope (Oracle B driving),
    /// same shape (i64 default), same justification (blargg `$6000` protocol
    /// has ROMs that need a one-shot reset partway through to converge).
    #[serde(default)]
    pub reset_after: i64,
}

/// Expected outcome for the test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedResult {
    pub exit_code: i32,
    #[serde(default)]
    pub stdout_contains: Option<String>,
}

/// Severity classification for failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureSeverity {
    /// Must pass; blocks release / CI gate.
    #[serde(rename = "blocking")]
    Blocking,
    /// Advisory only; continue-on-error.
    #[serde(rename = "advisory")]
    Advisory,
}
