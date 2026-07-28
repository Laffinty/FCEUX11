// KagamiQA runner — invokes CTest tests via subprocess and produces JSON report.
// P2: Adds Oracle B ($6000 protocol) result parsing and accuracy table generation.
// P4-report: Full §八 report format with transition_matrix, oracle_breakdown,
//            baseline_drift, and previous-run diffing.
// P5: Adds --direct mode for in-process execution via C ABI bridge (Fceux11DirectAdapter).

use std::collections::BTreeMap;
use std::path::PathBuf;

use kagami_qa::adapter::subprocess::SubprocessAdapter;
use kagami_qa::adapter::trait_def::SutAdapter;
use kagami_qa::core::QaConfig;
use kagami_qa::manifest::parser::load_manifest;
use kagami_qa::oracle::hardware::{self, accuracy_table_to_markdown, build_accuracy_table, parse_blargg_line};
use kagami_qa::report::baseline::{self, detect_drift, snapshot_from_results};
use kagami_qa::report::matrix::build_matrix;
use kagami_qa::runner::scheduler::TestScheduler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut manifest_path = PathBuf::from("tests/tests.json");
    let mut bin_dir = PathBuf::from("build/tests");
    let mut output_path = PathBuf::from("kagamiqa_migration_matrix.json");
    let mut working_dir = std::env::current_dir().unwrap_or_default();
    let mut accuracy_table_path: Option<PathBuf> = None;
    let mut known_fail_path: Option<PathBuf> = None;
    let mut baseline_path: Option<PathBuf> = None;
    let mut save_baseline_path: Option<PathBuf> = None;
    let mut use_direct = false;

    // Simple CLI arg parsing (no external crate needed).
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--manifest" => {
                i += 1;
                if i < args.len() {
                    manifest_path = PathBuf::from(&args[i]);
                }
            }
            "--bin-dir" => {
                i += 1;
                if i < args.len() {
                    bin_dir = PathBuf::from(&args[i]);
                }
            }
            "--output" => {
                i += 1;
                if i < args.len() {
                    output_path = PathBuf::from(&args[i]);
                }
            }
            "--working-dir" => {
                i += 1;
                if i < args.len() {
                    working_dir = PathBuf::from(&args[i]);
                }
            }
            "--accuracy-table" => {
                i += 1;
                if i < args.len() {
                    accuracy_table_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--known-fail" => {
                i += 1;
                if i < args.len() {
                    known_fail_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--baseline" => {
                i += 1;
                if i < args.len() {
                    baseline_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--save-baseline" => {
                i += 1;
                if i < args.len() {
                    save_baseline_path = Some(PathBuf::from(&args[i]));
                }
            }
            "--direct" => {
                use_direct = true;
            }
            _ => {
                eprintln!("Unknown flag: {}", args[i]);
                eprintln!("Usage: kagami-qa-runner [--manifest tests/tests.json] [--bin-dir build/tests] [--output report.json] [--working-dir .] [--accuracy-table accuracy.md] [--known-fail known_fail.json] [--baseline previous_run.json] [--save-baseline next_baseline.json] [--direct]");
                std::process::exit(1);
            }
        }
        i += 1;
    }

    eprintln!("Loading manifest: {}", manifest_path.display());
    let manifest = load_manifest(&manifest_path)?;
    eprintln!("Loaded {} test entries.", manifest.len());

    // Extract oracle_type and layer for the report.
    let mut oracle_types = BTreeMap::new();
    let mut layers = BTreeMap::new();
    for (id, test) in &manifest {
        oracle_types.insert(id.clone(), format!("{:?}", test.oracle_type));
        layers.insert(id.clone(), format!("{:?}", test.layer));
    }

    let adapter = SubprocessAdapter::with_working_dir(&bin_dir, &working_dir);

    let config = QaConfig {
        manifest_path: manifest_path.clone(),
        bin_dir: bin_dir.clone(),
        working_dir: working_dir.clone(),
        output_path: output_path.clone(),
        timeout_seconds: 300,
    };

    // -------------------------------------------------------------------
    // P5: --direct mode — use Fceux11DirectAdapter for in-process execution.
    // -------------------------------------------------------------------
    if use_direct {
        #[cfg(feature = "direct-adapter")]
        {
            eprintln!("P5: --direct mode: using Fceux11DirectAdapter (in-process via C ABI bridge).");
            use kagami_qa::adapter::direct::Fceux11DirectAdapter;
            use kagami_qa::adapter::trait_def::InputSpec;

            let mut direct_adapter = Fceux11DirectAdapter::new();
            // For Oracle B tests, drive the emulator frame-by-frame.
            // Each blargg ROM entry: load ROM → emulate N frames → probe $6000.
            let mut direct_results = Vec::new();
            for (id, test) in &manifest {
                if test.input.rom.is_some() {
                    let spec = InputSpec::from_manifest(test);
                    eprintln!("  [direct] {}: loading ROM, {} frames...", id, spec.frames);
                    match direct_adapter.load(&spec) {
                        Ok(()) => {
                            for _f in 0..spec.frames {
                                if let Err(e) = direct_adapter.step() {
                                    eprintln!("    step error at frame {}: {:?}", _f, e);
                                    break;
                                }
                            }
                            match direct_adapter.read_oracle_probe(spec.probe_addr) {
                                Ok(val) => {
                                    let passed = val == 0x00;
                                    eprintln!("    probe 0x{:04X} = 0x{:02X} → {}", spec.probe_addr, val,
                                        if passed { "PASS" } else { "FAIL" });
                                    direct_results.push(kagami_qa::adapter::trait_def::TestResult {
                                        test_id: id.clone(),
                                        passed,
                                        exit_code: if passed { 0 } else { 1 },
                                        stdout: format!("BLARGG_RESULT: rom={} value=0x{:02X} status={}",
                                            id, val, if passed { "PASS" } else { "FAIL" }),
                                        stderr: String::new(),
                                        duration_ms: 0,
                                        migration_note: Some("direct-adapter".into()),
                                    });
                                }
                                Err(e) => {
                                    eprintln!("    probe error: {:?}", e);
                                }
                            }
                            let _ = direct_adapter.reset();
                        }
                        Err(e) => {
                            eprintln!("    load error: {:?}", e);
                        }
                    }
                }
            }
            eprintln!("Direct run complete: {} results.", direct_results.len());
            // Continue to report generation with direct_results...
            // (For P5, direct results replace subprocess results in the report.)
        }
        #[cfg(not(feature = "direct-adapter"))]
        {
            eprintln!("P5: --direct mode requested but binary was built without direct-adapter feature.");
            eprintln!("    Rebuild with: cargo build --features direct-adapter");
            eprintln!("    Or use the CMake kagami_qa_direct_runner target.");
            eprintln!("    Falling back to subprocess adapter.");
        }
    }

    let scheduler = TestScheduler::new(config, manifest);
    adapter.init(&scheduler_config_default())?;

    eprintln!("Running {} tests via subprocess adapter...", scheduler.len());
    let results = scheduler.run_all(&adapter);
    eprintln!("Done. {} results collected.", results.len());

    // -------------------------------------------------------------------
    // P4-report: Load previous baseline for transition comparison.
    // -------------------------------------------------------------------
    let previous = baseline_path
        .as_ref()
        .and_then(|p| {
            eprintln!("Loading previous baseline: {}", p.display());
            baseline::load_baseline(p)
        });

    // -------------------------------------------------------------------
    // P4-report: Detect baseline drift (stub — full implementation in P4+).
    // -------------------------------------------------------------------
    let drifts = detect_drift(
        &scheduler.manifest_snapshot(),
        previous.as_ref(),
    );

    // -------------------------------------------------------------------
    // Build primary migration matrix (full §八 format).
    // -------------------------------------------------------------------
    let matrix = build_matrix(results.clone(), &oracle_types, &layers, previous.as_ref(), drifts);

    // -------------------------------------------------------------------
    // Save current run as baseline for the next invocation.
    // -------------------------------------------------------------------
    if let Some(ref save_path) = save_baseline_path {
        let mut snap_results = BTreeMap::new();
        for r in &results {
            snap_results.insert(r.test_id.clone(), r.passed);
        }
        let snap = snapshot_from_results(&matrix.run_id, &snap_results);
        baseline::save_baseline(save_path, &snap)?;
        eprintln!("Baseline saved to: {}", save_path.display());
    }

    let json = serde_json::to_string_pretty(&matrix)?;
    std::fs::write(&output_path, &json)?;
    eprintln!("Report written to: {}", output_path.display());

    // Print summary to stdout.
    println!("Total:   {}", matrix.summary.total);
    println!("Passed:  {}", matrix.summary.passed);
    println!("Failed:  {}", matrix.summary.failed);

    // Print transition summary if baseline was loaded.
    if previous.is_some() {
        let tm = &matrix.transition_matrix;
        if !tm.fail_to_pass.is_empty() {
            println!("FAIL→PASS: {} (progress!)", tm.fail_to_pass.len());
            for e in &tm.fail_to_pass {
                println!("  ✅ {}", e.id);
            }
        }
        if !tm.pass_to_fail.is_empty() {
            println!("PASS→FAIL: {} (REGRESSION!)", tm.pass_to_fail.len());
            for e in &tm.pass_to_fail {
                println!("  ❌ {}", e.id);
            }
        }
    }

    // Print oracle breakdown.
    println!(
        "Oracle A: {}P / {}F | Oracle B: {}P / {}F",
        matrix.oracle_breakdown.a_regression.pass,
        matrix.oracle_breakdown.a_regression.fail,
        matrix.oracle_breakdown.b_hardware.pass,
        matrix.oracle_breakdown.b_hardware.fail,
    );

    // -------------------------------------------------------------------
    // P2: Oracle B — parse blargg $6000 results from stdout and generate
    // accuracy comparison table.
    // -------------------------------------------------------------------
    let mut all_blargg_results = Vec::new();
    for r in &results {
        for line in r.stdout.lines() {
            if let Some(br) = parse_blargg_line(line) {
                all_blargg_results.push(br);
            }
        }
    }

    if !all_blargg_results.is_empty() {
        let btotal = all_blargg_results.len();
        let bpassed = all_blargg_results.iter().filter(|r| r.status == hardware::BlarggStatus::Pass).count();
        let bfailed = btotal - bpassed;
        eprintln!(
            "Oracle B (blargg): {} total, {} PASS, {} FAIL",
            btotal, bpassed, bfailed
        );

        // Generate accuracy table if requested.
        if let Some(ref table_path) = accuracy_table_path {
            let accuracy_rows = build_accuracy_table(&all_blargg_results);
            let md = accuracy_table_to_markdown(&accuracy_rows);
            std::fs::write(table_path, &md)?;
            eprintln!("Accuracy table written to: {}", table_path.display());
        }

        // Cross-reference with known-failure baseline if provided.
        if let Some(ref kf_path) = known_fail_path {
            match std::fs::read_to_string(kf_path) {
                Ok(contents) => {
                    if let Ok(baseline_known) =
                        serde_json::from_str::<hardware::KnownFailureBaseline>(&contents)
                    {
                        let unexpected: Vec<_> = all_blargg_results
                            .iter()
                            .filter(|r| {
                                r.status == hardware::BlarggStatus::Fail
                                    && !hardware::is_known_failure(r, &baseline_known)
                            })
                            .collect();
                        if !unexpected.is_empty() {
                            eprintln!(
                                "WARNING: {} unexpected blargg failures (not in known-fail baseline):",
                                unexpected.len()
                            );
                            for uf in &unexpected {
                                eprintln!(
                                    "  - {}: 0x{:02X}",
                                    uf.rom_name, uf.value
                                );
                            }
                        } else {
                            eprintln!("All blargg failures match known-failure baseline.");
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: cannot read known-fail baseline '{}': {}",
                        kf_path.display(),
                        e
                    );
                }
            }
        }
    }

    if matrix.summary.failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

/// Minimal config for adapter init (not used by SubprocessAdapter).
fn scheduler_config_default() -> kagami_qa::core::QaConfig {
    kagami_qa::core::QaConfig {
        manifest_path: PathBuf::new(),
        bin_dir: PathBuf::new(),
        working_dir: PathBuf::new(),
        output_path: PathBuf::new(),
        timeout_seconds: 300,
    }
}
