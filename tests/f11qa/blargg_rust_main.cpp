// KagamiQA Task 1 / C-1 — Thin C++ shim for the Rust blargg batch harness.
//
// Replaces tests/blargg_runner.cpp once parity (C++ vs Rust over the 177
// blargg ROMs in blargg_manifest.json) is verified. The Rust side provides:
//   extern "C" int kagami_qa_blargg_main(int argc, const char** argv);
// implemented in src/rust/crates/kagami-qa/src/runner/blargg.rs and
// re-exported by src/rust/src/lib.rs (fceux11_rust.lib).
//
// Usage is identical to the C++ runner:
//   fceux11_blargg_runner_rust [--manifest tests/fixtures/blargg_manifest.json]
//   fceux11_blargg_runner_rust --rom <path> [--frames N] [--reset-after N]

extern "C" {
    /// Entry point into the Rust kagami-qa blargg harness (direct mode).
    int kagami_qa_blargg_main(int argc, const char** argv);
}

int main(int argc, char** argv) {
    return kagami_qa_blargg_main(argc, const_cast<const char**>(argv));
}
