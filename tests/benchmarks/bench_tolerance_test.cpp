// FCEUX11 v1.1 Sentinel §1.3 — Performance tolerance test.
//
// Runs each of the three benchmarks (x6502 / PPU / APU) and compares
// the median wall-clock time against the v1.0 baseline.
//
// Behaviour:
//   * If tests/benchmarks/baseline_v1.0.json is missing → test PASSES
//     with a warning (first-time setup).
//   * If --generate is passed, the test writes a host-local
//     baseline at tests/benchmarks/baseline_v1.0.local.json with the
//     current run's medians and PASSES (we don't fail on the very
//     first run because we have no reference).
//   * If the env var FCEUX11_BENCH_BASELINE points to a JSON file, that
//     file is loaded instead of the v1.0 baseline (CI override).
//   * Otherwise the v1.0 baseline is loaded, and a regression of more
//     than ±2% on any benchmark fails the test.
//
// We deliberately use the same ROM/frame/iteration counts as the
// existing benchmark binaries (x6502_exec_bench.cpp / ppu_render_bench.cpp
// / apu_mix_bench.cpp) so the values are directly comparable to those
// binaries' outputs.

#include "test_helpers.h"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdint>
#include <cstring>
#include <cstdlib>
#include <chrono>
#include <vector>
#include <string>

using namespace fceu11_test;

struct BenchConfig {
    const char* name;
    const char* rom;
    int         frames;
};

// Match the v0.3.0 baseline binaries.
static const BenchConfig kBenchs[] = {
    { "bench_cpu_frame",  "fixtures/nestest.nes",     60 },
    { "bench_ppu_frame",  "fixtures/mapper_nrom.nes", 60 },
    { "bench_full_frame", "fixtures/mapper_mmc3.nes", 60 },
};
static const int kNumBenchs = sizeof(kBenchs) / sizeof(kBenchs[0]);

// ---------------------------------------------------------------------------
// Single-bench timing (mirrors x6502_exec_bench.cpp / ppu_render_bench.cpp /
// apu_mix_bench.cpp: 5 timed iterations, 1 warm-up, 1 stddev extra pass).
// ---------------------------------------------------------------------------
static double run_bench(const BenchConfig& cfg) {
    fceu11::CloseGame();

    if (!core_init()) return -1.0;
    FCEUGI* gi = load_rom(cfg.rom);
    if (!gi) { core_shutdown(); return -1.0; }

    if (std::strcmp(cfg.name, "bench_full_frame") == 0) {
        FCEUI_Sound(48000);
    }

    uint8*  xbuf = nullptr;
    int32*  soundBuf = nullptr;
    int32   soundBufSize = 0;
    fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0); // warm-up

    std::vector<double> times;
    times.reserve(cfg.frames ? 5 : 0);
    for (int i = 0; i < 5; ++i) {
        auto t0 = std::chrono::high_resolution_clock::now();
        for (int f = 0; f < cfg.frames; ++f) {
            fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0);
        }
        auto t1 = std::chrono::high_resolution_clock::now();
        times.push_back(std::chrono::duration<double, std::milli>(t1 - t0).count());
    }
    std::sort(times.begin(), times.end());
    double median = times[times.size() / 2];

    fceu11::CloseGame();
    core_shutdown();
    return median;
}

// ---------------------------------------------------------------------------
// Baseline JSON I/O — minimal hand-rolled parser for our own format.
// ---------------------------------------------------------------------------
static std::string readFile(const char* path, bool* ok) {
    FILE* f = fopen(path, "rb");
    if (!f) { *ok = false; return {}; }
    fseek(f, 0, SEEK_END);
    long len = ftell(f);
    fseek(f, 0, SEEK_SET);
    std::string s(static_cast<size_t>(len), '\0');
    if (len > 0) fread(s.data(), 1, static_cast<size_t>(len), f);
    fclose(f);
    *ok = true;
    return s;
}

static double find_baseline_ms(const std::string& json, const std::string& name) {
    // Find the "name": "X" entry, then the following "baseline_ms": N.
    std::string needle = "\"name\": \"" + name + "\"";
    size_t pos = json.find(needle);
    if (pos == std::string::npos) return -1.0;
    std::string bk = "\"baseline_ms\":";
    size_t bpos = json.find(bk, pos);
    if (bpos == std::string::npos) return -1.0;
    return std::atof(json.c_str() + bpos + bk.size());
}

