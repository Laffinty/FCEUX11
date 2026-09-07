// Root staticlib crate — Rust 2024 made `unsafe_op_in_unsafe_fn` deny
// by default, but this crate is a thin pass-through re-export layer
// that forwards every entry point straight to a per-crate FFI shim.
// The inner crates already wrap their unsafe calls in explicit
// `unsafe {}` blocks; allowing the lint here keeps the wrapper
// surface noise-free.
#![allow(unsafe_op_in_unsafe_fn)]

use std::path::Path;

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
    unsafe {
        kagami_qa::savestate_regression_entry::kagami_qa_savestate_regression_main(argc, argv)
    }
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
pub unsafe extern "C" fn fceux11_ppu_set_video_system(
    state: *mut fceux11_ppu::PpuState,
    pal: bool,
) {
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
pub unsafe extern "C" fn fceux11_ppu_set_mirror_mode(state: *mut fceux11_ppu::PpuState, mode: u32) {
    fceux11_ppu::ffi::fceux11_ppu_set_mirror_mode(state, mode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_cpu_read(state: *mut fceux11_ppu::PpuState, addr: u16) -> u8 {
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
pub unsafe extern "C" fn fceux11_ppu_tick_dots(
    state: *mut fceux11_ppu::PpuState,
    n_dots: u32,
) -> i32 {
    fceux11_ppu::ffi::fceux11_ppu_tick_dots(state, n_dots)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_take_nmi_pending(state: *mut fceux11_ppu::PpuState) -> i32 {
    fceux11_ppu::ffi::fceux11_ppu_take_nmi_pending(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_scanline(state: *const fceux11_ppu::PpuState) -> i16 {
    fceux11_ppu::ffi::fceux11_ppu_get_scanline(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_dot(state: *const fceux11_ppu::PpuState) -> u16 {
    fceux11_ppu::ffi::fceux11_ppu_get_dot(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_frame_count(state: *const fceux11_ppu::PpuState) -> u64 {
    fceux11_ppu::ffi::fceux11_ppu_get_frame_count(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_framebuffer(state: *mut fceux11_ppu::PpuState) -> *mut u8 {
    fceux11_ppu::ffi::fceux11_ppu_get_framebuffer(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_framebuffer_stride(
    state: *const fceux11_ppu::PpuState,
) -> u32 {
    fceux11_ppu::ffi::fceux11_ppu_get_framebuffer_stride(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_emergency_reset(state: *mut fceux11_ppu::PpuState) {
    fceux11_ppu::ffi::fceux11_ppu_emergency_reset(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_take_vbl_set_suppressed(
    state: *mut fceux11_ppu::PpuState,
) -> bool {
    fceux11_ppu::ffi::fceux11_ppu_take_vbl_set_suppressed(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_mark_vbl_set_suppressed(state: *mut fceux11_ppu::PpuState) {
    fceux11_ppu::ffi::fceux11_ppu_mark_vbl_set_suppressed(state)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_render_frame(state: *mut fceux11_ppu::PpuState) {
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

// =========================================================================
// v2.1 Phase 6.3.a — PPU internal data-bus open-bus + decay
//
// The root crate's cbindgen header doesn't see fceux11-ppu's FFI
// surface (cbindgen 0.29.3 + Rust-2024 `pub unsafe extern "C"` issue
// documented at `src/rust/build.rs`). The pattern below mirrors the
// other Phase 5+ exports in this file: hand-write the prototype in
// `merge_headers` (build.rs) and re-export through the root crate so
// the staticlib symbol is present. The C++ bridge calls these from
// per-CPU-cycle and per-frame paths.
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_set_current_cpu_cycle(
    state: *mut fceux11_ppu::PpuState,
    current_cpu_cycle: u64,
) {
    fceux11_ppu::ffi::fceux11_ppu_set_current_cpu_cycle(state, current_cpu_cycle)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_refresh_data_bus(
    state: *mut fceux11_ppu::PpuState,
    val: u8,
    current_cpu_cycle: u64,
) {
    fceux11_ppu::ffi::fceux11_ppu_refresh_data_bus(state, val, current_cpu_cycle)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_check_data_bus_decay(
    state: *mut fceux11_ppu::PpuState,
    current_cpu_cycle: u64,
) {
    fceux11_ppu::ffi::fceux11_ppu_check_data_bus_decay(state, current_cpu_cycle)
}

// =========================================================================
// v2.1 Phase 6.3.b — DMC DMA arbitration (scaffolding)
//
// Records the most recent DMC DMA stall request from the C++ APU.
// Currently inert — see `docs/history/v2.1_phase6_batch_compat.md`
// §6.3.b for the deferred per-dot-loop integration. The actual stall
// (skip `cpu.run(1)` for N cycles + advance timestamp) requires a new
// `fceux11_cpu_advance_cycles` API on the Rust CPU crate, which is a
// multi-session task. This FFI exists so the C++ APU can wire its
// hook NOW without having to revisit the build later.
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_dmc_dma_arbitration(
    state: *mut fceux11_ppu::PpuState,
    stall_cycles: u8,
) {
    fceux11_ppu::ffi::fceux11_ppu_dmc_dma_arbitration(state, stall_cycles)
}

// Phase 6.3.c.1: take-and-clear companion to
// fceux11_ppu_dmc_dma_arbitration. Consumed by the per-dot interleave
// loop in `fceux11_run_frame_interleaved` below.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_take_dmc_dma_stall(
    state: *mut fceux11_ppu::PpuState,
) -> u8 {
    fceux11_ppu::ffi::fceux11_ppu_take_dmc_dma_stall(state)
}

// =========================================================================
// v2.1 Phase 5.3 — in-Rust CPU/PPU per-dot interleave loop.
//
// Phase 5.1 drove the interleave from C++ (`FCEUPPU_Loop`): per dot it
// made THREE FFI crossings (advance_ppu_dots + take_nmi + Cpu::run).
// At 89342 dots/frame that is ~268k boundary crossings per frame and
// the dominant cost of the bench_tolerance_test +165% regression.
//
// This entry point moves the loop itself into Rust — one FFI per
// frame — while keeping the EXACT per-dot operation sequence the
// Phase 5.1 C++ loop validated (tick 1 dot → take NMI latch → run CPU
// by 1 dot unit). The CPU call reuses `fceux11_cpu_run_with_tick`
// verbatim (including its per-call working-copy sync of the 64-byte
// C++ blob), so C++-side state mutation visibility is unchanged; only
// the call-site orchestration moved across the boundary. Mapper event
// hooks still reach C++ through the existing vtable thunks and the
// per-instruction tick thunk, at the same dots as before.
// =========================================================================

/// Pulse the CPU NMI line (`TriggerNMI()` on the C++ side) at the dot
/// the Rust PPU asserts the VBL NMI — the Phase 5.1 loop's
/// `if (take_nmi()) TriggerNMI();` forwarded across the boundary.
///
/// Phase 5.3: the whole per-dot interleave loop now lives HERE (in
/// Rust, one FFI per frame) instead of in `FCEUPPU_Loop` (three FFIs
/// per dot). Two deliberate design constraints, both learned by
/// regression in this phase:
///
/// 1. The per-dot PPU calls go through `fceux11_ppu_tick_dots_direct`
///    / `fceux11_ppu_take_nmi_direct` — byte-identical to the validated
///    FFI bodies with only the per-call REGISTRY MUTEX removed. An
///    earlier fully-fused whole-frame function (single borrow chain,
///    callbacks called from within) diverged mapper_mmc1 frame 0 under
///    LTO and was discarded — see the ppu crate ffi.rs note.
/// 2. The CPU step calls the same `fceux11_cpu_run_with_tick(cpu, 1)`
///    the C++ `Cpu::run(1)` invoked, keeping the per-dot working-copy
///    sync (C++-side IRQ mutations from mapper hooks stay bit-exact).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_run_frame_interleaved(
    ppu_state: *mut fceux11_ppu::PpuState,
    cpu_state: *mut u8,
    trigger_nmi: Option<unsafe extern "C" fn()>,
    dots: u32,
) -> i32 {
    if ppu_state.is_null() || cpu_state.is_null() {
        return -1;
    }
    let mut frame_done = 0;
    for _ in 0..dots {
        fceux11_ppu::ffi::fceux11_ppu_tick_dots_direct(ppu_state, 1);
        if fceux11_ppu::ffi::fceux11_ppu_take_nmi_direct(ppu_state) != 0 {
            if let Some(cb) = trigger_nmi {
                unsafe { cb() }
            }
        }
        // Phase 6.3.c.1: drain any pending DMC DMA stall that the C++
        // APU's `DMCDMA()` recorded via
        // `fceux11_ppu_dmc_dma_arbitration`. We mirror the C++
        // `g_cpu.timestamp_ref() += stall` semantics on the Rust CPU
        // by subtracting `stall * 16` from the `count` budget — this
        // makes the next `run_with_tick` consume that many cycles of
        // "would-have-run" budget without actually executing them,
        // exactly like the C++ APU's stalled cycles during a DMC
        // fetch. The PPU continues normally; only the CPU skips.
        //
        // When `fceux11_ppu_dmc_dma_arbitration` is not yet wired
        // (Phase 6.3.c.2 follow-up), the take returns 0 and this
        // branch is dead code.
        let stall = fceux11_ppu::ffi::fceux11_ppu_take_dmc_dma_stall(ppu_state);
        if stall > 0 {
            fceux11_core::cpu::ffi::fceux11_cpu_advance_cycles(
                cpu_state,
                -(stall as i32),
            );
        } else {
            fceux11_core::cpu::ffi::fceux11_cpu_run_with_tick(cpu_state, 1);
        }
    }
    frame_done
}

// Debug accessors (added for Phase 4 bridge window-install verification)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_register_state(
    state: *const fceux11_ppu::PpuState,
    reg: u32,
) -> u8 {
    unsafe { fceux11_ppu::ffi::fceux11_ppu_get_register_state(state, reg) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_get_v_state(state: *const fceux11_ppu::PpuState) -> u16 {
    unsafe { fceux11_ppu::ffi::fceux11_ppu_get_v_state(state) }
}
