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

/// Attach an input device to a standard controller port (D9 / kgmqa-107).
///
/// `esi_type` is the SI_* integer value from git.h — e.g. SI_NONE = 0,
/// SI_SNES_MOUSE = 8. Call after kagami_bridge_init() and before
/// kagami_bridge_load_rom(); the setting persists across loads because
/// nothing in the core load path overrides the port driver (only the Qt
/// driver would).
///
/// The SI_SNES_MOUSE branch of SetInputStuff does not touch GameInfo, so
/// this is safe before a ROM is loaded.
///
/// Returns 0 on success, -1 on a bad port (must be 0 or 1), -2 when the
/// bridge is not initialised.
int kagami_bridge_set_input(int port, int esi_type);

/// Famicom microphone injection (v2.0.4 V6 / kgmqa-108).
///
/// The microphone is a second-controller (P2) peripheral: its report is
/// bit2 of $4016 as a LEVEL read (pressed=1, released=0 — NESdev wiki
/// "Standard controller" Famicom section `xxxx xMES`; Mesen2
/// NesController.h:116-118), and it is NOT part of the shift sequence nor
/// present on $4017. The preserved key mapping routes the P2 Start button
/// into the microphone (`replaceP2StartWithMicrophone`), so "pressing the
/// mic" here means setting the P2 Start bit in a persistent port-1 gamepad
/// buffer — the exact path the Qt driver feeds every frame (UpdateGP
/// shifts byte1 into joy[1]; JPRead reads joy[1]&8).
///
/// `port` must be 1 (P2 / second pad only — Mesen2 gates the mic on
/// `port == 1 && type == FamicomControllerP2`).
///
/// `on != 0`: enables the mic protocol (`replaceP2StartWithMicrophone`,
/// persists for the session) and presses the mic (P2 Start bit set).
/// `on == 0`: releases the mic (bit cleared). The protocol enable is the
/// key mapping, not the per-read level, so it intentionally persists —
/// toggling on/off is what produces the 0<->1 report transitions Famicom
/// games track.
///
/// Attaches SI_GAMEPAD to port 1 with a persistent bridge-owned buffer
/// (the kagami_bridge_init() default is SI_NONE/nullptr). A later
/// kagami_bridge_set_input(1, ...) re-points the port and detaches the
/// mic buffer — call this function after any set_input on port 1.
///
/// Returns 0 on success, -1 on a bad port, -2 when the bridge is not
/// initialised.
int kagami_bridge_set_famicom_mic(int port, int on);

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

#ifdef __cplusplus
}
#endif
