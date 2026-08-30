pub mod core;
pub mod manifest;
pub mod runner;
pub mod oracle;
pub mod report;
use std::path::Path;

pub mod adapter;
pub mod cli;

// =========================================================================
// Track C Task 1 / C-1: C-callable entry point for the blargg batch
// harness — re-implements `tests/blargg_runner.cpp` in Rust.
//
// Single-ROM mode (`--rom <path> [--frames N] [--reset-after N]`) prints a
// `BLARGG_RESULT:` line and returns 0 (PASS) or 1 (FAIL). Batch mode
// (`--manifest <blargg_manifest.json>`) drives every ROM in the manifest,
// prints progress to stderr and a JSON document to stdout, returning 0
// iff every ROM PASSed.
//
// Linked into `fceux11_rust.lib` so that the (single, future) C++
// trampoline can call it. The trampoline is intentionally tiny and lives
// outside `kagami-qa`.
// =========================================================================
#[cfg(feature = "direct-adapter")]
pub mod blargg_entry {
    use std::ffi::CStr;
    use std::io::Write;
    use std::os::raw::c_char;
    use std::path::PathBuf;

    use crate::adapter::direct::Fceux11DirectAdapter;
    use crate::adapter::trait_def::SutAdapter;
    use crate::runner::blargg::{
        load_blargg_manifest, parse_cli_args, run_batch, run_single,
    };

    /// C-ABI entry point replacing `tests/blargg_runner.cpp:main()`.
    ///
    /// Symbol is intentionally not `#[unsafe(no_mangle)]` here — the
    /// owning crate `fceux11-rust` (see its wrapper) declares the exported
    /// symbol. Keeping `no_mangle` in two places would create a
    /// duplicate-symbol collision when both libs are linked together.
    pub unsafe extern "C" fn kagami_qa_blargg_main(
        argc: i32,
        argv: *const *const c_char,
    ) -> i32 {
        let mut args = Vec::new();
        for i in 0..argc {
            let ptr = unsafe { *argv.offset(i as isize) };
            if ptr.is_null() {
                break;
            }
            let arg = match unsafe { CStr::from_ptr(ptr) }.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => String::from("<invalid-utf8>"),
            };
            args.push(arg);
        }

        let cli = match parse_cli_args(&args[1..]) {
            Ok(c) => c,
            Err(msg) => {
                let _ = writeln!(
                    std::io::stderr(),
                    "blargg_runner: {}\n\
                     Usage: fceux11_blargg_runner --rom <path> [--frames N] [--reset-after N]\n\
                            fceux11_blargg_runner --manifest <path.json>\n\
                            fceux11_blargg_runner <rom_path> [frames]",
                    msg
                );
                return 1;
            }
        };

        if let Some(manifest_path) = cli.manifest_path.clone() {
            return run_batch_path(&cli, manifest_path);
        }

        let rom_path = match cli.rom_path {
            Some(p) => p,
            None => {
                let _ = writeln!(
                    std::io::stderr(),
                    "Usage: fceux11_blargg_runner --rom <path> [--frames N] [--reset-after N]\n\
                           fceux11_blargg_runner --manifest <path.json>"
                );
                return 1;
            }
        };

        let mut adapter = Fceux11DirectAdapter::new().with_newppu();
        let (code, line) = run_single(&mut adapter, &rom_path, cli.frames, cli.reset_after);
        let _ = std::io::stdout().write_all(line.as_bytes());
        code
    }

    fn run_batch_path(cli: &crate::runner::blargg::BlarggCliArgs, path: PathBuf) -> i32 {
        let manifest = match load_blargg_manifest(&path) {
            Ok(m) => m,
            Err(e) => {
                let _ = writeln!(std::io::stderr(), "blargg_runner: {}", e);
                return 1;
            }
        };
        if manifest.roms.is_empty() {
            let _ = writeln!(std::io::stderr(), "blargg_runner: no valid entries in manifest");
            return 1;
        }

        let mut adapter = Fceux11DirectAdapter::new().with_newppu();
        let mut out = std::io::stdout();
        let mut err = std::io::stderr();
        run_batch(&mut adapter, &manifest, &mut out, &mut err)
        // NB: cli.reset_after is intentionally ignored in batch mode —
        // per-entry reset_after in the manifest wins (matches C++).
        .max(0)
    }
}

