pub mod trait_def;
pub mod subprocess;
// P5: In-process adapter — requires linking against fceux11_core (C++).
// Available ONLY when built with the "direct-adapter" Cargo feature
// (enabled by the CMake kagami_qa_direct_runner target). Plain `cargo
// build` / `cargo test` excludes this module so the linker never sees
// the kagami_bridge_* / fceux11_lua_* extern "C" symbols (those live in
// the C++ libraries that only the CMake build links against).
#[cfg(feature = "direct-adapter")]
pub mod direct;
