// KagamiQA runner — CLI entry point.
//
// Task 4 (FCEUX11-1.17_计划.md §5.3 step 1): argument parsing, execution
// modes and report generation live in the `kagami_qa::cli` module (L7);
// this file is a thin dispatch shell.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    kagami_qa::cli::run()
}
