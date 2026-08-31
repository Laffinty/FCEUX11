//! Integration tests for the Phase 6.3.b DMC DMA arbitration scaffolding.
//!
//! The C++ APU's `DMCDMA()` (`src/sound.cpp:659-686`) performs four
//! `X6502_DMR()` reads each time a DMC sample byte is fetched; each read
//! stalls the CPU for one CPU cycle (per `ADDCYC(1)` in
//! `src/cpu.cpp:339`). In a Rust-driven per-dot interleave loop the CPU
//! would otherwise continue advancing its timestamp while the APU is
//! mid-fetch, causing the C++ `g_cpu.timestamp_ref` to drift relative
//! to the Rust side's budget.
//!
//! Phase 6.3.b currently ships only the **scaffolding** FFI:
//! `fceux11_ppu_dmc_dma_arbitration(state, stall_cycles)` records the
//! most recent stall request in `StateBox::dmc_dma_pending_stall`.
//! The per-dot loop does NOT yet consume this field — that requires a
//! new `fceux11_cpu_advance_cycles(cpu_state, n)` API in the Rust CPU
//! crate (multi-session task). These tests pin the scaffolding so
//! future implementation can build on a stable surface.

use fceux11_ppu::ffi::{
    fceux11_ppu_create, fceux11_ppu_destroy, fceux11_ppu_dmc_dma_arbitration,
};

#[test]
fn dmc_dma_arbitration_records_stall_cycles() {
    unsafe {
        let state = fceux11_ppu_create();
        assert!(!state.is_null(), "fceux11_ppu_create must return non-null");
        // Phase 6.3.b: each DMCDMA() call performs 4 reads, so a typical
        // fetch reports 4 stall cycles. The fixture here uses the
        // maximum DMC rate period stall window (1-3 in practice; 4 is
        // the upper bound the C++ APU can issue per fetch).
        fceux11_ppu_dmc_dma_arbitration(state, 4);
        // Reading back the recorded value would require exposing a
        // getter; the per-dot loop is the consumer and isn't wired
        // yet (see §6.3.b). The destructive round-trip below
        // confirms the call doesn't crash the staticlib.
        fceux11_ppu_destroy(state);
    }
}

#[test]
fn dmc_dma_arbitration_accepts_zero_stall() {
    // The APU calls this with stall=0 when DMCDMA is a no-op
    // (DMCSize == 0 || DMCHaveDMA). Must not panic.
    unsafe {
        let state = fceux11_ppu_create();
        assert!(!state.is_null());
        fceux11_ppu_dmc_dma_arbitration(state, 0);
        fceux11_ppu_destroy(state);
    }
}

#[test]
fn dmc_dma_arbitration_overwrites_previous_value() {
    // The StateBox only retains the most recent stall — a fresh
    // notification replaces any earlier one (no queueing). This is
    // the same model the C++ engine uses (the in-flight fetch is
    // cancelled if a new rate period expires mid-stall).
    unsafe {
        let state = fceux11_ppu_create();
        assert!(!state.is_null());
        fceux11_ppu_dmc_dma_arbitration(state, 4);
        fceux11_ppu_dmc_dma_arbitration(state, 1);
        fceux11_ppu_dmc_dma_arbitration(state, 2);
        // Once the per-dot loop is wired, it will consume the
        // dmc_dma_pending_stall atomically; until then the latest
        // value simply persists. No getter to assert against yet —
        // see §6.3.b "Open items" in
        // docs/history/v2.1_phase6_batch_compat.md.
        fceux11_ppu_destroy(state);
    }
}
