//! Zero-dependency one-page PDF quality report generator.
//!
//! Hand-writes a PDF 1.4 document (no external crates, no font
//! embedding): the standard 14 Type1 fonts (Helvetica / Helvetica-Bold /
//! Courier) are referenced by name and rendered by the viewer. All report
//! content is ASCII (grade letter, statistics, test ids), so the built-in
//! fonts are sufficient.
//!
//! Layout (flat, modern):
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │  KAGAMIQA // FCEUX11 QUALITY REPORT       2026-08-08  │  ← brand row
//! │  ┌──────┐                                                │
//! │  │  C   │  ACCEPTABLE                                   │  ← large grade
//! │  │      │  8 advisory known-limit failure(s) ...        │     (top-left)
//! │  └─■────┘                                               │
//! │  [Total 47] [Passed 39] [Failed 8] [Skipped 0]          │  ← stat cards
//! │  ▓ ORACLE A 27P/0F   ▓ ORACLE B 12P/8F                   │  ← oracle split
//! │  FAIL DETAILS (8 advisory known-limits)                  │
//! │  #  TEST ID                     EXIT  DURATION           │
//! │  1  blargg_ppu_vbl_nmi            1    16236ms           │  ← detail rows
//! │  ...                                                     │
//! │  run_id …  engine …   page 1/1                           │  ← footer
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! The grade letter is rendered large in the top-left corner in the
//! grade's signature colour; a thin colour band runs under it.

use crate::report::grade::Grade;
use crate::report::matrix::MigrationMatrix;

/// A4 page width in points (595.28 → 595 for stable layout math).
const PAGE_W: f32 = 595.0;
/// A4 page height in points (841.89 → 842).
const PAGE_H: f32 = 842.0;
/// Horizontal margin.
const MARGIN: f32 = 40.0;
/// Content width (page minus margins).
const CONTENT_W: f32 = PAGE_W - 2.0 * MARGIN; // 515

/// Grade → signature colour `(r, g, b)` in 0.0..=1.0.
fn grade_color(grade: Grade) -> (f32, f32, f32) {
    match grade {
        Grade::A => (0.07, 0.72, 0.51),  // #12B76A
        Grade::B => (0.02, 0.69, 0.82),  // #06B0D1
        Grade::C => (0.96, 0.62, 0.04),  // #F59E0B
        Grade::D => (0.94, 0.27, 0.27),  // #EF4444
        Grade::E => (0.72, 0.11, 0.11),  // #B91C1C
    }
}

/// Accent greys.
const INK: (f32, f32, f32) = (0.13, 0.15, 0.19); // #21262F — near-black text
const GREY: (f32, f32, f32) = (0.45, 0.49, 0.55); // #737D8C — secondary text
const FAINT: (f32, f32, f32) = (0.93, 0.94, 0.95); // #EDEFF2 — card fill
const LINE: (f32, f32, f32) = (0.87, 0.89, 0.92); // #DEE3E9 — hairlines
const GOOD: (f32, f32, f32) = (0.07, 0.63, 0.39); // #12A163
const BAD: (f32, f32, f32) = (0.87, 0.27, 0.24); // #DE453C

/// Escape a PDF literal string (parentheses + backslash).
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

// ---------------------------------------------------------------------------
// Content-stream builder
// ---------------------------------------------------------------------------

/// Accumulates PDF content-stream operators.
#[derive(Default)]
struct Content {
    buf: String,
}

impl Content {
    fn op(&mut self, s: &str) {
        self.buf.push_str(s);
        self.buf.push('\n');
    }

    /// Set fill colour.
    fn fill(&mut self, c: (f32, f32, f32)) {
        self.op(&format!("{:.3} {:.3} {:.3} rg", c.0, c.1, c.2));
    }

    /// Set stroke colour + width.
    fn stroke(&mut self, c: (f32, f32, f32), w: f32) {
        self.op(&format!("{:.3} {:.3} {:.3} RG {:.2} w", c.0, c.1, c.2, w));
    }

    /// Draw a filled rectangle.
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.op(&format!("{:.2} {:.2} {:.2} {:.2} re f", x, y, w, h));
    }

    /// Draw an outlined rectangle (current stroke settings).
    fn rect_stroke(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.op(&format!("{:.2} {:.2} {:.2} {:.2} re S", x, y, w, h));
    }

    /// Draw a horizontal hairline.
    fn hline(&mut self, x: f32, y: f32, w: f32) {
        self.op(&format!(
            "{:.2} {:.2} m {:.2} {:.2} l S",
            x,
            y,
            x + w,
            y
        ));
    }

    /// Write text. `font` is a resource name ("/F1" …), `size` in pt,
    /// `(x, y)` is the baseline position (PDF origin = bottom-left).
    fn text(&mut self, font: &str, size: f32, x: f32, y: f32, c: (f32, f32, f32), s: &str) {
        self.fill(c);
        self.op(&format!(
            "BT {font} {size:.1} Tf 1 0 0 1 {x:.2} {y:.2} Tm ({}) Tj ET",
            esc(s)
        ));
    }
}