// =========================================================================
// Track C Task 1 / C-2: C-callable entry point for the rom_regression
// harness — re-implements `tests/rom_regression_test.cpp` in Rust.
//
// Loads `fixtures/golden_hashes.json`, drives the 13-ROM table (12
// mapper ROMs + nestest), 60 frames per ROM, CRC32 of the visible
// 256x240 region of XBuf per frame, compares against the golden
// hashes, prints the regression summary, and returns 0 iff every
// hash matches.
// =========================================================================
#[cfg(feature = "direct-adapter")]
pub mod rom_regression_entry {
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use crate::adapter::direct::Fceux11DirectAdapter;
    use crate::adapter::trait_def::SutAdapter;
    use crate::runner::rom_regression::{
        format_summary, load_golden_hashes, regression_exit_code, run_regression, FrameSource,
    };
    use crate::write_golden_hashes_json;

    /// C-ABI entry point replacing `tests/rom_regression_test.cpp:main()`.
    ///
    /// Phase 5.3: `--generate` writes the freshly collected per-frame CRCs
    /// to `fixtures/golden_hashes.json` (plan 5.3 item 4 - golden
    /// regeneration under the Do-NOT #3 three-piece process; the semantic
    /// basis is the plan v2.2 hardware-truth revision, NOT "make the test
    /// shut up"). Exit code stays 0 on a generate run.
    pub unsafe extern "C" fn kagami_qa_rom_regression_main(
        argc: i32,
        argv: *const *const std::os::raw::c_char,
    ) -> i32 {
        // WORKING_DIRECTORY is `tests/` (mirrors CMake CTest entry).
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let golden_path = workdir.join("fixtures/golden_hashes.json");
        let generate = argc > 1
            && (0..argc).any(|i| unsafe {
                let p = *argv.offset(i as isize);
                !p.is_null() && std::ffi::CStr::from_ptr(p).to_string_lossy() == "--generate"
            });
        let golden = match load_golden_hashes(&golden_path) {
            Ok(g) => Some(g),
            Err(e) => {
                if !generate {
                    let _ = writeln!(
                        std::io::stdout(),
                        "Golden hashes file not found: {}\n\
                         \n\
                         FAIL: Could not read golden hashes. Run with --generate to create baseline.\n\
                         RESULT: FAILED",
                        golden_path.display(),
                    );
                    let _ = writeln!(std::io::stdout(), "{}", e);
                    return 1;
                }
                None
            }
        };
        let missing = golden.is_none();

        let mut adapter = Fceux11DirectAdapter::new();
        let empty_golden = crate::runner::rom_regression::GoldenHashes {
            names: Vec::new(),
            frames: Vec::new(),
        };
        let outcome = run_regression(
            &mut adapter,
            golden.as_ref().unwrap_or(&empty_golden),
            &workdir,
        );
        // Per-mismatch detail lines (mirror the C++ harness, which prints
        // up to 5 mismatch lines with rom/frame/expected/actual). The
        // summary alone hides which ROM/frame diverged.
        for m in &outcome.mismatches {
            let _ = writeln!(
                std::io::stdout(),
                "Mismatch for {}: frame {} expected 0x{:08x} actual 0x{:08x}",
                m.rom_name, m.frame, m.expected, m.actual
            );
        }
        let summary = format_summary(&outcome);
        let _ = write!(std::io::stdout(), "{}", summary);

        if generate {
            write_golden_hashes_json(&golden_path, &outcome.collected);
            let _ = writeln!(
                std::io::stdout(),
                "GENERATE: wrote {} frame-CRC entries to {}{}",
                outcome.collected.len(),
                golden_path.display(),
                if missing { " (file was missing)" } else { "" },
            );
            return 0;
        }
        regression_exit_code(&outcome)
    }

    // Suppress unused-import warnings when only the harness entry is
    // referenced (the FrameSource trait is invoked via
    // Fceux11DirectAdapter at runtime through the impl below).
    const _: fn(&Fceux11DirectAdapter, &mut [u8]) -> Result<(), crate::core::QaError> =
        |a, b| <Fceux11DirectAdapter as FrameSource>::extract_frame(a, b);
}

