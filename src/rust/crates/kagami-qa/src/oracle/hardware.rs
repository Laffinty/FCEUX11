// KagamiQA P2 — Oracle B: hardware-consistency oracle via blargg $6000 protocol.
//
// The blargg test ROM protocol writes PASS (0x00) or FAIL (0x01-0xFF) to
// memory address $6000 after completing its test sequence. Bytes $6001-$6003
// carry optional diagnostic detail.
//
// This module parses the structured output of `fceux11_blargg_runner` and
// provides per-ROM verdicts + aggregate statistics for the accuracy table.
//
// Protocol reference:
//   $6000 = 0x00 → PASS (emulator matches real hardware for this test)
//   $6000 = 0x01-0xFF → FAIL (emulator diverges from real hardware)
//   $6001-$6003 = optional diagnostic sub-codes

use serde::{Deserialize, Serialize};

/// Parsed result from a single blargg ROM run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlarggResult {
    pub rom_name: String,
    pub probe_addr: u16,
    pub value: u8,
    pub diag: [u8; 3],
    pub status: BlarggStatus,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BlarggStatus {
    #[serde(rename = "PASS")]
    Pass,
    #[serde(rename = "FAIL")]
    Fail,
    /// ROM failed to load or runner crashed before producing a $6000 value.
    #[serde(rename = "ERROR")]
    Error,
}

/// Known-failure baseline entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownFailure {
    pub rom: String,
    pub category: String,
    pub result_code: String,
    pub expected_to_eventually_pass: bool,
    pub notes: String,
}

/// Known-failure baseline manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownFailureBaseline {
    pub baseline_version: String,
    pub generated_at: String,
    pub engine: String,
    pub summary: BaselineSummary,
    pub failures: Vec<KnownFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineSummary {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub timeout: usize,
}

/// Accuracy table row (TASVideos format).
#[derive(Debug, Clone, Serialize)]
pub struct AccuracyRow {
    pub rom: String,
    pub category: String,
    pub status: String,    // "PASS" / "FAIL (0xNN)" / "ERROR"
    pub code: String,       // "0x00" / "0x01" / "N/A"
    pub notes: String,
}

/// Parse a single `BLARGG_RESULT:` line from the blargg runner stdout.
///
/// Format: `BLARGG_RESULT: rom=<name> addr=0x6000 value=0xNN diag=[0xNN,0xNN,0xNN] status=PASS|FAIL duration_ms=<N>`
pub fn parse_blargg_line(line: &str) -> Option<BlarggResult> {
    let line = line.trim();
    if !line.starts_with("BLARGG_RESULT:") {
        return None;
    }

    let payload = line.strip_prefix("BLARGG_RESULT:")?.trim();

    // Extract fields with simple substring parsing (avoids regex dependency).
    let rom_name = extract_field(payload, "rom=", ' ')?;
    let addr_str = extract_field(payload, "addr=", ' ')?;
    let value_str = extract_field(payload, "value=", ' ')?;
    let diag_str = extract_field(payload, "diag=", ' ')?;
    let status_str = extract_field(payload, "status=", ' ')?;
    let duration_str = extract_field(payload, "duration_ms=", ' ');

    let probe_addr = u16::from_str_radix(addr_str.trim_start_matches("0x"), 16).ok()?;
    let value = u8::from_str_radix(value_str.trim_start_matches("0x"), 16).ok()?;

    // Parse diag=[0xNN,0xNN,0xNN]
    let diag = parse_diag_tuple(diag_str)?;

    let status = match status_str {
        "PASS" => BlarggStatus::Pass,
        "FAIL" => BlarggStatus::Fail,
        _ => BlarggStatus::Error,
    };

    let duration_ms = duration_str
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    Some(BlarggResult {
        rom_name: rom_name.to_string(),
        probe_addr,
        value,
        diag,
        status,
        duration_ms,
    })
}

/// Parse a batch JSON output from the blargg runner into individual results.
pub fn parse_batch_json(json: &str) -> Result<Vec<BlarggResult>, serde_json::Error> {
    #[derive(Deserialize)]
    struct BatchOutput {
        results: Vec<BlarggResult>,
    }
    let batch: BatchOutput = serde_json::from_str(json)?;
    Ok(batch.results)
}

/// Extract a `key=value` field from a delimited string.
fn extract_field<'a>(s: &'a str, key: &str, delim: char) -> Option<&'a str> {
    let rest = s.find(key)?;
    let after_key = &s[rest + key.len()..];
    let end = after_key.find(delim).unwrap_or(after_key.len());
    Some(&after_key[..end])
}

/// Parse `[0xNN,0xNN,0xNN]` into [u8; 3].
fn parse_diag_tuple(s: &str) -> Option<[u8; 3]> {
    let s = s.trim().trim_start_matches('[').trim_end_matches(']');
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 3 {
        return None;
    }
    let d0 = u8::from_str_radix(parts[0].trim().trim_start_matches("0x"), 16).ok()?;
    let d1 = u8::from_str_radix(parts[1].trim().trim_start_matches("0x"), 16).ok()?;
    let d2 = u8::from_str_radix(parts[2].trim().trim_start_matches("0x"), 16).ok()?;
    Some([d0, d1, d2])
}

