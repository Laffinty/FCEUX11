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
