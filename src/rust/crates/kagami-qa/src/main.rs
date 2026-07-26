// KagamiQA runner — invokes CTest tests via subprocess and produces JSON report.

use std::collections::BTreeMap;
use std::path::PathBuf;

use kagami_qa::adapter::subprocess::SubprocessAdapter;
use kagami_qa::adapter::trait_def::SutAdapter;
use kagami_qa::core::QaConfig;
use kagami_qa::manifest::parser::load_manifest;
use kagami_qa::report::matrix::build_matrix;
use kagami_qa::runner::scheduler::TestScheduler;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let mut manifest_path = PathBuf::from("tests/tests.json");
    let mut bin_dir = PathBuf::from("build/tests");
    let mut output_path = PathBuf::from("kagamiqa_migration_matrix.json");
    let mut working_dir = std::env::current_dir().unwrap_or_default();

    // Simple CLI arg parsing (no external crate needed for P1).
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
            _ => {
                eprintln!("Unknown flag: {}", args[i]);
                eprintln!("Usage: kagami-qa-runner [--manifest tests/tests.json] [--bin-dir build/tests] [--output report.json] [--working-dir .]");
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
        manifest_path,
        bin_dir: bin_dir.clone(),
        working_dir: working_dir.clone(),
        output_path: output_path.clone(),
        timeout_seconds: 300,
    };

    let scheduler = TestScheduler::new(config, manifest);
    adapter.init(&scheduler_config_default())?;

    eprintln!("Running {} tests via subprocess adapter...", scheduler.len());
    let results = scheduler.run_all(&adapter);
    eprintln!("Done. {} results collected.", results.len());

    let matrix = build_matrix(results, &oracle_types, &layers);

    let json = serde_json::to_string_pretty(&matrix)?;
    std::fs::write(&output_path, &json)?;
    eprintln!("Report written to: {}", output_path.display());

    // Print summary to stdout.
    println!("Total:  {}", matrix.summary.total);
    println!("Passed: {}", matrix.summary.passed);
    println!("Failed: {}", matrix.summary.failed);

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
