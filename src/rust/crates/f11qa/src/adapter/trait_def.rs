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

/// Input specification for the direct (in-process) adapter.
/// Mirrors the ROM/script fields from TestInput but is adapter-agnostic.
#[derive(Debug, Clone)]
pub struct InputSpec {
    /// Path to a ROM file (for Oracle B / hardware tests).
    pub rom_path: Option<String>,
    /// Path to a Lua script (for software-side dynamic tests).
    pub script_path: Option<String>,
    /// Number of frames to run (for Oracle B tests).
    pub frames: u32,
    /// Probe address for $6000 protocol.
    pub probe_addr: u32,
    /// v1.17 H-1: insert a soft reset after this many frames (sibling to
    /// `frames` and `probe_addr`). -1 = no mid-run reset (default behaviour,
    /// i.e. step all `frames` then probe). 0 = reset immediately after load.
    /// > 0 = step N frames, reset, step remaining frames. See TestInput
    /// for the manifest-side meaning.
    pub reset_after: i64,
}

// Default frame budget when the manifest entry does not state one.
// Deliberately private: exposing it makes cbindgen emit a stray comment into
// the generated fceux11_rust.h.
const DEFAULT_FRAMES: u32 = 300;

impl InputSpec {
    pub fn from_manifest(test: &TestManifest) -> Self {
        Self {
            rom_path: test.input.rom.clone(),
            script_path: test.input.script_path.clone(),
            frames: frames_from_args(&test.input.args).unwrap_or(DEFAULT_FRAMES),
            probe_addr: test.input.probe_addr.unwrap_or(0x6000),
            reset_after: test.input.reset_after,
        }
    }
}

/// Extract the `--frames N` budget from a manifest entry's argv.
///
/// Stage-2 S-2: this used to be hardcoded to 300 with the comment "can be
/// overridden in manifest" — but nothing ever overrode it, so direct mode ran
/// every Oracle B ROM for exactly 300 frames while subprocess mode honored the
/// per-test budget. Any ROM needing a longer run reported `$6000 == 0x80`
/// ("still running") and was scored as a failure. That is a direct/subprocess
/// parity break, not a ROM defect.
///
/// Accepts both `--frames 3000` and `--frames=3000`.
fn frames_from_args(args: &[String]) -> Option<u32> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if let Some(rest) = arg.strip_prefix("--frames=") {
            return rest.parse().ok();
        }
        if arg == "--frames" {
            return it.next()?.parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod input_spec_tests {
    use super::frames_from_args;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_separate_value_form() {
        assert_eq!(
            frames_from_args(&v(&["--rom", "a.nes", "--frames", "3000"])),
            Some(3000)
        );
    }

    #[test]
    fn reads_equals_form() {
        assert_eq!(frames_from_args(&v(&["--frames=500"])), Some(500));
    }

    #[test]
    fn absent_yields_none() {
        assert_eq!(frames_from_args(&v(&["--rom", "a.nes"])), None);
    }

    #[test]
    fn trailing_flag_without_value_yields_none() {
        assert_eq!(frames_from_args(&v(&["--frames"])), None);
    }

    #[test]
    fn non_numeric_value_yields_none() {
        assert_eq!(frames_from_args(&v(&["--frames", "many"])), None);
    }
}

/// System-Under-Test adapter trait.
///
/// Abstracts how the QA runner invokes tests against the FCEUX11 emulator.
///
/// # Implementations
///
/// | Adapter              | Mode        | Use case                            |
/// |----------------------|-------------|-------------------------------------|
/// | `SubprocessAdapter`  | subprocess  | P1–P3: wraps existing CTest binaries |
/// | `Fceux11DirectAdapter` | in-process | P5: frame-by-frame via C ABI FFI     |
///
/// The in-process adapter provides `step()` granularity needed for
/// runppu 重批 — the runner can interleave emulation steps with
/// oracle probes and state snapshots.
pub trait SutAdapter {
    /// Initialize the adapter.
    fn init(&self, config: &QaConfig) -> Result<(), QaError>;

    /// Run a single test (subprocess mode — for existing CTest binaries).
    fn run_test(&self, test: &TestManifest) -> Result<TestResult, QaError>;

    // ------------------------------------------------------------------
    // In-process (direct) interface — used by Fceux11DirectAdapter.
    // Default implementations return Unsupported so SubprocessAdapter
    // doesn't need to implement them.
    // ------------------------------------------------------------------

    /// Load the input (ROM or script) into the emulator.
    fn load(&mut self, _input: &InputSpec) -> Result<(), QaError> {
        Err(QaError::unsupported("load (in-process) not available for this adapter"))
    }

    /// Advance the emulator by one frame.
    fn step(&mut self) -> Result<(), QaError> {
        Err(QaError::unsupported("step (in-process) not available for this adapter"))
    }

    /// Read an oracle probe value from CPU address space.
    fn read_oracle_probe(&self, _addr: u32) -> Result<u8, QaError> {
        Err(QaError::unsupported("read_oracle_probe (in-process) not available for this adapter"))
    }

    /// Take a snapshot of current emulator state (for golden-master comparison).
    fn snapshot(&self) -> Result<Vec<u8>, QaError> {
        Err(QaError::unsupported("snapshot (in-process) not available for this adapter"))
    }

    /// Reset the emulator to post-power-on state.
    fn reset(&mut self) -> Result<(), QaError> {
        Err(QaError::unsupported("reset (in-process) not available for this adapter"))
    }
}