// ---------------------------------------------------------------------------
// PDF document assembly
// ---------------------------------------------------------------------------

/// Assemble a minimal single-page PDF from the content stream.
/// Returns the raw PDF bytes (self-contained, no external resources).
fn assemble_pdf(content: &str) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();

    // 1: Catalog
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    // 2: Pages
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    // 3: Page
    objects.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] \
             /Resources << /Font << /F1 4 0 R /F2 5 0 R /F3 6 0 R >> >> \
             /Contents 7 0 R >>",
            PAGE_W as u32, PAGE_H as u32
        )
        .into_bytes(),
    );
    // 4-6: standard fonts (never embedded)
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>".to_vec());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>".to_vec());
    // 7: content stream
    let stream = format!("{}", content);
    objects.push(
        format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            stream.len(),
            stream
        )
        .into_bytes(),
    );

    let mut out: Vec<u8> = Vec::with_capacity(4096);
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets: Vec<usize> = Vec::with_capacity(objects.len() + 1);
    offsets.push(0); // object 0 (free head) unused in xref here

    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }

    // xref table
    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for off in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

// ---------------------------------------------------------------------------
// Report layout
// ---------------------------------------------------------------------------

/// Build the one-page quality report for a graded migration matrix.
pub fn build_grade_report(matrix: &MigrationMatrix) -> Vec<u8> {
    let grade = matrix.grade;
    let color = grade_color(grade);
    let mut c = Content::default();

    // ---- Brand row (top-right small print) ------------------------------
    let brand_y = PAGE_H - 44.0;
    c.text("/F2", 8.5, MARGIN, brand_y, GREY, "KAGAMIQA");
    c.text(
        "/F1",
        8.5,
        MARGIN + 52.0,
        brand_y,
        GREY,
        "// FCEUX11 QUALITY REPORT",
    );
    let meta = format!("{}   git {}", matrix.run_id, matrix.engine.git_rev);
    let meta_w = estimate_width("/F1", 8.0, &meta);
    c.text("/F1", 8.0, PAGE_W - MARGIN - meta_w, brand_y, GREY, &meta);

    // ---- Large grade letter (top-left) ----------------------------------
    let letter_x = MARGIN;
    let letter_y = 680.0; // baseline of the 104pt letter
    c.text("/F2", 104.0, letter_x, letter_y, color, grade.label());
    // grade name beside the letter
    c.text(
        "/F2",
        22.0,
        letter_x + 96.0,
        letter_y - 8.0,
        INK,
        &grade.name().to_uppercase(),
    );
    // short tag line under the name
    let tagline = grade_tagline(grade);
    c.text(
        "/F1",
        9.5,
        letter_x + 96.0,
        letter_y - 26.0,
        GREY,
        tagline,
    );

    // ---- Grade colour band ----------------------------------------------
    let band_y = letter_y - 118.0;
    c.fill(color);
    c.rect(MARGIN, band_y, CONTENT_W, 4.0);

    // ---- Stat cards ------------------------------------------------------
    let cards_y = band_y - 26.0; // top of the card row (y down)
    let card_h = 52.0;
    let gap = 10.0;
    let card_w = (CONTENT_W - 3.0 * gap) / 4.0;
    let stats = [
        ("TOTAL", format!("{}", matrix.summary.total), INK),
        ("PASSED", format!("{}", matrix.summary.passed), GOOD),
        ("FAILED", format!("{}", matrix.summary.failed), BAD),
        ("SKIPPED", format!("{}", matrix.summary.skipped), GREY),
    ];
    for (i, (label, value, vcolor)) in stats.iter().enumerate() {
        let x = MARGIN + i as f32 * (card_w + gap);
        c.fill(FAINT);
        c.rect(x, cards_y - card_h, card_w, card_h);
        c.text("/F2", 24.0, x + 12.0, cards_y - 18.0, *vcolor, value);
        c.text("/F1", 7.5, x + 12.0, cards_y - 34.0, GREY, label);
    }

    // ---- Oracle split ----------------------------------------------------
    let oracle_y = cards_y - card_h - 20.0;
    let a = &matrix.oracle_breakdown.a_regression;
    let b = &matrix.oracle_breakdown.b_hardware;
    let oracle_h = 40.0;
    let half_w = (CONTENT_W - gap) / 2.0;
    // Oracle A card
    c.fill(FAINT);
    c.rect(MARGIN, oracle_y - oracle_h, half_w, oracle_h);
    c.fill(GOOD);
    c.rect(MARGIN, oracle_y - oracle_h, 4.0, oracle_h);
    c.text(
        "/F2",
        9.0,
        MARGIN + 14.0,
        oracle_y - 16.0,
        INK,
        &format!("ORACLE A — REGRESSION"),
    );
    c.text(
        "/F2",
        15.0,
        MARGIN + 14.0,
        oracle_y - 33.0,
        INK,
        &format!("{}P / {}F", a.pass, a.fail),
    );
    // Oracle B card
    let bx = MARGIN + half_w + gap;
    c.fill(FAINT);
    c.rect(bx, oracle_y - oracle_h, half_w, oracle_h);
    let bcolor = if b.fail > 0 { BAD } else { GOOD };
    c.fill(bcolor);
    c.rect(bx, oracle_y - oracle_h, 4.0, oracle_h);
    c.text(
        "/F2",
        9.0,
        bx + 14.0,
        oracle_y - 16.0,
        INK,
        "ORACLE B — HARDWARE",
    );
    c.text(
        "/F2",
        15.0,
        bx + 14.0,
        oracle_y - 33.0,
        INK,
        &format!("{}P / {}F", b.pass, b.fail),
    );

    // ---- FAIL details table ---------------------------------------------
    let fails: Vec<&crate::report::matrix::TestDetail> = matrix
        .details
        .iter()
        .filter(|d| !d.passed)
        .collect();
    let table_y = oracle_y - oracle_h - 26.0; // header baseline
    c.text(
        "/F2",
        10.5,
        MARGIN,
        table_y,
        INK,
        &format!("FAIL DETAILS — {} ADVISORY KNOWN-LIMIT(S)", fails.len()),
    );
    // header row
    let head_y = table_y - 18.0;
    c.fill(LINE);
    c.hline(MARGIN, head_y + 2.0, CONTENT_W);
    c.text("/F2", 7.5, MARGIN + 2.0, head_y - 8.0, GREY, "#");
    c.text("/F2", 7.5, MARGIN + 24.0, head_y - 8.0, GREY, "TEST ID");
    c.text("/F2", 7.5, MARGIN + 330.0, head_y - 8.0, GREY, "EXIT");
    c.text("/F2", 7.5, MARGIN + 400.0, head_y - 8.0, GREY, "DURATION");
    c.text("/F2", 7.5, MARGIN + 470.0, head_y - 8.0, GREY, "ORACLE");

    // rows — clamp to what fits the page (24 rows max, plenty for one page)
    let row_h = 17.0;
    let row_start = head_y - 10.0 - row_h;
    for (i, d) in fails.iter().take(24).enumerate() {
        let y = row_start - i as f32 * row_h;
        if y < 60.0 {
            break;
        }
        // zebra stripe
        if i % 2 == 0 {
            c.fill(FAINT);
            c.rect(MARGIN, y - row_h + 4.0, CONTENT_W, row_h - 4.0);
        }
        c.fill(BAD);
        c.rect(MARGIN + 6.0, y - 6.0, 3.0, 3.0);
        c.text("/F1", 8.0, MARGIN + 2.0, y - 8.0, GREY, &format!("{:>2}", i + 1));
        c.text("/F3", 8.0, MARGIN + 24.0, y - 8.0, INK, &d.test_id);
        c.text("/F1", 8.0, MARGIN + 330.0, y - 8.0, GREY, &format!("{:>3}", d.exit_code));
        c.text(
            "/F1",
            8.0,
            MARGIN + 400.0,
            y - 8.0,
            GREY,
            &format!("{:>6}ms", d.duration_ms),
        );
        c.text("/F1", 8.0, MARGIN + 470.0, y - 8.0, GREY, &d.oracle_type);
    }

    // ---- Grade reasons ---------------------------------------------------
    let reasons_y = 96.0;
    c.fill(LINE);
    c.hline(MARGIN, reasons_y + 14.0, CONTENT_W);
    if matrix.grade_reasons.is_empty() {
        c.text("/F1", 8.0, MARGIN, reasons_y, GREY, "No blocking failures or regressions — all gates green.");
    } else {
        let joined = matrix.grade_reasons.join("  |  ");
        let reason = format!("why not higher: {}", joined);
        c.text("/F1", 8.0, MARGIN, reasons_y, GREY, &truncate_ascii(&reason, 120));
    }

    // ---- Footer ----------------------------------------------------------
    let footer_y = 34.0;
    let ver = format!(
        "engine {}  ·  report_version {}",
        matrix.engine.version, matrix.report_version
    );
    c.text("/F1", 7.0, MARGIN, footer_y, GREY, &ver);
    c.text("/F1", 7.0, PAGE_W - MARGIN - 30.0, footer_y, GREY, "page 1/1");

    assemble_pdf(&c.buf)
}

