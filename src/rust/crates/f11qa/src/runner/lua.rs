//! Headless Lua script runner (Task 1 migration — replaces
//! `tests/lua_runner.cpp`).
//!
//! Runs a single `.lua` test script inside the fceux11-lua engine,
//! headless (null driver via the kagami_bridge C ABI). The Lua script
//! calls `emu.frameadvance()` which yields the coroutine; the harness
//! resumes it each frame via `fceux11_lua_frame_boundary()`.
//!
//! Lifecycle (mirrors the C++ runner):
//!   kagami_bridge_init → (optional ROM load + 1 warm-up frame)
//!   → fceux11_lua_init → fceux11_lua_load_script(path)
//!   → frame loop (emulate + frame_boundary while lua_running)
//!   → fceux11_lua_shutdown
//!
//! ## Output capture
//!
//! The C++ runner redirected C `stdout`/`stderr` to temp files via
//! `freopen` and scanned the captured text for assertion-failure
//! markers. The Lua test scripts report assertion results with the
//! *bare* `print(...)` (mlua's default, which writes to the C
//! `stdout` FILE*), so `freopen` captures them. `emu.print(...)` goes
//! through Rust's `println!` (OS handle) and is *not* captured — same
//! as the C++ runner, and not a failure signal for the test scripts.
//!
//! This Rust port uses the same C-level `freopen` mechanism via the
//! MSVC CRT (`__acrt_iob_func` + `freopen` FFI). The `LUA_RESULT` line
//! is printed with Rust's `println!` (OS handle), which also fixes the
//! C++ runner's bug where `LUA_RESULT` was invisible when stdio was
//! redirected (the `freopen("CONOUT$", ...)` restore silently failed).
//!
//! Usage (identical to the C++ runner):
//!   kagami_qa_lua_runner <script_path> [--rom <path>] [--frames N]
//!
//! Exit code: 0 = PASS, 1 = FAIL.

use std::ffi::{CString, c_char, c_int, c_uint, c_void};
use std::path::Path;
use std::time::Instant;

// ---------------------------------------------------------------------------
// FFI — fceux11-lua engine (exported from fceux11_rust.lib) + kagami_bridge
// + the MSVC CRT functions needed for output capture.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn fceux11_lua_init() -> c_int;
    fn fceux11_lua_shutdown() -> c_int;
    fn fceux11_lua_load_script(path: *const c_char, arg: *const c_char) -> c_int;
    fn fceux11_lua_frame_boundary();
    fn fceux11_lua_stop();
    fn fceux11_lua_running() -> c_int;

    fn kagami_bridge_init() -> c_int;
    fn kagami_bridge_load_rom(path: *const c_char) -> c_int;
    fn kagami_bridge_emulate_frame() -> c_int;
}

// NOTE: the MSVC CRT capture functions (`__acrt_iob_func`, `freopen`,
// `fclose`) are deliberately declared *inside* function bodies (see
// `run_lua_script`). cbindgen only scans module-level `extern "C"`
// blocks when generating the merged `fceux11_rust.h`; a module-level
// declaration of these CRT symbols would collide with the C runtime
// headers (`FILE *__acrt_iob_func(...)` vs the void* projection here)
// and break every C++ TU that includes the merged header.

/// Default frame budget (mirrors the C++ runner's `max_frames = 300`).
pub const DEFAULT_MAX_FRAMES: i32 = 300;

/// Outcome of running one Lua script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaOutcome {
    pub script_name: String,
    pub passed: bool,
    pub exit_code: i32,
    pub details: String,
    pub duration_ms: i64,
}

// ---------------------------------------------------------------------------
// CLI parsing
// ---------------------------------------------------------------------------

/// Parsed CLI arguments (mirrors the C++ `main` argument loop).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LuaCliArgs {
    pub script_path: Option<String>,
    pub rom_path: Option<String>,
    pub max_frames: i32,
}

