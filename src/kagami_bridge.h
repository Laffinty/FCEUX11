// KagamiQA — C ABI bridge for in-process SutAdapter.
//
// Exposes FCEUX11 core functions through a minimal, stable C ABI so the
// KagamiQA Rust crate can drive the emulator frame-by-frame without
// spawning a subprocess.  This is the foundation for P5 (runppu 重批)
// where per-frame oracle probes are needed.
//
// All functions are extern "C" and safe to call from Rust FFI.

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Initialise the emulator in headless mode (null driver, no Qt).
/// Must be called once before any other bridge function.
/// Returns 0 on success, non-zero on failure.
int kagami_bridge_init(void);

/// Load a ROM image.  Must be called after init.
/// Returns 0 on success, non-zero on failure.
int kagami_bridge_load_rom(const char *path);

/// Advance the emulator by one frame.
/// Returns 0 on success, non-zero on failure.
int kagami_bridge_emulate_frame(void);

/// Read one byte from CPU address space (0x0000–0xFFFF).
uint8_t kagami_bridge_read_byte(uint16_t addr);

/// Read one byte from PPU address space (0x0000–0x3FFF).
uint8_t kagami_bridge_read_ppu(uint16_t addr);

/// Reset the emulator to post-power-on state.
int kagami_bridge_reset(void);

/// Full teardown + re-init (Kill then Initialize), leaving no ROM loaded.
/// Mirrors the C++ savestate harness's per-ROM Initialize/Kill cycle.
int kagami_bridge_full_reset(void);

/// Tear down the emulator and free resources.
void kagami_bridge_kill(void);

/// Enable the new PPU (newppu = 1).  Must be called after init and
/// before load_rom for the new-PPU rendering path.
void kagami_bridge_set_newppu(int on);

/// Copy the first `len` bytes of the current XBuf (NES video buffer)
/// into `dst`. XBuf is laid out as 256x256 bytes; the visible 256x240
/// region is the first `256 * 240 = 61440` bytes — the same slice the
/// C++ `rom_regression_test.cpp` CRC32s.
///
/// Returns 0 on success, non-zero if XBuf is NULL (e.g. before the
/// first frame or after close_game).
///
/// # Safety
/// `dst` must point to at least `len` writable bytes.
int kagami_bridge_extract_frame_buffer(uint8_t *dst, uint32_t len);

/// Serialise the current emulator state into a savestate and write
/// the first `cap` bytes into `dst`. Returns the total size of the
/// savestate in `written_out` (always set, even on truncation).
///
/// On truncation (`written_out > cap`), the function still writes
/// `cap` bytes but reports the actual size via `written_out` so the
/// caller can retry with a larger buffer.
///
/// Returns 0 on success, non-zero if the savestate cannot be
/// produced (no ROM loaded, etc.).
///
/// `compression_level` matches `FCEUSS_SaveMS`'s argument; 0 disables
/// compression (used by the C++ `savestate_regression_test.cpp`).
///
/// # Safety
/// `dst` must point to at least `cap` writable bytes, or be NULL if
/// `cap == 0`. `written_out` must point to a valid writable `uint32_t`.
int kagami_bridge_save_state(uint8_t *dst, uint32_t cap,
                             uint32_t *written_out,
                             int compression_level);

/// Serialise the current cart's mapper state (Cart::save_mapper_state())
/// and write the first `cap` bytes into `dst`. Returns the total size in
/// `written_out` (always set, even on truncation).
///
/// Returns 0 on success, non-zero if no cart is loaded or the mapper
/// state cannot be produced. Mirrors the C++ `mapper_byte_diff_test.cpp`
/// capture step (`fceu11::g_cart->save_mapper_state()`).
///
/// # Safety
/// `dst` must point to at least `cap` writable bytes, or be NULL if
/// `cap == 0`. `written_out` must point to a valid writable `uint32_t`.
int kagami_bridge_save_mapper_state(uint8_t *dst, uint32_t cap,
                                    uint32_t *written_out);

/// Read the 6502 PC from the singleton `fceu11::Cpu`. Used by the
/// Phase 4.5 cycle-drift diagnostic harness
/// (`kagami-qa-cycle-trace`) to record per-call PC values for
/// cross-language diffing (see
/// `docs/plans/phase4-dispatch-budget-fix-2026-08-19.md` §5.2).
///
/// Returns the program counter as a 16-bit value (0x0000–0xFFFF). The
/// value is undefined if no CPU is initialised.
uint16_t kagami_bridge_get_cpu_pc(void);

/// Read the C++ `X6502::IRQlow` pending-interrupt bitmask. Phase 4.5
/// cycle-drift fix: the Rust CPU synchronises its `state.regs.irq_low`
/// with this host value around every dispatch boundary (via
/// `Bus::sync_irq_from_host` / `sync_irq_to_host`), because IRQs
/// asserted from the C++ side during a Rust call (mapper hooks, APU
/// frame-counter IRQ) mutate this blob and are otherwise invisible to
/// the Rust snapshot taken at call start.
uint32_t kagami_bridge_get_cpu_irq_low(void);

/// Write the C++ `X6502::IRQlow` pending-interrupt bitmask (the
/// counterpart of `kagami_bridge_get_cpu_irq_low`). Pushes back bits
/// the Rust dispatch consumed so they are not re-asserted next call.
void kagami_bridge_set_cpu_irq_low(uint32_t v);

// ---------------------------------------------------------------------------
// Cycle-trace hooks — Phase 4.5 cycle-drift diagnostic.
//
// Activated by setting `FCEUX11_CYCLE_LOG=<path>` before the first
// `kagami_bridge_init`. The CSV row format is documented at the
// `CycleTraceSink` definition in `src/kagami_bridge.cpp`.
//
// `set_frame` is called by the harness binary BEFORE each
// `kagami_bridge_emulate_frame` so per-frame boundaries appear in
// the CSV. `record` is invoked from `Cpu::run` (and possibly from
// `X6502_RunDebug` under Rust CPU=OFF) to capture every per-call
// cycle accounting delta.
// ---------------------------------------------------------------------------
void kagami_bridge_cycle_trace_set_frame(uint32_t frame_idx);

uint32_t kagami_bridge_cycle_trace_current_frame(void);

void kagami_bridge_cycle_trace_record(uint32_t cycles_arg,
                                      uint16_t pc_after,
                                      uint32_t cumulative_count,
                                      uint32_t irq_low);

uint64_t kagami_bridge_cycle_trace_row_count(void);

#ifdef __cplusplus
}
#endif
