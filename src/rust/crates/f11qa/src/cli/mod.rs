//! CLI layer (L7) — argument parsing and mode dispatch.
//!
//! Task 4 (FCEUX11-1.17_计划.md §5.3 step 1): the old 350-line main.rs
//! was split into this module so parsing, subprocess orchestration,
//! direct mode and report generation are individually unit-testable.
//! main.rs is now a thin dispatch shell.

pub(crate) mod args;
pub(crate) mod run_direct;
pub(crate) mod run_report;
pub(crate) mod run_subprocess;

use run_report::ReportInput;

/// Entry point for the `kagami-qa-runner` binary.
///
/// Preserves the v1.16 dispatch order: optional `--direct` stderr pass,
/// then the authoritative subprocess run, then report generation.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = match args::Args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("{}", msg);
            eprintln!(
                "Usage: kagami-qa-runner [--manifest tests/tests.json] [--bin-dir build/tests] \
                 [--output report.json] [--working-dir .] [--accuracy-table accuracy.md] \
                 [--known-fail known_fail.json] [--baseline previous_run.json] \
                 [--save-baseline next_baseline.json] [--filter 'tag=blargg & oracle=B'] \
                 [--pdf-report quality.pdf] [--direct]"
            );
            std::process::exit(1);
        }
    };

    if args.direct {
        run_direct::run(&args);
    }

    let outcome = run_subprocess::run(&args)?;
    let exit_code = run_report::generate(ReportInput {
        args: &args,
        outcome: &outcome,
    })?;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}
