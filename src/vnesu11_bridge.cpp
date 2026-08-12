// vNESU11 C-ABI bridge implementation — Phase 0.
//
// Wraps the vNESU11 extern "C" surface so C++ can drive it. Only compiled
// into real code when VNESU11_CORE=ON; when OFF it contributes no-op
// definitions so the header's callers stay linkable.
//
// NOTE: we deliberately do NOT include the cbindgen-generated
// vnesu11_ffi.h here. The Rust exports use `*mut VNesSocOpaque` (an
// opaque pointer); bridge.h uses `void*` for the same handle to avoid
// leaking the cbindgen type into every C++ TU. The `extern "C"`
// declarations below mirror the Rust signatures with `void*` in place of
// `VNesSocOpaque*` — identical ABI on x86-64 (both are plain pointers),
// no type-name collision with bridge.h's `struct VNesSocOpaque`.

#include "vnesu11_bridge.h"

#include <cstdio>
#include <cstdlib>

#ifdef VNESU11_CORE_ENABLED

#include "vnesu11_shadow.h"  // v2.0 wip (Phase 6): shadow harness data + log

// Rust extern "C" exports (see crates/vnesu11/src/ffi.rs). ABI-compatible
// with the cbindgen header; declared locally so we control the handle type.
extern "C" {
void* vnesu11_create(void);
void  vnesu11_destroy(void* soc);
void  vnesu11_power_on(void* soc);
void  vnesu11_reset(void* soc);
int   vnesu11_set_read_handler(void* soc, uint16_t start, uint16_t end,
                                uint8_t (*fn)(void*, uint16_t), void* ctx);
int   vnesu11_set_write_handler(void* soc, uint16_t start, uint16_t end,
                                 void (*fn)(void*, uint16_t, uint8_t), void* ctx);
void  vnesu11_clear_mapper_handlers(void* soc);
int   vnesu11_attach_mapper_meta(void* soc, void* mapper, const void* vtable);
int   vnesu11_set_system_type(void* soc, uint32_t system_type);
int   vnesu11_chr_set_page(void* soc, uint8_t page_idx, const uint8_t* src);
}  // extern "C"

namespace fceu11 {

// Defined here (bridge.h declares `extern`, not `inline`, so exactly one
// definition across all TUs — this .cpp is it).
void* g_vnesu11_soc = nullptr;

void vnesu11_init(void) {
    if (g_vnesu11_soc) return;
    g_vnesu11_soc = vnesu11_create();
    if (!g_vnesu11_soc) {
        std::fprintf(stderr, "vnesu11_bridge: vnesu11_create() failed\n");
    }
}

void vnesu11_kill(void) {
    if (g_vnesu11_soc) {
        vnesu11_destroy(g_vnesu11_soc);
        g_vnesu11_soc = nullptr;
    }
}

}  // namespace fceu11

// ---------------------------------------------------------------------------
// Bridge functions (thin trampolines)
// ---------------------------------------------------------------------------

void* vnesu11_create_bridge(void) {
    return vnesu11_create();
}

void vnesu11_destroy_bridge(void* soc) {
    vnesu11_destroy(soc);
}

void vnesu11_power_on_bridge(void* soc) {
    vnesu11_power_on(soc);
    fceu11::vnesu11_shadow_reset();
}

void vnesu11_reset_bridge(void* soc) {
    vnesu11_reset(soc);
    fceu11::vnesu11_shadow_reset();
}

int vnesu11_set_read_handler_bridge(void* soc, uint16_t start, uint16_t end,
                                    uint8_t (*fn)(void*, uint16_t),
                                    void* ctx) {
    return vnesu11_set_read_handler(soc, start, end, fn, ctx);
}

int vnesu11_set_write_handler_bridge(void* soc, uint16_t start, uint16_t end,
                                     void (*fn)(void*, uint16_t, uint8_t),
                                     void* ctx) {
    return vnesu11_set_write_handler(soc, start, end, fn, ctx);
}

void vnesu11_clear_mapper_handlers_bridge(void* soc) {
    vnesu11_clear_mapper_handlers(soc);
}

