//! Blargg $6000 batch harness (Track C — Task 1 / C-1).
//!
//! Re-implements the functionality of the original C++ `blargg_runner.cpp`
//! in pure Rust, driving the FCEUX11 core through the [`SutAdapter`]
//! in-process (direct) interface. Preserves byte-for-byte parity with the
//! C++ version for:
//!
//! - single-ROM mode (`--rom <path> --frames N`) → `BLARGG_RESULT: …` line
//!   on stdout, exit 0 on PASS / 1 on FAIL;
//! - batch mode (`--manifest <path.json>`) → one stderr progress line per
//!   ROM plus a JSON object on stdout, exit 0 only if every ROM PASSes.
//!
//! ## Manifest schema (consumes only existing fields)
//!
//! The blargg manifest format produced by `scripts/generate_blargg_manifest.ps1`
//! contains these fields per entry (no new fields are added — Track C obeys
//! the Stage-3 freeze rule on schema):
//!
//! ```jsonc
//! {
//!   "roms": [
//!     {
//!       "name":         "apu_01_len_ctr",
//!       "path":         "fixtures/blargg/apu/apu_01_len_ctr.nes",
//!       "category":     "apu",
//!       "frames":       600,
//!       "probe_addr":   24576,    // 0x6000
//!       "description":  "…",
//!       "reset_after":  60        // OPTIONAL — present for ROMs that gate
//!                                 // their test behind a manual soft-reset
//!                                 // (apu_reset_*, mmc3_irq*). Missing means
//!                                 // "no reset" (legacy behaviour).
//!     }
//!   ]
//! }
//! ```
//!
//! ## Behaviour parity points
//!
//! For each entry, the harness:
//! 1. Loads the ROM via `adapter.load(rom_path)`.
//! 2. If `reset_after >= 0`, runs `reset_after` frames, calls
//!    `adapter.reset()`, then resumes. While `$6000 == 0x81` and ≥ 20
//!    frames have elapsed since the reset, presses RESET again every 6
//!    frames (apu_reset_4017_written needs a second reset).
//! 3. Reads `$probe_addr`, plus `$probe_addr+1..+3` as diagnostics, and
//!    `$probe_addr+4+` as the optional ASCII detail string (blargg error
//!    text). For FAIL cases, re-samples the diag string one frame later.
//! 4. Records `value`, the three diag bytes, the diag string, and
//!    `duration_ms`.
//!
//! Per-ROM exit code semantics match C++ exactly:
//! - exit 0 if every ROM PASSes (and none of the load failures);
//! - exit 1 otherwise.
//!
//! ## Limitations
//!
//! - The harness assumes a live FFI link to fceux11_core. Pure-Rust unit
//!   tests use the same [`SutAdapter`] trait with mocked implementations
//!   (no FFI), exercising only the harness logic.
//! - JSON output is hand-rolled (no serde_json in the hot path) so that the
//!   serialisation order matches the C++ version byte-for-byte and we
//!   don't pull serde_json into the FFI binary.

use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::adapter::trait_def::{InputSpec, SutAdapter};
use crate::core::QaError;

/// Number of frames after which a sticky `$6000 == 0x81` re-triggers a
/// RESET (parity with `blargg_runner.cpp:200-212`).
const RESET_COOLDOWN_FRAMES: u32 = 20;
/// Polling window for re-reset during the post-`reset_after` phase.
const RESET_POLL_CHUNK: u32 = 6;
/// Length of the diagnostic triplet read alongside `$6000`.
const DIAG_BYTE_COUNT: usize = 3;
/// Maximum length of the optional ASCII detail string at `$6004+`.
const DIAG_STRING_MAX: usize = 512;

/// A single entry parsed from `blargg_manifest.json`.
///
/// Mirrors the C++ `ManifestEntry` struct (blargg_runner.cpp:30-42) — only
/// the fields the harness actually consumes are kept here. `description`
/// is preserved for completeness (the C++ version emits it into the
/// stderr progress line).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlarggManifestEntry {
    pub name: String,
    pub path: String,
    pub category: String,
    pub frames: u32,
    pub probe_addr: u32,
    pub description: String,
    /// -1 means "no reset" (legacy behaviour). ≥0 means press RESET at
    /// this frame and continue.
    pub reset_after: i32,
}

/// Parsed view of a `blargg_manifest.json` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlarggManifest {
    pub roms: Vec<BlarggManifestEntry>,
}

/// Outcome of running one ROM against the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlarggResult {
    pub rom_name: String,
    pub probe_addr: u32,
    pub value: u8,
    pub diag: [u8; DIAG_BYTE_COUNT],
    pub passed: bool,
    pub duration_ms: i64,
    /// Optional ASCII detail string (blargg error text) read from
    /// `$probe_addr+4+`. Empty when no detail was written or when the
    /// test passed.
    pub diag_string: String,
    /// The `reset_after` value used for this ROM (matches C++ behaviour:
    /// per-ROM value overrides the global `--reset-after` CLI flag).
    pub reset_after_used: i32,
}