// =========================================================================
// Track C Task 1 / C-3: C-callable entry point for the
// savestate_regression harness — re-implements
// `tests/savestate_regression_test.cpp` in Rust.
//
// Loads `fixtures/golden_savestate_hashes.json`, drives the 12-ROM
// table (vrc7 omitted because its savestate contains a non-
// deterministic heap pointer), 60 frames per ROM, MD5 of the
// serialised savestate per ROM, compares against the golden hashes,
// prints the regression summary, and returns 0 iff every hash matches.
// =========================================================================
#[cfg(feature = "direct-adapter")]
pub mod savestate_regression_entry {
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use crate::adapter::direct::Fceux11DirectAdapter;
    use crate::adapter::trait_def::SutAdapter;
    use crate::runner::savestate_regression::{
        format_summary, load_golden_savestate_hashes, regression_exit_code, run_regression,
        StateSnapshot,
    };
    use crate::write_golden_savestate_hashes_json;

    /// C-ABI entry point replacing `tests/savestate_regression_test.cpp:main()`.
    ///
    /// Phase 5.3: `--generate` writes the freshly collected per-ROM MD5s
    /// to `fixtures/golden_savestate_hashes.json` (plan 5.3 item 4 -
    /// golden regeneration under the Do-NOT #3 three-piece process; the
    /// semantic basis is the plan v2.2 hardware-truth revision). Exit
    /// code stays 0 on a generate run.
    pub unsafe extern "C" fn kagami_qa_savestate_regression_main(
        argc: i32,
        argv: *const *const std::os::raw::c_char,
    ) -> i32 {
        // WORKING_DIRECTORY is `tests/` (mirrors CMake CTest entry).
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let golden_path = workdir.join("fixtures/golden_savestate_hashes.json");
        let generate = argc > 1
            && (0..argc).any(|i| unsafe {
                let p = *argv.offset(i as isize);
                !p.is_null() && std::ffi::CStr::from_ptr(p).to_string_lossy() == "--generate"
            });
        let golden = match load_golden_savestate_hashes(&golden_path) {
            Ok(g) => Some(g),
            Err(e) => {
                if !generate {
                    let _ = writeln!(
                        std::io::stdout(),
                        "Golden hashes file not found: {}\n\n\
                         No golden hashes found. Run with --generate to create {}\n\
                         RESULT: FAILED",
                        golden_path.display(),
                        golden_path.display(),
                    );
                    let _ = writeln!(std::io::stdout(), "{}", e);
                    return 1;
                }
                None
            }
        };

        let mut adapter = Fceux11DirectAdapter::new();
        let empty_golden = crate::runner::savestate_regression::GoldenSavestateHashes {
            names: Vec::new(),
            hashes: Vec::new(),
        };
        let outcome = run_regression(
            &mut adapter,
            golden.as_ref().unwrap_or(&empty_golden),
            &workdir,
        );
        // TEMP parity debug: per-ROM hashes for cross-check vs C++.
        for (name, actual) in &outcome.collected {
            eprintln!("  [rust-hash] {name}: {actual}");
        }
        // Per-mismatch detail lines (mirror the C++ harness): name +
        // expected/actual MD5.
        for m in &outcome.mismatches {
            let _ = writeln!(
                std::io::stdout(),
                "Mismatch for {}: expected {} actual {}",
                m.rom_name, m.expected, m.actual
            );
        }
        let summary = format_summary(&outcome);
        let _ = write!(std::io::stdout(), "{}", summary);

        if generate {
            write_golden_savestate_hashes_json(&golden_path, &outcome.collected);
            let _ = writeln!(
                std::io::stdout(),
                "GENERATE: wrote {} savestate-MD5 entries to {}",
                outcome.collected.len(),
                golden_path.display(),
            );
            return 0;
        }
        regression_exit_code(&outcome)
    }

    // Suppress unused-import warnings when only the harness entry is
    // referenced (the StateSnapshot trait is invoked via
    // Fceux11DirectAdapter at runtime through the impl below).
    const _: fn(&Fceux11DirectAdapter) -> Result<Vec<u8>, crate::core::QaError> =
        |a| <Fceux11DirectAdapter as StateSnapshot>::snapshot_state(a);

