//! Direct (in-process) mode — frame-level driving via the C ABI bridge.
//!
//! Task 4 (FCEUX11-1.17_计划.md §5.3 step 2): the per-test driving loop
//! lives in `runner::direct::run_direct_rom_tests` (shared with the
//! C-ABI `direct_entry`); this module only wires it to the CLI.
//!
//! NOTE (v1.16 behaviour preserved): `--direct` prints per-ROM verdicts
//! to stderr, but the authoritative matrix still comes from the
//! subprocess scheduler (`cli::run_subprocess`). The fully in-process
//! report path is the `kagami_qa_direct_runner` CMake target.

use super::args::Args;

pub fn run(args: &Args) {
    #[cfg(feature = "direct-adapter")]
    {
        use crate::adapter::direct::Fceux11DirectAdapter;
        use crate::adapter::trait_def::SutAdapter;
        use crate::manifest::parser::load_manifest;
        use crate::runner::direct::run_direct_rom_tests;

        let mut manifest = match load_manifest(&args.manifest_path) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error loading manifest: {:?}", e);
                return;
            }
        };

        // Task 4 §5.3 step 6: same --filter support as the subprocess path.
        if let Some(expr) = &args.filter_expr {
            match crate::manifest::filter::Filter::parse(expr) {
                Ok(f) => {
                    eprintln!("[direct] filtering by '{}'", expr);
                    manifest = f.apply(&manifest);
                }
                Err(e) => {
                    eprintln!("[direct] invalid --filter expression: {}", e);
                    return;
                }
            }
        }

        let mut direct_adapter = Fceux11DirectAdapter::new();
        eprintln!(
            "P5: --direct mode: using Fceux11DirectAdapter (in-process via C ABI bridge)."
        );
        let results = run_direct_rom_tests(&mut direct_adapter, &manifest);

        // A-2 honesty guard (runner::direct_mode_honesty_guard): a
        // non-empty manifest that drove zero tests (no input.rom entries)
        // must not fall through as a silent success. This module has no
        // exit-code channel of its own (the authoritative matrix comes
        // from the subprocess pass), so a guard hit uses the crate's
        // existing fatal-exit style — the same `std::process::exit(1)`
        // cli::run applies to bad args. The subprocess path is untouched;
        // it is simply not reached in this refusal case.
        if let Err(msg) =
            crate::runner::direct_mode_honesty_guard(manifest.len(), results.len())
        {
            eprintln!("{}", msg);
            std::process::exit(1);
        }

        for r in &results {
            eprintln!(
                "  [direct] {}: {}",
                r.test_id,
                if r.passed { "PASS" } else { "FAIL" }
            );
        }
        eprintln!("Direct run complete: {} results.", results.len());
    }

    #[cfg(not(feature = "direct-adapter"))]
    {
        let _ = args;
        eprintln!("P5: --direct mode requested but binary was built without direct-adapter feature.");
        eprintln!("    Rebuild with: cargo build --features direct-adapter");
        eprintln!("    Or use the CMake kagami_qa_direct_runner target.");
        eprintln!("    Falling back to subprocess adapter.");
    }
}
