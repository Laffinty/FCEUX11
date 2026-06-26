// FCEUX11 v1.1 Sentinel §1.3 — Performance tolerance test.
//
// Runs each of the three benchmarks (x6502 / PPU / APU) and compares
// the median wall-clock time against the loaded baseline.
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
//   * Otherwise the v1.3/v1.4 baseline (fixtures/bench_baseline.json)
//     is loaded and a **regression** of more than `tolerance_pct` on
//     any benchmark fails the test.
//
// Asymmetric gate (v1.4 Gateway Phase 2): a SPEEDUP (current run
// faster than baseline) is always considered a pass — performance
// improvements are celebrated, not treated as "exceeding tolerance
// in the negative direction". Only a SLOWDOWN (current run slower
// than baseline by more than `tolerance_pct`) is a failure. The
// `tolerance_pct` field in the baseline JSON therefore now means
// "max acceptable regression", not "± deviation".
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
// Measurement configuration. Defaults implement the R4 methodology fix
// (see docs/refactor_plan.md §9.7 #1): the original 1-frame warm-up +
// 5-iteration median was fragile against cold-cache contamination —
// when a hot header changed, the resulting link-time layout shift was
// amplified by the under-warmed first measurement, producing stable
// +4-5% false regressions that disappeared under Google Benchmark's
// long warm-up (§9.3 step 3). The new defaults (3 full warm-up passes
// + 7 timed iterations with the single min/max dropped, median of the
// remaining 5) bring the methodology in line with Google Benchmark's
// `benchmark_min_time` approach while keeping the same effective sample
// count for the median. Both knobs are CLI-tunable so a dedicated
// runner can tighten or loosen the gate.
// ---------------------------------------------------------------------------
struct RunConfig {
    int warmup_iterations = 3;  // full passes of cfg.frames before timing
    int meas_iterations   = 7;  // timed passes; min + max dropped, median of rest
};

// ---------------------------------------------------------------------------
// Single-bench timing. R4 methodology: multi-pass warm-up + 7 timed
// iterations, drop the single min and single max, take the median of
// the remaining samples. For the default 7 iterations this yields a
// median of 5 (same effective sample count as the old 5-iteration
// median) while discarding the most likely cold-cache / scheduling
// outliers.
// ---------------------------------------------------------------------------
static double run_bench(const BenchConfig& cfg, const RunConfig& rc) {
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

    // R4: multi-pass warm-up to load code pages + warm caches before the
    // timed loop. The original 1-frame warm-up left cold-cache residue
    // that contaminated the median of 5 iterations.
    for (int w = 0; w < rc.warmup_iterations; ++w) {
        for (int f = 0; f < cfg.frames; ++f) {
            fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0);
        }
    }

    std::vector<double> times;
    times.reserve(static_cast<size_t>(rc.meas_iterations));
    for (int i = 0; i < rc.meas_iterations; ++i) {
        auto t0 = std::chrono::high_resolution_clock::now();
        for (int f = 0; f < cfg.frames; ++f) {
            fceu11::Emulate(&xbuf, &soundBuf, &soundBufSize, 0);
        }
        auto t1 = std::chrono::high_resolution_clock::now();
        times.push_back(std::chrono::duration<double, std::milli>(t1 - t0).count());
    }
    std::sort(times.begin(), times.end());

    // R4: drop the single min and single max, then take the median of
    // the remaining (n-2) samples. With the default 7 iterations this
    // yields a median of 5. For n < 3 we fall back to the plain median.
    double median;
    const size_t n = times.size();
    if (n >= 3) {
        // remaining slice is [1 .. n-2]; its median sits at index 1 + (n-2)/2.
        median = times[1 + (n - 2) / 2];
    } else if (n > 0) {
        median = times[n / 2];
    } else {
        median = -1.0;
    }

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

// v1.3 Legion Phase 7.3: The hand-rolled parser previously searched
// for the literal substring `"name": "X"` (single space after colon),
// which fails to match the columnar-style JSON where authors align
// values with `"name":             "X"`. The same bug silently
// affected baseline_v1.0.json from the start. The fix below scans
// for the key, then skips any horizontal whitespace (space/tab, but
// NOT newline — we want the value to be on the same line for sane
// diagnostics) before the colon, then any whitespace after the colon.
static const char* skip_hspace(const char* p) {
    while (*p == ' ' || *p == '\t') ++p;
    return p;
}

