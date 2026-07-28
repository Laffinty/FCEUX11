pub mod trait_def;
pub mod subprocess;
// P5: In-process adapter — requires linking against fceux11_core (C++).
// Available when built with the "direct-adapter" Cargo feature (enabled
// by the CMake kagami_qa_direct_runner target). Also available in plain
// `cargo build` (non-test) for compilation-check purposes, but runtime
// use requires the CMake-linked build.
#[cfg(any(feature = "direct-adapter", not(test)))]
pub mod direct;