    // Reference Path to keep the std::path import active even when
    // the harness entry is the only consumer of this module.
    const _: fn(&Path) -> () = |_| ();
}

// =========================================================================
// Task 1 (mapper migration): C-callable entry point for the mapper
// byte-diff harness — re-implements `tests/core/mapper_byte_diff_test.cpp`.
//
// Loads `fixtures/golden_mapper/<name>.bin` for each of the 175 mapper
// test cases, drives the 175-case table (fresh engine per ROM, N frames
// each, capture `Cart::save_mapper_state()`), compares body bytes against
// the golden, prints the summary, and returns 0 iff no case failed
// (SKIP — missing golden / empty body on both sides — is allowed, exactly
// like the C++ harness).
// =========================================================================
#[cfg(feature = "direct-adapter")]
pub mod mapper_byte_diff_entry {
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use crate::adapter::direct::Fceux11DirectAdapter;
    use crate::runner::mapper_byte_diff::{
        format_summary, regression_exit_code, run_regression_filtered, MapperStateSource,
    };

    /// C-ABI entry point replacing `tests/core/mapper_byte_diff_test.cpp:main()`.
    ///
    /// Phase 5.3: supports `--only <name>` (repeatable) to run a single
    /// mapper case — the `mapper_<name>_byte_diff` ctest entries gate one
    /// mapper each. With no `--only`, the full 175-case table runs.
    pub unsafe extern "C" fn kagami_qa_mapper_byte_diff_main(
        argc: i32,
        argv: *const *const std::os::raw::c_char,
    ) -> i32 {
        // WORKING_DIRECTORY is `tests/` (mirrors CMake CTest entry).
        let workdir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let golden_dir = workdir.join("fixtures/golden_mapper");

        let mut only: Vec<String> = Vec::new();
        let mut args: Vec<String> = Vec::new();
        for i in 0..argc {
            let ptr = unsafe { *argv.offset(i as isize) };
            if ptr.is_null() {
                break;
            }
            // SAFETY: ptr points to a null-terminated C string (the
            // C-ABI contract for argv).
            let arg = match unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => String::from("<invalid-utf8>"),
            };
            args.push(arg);
        }
        let mut it = args.iter().skip(1);
        while let Some(a) = it.next() {
            if a == "--only" {
                match it.next() {
                    Some(name) => only.push(name.clone()),
                    None => {
                        let _ = write!(std::io::stdout(), "--only requires a case name
");
                        return 2;
                    }
                }
            }
            // Unknown args are ignored for backward compatibility (the
            // pre-5.3 entry ignored argv entirely).
        }

        let mut adapter = Fceux11DirectAdapter::new();
        let outcome = run_regression_filtered(&mut adapter, &workdir, &golden_dir, &only);
        let summary = format_summary(&outcome);
        let _ = write!(std::io::stdout(), "{}", summary);
        regression_exit_code(&outcome)
    }

    // Suppress unused-import warnings when only the harness entry is
    // referenced (the MapperStateSource trait is invoked via
    // Fceux11DirectAdapter at runtime through the impl below).
    const _: fn(&Fceux11DirectAdapter) -> Result<Vec<u8>, crate::core::QaError> =
        |a| <Fceux11DirectAdapter as MapperStateSource>::snapshot_mapper_state(a);

    // Reference Path to keep the std::path import active even when
    // the harness entry is the only consumer of this module.
    const _: fn(&Path) -> () = |_| ();
}

// =========================================================================
// Task 1 (lua): C-callable entry point for the headless Lua script
// runner — re-implements `tests/lua_runner.cpp`.
//
// Parses `<script_path> [--rom <path>] [--frames N]`, runs the script
// inside the fceux11-lua engine with C-level stdout/stderr capture,
// scans the captured output for assertion-failure markers, prints the
// `LUA_RESULT:` line, and returns 0 (PASS) or 1 (FAIL).
// =========================================================================
#[cfg(feature = "direct-adapter")]
pub mod lua_entry {
    use std::io::Write;

