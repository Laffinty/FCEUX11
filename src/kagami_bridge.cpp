// KagamiQA — C ABI bridge implementation.
//
// Wraps the FCEUX11 core behind a minimal extern "C" API so the Rust
// kagami-qa crate can drive the emulator in-process.  Uses the null
// driver (no Qt) and provides frame-by-frame oracle probe access.

#include "kagami_bridge.h"

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "cpu.h"
#include "bus.h"
#include "state.h"
#include "cart.h"
#include "cart_class.h"        // fceu11::g_cart, Cart::save_mapper_state
#include "sound.h"
#include "ppu.h"
#include "video.h"               // for XBuf
#include "emufile.h"             // for EMUFILE_MEMORY
#include "drivers/common/nes_shm.h"
#include "driver_callbacks.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>

// ---------------------------------------------------------------------------
// Singleton state
// ---------------------------------------------------------------------------
static bool g_initialised = false;
static bool g_rom_loaded  = false;

// ---------------------------------------------------------------------------
// Init / Kill
// ---------------------------------------------------------------------------
int kagami_bridge_init(void) {
    if (g_initialised) return 0;  // idempotent

    // Register null driver callbacks (headless — no Qt, no GUI).
    // Every FCEUD_* forwarding function checks the fn pointer before
    // calling, so all-nullptr is safe.
    fceu11::register_driver(fceu11::DriverCallbacks{});

    if (!fceu11::Initialize()) {
        std::fprintf(stderr, "kagami_bridge: Initialize() failed\n");
        return -1;
    }

    if (!nes_shm) {
        nes_shm = open_nes_shm();
    }

    // No input devices needed — ROM runs autonomously.
    FCEUI_SetInput(0,    static_cast<ESI>(SI_NONE),    nullptr, 0);
    FCEUI_SetInput(1,    static_cast<ESI>(SI_NONE),    nullptr, 0);
    FCEUI_SetInputFC(static_cast<ESIFC>(SIFC_NONE),    nullptr, 0);
    FCEUI_SetInputFourscore(false);

    g_initialised = true;
    return 0;
}

void kagami_bridge_kill(void) {
    if (g_rom_loaded) {
        fceu11::CloseGame();
        g_rom_loaded = false;
    }
    if (g_initialised) {
        fceu11::Kill();
        g_initialised = false;
    }
}

// ---------------------------------------------------------------------------
// ROM loading
// ---------------------------------------------------------------------------
int kagami_bridge_load_rom(const char *path) {
    if (!g_initialised) {
        std::fprintf(stderr, "kagami_bridge: not initialised\n");
        return -1;
    }
    if (g_rom_loaded) {
        fceu11::CloseGame();
        g_rom_loaded = false;
    }

    FCEUGI *gi = fceu11::LoadGame(path, 1, true);
    if (!gi) {
        std::fprintf(stderr, "kagami_bridge: LoadGame('%s') failed\n", path);
        return -2;
    }

    g_rom_loaded = true;
    return 0;
}

// ---------------------------------------------------------------------------
// Frame emulation
// ---------------------------------------------------------------------------
int kagami_bridge_emulate_frame(void) {
    if (!g_rom_loaded) {
        std::fprintf(stderr, "kagami_bridge: no ROM loaded\n");
        return -1;
    }

    uint8 *xbuf       = nullptr;
    int32 *sbuf       = nullptr;
    int    sbuf_size  = 0;

    fceu11::Emulate(&xbuf, &sbuf, &sbuf_size, 0);
    return 0;
}

// ---------------------------------------------------------------------------
// Oracle probes
// ---------------------------------------------------------------------------
uint8_t kagami_bridge_read_byte(uint16_t addr) {
    if (addr < 0x10000) {
        return ARead[addr](addr);
    }
    return 0xFF;
}

uint8_t kagami_bridge_read_ppu(uint16_t addr) {
    // FFCEUX_PPURead handles address mirroring internally.
    if (FFCEUX_PPURead) {
        return FFCEUX_PPURead(addr);
    }
    return 0x00;
}

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------
int kagami_bridge_reset(void) {
    if (!g_initialised) return -1;

    // Soft reset (matches C++ blargg_runner's fceu11::ResetNES()): the ROM
    // stays loaded and the engine stays initialised, so frame-stepping can
    // continue after the reset. This is what apu_reset_* / cpu_reset_* blargg
    // ROMs expect (their test begins only after a manual reset).
    //
    // The previous implementation (CloseGame + Kill + re-init) unloaded the
    // ROM, which made every subsequent step() fail with "no ROM loaded" —
    // Rust-side reset_after runs then produced 0xFE load-failures. Task 1
    // parity required matching the C++ semantics exactly.
    fceu11::ResetNES();
    return 0;
}

