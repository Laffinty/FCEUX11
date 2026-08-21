// Cycle-trace diagnostic harness — Phase 4.5 cycle-drift root cause
// localization.
//
// Usage:
//   kagami_qa_cycle_trace <rom.nes> <frame_count> <output.csv>
//
// Drives the emulator frame-by-frame through the kagami_bridge C ABI
// (the same entry points used by `kagami_qa_direct_runner`), with the
// `FCEUX11_CYCLE_LOG` environment variable set to <output.csv>. The C++
// `CycleTraceSink` (defined in src/kagami_bridge.cpp) opens the CSV at
// first use and appends one row per `Cpu::run` call.
//
// Pre-Phase-7 this was run TWICE (once per `FCEUX11_RUST_CPU` setting)
// and the CSVs diffed via tools/cross_lang_diff.py to localize per-frame
// cycle drift (docs/plans/phase4-dispatch-budget-fix-2026-08-19.md §5.2);
// since Phase 7 deleted the C++ CPU, only the Rust-CPU side exists.

#include "kagami_bridge.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

namespace {

constexpr const char* USAGE =
    "Usage: %s <rom.nes> <frame_count> <output_csv>\n"
    "  Sets FCEUX11_CYCLE_LOG=<output_csv> in the process env, then\n"
    "  drives 0..frame_count frames through the kagami_bridge.\n";

int badexit(int rc) {
    kagami_bridge_full_reset();
    kagami_bridge_kill();
    return rc;
}

} // namespace

int main(int argc, char** argv) {
    if (argc != 4) {
        std::fprintf(stderr, USAGE, argv[0]);
        return 2;
    }
    const char* rom_path = argv[1];
    long frame_count_l = std::strtol(argv[2], nullptr, 10);
    if (frame_count_l <= 0 || frame_count_l > 0xFFFFFFL) {
        std::fprintf(stderr, "invalid frame_count: %s\n", argv[2]);
        return 2;
    }
    uint32_t frame_count = static_cast<uint32_t>(frame_count_l);
    const char* trace_csv = argv[3];

    // Set the env var BEFORE kagami_bridge_init so the C++ CycleTraceSink
    // reads the path at first construction. Process-lifetime cache.
    // MSVC ships `_putenv_s` (POSIX `setenv` is not in the standard
    // CRT); on Windows we use it. On non-Windows the more familiar
    // `setenv` would also work, but kagami_bridge only runs on the
    // CI's MSVC environment today.
    std::string env_line = std::string("FCEUX11_CYCLE_LOG=") + trace_csv;
    if (_putenv(env_line.c_str()) != 0) {
        std::fprintf(stderr, "_putenv FCEUX11_CYCLE_LOG failed\n");
        return 2;
    }

    std::fprintf(stdout, "[cycle-trace] rom=%s frames=%u trace=%s\n",
                 rom_path, frame_count, trace_csv);

    if (kagami_bridge_init() != 0) {
        std::fprintf(stderr, "kagami_bridge_init failed\n");
        return 1;
    }
    if (kagami_bridge_load_rom(rom_path) != 0) {
        std::fprintf(stderr, "kagami_bridge_load_rom('%s') failed\n", rom_path);
        return badexit(1);
    }

    // Drive frame_count frames. set_frame() is called BEFORE
    // emulate_frame() so the trace's frame column reflects the
    // upcoming frame's start.
    bool ok = true;
    for (uint32_t f = 0; f < frame_count; ++f) {
        kagami_bridge_cycle_trace_set_frame(f);
        if (kagami_bridge_emulate_frame() != 0) {
            std::fprintf(stderr, "kagami_bridge_emulate_frame failed at frame %u\n", f);
            ok = false;
            break;
        }
    }

    uint64_t rows = kagami_bridge_cycle_trace_row_count();
    std::fprintf(stdout, "[cycle-trace] wrote %llu rows to %s\n",
                 (unsigned long long)rows, trace_csv);

    int rc = badexit(ok ? 0 : 1);
    return rc;
}
