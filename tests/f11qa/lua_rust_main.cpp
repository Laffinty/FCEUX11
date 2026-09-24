// KagamiQA Task 1 (lua) — Thin C++ shim for the Rust headless Lua
// script runner.
//
// Replaces tests/lua_runner.cpp once parity (C++ vs Rust over the lua
// test scripts) is verified. The Rust side provides:
//   extern "C" int kagami_qa_lua_main(int argc, const char** argv);
// implemented in src/rust/crates/kagami-qa/src/runner/lua.rs and
// re-exported by src/rust/src/lib.rs (fceux11_rust.lib).
//
// Usage is identical to the C++ runner:
//   kagami_qa_lua_runner <script_path> [--rom <path>] [--frames N]

extern "C" {
    /// Entry point into the Rust kagami-qa Lua runner (direct mode).
    int kagami_qa_lua_main(int argc, const char** argv);
}

int main(int argc, char** argv) {
    return kagami_qa_lua_main(argc, const_cast<const char**>(argv));
}
