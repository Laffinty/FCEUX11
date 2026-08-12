// vNESU11 mapper adapter — implementation (Phase 5 stage 1).
//
// Two halves:
//  1. Per-range forwarding: each C++ `SetReadHandler(start, end, fn)`
//     call registers a (start, end, thunk, ctx) entry in the Rust
//     range table. The `ctx` points at a fixed-slot thunk holding the
//     original C++ fn; the Rust bus calls `thunk(ctx, addr)` on every
//     mapper-region access, which dispatches to the original fn.
//  2. Mapper meta vtable: a `#[repr(C)]`-compatible
//     `MapperMetaVtable` (mirroring / fill_audio / tick_irq /
//     save_state / load_state) backed by the Cart. Phase 5 wires the
//     plumbing (attach + savestate delegation); the deep audio / IRQ
//     wiring lands in Phase 6 shadow-run integration.

#include "vnesu11_mapper_adapter.h"

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <vector>

#include "cart.h"         // CartInfo, currCartInfo
#include "cart_class.h"   // fceu11::g_cart, Cart::save_mapper_state
#include "x6502.h"        // g_cpu.native_layout().IRQlow, FCEU_IQEXT
#include "vnesu11_bridge.h"  // fceu11::g_vnesu11_soc

#ifdef VNESU11_CORE_ENABLED

// Rust extern "C" exports (see crates/vnesu11/src/ffi.rs). Declared
// locally so this TU controls the exact ABI types; the cbindgen header
// (target/vnesu11_ffi.h) carries the same signatures.
extern "C" {
void* vnesu11_create(void);
int   vnesu11_set_read_handler(void* soc, uint16_t start, uint16_t end,
                               uint8_t (*fn)(void*, uint16_t), void* ctx);
int   vnesu11_set_write_handler(void* soc, uint16_t start, uint16_t end,
                                void (*fn)(void*, uint16_t, uint8_t), void* ctx);
void  vnesu11_clear_mapper_handlers(void* soc);
int   vnesu11_attach_mapper_meta(void* soc, void* mapper, const void* vtable);
int   vnesu11_set_system_type(void* soc, uint32_t system_type);
int   vnesu11_chr_set_page(void* soc, uint8_t page_idx, const uint8_t* src);
}  // extern "C"

// ---------------------------------------------------------------------------
// Thunk pool
// ---------------------------------------------------------------------------
//
// Mirrors crates/vnesu11/src/mapper.rs MAX_RANGES=64 per direction.
// The pool is reset on every game load (vnesu11_on_game_load), so no
// heap allocation and no lifetime tracking is needed.

namespace {

constexpr int kMaxRanges = 64;

struct ReadThunk {
    readfunc fn;
    uint32_t start;
    uint32_t end;
};
struct WriteThunk {
    writefunc fn;
    uint32_t start;
    uint32_t end;
};

ReadThunk  g_read_thunks[kMaxRanges];
WriteThunk g_write_thunks[kMaxRanges];
int g_read_thunk_count = 0;
int g_write_thunk_count = 0;

}  // namespace

extern "C" uint8_t vnesu11_mapper_read_thunk(void* ctx, uint16_t addr) {
    auto* t = static_cast<ReadThunk*>(ctx);
    return t ? t->fn(addr) : 0;
}

extern "C" void vnesu11_mapper_write_thunk(void* ctx, uint16_t addr, uint8_t val) {
    auto* t = static_cast<WriteThunk*>(ctx);
    if (t) t->fn(addr, val);
}

// ---------------------------------------------------------------------------
// Mapper meta vtable — #[repr(C)] mirror of the Rust MapperMetaVtable.
// ---------------------------------------------------------------------------

namespace {

struct VNesMapperMetaVtable {
    uint8_t (*mirroring)(void*);
    void    (*fill_audio)(void*, int16_t*, size_t);
    void    (*tick_irq)(void*, bool*);
    int32_t (*save_state)(void*, uint8_t*, size_t, size_t*);
    int32_t (*load_state)(void*, const uint8_t*, size_t);
};

/// Current nametable mirroring. `currCartInfo->mirror` holds
/// MI_H(0)/MI_V(1)/MI_0(2)/MI_1(3) — the same encoding as the Rust
/// `Mirroring` enum (Horizontal/Vertical/SingleScreenLow/
/// SingleScreenHigh), so the byte passes through unchanged.
uint8_t mapper_meta_mirroring(void* /*ctx*/) {
    if (currCartInfo) {
        return static_cast<uint8_t>(currCartInfo->mirror & 0xFF);
    }
    return 0;  // MI_H
}

/// Expansion-audio fill. Phase 5 keeps this a no-op: the Rust APU
/// output buffer ↔ C++ ExpansionAudio::fill() bridge is Phase 6
/// shadow-run integration (the vtable entry exists so the FFI shape
/// is stable).
void mapper_meta_fill_audio(void* /*ctx*/, int16_t* /*out*/, size_t /*count*/) {}

/// Mapper IRQ line. Phase 6 (S6/S7 wiring): query the C++ CPU's IRQ
/// pending bitmask and report whether `FCEU_IQEXT` is set — that's
/// the mapper-driven IRQ line (MMC3 scanline, FDS disk, etc.). The
/// Rust `route_interrupts()` aggregates this into `CpuCore::IRQ_EXT`.
void mapper_meta_tick_irq(void* /*ctx*/, bool* out) {
    if (!out) return;
    *out = (g_cpu.native_layout().IRQlow & FCEU_IQEXT) != 0;
}

/// Serialize the mapper's savestate body (Cart::save_mapper_state).
int32_t mapper_meta_save_state(void* /*ctx*/, uint8_t* dst, size_t cap,
                               size_t* written) {
    if (!dst || !written) return -1;
    *written = 0;
    if (!fceu11::g_cart) return -3;
    const std::vector<uint8_t> body = fceu11::g_cart->save_mapper_state();
    if (body.size() > cap) return -2;  // buffer too small
    if (!body.empty()) {
        std::memcpy(dst, body.data(), body.size());
    }
    *written = body.size();
    return 0;
}

/// Restore the mapper's savestate body (Cart::load_mapper_state).
int32_t mapper_meta_load_state(void* /*ctx*/, const uint8_t* src, size_t len) {
    if (!src) return -1;
    if (!fceu11::g_cart) return -3;
    const std::vector<uint8_t> body(src, src + len);
    return fceu11::g_cart->load_mapper_state(body) ? 0 : -4;
}

}  // namespace

