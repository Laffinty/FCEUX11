// FCEUX11 v2.1 PPU Refactor — Phase 2: C++ bridge implementation.
//
// Compiled only when CMake option `FCEUX11_RUST_PPU=ON` (see
// `src/CMakeLists.txt`). When off, this file is excluded and the
// `ppu_rust_bridge.h` no-op inlines take over.

#ifdef FCEUX11_RUST_PPU

#include "ppu_rust_bridge.h"

#include "types.h"
#include "fceu.h"
#include "cpu.h"
#include "bus.h"           // fceu11::g_bus.aread_[] / bwrite_[]
#include "video.h"         // XBuf
#include "ppu.h"           // FCEUPPU_Init / Power / Shutdown / FCEUX_PPURead

extern "C" {
#include "rust/fceux11_rust.h"  // cbindgen output for fceux11_ppu_*
}

#include <cstring>
#include <cstdio>

namespace {

// ---------------------------------------------------------------------------
// Opaque handle + active flag
// ---------------------------------------------------------------------------

PpuState* g_ppu_state = nullptr;
bool      g_active    = false;

constexpr uint32_t kNtscCpuCyclesPerFrame = 89342;

// ---------------------------------------------------------------------------
// C++ thunks for the bus callback vtable. Forward to existing g_bus
// tables, matching the legacy FFCEUX_PPURead / FFCEUX_PPUWrite hooks.
// ---------------------------------------------------------------------------

uint8_t bridge_bus_read(uint32_t addr) {
    if (addr >= 0x10000) return 0x00;
    return fceu11::g_bus.aread_table()[addr](addr);
}

void bridge_bus_write(uint32_t addr, uint8_t value) {
    if (addr >= 0x10000) return;
    fceu11::g_bus.bwrite_table()[addr](addr, value);
}

void bridge_notify_a12_rising() {
    // Phase 3: forward to the C++ MMC3 A12 rising-edge detector. The
    // A12 detection lives in `src/ppu.cpp` (`MMC3_hb` / A12-rise
    // tracker) — for now the existing C++ new PPU's `runppu(1)` loop
    // continues to drive A12 detection from its BG-fetch hot path.
    // The Rust scheduler fires this hook at every dot-256 BG fetch
    // (visible scanlines, rendering on) so MMC3 still sees the same
    // edge rate as the C++ new PPU.
}

void bridge_notify_hblank() {
    // Phase 3: forward to the C++ mapper's `GameHBIRQHook` callback.
    // Mappers like MMC3, VRC4/6 use this to clock their scanline
    // counter. The C++ side declares `GameHBIRQHook` as a function
    // pointer in `src/ppu.h`.
    if (GameHBIRQHook) {
        GameHBIRQHook();
    }
}

void bridge_notify_hblank2() {
    // Phase 3: forward to the C++ mapper's secondary HBlank hook
    // (`GameHBIRQHook2`). VRC IRQ uses this.
    if (GameHBIRQHook2) {
        GameHBIRQHook2();
    }
}

void bridge_notify_scanline(int16_t sl) {
    // Phase 3: forward to the C++ `PPU_hook` callback with the new
    // scanline. `PPU_hook` is a function pointer the mapper can
    // install for per-scanline timing.
    if (PPU_hook) {
        PPU_hook(sl);
    }
}

void bridge_notify_vblank(bool asserted) {
    // Phase 3: the Rust state machine manages the VBL flag itself
    // (sl 241 dot 1 sets, sl 261 dot 1 clears). The C++ legacy
    // PPU_hook / mapper callbacks are notified via PPU_hook (which
    // is fired in `notify_scanline` above when transitioning into
    // sl 241 / sl 261).
}

}  // namespace

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

void ppu_rust_bridge_init() {
    if (g_ppu_state != nullptr) {
        return;  // idempotent
    }
    g_ppu_state = fceux11_ppu_create();
    if (g_ppu_state == nullptr) {
        std::fprintf(stderr, "ppu_rust_bridge: fceux11_ppu_create failed\n");
        return;
    }

    // Install the bus callback vtable.
    fceux11_ppu_bus_callbacks cb = {
        &bridge_bus_read,
        &bridge_bus_write,
        &bridge_notify_a12_rising,
        &bridge_notify_hblank,
        &bridge_notify_hblank2,
        &bridge_notify_scanline,
        &bridge_notify_vblank,
    };
    fceux11_ppu_install_bus_callbacks(g_ppu_state, &cb);

    // Install $2000-$2007 / $4014 CPU read/write routing. These
    // overwrite the existing FFCEUX_PPURead / FFCEUX_PPUWrite
    // function pointers on `g_bus` (the existing PPU handling is
    // restored when the bridge is torn down via SetReadHandler
    // reset, which the C++ side does on every PowerNES).
    for (uint32_t a = 0x2000; a <= 0x2007; a++) {
        fceu11::g_bus.set_read_handler(a, a, [](uint32_t A) -> uint8_t {
            return ppu_rust_bridge_cpu_read(A);
        });
        fceu11::g_bus.set_write_handler(a, a, [](uint32_t A, uint8_t V) {
            ppu_rust_bridge_cpu_write(A, V);
        });
    }
    // $4014 — OAM DMA. Single address, single read handler.
    fceu11::g_bus.set_read_handler(0x4014, 0x4014, [](uint32_t /*A*/) -> uint8_t {
        // $4014 is write-only; reads return open bus.
        return 0x00;
    });
    fceu11::g_bus.set_write_handler(0x4014, 0x4014, [](uint32_t A, uint8_t V) {
        ppu_rust_bridge_cpu_write(A, V);
    });

    g_active = true;
}