/// Parse the argument list (excluding argv[0]). Mirrors the C++ loop:
/// `--rom <path>`, `--frames N`, and up to two positionals (script,
/// then optional rom).
pub fn parse_cli_args(args: &[String]) -> Result<LuaCliArgs, String> {
    let mut cli = LuaCliArgs {
        max_frames: DEFAULT_MAX_FRAMES,
        ..Default::default()
    };
    let mut positionals: Vec<&String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if a == "--rom" && i + 1 < args.len() {
            cli.rom_path = Some(args[i + 1].clone());
            i += 2;
        } else if a == "--frames" && i + 1 < args.len() {
            cli.max_frames = args[i + 1]
                .parse()
                .map_err(|_| format!("invalid --frames value: {}", args[i + 1]))?;
            i += 2;
        } else if a.starts_with('-') {
            i += 1; // unknown flag: ignore (mirrors C++).
        } else {
            positionals.push(a);
            i += 1;
        }
    }
    if let Some(p) = positionals.first() {
        cli.script_path = Some((*p).clone());
    }
    if cli.rom_path.is_none() {
        if let Some(p) = positionals.get(1) {
            cli.rom_path = Some((*p).clone());
        }
    }
    Ok(cli)
}

// ---------------------------------------------------------------------------
// Output capture via the C runtime
// ---------------------------------------------------------------------------

/// Redirect C `stdout` (stream index 1) or `stderr` (2) to `path` using
/// `freopen`. The CRT symbols are declared locally (see the module note
/// on why they must not appear at module level).
unsafe fn capture_stream(stream_index: c_uint, path: &Path) -> Result<(), String> {
    unsafe extern "C" {
        fn __acrt_iob_func(index: c_uint) -> *mut c_void;
        fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    }
    let c_path = CString::new(path.to_string_lossy().as_bytes())
        .map_err(|_| "invalid capture path".to_string())?;
    let mode = c"w";
    let stream = unsafe { __acrt_iob_func(stream_index) };
    let rc = unsafe { freopen(c_path.as_ptr(), mode.as_ptr(), stream) };
    if rc.is_null() {
        return Err(format!(
            "freopen('{}') failed",
            path.to_string_lossy()
        ));
    }
    Ok(())
}

/// Restore a captured C stream to the console (`CONOUT$` on Windows).
/// Best-effort: mirrors the C++ runner's restore attempt.
unsafe fn restore_stream(stream_index: c_uint) {
    unsafe extern "C" {
        fn __acrt_iob_func(index: c_uint) -> *mut c_void;
        fn freopen(path: *const c_char, mode: *const c_char, stream: *mut c_void) -> *mut c_void;
    }
    let conout = c"CONOUT$";
    let mode = c"w";
    let stream = unsafe { __acrt_iob_func(stream_index) };
    let _ = unsafe { freopen(conout.as_ptr(), mode.as_ptr(), stream) };
}

/// Close a C stream handle (used to flush the capture files).
unsafe fn close_stream(stream_index: c_uint) {
    unsafe extern "C" {
        fn __acrt_iob_func(index: c_uint) -> *mut c_void;
        fn fclose(stream: *mut c_void) -> c_int;
    }
    let _ = unsafe { fclose(__acrt_iob_func(stream_index)) };
}

/// C-string literal helper (Rust 1.77+ `c"..."`).
const _: () = ();

// ---------------------------------------------------------------------------
// Script execution
// ---------------------------------------------------------------------------

/// Extract the script's base file name from a path (mirrors the C++
/// `strrchr` logic for '/' and '\\').
fn script_base_name(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    base.to_string()
}

/// Scan captured output for failure markers (mirrors the C++ P5 M2-fix
/// classification). Returns (has_error, first_failing_line, fail_count).
fn scan_captured(stdout_str: &str, stderr_str: &str) -> (bool, String, usize) {
    let mut has_error = false;

    // stdout: script-printed failure markers.
    if stdout_str.contains("FAIL:")
        || stdout_str.contains("FAIL ")
        || stdout_str.contains("ERROR:")
    {
        has_error = true;
    }
    // stderr: Lua runtime errors.
    if stderr_str.contains("runtime error")
        || stderr_str.contains("stack traceback")
        || stderr_str.contains("assertion failed")
        || stderr_str.contains("ERROR:")
        || stderr_str.contains("PANIC:")
        || stderr_str.contains("syntax error")
    {
        has_error = true;
    }

    // Count FAIL lines for the detail string.
    let mut fail_count = 0usize;
    let mut first_fail: String = String::new();
    for line in stdout_str.lines().chain(stderr_str.lines()) {
        if line.contains("FAIL:") || line.contains("FAIL ") {
            fail_count += 1;
            if first_fail.is_empty() && line.len() > 6 {
                first_fail = line.to_string();
            }
        }
    }
    (has_error, first_fail, fail_count)
}

