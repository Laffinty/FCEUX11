// vNESU11 Shadow Run harness — Phase 6 §2.5 (skeleton + CPU sync).
//
// Holds the latest Rust emulator capture (palette-index frame +
// i16 stereo audio) for external harnesses to consume. Provides a
// CRC32 helper for frame comparison against baseline files.
//
// The "real" 3-tier diff (XBuf CRC, audio SNR, savestate MD5) is
// driven from `kagami_qa_shadow_run_runner` (Phase 6 §3.3); this
// file exposes the data + a periodic log hook + the C++→Rust state
// sync helper.

#include "vnesu11_shadow.h"

#include <cstdio>
#include <cstring>

#include "cpu.h"          // g_cpu.native_layout()
#include "fceu.h"         // ::RAM, X6502
#include "sound.h"        // fhcnt / fcnt / IRQFrameMode / SIRQStat / EnabledChannels / fc_reset_in / fc_pending_mode
#include "ppu.h"          // PPUGenLatch, g_ppu (Ppu::reg/ntaram/oam)
#include "ppu_state.h"    // PALRAM
#include "vnesu11_bridge.h"

#ifdef VNESU11_CORE_ENABLED
// Rust extern "C" exports (see crates/vnesu11/src/ffi.rs).
// `vnesu11_apu_poke_state` is declared in vnesu11_shadow.h (FFI
// block lives outside any namespace).
extern "C" {
int vnesu11_set_wram(void* soc, const uint8_t* src);
void vnesu11_cpu_poke_regs(void* soc, const void* regs);
void vnesu11_cpu_peek_regs(void* soc, void* out);
}
#endif

// Global C++ symbols read by the APU sync below. Declared OUTSIDE the
// fceu11 namespace so they resolve to ::PAL / ::fhcnt (the definitions
// in src/fceu.cpp / src/sound.cpp are at global scope).
extern uint8 PAL;

