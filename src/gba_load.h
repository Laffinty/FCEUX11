// GBA cartridge loading and session state (v2.0 S2-b4 stage 1).
//
// This file is the C++ side of GBAEUX11's only front-end touchpoint in stage 1:
// recognising a `.gba`, handing it to the Rust core, and remembering that the
// active machine is no longer a NES. It deliberately does **not** touch video,
// audio or the emu loop's frame step -- those are stage 2 and stage 3, and the
// reasons they are separate are in the v2.0 plan's r39.
//
// # One machine at a time
//
// A session is either NES or GBA, never both. That is not a simplification for
// its own sake: the whole front end has exactly one frame slot, one audio ring
// and one throttle, and running both cores would mean two producers into one
// consumer. What the user gets is a second kind of game they can open, which is
// what the plan asks for -- `.gba` files are playable.
//
// # The screen will be black
//
// Stage 1 has no 32-bit path into the video pool, so a GBA session advances the
// machine and draws nothing. That is a deliberate choice over the alternative:
// leaving the previous NES frame on screen would show a **wrong** picture rather
// than a missing one, and a black screen cannot be mistaken for a game that
// rendered badly. `GbaLoad` clears the pool for exactly this reason.

#pragma once

#include <string>

// Whether the active machine is a GBA rather than a NES.
//
// Read on the emulation thread every frame and by the GUI when it needs to
// refuse an NES-only feature. A separate flag rather than a fifth `EGIT` value
// on purpose: `GameInfo->type` is read in about twenty places, all of them
// NES-specific logic, and handing them a value they do not recognise would drop
// each of them into "treat it as an ordinary cartridge" -- which is worse than
// not claiming it at all. See the v2.0 plan's r39.
bool fceu11_gba_active(void);

/// Hand the active session back to the NES.
///
/// Safe to call when no GBA session was active. Called when a NES ROM is loaded
/// over a GBA one, so the two can never both claim the frame slot.
void fceu11_gba_deactivate(void);

/// The loaded cartridge's path, or empty when no GBA session is active.
const std::string& fceu11_gba_path(void);

/// Advance the GBA machine by one video frame and hand its audio to the sound
/// device. Emulation thread only.
///
/// Audio is wired here even though video is not: the samples come out of the
/// core already, and `WriteSound` is the same call the NES side makes, so
/// leaving it out would mean throwing away work that needs no stage-2 video path
/// to be correct. What is *not* done is the volume and the throttle target,
/// which are stage 3.
void fceu11_gba_step_frame(void);

/// Build the GBA-side button mask from the NES input state.
///
/// Stage 1 reads the NES pad, because that is what exists: there is no GBA
/// binding table yet, and inventing one is the key-mapping work the plan puts
/// in S3. Section 7.2's mapping is A/B/Select/Start to the same, the D-pad to
/// the same, and NES's shoulder buttons onto L and R.
uint16_t fceu11_gba_button_mask(void);

// ---- stage 2': the frame (v2.0 S2-b4) ------------------------------------
//
// The GBA owns its own frame buffer rather than writing the NES pixel pool.
// Two reasons, and the second is the one that matters:
//
//  1. The pool is sized and indexed for the NES frame; a 240x160 producer
//     writing into it would leave the NES viewer reading a half-old frame.
//  2. `nes_shm::blitUpdated` is a shared "a new frame is ready" flag, and
//     reusing *that* is fine — it is the same window and the same GUI thread.
//     So the handshake is shared and the buffer is not. The NES pool is still
//     only ever written by the NES blitter.

/// The current GBA frame as RGBA, 240x160, or null when no GBA is running.
///
/// Valid until the next call to [`fceu11_gba_step_frame`]. The pointer is into
/// storage this module owns; the viewer copies it out rather than keeping it.
const uint8_t* fceu11_gba_frame();

/// Width of the frame [`fceu11_gba_frame`] returns, in pixels.
uint32_t fceu11_gba_frame_width();

/// Height of the frame [`fceu11_gba_frame`] returns, in pixels.
uint32_t fceu11_gba_frame_height();

/// Whether a frame has arrived since the viewer last asked.
///
/// Lets the viewer skip re-uploading an unchanged frame on the repaints that
/// are not frame boundaries. Bumps on every emulated frame, wraps at 32 bits,
/// and is only ever compared for inequality — a wrap is harmless.
uint32_t fceu11_gba_frame_serial();

// ---- stage 3': audio (v2.0 S2-b4) ----------------------------------------
//
// The core holds the samples; the Qt layer pushes them at the device. The
// split is not a convenience: `WriteSound` lives in the Qt driver library and
// this file is in the core library, so calling across that edge would put a
// link-time dependency on the driver into `fceux11_core` -- and the four F11QA
// test executables link the core without the driver, so every one of them would
// fail to link. That is invariant 9's allowed coupling ③, used at the only
// layer where it is safe.

/// The GBA's video frame rate, for pacing.
///
/// 16777216 / (228 * 1232): the CPU clock over the cycles in one video frame.
/// Spelled out rather than quoted as "59.7275" so the derivation is checkable
/// against `audio.rs`'s `CPU_CLOCK` and `FRAME_CYCLES`, which is where the audio
/// side derives the same number. **If either moves, the other must move with
/// it** — the two are the same hardware constant written twice, on purpose,
/// rather than a constant that would have needed a new C ABI export just to be
/// read once.
double fceu11_gba_base_rate();

/// Hand the device's rate and the host's volume to the core.
///
/// Must be called when a GBA session starts, before the first frame. The rate
/// is the *negotiated* one (what the device opened at), which is not always
/// what was asked for; the volume is the 0-150 scale the NES side uses, and it
/// has to be applied here because the sound device applies none.
void fceu11_gba_configure_audio(uint32_t device_rate, uint32_t volume);

/// Drain up to `cap` mono int32 samples of this frame's audio.
///
/// Returns the number written, or 0 when there is nothing yet. The count is
/// whatever the core's fractional accumulator says for this frame -- at 44100 Hz
/// and 59.7275 fps that is 738 or 739, alternating -- and a caller that assumed
/// a fixed 738 would drift by about 21 samples a second.
uint32_t fceu11_gba_audio(int32_t* dst, uint32_t cap);
