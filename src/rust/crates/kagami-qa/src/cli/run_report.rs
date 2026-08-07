//! Report generation — baseline, drift, migration matrix, accuracy table,
//! known-failure cross-check and the process exit code.
//!
//! Task 4 (FCEUX11-1.17_计划.md §5.3 step 1): extracted from the old
//! monolithic main.rs so the report pipeline is one unit. Behaviour is
//! byte-for-byte the same as v1.16 (same fields, same stdout summary,
//! same exit-code rule: any failed test → exit 1).

use crate::oracle::hardware::{self, accuracy_table_to_markdown, build_accuracy_table, parse_blargg_line};
use crate::report::baseline::{self, detect_drift, snapshot_from_results};
use crate::report::matrix::build_matrix;

use super::args::Args;
use super::run_subprocess::SubprocessOutcome;

/// Inputs the report layer consumes from the run.
pub struct ReportInput<'a> {
    pub args: &'a Args,
    pub outcome: &'a SubprocessOutcome,
}

/// Generate the migration matrix + accuracy table, persist them, and
/// return the process exit code (0 = all passed, 1 = any failed).
pub fn generate(input: ReportInput) -> Result<i32, Box<dyn std::error::Error>> {
    let args = input.args;
    let outcome = input.outcome;
    let results = &outcome.results;

    // -------------------------------------------------------------------
    // Previous baseline + drift detection.
    // -------------------------------------------------------------------
    let previous = args.baseline_path.as_ref().and_then(|p| {
        eprintln!("Loading previous baseline: {}", p.display());
        baseline::load_baseline(p)
    });

    let mut current_pass_map = std::collections::BTreeMap::new();
    for r in results {
        current_pass_map.insert(r.test_id.clone(), r.passed);
    }
    let drifts = detect_drift(&outcome.manifest, previous.as_ref(), &current_pass_map);

    // -------------------------------------------------------------------
    // Build primary migration matrix (full §八 format).
    // -------------------------------------------------------------------
    let mut matrix = build_matrix(
        results.clone(),
        &outcome.oracle_types,
        &outcome.layers,
        previous.as_ref(),
        drifts,
    );

    // -------------------------------------------------------------------
    // Task 5 — release-readiness grade (A–E) attached to the matrix.
    // -------------------------------------------------------------------
    let (grade, grade_reasons) =
        crate::report::grade::compute_grade(&matrix, &outcome.manifest);
    matrix.grade = grade;
    matrix.grade_reasons = grade_reasons;

    // -------------------------------------------------------------------
    // Save current run as baseline for the next invocation.
    // -------------------------------------------------------------------
    if let Some(ref save_path) = args.save_baseline_path {
        let mut snap_results = std::collections::BTreeMap::new();
        for r in results {
            snap_results.insert(r.test_id.clone(), r.passed);
        }
        let snap = snapshot_from_results(&matrix.run_id, &snap_results);
        baseline::save_baseline(save_path, &snap)?;
        eprintln!("Baseline saved to: {}", save_path.display());
    }

    // -------------------------------------------------------------------
    // Write matrix JSON.
    // -------------------------------------------------------------------
    let json = serde_json::to_string_pretty(&matrix)?;
    std::fs::write(&args.output_path, &json)?;
    eprintln!("Report written to: {}", args.output_path.display());

    // -------------------------------------------------------------------
    // Stdout summary.
    // -------------------------------------------------------------------
    println!("Total:   {}", matrix.summary.total);
    println!("Passed:  {}", matrix.summary.passed);
    println!("Failed:  {}", matrix.summary.failed);
    println!(
        "Grade: {} ({})",
        matrix.grade.label(),
        matrix.grade.name()
    );

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

    println!(
        "Oracle A: {}P / {}F | Oracle B: {}P / {}F",
        matrix.oracle_breakdown.a_regression.pass,
        matrix.oracle_breakdown.a_regression.fail,
        matrix.oracle_breakdown.b_hardware.pass,
        matrix.oracle_breakdown.b_hardware.fail,
    );

    // -------------------------------------------------------------------
    // Oracle B — parse blargg $6000 results from stdout and generate the
    // accuracy comparison table + known-failure cross-check.
    // -------------------------------------------------------------------
    let mut all_blargg_results = Vec::new();
    for r in results {
        for line in r.stdout.lines() {
            if let Some(br) = parse_blargg_line(line) {
                all_blargg_results.push(br);
            }
        }
    }

    if !all_blargg_results.is_empty() {
        let btotal = all_blargg_results.len();
        let bpassed = all_blargg_results
            .iter()
            .filter(|r| r.status == hardware::BlarggStatus::Pass)
            .count();
        let bfailed = btotal - bpassed;
        eprintln!(
            "Oracle B (blargg): {} total, {} PASS, {} FAIL",
            btotal, bpassed, bfailed
        );

        // Accuracy table (Markdown).
        if let Some(ref table_path) = args.accuracy_table_path {
            let accuracy_rows = build_accuracy_table(&all_blargg_results);
            let md = accuracy_table_to_markdown(&accuracy_rows);
            std::fs::write(table_path, &md)?;
            eprintln!("Accuracy table written to: {}", table_path.display());
        }

        // Cross-reference with the versioned known-failure baseline.
        if let Some(ref kf_path) = args.known_fail_path {
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
                                eprintln!("  - {}: 0x{:02X}", uf.rom_name, uf.value);
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

    // Stage-2 §九: any failed test fails the run (advisory/blocking nuance
    // is surfaced in the matrix, not the exit code).
    Ok(if matrix.summary.failed > 0 { 1 } else { 0 })
}
