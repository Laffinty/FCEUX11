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

// Phase 5.1: canonical PPU-space accessors (src/ppu.cpp / ppu.h). The
// Rust PPU's bus callbacks route through these so nametable RAM /
// CHR / palette semantics (RAM gating, palette mirroring, mapper
// bank state) match the C++ engine exactly — and so PPU-space
// addresses never re-enter this bridge's own CPU-space register
// handlers (see bridge_bus_read).
extern uint8_t (*FFCEUX_PPURead)(uint32_t);
// Palette RAM (defined in ppu.cpp); the palette window mirrors it.
extern std::array<uint8_t, 0x20> PALRAM;

namespace {

// ---------------------------------------------------------------------------
// Opaque handle + active flag
// ---------------------------------------------------------------------------

PpuState* g_ppu_state = nullptr;
bool      g_active    = false;

constexpr uint32_t kNtscCpuCyclesPerFrame = 89342;

// CHR/NT window backing stores (namespace-scope so both
// `bridge_refresh_windows` and the power path share them; the Rust
// side holds these pointers, so they must outlive any single call).
uint8_t g_chr_window[8192];
uint8_t g_nt_window[4096];

// Phase 5.1: (re)install the CHR/NT/Palette windows from the live C++
// bus state. Called from `ppu_rust_bridge_power` and after every
// CPU-space $2007 write: the Rust renderer reads these COPIES, so
// without the refresh, PPU-space writes (nametable text, palette
// changes, CHR-RAM uploads) would never become visible to the Rust
// renderer — rom_regression's nestest frames diverged from frame 3
// (the first display-write frame) for exactly this reason.
//
// CHR: build a contiguous 8 KiB window by reading through
// g_bus.vpage[p] for each 1 KiB page $0000-$1FFF. For NROM all 8
// pages point to the same 8 KiB buffer; for mappers with CHR
// banking, each page can resolve to a different bank.
void bridge_refresh_windows() {
    auto& vpage = fceu11::g_bus.vpage();
    for (uint32_t p = 0; p < 8; ++p) {
        // g_bus.set_vpage stores `ptr - addr` so a read of
        // `vpage_[idx] + addr` recovers `ptr`. We undo that here.
        const uint8_t* page_base = vpage[p] + (p << 10);
        for (uint32_t i = 0; i < 1024; ++i) {
            g_chr_window[p * 1024 + i] = page_base[i];
        }
    }
    fceux11_ppu_set_chr_window(g_ppu_state, 0, g_chr_window, 8192, false);
    // NT: 4 KiB window covering vnapage[0..4]. The C++ vnapage table
    // is set up by the mapper's Power handler via Ppu::set_mirror_page
    // / set_mirror_mode; the Rust renderer's `map_nametable_addr` has
    // a four-screen case that uses pages 2/3 directly, so all four
    // pages must be present. (A 2 KiB buffer here was also the
    // static-buffer-overflow root cause fixed in this phase: the old
    // 4-page loop wrote 4096 bytes into 2048 on every Power.)
    auto& vnapage = fceu11::g_ppu.vnapage();
    for (uint32_t p = 0; p < 4; ++p) {
        const uint8_t* page_base = vnapage[p];
        if (page_base == nullptr) {
            // Mapper hasn't set this page; fall back to NTARAM
            // base. This happens for mappers that only configure
            // pages 0/1 and rely on the default for 2/3.
            page_base = &fceu11::g_ppu.ntaram()[0];
        }
        for (uint32_t i = 0; i < 1024; ++i) {
            g_nt_window[p * 1024 + i] = page_base[i];
        }
    }
    fceux11_ppu_set_nt_window(g_ppu_state, g_nt_window, 4096);
    // Palette: 32 bytes from PALRAM (declared at file scope above).
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


// ---------------------------------------------------------------------------
// C++ thunks for the bus callback vtable. Forward to existing g_bus
// tables, matching the legacy FFCEUX_PPURead / FFCEUX_PPUWrite hooks.
// ---------------------------------------------------------------------------

uint8_t bridge_bus_read(uint32_t addr) {
    // Phase 5.1: `addr` is a PPU-space ($0000-$3FFF) byte address. It
    // MUST go through the C++ PPU-space accessors (FFCEUX_PPURead /
    // FFCEUX_PPURead_Default), NOT the CPU-space dispatch table:
    // `g_bus.aread_table()[$2000-$2007]` now holds THIS bridge's own
    // CPU-register handlers (installed by bridge_init/bridge_power),
    // so a PPU-space read/write landing in $2000-$2007 would
    // re-enter `fceux11_ppu_cpu_read/write` → `read_data`/`write_data`
    // → bus.read/write → back here — unbounded recursion (observed as
    // a STATUS_STACK_OVERFLOW once nestest's $2007 nametable writes
    // land on v == $2007).
    addr &= 0x3FFF;
    if (FFCEUX_PPURead) return FFCEUX_PPURead(addr);
    return FFCEUX_PPURead_Default(addr);
}

void bridge_bus_write(uint32_t addr, uint8_t value) {
    if (addr >= 0x4000) {
        // Defensive: the Rust side never emits out-of-PPU addresses.
        return;
    }
    // Phase 5.1: same re-entrancy argument as bridge_bus_read — route
    // PPU-space writes through the C++ PPU-space accessor, never the
    // CPU-space table (whose $2000-$2007 slots are our own handlers).
    if (FFCEUX_PPUWrite) {
        FFCEUX_PPUWrite(addr, value);
    }
    // Else: hooks not yet installed (pre-Power). Old engines dropped
    // $2007 writes then too.
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
    //
    // Phase 5.1: also keep the legacy `::scanline` view live. The
    // per-cycle interleave fires this thunk on every true scanline
    // transition mid-frame, so consumers reading `g_cpu.scanline()`
    // (e.g. MMC3_hb_PALStarWarsHack, savestate CPU views) observe the
    // current line instead of a stale frame-end snapshot.
    fceu11::cpu_instance().set_scanline(sl);
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

        // Phase 4/5.1: install CHR/NT/Palette windows from the C++ bus
        // state. The mapper's Power handler has already populated
        // `g_bus.vpage[]` and `g_ppu.vnapage()` by this point. The
        // copy logic lives in `bridge_refresh_windows` (shared with
        // the post-$2007-write refresh path).
        bridge_refresh_windows();
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
    // Phase 5.1+ ONLY: use the per-cycle interleave driven from
    // `FCEUPPU_Loop` (`ppu_rust_bridge_emit_one_cpu_cycle` +
    // `fceu11::cpu_instance().run(3)`). This batch model — run the
    // CPU for the full frame, then advance the PPU — is the old
    // cold-start stub kept for the pre-per-cycle wiring. It is not
    // called by any active v2.1 call site: under it the CPU runs with
    // the PPU frozen at the frame boundary, mapper hooks fire one
    // frame late, and `fceux11_ppu_emulate_frame(89342)` advances 3
    // PPU frames (its `n_cycles` argument is a CPU-cycle budget that
    // the FFI multiplies by 3) while `cpu.run(89342)` advances 1.
    fceu11::cpu_instance().run(static_cast<int32_t>(kNtscCpuCyclesPerFrame));
    int rc = fceux11_ppu_emulate_frame(g_ppu_state, kNtscCpuCyclesPerFrame);
    ppu_rust_bridge_copy_framebuffer();
    return rc;
}

/// Phase 5.1: advance the Rust PPU by exactly one CPU cycle's worth of
/// PPU dots (3) — rendering visible scanlines at their start, firing
/// mapper event hooks, and pumping the OAM DMA. `FCEUPPU_Loop`'s
/// per-cycle interleave calls this between
/// `fceu11::cpu_instance().run(3)` invocations (3 dot units = 1 CPU
/// cycle in the `Cpu::run` dot-unit convention), mirroring the C++
/// new PPU's `runppu(1)` + `X6502_Run(1)` interleaving.
///
/// Returns 1 if the frame is complete (sl 260 dot 340 → wrap to
/// next frame), 0 otherwise.
int ppu_rust_bridge_emit_one_cpu_cycle() {
    if (g_ppu_state == nullptr) {
        return 0;
    }
    return fceux11_ppu_tick_dots(g_ppu_state, 3);
}

/// Phase 5.1: advance the Rust PPU by an arbitrary dot count with the
/// same per-dot pipeline as `emit_one_cpu_cycle`. The FCEUPPU_Loop
/// per-dot interleave calls this with 1 for each of the 89342 dots of
/// an NTSC frame.
void ppu_rust_bridge_advance_ppu_dots(uint32_t dots) {
    if (g_ppu_state == nullptr || dots == 0) {
        return;
    }
    fceux11_ppu_tick_dots(g_ppu_state, dots);
}

/// Phase 5.1: take-and-clear the Rust PPU's NMI latch. The per-cycle
/// interleave polls this every dot and pulses the CPU NMI line via
/// `TriggerNMI()` when set — the same latch path the C++ engines use
/// for the VBL NMI (including the one-instruction "fresh" deferral).
int ppu_rust_bridge_take_nmi() {
    if (g_ppu_state == nullptr) {
        return 0;
    }
    return fceux11_ppu_take_nmi_pending(g_ppu_state);
}

// Phase 5.3: NMI callback handed to the in-Rust interleave loop. Fires
// at the exact dot the Rust PPU asserts the VBL NMI — the same point
// the Phase 5.1 C++ loop pulsed TriggerNMI() from.
static void bridge_trigger_nmi() {
    TriggerNMI();
}

int ppu_rust_bridge_run_frame_interleaved(uint32_t dots) {
    if (g_ppu_state == nullptr) {
        return -1;
    }
    // The in-Rust loop bypasses the C++ facade's lazy hook install
    // (Cpu::run normally does this before its first FFI). `run(0)`
    // executes nothing (fceux11_cpu_run_with_tick returns immediately
    // for a zero budget) but installs the bus + tick thunks.
    fceu11::cpu_instance().run(0);
    return fceux11_run_frame_interleaved(
        g_ppu_state,
        reinterpret_cast<uint8_t*>(&fceu11::cpu_instance().native_layout()),
        &bridge_trigger_nmi,
        dots);
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
    // NOTE (Phase 5.3 open item, DO NOT "fix" without a dedicated gate):
    // the C++ A2002 handler (src/ppu.cpp:614-632) adds two newppu-only
    // behaviors the bridge path lacks — (1) $2002 read at 1 dot before
    // the VBL set suppresses the flag set + NMI for the frame; (2) read
    // at/1 dot after the set cancels the pending VBL NMI
    // (X6502_IRQEnd(FCEU_IQNMI)). A direct port was attempted in 5.3 and
    // REGRESSED mapper_mmc1 frame 0 (rom_regression) — the windows
    // interact with ppudead frames and the Rust flag set point (dot 1
    // vs C++ cy 0). blargg 05-nmi_timing / 06-suppression parity is
    // deferred to Phase 6 with a dedicated test gate.
}

void ppu_rust_bridge_cpu_write(uint32_t addr, uint8_t value) {
    if (g_ppu_state == nullptr) {
        return;
    }
    // NOTE (Phase 5.3 open item): the C++ B2000 (src/ppu.cpp:961) also
    // fires TriggerNMI2() on a rising $2000 NMI-enable edge while the
    // VBL flag is set. A direct port here improved blargg 05-nmi_timing
    // odd offsets but the even-offset cases still never fire — the
    // remaining gap is the missing A2002 suppression/cancellation
    // (see ppu_rust_bridge_cpu_read) — so the edge is deferred to the
    // same Phase 6 NMI-timing gate rather than partially ported.
    fceux11_ppu_cpu_write(g_ppu_state, static_cast<uint16_t>(addr), value);
    // Phase 5.1: a $2007 write landed in CHR/NT/palette space (the
    // Rust `write_data` routed it to the C++ authoritative arrays via
    // bridge_bus_write). Refresh the renderer's window copies so the
    // next render observes the write. Without this, the Rust renderer
    // reads its at-Power snapshot forever and post-boot display
    // writes are invisible (rom_regression nestest frames 3+).
    if (addr == 0x2007) {
        bridge_refresh_windows();
    }
}

bool ppu_rust_bridge_active() {
    return g_active && g_ppu_state != nullptr;
}

#endif // FCEUX11_RUST_PPU