// ---------------------------------------------------------------------------
// Manifest parser — minimal hand-rolled JSON for the blargg manifest schema
// (the C++ version uses its own ad-hoc parser at blargg_runner.cpp:280-380).
// We deliberately keep the same fragility — this schema is generated by our
// build system, not by user input, so the parser only needs to handle the
// exact shape we emit.
// ---------------------------------------------------------------------------

/// Parse a `blargg_manifest.json` document from a string.
///
/// Returns an error string on malformed input. We do not pull in a JSON
/// library — the format is constrained and the parser mirrors the C++
/// version's behaviour (entries missing both `name` and `path` are
/// silently skipped; entries with a `path` but no `name` are kept with
/// an empty `name` to match the C++ behaviour at blargg_runner.cpp:367).
pub fn parse_blargg_manifest(json: &str) -> Result<BlarggManifest, String> {
    let roms_start = json.find("\"roms\"").ok_or("no 'roms' key in manifest")?;
    let array_start = json.find('[').ok_or("no array after 'roms'")?;
    if array_start < roms_start {
        return Err("malformed manifest: '[' before 'roms'".into());
    }

    let mut entries = Vec::new();
    let mut pos = array_start + 1;
    while pos < json.len() {
        // Skip until next object.
        let obj_start = match json[pos..].find('{') {
            None => break,
            Some(off) => pos + off,
        };
        let obj_end_rel = match json[obj_start..].find('}') {
            None => return Err("unterminated object in manifest".into()),
            Some(off) => off,
        };
        let obj_end = obj_start + obj_end_rel;
        let obj = &json[obj_start..=obj_end];
        let entry = parse_entry(obj);
        // C++ behaviour (blargg_runner.cpp:367-371): skip entries with
        // empty name or empty path; otherwise keep the entry as-is.
        if !entry.name.is_empty() && !entry.path.is_empty() {
            entries.push(entry);
        }
        pos = obj_end + 1;
    }
    Ok(BlarggManifest { roms: entries })
}

fn parse_entry(obj: &str) -> BlarggManifestEntry {
    let name = extract_string(obj, "name").unwrap_or_default();
    let path = extract_string(obj, "path").unwrap_or_default();
    let frames = extract_int(obj, "frames").unwrap_or(0).max(0) as u32;
    let probe_addr = extract_int(obj, "probe_addr").unwrap_or(0x6000).max(0) as u32;
    let description = extract_string(obj, "description").unwrap_or_default();
    let category = extract_string(obj, "category").unwrap_or_default();
    // C++: missing "reset_after" → -1 (no reset). The int parser returns
    // 0 for missing keys, which would erroneously trigger the reset branch
    // if we did not check presence explicitly.
    let reset_after = if obj.contains("\"reset_after\"") {
        extract_int(obj, "reset_after").map(|v| v as i32).unwrap_or(-1)
    } else {
        -1
    };
    BlarggManifestEntry {
        name,
        path,
        category,
        frames,
        probe_addr,
        description,
        reset_after,
    }
}

fn extract_string(haystack: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let pos = haystack.find(&needle)?;
    let colon = haystack[pos + needle.len()..].find(':')?;
    let after_colon = pos + needle.len() + colon + 1;
    let val_start_rel = haystack[after_colon..].find('"')?;
    let val_start = after_colon + val_start_rel + 1;
    let val_end_rel = haystack[val_start..].find('"')?;
    Some(haystack[val_start..val_start + val_end_rel].to_string())
}

fn extract_int(haystack: &str, key: &str) -> Option<i64> {
    let needle = format!("\"{}\"", key);
    let pos = haystack.find(&needle)?;
    let colon = haystack[pos + needle.len()..].find(':')?;
    let mut i = pos + needle.len() + colon + 1;
    let bytes = haystack.as_bytes();
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r') {
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_digit() {
        return None;
    }
    let start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    haystack[start..i].parse::<i64>().ok()
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

/// Load a manifest from disk and parse it.
pub fn load_blargg_manifest(path: &Path) -> Result<BlarggManifest, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read manifest '{}': {}", path.display(), e))?;
    parse_blargg_manifest(&text)
}

// ---------------------------------------------------------------------------
// Per-ROM execution. Mirrors `run_one_rom()` in blargg_runner.cpp:160-260.
// ---------------------------------------------------------------------------

