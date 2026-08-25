// FCEUX11 v2.1 PPU Refactor — Phase 2: C++ bridge to Rust PPU.
//
// When `FCEUX11_RUST_PPU` is enabled at compile time, this header
// declares the install/shutdown hooks and the in-loop dispatch entry
// points. The actual implementation lives in `ppu_rust_bridge.cpp`,
// which is only compiled into `fceux11_core` when the CMake option
// `FCEUX11_RUST_PPU=ON`.
//
// The bridge is intentionally thin: it
//   1. Owns the opaque `fceux11_ppu_state` handle (allocated in
//      `FCEUPPU_Init`, freed in `FCEUPPU_Shutdown`).
//   2. Installs a C++ thunk vtable that forwards PPU bus reads /
//      writes to `g_bus.aread_[]` / `g_bus.bwrite_[]` (the existing
//      dispatch table in `src/bus.cpp`).
//   3. Hooks the `$2000-$2007` CPU address range so that reads /
//      writes route through `fceux11_ppu_cpu_read` / `cpu_write`.
//   4. Replaces `FCEUPPU_Loop` body when `FCEUX11_RUST_PPU=ON` so
//      the Rust PPU drives the frame timing / rendering.
//
// Off by default; the C++ PPU path remains bit-identical to Phase 0/1
// when `FCEUX11_RUST_PPU` is not defined.

#ifndef FCEU11_PPU_RUST_BRIDGE_H
#define FCEU11_PPU_RUST_BRIDGE_H

#include <cstdint>

#ifdef FCEUX11_RUST_PPU

// Opaque handle to the Rust-side PpuState. The bridge owns the lifetime.
extern "C" {
    struct PpuState;
}

// Lifecycle — called from FCEUPPU_Init / FCEUPPU_Shutdown / FCEUPPU_Power.
void ppu_rust_bridge_init();
void ppu_rust_bridge_power();
void ppu_rust_bridge_shutdown();

// Frame driver — replaces FCEUPPU_Loop's inner loop when Rust PPU is
// the active engine. Returns the same int as FCEUPPU_Loop (0).
int ppu_rust_bridge_emit_frame(int skip);

// Bank-window setup — called from setchr*/setntamem paths.
void ppu_rust_bridge_set_chr_window(uint32_t slot, const uint8_t* ptr, uint32_t len, bool is_ram);
void ppu_rust_bridge_set_nt_window(const uint8_t* ptr, uint32_t len);
void ppu_rust_bridge_set_palette_window(const uint8_t* ptr, uint32_t len);
void ppu_rust_bridge_set_mirror_mode(uint32_t mode);

// CPU bus routing — registered in FCEU_ResetHooks, called from
// existing FFCEUX_PPURead/FFCEUX_PPUWrite slots.
uint8_t ppu_rust_bridge_cpu_read(uint32_t addr);
void    ppu_rust_bridge_cpu_write(uint32_t addr, uint8_t value);

// Frame buffer copy — called at the end of the Rust frame emission to
// mirror the Rust-side 256x256 buffer into C++ XBuf.
void ppu_rust_bridge_copy_framebuffer();

// Engine flag — set by ppu_rust_bridge_init when the option is on and
// the bridge successfully initialised. FCEUPPU_Loop reads this to
// decide whether to delegate to the Rust path.
bool ppu_rust_bridge_active();

#else  // !FCEUX11_RUST_PPU

// When the option is off, the bridge functions are no-ops returning
// disabled / passthrough values.
inline void ppu_rust_bridge_init() {}
inline void ppu_rust_bridge_power() {}
inline void ppu_rust_bridge_shutdown() {}
inline int  ppu_rust_bridge_emit_frame(int /*skip*/) { return 0; }
inline void ppu_rust_bridge_set_chr_window(uint32_t /*slot*/, const uint8_t* /*ptr*/, uint32_t /*len*/, bool /*is_ram*/) {}
inline void ppu_rust_bridge_set_nt_window(const uint8_t* /*ptr*/, uint32_t /*len*/) {}
inline void ppu_rust_bridge_set_palette_window(const uint8_t* /*ptr*/, uint32_t /*len*/) {}
inline void ppu_rust_bridge_set_mirror_mode(uint32_t /*mode*/) {}
inline uint8_t ppu_rust_bridge_cpu_read(uint32_t /*addr*/) { return 0; }
inline void     ppu_rust_bridge_cpu_write(uint32_t /*addr*/, uint8_t /*value*/) {}
inline void ppu_rust_bridge_copy_framebuffer() {}
inline bool ppu_rust_bridge_active() { return false; }

#endif // FCEUX11_RUST_PPU

#endif // FCEU11_PPU_RUST_BRIDGE_H