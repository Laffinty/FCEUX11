// vNESU11 Shadow Run harness — Phase 6 §2.5.
//
// Frame-level 3-tier diff (XBuf CRC / audio SNR / savestate MD5) of
// the Rust emulator's output against the C++ pipeline's. The C++
// Emulate() runs the Rust emulator in parallel via
// `vnesu11_emulate_frame_bridge`; this header exposes the captured
// data to external harnesses (kagami-qa-runner, Rust integration
// tests, etc.).
//
// Format note: the Rust xbuf holds palette indices (0..=3 per pixel,
// encoded as `(palette << 2) | color`); the C++ XBuf holds NES color
// indices after palette-RAM lookup. Direct byte comparison is not
// meaningful; shadow run operates on CRCs separately.

#pragma once

#include <stdint.h>
#include <stddef.h>

namespace fceu11 {

/// Latest shadow capture (palette-index frame + i16 stereo audio).
/// Updated by `vnesu11_emulate_frame_bridge` after each Rust frame.
struct ShadowData {
    uint8_t  xbuf[61440];      // Rust palette-index frame (256×240)
    int16_t  sbuf[32768];      // Rust i16 stereo (one per CPU cycle)
    size_t   sbuf_written;     // Number of valid stereo samples
    uint64_t frame_count;      // Total frames captured
};

/// Read the latest shadow capture into `out`. Returns 0 on success,
/// -1 if `out` is null.
int vnesu11_shadow_get(ShadowData* out) noexcept;

/// Reset the shadow capture (called on Power/Reset).
void vnesu11_shadow_reset() noexcept;

/// CRC32 of a byte buffer (IEEE 802.3 polynomial). Exposed for the
/// shadow run harness to compare Rust palette-index frame CRCs
/// against a baseline file.
uint32_t vnesu11_shadow_crc32(const uint8_t* data, size_t n) noexcept;

/// Compute a 3-tier diff stat (frame CRC + audio count + frame index)
/// and log to stderr when `vn_frame_count % every_n == 0`. This is
/// called from the C++ Emulate() under VNESU11_CORE_ENABLED.
void vnesu11_shadow_log_every(uint64_t every_n) noexcept;

/// Sync the C++ post-frame CPU + WRAM state into the Rust SoC, so the
/// Rust shadow frame runs from the same starting state as the C++
/// frame did. Called from `fceu11::Emulate()` before the Rust frame.
/// Under VNESU11_CORE=OFF this is a no-op.
void vnesu11_shadow_sync_from_cpp() noexcept;

/// Last comparison result (set by the runner / shadow log).
struct ShadowCompare {
    bool cpu_regs_match;   // PC/A/X/Y/S/P identical
    uint32_t cpp_xbuf_crc; // C++ XBuf (NES colors) CRC
    uint32_t rust_xbuf_crc;// Rust shadow xbuf (palette indices) CRC
    uint64_t frame_index;
};

/// Read the latest shadow comparison. Returns 0 on success.
int vnesu11_shadow_compare(ShadowCompare* out) noexcept;

/// `every_n` default — log every 60 frames.
constexpr uint64_t kShadowLogEveryFrames = 60;

/// Phase 6 P2 APU state mirror. Pushed from C++ into Rust after each
/// C++ frame via `vnesu11_apu_poke_state`. Layout MUST match the
/// `ApuStateMirror` `#[repr(C)]` struct in
/// `crates/vnesu11/src/ffi.rs`. Field order frozen against the FFI
/// signature (audit S1; verified by tests/layout_check.rs).
///
/// Scope: timing-critical fields only (frame counter + master cycle
/// counter + IRQ pending + channel enables). Per-channel timers /
/// envelopes / sweeps / DMC buffers are deferred to the broader
/// state-sync pass (see phase_6_integration.md §9). Without at
/// least this slice, Rust's frame counter IRQ lands in a different
/// instruction window than C++'s (e.g. blargg cpu_dummy_reads
/// diverges after frame 2 — Rust enters the IRQ handler at $E622
/// while C++ stays in the $2002 VBlank-wait loop).
struct ApuStateMirror {
    uint64_t cycles;               // Master cycle counter
    uint64_t fc_cycle_count;       // Frame counter period position
    uint8_t  fc_step;              // Step counter (0..=3 / 0..=4)
    bool     fc_five_step;         // $4017 bit 7
    bool     fc_irq_inhibit;       // $4017 bit 6
    bool     pal;                  // PAL timing (C++ global PAL)
    uint8_t  fc_pending_mode;      // Pending mode bits
    uint8_t  fc_reset_in;          // Cycles until reset matures
    bool     fc_quarter_frame;     // Quarter-frame flag latched
    bool     fc_half_frame;        // Half-frame flag latched
    bool     frame_irq_pending;    // Frame IRQ pending
    bool     dmc_irq_pending;      // DMC IRQ pending
    uint8_t  enabled_channels;     // $4015 channel-enable mask
};

/// Phase 6 P2 PPU state mirror. Pushed from C++ into Rust after each
/// C++ frame via `vnesu11_ppu_poke_state`. Layout MUST match the
/// `PpuStateMirror` `#[repr(C)]` struct in `crates/vnesu11/src/ffi.rs`.
///
/// Scope: registers + memory the CPU can read (status / read buffer /
/// palette / VRAM / OAM). Internal render latches are NOT synced —
/// they only affect the rendered frame, not CPU-observable state; the
/// CPU instruction stream stays identical once the reads match (the
/// $2005/$2006 write sequence replays identically from the synced
/// start because both cores begin with v/t/x/w = 0 after power-on).
struct PpuStateMirror {
    uint8_t  ppuctrl;      // $2000 PPUCTRL (PPU[0])
    uint8_t  ppumask;      // $2001 PPUMASK (PPU[1])
    uint8_t  status;       // $2002 PPUSTATUS (PPU[2])
    uint8_t  oam_addr;     // $2003 OAMADDR (PPU[3])
    uint8_t  read_buffer;  // $2007 read buffer (PPUGenLatch)
    uint8_t  open_bus;     // CPU data-bus open value (X6502.DB)
    uint8_t  palette[32];  // PALRAM
    uint8_t  vram[2048];   // NTARAM (4 nametables)
    uint8_t  oam[256];     // g_ppu.oam() / SPRAM
    uint8_t  vbl_set_suppressed;  // fceu11_ppu_peek_vbl_set_suppressed()
};

}  // namespace fceu11

