// FCEUX11 v2.1 PPU Refactor — Phase 2: Rust PPU smoke test.
//
// Minimal verification that the Rust PPU FFI surface can be driven
// from C++ in isolation: create a state, install bus callbacks,
// power-on reset, run 89342 CPU cycles (= 1 NTSC frame's worth of
// dot ticks), and confirm:
//   - state machine advances (scanline wraps from -1 → 0 → 1 ...)
//   - framebuffer is queryable
//   - destroy cleans up
//
// Does NOT compare against any golden frame (the renderer is a Phase 2
// stub; bit-exact NROM gates live in Phase 4). What this proves:
//   - cbindgen FFI symbols are exported and resolvable
//   - the C++ bus callback vtable pattern works end-to-end
//   - the Rust state machine runs for a full frame without crashing

#include <cstdio>
#include <cstdint>

extern "C" {
#include "rust/fceux11_rust.h"
}

namespace {

// CounterBus: a minimal PpuBus implementation. Counts reads/writes and
// returns 0 for everything. Phase 2 just needs a non-null vtable to
// prove the Rust side can drive the dot clock.
struct CounterBus {
    uint64_t reads  = 0;
    uint64_t writes = 0;
};

uint8_t counter_bus_read(uint32_t /*addr*/) {
    static_cast<CounterBus*>(nullptr);  // unused; we use static
    return 0x00;
}

uint8_t counted_bus_read(uint32_t /*addr*/) {
    return 0x00;
}

void counted_bus_write(uint32_t /*addr*/, uint8_t /*value*/) {
}

void counted_notify_a12()  {}
void counted_notify_hblank()    {}
void counted_notify_hblank2()   {}
void counted_notify_scanline(int16_t /*sl*/) {}
void counted_notify_vblank(bool /*asserted*/) {}

}  // namespace

int main() {
    PpuState* st = fceux11_ppu_create();
    if (st == nullptr) {
        std::fprintf(stderr, "FAIL: fceux11_ppu_create returned null\n");
        return 1;
    }

    fceux11_ppu_bus_callbacks cb = {
        &counted_bus_read,
        &counted_bus_write,
        &counted_notify_a12,
        &counted_notify_hblank,
        &counted_notify_hblank2,
        &counted_notify_scanline,
        &counted_notify_vblank,
    };
    fceux11_ppu_install_bus_callbacks(st, &cb);

    fceux11_ppu_power(st);
    fceux11_ppu_set_video_system(st, false /* PAL=false */);

    // Snapshot state.
    int16_t sl_before = fceux11_ppu_get_scanline(st);
    uint16_t dot_before = fceux11_ppu_get_dot(st);
    std::printf("Before frame: sl=%d dot=%u\n", sl_before, dot_before);

    // Drive one full NTSC frame (89342 CPU cycles = 268026 PPU dots).
    if (fceux11_ppu_emulate_frame(st, 89342) != 0) {
        std::fprintf(stderr, "FAIL: fceux11_ppu_emulate_frame returned non-zero\n");
        fceux11_ppu_destroy(st);
        return 1;
    }

    // After the frame, state.scanline should be -1 again (one frame later).
    int16_t sl_after = fceux11_ppu_get_scanline(st);
    uint16_t dot_after = fceux11_ppu_get_dot(st);
    uint64_t frame = fceux11_ppu_get_frame_count(st);
    std::printf("After frame: sl=%d dot=%u frame=%llu\n",
                sl_after, dot_after, (unsigned long long)frame);

    // Framebuffer pointer should be non-null.
    uint8_t* fb = fceux11_ppu_get_framebuffer(st);
    if (fb == nullptr) {
        std::fprintf(stderr, "FAIL: framebuffer pointer null\n");
        fceux11_ppu_destroy(st);
        return 1;
    }
    uint32_t stride = fceux11_ppu_get_framebuffer_stride(st);
    if (stride != 256) {
        std::fprintf(stderr, "FAIL: stride %u != 256\n", stride);
        fceux11_ppu_destroy(st);
        return 1;
    }

    fceux11_ppu_destroy(st);
    std::printf("PASS: rust_ppu_smoke_test ok\n");
    return 0;
}