// Phase 6 §2.2: real implementation. Calls the Rust
// `vnesu11_emulate_frame` FFI and copies its frame_buffer (palette
// indices, 256×240 = 61440 bytes) into the caller's `xbuf`. Drains
// the APU output buffer (i16 interleaved stereo) into the caller's
// `sbuf`.
//
// Format note (Phase 6 §2.5 shadow run, ongoing):
//   - Rust xbuf = palette indices (0..=3 per pixel encoded as
//     (palette << 2) | color).
//   - C++ XBuf   = NES color indices (after C++ palette RAM lookup).
//   Direct byte comparison is therefore not meaningful; the Phase 6
//   shadow harness compares CRCs separately and notes the format
//   delta in its report.
//   - Rust sbuf = int16 stereo samples (one per CPU cycle, ~29,780
//     per frame).
//   - C++ WaveFinal = int32 stereo samples (one per ~735 audio
//     frames, after C++ mixer + filter).
//   The Rust i16 buffer is kept in a side buffer (ShadowData.sbuf)
//   for later rate-corrected SNR diff (Phase 6 ongoing).
extern "C" {
int vnesu11_emulate_frame(
    void* soc, int skip,
    uint8_t* xbuf, int16_t* sbuf, size_t sbuf_cap,
    size_t* sbuf_written);
}  // extern "C"

namespace fceu11 {

// `ShadowData` + its storage and `vnesu11_shadow_reset` live in
// vnesu11_shadow.cpp (the shadow harness module). This TU only
// writes into it via `vnesu11_emulate_frame_bridge`.
extern ShadowData g_shadow;

}  // namespace fceu11

// Phase 6 §2.2: bridge wrapper invoked by Emulate() (under
// VNESU11_CORE_ENABLED). Runs one frame through the Rust core, copying
// the Rust frame_buffer into the caller's xbuf.
void vnesu11_emulate_frame_bridge(
    void* soc, int skip,
    uint8_t* xbuf, int16_t* sbuf, size_t sbuf_cap, size_t* sbuf_written)
{
    if (!soc || !sbuf_written) return;
    *sbuf_written = 0;
    if (!xbuf) return;

    // Run the Rust emulator for one frame.
    const int rc = vnesu11_emulate_frame(
        soc, skip, xbuf, sbuf, sbuf_cap, sbuf_written);

    // Capture into the shadow buffer so the harness can compare
    // against the C++ XBuf / WaveFinal later.
    if (rc == 0 && xbuf && *sbuf_written <= sizeof(fceu11::g_shadow.sbuf) / sizeof(int16_t)) {
        std::memcpy(fceu11::g_shadow.xbuf, xbuf, 61440);
        std::memcpy(
            fceu11::g_shadow.sbuf, sbuf,
            *sbuf_written * sizeof(int16_t));
        fceu11::g_shadow.sbuf_written = *sbuf_written;
        fceu11::g_shadow.frame_count++;
    }
}

#else  // !VNESU11_CORE_ENABLED

// When VNESU11_CORE is OFF, the bridge contributes no-op definitions so
// the header's callers (bus.cpp forwarding, etc. — all gated by
// VNESU11_CORE_ENABLED themselves) stay linkable.
namespace fceu11 {
void* g_vnesu11_soc = nullptr;
void vnesu11_init(void) {}
void vnesu11_kill(void) {}
}  // namespace fceu11

void* vnesu11_create_bridge(void) { return nullptr; }
void vnesu11_destroy_bridge(void* /*soc*/) {}
void vnesu11_power_on_bridge(void* /*soc*/) {}
void vnesu11_reset_bridge(void* /*soc*/) {}
int vnesu11_set_read_handler_bridge(void* /*soc*/, uint16_t /*start*/, uint16_t /*end*/,
                                    uint8_t (*/*fn*/)(void*, uint16_t), void* /*ctx*/) { return -1; }
int vnesu11_set_write_handler_bridge(void* /*soc*/, uint16_t /*start*/, uint16_t /*end*/,
                                     void (*/*fn*/)(void*, uint16_t, uint8_t), void* /*ctx*/) { return -1; }
void vnesu11_clear_mapper_handlers_bridge(void* /*soc*/) {}
void vnesu11_emulate_frame_bridge(void* /*soc*/, int /*skip*/,
                                  uint8_t* /*xbuf*/, int16_t* /*sbuf*/, size_t /*sbuf_cap*/,
                                  size_t* /*sbuf_written*/) {}

#endif  // VNESU11_CORE_ENABLED