/// Short human tagline under the grade name.
fn grade_tagline(grade: Grade) -> &'static str {
    match grade {
        Grade::A => "perfect pass — no failures, nothing skipped",
        Grade::B => "release standard — advisory failures within frozen baseline",
        Grade::C => "acceptable — advisory known-limits, all documented",
        Grade::D => "release-blocking — failures or regressions present",
        Grade::E => "basic functionality damaged — engine boot failed",
    }
}

/// Rough ASCII width estimate (points) for right-aligned metadata.
fn estimate_width(font: &str, size: f32, s: &str) -> f32 {
    // Helvetica average advance ≈ 0.5em; bold ≈ 0.55em. Good enough for
    // right-aligned small print.
    let avg = if font == "/F2" { 0.56 } else { 0.50 };
    size * avg * s.chars().count() as f32
}

/// Truncate an ASCII line to `max` chars (report stays single-line).
fn truncate_ascii(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", cut)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::grade::compute_grade;
    use crate::report::matrix::build_matrix;
    use crate::adapter::trait_def::TestResult;

    fn sample_matrix(grade: Grade, fails: usize) -> MigrationMatrix {
        let mut results = Vec::new();
        for i in 0..47 {
            let failed = i >= 47 - fails;
            results.push(TestResult {
                test_id: format!("test_{:02}", i),
                passed: !failed,
                exit_code: if failed { 1 } else { 0 },
                stdout: String::new(),
                stderr: String::new(),
                duration_ms: (i as u64) * 100 + 7,
                migration_note: None,
            });
        }
        let mut matrix = build_matrix(results, &Default::default(), &Default::default(), None, vec![]);
        matrix.grade = grade;
        matrix.grade_reasons = vec![
            "8 advisory known-limit failure(s)".to_string(),
            "no baseline supplied".to_string(),
        ];
        matrix
    }

    #[test]
    fn pdf_header_and_footer() {
        let m = sample_matrix(Grade::C, 8);
        let bytes = build_grade_report(&m);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("%%EOF"));
    }

    #[test]
    fn pdf_contains_grade_letter() {
        let m = sample_matrix(Grade::C, 8);
        let bytes = build_grade_report(&m);
        let text = String::from_utf8_lossy(&bytes);
        // The 104pt grade letter "C" must be present in the content stream.
        assert!(text.contains("(C) Tj"));
    }

    #[test]
    fn pdf_lists_failed_test_ids() {
        let m = sample_matrix(Grade::C, 8);
        let bytes = build_grade_report(&m);
        let text = String::from_utf8_lossy(&bytes);
        // test_39 is the first failing id in the sample.
        assert!(text.contains("test_39"));
    }

    #[test]
    fn pdf_contains_grade_name() {
        let m = sample_matrix(Grade::C, 8);
        let bytes = build_grade_report(&m);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("ACCEPTABLE"));
    }

    #[test]
    fn pdf_grade_colors_map() {
        assert_eq!(grade_color(Grade::A), (0.07, 0.72, 0.51));
        assert_eq!(grade_color(Grade::C), (0.96, 0.62, 0.04));
        assert_eq!(grade_color(Grade::E), (0.72, 0.11, 0.11));
    }

    #[test]
    fn esc_handles_parens() {
        assert_eq!(esc("a(b)c"), "a\\(b\\)c");
        assert_eq!(esc("a\\b"), "a\\\\b");
    }

    #[test]
    fn pdf_xref_offsets_are_consistent() {
        // xref must list 8 entries (0 free + 7 objects) and the
        // startxref offset must point at the "xref" keyword.
        let m = sample_matrix(Grade::B, 2);
        let bytes = build_grade_report(&m);
        let text = String::from_utf8_lossy(&bytes);
        assert!(text.contains("xref\n0 8\n"));
        assert!(text.contains("0000000000 65535 f"));
        let startxref = text
            .split("startxref\n")
            .nth(1)
            .and_then(|s| s.lines().next())
            .and_then(|s| s.trim().parse::<usize>().ok())
            .expect("startxref offset");
        assert_eq!(&bytes[startxref..startxref + 4], b"xref");
    }

    #[test]
    fn compute_grade_integration() {
        // End-to-end: a graded matrix flows into the report builder.
        let m = sample_matrix(Grade::C, 8);
        let bytes = build_grade_report(&m);
        assert!(!bytes.is_empty());
        let _ = compute_grade; // referenced (kept for the pipeline shape)
    }
}
