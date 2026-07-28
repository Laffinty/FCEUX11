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

/// Tear down the emulator and free resources.
void kagami_bridge_kill(void);

/// Enable the new PPU (newppu = 1).  Must be called after init and
/// before load_rom for the new-PPU rendering path.
void kagami_bridge_set_newppu(int on);

#ifdef __cplusplus
}
#endif
