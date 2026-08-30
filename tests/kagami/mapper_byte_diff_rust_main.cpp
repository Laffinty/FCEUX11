// KagamiQA Task 1 (mapper) — Thin C++ shim for the Rust mapper
// byte-diff harness.
//
// Replaces tests/core/mapper_byte_diff_test.cpp once parity (C++ vs
// Rust over the 175 mapper cases in fixtures/golden_mapper/) is
// verified. The Rust side provides:
//   extern "C" int kagami_qa_mapper_byte_diff_main(int argc, const char** argv);
// implemented in src/rust/crates/kagami-qa/src/runner/mapper_byte_diff.rs
// and re-exported by src/rust/src/lib.rs (fceux11_rust.lib).
//
// Default mode (verify):
//   kagami_qa_mapper_byte_diff_runner [--only <name>] [--frames N]
//
// Generate mode (Phase 6.1.c, was C++-only before Phase 5.3 retired
// that harness):
//   kagami_qa_mapper_byte_diff_runner --only <name> --frames N --generate
// overwrites fixtures/golden_mapper/<name>.bin with the live mapper
// state captured at the (overridden) frame count. Used to author a
// fresh golden at a non-default frame count (e.g. mmc1_frame0.bin).

#include <cstdio>
#include <cstring>

extern "C" {
    /// Entry point into the Rust kagami-qa mapper byte-diff harness
    /// (direct mode).
    int kagami_qa_mapper_byte_diff_main(int argc, const char** argv);
}

int main(int argc, char** argv) {
    // All args are forwarded to the Rust harness verbatim — verify and
    // generate are both handled there.
    return kagami_qa_mapper_byte_diff_main(argc, const_cast<const char**>(argv));
}