/// Run one Lua script and classify PASS/FAIL. `workdir` is the current
/// working directory (mirrors `WORKING_DIRECTORY = tests/`).
pub fn run_lua_script(
    script_path: &str,
    rom_path: Option<&str>,
    max_frames: i32,
) -> LuaOutcome {
    let script_name = script_base_name(script_path);
    let t0 = Instant::now();

    // Create temp files for stdout/stderr capture.
    let tmp_dir = std::env::temp_dir();
    let unique = format!("kagami_lua_{}_{}", std::process::id(), script_name);
    let tmp_stdout = tmp_dir.join(format!("{}.out", unique));
    let tmp_stderr = tmp_dir.join(format!("{}.err", unique));

    // ---- Capture C stdout/stderr + run the script ----
    let (early_exit, has_error, first_fail, fail_count) = unsafe {
        if capture_stream(1, &tmp_stdout).and_then(|_| capture_stream(2, &tmp_stderr)).is_err() {
            return LuaOutcome {
                script_name,
                passed: false,
                exit_code: 1,
                details: "output capture setup failed".to_string(),
                duration_ms: t0.elapsed().as_millis() as i64,
            };
        }
        let exit_candidate = run_script_inner(script_path, rom_path, max_frames);
        // Restore the C streams (best-effort; Rust println! is
        // unaffected by the C freopen either way).
        restore_stream(1);
        restore_stream(2);
        close_stream(1);
        close_stream(2);
        let stdout_str = std::fs::read_to_string(&tmp_stdout).unwrap_or_default();
        let stderr_str = std::fs::read_to_string(&tmp_stderr).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_stdout);
        let _ = std::fs::remove_file(&tmp_stderr);
        let (he, ff, fc) = scan_captured(&stdout_str, &stderr_str);
        (exit_candidate, he, ff, fc)
    };

    let duration_ms = t0.elapsed().as_millis() as i64;

    // ---- Classify ----
    if early_exit < 0 {
        return LuaOutcome {
            script_name,
            passed: false,
            exit_code: 1,
            details: format!("engine error (code {})", early_exit),
            duration_ms,
        };
    }
    if has_error {
        let details = if !first_fail.is_empty() {
            truncate(&first_fail, 200)
        } else {
            "Lua error/assert detected".to_string()
        };
        return LuaOutcome {
            script_name,
            passed: false,
            exit_code: 1,
            details,
            duration_ms,
        };
    }
    if fail_count > 0 {
        return LuaOutcome {
            script_name,
            passed: false,
            exit_code: 1,
            details: format!("{} test failure(s) detected", fail_count),
            duration_ms,
        };
    }
    LuaOutcome {
        script_name,
        passed: true,
        exit_code: 0,
        details: "script completed (all assertions passed)".to_string(),
        duration_ms,
    }
}

/// Helper: truncate a string to `max` chars with a "..." suffix.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(3)).collect();
        t.push_str("...");
        t
    }
}

/// Drive the engine + Lua lifecycle. Returns a negative exit-code
/// candidate on early failure (core init / ROM load / lua init / script
/// load), or 0 if the script ran to completion.
fn run_script_inner(
    script_path: &str,
    rom_path: Option<&str>,
    max_frames: i32,
) -> i32 {
    // SAFETY notes: all FFI calls below are safe per the C-ABI contract
    // (valid pointers, engine lifecycle order).
    unsafe {
        // --- Initialise engine ---
        if kagami_bridge_init() != 0 {
            return -1;
        }

        // --- Load ROM (optional) ---
        if let Some(rom) = rom_path {
            if !rom.is_empty() {
                let c_rom = match CString::new(rom) {
                    Ok(c) => c,
                    Err(_) => return -2,
                };
                if kagami_bridge_load_rom(c_rom.as_ptr()) != 0 {
                    return -2;
                }
                // One warm-up frame so mapper banking settles.
                if kagami_bridge_emulate_frame() != 0 {
                    return -2;
                }
            }
        }

        // --- Initialise Lua engine ---
        if fceux11_lua_init() != 0 {
            return -3;
        }

        // --- Load Lua script ---
        let c_script = match CString::new(script_path) {
            Ok(c) => c,
            Err(_) => {
                fceux11_lua_shutdown();
                return -4;
            }
        };
        if fceux11_lua_load_script(c_script.as_ptr(), std::ptr::null()) != 0 {
            fceux11_lua_shutdown();
            return -4;
        }

        // --- Frame loop ---
        let mut frame_count = 0i32;
        while fceux11_lua_running() != 0 {
            if max_frames > 0 && frame_count >= max_frames {
                eprintln!("lua_runner: timeout after {} frames", max_frames);
                fceux11_lua_stop();
                break;
            }
            if kagami_bridge_emulate_frame() != 0 {
                break;
            }
            fceux11_lua_frame_boundary();
            frame_count += 1;
        }

        // --- Shutdown Lua ---
        fceux11_lua_shutdown();
        0
    }
}

