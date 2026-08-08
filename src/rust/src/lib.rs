pub use fceux11_core;
pub use fceux11_debug;
pub use fceux11_formats;
pub use fceux11_lua;
pub use fceux11_media;
pub use fceux11_utils;

// =========================================================================
// Stage-2 §七 (C-1) — single-symbol re-export of kagami_qa_direct_main.
//
// Rationale: cargo's staticlib does NOT propagate `#[no_mangle]` symbols from
// rlib transitive dependencies into the produced .lib file — even with the
// `direct-adapter` feature enabled, kagami-qa's symbol stays inside its rlib
// and is never exported by fceux11_rust.lib. LTO further eliminates unreferenced
// code from rlib objects before they reach the staticlib archive.
//
// This wrapper provides the reference that keeps `kagami_qa_direct_main`
// reachable through LTO, and is itself the single exported C-ABI entry point
// consumed by tests/kagami_direct_main.cpp.
//
// Compiled only when the `direct-adapter` feature is enabled (which the CMake
// build always passes — see src/rust/CMakeLists.txt).
// =========================================================================
#[cfg(feature = "direct-adapter")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kagami_qa_direct_main(
    argc: i32,
    argv: *const *const std::os::raw::c_char,
) -> i32 {
    // SAFETY: argv was constructed by the C caller per the C-ABI contract
    // (argc ≥ 0, argv points to argc null-terminated C strings). The inner
    // function replicates this contract verbatim.
    unsafe { kagami_qa::direct_entry::kagami_qa_direct_main(argc, argv) }
}

// =========================================================================
// Task 1 / C-1 — re-export of kagami_qa_blargg_main.
//
// Same rationale as kagami_qa_direct_main above: the staticlib does not
// propagate #[no_mangle] symbols from rlib transitive deps, so the blargg
// batch harness entry point must be re-exported here to survive LTO and
// reach the produced fceux11_rust.lib. Consumed by a thin C++ shim
// (tests/kagami/blargg_rust_main.cpp) that replaces tests/blargg_runner.cpp
// once parity is verified.
// =========================================================================
#[cfg(feature = "direct-adapter")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kagami_qa_blargg_main(
    argc: i32,
    argv: *const *const std::os::raw::c_char,
) -> i32 {
    // SAFETY: argv is constructed by the C caller per the C-ABI contract.
    unsafe { kagami_qa::blargg_entry::kagami_qa_blargg_main(argc, argv) }
}

// =========================================================================
// Task 1 / C-2 — re-export of kagami_qa_rom_regression_main.
//
// Replaces tests/rom_regression_test.cpp once parity is verified.
// =========================================================================
#[cfg(feature = "direct-adapter")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kagami_qa_rom_regression_main(
    argc: i32,
    argv: *const *const std::os::raw::c_char,
) -> i32 {
    // SAFETY: argv is constructed by the C caller per the C-ABI contract.
    unsafe { kagami_qa::rom_regression_entry::kagami_qa_rom_regression_main(argc, argv) }
}

// =========================================================================
// Task 1 / C-3 — re-export of kagami_qa_savestate_regression_main.
//
// Replaces tests/savestate_regression_test.cpp once parity is verified.
// =========================================================================
#[cfg(feature = "direct-adapter")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kagami_qa_savestate_regression_main(
    argc: i32,
    argv: *const *const std::os::raw::c_char,
) -> i32 {
    // SAFETY: argv is constructed by the C caller per the C-ABI contract.
    unsafe { kagami_qa::savestate_regression_entry::kagami_qa_savestate_regression_main(argc, argv) }
}

// =========================================================================
// Task 1 (mapper) — re-export of kagami_qa_mapper_byte_diff_main.
//
// Replaces tests/core/mapper_byte_diff_test.cpp once parity is verified.
// =========================================================================
#[cfg(feature = "direct-adapter")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kagami_qa_mapper_byte_diff_main(
    argc: i32,
    argv: *const *const std::os::raw::c_char,
) -> i32 {
    // SAFETY: argv is constructed by the C caller per the C-ABI contract.
    unsafe { kagami_qa::mapper_byte_diff_entry::kagami_qa_mapper_byte_diff_main(argc, argv) }
}

// =========================================================================
// Task 1 (lua) — re-export of kagami_qa_lua_main.
//
// Replaces tests/lua_runner.cpp once parity is verified.
// =========================================================================
#[cfg(feature = "direct-adapter")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kagami_qa_lua_main(
    argc: i32,
    argv: *const *const std::os::raw::c_char,
) -> i32 {
    // SAFETY: argv is constructed by the C caller per the C-ABI contract.
    unsafe { kagami_qa::lua_entry::kagami_qa_lua_main(argc, argv) }
}
