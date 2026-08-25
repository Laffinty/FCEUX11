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

// =========================================================================
// Phase 2 of the v2.1 PPU refactor plan — re-export of all fceux11_ppu_*
// C-ABI entry points. Same rationale as the kagami_qa_* wrappers above:
// the cargo staticlib does not propagate #[no_mangle] symbols from rlib
// transitive deps, so each Rust PPU FFI function is re-exported here
// under the same name to survive LTO and reach fceux11_rust.lib.
//
// Compiled unconditionally (no feature gate) because fceux11-ppu is
// always present in the workspace. The Phase 2 C++ bridge
// (`src/ppu_rust_bridge.cpp`) uses these symbols when
// `FCEUX11_RUST_PPU=ON`.
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_create() -> *mut fceux11_ppu::PpuState {
    fceux11_ppu::ffi::fceux11_ppu_create()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_destroy(state: *mut fceux11_ppu::PpuState) {
    fceux11_ppu::ffi::fceux11_ppu_destroy(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_power(state: *mut fceux11_ppu::PpuState) {
    fceux11_ppu::ffi::fceux11_ppu_power(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_reset(state: *mut fceux11_ppu::PpuState) {
    fceux11_ppu::ffi::fceux11_ppu_reset(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_video_system(state: *mut fceux11_ppu::PpuState, pal: bool) {
    fceux11_ppu::ffi::fceux11_ppu_set_video_system(state, pal)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_install_bus_callbacks(
    state: *mut fceux11_ppu::PpuState,
    cb: *const fceux11_ppu::ffi::fceux11_ppu_bus_callbacks,
) {
    fceux11_ppu::ffi::fceux11_ppu_install_bus_callbacks(state, cb)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_chr_window(
    state: *mut fceux11_ppu::PpuState,
    slot: u32,
    ptr: *const u8,
    len: usize,
    is_ram: bool,
) {
    fceux11_ppu::ffi::fceux11_ppu_set_chr_window(state, slot, ptr, len, is_ram)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_nt_window(
    state: *mut fceux11_ppu::PpuState,
    ptr: *const u8,
    len: usize,
) {
    fceux11_ppu::ffi::fceux11_ppu_set_nt_window(state, ptr, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_palette_window(
    state: *mut fceux11_ppu::PpuState,
    ptr: *const u8,
    len: usize,
) {
    fceux11_ppu::ffi::fceux11_ppu_set_palette_window(state, ptr, len)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_mirror_mode(
    state: *mut fceux11_ppu::PpuState,
    mode: u32,
) {
    fceux11_ppu::ffi::fceux11_ppu_set_mirror_mode(state, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_cpu_read(
    state: *mut fceux11_ppu::PpuState,
    addr: u16,
) -> u8 {
    fceux11_ppu::ffi::fceux11_ppu_cpu_read(state, addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_cpu_write(
    state: *mut fceux11_ppu::PpuState,
    addr: u16,
    val: u8,
) {
    fceux11_ppu::ffi::fceux11_ppu_cpu_write(state, addr, val)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_emulate_frame(
    state: *mut fceux11_ppu::PpuState,
    n_cycles: u32,
) -> i32 {
    fceux11_ppu::ffi::fceux11_ppu_emulate_frame(state, n_cycles)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_tick_cpu_cycle(
    state: *mut fceux11_ppu::PpuState,
    n_cycles: u32,
) -> i32 {
    fceux11_ppu::ffi::fceux11_ppu_tick_cpu_cycle(state, n_cycles)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_scanline(
    state: *const fceux11_ppu::PpuState,
) -> i16 {
    fceux11_ppu::ffi::fceux11_ppu_get_scanline(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_dot(
    state: *const fceux11_ppu::PpuState,
) -> u16 {
    fceux11_ppu::ffi::fceux11_ppu_get_dot(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_frame_count(
    state: *const fceux11_ppu::PpuState,
) -> u64 {
    fceux11_ppu::ffi::fceux11_ppu_get_frame_count(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_framebuffer(
    state: *mut fceux11_ppu::PpuState,
) -> *mut u8 {
    fceux11_ppu::ffi::fceux11_ppu_get_framebuffer(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_framebuffer_stride(
    state: *const fceux11_ppu::PpuState,
) -> u32 {
    fceux11_ppu::ffi::fceux11_ppu_get_framebuffer_stride(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_emergency_reset(
    state: *mut fceux11_ppu::PpuState,
) {
    fceux11_ppu::ffi::fceux11_ppu_emergency_reset(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_take_vbl_set_suppressed(
    state: *mut fceux11_ppu::PpuState,
) -> bool {
    fceux11_ppu::ffi::fceux11_ppu_take_vbl_set_suppressed(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_mark_vbl_set_suppressed(
    state: *mut fceux11_ppu::PpuState,
) {
    fceux11_ppu::ffi::fceux11_ppu_mark_vbl_set_suppressed(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_render_frame(
    state: *mut fceux11_ppu::PpuState,
) {
    fceux11_ppu::ffi::fceux11_ppu_render_frame(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_status_vbl_set_suppressed(
    state: *mut fceux11_ppu::PpuState,
) -> bool {
    fceux11_ppu::ffi::fceux11_ppu_get_status_vbl_set_suppressed(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_status_vbl_set_suppressed(
    state: *mut fceux11_ppu::PpuState,
) {
    fceux11_ppu::ffi::fceux11_ppu_set_status_vbl_set_suppressed(state)
}