// ---------------------------------------------------------------------------
// Result printing
// ---------------------------------------------------------------------------

/// Format the LUA_RESULT line (mirrors the C++ `print_result` format).
pub fn format_result(outcome: &LuaOutcome) -> String {
    format!(
        "LUA_RESULT: script={} status={} duration_ms={} details={}\n",
        outcome.script_name,
        if outcome.passed { "PASS" } else { "FAIL" },
        outcome.duration_ms,
        outcome.details
    )
}

/// Exit code from an outcome (0 = PASS, 1 = FAIL).
pub fn exit_code(outcome: &LuaOutcome) -> i32 {
    if outcome.passed { 0 } else { 1 }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_script_only() {
        let args = vec!["tests/lua_scripts/test_bit.lua".to_string()];
        let cli = parse_cli_args(&args).unwrap();
        assert_eq!(cli.script_path.as_deref(), Some("tests/lua_scripts/test_bit.lua"));
        assert_eq!(cli.rom_path, None);
        assert_eq!(cli.max_frames, DEFAULT_MAX_FRAMES);
    }

    #[test]
    fn parse_cli_rom_and_frames() {
        let args = vec![
            "tests/lua_scripts/test_emu.lua".to_string(),
            "--frames".to_string(),
            "120".to_string(),
            "--rom".to_string(),
            "fixtures/nestest.nes".to_string(),
        ];
        let cli = parse_cli_args(&args).unwrap();
        assert_eq!(cli.script_path.as_deref(), Some("tests/lua_scripts/test_emu.lua"));
        assert_eq!(cli.rom_path.as_deref(), Some("fixtures/nestest.nes"));
        assert_eq!(cli.max_frames, 120);
    }

    #[test]
    fn parse_cli_positional_rom() {
        let args = vec![
            "script.lua".to_string(),
            "rom.nes".to_string(),
        ];
        let cli = parse_cli_args(&args).unwrap();
        assert_eq!(cli.script_path.as_deref(), Some("script.lua"));
        assert_eq!(cli.rom_path.as_deref(), Some("rom.nes"));
    }

    #[test]
    fn scan_captured_detects_fail_prefix() {
        let (he, ff, fc) = scan_captured("FAIL: my check", "");
        assert!(he);
        assert!(ff.contains("my check"));
        assert_eq!(fc, 1);
    }

    #[test]
    fn scan_captured_detects_fail_space() {
        let (he, _, fc) = scan_captured("FAIL something", "");
        assert!(he);
        assert_eq!(fc, 1);
    }

    #[test]
    fn scan_captured_detects_runtime_error() {
        let (he, _, _) = scan_captured("", "runtime error: attempt to call nil");
        assert!(he);
    }

    #[test]
    fn scan_captured_ignores_benign_summary() {
        // "bit library: 29 passed, 0 failed" must NOT be a failure.
        let (he, _, fc) = scan_captured("bit library: 29 passed, 0 failed", "");
        assert!(!he);
        assert_eq!(fc, 0);
    }

    #[test]
    fn scan_captured_counts_multiple_fails() {
        let (_, _, fc) = scan_captured("FAIL: a\nFAIL: b\nFAIL: c\n", "");
        assert_eq!(fc, 3);
    }

    #[test]
    fn script_base_name_extracts() {
        assert_eq!(script_base_name("tests/lua_scripts/test_bit.lua"), "test_bit.lua");
        assert_eq!(script_base_name("C:\\scripts\\x.lua"), "x.lua");
        assert_eq!(script_base_name("plain.lua"), "plain.lua");
    }

    #[test]
    fn truncate_shortens_long_lines() {
        let long = "x".repeat(300);
        let t = truncate(&long, 200);
        assert!(t.len() <= 200);
        assert!(t.ends_with("..."));
    }
}