    use crate::runner::lua::{exit_code, format_result, parse_cli_args, run_lua_script};

    /// C-ABI entry point replacing `tests/lua_runner.cpp:main()`.
    pub unsafe extern "C" fn kagami_qa_lua_main(
        argc: i32,
        argv: *const *const std::os::raw::c_char,
    ) -> i32 {
        let mut args: Vec<String> = Vec::new();
        for i in 0..argc {
            let ptr = unsafe { *argv.offset(i as isize) };
            if ptr.is_null() {
                break;
            }
            // SAFETY: ptr points to a null-terminated C string (the
            // C-ABI contract for argv).
            let arg = match unsafe { std::ffi::CStr::from_ptr(ptr) }.to_str() {
                Ok(s) => s.to_string(),
                Err(_) => String::from("<invalid-utf8>"),
            };
            args.push(arg);
        }

        let cli = match parse_cli_args(&args[1..]) {
            Ok(c) => c,
            Err(e) => {
                let _ = writeln!(
                    std::io::stdout(),
                    "LUA_RESULT: script=<invalid> status=FAIL details={}",
                    e
                );
                return 1;
            }
        };

        let script_path = match cli.script_path {
            Some(s) => s,
            None => {
                let _ = writeln!(
                    std::io::stdout(),
                    "Usage: kagami_qa_lua_runner <script_path> [--rom <path>] [--frames N]"
                );
                return 1;
            }
        };

        let outcome = run_lua_script(&script_path, cli.rom_path.as_deref(), cli.max_frames);
        let _ = write!(std::io::stdout(), "{}", format_result(&outcome));
        exit_code(&outcome)
    }
}

// =========================================================================
// P5: C-callable entry point for kagami_qa_direct_runner (CMake target).
//
// Called from tests/kagami_direct_main.cpp when the direct runner is built
// via CMake with the direct-adapter Cargo feature enabled.
//
// Task 4 (FCEUX11-1.17_计划.md §5.3 step 2): the per-test driving loop no
// longer lives here — it was deduplicated into
// `runner::direct::run_direct_rom_tests`, shared with the CLI `--direct`
// mode. This module parses CLI args, loads the manifest, delegates to the
// shared core, and computes the exit code from blocking failures.
// =========================================================================

#[cfg(feature = "direct-adapter")]
pub mod direct_entry {
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::path::PathBuf;

    use crate::adapter::direct::Fceux11DirectAdapter;
    use crate::adapter::trait_def::SutAdapter;
    use crate::manifest::parser::load_manifest;
    use crate::runner::direct::run_direct_rom_tests;

