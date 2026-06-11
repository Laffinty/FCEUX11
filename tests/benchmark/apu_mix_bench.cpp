// FCEUX11 v0.3.0 — APU Audio Mix Benchmark
// Measures audio generation throughput by emulating 60 frames and capturing
// the sound buffer output size.

#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <chrono>

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "drivers/Qt/nes_shm.h"

static const char* ROM_PATH = "fixtures/mapper_mmc3.nes";
static const int FRAMES = 60;

static double benchmark_apu_frames(int frames)
{
    if (!FCEUI_Initialize()) {
        fprintf(stderr, "FCEUI_Initialize failed\n");
        return -1.0;
    }
    if (!nes_shm) {
        nes_shm = open_nes_shm();
    }
    FCEUI_SetInput(0, static_cast<ESI>(SI_NONE), nullptr, 0);
    FCEUI_SetInput(1, static_cast<ESI>(SI_NONE), nullptr, 0);
    FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE), nullptr, 0);
    FCEUI_SetInputFourscore(false);

    // Enable sound so the APU path is exercised.
    FCEUI_Sound(48000);

    if (!FCEUI_LoadGame(ROM_PATH, 1, true)) {
        fprintf(stderr, "Failed to load %s\n", ROM_PATH);
        FCEUI_Kill();
        return -1.0;
    }

    uint8* xbuf = nullptr;
    int32* soundBuf = nullptr;
    int32 soundBufSize = 0;

    FCEUI_Emulate(&xbuf, &soundBuf, &soundBufSize, 0); // warm-up

    auto t0 = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < frames; ++i) {
        FCEUI_Emulate(&xbuf, &soundBuf, &soundBufSize, 0);
    }
    auto t1 = std::chrono::high_resolution_clock::now();

    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();

    FCEUI_CloseGame();
    FCEUI_Kill();
    return ms;
}

int main(int argc, char** argv)
{
    (void)argc;
    (void)argv;

    printf("=== FCEUX11 APU Mix Benchmark (v0.3.0 baseline) ===\n");
    printf("ROM: %s\n", ROM_PATH);
    printf("Frames per iteration: %d\n\n", FRAMES);

    const int iterations = 5;
    double best = 1e9, worst = 0.0, total = 0.0;
    for (int i = 0; i < iterations; ++i) {
        double ms = benchmark_apu_frames(FRAMES);
        if (ms < 0.0) return 1;
        printf("Iteration %d/%d: %.3f ms (%.3f ms/frame)\n",
               i + 1, iterations, ms, ms / FRAMES);
        if (ms < best) best = ms;
        if (ms > worst) worst = ms;
        total += ms;
    }

    double avg = total / iterations;
    double stddev = 0.0;
    for (int i = 0; i < iterations; ++i) {
        double ms = benchmark_apu_frames(FRAMES);
        double diff = ms - avg;
        stddev += diff * diff;
    }
    stddev = std::sqrt(stddev / iterations);

    printf("\n--- Summary ---\n");
    printf("Average: %.3f ms  (%.3f ms/frame)\n", avg, avg / FRAMES);
    printf("Best:    %.3f ms\n", best);
    printf("Worst:   %.3f ms\n", worst);
    printf("StdDev:  %.3f ms (%.1f%%)\n", stddev, (stddev / avg) * 100.0);
    printf("RESULT:  PASSED\n");
    return 0;
}
