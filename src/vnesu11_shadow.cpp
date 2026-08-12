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
#include "vnesu11_bridge.h"

#ifdef VNESU11_CORE_ENABLED
// Rust extern "C" exports (see crates/vnesu11/src/ffi.rs).
extern "C" {
int vnesu11_set_wram(void* soc, const uint8_t* src);
void vnesu11_cpu_poke_regs(void* soc, const void* regs);
void vnesu11_cpu_peek_regs(void* soc, void* out);
}
#endif

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
}

#else  // !VNESU11_CORE_ENABLED

// No-op: the shadow sync requires the Rust core. Kept so the symbol
// stays defined for any TU that references it unconditionally.
void vnesu11_shadow_sync_from_cpp() noexcept {}

#endif  // VNESU11_CORE_ENABLED

}  // namespace fceu11