// FFI declarations live OUTSIDE the namespace so they don't get
// tagged `fceu11::vnesu11_apu_poke_state` (which would mismatch the
// Rust `#[unsafe(no_mangle)] pub extern "C" fn` symbol).
#ifdef __cplusplus
extern "C" {
#endif

/// Push C++'s APU state into Rust. Called by the shadow sync path
/// after each C++ frame. Returns 0 on success, -1 on null SoC,
/// -2 on null state pointer.
int vnesu11_apu_poke_state(void* soc,
                           const struct fceu11::ApuStateMirror* state);

/// Snapshot Rust's APU state into the mirror. Provided for
/// round-trip tests / savestate parity work.
int vnesu11_apu_peek_state(void* soc,
                           struct fceu11::ApuStateMirror* out_state);

/// Push C++'s PPU state (registers + CPU-observable memory) into
/// Rust. Called by the shadow sync path after each C++ frame.
/// Returns 0 on success, -1 on null SoC, -2 on null state pointer.
int vnesu11_ppu_poke_state(void* soc,
                           const struct fceu11::PpuStateMirror* state);

/// Snapshot Rust's PPU state into the mirror. Provided for
/// round-trip tests / savestate parity work.
int vnesu11_ppu_peek_state(void* soc,
                           struct fceu11::PpuStateMirror* out_state);

#ifdef __cplusplus
}
#endif