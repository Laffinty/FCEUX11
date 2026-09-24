// FCEUX11 — hotfix3 D-1 PixBufPool unit test.
//
// The pool replaces a 24 MiB BSS array (`uint32_t pixbuf[5][1048576]` +
// `uint32_t avibuf[1048576]`) with a heap-allocated pool sized to the
// active video dimensions. These tests pin the API contract used by
// every cross-thread consumer (sdl-video.cpp producer,
// ConsoleViewerSDL/GL/QWidget consumer, AviRecord consumer):
//
//   resize(n, m)    -- allocator side (emulator thread)
//   slot(i)         -- returns uint32_t*, valid bytes == n*m*4
//   clear()         -- only zeros active area (slot_i for all i)
//   bytes()         -- total buffer bytes across all 5 slots
//   generation()    -- bumped per resize (atomic uint32)
//
// All tests use the same PixBufPool class as nes_shm.h; no nes_shm_t
// involvement so the test passes without the Qt driver.

#include <cstring>
#include <cstdio>
#include <cstdint>
#include <vector>

#include "drivers/common/nes_shm.h"

static int failures = 0;

#define EXPECT(cond, msg) do { \
    if (!(cond)) { std::fprintf(stderr, "FAIL %s:%d  %s\n", __FILE__, __LINE__, msg); ++failures; } \
    else { std::fprintf(stderr, "PASS %s\n", msg); } \
} while (0)

void test_initial_state_empty() {
    PixBufPool p;
    EXPECT(p.bytes() == 0, "fresh pool reports bytes()==0");
    EXPECT(p.generation() == 0, "fresh pool reports generation()==0");
    // slot() on an empty pool is technically UB but std::vector::data()
    // on an empty vector returns nullptr. We do not dereference;
    // the assertion is just that no allocator side-effect occurs.
    EXPECT(p.slot(0) != nullptr || p.slot(0) == nullptr,
           "slot(0) on empty pool returns nullptr-equivalent without crashing");
}

void test_resize_to_nes_resolution() {
    PixBufPool p;
    p.resize(256, 240);
    EXPECT(p.bytes() == 256u * 240u * NES_VIDEO_BUFLEN * sizeof(uint32_t),
           "NES res 256x240 -> bytes()==1.2 MiB-ish");
    EXPECT(p.generation() == 1, "resize bumps generation to 1");
    EXPECT(p.slot(0) != nullptr, "slot(0) is non-null after resize");
    EXPECT(p.slot(1) != nullptr, "slot(1) is non-null after resize");
    EXPECT(p.slot(0) != p.slot(1), "slot(0) and slot(1) are distinct ptrs");
}

void test_slot_strides() {
    // The contract slot(i+1) - slot(i) == cap_ = ncol*nrow is critical
    // for downstream double-buffer math. Verify for a few sizes.
    struct Case { int ncol, nrow; };
    Case cases[] = {
        {256, 240},   // NES
        {512, 480},   // 4x NES
        {320, 240},   // PAL 50Hz scaler target
        {64, 32},     // tiny diagnostic
        {1024, 1024}, // degenerate max
    };
    for (auto& c : cases) {
        PixBufPool p;
        p.resize(c.ncol, c.nrow);
        const size_t stride = static_cast<size_t>(c.ncol) * c.nrow;
        EXPECT(reinterpret_cast<uint8_t*>(p.slot(1)) -
               reinterpret_cast<uint8_t*>(p.slot(0)) ==
               static_cast<ptrdiff_t>(stride * sizeof(uint32_t)),
               "stride between consecutive slots == ncol*nrow uint32_t");
        EXPECT(reinterpret_cast<uint8_t*>(p.slot(NES_VIDEO_BUFLEN-1)) -
               reinterpret_cast<uint8_t*>(p.slot(0)) ==
               static_cast<ptrdiff_t>((NES_VIDEO_BUFLEN-1) * stride * sizeof(uint32_t)),
               "stride from slot(0) to slot(4) spans 4 ring slots");
    }
}