    /// Main entry point called from C++ (kagami_direct_main.cpp).
    ///
    /// Stage-2 §七 (C-1): this function deliberately does NOT carry
    /// `#[unsafe(no_mangle)]`. The exported C-ABI symbol `kagami_qa_direct_main`
    /// is owned by the root crate `fceux11-rust` (see its wrapper), which calls
    /// this function via the rlib. Keeping `no_mangle` here would create a
    /// duplicate-symbol collision when both libs are linked together.
    pub unsafe extern "C" fn kagami_qa_direct_main(
        argc: i32,
        argv: *const *const c_char,
    ) -> i32 {
        // Convert C args to Rust strings.
        // S2-fix: wrap raw-pointer deref + CStr::from_ptr in explicit unsafe
        // blocks (Rust 2024 unsafe_op_in_unsafe_fn lint requires explicit
        // unsafe blocks even inside unsafe fn bodies).
        let mut args = Vec::new();
        for i in 0..argc {
            let ptr = unsafe { *argv.offset(i as isize) };
            if ptr.is_null() {
                break;
            }
            match unsafe { CStr::from_ptr(ptr) }.to_str() {
                Ok(s) => args.push(s.to_string()),
                Err(_) => args.push(String::from("<invalid-utf8>")),
            }
        }

        // Parse key flags (same as the CLI).
        let mut manifest_path = PathBuf::from("tests/tests.json");
        let mut output_path = PathBuf::from("kagamiqa_direct_matrix.json");
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--manifest" => {
                    i += 1;
                    if i < args.len() {
                        manifest_path = PathBuf::from(&args[i]);
                    }
                }
                "--output" => {
                    i += 1;
                    if i < args.len() {
                        output_path = PathBuf::from(&args[i]);
                    }
                }
                _ => {}
            }
            i += 1;
        }

        eprintln!("KagamiQA P5 Direct Runner (in-process via C ABI bridge)");
        eprintln!("Loading manifest: {}", manifest_path.display());

        let manifest = match load_manifest(&manifest_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error loading manifest: {:?}", e);
                return 1;
            }
        };
        eprintln!("Loaded {} test entries.", manifest.len());

        // Task 4: drive the emulator through the shared execution core —
        // the same code path the CLI `--direct` mode uses.
        let mut direct_adapter = Fceux11DirectAdapter::new().with_newppu();
        let results = run_direct_rom_tests(&mut direct_adapter, &manifest);

        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        let blocking_failed = results
            .iter()
            .filter(|r| !r.passed)
            .filter(|r| {
                manifest
                    .get(&r.test_id)
                    .map(|t| {
                        matches!(
                            t.failure_means,
                            crate::manifest::schema::FailureSeverity::Blocking
                        )
                    })
                    .unwrap_or(false)
            })
            .count();

        for r in &results {
            eprintln!(
                "  [direct] {}: {}",
                r.test_id,
                if r.passed { "PASS" } else { "FAIL" }
            );
        }
        eprintln!(
            "Direct run complete: {} total, {} PASS, {} FAIL ({} blocking)",
            total,
            passed,
            failed,
            blocking_failed
        );

        // Write minimal JSON report.
        let report = serde_json::json!({
            "runner": "kagami-qa-direct-runner",
            "mode": "in-process",
            "summary": {
                "total": total,
                "passed": passed,
                "failed": failed
            }
        });
        if let Ok(json) = serde_json::to_string_pretty(&report) {
            if let Err(e) = std::fs::write(&output_path, &json) {
                eprintln!("Error writing output: {}", e);
            } else {
                eprintln!("Report written to: {}", output_path.display());
            }
        }

        // Stage-2: respect failure_means. Advisory failures (D-2
        // missing-ROM, E-1 pending accuracy rebalance) are reported in
        // the matrix but do not propagate to the process exit code —
        // the same convention as kagami-qa-runner (subprocess mode).
        if blocking_failed > 0 { 1 } else { 0 }
    }
}

// =========================================================================
// Phase 5.3 - golden writers for the `--generate` mode of the
// rom/savestate regression harnesses (plan 5.3 item 4). Both emit the
// exact document shape the original C++ writers produced, which the
// hand-rolled tolerant parsers in this crate read back.
// =========================================================================

/// Write `golden_hashes.json`: `{ "name": { "frames": [u32; 60] } }` per ROM.
fn write_golden_hashes_json(path: &Path, collected: &std::collections::BTreeMap<String, Vec<u32>>) {
    let mut s = String::from("{\n");
    for (i, (name, frames)) in collected.iter().enumerate() {
        s.push_str("  \"");
        s.push_str(name);
        s.push_str("\": {\n    \"frames\": [");
        for (j, f) in frames.iter().enumerate() {
            if j > 0 {
                s.push_str(", ");
            }
            s.push_str(&f.to_string());
        }
        s.push_str("]\n  }");
        if i + 1 < collected.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("}\n");
    if let Err(e) = std::fs::write(path, s) {
        eprintln!("GENERATE: failed to write {}: {e}", path.display());
    }
}

/// Write `golden_savestate_hashes.json`: `{ "name": { "hash": "md5" } }` per ROM.
fn write_golden_savestate_hashes_json(
    path: &Path,
    collected: &std::collections::BTreeMap<String, String>,
) {
    let mut s = String::from("{\n");
    for (i, (name, hash)) in collected.iter().enumerate() {
        s.push_str("  \"");
        s.push_str(name);
        s.push_str("\": {\n    \"hash\": \"");
        s.push_str(hash);
        s.push_str("\"\n  }");
        if i + 1 < collected.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("}\n");
    if let Err(e) = std::fs::write(path, s) {
        eprintln!("GENERATE: failed to write {}: {e}", path.display());
    }
}
