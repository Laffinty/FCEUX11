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

// Phase 6.4: page-base pointers captured by the last window refresh.
// The Rust renderer reads the window backing stores through the
// pointers installed at power (which never change), so a mid-frame
// mapper bank switch only needs the CONTENTS re-copied — no FFI
// re-install. Comparing these pointers is a cheap dirty test
// (12 compares) run at every scanline boundary.
const uint8_t* g_chr_page_base[8] = {};
const uint8_t* g_nt_page_base[4] = {};
// Mirror mode last pushed to the Rust side; a mismatch with the
// pattern derived from vnapage means the push is stale and is
// re-done at the next frame boundary (see
// bridge_push_mirror_mode_if_dirty).
uint32_t g_mirror_mode_pushed = 0;
bool     g_mirror_mode_pushed_valid = false;

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
// Phase 6.4: mirror-mode derivation from the vnapage pointer pattern
// (defined below bridge_refresh_windows; forward-declared because the
// full install records the pushed mode alongside the copy).
uint32_t bridge_derive_mirror_mode();

void bridge_refresh_windows() {
    auto& vpage = fceu11::g_bus.vpage();
    const uint8_t* nt_base = &fceu11::g_ppu.ntaram()[0];
    for (uint32_t p = 0; p < 8; ++p) {
        // g_bus.set_vpage stores `ptr - addr` so a read of
        // `vpage_[idx] + addr` recovers `ptr`. We undo that here.
        const uint8_t* page_base = vpage[p] + (p << 10);
        g_chr_page_base[p] = page_base;
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
            page_base = nt_base;
        }
        g_nt_page_base[p] = page_base;
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
    const uint32_t mode = bridge_derive_mirror_mode();
    fceux11_ppu_set_mirror_mode(g_ppu_state, mode);
    g_mirror_mode_pushed = mode;
    g_mirror_mode_pushed_valid = true;
}

// Phase 6.4: mirror-mode derivation from the vnapage pointer pattern.
// Extracted verbatim from bridge_refresh_windows so both the full
// install and the deferred re-push (frame boundary) share it.
uint32_t bridge_derive_mirror_mode() {
    auto& vnapage = fceu11::g_ppu.vnapage();
    const uint8_t* nt_base = &fceu11::g_ppu.ntaram()[0];
    auto is_p0 = vnapage[0] == nt_base;
    auto is_p1 = vnapage[1] == nt_base + 0x400;
    auto is_p2 = vnapage[2] == nt_base;
    auto is_p3 = vnapage[3] == nt_base + 0x400;
    if (is_p0 && is_p1 && !is_p2 && !is_p3) {
        return 0;  // horizontal (vnapage[0]=vnapage[1]=base, [2]=[3]=base+0x400)
    }
    if (is_p0 && !is_p1 && is_p2 && !is_p3) {
        return 1;  // vertical (vnapage[0]=vnapage[2]=base, [1]=[3]=base+0x400)
    }
    if (vnapage[0] == nt_base && vnapage[1] == nt_base && vnapage[2] == nt_base && vnapage[3] == nt_base) {
        return 2;  // single-low
    }
    if (vnapage[0] == nt_base + 0x400 && vnapage[1] == nt_base + 0x400) {
        return 3;  // single-high
    }
    return 4;      // four-screen or other
}