// ---------------------------------------------------------------------------
// Full teardown + re-init (mirrors C++ savestate_regression_test.cpp:
// each computeSavestateHash() does a fresh fceu11::Initialize()/Kill()
// cycle per ROM). Used by the C-3 savestate harness between ROMs so the
// engine starts from the same pristine state the golden hashes were
// generated with.
// ---------------------------------------------------------------------------
int kagami_bridge_full_reset(void) {
    if (g_rom_loaded) {
        fceu11::CloseGame();
        g_rom_loaded = false;
    }
    if (g_initialised) {
        fceu11::Kill();
        g_initialised = false;
    }
    return kagami_bridge_init();
}

// ---------------------------------------------------------------------------
// PPU mode
// ---------------------------------------------------------------------------
void kagami_bridge_set_newppu(int on) {
    extern int newppu;
    newppu = (on != 0) ? 1 : 0;
}

// ---------------------------------------------------------------------------
// CPU PC peek — Phase 4.5 cycle-drift diagnostic.
//
// The diagnostic harness `kagami-qa-cycle-trace` records per
// `X6502_RunDebug` call's cycles_arg + post-call PC to a CSV (gated by
// `FCEUX11_CYCLE_LOG`). Both Rust CPU ON and OFF paths funnel through
// here so the diff can be byte-equal across the two builds.
//
// Reading `pc()` from the Cpu accessor goes through `Cpu::pc()` which
// reads `layout_.PC` — pinned by `static_assert` on the savestate
// layout. The value is a CPU register, not a memory address.
// ---------------------------------------------------------------------------
uint16_t kagami_bridge_get_cpu_pc(void) {
    return fceu11::cpu_instance().pc();
}

// ---------------------------------------------------------------------------
// CPU IRQlow peek/poke — Phase 4.5 cycle-drift fix.
//
// The Rust CPU's `Bus::sync_irq_from_host` / `sync_irq_to_host` read /
// write the C++ `X6502::IRQlow` blob around every dispatch boundary so
// that IRQs asserted by the C++ side during a Rust `run_with_tick`
// call (mapper `X6502_IRQBegin`, APU frame-counter IRQ via
// `FCEU_SoundCPUHook`) are visible to the Rust dispatch, and bits the
// Rust dispatch consumed (NMI) are not re-asserted on the next call.
// ---------------------------------------------------------------------------
uint32_t kagami_bridge_get_cpu_irq_low(void) {
    return fceu11::cpu_instance().native_layout().IRQlow;
}

void kagami_bridge_set_cpu_irq_low(uint32_t v) {
    fceu11::cpu_instance().native_layout().IRQlow = v;
}

bool kagami_bridge_get_cpu_nmi_fresh(void) {
    return x6502_nmi_fresh_get();
}

void kagami_bridge_set_cpu_nmi_fresh(bool v) {
    x6502_nmi_fresh_set(v);
}

// ---------------------------------------------------------------------------
// Cycle-trace sink — Phase 4.5 cycle-drift diagnostic.
//
// `FCEUX11_CYCLE_LOG=<path>` activates a per-`X6502_RunDebug` (or its
// FFI equivalent) CSV log with one row per Cpu::run call. Rows have
// no frames column on the wire (the trace span is bounded by the
// caller that opens the file); instead we use `pc_after` as the
// canary for cross-language diffing.
//
// Activation reads the env var ONCE on construction; the sink is a
// function-local static so no header changes leak beyond kagami_bridge.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Frame buffer extraction (Track C Task 1 / C-2)
//
// Track C C-2 replaces tests/rom_regression_test.cpp with a Rust harness
// under kagami-qa::runner::rom_regression. The C++ harness CRC32s the
// visible 256x240 region of XBuf after each frame; this FFI is the
// minimal surface that lets the Rust side do the same byte-for-byte.
// ---------------------------------------------------------------------------
int kagami_bridge_extract_frame_buffer(uint8_t *dst, uint32_t len) {
    if (!dst) {
        return -1;
    }
    extern uint8 *XBuf;
    if (!XBuf) {
        return -2;
    }
    std::memcpy(dst, XBuf, len);
    return 0;
}

// ---------------------------------------------------------------------------
// Savestate serialisation (Track C Task 1 / C-3)
//
// Track C C-3 replaces tests/savestate_regression_test.cpp with a Rust
// harness under kagami-qa::runner::savestate_regression. The C++
// harness runs N frames, then FCEUSS_SaveMS into an EMUFILE_MEMORY
// wrapper and MD5s the bytes; this FFI is the minimal surface that
// lets the Rust side do the same byte-for-byte.
//
// Returns 0 on success. `written_out` always receives the actual
// savestate size (caller can compare with `cap` to detect truncation
// and retry with a larger buffer).
// ---------------------------------------------------------------------------
int kagami_bridge_save_state(uint8_t *dst, uint32_t cap,
                             uint32_t *written_out,
                             int compression_level) {
    if (!written_out) {
        return -1;
    }
    *written_out = 0;
    if (cap > 0 && !dst) {
        return -2;
    }

    std::vector<std::byte> buffer;
    EMUFILE_MEMORY file(&buffer);
    if (!FCEUSS_SaveMS(&file, compression_level)) {
        return -3;
    }
    const size_t total = buffer.size();
    *written_out = static_cast<uint32_t>(total);

    if (cap > 0) {
        const size_t to_copy = std::min(static_cast<size_t>(cap), total);
        std::memcpy(dst, buffer.data(), to_copy);
    }
    return 0;
}

