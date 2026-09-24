// KagamiQA P5 — Thin C++ entry point for in-process direct runner.
//
// This executable links the Rust kagami-qa static library (built with the
// direct-adapter feature) against fceux11_core + fceux11_drivers_null.
// The Rust library uses the C ABI bridge (kagami_bridge_* functions from
// src/kagami_bridge.cpp) to drive the emulator frame-by-frame without
// spawning a subprocess.
//
// The Rust side provides:
//   extern "C" int kagami_qa_direct_main(int argc, const char** argv);
//
// Usage:
//   kagami_qa_direct_runner [--manifest tests.json] [--output report.json] [...]

extern "C" {
    /// Entry point into the Rust kagami-qa library (direct mode).
    /// Parses CLI args, loads the test manifest, and runs all tests
    /// via Fceux11DirectAdapter. Returns 0 on success, 1 on failure.
    int kagami_qa_direct_main(int argc, const char** argv);
}

int main(int argc, char** argv) {
    // Pass control to the Rust side. The Rust main parses CLI args,
    // loads the test manifest, and runs all Oracle B tests in-process
    // using the C ABI bridge.
    return kagami_qa_direct_main(argc, const_cast<const char**>(argv));
}
