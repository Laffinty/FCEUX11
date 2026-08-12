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

}  // namespace fceu11