/// Generate a TASVideos-format accuracy table from blargg results.
pub fn build_accuracy_table(results: &[BlarggResult]) -> Vec<AccuracyRow> {
    results
        .iter()
        .map(|r| {
            let (status_str, code_str) = match &r.status {
                BlarggStatus::Pass => ("✅ PASS".to_string(), format!("0x{:02X}", r.value)),
                BlarggStatus::Fail => (
                    format!("❌ FAIL (0x{:02X})", r.value),
                    format!("0x{:02X}", r.value),
                ),
                BlarggStatus::Error => ("⚠ ERROR".to_string(), "N/A".to_string()),
            };

            // Derive category from ROM name prefix.
            let category = if r.rom_name.starts_with("cpu") {
                "CPU"
            } else if r.rom_name.contains("sprite") || r.rom_name.contains("vbl") {
                "PPU"
            } else if r.rom_name.starts_with("apu") {
                "APU"
            } else if r.rom_name.starts_with("mmc") {
                "MMC3"
            } else {
                "other"
            };

            let notes = if r.value == 0xFE {
                "ROM load failure".to_string()
            } else if r.value == 0xFF {
                "Runner error".to_string()
            } else if r.status == BlarggStatus::Pass {
                format!("Passed in {}ms", r.duration_ms)
            } else {
                format!(
                    "Diag: [{:02X},{:02X},{:02X}]",
                    r.diag[0], r.diag[1], r.diag[2]
                )
            };

            AccuracyRow {
                rom: r.rom_name.clone(),
                category: category.to_string(),
                status: status_str,
                code: code_str,
                notes,
            }
        })
        .collect()
}

/// Generate an accuracy comparison table as a Markdown string.
pub fn accuracy_table_to_markdown(rows: &[AccuracyRow]) -> String {
    let mut md = String::new();
    md.push_str("# FCEUX11 v1.16 Blargg Accuracy Table\n\n");
    md.push_str("> Generated by KagamiQA P2 Oracle B runner.\n");
    md.push_str("> Protocol: blargg $6000 memory-mapped result (0x00=PASS, 0x01+=FAIL).\n");
    md.push_str("> Format: TASVideos emulator accuracy comparison.\n\n");

    // Summary stats
    let total = rows.len();
    let passed = rows.iter().filter(|r| r.status.contains("PASS")).count();
    let failed = total - passed;
    md.push_str(&format!(
        "| | Count |\n|---|---|\n| Total | {} |\n| Passed | {} |\n| Failed | {} |\n\n",
        total, passed, failed
    ));

    // Per-ROM table
    md.push_str("| ROM | Category | Status | Code | Notes |\n");
    md.push_str("|-----|----------|--------|------|-------|\n");
    for row in rows {
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            row.rom, row.category, row.status, row.code, row.notes
        ));
    }
    md.push('\n');
    md
}

/// Check a blargg result against the known-failure baseline.
/// Returns true if this failure is documented in the baseline (known).
pub fn is_known_failure(result: &BlarggResult, baseline: &KnownFailureBaseline) -> bool {
    baseline
        .failures
        .iter()
        .any(|kf| kf.rom == result.rom_name)
}