void ppu_rust_bridge_power() {
    if (g_ppu_state != nullptr) {
        fceux11_ppu_power(g_ppu_state);
        // After power, sync the per-state defaults.
        fceux11_ppu_set_video_system(g_ppu_state, false /* PAL=false for NTSC */);
    }
}

void ppu_rust_bridge_shutdown() {
    if (g_ppu_state != nullptr) {
        fceux11_ppu_destroy(g_ppu_state);
        g_ppu_state = nullptr;
    }
    g_active = false;
}

// ---------------------------------------------------------------------------
// Frame driver
// ---------------------------------------------------------------------------

int ppu_rust_bridge_emit_frame(int skip) {
    if (g_ppu_state == nullptr) {
        return 0;
    }
    (void)skip;  // Phase 2 doesn't differentiate skip frames.
    int rc = fceux11_ppu_emulate_frame(g_ppu_state, kNtscCpuCyclesPerFrame);
    ppu_rust_bridge_copy_framebuffer();
    return rc;
}

/// Phase 3: advance the Rust PPU by exactly one CPU cycle (3 PPU dots)
/// and fire mapper event hooks. The C++ `FCEUPPU_Loop` calls this
/// between `X6502_Run(1)` invocations to achieve per-cycle CPU/PPU
/// interleave (mirrors the C++ new PPU's `runppu(1)` + `X6502_Run(1)`
/// interleaving in `src/ppu_rendering.cpp:1711-2170`).
///
/// Returns 1 if the frame is complete (sl 261 dot 340 → wrap to
/// next frame), 0 otherwise.
int ppu_rust_bridge_emit_one_cpu_cycle() {
    if (g_ppu_state == nullptr) {
        return 0;
    }
    return fceux11_ppu_tick_one_cpu_cycle(g_ppu_state);
}

void ppu_rust_bridge_copy_framebuffer() {
    if (g_ppu_state == nullptr) return;
    uint8_t* rust_fb = fceux11_ppu_get_framebuffer(g_ppu_state);
    if (rust_fb == nullptr || XBuf == nullptr) return;
    // Visible area: 256x240 = 61440 bytes (XBuf is 256x256).
    std::memcpy(XBuf, rust_fb, 256 * 240);
}

// ---------------------------------------------------------------------------
// Bank-window setup — installed from setchr*/setntamem/setmirror*
// ---------------------------------------------------------------------------

void ppu_rust_bridge_set_chr_window(uint32_t slot, const uint8_t* ptr, uint32_t len, bool is_ram) {
    if (g_ppu_state != nullptr) {
        fceux11_ppu_set_chr_window(g_ppu_state, slot, ptr, len, is_ram);
    }
}

void ppu_rust_bridge_set_nt_window(const uint8_t* ptr, uint32_t len) {
    if (g_ppu_state != nullptr) {
        fceux11_ppu_set_nt_window(g_ppu_state, ptr, len);
    }
}

void ppu_rust_bridge_set_palette_window(const uint8_t* ptr, uint32_t len) {
    if (g_ppu_state != nullptr) {
        fceux11_ppu_set_palette_window(g_ppu_state, ptr, len);
    }
}

void ppu_rust_bridge_set_mirror_mode(uint32_t mode) {
    if (g_ppu_state != nullptr) {
        fceux11_ppu_set_mirror_mode(g_ppu_state, mode);
    }
}

// ---------------------------------------------------------------------------
// CPU-side $2000-$2007 / $4014 routing
// ---------------------------------------------------------------------------

uint8_t ppu_rust_bridge_cpu_read(uint32_t addr) {
    if (g_ppu_state == nullptr) {
        return 0;
    }
    return fceux11_ppu_cpu_read(g_ppu_state, static_cast<uint16_t>(addr));
}

void ppu_rust_bridge_cpu_write(uint32_t addr, uint8_t value) {
    if (g_ppu_state == nullptr) {
        return;
    }
    fceux11_ppu_cpu_write(g_ppu_state, static_cast<uint16_t>(addr), value);
}

bool ppu_rust_bridge_active() {
    return g_active && g_ppu_state != nullptr;
}

#endif // FCEUX11_RUST_PPU