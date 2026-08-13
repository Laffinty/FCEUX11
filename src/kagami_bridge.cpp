// KagamiQA — C ABI bridge implementation.
//
// Wraps the FCEUX11 core behind a minimal extern "C" API so the Rust
// kagami-qa crate can drive the emulator in-process.  Uses the null
// driver (no Qt) and provides frame-by-frame oracle probe access.

#include "kagami_bridge.h"

#include "types.h"
#include "fceu.h"
#include "driver.h"
#include "x6502.h"
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
#include "vnesu11_bridge.h"      // fceu11::g_vnesu11_soc

#include <cstdio>
#include <cstdlib>
#include <cstring>

#ifdef VNESU11_CORE_ENABLED
extern "C" {
uint8_t vnesu11_cpu_peek(const void* soc, uint16_t addr);
int     vnesu11_emulate_frame(void* soc, int skip, uint8_t* xbuf,
                              int16_t* sbuf, size_t sbuf_cap,
                              size_t* sbuf_written);
int     vnesu11_power_on_with_init(void* soc);
int     vnesu11_set_ram_init(void* soc, uint32_t option, uint32_t seed);
void    vnesu11_reset(void* soc);
}
#endif

// ---------------------------------------------------------------------------
// Singleton state
// ---------------------------------------------------------------------------
static bool g_initialised = false;
static bool g_rom_loaded  = false;

/// When set, the blargg/regression harness measures the vNESU11 Rust core
/// as the primary emulator instead of the C++ core. This is the missing
/// measurement path: under the default shadow run, C++ is primary and the
/// Rust core's output is discarded, so `ARead[]` probes report C++ state.
/// Env-var gate keeps the default behaviour untouched for the existing
/// goldens.
static bool g_rust_primary = []() -> bool {
    const char* e = std::getenv("VNESU11_RUST_PRIMARY");
    return e && (std::strcmp(e, "1") == 0);
}();

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

#ifdef VNESU11_CORE_ENABLED
    if (g_rust_primary && fceu11::g_vnesu11_soc) {
        // The Rust core registers mapper handlers via
        // vnesu11_on_game_load (called inside LoadGame), but it has not
        // been powered on. Bring its RAM/CPU/PPU/APU to a power-on state
        // so it emulates the ROM independently of the C++ core.
        vnesu11_set_ram_init(fceu11::g_vnesu11_soc, 0, 0);
        vnesu11_power_on_with_init(fceu11::g_vnesu11_soc);
        vnesu11_reset(fceu11::g_vnesu11_soc);
    }
#endif

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

#ifdef VNESU11_CORE_ENABLED
    if (g_rust_primary && fceu11::g_vnesu11_soc) {
        uint8_t rust_xbuf[61440];
        int16_t rust_sbuf[32768];
        size_t  rust_sbuf_written = 0;
        const int rc = vnesu11_emulate_frame(
            fceu11::g_vnesu11_soc, 0, rust_xbuf, rust_sbuf,
            sizeof(rust_sbuf) / sizeof(rust_sbuf[0]),
            &rust_sbuf_written);
        return (rc == 0) ? 0 : -3;
    }
#endif

    fceu11::Emulate(&xbuf, &sbuf, &sbuf_size, 0);
    return 0;
}

// ---------------------------------------------------------------------------
// Oracle probes
// ---------------------------------------------------------------------------
uint8_t kagami_bridge_read_byte(uint16_t addr) {
    if (addr < 0x10000) {
#ifdef VNESU11_CORE_ENABLED
        if (g_rust_primary && fceu11::g_vnesu11_soc) {
            return vnesu11_cpu_peek(fceu11::g_vnesu11_soc, addr);
        }
#endif
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
#ifdef VNESU11_CORE_ENABLED
    if (g_rust_primary && fceu11::g_vnesu11_soc) {
        vnesu11_reset(fceu11::g_vnesu11_soc);
        return 0;
    }
#endif
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