// ---------------------------------------------------------------------------
// Forwarding entry points
// ---------------------------------------------------------------------------

namespace fceu11 {

void vnesu11_forward_set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept {
    if (!g_vnesu11_soc) return;
    if (end < start) return;
    if (!fn) return;  // fceu.cpp substitutes ANull before calling us.

    // Re-registration of the same range (mappers call Power() again on
    // reset): update the thunk's fn in place so both the C++ ARead[]
    // table and the Rust range table stay consistent.
    for (int i = 0; i < g_read_thunk_count; ++i) {
        if (g_read_thunks[i].start == start && g_read_thunks[i].end == end) {
            g_read_thunks[i].fn = fn;
            return;
        }
    }
    if (g_read_thunk_count >= kMaxRanges) {
        std::fprintf(stderr, "vnesu11_mapper_adapter: read range pool exhausted\n");
        return;
    }
    const int i = g_read_thunk_count++;
    g_read_thunks[i] = ReadThunk{fn, start, end};
    const int rc = vnesu11_set_read_handler(
        g_vnesu11_soc,
        static_cast<uint16_t>(start),
        static_cast<uint16_t>(end),
        vnesu11_mapper_read_thunk,
        &g_read_thunks[i]);
    if (rc != 0) {
        std::fprintf(stderr, "vnesu11_mapper_adapter: set_read_handler rc=%d\n", rc);
        g_read_thunk_count--;
    }
}

void vnesu11_forward_set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept {
    if (!g_vnesu11_soc) return;
    if (end < start) return;
    if (!fn) return;

    for (int i = 0; i < g_write_thunk_count; ++i) {
        if (g_write_thunks[i].start == start && g_write_thunks[i].end == end) {
            g_write_thunks[i].fn = fn;
            return;
        }
    }
    if (g_write_thunk_count >= kMaxRanges) {
        std::fprintf(stderr, "vnesu11_mapper_adapter: write range pool exhausted\n");
        return;
    }
    const int i = g_write_thunk_count++;
    g_write_thunks[i] = WriteThunk{fn, start, end};
    const int rc = vnesu11_set_write_handler(
        g_vnesu11_soc,
        static_cast<uint16_t>(start),
        static_cast<uint16_t>(end),
        vnesu11_mapper_write_thunk,
        &g_write_thunks[i]);
    if (rc != 0) {
        std::fprintf(stderr, "vnesu11_mapper_adapter: set_write_handler rc=%d\n", rc);
        g_write_thunk_count--;
    }
}

void vnesu11_on_game_load(int system_type) noexcept {
    if (!g_vnesu11_soc) return;

    // 1. Drop the previous game's ranges + thunk pool.
    vnesu11_clear_mapper_handlers(g_vnesu11_soc);
    g_read_thunk_count = 0;
    g_write_thunk_count = 0;

    // 2. System type (C++ EGIT: 0=CART, 1=VSUNI, 2=FDS, 3=NSF).
    vnesu11_set_system_type(g_vnesu11_soc, static_cast<uint32_t>(system_type));

    // 3. Mapper meta vtable (ctx = the active Cart).
    static const VNesMapperMetaVtable kMeta = {
        mapper_meta_mirroring,
        mapper_meta_fill_audio,
        mapper_meta_tick_irq,
        mapper_meta_save_state,
        mapper_meta_load_state,
    };
    vnesu11_attach_mapper_meta(g_vnesu11_soc, fceu11::g_cart, &kMeta);
}

}  // namespace fceu11

#else  // !VNESU11_CORE_ENABLED

// No-op definitions so the header's callers (fceu.cpp hooks) stay
// linkable when VNESU11_CORE is OFF.
namespace fceu11 {
void vnesu11_forward_set_read_handler(uint32_t /*start*/, uint32_t /*end*/,
                                      readfunc /*fn*/) noexcept {}
void vnesu11_forward_set_write_handler(uint32_t /*start*/, uint32_t /*end*/,
                                       writefunc /*fn*/) noexcept {}
void vnesu11_on_game_load(int /*system_type*/) noexcept {}
}  // namespace fceu11

#endif  // VNESU11_CORE_ENABLED