/// Run a single ROM against the harness.
///
/// `reset_after_override` matches the C++ semantics: a non-negative value
/// takes precedence over `entry.reset_after` if `use_entry_reset_after`
/// is false. The C++ runner uses per-entry values from the manifest and
/// ignores the CLI override inside the batch loop; the single-ROM CLI
/// honours the `--reset-after` flag instead.
pub fn run_one_rom<A: SutAdapter + ?Sized>(
    adapter: &mut A,
    entry: &BlarggManifestEntry,
    reset_after_override: Option<i32>,
) -> Result<BlarggResult, QaError> {
    // Per-ROM reset_after wins over CLI override inside batch mode.
    let reset_after_used = match reset_after_override {
        Some(v) if entry.reset_after < 0 => v,
        _ => entry.reset_after,
    };

    let start = Instant::now();

    let spec = InputSpec {
        rom_path: Some(entry.path.clone()),
        script_path: None,
        frames: entry.frames,
        probe_addr: entry.probe_addr,
    };
    adapter.load(&spec)?;

    if entry.frames > 0 && reset_after_used >= 0 && (reset_after_used as u32) < entry.frames {
        step_n(adapter, reset_after_used as u32)?;
        adapter.reset()?;
        // Re-reset polling: while $6000 reads 0x81 after a 20-frame
        // cooldown, press RESET again. apu_reset_4017_written needs this
        // second reset.
        let mut done = reset_after_used as u32;
        let mut since_reset: u32 = 0;
        while done < entry.frames {
            let step = std::cmp::min(RESET_POLL_CHUNK, entry.frames - done);
            step_n(adapter, step)?;
            done += step;
            since_reset += step;
            if since_reset >= RESET_COOLDOWN_FRAMES && adapter.read_oracle_probe(entry.probe_addr)? == 0x81 {
                adapter.reset()?;
                since_reset = 0;
            }
        }
    } else {
        step_n(adapter, entry.frames)?;
    }

    let value = adapter.read_oracle_probe(entry.probe_addr)?;
    let mut diag = [0u8; DIAG_BYTE_COUNT];
    for (i, slot) in diag.iter_mut().enumerate() {
        *slot = adapter.read_oracle_probe(entry.probe_addr + 1 + i as u32)?;
    }
    let passed = value == 0x00;

    let diag_string = if !passed {
        let mut s = read_diag_string(adapter, entry.probe_addr + DIAG_BYTE_COUNT as u32 + 1)?;
        if s.is_empty() {
            // Some ROMs overwrite the detail AFTER writing $6000; re-sample
            // one frame later (matches C++ behaviour blargg_runner.cpp:237-243).
            adapter.step()?;
            s = read_diag_string(adapter, entry.probe_addr + DIAG_BYTE_COUNT as u32 + 1)?;
        }
        s
    } else {
        String::new()
    };

    let _ = adapter.reset();
    Ok(BlarggResult {
        rom_name: entry.name.clone(),
        probe_addr: entry.probe_addr,
        value,
        diag,
        passed,
        duration_ms: start.elapsed().as_millis() as i64,
        diag_string,
        reset_after_used,
    })
}

fn step_n<A: SutAdapter + ?Sized>(adapter: &mut A, n: u32) -> Result<(), QaError> {
    for _ in 0..n {
        adapter.step()?;
    }
    Ok(())
}

fn read_diag_string<A: SutAdapter + ?Sized>(
    adapter: &A,
    start_addr: u32,
) -> Result<String, QaError> {
    let mut s = String::with_capacity(64);
    for offset in 0..DIAG_STRING_MAX {
        let b = adapter.read_oracle_probe(start_addr + offset as u32)?;
        if b == 0x00 || b == 0xFF {
            break;
        }
        if b < 0x20 && b != b'\n' && b != b'\r' {
            break;
        }
        s.push(b as char);
    }
    Ok(s)
}

// ---------------------------------------------------------------------------
// Output formatting. Mirrors `print_single_result()` / `print_batch_json()`
// in blargg_runner.cpp:380-430.
// ---------------------------------------------------------------------------

/// Format the single-ROM `BLARGG_RESULT:` line, matching the C++
/// `print_single_result()` byte-for-byte.
pub fn format_single_result(r: &BlarggResult) -> String {
    let mut s = format!(
        "BLARGG_RESULT: rom={} addr=0x{:04X} value=0x{:02X} diag=[0x{:02X},0x{:02X},0x{:02X}] status={} duration_ms={}",
        r.rom_name,
        r.probe_addr,
        r.value,
        r.diag[0], r.diag[1], r.diag[2],
        if r.passed { "PASS" } else { "FAIL" },
        r.duration_ms,
    );
    if !r.diag_string.is_empty() {
        s.push_str(&format!(" diag_string=\"{}\"", r.diag_string));
    }
    s.push('\n');
    s
}

/// Format the batch JSON document, matching the C++ `print_batch_json()`
/// byte-for-byte.
pub fn format_batch_json(results: &[BlarggResult]) -> String {
    let mut buf = String::with_capacity(64 + results.len() * 200);
    buf.push_str("{\n  \"runner\": \"kagami-qa-blargg-runner\",\n");
    buf.push_str("  \"protocol\": \"$6000\",\n");
    buf.push_str("  \"results\": [\n");
    for (i, r) in results.iter().enumerate() {
        let comma = if i + 1 < results.len() { "," } else { "" };
        buf.push_str(&format!(
            "    {{\"rom\":\"{}\",\"addr\":\"0x{:04X}\",\"value\":\"0x{:02X}\",\
             \"diag\":[{},{},{}],\"status\":\"{}\",\"duration_ms\":{},\
             \"reset_after\":{}",
            escape_json(&r.rom_name),
            r.probe_addr,
            r.value,
            r.diag[0], r.diag[1], r.diag[2],
            if r.passed { "PASS" } else { "FAIL" },
            r.duration_ms,
            r.reset_after_used,
        ));
        if !r.diag_string.is_empty() {
            buf.push_str(&format!(",\"diag_string\":\"{}\"", escape_json(&r.diag_string)));
        }
        buf.push_str(&format!("}}{}\n", comma));
    }
    buf.push_str("  ]\n}\n");
    buf
}

fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// High-level entry points (single ROM / batch / CLI parser).
// ---------------------------------------------------------------------------

/// Single-ROM execution. Returns (exit_code, BLARGG_RESULT line).
///
/// Exit codes match `blargg_runner.cpp:main()`:
/// - 0 on PASS
/// - 1 on FAIL or load error
pub fn run_single<A: SutAdapter + ?Sized>(
    adapter: &mut A,
    rom_path: &str,
    frames: u32,
    reset_after: i32,
) -> (i32, String) {
    let entry = BlarggManifestEntry {
        name: rom_basename(rom_path),
        path: rom_path.to_string(),
        category: String::new(),
        frames,
        probe_addr: 0x6000,
        description: String::new(),
        reset_after: -1, // entry-level "no override"; CLI takes precedence
    };
    let result = match run_one_rom(adapter, &entry, Some(reset_after)) {
        Ok(r) => r,
        Err(_) => BlarggResult {
            rom_name: entry.name.clone(),
            probe_addr: 0x6000,
            value: 0xFE, // 0xFE = ROM load failure
            diag: [0, 0, 0],
            passed: false,
            duration_ms: 0,
            diag_string: String::new(),
            reset_after_used: -1,
        },
    };
    let line = format_single_result(&result);
    let exit = if result.passed { 0 } else { 1 };
    (exit, line)
}

fn rom_basename(path: &str) -> String {
    let p = Path::new(path);
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/// Batch execution — reads the manifest, runs every entry, writes JSON
/// to the supplied writer, returns the exit code (0 = all PASS, 1 = any
/// FAIL or load error).
///
/// Per-entry stderr progress lines (matching C++ format) are also written
/// to `err_writer` so the test runner can stream progress.
pub fn run_batch<A: SutAdapter + ?Sized, W: Write, E: Write>(
    adapter: &mut A,
    manifest: &BlarggManifest,
    out_writer: &mut W,
    err_writer: &mut E,
) -> i32 {
    let mut results = Vec::with_capacity(manifest.roms.len());
    let mut fail_count = 0usize;
    for entry in &manifest.roms {
        let progress = if entry.reset_after >= 0 {
            format!("  [{}] {} frames (reset @ {})...", entry.name, entry.frames, entry.reset_after)
        } else {
            format!("  [{}] {} frames...", entry.name, entry.frames)
        };
        let _ = writeln!(err_writer, "{}", progress);

        match run_one_rom(adapter, entry, None) {
            Ok(r) => {
                let status = if r.passed { "PASS" } else { "FAIL" };
                let _ = writeln!(err_writer, " {} (0x{:02X}) {}ms", status, r.value, r.duration_ms);
                if !r.passed {
                    fail_count += 1;
                }
                results.push(r);
            }
            Err(_) => {
                let r = BlarggResult {
                    rom_name: entry.name.clone(),
                    probe_addr: entry.probe_addr,
                    value: 0xFE,
                    diag: [0, 0, 0],
                    passed: false,
                    duration_ms: 0,
                    diag_string: String::new(),
                    reset_after_used: entry.reset_after,
                };
                let _ = writeln!(err_writer, " FAIL (0x{:02X}) 0ms", r.value);
                fail_count += 1;
                results.push(r);
            }
        }
    }

    let total = results.len();
    let json = format_batch_json(&results);
    let _ = out_writer.write_all(json.as_bytes());

    let _ = writeln!(err_writer, "");
    let _ = writeln!(err_writer, "=== Blargg Suite Summary ===");
    let _ = writeln!(err_writer, "Total:  {}", total);
    let _ = writeln!(err_writer, "Passed: {}", total - fail_count);
    let _ = writeln!(err_writer, "Failed: {}", fail_count);

    if fail_count > 0 { 1 } else { 0 }
}

// ---------------------------------------------------------------------------
// CLI argument parser (mirrors blargg_runner.cpp main).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct BlarggCliArgs {
    pub rom_path: Option<String>,
    pub manifest_path: Option<PathBuf>,
    pub frames: u32,
    pub reset_after: i32,
}

impl Default for BlarggCliArgs {
    fn default() -> Self {
        Self {
            rom_path: None,
            manifest_path: None,
            frames: 300,
            reset_after: -1,
        }
    }
}

