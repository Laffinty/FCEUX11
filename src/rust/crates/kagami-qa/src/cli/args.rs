//! CLI argument parsing for `kagami-qa-runner`.
//!
//! Task 4 (FCEUX11-1.17_计划.md §5.3 step 1): extracted from the old
//! monolithic main.rs so the flag grammar is a unit-testable unit. The
//! grammar is unchanged from v1.16; the only intentional behaviour
//! tightening is that a flag missing its value now fails loudly instead
//! of silently keeping the default (project precedent: R4-1 warning→error).

use std::path::PathBuf;

/// Parsed command-line arguments.
#[derive(Debug, Clone)]
pub struct Args {
    pub manifest_path: PathBuf,
    pub bin_dir: PathBuf,
    pub output_path: PathBuf,
    pub working_dir: PathBuf,
    pub accuracy_table_path: Option<PathBuf>,
    pub known_fail_path: Option<PathBuf>,
    pub baseline_path: Option<PathBuf>,
    pub save_baseline_path: Option<PathBuf>,
    pub filter_expr: Option<String>,
    pub direct: bool,
    /// Optional one-page PDF quality report output path (Task 1.17
    /// add-on: `report/pdf.rs`).
    pub pdf_report_path: Option<PathBuf>,
}

impl Args {
    /// Parse CLI arguments (excluding argv[0]).
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut manifest_path = PathBuf::from("tests/tests.json");
        let mut bin_dir = PathBuf::from("build/tests");
        let mut output_path = PathBuf::from("kagamiqa_migration_matrix.json");
        let mut working_dir = std::env::current_dir().unwrap_or_default();
        let mut accuracy_table_path: Option<PathBuf> = None;
        let mut known_fail_path: Option<PathBuf> = None;
        let mut baseline_path: Option<PathBuf> = None;
        let mut save_baseline_path: Option<PathBuf> = None;
        let mut filter_expr: Option<String> = None;
        let mut direct = false;
        let mut pdf_report_path: Option<PathBuf> = None;

        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--manifest" => {
                    manifest_path = PathBuf::from(next_value(&mut iter, "--manifest")?);
                }
                "--bin-dir" => {
                    bin_dir = PathBuf::from(next_value(&mut iter, "--bin-dir")?);
                }
                "--output" => {
                    output_path = PathBuf::from(next_value(&mut iter, "--output")?);
                }
                "--working-dir" => {
                    working_dir = PathBuf::from(next_value(&mut iter, "--working-dir")?);
                }
                "--accuracy-table" => {
                    accuracy_table_path =
                        Some(PathBuf::from(next_value(&mut iter, "--accuracy-table")?));
                }
                "--known-fail" => {
                    known_fail_path = Some(PathBuf::from(next_value(&mut iter, "--known-fail")?));
                }
                "--baseline" => {
                    baseline_path = Some(PathBuf::from(next_value(&mut iter, "--baseline")?));
                }
                "--save-baseline" => {
                    save_baseline_path =
                        Some(PathBuf::from(next_value(&mut iter, "--save-baseline")?));
                }
                "--filter" => {
                    filter_expr = Some(next_value(&mut iter, "--filter")?);
                }
                "--direct" => {
                    direct = true;
                }
                "--pdf-report" => {
                    pdf_report_path = Some(PathBuf::from(next_value(&mut iter, "--pdf-report")?));
                }
                other => {
                    return Err(format!("Unknown flag: {}", other));
                }
            }
        }

        Ok(Self {
            manifest_path,
            bin_dir,
            output_path,
            working_dir,
            accuracy_table_path,
            known_fail_path,
            baseline_path,
            save_baseline_path,
            filter_expr,
            direct,
            pdf_report_path,
        })
    }
}

/// Pull the value that must follow a flag, or fail loudly if absent.
fn next_value<I>(iter: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    iter.next()
        .ok_or_else(|| format!("Missing value for {}", flag))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(items: &[&str]) -> Args {
        Args::parse(items.iter().map(|s| s.to_string())).unwrap()
    }

    #[test]
    fn defaults() {
        let a = parse(&[]);
        assert_eq!(a.manifest_path, PathBuf::from("tests/tests.json"));
        assert_eq!(a.bin_dir, PathBuf::from("build/tests"));
        assert_eq!(a.output_path, PathBuf::from("kagamiqa_migration_matrix.json"));
        assert!(!a.direct);
        assert!(a.baseline_path.is_none());
        assert!(a.accuracy_table_path.is_none());
    }

    #[test]
    fn parses_all_flags() {
        let a = parse(&[
            "--manifest",
            "m.json",
            "--bin-dir",
            "b",
            "--output",
            "o.json",
            "--working-dir",
            "w",
            "--accuracy-table",
            "a.md",
            "--known-fail",
            "k.json",
            "--baseline",
            "base.json",
            "--save-baseline",
            "next.json",
            "--direct",
        ]);
        assert_eq!(a.manifest_path, PathBuf::from("m.json"));
        assert_eq!(a.bin_dir, PathBuf::from("b"));
        assert_eq!(a.output_path, PathBuf::from("o.json"));
        assert_eq!(a.working_dir, PathBuf::from("w"));
        assert_eq!(a.accuracy_table_path, Some(PathBuf::from("a.md")));
        assert_eq!(a.known_fail_path, Some(PathBuf::from("k.json")));
        assert_eq!(a.baseline_path, Some(PathBuf::from("base.json")));
        assert_eq!(a.save_baseline_path, Some(PathBuf::from("next.json")));
        assert!(a.direct);
    }

    #[test]
    fn unknown_flag_errors() {
        assert!(Args::parse(vec!["--nope".to_string()]).is_err());
    }

    #[test]
    fn filter_flag_parses() {
        let a = parse(&["--filter", "tag=blargg"]);
        assert_eq!(a.filter_expr.as_deref(), Some("tag=blargg"));
    }

    #[test]
    fn pdf_report_flag_parses() {
        let a = parse(&["--pdf-report", "build/report.pdf"]);
        assert_eq!(a.pdf_report_path, Some(PathBuf::from("build/report.pdf")));
    }

    #[test]
    fn missing_value_errors() {
        assert!(Args::parse(vec!["--manifest".to_string()]).is_err());
    }
}