/// Compute summary from a TestResult's stdout containing blargg output.
/// Extracts BLARGG_RESULT lines and returns aggregated statistics.
pub fn summarize_blargg_output(stdout: &str) -> (usize, usize, usize) {
    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;

    for line in stdout.lines() {
        if let Some(r) = parse_blargg_line(line) {
            total += 1;
            match r.status {
                BlarggStatus::Pass => passed += 1,
                _ => failed += 1,
            }
        }
    }

    (total, passed, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pass_line() {
        let line = "BLARGG_RESULT: rom=01-basics.nes addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=123";
        let r = parse_blargg_line(line).unwrap();
        assert_eq!(r.rom_name, "01-basics.nes");
        assert_eq!(r.value, 0x00);
        assert_eq!(r.status, BlarggStatus::Pass);
        assert_eq!(r.duration_ms, 123);
    }

    #[test]
    fn parse_fail_line() {
        let line = "BLARGG_RESULT: rom=cpu_timing_test.nes addr=0x6000 value=0x03 diag=[0x01,0x00,0x00] status=FAIL duration_ms=456";
        let r = parse_blargg_line(line).unwrap();
        assert_eq!(r.rom_name, "cpu_timing_test.nes");
        assert_eq!(r.value, 0x03);
        assert_eq!(r.status, BlarggStatus::Fail);
        assert_eq!(r.diag, [0x01, 0x00, 0x00]);
    }

    #[test]
    fn non_blargg_line_returns_none() {
        assert!(parse_blargg_line("some other output").is_none());
        assert!(parse_blargg_line("").is_none());
    }

    #[test]
    fn accuracy_table_generation() {
        let results = vec![
            BlarggResult {
                rom_name: "01-basics.nes".into(),
                probe_addr: 0x6000,
                value: 0x00,
                diag: [0; 3],
                status: BlarggStatus::Pass,
                duration_ms: 100,
            },
            BlarggResult {
                rom_name: "cpu_timing_test.nes".into(),
                probe_addr: 0x6000,
                value: 0x03,
                diag: [1, 0, 0],
                status: BlarggStatus::Fail,
                duration_ms: 200,
            },
        ];
        let table = build_accuracy_table(&results);
        assert_eq!(table.len(), 2);
        assert!(table[0].status.contains("PASS"));
        assert!(table[1].status.contains("FAIL"));
    }

    #[test]
    fn summarize_output() {
        let stdout = "BLARGG_RESULT: rom=a.nes addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=10\nBLARGG_RESULT: rom=b.nes addr=0x6000 value=0x01 diag=[0x01,0x00,0x00] status=FAIL duration_ms=20\n";
        let (total, passed, failed) = summarize_blargg_output(stdout);
        assert_eq!(total, 2);
        assert_eq!(passed, 1);
        assert_eq!(failed, 1);
    }

    /// P2 end-to-end: simulate full blargg pipeline → parse → table → markdown.
    #[test]
    fn e2e_blargg_pipeline() {
        // Simulated output from fceux11_blargg_runner --manifest (batch mode).
        let simulated_stdout = "\
BLARGG_RESULT: rom=01-basics.nes addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=123
BLARGG_RESULT: rom=02-implied.nes addr=0x6000 value=0x00 diag=[0x00,0x00,0x00] status=PASS duration_ms=115
BLARGG_RESULT: rom=cpu_timing_test.nes addr=0x6000 value=0x03 diag=[0x01,0x00,0x00] status=FAIL duration_ms=456
BLARGG_RESULT: rom=vbl_nmi_timing.nes addr=0x6000 value=0x01 diag=[0x02,0x00,0x00] status=FAIL duration_ms=234
BLARGG_RESULT: rom=apu_test.nes addr=0x6000 value=0xFF diag=[0x07,0x00,0x00] status=FAIL duration_ms=567
";

        // Step 1: Parse all BLARGG_RESULT lines.
        let mut results = Vec::new();
        for line in simulated_stdout.lines() {
            if let Some(r) = parse_blargg_line(line) {
                results.push(r);
            }
        }
        assert_eq!(results.len(), 5, "should parse 5 results");

        // Step 2: Verify parse correctness.
        assert_eq!(results[0].status, BlarggStatus::Pass);
        assert_eq!(results[0].value, 0x00);
        assert_eq!(results[2].status, BlarggStatus::Fail);
        assert_eq!(results[2].value, 0x03);

        // Step 3: Build accuracy table.
        let table = build_accuracy_table(&results);
        assert_eq!(table.len(), 5);
        let pass_count = table.iter().filter(|r| r.status.contains("PASS")).count();
        let fail_count = table.iter().filter(|r| r.status.contains("FAIL")).count();
        assert_eq!(pass_count, 2);
        assert_eq!(fail_count, 3);

        // Step 4: Generate markdown.
        let md = accuracy_table_to_markdown(&table);
        assert!(md.contains("FCEUX11 v1.16 Blargg Accuracy Table"));
        assert!(md.contains("01-basics.nes"));
        assert!(md.contains("cpu_timing_test.nes"));
        assert!(md.contains("✅ PASS"));
        assert!(md.contains("❌ FAIL"));
        assert!(md.contains("0x00"));   // PASS value
        assert!(md.contains("0x03"));   // FAIL value

        // Step 5: Verify known-failure baseline logic.
        let baseline = serde_json::from_str::<KnownFailureBaseline>(
            r#"{
            "baseline_version": "v1.16.0-P2",
            "generated_at": "2026-07-27T00:00:00Z",
            "engine": "FCEUX11 v1.16",
            "summary": {"total": 5, "pass": 2, "fail": 3, "timeout": 0},
            "failures": [
              {"rom": "cpu_timing_test.nes", "category": "cpu", "result_code": "0x03", "expected_to_eventually_pass": true, "notes": ""},
              {"rom": "vbl_nmi_timing.nes", "category": "ppu", "result_code": "0x01", "expected_to_eventually_pass": true, "notes": ""}
            ]
        }"#).unwrap();

        // Known failures should match.
        assert!(is_known_failure(&results[2], &baseline));  // cpu_timing_test
        assert!(is_known_failure(&results[3], &baseline));  // vbl_nmi_timing
        // apu_test is NOT in the baseline → not a known failure.
        assert!(!is_known_failure(&results[4], &baseline)); // apu_test — unexpected

        // Step 6: Summarize.
        let (total, passed, failed) = summarize_blargg_output(simulated_stdout);
        assert_eq!(total, 5);
        assert_eq!(passed, 2);
        assert_eq!(failed, 3);
    }
}
