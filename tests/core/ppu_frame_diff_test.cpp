// FCEUX11 v1.5 Prism §5.3 — visual frame-diff regression test.
//
// Goal: pin down PPU rendering output to a byte-exact golden, so any v1.5
// refactor of the PPU (Ppu class, Bus->PPU decoupling, internal state
// migration) that drifts a single pixel will fail the test suite.
//
// Per ROM:
//   1. fceu11::Initialize / LoadGame
//   2. Run N frames (per docs/internal/v1.5_prism_build_plan.md §4.1)
//   3. After the last frame, snapshot the visible 256x240 portion of XBuf
//      (61440 bytes of NES palette indices + deemphasis bits)
//   4. In GENERATE mode: write the bytes to fixtures/golden_frames/<rom>.xbuf
//      In VERIFY mode:   read the golden file and memcmp against the snapshot
//
// Golden file format:
//   Plain binary, 61440 bytes (256 * 240), no header. The dimensions are
//   fixed and documented here. We deliberately do not use PNG/PPM for the
//   canonical diff target — PNG encoding/decoding round-trips introduce
//   lossy or non-deterministic channels, and PPM's text header breaks
//   byte-level memcmp. The raw XBuf format is the most direct possible
//   representation of what PPU produced; the diff is a flat memcmp.
//
// Run with --generate to write fresh goldens (run once after the v1.4
// baseline is locked; rerun ONLY when an intentional PPU change lands).

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <string>
#include <chrono>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "state.h"
#include "drivers/common/nes_shm.h"

static const int FRAME_WIDTH  = 256;
static const int FRAME_HEIGHT = 240;
static const int FRAME_BYTES  = FRAME_WIDTH * FRAME_HEIGHT;
static const double WATCHDOG_SECONDS_PER_FRAME = 30.0;
static const char*  GOLDEN_DIR = "fixtures/golden_frames";

struct RomTestCase {
    const char* filename;
    const char* name;
    int         frames;  // frame index at which to snapshot XBuf
};

static const RomTestCase tests[] = {
    // docs/internal/v1.5_prism_build_plan.md §4.1
    { "fixtures/mapper_nrom.nes",  "nrom",  60  },
    { "fixtures/mapper_mmc3.nes",  "mmc3",  120 },
    { "fixtures/mapper_mmc1.nes",  "mmc1",  90  },
    { "fixtures/mapper_vrc6.nes",  "vrc6",  60  },
    { "fixtures/mapper_mmc5.nes",  "mmc5",  90  },
};

static const int NUM_TESTS = sizeof(tests) / sizeof(tests[0]);

// ---------------------------------------------------------------------------
// Per-ROM: load, run, snapshot XBuf
// ---------------------------------------------------------------------------

static bool snapshotFrame(const char* romPath, int framesToRun,
                          std::vector<uint8_t>& out)
{
    if (!fceu11::Initialize()) {
        std::fprintf(stderr, "FCEUI_Initialize failed for %s\n", romPath);
        return false;
    }
    close_nes_shm();
    nes_shm = open_nes_shm();

    // Match the savestate_regression_test hygiene: disable interactive /
    // GUI-only features that can block on a headless CI runner.
    AutoResumePlay = false;
    FCEU_StateRecorderSetEnabled(false);
    FCEUI_SetInput(0,   static_cast<ESI>(SI_NONE),    nullptr, 0);
    FCEUI_SetInput(1,   static_cast<ESI>(SI_NONE),    nullptr, 0);
    FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE),   nullptr, 0);
    FCEUI_SetInputFourscore(false);

    if (!fceu11::LoadGame(romPath, 1, true)) {
        std::fprintf(stderr, "LoadGame failed for %s\n", romPath);
        fceu11::Kill();
        return false;
    }

    uint8*  xbuf  = nullptr;
    int32*  sbuf  = nullptr;
    int32   ssize = 0;
    for (int i = 0; i < framesToRun; ++i) {
        auto t0 = std::chrono::steady_clock::now();
        fceu11::Emulate(&xbuf, &sbuf, &ssize, 0);
        auto dt = std::chrono::steady_clock::now() - t0;
        double secs = std::chrono::duration<double>(dt).count();
        if (secs > WATCHDOG_SECONDS_PER_FRAME) {
            std::fprintf(stderr,
                "WATCHDOG: %s frame %d took %.1fs (limit %.1fs)\n",
                romPath, i, secs, WATCHDOG_SECONDS_PER_FRAME);
            fceu11::CloseGame();
            fceu11::Kill();
            return false;
        }
    }

    out.assign(FRAME_BYTES, 0);
    if (xbuf) {
        // XBuf is 256x256; the visible area is the first 256*240 bytes.
        std::memcpy(out.data(), xbuf, FRAME_BYTES);
    } else {
        std::fprintf(stderr, "XBuf null after Emulate for %s\n", romPath);
        fceu11::CloseGame();
        fceu11::Kill();
        return false;
    }

    fceu11::CloseGame();
    fceu11::Kill();
    return true;
}