pub fn parse_cli_args(args: &[String]) -> Result<BlarggCliArgs, String> {
    let mut out = BlarggCliArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--rom" => {
                out.rom_path = Some(
                    iter.next()
                        .ok_or_else(|| "Missing value for --rom".to_string())?
                        .clone(),
                );
            }
            "--frames" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "Missing value for --frames".to_string())?;
                out.frames = v
                    .parse::<u32>()
                    .map_err(|e| format!("Invalid --frames value '{}': {}", v, e))?;
            }
            "--manifest" => {
                out.manifest_path = Some(PathBuf::from(
                    iter.next()
                        .ok_or_else(|| "Missing value for --manifest".to_string())?,
                ));
            }
            "--reset-after" => {
                let v = iter
                    .next()
                    .ok_or_else(|| "Missing value for --reset-after".to_string())?;
                out.reset_after = v
                    .parse::<i32>()
                    .map_err(|e| format!("Invalid --reset-after value '{}': {}", v, e))?;
            }
            other if other.starts_with("--") => {
                return Err(format!("Unknown flag: {}", other));
            }
            // Positional: rom_path or frames (C++ backward compat).
            positional => {
                if out.rom_path.is_none() {
                    out.rom_path = Some(positional.to_string());
                } else if out.frames == 300 {
                    out.frames = positional
                        .parse::<u32>()
                        .map_err(|e| format!("Invalid positional frames value '{}': {}", positional, e))?;
                }
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::trait_def::{InputSpec, SutAdapter, TestResult};
    use crate::core::{ErrorKind, QaConfig, QaError};
    use crate::manifest::schema::{ExpectedResult, FailureSeverity, OracleType, TestInput, TestLayer, TestManifest};

    // ---------------- Mock adapter ----------------------------------------
    /// Adapter that scripts the probe value and step behaviour per ROM,
    /// for byte-level harness testing without an FFI link.
    struct ScriptedAdapter {
        init_count: usize,
        loaded_rom: Option<String>,
        // (addr, return value) overrides; default returns 0x80 ("still running")
        probe_table: HashMap<u32, u8>,
        step_calls: u32,
        step_limit: u32,
        // Count of resets invoked.
        reset_calls: u32,
        // The frames-to-emulate before the per-ROM 0x81 polling kicks in.
        post_reset_probe_value: u8,
    }

    impl ScriptedAdapter {
        fn new() -> Self {
            Self {
                init_count: 0,
                loaded_rom: None,
                probe_table: HashMap::new(),
                step_calls: 0,
                step_limit: u32::MAX,
                reset_calls: 0,
                post_reset_probe_value: 0x00,
            }
        }
    }

    impl SutAdapter for ScriptedAdapter {
        fn init(&self, _config: &QaConfig) -> Result<(), QaError> {
            Ok(())
        }
        fn run_test(&self, _test: &TestManifest) -> Result<TestResult, QaError> {
            unimplemented!("blargg harness uses load/step/read_oracle_probe")
        }
        fn load(&mut self, input: &InputSpec) -> Result<(), QaError> {
            self.init_count += 1;
            self.loaded_rom = input.rom_path.clone();
            Ok(())
        }
        fn step(&mut self) -> Result<(), QaError> {
            if self.step_calls >= self.step_limit {
                return Err(QaError {
                    kind: ErrorKind::TestExecFailed,
                    message: "step limit hit".into(),
                });
            }
            self.step_calls += 1;
            Ok(())
        }
        fn read_oracle_probe(&self, addr: u32) -> Result<u8, QaError> {
            Ok(self.probe_table.get(&addr).copied().unwrap_or(self.post_reset_probe_value))
        }
        fn reset(&mut self) -> Result<(), QaError> {
            self.reset_calls += 1;
            Ok(())
        }
    }

    fn make_entry(name: &str, frames: u32, reset_after: i32) -> BlarggManifestEntry {
        BlarggManifestEntry {
            name: name.into(),
            path: format!("fixtures/{}.nes", name),
            category: "test".into(),
            frames,
            probe_addr: 0x6000,
            description: format!("test {}", name),
            reset_after,
        }
    }

    // ---------------- Manifest parser tests -------------------------------

    #[test]
    fn parses_minimal_manifest() {
        let json = r#"{
            "_comment": "test",
            "roms": [
                {"name":"a","path":"a.nes","frames":300,"probe_addr":24576,"category":"cpu","description":"d"},
                {"name":"b","path":"b.nes","frames":600,"probe_addr":24576,"category":"apu","description":"d","reset_after":60}
            ]
        }"#;
        let m = parse_blargg_manifest(json).unwrap();
        assert_eq!(m.roms.len(), 2);
        assert_eq!(m.roms[0].name, "a");
        assert_eq!(m.roms[0].reset_after, -1);
        assert_eq!(m.roms[1].reset_after, 60);
    }

    #[test]
    fn manifest_without_reset_after_defaults_to_minus_one() {
        let json = r#"{"roms":[{"name":"x","path":"x.nes","frames":10,"probe_addr":24576}]}"#;
        let m = parse_blargg_manifest(json).unwrap();
        assert_eq!(m.roms.len(), 1);
        assert_eq!(m.roms[0].reset_after, -1);
    }

    #[test]
    fn manifest_missing_roms_errors() {
        let r = parse_blargg_manifest(r#"{"_comment":"foo"}"#);
        assert!(r.is_err());
    }

    #[test]
    fn manifest_skips_entry_without_name() {
        let json = r#"{"roms":[{"path":"x.nes","frames":10,"probe_addr":24576}, {"name":"ok","path":"ok.nes","frames":10,"probe_addr":24576}]}"#;
        let m = parse_blargg_manifest(json).unwrap();
        assert_eq!(m.roms.len(), 1);
        assert_eq!(m.roms[0].name, "ok");
    }

    #[test]
    fn manifest_int_extraction_works() {
        let entry = parse_entry(r#"{"name":"x","path":"y","frames":42,"probe_addr":24576,"reset_after":7}"#);
        assert_eq!(entry.frames, 42);
        assert_eq!(entry.probe_addr, 24576);
        assert_eq!(entry.reset_after, 7);
    }

    #[test]
    fn manifest_entry_with_missing_name_is_skipped() {
        let entry = parse_entry(r#"{"path":"x.nes","frames":10,"probe_addr":24576}"#);
        // parse_entry returns the entry unconditionally; the parent parser
        // is responsible for filtering entries with empty name/path.
        assert_eq!(entry.name, "");
        let m = parse_blargg_manifest(
            r#"{"roms":[{"path":"x.nes","frames":10,"probe_addr":24576}, {"name":"ok","path":"ok.nes","frames":10,"probe_addr":24576}]}"#,
        ).unwrap();
        assert_eq!(m.roms.len(), 1);
        assert_eq!(m.roms[0].name, "ok");
    }

    // ---------------- CLI parsing tests -----------------------------------

    #[test]
    fn cli_default() {
        let a = parse_cli_args(&[]).unwrap();
        assert_eq!(a.frames, 300);
        assert_eq!(a.reset_after, -1);
        assert!(a.rom_path.is_none());
        assert!(a.manifest_path.is_none());
    }

    #[test]
    fn cli_parses_all_flags() {
        let a = parse_cli_args(&[
            "--rom".to_string(),
            "x.nes".to_string(),
            "--frames".to_string(),
            "120".to_string(),
            "--reset-after".to_string(),
            "30".to_string(),
        ]).unwrap();
        assert_eq!(a.rom_path.as_deref(), Some("x.nes"));
        assert_eq!(a.frames, 120);
        assert_eq!(a.reset_after, 30);
    }

    #[test]
    fn cli_manifest_flag() {
        let a = parse_cli_args(&[
            "--manifest".to_string(),
            "tests/fixtures/blargg_manifest.json".to_string(),
        ])
        .unwrap();
        assert_eq!(
            a.manifest_path,
            Some(PathBuf::from("tests/fixtures/blargg_manifest.json"))
        );
    }

    #[test]
    fn cli_positional_backward_compat() {
        // blargg_runner accepts positional: <rom> [frames]
        let a = parse_cli_args(&["rom.nes".to_string()]).unwrap();
        assert_eq!(a.rom_path.as_deref(), Some("rom.nes"));
        assert_eq!(a.frames, 300);

        let a = parse_cli_args(&["rom.nes".to_string(), "120".to_string()]).unwrap();
        assert_eq!(a.rom_path.as_deref(), Some("rom.nes"));
        assert_eq!(a.frames, 120);
    }

    #[test]
    fn cli_unknown_flag_errors() {
        assert!(parse_cli_args(&["--bogus".to_string()]).is_err());
    }

    #[test]
    fn cli_missing_value_errors() {
        assert!(parse_cli_args(&["--rom".to_string()]).is_err());
        assert!(parse_cli_args(&["--frames".to_string()]).is_err());
        assert!(parse_cli_args(&["--reset-after".to_string()]).is_err());
    }

    // ---------------- Harness execution tests ----------------------------

    #[test]
    fn pass_run_writes_0x00_and_passes() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let entry = make_entry("pass", 60, -1);
        let r = run_one_rom(&mut a, &entry, None).unwrap();
        assert!(r.passed);
        assert_eq!(r.value, 0x00);
        assert_eq!(r.diag, [0, 0, 0]);
        assert!(r.diag_string.is_empty());
        assert_eq!(r.reset_after_used, -1);
    }

    #[test]
    fn fail_run_writes_0x01_and_fails() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x01;
        a.probe_table.insert(0x6001, 0xAB);
        a.probe_table.insert(0x6002, 0xCD);
        a.probe_table.insert(0x6003, 0xEF);
        let entry = make_entry("fail", 60, -1);
        let r = run_one_rom(&mut a, &entry, None).unwrap();
        assert!(!r.passed);
        assert_eq!(r.value, 0x01);
        assert_eq!(r.diag, [0xAB, 0xCD, 0xEF]);
    }

    #[test]
    fn reset_after_presses_reset_then_completes() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let entry = make_entry("with_reset", 60, 30);
        let r = run_one_rom(&mut a, &entry, None).unwrap();
        assert!(r.passed);
        assert_eq!(r.reset_after_used, 30);
        // reset() is called once for the post-reset_after + once for
        // the final cleanup call.
        assert_eq!(a.reset_calls, 2);
    }

    #[test]
    fn reset_after_disabled_when_negative() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let entry = make_entry("no_reset", 60, -1);
        let _ = run_one_rom(&mut a, &entry, None).unwrap();
        // Only the final cleanup reset() runs.
        assert_eq!(a.reset_calls, 1);
    }

    #[test]
    fn cli_reset_after_override_wins_when_entry_has_none() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let entry = make_entry("no_entry_reset", 60, -1);
        let r = run_one_rom(&mut a, &entry, Some(20)).unwrap();
        assert_eq!(r.reset_after_used, 20);
        assert_eq!(a.reset_calls, 2);
    }

    #[test]
    fn cli_reset_after_override_loses_when_entry_has_one() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let entry = make_entry("with_entry_reset", 60, 30);
        let r = run_one_rom(&mut a, &entry, Some(20)).unwrap();
        // Per-ROM value wins; CLI override ignored.
        assert_eq!(r.reset_after_used, 30);
    }

    #[test]
    fn sticky_0x81_triggers_extra_reset_after_cooldown() {
        // The probe returns 0x81 for >20 frames after the first reset.
        // We expect a second reset() call inside the polling loop.
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x81;
        let entry = make_entry("sticky", 60, 30);
        let _ = run_one_rom(&mut a, &entry, None).unwrap();
        // 1 reset for reset_after + 1 reset for the sticky 0x81 +
        // 1 reset for final cleanup = 3.
        assert!(
            a.reset_calls >= 3,
            "expected ≥3 resets, got {}",
            a.reset_calls
        );
    }

    #[test]
    fn diag_string_is_captured_when_present() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x01;
        // ASCII "OPCODE 0x42" starting at $6004.
        let ascii = b"OPCODE 0x42";
        for (i, b) in ascii.iter().enumerate() {
            a.probe_table.insert(0x6004 + i as u32, *b);
        }
        a.probe_table.insert(0x6004 + ascii.len() as u32, 0x00); // NUL terminator
        let entry = make_entry("with_diag", 60, -1);
        let r = run_one_rom(&mut a, &entry, None).unwrap();
        assert!(!r.passed);
        assert_eq!(r.diag_string, "OPCODE 0x42");
    }

    #[test]
    fn diag_string_truncates_at_nul() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x01;
        for (i, b) in b"OK".iter().enumerate() {
            a.probe_table.insert(0x6004 + i as u32, *b);
        }
        a.probe_table.insert(0x6006, 0xFF); // 0xFF also terminates
        a.probe_table.insert(0x6007, b'X');
        let entry = make_entry("nul", 60, -1);
        let r = run_one_rom(&mut a, &entry, None).unwrap();
        assert_eq!(r.diag_string, "OK");
    }

    // ---------------- Output formatting tests ----------------------------

    #[test]
    fn single_result_line_matches_cxx_format() {
        let r = BlarggResult {
            rom_name: "instr_v5_all".into(),
            probe_addr: 0x6000,
            value: 0x00,
            diag: [0, 0, 0],
            passed: true,
            duration_ms: 4823,
            diag_string: String::new(),
            reset_after_used: -1,
        };
        let line = format_single_result(&r);
        assert_eq!(
            line,
            "BLARGG_RESULT: rom=instr_v5_all addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=4823\n"
        );
    }

    #[test]
    fn single_result_line_with_diag_string() {
        let r = BlarggResult {
            rom_name: "fail".into(),
            probe_addr: 0x6000,
            value: 0x01,
            diag: [0xAB, 0xCD, 0xEF],
            passed: false,
            duration_ms: 100,
            diag_string: "OPCODE 0x42".into(),
            reset_after_used: -1,
        };
        let line = format_single_result(&r);
        assert!(line.starts_with("BLARGG_RESULT: rom=fail "));
        assert!(line.contains("status=FAIL"));
        assert!(line.contains("diag_string=\"OPCODE 0x42\""));
        assert!(line.ends_with('\n'));
    }

    #[test]
    fn batch_json_header_and_footer_match_cxx() {
        let results = vec![BlarggResult {
            rom_name: "a".into(),
            probe_addr: 0x6000,
            value: 0x00,
            diag: [0, 0, 0],
            passed: true,
            duration_ms: 10,
            diag_string: String::new(),
            reset_after_used: -1,
        }];
        let s = format_batch_json(&results);
        assert!(s.starts_with("{\n  \"runner\": \"kagami-qa-blargg-runner\",\n  \"protocol\": \"$6000\",\n  \"results\": [\n"));
        assert!(s.contains("\"rom\":\"a\""));
        assert!(s.contains("\"value\":\"0x00\""));
        assert!(s.contains("\"status\":\"PASS\""));
        assert!(s.contains("\"reset_after\":-1"));
        assert!(s.ends_with("  ]\n}\n"));
    }

    #[test]
    fn batch_json_with_diag_string_escapes_quotes() {
        let results = vec![BlarggResult {
            rom_name: "a".into(),
            probe_addr: 0x6000,
            value: 0x01,
            diag: [0, 0, 0],
            passed: false,
            duration_ms: 0,
            diag_string: "needs \"quote\"".into(),
            reset_after_used: -1,
        }];
        let s = format_batch_json(&results);
        assert!(s.contains("\"diag_string\":\"needs \\\"quote\\\"\""));
    }

    #[test]
    fn batch_json_with_reset_after_field() {
        let results = vec![BlarggResult {
            rom_name: "r".into(),
            probe_addr: 0x6000,
            value: 0x00,
            diag: [0, 0, 0],
            passed: true,
            duration_ms: 0,
            diag_string: String::new(),
            reset_after_used: 60,
        }];
        let s = format_batch_json(&results);
        assert!(s.contains("\"reset_after\":60"));
    }

    #[test]
    fn batch_json_commas_between_results() {
        let results = vec![
            BlarggResult {
                rom_name: "a".into(),
                probe_addr: 0x6000,
                value: 0x00,
                diag: [0, 0, 0],
                passed: true,
                duration_ms: 0,
                diag_string: String::new(),
                reset_after_used: -1,
            },
            BlarggResult {
                rom_name: "b".into(),
                probe_addr: 0x6000,
                value: 0x00,
                diag: [0, 0, 0],
                passed: true,
                duration_ms: 0,
                diag_string: String::new(),
                reset_after_used: -1,
            },
        ];
        let s = format_batch_json(&results);
        // First result has trailing comma; last has none.
        assert!(s.contains("\"rom\":\"a\",\"addr\":\"0x6000\"") && s.contains("},\n"));
        assert!(s.contains("\"rom\":\"b\"") && s.ends_with("  ]\n}\n"));
    }

    // ---------------- Batch execution tests ------------------------------

    #[test]
    fn batch_run_returns_zero_on_all_pass() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let manifest = BlarggManifest {
            roms: vec![make_entry("p1", 10, -1), make_entry("p2", 10, -1)],
        };
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        {
            let mut bufw = BufWriter::new(&mut out);
            let mut errbw = BufWriter::new(&mut err);
            let code = run_batch(&mut a, &manifest, &mut bufw, &mut errbw);
            let _ = bufw.flush();
            let _ = errbw.flush();
            assert_eq!(code, 0);
        }
        let stdout = String::from_utf8(out).unwrap();
        assert!(stdout.contains("\"results\""));
        assert!(stdout.contains("\"p1\""));
        assert!(stdout.contains("\"p2\""));
        let stderr = String::from_utf8(err).unwrap();
        assert!(stderr.contains("=== Blargg Suite Summary ==="));
        assert!(stderr.contains("Passed: 2"));
        assert!(stderr.contains("Failed: 0"));
    }

    #[test]
    fn batch_run_returns_one_on_any_fail() {
        let mut a = ScriptedAdapter::new();
        // First probe returns 0x00, second returns 0x81 (sticky → still
        // running). Either way the second is a FAIL.
        a.post_reset_probe_value = 0x81;
        let manifest = BlarggManifest {
            roms: vec![make_entry("p1", 10, -1), make_entry("p2", 10, -1)],
        };
        let mut out: Vec<u8> = Vec::new();
        let mut err: Vec<u8> = Vec::new();
        {
            let mut bufw = BufWriter::new(&mut out);
            let mut errbw = BufWriter::new(&mut err);
            let code = run_batch(&mut a, &manifest, &mut bufw, &mut errbw);
            let _ = bufw.flush();
            let _ = errbw.flush();
            assert_eq!(code, 1);
        }
        let stderr = String::from_utf8(err).unwrap();
        assert!(stderr.contains("Failed: 2"));
    }

    #[test]
    fn single_run_returns_zero_on_pass() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x00;
        let (code, line) = run_single(&mut a, "x.nes", 10, -1);
        assert_eq!(code, 0);
        assert!(line.starts_with("BLARGG_RESULT: rom=x.nes "));
        assert!(line.contains("status=PASS"));
    }

    #[test]
    fn single_run_returns_one_on_fail() {
        let mut a = ScriptedAdapter::new();
        a.post_reset_probe_value = 0x81;
        let (code, line) = run_single(&mut a, "x.nes", 10, -1);
        assert_eq!(code, 1);
        assert!(line.contains("status=FAIL"));
    }

    #[test]
    fn single_run_emits_0xFE_on_load_failure() {
        struct FailLoadAdapter;
        impl SutAdapter for FailLoadAdapter {
            fn init(&self, _config: &QaConfig) -> Result<(), QaError> {
                Ok(())
            }
            fn run_test(&self, _test: &TestManifest) -> Result<TestResult, QaError> {
                unimplemented!()
            }
            fn load(&mut self, _input: &InputSpec) -> Result<(), QaError> {
                Err(QaError {
                    kind: ErrorKind::TestExecFailed,
                    message: "ROM load failed".into(),
                })
            }
        }
        let mut a = FailLoadAdapter;
        let (code, line) = run_single(&mut a, "missing.nes", 10, -1);
        assert_eq!(code, 1);
        assert!(line.contains("value=0xFE"));
        assert!(line.contains("status=FAIL"));
    }
}
