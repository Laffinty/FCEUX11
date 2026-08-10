// vNESU11 C-ABI bridge — C++ wrapper for the vnesu11 Rust crate.
//
// Phase 0: minimal lifecycle stubs (create/destroy/power/reset) + per-range
// handler forwarding (SetReadHandler/SetWriteHandler). Phase 6 will expand
// this to the full surface (see 02_architecture.md §5).

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Opaque handle for a vNESU11 SoC instance.
/// Forward-declared; C++ code must always go through the accessors below.
struct VNesSocOpaque;

// -------------------------------------------------------------------------
// Lifecycle
// -------------------------------------------------------------------------
void* vnesu11_create_bridge(void);
void  vnesu11_destroy_bridge(void* soc);
void  vnesu11_power_on_bridge(void* soc);
void  vnesu11_reset_bridge(void* soc);

// -------------------------------------------------------------------------
// Frame emulation (Phase 0 §1.4: routed under #ifdef VNESU11_CORE_ENABLED
// in src/fceu.cpp::Emulate). The Phase 0 stub returns -1; Phase 1+ wires
// real emulation.
// -------------------------------------------------------------------------
void vnesu11_emulate_frame_bridge(
    void* soc, int skip,
    uint8_t* xbuf, int16_t* sbuf, size_t sbuf_cap, size_t* sbuf_written);

// -------------------------------------------------------------------------
// Mapper per-range handler registration (Phase 0 — see ADR-010)
// -------------------------------------------------------------------------
int vnesu11_set_read_handler_bridge(void* soc, uint16_t start, uint16_t end,
                                    uint8_t (*fn)(void*, uint16_t),
                                    void* ctx);
int vnesu11_set_write_handler_bridge(void* soc, uint16_t start, uint16_t end,
                                     void (*fn)(void*, uint16_t, uint8_t),
                                     void* ctx);
void vnesu11_clear_mapper_handlers_bridge(void* soc);

#ifdef __cplusplus
}  // extern "C"
#endif

// -------------------------------------------------------------------------
// C++ helpers (Phase 0: minimal; Phase 6 expands)
// -------------------------------------------------------------------------
#ifdef __cplusplus

namespace fceu11 {

// Global SoC handle (set by vnesu11_init, used by per-range forwarding).
// Declared `extern`; the single definition lives in vnesu11_bridge.cpp.
extern void* g_vnesu11_soc;

void vnesu11_init(void);
void vnesu11_kill(void);

}  // namespace fceu11

#endif
