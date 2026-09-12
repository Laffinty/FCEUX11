// FCEUX11 v2.1.1.7 Step B.5-2d - PAL/Dendy timing gate.
//
// Three checks:
//   1. the bridge's frame-length getter follows the selected region
//      (89342 NTSC / 106392 PAL / 106392 Dendy);
//   2. `FCEUPPU_SetVideoSystem` actually reaches the bridge (the
//      production wiring, not just the setter);
//   3. a ROM runs 600 frames in PAL mode without crashing.
//
// The fixture ROM corpus is gitignored (AGENTS.md gotcha 5), so check 3
// reports SKIP when the ROM is absent; checks 1-2 always run.

#include <cstdio>
#include <cstdint>

extern "C" {
#include "kagami_bridge.h"
}

#include "ppu.h"              // FCEUPPU_SetVideoSystem
#include "ppu_rust_bridge.h"  // ppu_rust_bridge_set_video_system / _dots_per_frame

namespace {

int g_failures = 0;

void Check(bool cond, const char* what) {
    if (!cond) {
        std::fprintf(stderr, "FAIL: %s\n", what);
        ++g_failures;
    } else {
        std::printf("ok: %s\n", what);
    }
}

bool FileExists(const char* path) {
    FILE* f = std::fopen(path, "rb");
    if (f == nullptr) {
        return false;
    }
    std::fclose(f);
    return true;
}

}  // namespace

int main() {
    // ---- 1. bridge getter follows the region ----------------------------
    ppu_rust_bridge_set_video_system(false, false);
    Check(ppu_rust_bridge_ppu_dots_per_frame() == 89342u, "NTSC frame = 89342 dots");
    ppu_rust_bridge_set_video_system(true, false);
    Check(ppu_rust_bridge_ppu_dots_per_frame() == 106392u, "PAL frame = 106392 dots");
    ppu_rust_bridge_set_video_system(false, true);
    Check(ppu_rust_bridge_ppu_dots_per_frame() == 106392u, "Dendy frame = 106392 dots");
    ppu_rust_bridge_set_video_system(false, false);

    // ---- 2. production wiring: FCEUPPU_SetVideoSystem -> bridge ----------
    FCEUPPU_SetVideoSystem(1);
    Check(ppu_rust_bridge_ppu_dots_per_frame() == 106392u,
          "FCEUPPU_SetVideoSystem(1) selects the PAL frame budget");
    FCEUPPU_SetVideoSystem(0);
    Check(ppu_rust_bridge_ppu_dots_per_frame() == 89342u,
          "FCEUPPU_SetVideoSystem(0) selects the NTSC frame budget");

    // ---- 3. PAL 600-frame smoke -----------------------------------------
    const char* rom = "fixtures/nestest.nes";
    if (!FileExists(rom)) {
        std::printf("SKIP: %s not present (gitignored ROM corpus)\n", rom);
        return (g_failures == 0) ? 0 : 1;
    }

    if (kagami_bridge_init() != 0) {
        std::fprintf(stderr, "FAIL: kagami_bridge_init\n");
        return 1;
    }
    if (kagami_bridge_load_rom(rom) != 0) {
        std::fprintf(stderr, "FAIL: kagami_bridge_load_rom('%s')\n", rom);
        kagami_bridge_kill();
        return 1;
    }
    // The load path re-applies the ROM's own region, so switch after it.
    kagami_bridge_set_video_system(1, 0);
    Check(ppu_rust_bridge_ppu_dots_per_frame() == 106392u,
          "PAL frame budget active after kagami_bridge_set_video_system(1, 0)");

    for (int i = 0; i < 600; ++i) {
        if (kagami_bridge_emulate_frame() != 0) {
            std::fprintf(stderr, "FAIL: emulate_frame(%d)\n", i);
            ++g_failures;
            break;
        }
    }
    Check(g_failures == 0, "600 PAL frames ran without error");

    kagami_bridge_kill();

    if (g_failures != 0) {
        std::fprintf(stderr, "pal_timing_test: %d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("PASS: pal_timing_test ok\n");
    return 0;
}