namespace fceu11 {

// ShadowData — single definition (the bridge references it via
// `extern` in vnesu11_bridge.cpp).
ShadowData g_shadow;

// Last comparison result (filled by `vnesu11_shadow_sync_from_cpp`
// on the C++ side and read by the runner).
static ShadowCompare g_compare{};

int vnesu11_shadow_get(ShadowData* out) noexcept {
    if (!out) return -1;
    *out = g_shadow;
    return 0;
}

void vnesu11_shadow_reset() noexcept {
    g_shadow = ShadowData{};
    g_compare = ShadowCompare{};
}

int vnesu11_shadow_compare(ShadowCompare* out) noexcept {
    if (!out) return -1;
    *out = g_compare;
    return 0;
}

// CRC32 (IEEE 802.3, polynomial 0xEDB88320). Standard lookup-table
// implementation.
namespace {
struct Crc32Table {
    uint32_t v[256];
    constexpr Crc32Table() : v{} {
        for (uint32_t i = 0; i < 256; ++i) {
            uint32_t c = i;
            for (int k = 0; k < 8; ++k) {
                c = (c >> 1) ^ (0xEDB88320u & (~(c & 1) + 1));
            }
            v[i] = c;
        }
    }
};
constexpr Crc32Table kCrc32{};
}  // namespace

uint32_t vnesu11_shadow_crc32(const uint8_t* data, size_t n) noexcept {
    uint32_t c = 0xFFFFFFFFu;
    for (size_t i = 0; i < n; ++i) {
        c = kCrc32.v[(c ^ data[i]) & 0xFF] ^ (c >> 8);
    }
    return c ^ 0xFFFFFFFFu;
}

void vnesu11_shadow_log_every(uint64_t every_n) noexcept {
    if (every_n == 0) return;
    if (g_shadow.frame_count == 0) return;
    if (g_shadow.frame_count % every_n != 0) return;

    const uint32_t xcrc = vnesu11_shadow_crc32(g_shadow.xbuf, sizeof(g_shadow.xbuf));
    std::fprintf(stderr,
        "[vnesu11_shadow] frame=%llu xbuf_crc=0x%08X audio_samples=%zu\n",
        static_cast<unsigned long long>(g_shadow.frame_count),
        static_cast<unsigned>(xcrc),
        g_shadow.sbuf_written);
}

#ifdef VNESU11_CORE_ENABLED

/// Sync the C++ post-frame CPU + WRAM state into the Rust SoC. Runs
/// inside `fceu11::Emulate()` *after* the C++ pipeline commits its
/// XBuf/SoundBuf, and *before* the Rust shadow frame — so the Rust
/// core executes the next frame from the same starting state the C++
/// core used.
///
/// The mapper state needs no sync: the Rust bus delegates mapper
/// reads/writes to the C++ mappers through the per-range FFI thunks.
/// CPU registers are copied as a raw `CpuRegsLayout` blob — the
/// layout parity is enforced by `crates/vnesu11/tests/layout_check.rs`
/// and the `x6502struct.h` static_asserts (audit S1).
void vnesu11_shadow_sync_from_cpp() noexcept {
    if (!g_vnesu11_soc) return;

    // 1. WRAM (2 KiB) — the C++ RAM global.
    if (RAM) {
        vnesu11_set_wram(g_vnesu11_soc, RAM);
    }

    // 2. CPU registers (PC/A/X/Y/S/P/...).
    //    `g_cpu.native_layout()` is the X6502 struct; `CpuRegsLayout`
    //    is its 64-byte #[repr(C)] mirror. The FCEUDEF_DEBUGGER build
    //    has the 3 hook pointers at 32/40/48 in both, so the blobs are
    //    layout-identical.
    vnesu11_cpu_poke_regs(g_vnesu11_soc, &g_cpu.native_layout());

    // 3. APU state (Phase 6 P2 — frame counter + master cycle counter).
    //    The shadow harness previously only synced WRAM + CPU, which
    //    left Rust's frame counter free-running. That drifted Rust's
    //    IRQ firing by one frame's worth of cycles from C++'s, causing
    //    blargg cpu_dummy_reads to diverge at frame 3 (Rust enters
    //    the IRQ handler at $E622 while C++ stays in the $2002 wait
    //    loop). Snapshotting the C++ side's fhcnt + fcnt + IRQFrameMode
    //    + fc_pending_mode + fc_reset_in + cycle count and pushing them
    //    into Rust aligns the frame counter timing.
    fceu11::ApuStateMirror apu{};
    apu.cycles = static_cast<uint64_t>(g_cpu.timestamp_ref())
               + static_cast<uint64_t>(timestampbase);  // C++ master cycle count
    apu.fc_cycle_count = static_cast<uint64_t>(fhcnt);
    apu.fc_step = fcnt;
    // IRQFrameMode = (raw $4017 & 0xC0) >> 6 — 5-step in bit 1, inhibit in bit 0.
    apu.fc_five_step = (IRQFrameMode & 0x2) != 0;
    apu.fc_irq_inhibit = (IRQFrameMode & 0x1) != 0;
    // PAL timing — C++ global `PAL` (declared above, outside the
    // namespace, so it resolves to ::PAL not fceu11::PAL).
    apu.pal = (::PAL != 0);
    apu.fc_pending_mode = fc_pending_mode & 0xC0;
    apu.fc_reset_in = static_cast<uint8_t>(fc_reset_in < 0 ? 0 : fc_reset_in);
    apu.fc_quarter_frame = false; // latched only inside FCEU_SoundCPUHook; not
                                  // needed for cross-frame sync (consumer is the
                                  // next instruction's IRQ check, not envelope ticks).
    apu.fc_half_frame = false;
    apu.frame_irq_pending = (SIRQStat & 0x40) != 0;
    apu.dmc_irq_pending = (SIRQStat & 0x80) != 0;
    apu.enabled_channels = EnabledChannels;
    vnesu11_apu_poke_state(g_vnesu11_soc, &apu);

    // 4. PPU state (Phase 6 P2 — registers + CPU-observable memory).
    //    Previously the sync only covered WRAM + CPU + APU frame
    //    counter, leaving Rust's PPU free-running. The C++ and Rust
    //    PPUs then diverged, so $2002 reads (e.g. the BIT $2002
    //    VBlank-wait loop at $E48D in cpu_dummy_reads.nes) returned
    //    different values and the CPU took different branches after a
    //    few frames. Syncing the CPU-observable PPU state closes that.
    fceu11::PpuStateMirror ppu{};
    // Registers — g_ppu is fceu11::Ppu (see ppu_class.h); reg(0..3)
    // is the PPU[0..3] register file ($2000-$2003).
    ppu.ppuctrl = fceu11::g_ppu.reg(0);
    ppu.ppumask = fceu11::g_ppu.reg(1);
    ppu.status  = fceu11::g_ppu.reg(2);
    ppu.oam_addr = fceu11::g_ppu.reg(3);
    // $2007 read buffer + CPU open bus.
    ppu.read_buffer = PPUGenLatch;
    ppu.open_bus = g_cpu.native_layout().DB;
    // Palette RAM (C++ PALRAM[0x20]).
    std::memcpy(ppu.palette, PALRAM.data(), sizeof(ppu.palette));
    // Name-table RAM (C++ NTARAM[0x800] → g_ppu.ntaram()).
    std::memcpy(ppu.vram, fceu11::g_ppu.ntaram(), sizeof(ppu.vram));
    // OAM (C++ g_ppu.oam()[256]).
    std::memcpy(ppu.oam, fceu11::g_ppu.oam(), sizeof(ppu.oam));
    vnesu11_ppu_poke_state(g_vnesu11_soc, &ppu);
}

#else  // !VNESU11_CORE_ENABLED

// No-op: the shadow sync requires the Rust core. Kept so the symbol
// stays defined for any TU that references it unconditionally.
void vnesu11_shadow_sync_from_cpp() noexcept {}

#endif  // VNESU11_CORE_ENABLED

}  // namespace fceu11