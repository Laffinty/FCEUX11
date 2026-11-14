// KagamiQA — C ABI bridge implementation.
//
// Wraps the FCEUX11 core behind a minimal extern "C" API so the Rust
// kagami-qa crate can drive the emulator in-process.  Uses the null
// driver (no Qt) and provides frame-by-frame oracle probe access.

#include "f11qa_bridge.h"

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

#include <cstdio>
#include <cstdlib>
#include <cstring>

// ---------------------------------------------------------------------------
// Singleton state
// ---------------------------------------------------------------------------
static bool g_initialised = false;
static bool g_rom_loaded  = false;

// ---------------------------------------------------------------------------
// Famicom microphone injection state (v2.0.4 V6 / kgmqa-108)
//
// The mic is not an ESI port device — it is the P2 Start button routed into
// the $4016 D2 level read by replaceP2StartWithMicrophone (the preserved key
// mapping). The bridge therefore owns a persistent port-1 gamepad buffer:
// FCEU_UpdateInput() hands joyports[1].ptr to UpdateGP every frame, which
// shifts byte1 into joy[1], and JPRead reads joy[1]&8. Setting bit3 of byte1
// (= 0x0800 in the little-endian uint32 buffer) is "P2 Start pressed" = "mic
// pressed" — the same path the Qt driver's JSreturn feeds.
// ---------------------------------------------------------------------------
static uint32_t g_famicom_p2_pad = 0;  // persistent port-1 gamepad state

// Byte layout per controller (UpdateGamepad, Qt input.cpp): bit0=A .. bit3=
// Start .. bit7=Right. Controller 2 lives at byte1 of the port-1 dword.
static constexpr uint32_t FAMICOM_P2_START_BIT = 0x0800u;

// v2.0.4 V6: reserved `sut_input` device value for the manifest chain. The
// Rust side forwards `sut_input: [port0, port1]` verbatim as
// `port=type` pairs (rom_runner.rs --sut-input → blargg.rs → direct.rs
// kagami_bridge_set_input), so a manifest can carry the mic without any
// Rust change: `sut_input: [1, 64]`. 0x40 lies outside the ESI enum
// (git.h fceu11::InputDevice: 0..10; SI_UNSET = -1) and cannot collide
// with any SI_* value; the bridge intercepts it BEFORE FCEUI_SetInput, so
// the out-of-enum value never reaches the core. Semantics: "Famicom P2
// with microphone, pressed" (= kagami_bridge_set_famicom_mic(1, 1)).
static constexpr int KAGAMI_SUT_FAMICOM_P2_MIC = 0x40;

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
// Input device selection (D9 / kgmqa-107)
// ---------------------------------------------------------------------------
int kagami_bridge_set_input(int port, int esi_type) {
    if (port < 0 || port > 1) {
        std::fprintf(stderr, "kagami_bridge_set_input: bad port %d\n", port);
        return -1;
    }
    if (!g_initialised) {
        std::fprintf(stderr, "kagami_bridge_set_input: not initialised\n");
        return -2;
    }
    // v2.0.4 V6: manifest `sut_input` mic encoding — see the
    // KAGAMI_SUT_FAMICOM_P2_MIC comment above. Intercepted before the
    // ESI cast so the out-of-enum value never reaches the core.
    if (esi_type == KAGAMI_SUT_FAMICOM_P2_MIC) {
        return kagami_bridge_set_famicom_mic(port, 1);
    }
    // Same shape as the init-time SI_NONE default; the SI_SNES_MOUSE branch
    // of SetInputStuff (input.cpp:522) does not depend on GameInfo.
    // ptr must NOT be nullptr for real device drivers: FCEU_UpdateInput()
    // (input.cpp:421) hands joyports[port].ptr straight to the driver's
    // Update every frame, and e.g. UpdateSNESMouse reads ptr[0..2]
    // unconditionally — a null ptr would fault on frame 1. The headless
    // bridge injects no movement, so a persistent zero buffer is exactly
    // the "idle device" state.
    static int32 zero_input[3] = {0, 0, 0};
    FCEUI_SetInput(port, static_cast<ESI>(esi_type), zero_input, 0);
    return 0;
}

// ---------------------------------------------------------------------------
// Famicom microphone injection (v2.0.4 V6 / kgmqa-108) — see f11qa_bridge.h
// ---------------------------------------------------------------------------
int kagami_bridge_set_famicom_mic(int port, int on) {
    if (port != 1) {
        std::fprintf(stderr,
                     "kagami_bridge_set_famicom_mic: bad port %d "
                     "(mic is P2 / second pad only)\n",
                     port);
        return -1;
    }
    if (!g_initialised) {
        std::fprintf(stderr, "kagami_bridge_set_famicom_mic: not initialised\n");
        return -2;
    }
    // Core symbol lives in src/input.cpp (declared for the Qt driver in
    // src/drivers/Qt/input.h:145). Protocol enable persists for the session:
    // it is the key mapping, not the per-read level.
    extern bool replaceP2StartWithMicrophone;
    replaceP2StartWithMicrophone = true;
    if (on) {
        g_famicom_p2_pad |= FAMICOM_P2_START_BIT;
    } else {
        g_famicom_p2_pad &= ~FAMICOM_P2_START_BIT;
    }
    // Attach the gamepad driver with OUR persistent buffer (the init default
    // is SI_NONE/nullptr). The SI_GAMEPAD branch of SetInputStuff does not
    // depend on GameInfo (same shape as kagami_bridge_set_input), and
    // SetInputStuff is idempotent, so re-calling per press/release is safe.
    FCEUI_SetInput(1, static_cast<ESI>(SI_GAMEPAD), &g_famicom_p2_pad, 0);
    return 0;
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
// Check probes
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