// Phase 6.4: scanline-granularity CHR/NT window invalidation — the
// "CHR 窗口缓存与失效" deliverable (plan §7 Phase 6). Multi-bank
// mappers (MMC3/VRC4/…) rewrite `g_bus.vpage[]` mid-frame via bank
// registers; previously the Rust renderer's window snapshot went
// stale until the next $2007 write, so games that switch CHR banks
// per scanline (status bars, split screens) rendered with boot-time
// banks.
//
// Re-copies ONLY the pages whose base pointer moved. Runs from
// `bridge_notify_scanline`, which fires on the Rust scheduler's call
// stack while the StateBox borrow is held — so this must NOT call
// any `fceux11_ppu_*` FFI (re-entry would alias the borrow; the same
// hazard the Phase 5.1 bridge-recursion fix eliminated). Plain
// C++-side memory copies suffice: the renderer reads the window
// backing stores through the power-time pointers, and the notify
// lands BEFORE the new scanline's dot-0 render, so the next
// render_scanline sees the new banks at scanline granularity —
// matching how hblank-timed bank switches are meant to be observed.
//
// NT page moves (mirroring changes) re-copy the page contents here
// too; the mode byte itself is re-pushed at the next frame boundary
// (bridge_push_mirror_mode_if_dirty) because that push is an FFI.
void bridge_refresh_window_contents_if_dirty() {
    auto& vpage = fceu11::g_bus.vpage();
    for (uint32_t p = 0; p < 8; ++p) {
        const uint8_t* page_base = vpage[p] + (p << 10);
        if (page_base == g_chr_page_base[p]) continue;
        g_chr_page_base[p] = page_base;
        std::memcpy(&g_chr_window[p * 1024], page_base, 1024);
    }
    auto& vnapage = fceu11::g_ppu.vnapage();
    const uint8_t* nt_base = &fceu11::g_ppu.ntaram()[0];
    for (uint32_t p = 0; p < 4; ++p) {
        const uint8_t* page_base = vnapage[p] ? vnapage[p] : nt_base;
        if (page_base == g_nt_page_base[p]) continue;
        g_nt_page_base[p] = page_base;
        std::memcpy(&g_nt_window[p * 1024], page_base, 1024);
    }
}

