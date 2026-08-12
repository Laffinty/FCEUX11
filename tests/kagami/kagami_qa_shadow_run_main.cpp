// KagamiQA Phase 6 - Shadow Run runner (frame-level CPU compare).
//
// Loads a ROM, runs N frames through the C++ pipeline (primary). The
// vNESU11 Rust core is driven in parallel by the fceu11::Emulate()
// shadow hook (src/fceu.cpp), which syncs the C++ post-frame CPU +
// WRAM state into the Rust SoC and then runs one Rust frame.
//
// Because the sync happens *after* each C++ frame, the Rust core is
// always one frame ahead; the runner compares C++ state at frame k+1
// against the Rust state captured after the hook of frame k. A match
// proves both cores made the identical S_k -> S_k+1 transition.
//
// Usage:
//   kagami_qa_shadow_run_runner <rom.nes> [--frames N] [--use_newppu 0|1]
//
// Output: a final SHADOW_RESULT line to stdout; per-frame DIFF lines
// to stderr. Returns 0 if every frame's CPU regs matched.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "fceu.h"
#include "x6502.h"          // g_cpu.native_layout() + X6502
#include "kagami_bridge.h"
#include "vnesu11_bridge.h"
#include "vnesu11_mapper_adapter.h"  // vnesu11_on_game_load

#ifdef VNESU11_CORE_ENABLED

// 64-byte #[repr(C)] mirror of crates/vnesu11/src/cpu/regs.rs
// `CpuRegsLayout` (audit S1: field order frozen against x6502struct.h,
// verified by tests/layout_check.rs + the x6502struct.h static_asserts).
// Declared locally so the test tree doesn't need the cbindgen header.
struct CpuRegsLayoutMirror {
    int32_t tcount;
    uint16_t PC;
    uint8_t A, X, Y, S, P, moo_pi, jammed;
    int32_t count;
    uint32_t irq_low;
    uint8_t db;
    int32_t preexec;
    void* cpu_hook;
    void* read_hook;
    void* write_hook;
};

extern "C" {
void vnesu11_cpu_peek_regs(void* soc, CpuRegsLayoutMirror* out);
}

namespace {

// The C++ X6502 struct layout mirrors the Rust CpuRegsLayout (audit
// S1; verified by tests/layout_check.rs + x6502struct.h static_asserts).
// We compare the canonical register set by field, not by blob, so any
// padding or hook-pointer drift can't cause false mismatches.
struct Regs {
    uint16_t pc;
    uint8_t a, x, y, s, p;
};

Regs read_cpp_regs() {
    const X6502& c = g_cpu.native_layout();
    return Regs{ c.PC, c.A, c.X, c.Y, c.S, c.P };
}

bool regs_equal(const Regs& a, const Regs& b) {
    return a.pc == b.pc && a.a == b.a && a.x == b.x &&
           a.y == b.y && a.s == b.s && a.p == b.p;
}

}  // namespace

#endif  // VNESU11_CORE_ENABLED (CpuRegsLayoutMirror + Regs helpers)

int main(int argc, char** argv) {
    if (argc < 2) {
        std::fprintf(stderr,
            "usage: %s <rom.nes> [--frames N] [--use_newppu 0|1]\n", argv[0]);
        return 2;
    }
    const char* rom = argv[1];
    int frames = 120;
    int use_newppu = 1;
    for (int i = 2; i < argc; ++i) {
        if (std::strcmp(argv[i], "--frames") == 0 && i + 1 < argc) {
            frames = std::atoi(argv[++i]);
        } else if (std::strcmp(argv[i], "--use_newppu") == 0 && i + 1 < argc) {
            use_newppu = std::atoi(argv[++i]);
        }
    }

#ifdef VNESU11_CORE_ENABLED
    // Headless init (no Qt). Also ensures the Rust SoC exists.
    if (kagami_bridge_init() != 0) {
        std::fprintf(stderr, "shadow_run: init failed\n");
        return 1;
    }
    kagami_bridge_set_newppu(use_newppu);
    fceu11::vnesu11_init();  // idempotent - creates g_vnesu11_soc

    if (kagami_bridge_load_rom(rom) != 0) {
        std::fprintf(stderr, "shadow_run: load '%s' failed\n", rom);
        return 1;
    }
    // NOTE: LoadGameVirtual (via kagami_bridge_load_rom) already called
    // fceu11::vnesu11_on_game_load, which registered the mapper's
    // per-range handlers into the Rust SoC during PowerNES(). Do NOT
    // call it again here — that would clear the registrations and the
    // Rust CPU would read open-bus garbage.

    // Prime: frame 0 runs C++ S0->S1 and (via the hook) Rust S1->S2.
    Regs rust_state{};   // Rust state captured after the hook of frame k
    bool have_rust = false;
    int total_frames = 0;
    int cpu_matches = 0;
    int cpu_differs = 0;

    for (int f = 0; f < frames; ++f) {
        // C++ runs frame f: S_f -> S_f+1 (hook: Rust S_f+1 -> S_f+2).
        kagami_bridge_emulate_frame();

        // Read Rust post-hook state (S_f+2) for the next comparison.
        CpuRegsLayoutMirror rust_regs{};
        vnesu11_cpu_peek_regs(fceu11::g_vnesu11_soc, &rust_regs);
        const Regs rust_after{ rust_regs.PC, rust_regs.A, rust_regs.X,
                               rust_regs.Y, rust_regs.S, rust_regs.P };

        if (f >= 1 && have_rust) {
            // Compare C++ S_f+1 (now) vs Rust S_f+1 (captured after
            // frame f-1's hook).
            const Regs cpp_now = read_cpp_regs();
            const bool match = regs_equal(cpp_now, rust_state);
            if (match) {
                ++cpu_matches;
            } else {
                ++cpu_differs;
                std::fprintf(stderr,
                    "SHADOW frame=%d cpu=DIFF cpp{pc=%04X a=%02X x=%02X y=%02X s=%02X p=%02X} "
                    "rust{pc=%04X a=%02X x=%02X y=%02X s=%02X p=%02X}\n",
                    f, cpp_now.pc, cpp_now.a, cpp_now.x, cpp_now.y, cpp_now.s, cpp_now.p,
                    rust_state.pc, rust_state.a, rust_state.x, rust_state.y,
                    rust_state.s, rust_state.p);
            }
        }
        rust_state = rust_after;
        have_rust = true;
        ++total_frames;
    }

    std::printf(
        "SHADOW_RESULT rom=%s frames=%d cpu_match=%d cpu_diff=%d\n",
        rom, total_frames, cpu_matches, cpu_differs);

    kagami_bridge_kill();
    return (cpu_differs == 0) ? 0 : 1;
#else
    (void)rom;
    (void)frames;
    (void)use_newppu;
    std::fprintf(stderr,
        "shadow_run: built without VNESU11_CORE - no shadow to run.\n");
    return 3;
#endif
}