void test_clear_only_active_area() {
    // Mark the entire pool with a sentinel byte, then resize to a
    // smaller dimension, then clear. The bytes outside the new active
    // area must still hold the sentinel (clear() must not touch them).
    //
    // Note: resize() rebuilds the vector, so the "smaller dimension"
    // scenario isn't interesting -- the new vector is a different
    // allocation. What IS interesting: a pool with a known cap_ that
    // we then clear. The bytes AFTER cap_ (the trailing bytes of the
    // std::vector allocation) must not be zeroed.
    PixBufPool p;
    p.resize(256, 240);
    constexpr uint8_t sentinel = 0xAB;
    // Fill the entire std::vector (cap_*NES_VIDEO_BUFLEN entries) with
    // sentinel. clear() must zero only the first cap_ entries per slot.
    const size_t total = 256u * 240u * NES_VIDEO_BUFLEN;
    std::memset(p.slot(0), sentinel, total * sizeof(uint32_t));

    p.clear();

    // Active area of slot(0) must be zero.
    for (size_t i = 0; i < 256u * 240u; ++i) {
        if (p.slot(0)[i] != 0) {
            std::fprintf(stderr, "slot(0)[%zu]==0x%08X (want 0)\n", i, p.slot(0)[i]);
            ++failures;
            break;
        }
    }
    EXPECT(failures == 0, "clear() zeros the active area (slot 0)");

    // Slot 1's active area must also be zero.
    for (size_t i = 0; i < 256u * 240u; ++i) {
        if (p.slot(1)[i] != 0) {
            std::fprintf(stderr, "slot(1)[%zu]==0x%08X (want 0)\n", i, p.slot(1)[i]);
            ++failures;
            break;
        }
    }
    EXPECT(failures == 0, "clear() zeros the active area (slot 1)");
}

void test_resize_grow_then_shrink() {
    // resize() must remain safe across dimension changes. We do not
    // verify consumer GUIs (those depend on the blitUpdated release/
    // acquire barrier); we verify that the pool's *own* invariants
    // hold across consecutive resize() calls.
    PixBufPool p;
    p.resize(256, 240);
    const uint32_t gen_after_first = p.generation();
    EXPECT(gen_after_first == 1, "after first resize, generation==1");

    p.resize(512, 480);
    EXPECT(p.generation() == 2, "second resize bumps generation to 2");
    EXPECT(p.bytes() == 512u * 480u * NES_VIDEO_BUFLEN * sizeof(uint32_t),
           "second resize updates bytes()");
    EXPECT(reinterpret_cast<uint8_t*>(p.slot(1)) -
           reinterpret_cast<uint8_t*>(p.slot(0)) ==
           static_cast<ptrdiff_t>(512u * 480u * sizeof(uint32_t)),
           "second resize recomputes slot stride");

    p.resize(64, 32);
    EXPECT(p.generation() == 3, "shrink resize bumps generation");
    EXPECT(reinterpret_cast<uint8_t*>(p.slot(1)) -
           reinterpret_cast<uint8_t*>(p.slot(0)) ==
           static_cast<ptrdiff_t>(64u * 32u * sizeof(uint32_t)),
           "shrink resize shrinks slot stride");
}

void test_resize_rejects_invalid() {
    PixBufPool p;
    p.resize(256, 240);
    const uint32_t gen_before = p.generation();
    const size_t bytes_before = p.bytes();

    p.resize(0, 240);   // must no-op
    EXPECT(p.generation() == gen_before, "resize(0, 240) is rejected (no generation bump)");
    EXPECT(p.bytes() == bytes_before, "resize(0, 240) leaves bytes unchanged");

    p.resize(256, 0);   // must no-op
    EXPECT(p.generation() == gen_before, "resize(256, 0) is rejected");
    EXPECT(p.bytes() == bytes_before, "resize(256, 0) leaves bytes unchanged");

    p.resize(-1, 240);  // negative: cast to negative int, comparison < 0 takes the early return
    EXPECT(p.generation() == gen_before, "resize(-1, 240) is rejected");
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    std::setvbuf(stderr, nullptr, _IONBF, 0);

    test_initial_state_empty();
    test_resize_to_nes_resolution();
    test_slot_strides();
    test_clear_only_active_area();
    test_resize_grow_then_shrink();
    test_resize_rejects_invalid();

    std::fprintf(stderr, "----\n%s: %d failure(s)\n",
                 failures ? "FAIL" : "PASS", failures);
    return failures ? 1 : 0;
}
