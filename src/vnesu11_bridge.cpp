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
}

void vnesu11_reset_bridge(void* soc) {
    vnesu11_reset(soc);
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

// Phase 0 §1.4: route fceu.cpp::Emulate through this trampoline when
// VNESU11_CORE is enabled. Phase 0 stub: writes nothing, reports 0 samples.
// Phase 1+ wires real emulation through vnesu11_emulate_frame().
void vnesu11_emulate_frame_bridge(
    void* soc, int skip,
    uint8_t* xbuf, int16_t* sbuf, size_t sbuf_cap, size_t* sbuf_written)
{
    if (!soc || !sbuf_written) return;
    if (sbuf_cap > 0 && sbuf) sbuf[0] = 0;  // touch buffer to avoid UB
    *sbuf_written = 0;
    (void)xbuf;
    (void)skip;
    // The actual call to vnesu11_emulate_frame would go here in Phase 6+.
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
