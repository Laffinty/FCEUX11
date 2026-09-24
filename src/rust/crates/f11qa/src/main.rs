// F11QA runner — CLI entry point.
//
// v1.8 §1: F11QA = KagamiQA 的 v1.8 重命名。
// CLI argument parsing, execution modes and report generation live in
// the `f11qa::cli` module; this file is a thin dispatch shell.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    f11qa::cli::run()
}