// Phase 6.4: deferred mirror-mode re-push. Frame-boundary C++ code
// only (never from the scanline callback — see
// bridge_refresh_window_contents_if_dirty for the borrow rationale).
void bridge_push_mirror_mode_if_dirty() {
    if (g_ppu_state == nullptr || !g_mirror_mode_pushed_valid) return;
    const uint32_t mode = bridge_derive_mirror_mode();
    if (mode == g_mirror_mode_pushed) return;
    fceux11_ppu_set_mirror_mode(g_ppu_state, mode);
    g_mirror_mode_pushed = mode;
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

uint8_t bridge_cpu_read(uint32_t addr) {
    // Phase 6.6 (Session A): CPU-address-space read for the OAM DMA
    // source fetch ($4014 copies from CPU page $xx00-$xxFF). Routes
    // through the CPU-space dispatch table (`g_bus.read`, the
    // `X6502_DMR` data path) — NOT the PPU-space `bridge_bus_read`
    // chain, which resolved DMA source bytes out of CHR/nametable
    // space and filled OAM with garbage (the Rust engine's sprites
    // have been reading PPU-space bytes as OAM since Phase 3).
    return fceu11::g_bus.read(static_cast<uint16_t>(addr));
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
    // Phase 6.4: scanline-granularity CHR/NT window invalidation.
    // Fires BEFORE the new scanline's dot-0 render (the scheduler
    // notifies on the transition tick; render_scanline_if_start runs
    // at the top of the next dot iteration), so bank switches made
    // during the previous scanline are visible to it. Content-only:
    // no PPU-crate FFI from inside this callback (StateBox borrow is
    // held; see bridge_refresh_window_contents_if_dirty).
    bridge_refresh_window_contents_if_dirty();
    if (PPU_hook) {
        PPU_hook(sl);
    }
}

void bridge_notify_vblank(bool asserted) {
    // Phase 3: the Rust state machine manages the VBL flag itself
    // (sl 241 dot 0 sets, sl -1 dot 1 clears). The C++ legacy
    // PPU_hook / mapper callbacks are notified via PPU_hook (which
    // is fired in `notify_scanline` above when transitioning into
    // sl 241 / sl -1).
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
        &bridge_cpu_read,
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
    // Phase 6.1.d: install $2008-$3FFF mirrors so reads at $200A
    // ($2002 mirror) route through the Rust bridge — otherwise the
    // C++ A2002 handler returns the legacy C++ `PPU_status` which
    // is NOT advanced when the Rust PPU is active (FCEUPPU_Loop
    // delegates to ppu_rust_bridge_run_frame_interleaved and skips
    // FCEUX_PPU_Loop). The blargg vbl_basics subtest 1 asserts
    // $2002 == $200A; if $200A goes to C++ A2002 we return stale
    // C++ status, failing the test. Routing $2008-$3FFF through the
    // bridge keeps both reads observing the same Rust PPU state.
    // (Writes are not mirrored: only $2000-$2007 are write-enabled.)
    for (uint32_t a = 0x2008; a < 0x4000; a++) {
        fceu11::g_bus.set_read_handler(a, a, [](uint32_t A) -> uint8_t {
            return ppu_rust_bridge_cpu_read(A);
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
        // Phase 6.1.d: re-install $2008-$3FFF read mirrors (see
        // bridge_init for rationale; C++ FCEUPPU_Power line 1186
        // installs the original A200x/A2002/etc. mirrors here).
        for (uint32_t a = 0x2008; a < 0x4000; a++) {
            fceu11::g_bus.set_read_handler(a, a, [](uint32_t A) -> uint8_t {
                return ppu_rust_bridge_cpu_read(A);
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
    // Phase 6.3.a: per-frame open-bus decay check. Per-frame
    // granularity (~16.67 ms) is well within the 600 ms threshold
    // tolerance set by blargg ppu_open_bus / ppu_read_buffer.
    const uint64 now = fceu11::cpu_instance().timestamp_base()
        + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());
    fceux11_ppu_check_data_bus_decay(g_ppu_state, now);
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
    // Phase 6.3.a: refresh the Rust-side "current CPU cycle" snapshot
    // the FFI uses to stamp the open-bus latch. Done before the PPU
    // tick so any CPU write later in this cycle observes the up-to-
    // date value. The PPU tick itself doesn't write to PPU regs, but
    // a sprite0 hit / VBL write-back at $2002 is not driven from here
    // — those are explicit CPU reads.
    const uint64 now = fceu11::cpu_instance().timestamp_base()
        + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());
    fceux11_ppu_set_current_cpu_cycle(g_ppu_state, now);
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
    // Phase 6.4: deferred mirror-mode re-push. Mirroring changes made
    // mid-frame (mapper writes during vblank/NMI) re-copied the NT
    // page contents at the scanline boundary; the mode byte itself
    // could not be pushed from the callback (FFI while the Rust
    // scheduler holds the StateBox borrow), so it lands here —
    // before the first visible scanline of the coming frame.
    bridge_push_mirror_mode_if_dirty();
    int rc = fceux11_run_frame_interleaved(
        g_ppu_state,
        reinterpret_cast<uint8_t*>(&fceu11::cpu_instance().native_layout()),
        &bridge_trigger_nmi,
        dots);
    // Phase 6.3.a: per-frame open-bus decay check (same reasoning
    // as `ppu_rust_bridge_emit_frame` above).
    const uint64 now = fceu11::cpu_instance().timestamp_base()
        + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());
    fceux11_ppu_check_data_bus_decay(g_ppu_state, now);
    return rc;
}

void ppu_rust_bridge_copy_framebuffer() {
    if (g_ppu_state == nullptr) return;
    uint8_t* rust_fb = fceux11_ppu_get_framebuffer(g_ppu_state);
    if (rust_fb == nullptr || XBuf == nullptr) return;
    // Visible area: 256x240 = 61440 bytes (XBuf is 256x256).
    std::memcpy(XBuf, rust_fb, 256 * 240);
}

// Phase 6.3.c.2: accessor for the Rust PPU state handle. Used by
// the C++ APU's `DMCDMA()` (`src/sound.cpp:659-686`) via the
// `ppu_rust_bridge_dmc_dma_arbitration` thunk below; future
// integration points (kagami bridge, additional APU hooks) can
// request the handle via this without re-defining it.
//
// Returns `nullptr` when the Rust PPU is inactive (e.g.
// `FCEUX11_RUST_PPU=OFF` build, or the bridge hasn't initialised
// yet — `ppu_rust_bridge_init` is lazy, gated on the first
// `FCEUPPU_Loop` call from the C++ side).
PpuState* ppu_rust_bridge_get_state() {
    return g_ppu_state;
}

// Phase 6.3.c.2: forward DMC DMA stall notification from the C++
// APU to the Rust scheduler. `sound.cpp::DMCDMA()` calls this
// after its four `X6502_DMR` reads so the Rust side's per-dot
// loop can consume the stall via `fceux11_cpu_advance_cycles(cpu,
// -stall_cycles)` and keep `count`/`timestamp_ref` in sync.
//
// No-op when the bridge hasn't installed a state (i.e.
// `g_ppu_state == nullptr`); the per-dot loop sees
// `take_dmc_dma_stall == 0` and falls through to the regular
// `cpu_run_with_tick` branch — behaviour identical to the
// pre-Phase-6.3.c.2 baseline.
void ppu_rust_bridge_dmc_dma_arbitration(uint8_t stall_cycles) {
    if (g_ppu_state == nullptr) return;
    fceux11_ppu_dmc_dma_arbitration(g_ppu_state, stall_cycles);
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
    // A2002 suppression / NMI-cancel: intentionally NOT ported in the
    // bridge. The Rust PPU owns the A2002 mechanism via its internal
    // `vbl_suppressed_this_frame` flag (see fceux11-ppu/src/frame.rs
    // and fceux11-ppu/src/ffi.rs). Per §6.6.ter.12 of
    // docs/plans/v2.1_ppu_rust_refactor_plan.md, a bridge-layer A2002
    // port caused a net -25 batch_compat regression (256 FIXED - 281
    // BROKEN); removing the bridge layer restores the §6.6 baseline
    // 2899/84.0% from 2874/83.3%. Bridge role: message forwarding only,
    // no state-machine logic.
    // Session C v3 (PPU/CPU phase investigation): dump every $2002
    // read with (last PC, PPU sl, dot, CPU cycle). Env-gated by
    // FCEUX11_PPU_PHASE_TRACE=1; silent otherwise. fceu11_e1_last_pc()
    // gives the PC of the last instruction that was dispatched by
    // the Cpu facade (so the $2002 read in this instruction is the
    // next CPU op to run). The trace lets us compare the Rust PPU's
    // (sl, dot) landing for the same instruction against the C++
    // engine's E1 P2002_READ stream.
    if (addr == 0x2002) {
        static const bool on = []() {
            const char* e = std::getenv("FCEUX11_PPU_PHASE_TRACE");
            return e && e[0] == '1' && e[1] == '\0';
        }();
        if (on) {
            const int16_t sl = fceux11_ppu_get_scanline(g_ppu_state);
            const uint16_t dot = fceux11_ppu_get_dot(g_ppu_state);
            const uint16_t pc = fceu11_e1_last_pc();
            const uint64 now = fceu11::cpu_instance().timestamp_base()
                + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());
            std::fprintf(stderr,
                "R3 P2002_READ abs=%llu sl=%d dot=%d pc=0x%04X\n",
                (unsigned long long)now, (int)sl, (int)dot,
                (unsigned)pc);
        }
    }
    return fceux11_ppu_cpu_read(g_ppu_state, static_cast<uint16_t>(addr));
}

void ppu_rust_bridge_cpu_write(uint32_t addr, uint8_t value) {
    if (g_ppu_state == nullptr) {
        return;
    }
    // Phase 6.4: stamp the open-bus decay reference with the CPU's
    // live cycle at write time. The only other stamp site was the
    // retired Phase 5.1 per-dot C++ loop (`advance_ppu_dots`); since
    // Phase 5.3 the frame loop runs in Rust and never updated
    // `current_cpu_cycle`, so every `refresh_data_bus` stamped cycle
    // 0 and the per-frame decay check zeroed the latch ~600 ms after
    // boot regardless of write recency (blargg ppu_open_bus /
    // ppu_read_buffer divergence; §6.3.a.4 A/B follow-up).
    const uint64 now = fceu11::cpu_instance().timestamp_base()
        + static_cast<uint64>(fceu11::cpu_instance().timestamp_ref());
    fceux11_ppu_set_current_cpu_cycle(g_ppu_state, now);
    // Phase 6.1.b — port B2000 rising-edge NMI-enable to Rust bridge.
    // C++ reference (src/ppu.cpp:958-962):
    //   if (!(PPU[0] & 0x80) && (V & 0x80) && (PPU_status & 0x80))
    //       TriggerNMI2();
    // We detect the 0→1 transition on bit 7 BEFORE the write by
    // reading ctrl via fceux11_ppu_get_register_state, then check VBL
    // status AFTER the write (Rust PPU's tick_dot asserts VBL on sl
    // 241 dot 1, so by the time the CPU executes B2000 the flag is
    // visible per get_register_state).
    if (addr == 0x2000) {
        const uint8_t old_ctrl = fceux11_ppu_get_register_state(g_ppu_state, 0);
        fceux11_ppu_cpu_write(g_ppu_state, static_cast<uint16_t>(addr), value);
        const uint8_t status = fceux11_ppu_get_register_state(g_ppu_state, 2);
        if (!(old_ctrl & 0x80) && (value & 0x80) && (status & 0x80)) {
            TriggerNMI2();
        }
    } else {
        fceux11_ppu_cpu_write(g_ppu_state, static_cast<uint16_t>(addr), value);
    }
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