int kagami_bridge_save_mapper_state(uint8_t *dst, uint32_t cap,
                                    uint32_t *written_out) {
    if (!written_out) {
        return -1;
    }
    *written_out = 0;
    if (cap > 0 && !dst) {
        return -2;
    }
    if (!fceu11::g_cart) {
        return -3;
    }

    const std::vector<uint8_t> body = fceu11::g_cart->save_mapper_state();
    const size_t total = body.size();
    *written_out = static_cast<uint32_t>(total);

    if (cap > 0) {
        const size_t to_copy = std::min(static_cast<size_t>(cap), total);
        std::memcpy(dst, body.data(), to_copy);
    }
    return 0;
}

// ---------------------------------------------------------------------------
// Cycle-trace sink (Phase 4.5 cycle-drift diagnostic)
//
// `FCEUX11_CYCLE_LOG=<path>` activates a one-row-per-`Cpu::run` CSV.
// The Rust harness `kagami-qa-cycle-trace` runs a ROM and records the
// CSV. Pre-Phase-7 the same ROM was also run under
// `FCEUX11_RUST_CPU=OFF` and the two CSVs diffed to localize the
// dominant cycle drift root cause; since Phase 7 deleted the C++ CPU,
// only the Rust-CPU side exists.
//
// Header: `frame,call_idx,cycles_arg,pc_after,cum_count`
//   - `frame` is filled by the harness BEFORE calling `emulate_frame`
//     (passed via the env-var-controlled `current_frame_idx` global).
//   - `call_idx` is a monotonic counter incremented by `record`.
//   - `cycles_arg` is the `cycles` argument to the matching `Cpu::run`.
//   - `pc_after` is the program counter at end of the call.
//   - `cum_count` is `Cpu::count` (1/16-dot-unit accumulator) at end.
// ---------------------------------------------------------------------------
namespace {

struct CycleTraceSink {
    FILE* fp = nullptr;
    uint32_t call_idx = 0;
    uint32_t current_frame_idx = 0;
    uint64_t row_count = 0;

    CycleTraceSink() {
        const char* path = std::getenv("FCEUX11_CYCLE_LOG");
        if (!path || !*path) {
            return;
        }
        fp = std::fopen(path, "w");
        if (!fp) {
            std::fprintf(stderr, "kagami_bridge: cannot open FCEUX11_CYCLE_LOG='%s'\n", path);
            return;
        }
        std::fprintf(fp, "frame,call_idx,cycles_arg,pc_after,cum_count,irq_low\n");
        std::fflush(fp);
    }

    ~CycleTraceSink() {
        if (fp) {
            std::fclose(fp);
            fp = nullptr;
        }
    }

    void set_frame(uint32_t f) {
        current_frame_idx = f;
    }

    void record(uint32_t cycles_arg, uint16_t pc_after, uint32_t cum_count,
                uint32_t irq_low) {
        if (!fp) return;
        std::fprintf(fp, "%u,%u,%u,%u,%u,%u\n",
                     current_frame_idx,
                     call_idx,
                     cycles_arg,
                     pc_after,
                     cum_count,
                     irq_low);
        ++call_idx;
        ++row_count;
        if ((row_count & 0x3FF) == 0) {
            std::fflush(fp);
        }
    }
};

// Singleton sink, lazily opened at first use. Re-checking the env on
// every Cpu::run call would add a syscall per call; this is checked
// once at first call (process-lifetime).
CycleTraceSink& sink() {
    static CycleTraceSink s;
    return s;
}

} // namespace

extern "C" void kagami_bridge_cycle_trace_record(uint32_t cycles_arg,
                                                uint16_t pc_after,
                                                uint32_t cumulative_count,
                                                uint32_t irq_low) {
    sink().record(cycles_arg, pc_after, cumulative_count, irq_low);
}

extern "C" void kagami_bridge_cycle_trace_set_frame(uint32_t frame_idx) {
    sink().set_frame(frame_idx);
}

extern "C" uint32_t kagami_bridge_cycle_trace_current_frame(void) {
    return sink().current_frame_idx;
}

extern "C" uint64_t kagami_bridge_cycle_trace_row_count(void) {
    return sink().row_count;
}
