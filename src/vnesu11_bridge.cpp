// vNESU11 C-ABI bridge implementation — Phase 0.
//
// Wraps the vNESU11 extern "C" surface so C++ can drive it without
// including the cbindgen-generated header. Phase 6 will expand this
// to cover FCEUI_* compat shims and the full emulation entry points.

#include "vnesu11_bridge.h"

#include <cstdio>
#include <cstdlib>

// Direct declarations of the vNESU11 extern "C" entry points (declared in
// crates/vnesu11/src/ffi.rs and exported by vnesu11.lib). We declare them
// locally rather than via a generated header so Phase 0 ships without the
// cbindgen step (see phase_0_foundation.md §1.2 / build.rs TODO).
extern "C" {

void* vnesu11_create(void);
void  vnesu11_destroy(void* soc);
void  vnesu11_power_on(void* soc);
void  vnesu11_reset(void* soc);

int   vnesu11_set_read_handler(void* soc, uint16_t start, uint16_t end,
                                uint8_t (*fn)(void*, uint16_t),
                                void* ctx);
int   vnesu11_set_write_handler(void* soc, uint16_t start, uint16_t end,
                                 void (*fn)(void*, uint16_t, uint8_t),
                                 void* ctx);
void  vnesu11_clear_mapper_handlers(void* soc);

}  // extern "C"

namespace fceu11 {

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
