// KagamiQA Task 1 / C-2 — Thin C++ shim for the Rust ROM regression harness.
//
// Replaces tests/rom_regression_test.cpp once parity is verified. The Rust
// side provides:
//   extern "C" int kagami_qa_rom_regression_main(int argc, const char** argv);
// implemented in src/rust/crates/kagami-qa/src/runner/rom_regression.rs and
// re-exported by src/rust/src/lib.rs (fceux11_rust.lib).

extern "C" {
    int kagami_qa_rom_regression_main(int argc, const char** argv);
}

int main(int argc, char** argv) {
    return kagami_qa_rom_regression_main(argc, const_cast<const char**>(argv));
}
