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
#include "ppu_class.h"     // fceu11::g_ppu.ntaram() / vnapage()

extern "C" {
#include "rust/fceux11_rust.h"  // cbindgen output for fceux11_ppu_*
}

#include <cstring>
#include <cstdio>
#include <array>

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

        // Phase 4: re-install $2000-$2007 / $4014 handlers. FCEUPPU_Power
        // (called before the mapper Power via ppu.cpp:1208) overwrote
        // these with the C++ PPU's A200x/A2002/etc. handlers. Since
        // the bridge needs to route CPU writes to the Rust PPU, we
        // re-install the bridge thunks here AFTER the C++ PPU's
        // power setup. (Phase 3 called bridge_init before FCEUPPU_Power
        // and the C++ handlers took precedence — fixed in Phase 4.)
        for (uint32_t a = 0x2000; a <= 0x2007; a++) {
            fceu11::g_bus.set_read_handler(a, a, [](uint32_t A) -> uint8_t {
                return ppu_rust_bridge_cpu_read(A);
            });
            fceu11::g_bus.set_write_handler(a, a, [](uint32_t A, uint8_t V) {
                ppu_rust_bridge_cpu_write(A, V);
            });
        }
        fceu11::g_bus.set_read_handler(0x4014, 0x4014, [](uint32_t /*A*/) -> uint8_t {
            return 0x00;
        });
        fceu11::g_bus.set_write_handler(0x4014, 0x4014, [](uint32_t A, uint8_t V) {
            ppu_rust_bridge_cpu_write(A, V);
        });

        // Phase 4: install CHR/NT/Palette windows from the C++ bus
        // state. The mapper's Power handler has already populated
        // `g_bus.vpage[]` and `g_ppu.vnapage()` by this point.
        //
        // CHR: build a contiguous 8 KiB window by reading through
        // g_bus.vpage[p] for each 1 KiB page $0000-$1FFF. For NROM
        // all 8 pages point to the same 8 KiB buffer; for mappers
        // with CHR banking, each page can resolve to a different
        // bank. We do the resolution eagerly (one read per byte)
        // so the Rust PPU's hot path can do direct array indexing
        // without going through the FFI bus callback. This is the
        // "CHR window cache" the v2.1 plan §6.5 calls for.
        auto& vpage = fceu11::g_bus.vpage();
        static uint8_t chr_window[8192];
        for (uint32_t p = 0; p < 8; ++p) {
            // g_bus.set_vpage stores `ptr - addr` so a read of
            // `vpage_[idx] + addr` recovers `ptr`. We undo that here.
            const uint8_t* page_base = vpage[p] + (p << 10);
            for (uint32_t i = 0; i < 1024; ++i) {
                chr_window[p * 1024 + i] = page_base[i];
            }
        }
        fceux11_ppu_set_chr_window(g_ppu_state, 0, chr_window, 8192, false);
        // NT: 2 KiB window covering vnapage[0] (page 0) and
        // vnapage[1] (page 1). The C++ vnapage table is set up by
        // the mapper's Power handler via Ppu::set_mirror_page /
        // set_mirror_mode. For four-screen or 4-page mappers,
        // vnapage[2] and vnapage[3] are also used; we copy them too
        // so the Rust renderer's `map_nametable_addr` works for
        // all 5 mirror modes (it has a four-screen case that uses
        // page 2/3 directly).
        auto& vnapage = fceu11::g_ppu.vnapage();
        static uint8_t nt_window[2048];
        for (uint32_t p = 0; p < 4; ++p) {
            const uint8_t* page_base = vnapage[p];
            if (page_base == nullptr) {
                // Mapper hasn't set this page; fall back to NTARAM
                // base. This happens for mappers that only configure
                // pages 0/1 and rely on the default for 2/3.
                page_base = &fceu11::g_ppu.ntaram()[0];
            }
            for (uint32_t i = 0; i < 1024; ++i) {
                nt_window[p * 1024 + i] = page_base[i];
            }
        }
        fceux11_ppu_set_nt_window(g_ppu_state, nt_window, 2048);
        // Palette: 32 bytes from PALRAM (declared in ppu.cpp).
        extern std::array<uint8_t, 0x20> PALRAM;
        fceux11_ppu_set_palette_window(g_ppu_state, PALRAM.data(), static_cast<uint32_t>(PALRAM.size()));
        // Mirror mode detection: compare the 4 vnapage pointers to
        // figure out which FCEUX mirror mode the mapper selected.
        // This avoids the need to thread the mode byte through Bus
        // and lets us auto-detect from the side-effect of
        // Ppu::set_mirror_mode.
        const uint8_t* nt_base = &fceu11::g_ppu.ntaram()[0];
        auto is_p0 = vnapage[0] == nt_base;
        auto is_p1 = vnapage[1] == nt_base + 0x400;
        auto is_p2 = vnapage[2] == nt_base;
        auto is_p3 = vnapage[3] == nt_base + 0x400;
        uint32_t mode = 0;
        if (is_p0 && is_p1 && !is_p2 && !is_p3) {
            mode = 0;  // horizontal (vnapage[0]=vnapage[1]=base, [2]=[3]=base+0x400)
        } else if (is_p0 && !is_p1 && is_p2 && !is_p3) {
            mode = 1;  // vertical (vnapage[0]=vnapage[2]=base, [1]=[3]=base+0x400)
        } else if (vnapage[0] == nt_base && vnapage[1] == nt_base && vnapage[2] == nt_base && vnapage[3] == nt_base) {
            mode = 2;  // single-low
        } else if (vnapage[0] == nt_base + 0x400 && vnapage[1] == nt_base + 0x400) {
            mode = 3;  // single-high
        } else {
            mode = 4;  // four-screen or other
        }
        fceux11_ppu_set_mirror_mode(g_ppu_state, mode);
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
    // Phase 4: drive the CPU and the Rust PPU. The C++ Emulate()
    // function only calls FCEUPPU_Loop, which delegates to the
    // bridge — the CPU is NOT driven by anyone else. So we must
    // run the CPU here. The "per-cycle interleave" documented in
    // Phase 3 §10.2 is a follow-on; for now we use the simpler
    // "run CPU for the full frame, then advance PPU" model. This
    // is enough for NROM bit-exact because the NROM mapper has no
    // per-cycle PPU side effects beyond $2000-$2007 writes.
    fceu11::cpu_instance().run(static_cast<int32_t>(kNtscCpuCyclesPerFrame));
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