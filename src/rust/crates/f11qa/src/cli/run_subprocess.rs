//! Subprocess-mode orchestration — the default execution path.
//!
//! Loads the manifest, drives every test through `SubprocessAdapter`
//! (forking the CTest test binaries) and hands the raw results to the
//! report layer. Task 4 (FCEUX11-1.17_计划.md §5.3 step 1): extracted
//! from the old monolithic main.rs.

use std::collections::BTreeMap;

use crate::adapter::subprocess::SubprocessAdapter;
use crate::adapter::trait_def::{SutAdapter, TestResult};
use crate::core::QaConfig;
use crate::manifest::parser::load_manifest;
use crate::manifest::schema::TestManifest;
use crate::runner::scheduler::TestScheduler;

use super::args::Args;

/// Everything the report layer needs from a subprocess run.
pub struct SubprocessOutcome {
    pub results: Vec<TestResult>,
    pub oracle_types: BTreeMap<String, String>,
    pub layers: BTreeMap<String, String>,
    pub manifest: BTreeMap<String, TestManifest>,
}

pub fn run(args: &Args) -> Result<SubprocessOutcome, Box<dyn std::error::Error>> {
    eprintln!("Loading manifest: {}", args.manifest_path.display());
    let mut manifest = load_manifest(&args.manifest_path)?;
    eprintln!("Loaded {} test entries.", manifest.len());

    // Task 4 §5.3 step 6: --filter narrows the manifest before scheduling,
    // so precision work can run a subset (e.g. "tag=blargg") without
    // external grep/ctest. Applied here AND in direct mode.
    if let Some(expr) = &args.filter_expr {
        let filter = crate::manifest::filter::Filter::parse(expr)
            .map_err(|e| format!("invalid --filter expression: {e}"))?;
        eprintln!("Filtering {} entries by '{}'", manifest.len(), expr);
        manifest = filter.apply(&manifest);
        eprintln!("Filtered to {} entries.", manifest.len());
    }

    let mut oracle_types = BTreeMap::new();
    let mut layers = BTreeMap::new();
    for (id, test) in &manifest {
        oracle_types.insert(id.clone(), format!("{:?}", test.oracle_type));
        layers.insert(id.clone(), format!("{:?}", test.layer));
    }

    let adapter = SubprocessAdapter::with_working_dir(&args.bin_dir, &args.working_dir);

    let config = QaConfig {
        manifest_path: args.manifest_path.clone(),
        bin_dir: args.bin_dir.clone(),
        working_dir: args.working_dir.clone(),
        output_path: args.output_path.clone(),
        timeout_seconds: 300,
    };

    // Stage-2 §九 L3: init with the real config, not a synthetic placeholder.
    // SubprocessAdapter::init is a no-op today, but the ordering is load-bearing
    // for any future adapter that reads config during init.
    adapter.init(&config)?;

    let scheduler = TestScheduler::new(config, manifest.clone());
    eprintln!("Running {} tests via subprocess adapter...", scheduler.len());
    let results = scheduler.run_all(&adapter);
    eprintln!("Done. {} results collected.", results.len());

    Ok(SubprocessOutcome {
        results,
        oracle_types,
        layers,
        manifest,
    })
}
