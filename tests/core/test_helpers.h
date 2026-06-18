// FCEUX11 v1.1 Sentinel — shared test helpers for tests/core/.
//
// Provides:
//   - core_init(): bring the engine up, set dummy inputs, return true on
//     success. Idempotent within a single process (uses an internal once-flag
//     tracked via the Initialize/Kill pair; if the caller wants a fresh
//     instance they should pair core_init() with core_shutdown()).
//   - core_shutdown(): call after the test body to drop global state.
//   - load_rom(rom_path): wraps fceu11::LoadGame with a check for null.
//   - emulate_n(n): run n frames, accumulating into the global sound/video
//     buffers (re-uses the same call shape as rom_regression_test.cpp).
//   - xbuf(), sound_buf(), sound_buf_size(): accessors for the most recent
//     Emulate call's outputs (the underlying pointers are owned by the
//     engine; the test only reads from them while no further Emulate call
//     is in flight).
//
// All helpers deliberately stay header-only and inline — v1.1 forbids
// source-code changes, so we add no new .cpp file in src/.

#pragma once

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cstdint>
#include <atomic>
#include <string>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "x6502.h"
#include "ppu.h"
#include "sound.h"
#include "cart.h"
#include "state.h"
#include "emufile.h"
#include "utils/md5.h"
#include "drivers/Qt/nes_shm.h"

namespace fceu11_test {

// ---------------------------------------------------------------------------
// Process-wide state
// ---------------------------------------------------------------------------
inline int   g_last_sound_buf_size = 0;
inline int32* g_last_sound_buf = nullptr;
inline uint8* g_last_xbuf = nullptr;
inline std::atomic<int> g_init_count{0};

// ---------------------------------------------------------------------------
// core_init / core_shutdown
// ---------------------------------------------------------------------------
inline bool core_init() {
    if (g_init_count.fetch_add(1) == 0) {
        if (!fceu11::Initialize()) {
            std::fprintf(stderr, "core_init: fceu11::Initialize() failed\n");
            g_init_count.store(0);
            return false;
        }
        if (!nes_shm) {
            nes_shm = open_nes_shm();
        }
        FCEUI_SetInput(0,    static_cast<ESI>(SI_NONE),    nullptr, 0);
        FCEUI_SetInput(1,    static_cast<ESI>(SI_NONE),    nullptr, 0);
        FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE),    nullptr, 0);
        FCEUI_SetInputFourscore(false);
    }
    return true;
}

inline void core_shutdown() {
    if (g_init_count.fetch_sub(1) <= 1) {
        g_init_count.store(0);
        fceu11::Kill();
    }
}

// ---------------------------------------------------------------------------
// ROM loading
// ---------------------------------------------------------------------------
inline FCEUGI* load_rom(const char* path) {
    FCEUGI* gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        std::fprintf(stderr, "load_rom: failed to load %s\n", path);
    }
    return gi;
}

// ---------------------------------------------------------------------------
// Emulation driver
// ---------------------------------------------------------------------------
inline void emulate_one() {
    fceu11::Emulate(&g_last_xbuf, &g_last_sound_buf, &g_last_sound_buf_size, 0);
}

inline void emulate_n(int n) {
    for (int i = 0; i < n; ++i) {
        emulate_one();
    }
}

inline uint8*  xbuf()         { return g_last_xbuf; }
inline int32_t* sound_buf()   { return g_last_sound_buf; }
inline int     sound_buf_size() { return g_last_sound_buf_size; }

// ---------------------------------------------------------------------------
// Trivial PASS/FAIL accounting used by the v1.1 test files.
// ---------------------------------------------------------------------------
struct TestContext {
    int passed = 0;
    int failed = 0;
    const char* current_test = nullptr;
};

inline int report_and_exit(TestContext& ctx, const char* suite_name) {
    std::printf("\n=== %s ===\n", suite_name);
    std::printf("Passed:    %d\n", ctx.passed);
    std::printf("Failed:    %d\n", ctx.failed);
    std::printf("Total:     %d\n", ctx.passed + ctx.failed);
    if (ctx.failed == 0) {
        std::printf("RESULT:    PASSED\n");
        return 0;
    }
    std::printf("RESULT:    FAILED\n");
    return 1;
}

inline void check_true(TestContext& ctx, bool cond, const char* label) {
    if (cond) {
        std::printf("  ok    %s\n", label);
        ++ctx.passed;
    } else {
        std::printf("  FAIL  %s\n", label);
        ++ctx.failed;
    }
}

#define FCEU11_EXPECT(ctx, cond, label)                                          \
    do {                                                                          \
        fceu11_test::check_true((ctx), (cond), (label));                          \
    } while (0)

}  // namespace fceu11_test