static double find_baseline_ms(const std::string& json, const std::string& name) {
    // Find the "name" key followed by an optional space-aligned colon.
    // We look for any occurrence of `"name"` and then verify the value.
    const std::string key = "\"name\"";
    size_t pos = 0;
    while ((pos = json.find(key, pos)) != std::string::npos) {
        const char* p = json.c_str() + pos + key.size();
        p = skip_hspace(p);
        if (*p != ':') { ++pos; continue; }
        ++p;
        p = skip_hspace(p);
        if (*p != '"') { ++pos; continue; }
        ++p;
        size_t vlen = 0;
        while (p[vlen] && p[vlen] != '"') ++vlen;
        if (vlen == name.size() && std::memcmp(p, name.data(), vlen) == 0) {
            // Match. Now find the "baseline_ms" key AFTER this position.
            std::string bk = "\"baseline_ms\"";
            size_t bpos = json.find(bk, pos);
            if (bpos == std::string::npos) return -1.0;
            const char* q = json.c_str() + bpos + bk.size();
            q = skip_hspace(q);
            if (*q != ':') return -1.0;
            ++q;
            q = skip_hspace(q);
            return std::atof(q);
        }
        ++pos;
    }
    return -1.0;
}

static double find_tolerance_pct(const std::string& json) {
    std::string bk = "\"tolerance_pct\"";
    size_t bpos = json.find(bk);
    if (bpos == std::string::npos) return 2.0;
    const char* p = json.c_str() + bpos + bk.size();
    p = skip_hspace(p);
    if (*p != ':') return 2.0;
    ++p;
    p = skip_hspace(p);
    return std::atof(p);
}

int main(int argc, char** argv) {
    std::setvbuf(stdout, nullptr, _IONBF, 0);

    bool generate = false;
    RunConfig rc;
    for (int i = 1; i < argc; ++i) {
        if (std::strcmp(argv[i], "--generate") == 0) {
            generate = true;
        } else if (std::strncmp(argv[i], "--warmup-iterations=", 20) == 0) {
            int v = std::atoi(argv[i] + 20);
            rc.warmup_iterations = (v >= 0) ? v : 0;
        } else if (std::strncmp(argv[i], "--iterations=", 13) == 0) {
            int v = std::atoi(argv[i] + 13);
            // Need >= 3 to drop min/max and still have a median; clamp.
            rc.meas_iterations = (v >= 3) ? v : 7;
        }
    }

    std::printf("=== FCEUX11 v1.1 Bench Tolerance Test ===\n");
    std::printf("Mode: %s  (warmup=%d passes, measure=%d iters, drop min/max)\n\n",
                generate ? "GENERATE local baseline" : "VERIFY against baseline",
                rc.warmup_iterations, rc.meas_iterations);

    // Pick baseline path: env var > v1.3 default > v1.0 fallback.
    // v1.3 Legion Phase 7.3: the v1.3 baseline (fixtures/bench_baseline.json)
    // is the canonical 1%-tolerance reference. The v1.0 baseline
    // (benchmarks/baseline_v1.0.json) remains as a historical fallback
    // — measured on a much faster dedicated runner, so typical CI
    // hosts will sit ~40-80% above it and is therefore useless for
    // regression detection unless the v1.0 baseline is also being
    // treated as an "absolute speed floor" rather than a peer.
    const char* baseline_path = std::getenv("FCEUX11_BENCH_BASELINE");
    if (!baseline_path || !*baseline_path) {
        baseline_path = "fixtures/bench_baseline.json";
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
        double ms = run_bench(kBenchs[i], rc);
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
        // Asymmetric gate: speedups (dev < 0) always pass; only
        // slowdowns (dev > 0) must stay within `tolerance_pct`.
        bool is_speedup = (dev_pct < 0.0);
        bool within     = is_speedup || (dev_pct <= tolerance_pct);
        if (is_speedup) {
            std::printf("  baseline: %.3f ms  deviation: %+.2f%%  (speedup; no upper limit)\n",
                        base_ms, dev_pct);
        } else {
            std::printf("  baseline: %.3f ms  deviation: %+.2f%%  max-regression: +%.1f%%\n",
                        base_ms, dev_pct, tolerance_pct);
        }
        if (within) {
            std::printf("  PASS\n");
            ++ctx.passed;
        } else {
            std::printf("  FAIL: regression exceeds max-regression threshold\n");
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
