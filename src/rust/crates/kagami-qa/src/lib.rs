pub mod core;
pub mod manifest;
pub mod runner;
pub mod oracle;
pub mod report;
pub mod adapter;

// =========================================================================
// P5: C-callable entry point for kagami_qa_direct_runner (CMake target).
//
// Called from tests/kagami_direct_main.cpp when the direct runner is built
// via CMake with the direct-adapter Cargo feature enabled.
//
// This function parses CLI args, loads the test manifest, and runs all
// Oracle B tests in-process using Fceux11DirectAdapter (C ABI bridge).
// =========================================================================

#[cfg(feature = "direct-adapter")]
mod direct_entry {
    use std::collections::BTreeMap;
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use std::path::PathBuf;

    use crate::adapter::direct::Fceux11DirectAdapter;
    use crate::adapter::trait_def::{InputSpec, SutAdapter};
    use crate::manifest::parser::load_manifest;

    /// Main entry point called from C++ (kagami_direct_main.cpp).
    /// Parses CLI args and runs Oracle B tests in-process.
    #[no_mangle]
    pub unsafe extern "C" fn kagami_qa_direct_main(
        argc: i32,
        argv: *const *const c_char,
    ) -> i32 {
        // Convert C args to Rust strings.
        let mut args = Vec::new();
        for i in 0..argc {
            let ptr = *argv.offset(i as isize);
            if ptr.is_null() {
                break;
            }
            match CStr::from_ptr(ptr).to_str() {
                Ok(s) => args.push(s.to_string()),
                Err(_) => args.push(String::from("<invalid-utf8>")),
            }
        }

        // Parse key flags (same as main.rs).
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

        // Run Oracle B tests via direct adapter.
        let mut direct_adapter = Fceux11DirectAdapter::new();
        let mut passed = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        for (id, test) in &manifest {
            if test.input.rom.is_some() {
                let spec = InputSpec::from_manifest(test);
                eprint!("  [direct] {}: {} frames...", id, spec.frames);

                match direct_adapter.load(&spec) {
                    Ok(()) => {
                        let mut step_ok = true;
                        for _f in 0..spec.frames {
                            if direct_adapter.step().is_err() {
                                step_ok = false;
                                break;
                            }
                        }
                        if step_ok {
                            match direct_adapter.read_oracle_probe(spec.probe_addr) {
                                Ok(val) => {
                                    let is_pass = val == 0x00;
                                    if is_pass {
                                        passed += 1;
                                        eprintln!(" PASS (0x00)");
                                    } else {
                                        failed += 1;
                                        eprintln!(" FAIL (0x{:02X})", val);
                                    }
                                    results.push(crate::adapter::trait_def::TestResult {
                                        test_id: id.clone(),
                                        passed: is_pass,
                                        exit_code: if is_pass { 0 } else { 1 },
                                        stdout: format!(
                                            "BLARGG_RESULT: rom={} value=0x{:02X} status={}",
                                            id,
                                            val,
                                            if is_pass { "PASS" } else { "FAIL" }
                                        ),
                                        stderr: String::new(),
                                        duration_ms: 0,
                                        migration_note: Some("direct-adapter".into()),
                                    });
                                }
                                Err(e) => {
                                    failed += 1;
                                    eprintln!(" PROBE_ERROR: {:?}", e);
                                }
                            }
                        } else {
                            failed += 1;
                            eprintln!(" STEP_ERROR");
                        }
                        let _ = direct_adapter.reset();
                    }
                    Err(e) => {
                        failed += 1;
                        eprintln!(" LOAD_ERROR: {:?}", e);
                    }
                }
            }
        }

        eprintln!(
            "Direct run complete: {} total, {} PASS, {} FAIL",
            passed + failed,
            passed,
            failed
        );

        // Write minimal JSON report.
        let total = passed + failed;
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

        if failed > 0 { 1 } else { 0 }
    }
}

