pub mod trait_def;
pub mod subprocess;
// In-process adapter — requires linking against fceux11_core (C++).
// Skip during `cargo test`; available in full cmake build and `cargo build`.
#[cfg(not(test))]
pub mod direct;