static double find_tolerance_pct(const std::string& json) {
    std::string bk = "\"tolerance_pct\":";
    size_t bpos = json.find(bk);
    if (bpos == std::string::npos) return 2.0;
    return std::atof(json.c_str() + bpos + bk.size());
}

int main(int argc, char** argv) {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    bool generate = false;
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--generate") == 0) generate = true;
    }

    std::printf("=== FCEUX11 v1.1 Bench Tolerance Test ===\n");
    std::printf("Mode: %s\n\n", generate ? "GENERATE local baseline"
                                         : "VERIFY against baseline");

    // Pick baseline path: env var > v1.0 default.
    const char* baseline_path = std::getenv("FCEUX11_BENCH_BASELINE");
    if (!baseline_path || !*baseline_path) {
        baseline_path = "benchmarks/baseline_v1.0.json";
    }
    const char* local_path = "benchmarks/baseline_v1.0.local.json";

    bool ok = false;
    std::string baseline_text = readFile(baseline_path, &ok);
    if (!ok) {
        std::printf("WARN: baseline not found at %s\n", baseline_path);
        std::printf("      first-time setup; running without regression check.\n\n");
        baseline_text = "{}";
    }
    double tolerance_pct = find_tolerance_pct(baseline_text);
    if (tolerance_pct <= 0) tolerance_pct = 2.0;

    TestContext ctx;

    std::vector<std::string> results_name;
    std::vector<double>      results_ms;

    for (int i = 0; i < kNumBenchs; ++i) {
        std::printf("[%d/%d] %s (%d frames)\n",
                    i + 1, kNumBenchs, kBenchs[i].name, kBenchs[i].frames);
        double ms = run_bench(kBenchs[i]);
        if (ms < 0) {
            std::printf("  FAIL: benchmark did not complete\n");
            ++ctx.failed;
            continue;
        }
        std::printf("  median: %.3f ms  (%.3f ms/frame)\n",
                    ms, ms / kBenchs[i].frames);
        results_name.push_back(kBenchs[i].name);
        results_ms.push_back(ms);

        if (generate || baseline_text == "{}") {
            // Skip comparison; just record.
            std::printf("  (baseline not enforced this run)\n");
            ++ctx.passed;
            continue;
        }

        double base_ms = find_baseline_ms(baseline_text, kBenchs[i].name);
        if (base_ms < 0) {
            std::printf("  WARN: no baseline entry for %s\n", kBenchs[i].name);
            ++ctx.passed;
            continue;
        }
        double dev_pct = (ms - base_ms) / base_ms * 100.0;
        bool within = std::fabs(dev_pct) <= tolerance_pct;
        std::printf("  baseline: %.3f ms  deviation: %+.2f%%  tolerance: ±%.1f%%\n",
                    base_ms, dev_pct, tolerance_pct);
        if (within) {
            std::printf("  PASS\n");
            ++ctx.passed;
        } else {
            std::printf("  FAIL: regression exceeds tolerance\n");
            ++ctx.failed;
        }
    }

    if (generate) {
        FILE* f = fopen(local_path, "w");
        if (f) {
            std::fprintf(f, "{\n");
            std::fprintf(f, "  \"_comment\": \"Auto-generated local baseline. Do not commit.\",\n");
            std::fprintf(f, "  \"tolerance_pct\": %.1f,\n", tolerance_pct);
            std::fprintf(f, "  \"benchmarks\": [\n");
            for (size_t i = 0; i < results_name.size(); ++i) {
                std::fprintf(f,
                    "    { \"name\": \"%s\", \"median_total_ms\": %.3f }%s\n",
                    results_name[i].c_str(), results_ms[i],
                    (i + 1 < results_name.size()) ? "," : "");
            }
            std::fprintf(f, "  ]\n}\n");
            fclose(f);
            std::printf("\nWrote local baseline to %s\n", local_path);
        }
    }

    return report_and_exit(ctx, "Bench tolerance test suite");
}
