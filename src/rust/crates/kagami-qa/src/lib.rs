pub mod core;
pub mod manifest;
pub mod runner;
pub mod oracle;
pub mod report;
pub mod adapter;
pub mod cli;

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
        let mut direct_adapter = Fceux11DirectAdapter::new();
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