// ---------------------------------------------------------------------------
// Golden file I/O
// ---------------------------------------------------------------------------

static std::string goldenPath(const char* name) {
    std::string p = GOLDEN_DIR;
    p += "/";
    p += name;
    p += ".xbuf";
    return p;
}

static bool writeGolden(const char* path, const std::vector<uint8_t>& bytes) {
    FILE* f = std::fopen(path, "wb");
    if (!f) {
        std::fprintf(stderr, "Cannot open golden for write: %s\n", path);
        return false;
    }
    size_t wrote = std::fwrite(bytes.data(), 1, bytes.size(), f);
    std::fclose(f);
    if (wrote != bytes.size()) {
        std::fprintf(stderr, "Short write on %s (%zu/%zu)\n",
                     path, wrote, bytes.size());
        return false;
    }
    return true;
}

static bool readGolden(const char* path, std::vector<uint8_t>& out) {
    FILE* f = std::fopen(path, "rb");
    if (!f) return false;
    std::fseek(f, 0, SEEK_END);
    long len = std::ftell(f);
    std::fseek(f, 0, SEEK_SET);
    if (len != FRAME_BYTES) {
        std::fprintf(stderr,
            "Golden %s has wrong size: %ld bytes (expected %d)\n",
            path, len, FRAME_BYTES);
        std::fclose(f);
        return false;
    }
    out.assign(FRAME_BYTES, 0);
    size_t got = std::fread(out.data(), 1, FRAME_BYTES, f);
    std::fclose(f);
    return got == static_cast<size_t>(FRAME_BYTES);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

int main(int argc, char** argv)
{
    setvbuf(stdout, NULL, _IONBF, 0);

    bool generateMode = false;
    if (argc > 1 && std::strcmp(argv[1], "--generate") == 0) {
        generateMode = true;
    }

    std::printf("=== FCEUX11 v1.5 PPU Visual Frame-Diff Test ===\n");
    std::printf("Mode: %s\n\n",
                generateMode ? "GENERATE goldens" : "VERIFY against goldens");

    int failures = 0;
    for (int i = 0; i < NUM_TESTS; ++i) {
        const RomTestCase& tc = tests[i];
        std::printf("[%d/%d] %-5s (%s, frame %d)\n",
                    i + 1, NUM_TESTS, tc.name, tc.filename, tc.frames);

        std::vector<uint8_t> frame;
        if (!snapshotFrame(tc.filename, tc.frames, frame)) {
            std::printf("  RESULT: snapshot failed\n");
            ++failures;
            continue;
        }
        std::printf("  snapshot: %zu bytes\n", frame.size());

        std::string path = goldenPath(tc.name);

        if (generateMode) {
            if (!writeGolden(path.c_str(), frame)) {
                std::printf("  RESULT: golden write failed\n");
                ++failures;
                continue;
            }
            std::printf("  wrote golden: %s\n", path.c_str());
            continue;
        }

        std::vector<uint8_t> golden;
        if (!readGolden(path.c_str(), golden)) {
            std::printf("  RESULT: missing golden %s (run --generate first)\n",
                        path.c_str());
            ++failures;
            continue;
        }

        // Byte-exact memcmp — plan §5.3 mandates 0-pixel difference.
        if (std::memcmp(golden.data(), frame.data(), FRAME_BYTES) != 0) {
            // Locate first diverging byte for actionable diagnostics.
            int first_diff = -1;
            for (int j = 0; j < FRAME_BYTES; ++j) {
                if (golden[j] != frame[j]) { first_diff = j; break; }
            }
            int diff_count = 0;
            for (int j = 0; j < FRAME_BYTES; ++j) {
                if (golden[j] != frame[j]) ++diff_count;
            }
            std::printf("  RESULT: MISMATCH — %d / %d pixels differ "
                        "(first at offset %d: golden=%02X actual=%02X)\n",
                        diff_count, FRAME_BYTES, first_diff,
                        golden[first_diff], frame[first_diff]);
            ++failures;
        } else {
            std::printf("  ok 0-pixel diff vs golden\n");
        }
    }

    std::printf("\n=== Results ===\n");
    std::printf("ROMs:    %d\n", NUM_TESTS);
    std::printf("Failed:  %d\n", failures);
    if (failures == 0) {
        std::printf("RESULT:  PASSED\n");
        return 0;
    }
    std::printf("RESULT:  FAILED\n");
    return 1;
}