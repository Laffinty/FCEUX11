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

/// The state of a GBA pad, in the core's own bit order.
///
/// This is the GBA `Keypad` layout (`gba-core`'s `GbaButton`), not the NES one:
/// the two disagree about the d-pad, and the NES table is what
/// `fceu11_gba_button_mask` used to read.
enum GbaPadBit : uint16_t
{
	kGbaPadA = 1u << 0,
	kGbaPadB = 1u << 1,
	kGbaPadSelect = 1u << 2,
	kGbaPadStart = 1u << 3,
	kGbaPadRight = 1u << 4,
	kGbaPadLeft = 1u << 5,
	kGbaPadUp = 1u << 6,
	kGbaPadDown = 1u << 7,
	kGbaPadR = 1u << 8,
	kGbaPadL = 1u << 9,
};

/// Hand the pad state to the GBA module for the frame about to be stepped.
///
/// Pushed in rather than read here because this file is in the core library and
/// `Qt/input.h` cannot be included from it: that header uses
/// `FAMILYKEYBOARD_NUM_BUTTONS` before anything defines it, the same trap
/// `dface.h` sets. So the keyboard stays on the Qt side and hands over a mask,
/// built there from the bindings the user actually has.
///
/// A full state, not a delta: a button the mask does not name is released.
///
/// **Why not `joy[]`**, which is what this used to read: `joy[]` is the NES
/// side's own value, written by `UpdateGP` copying out of the NES controller
/// port register (`input.cpp:231`). A GBA session does not run the NES core,
/// so nothing writes that register, so `joy[]` stays zero and the game never
/// sees a key. That is the whole reason the eight face and d-pad buttons were
/// dead while L and R worked -- those two came from `g_keyState` instead, and
/// the eight that went through `joy[]` never had a value to read.
void fceu11_gba_set_pad_state(uint16_t mask);

/// The button mask for the frame about to be stepped.
///
/// Player 1. The NES side has only two worth of bindings to offer a GBA, and a
/// game that needs two GBA pads is not what plan section 7.2 promises.
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

// ---- the battery save (v2.0 S3-1) ---------------------------------------
//
// A `.srm` is the raw contents of the cartridge's save memory: no header, no
// length, no padding. That is the format mGBA reads and writes, and a bare
// image is the only shape another emulator is likely to accept.
//
// Which is exactly why a cartridge with **no** save hardware must refuse the
// write rather than invent 32 KB of it: the file would look like a real save,
// and the next game write would clobber it. Plan section 7.3 requires the
// refusal, and `gba_battery_write` enforces it on the core side.

/// Read `<cartridge>.srm` into the machine, if one exists.
///
/// Called after a cartridge is loaded. Silent when there is no save file or no
/// save hardware: a first run is not an error.
void fceu11_gba_load_battery(void);

/// Write the save memory out if it changed.
///
/// Called every frame, and **rate-limited**: a cartridge that rewrites its save
/// memory continuously (GT Championship does) would otherwise turn that into a
/// 32 KB file write per frame. `force` skips the limit and is what the teardown
/// and savestate paths use, so the last seconds of play are never waiting on a
/// timer. A cart with no save hardware is skipped rather than written.
void fceu11_gba_flush_battery(bool force = false);

// ---- the instant savestate (v2.0 S3-3) ------------------------------------
//
// A slot is the *same* file the NES side already writes: `FCEU_MakeFName`
// derives it from the loaded file's stem, and that call happens for a `.gba`
// too, so a GBA session gets `<stem>.fc1` and never touches a NES game's
// slots. The two formats are not interchangeable and the core knows it --
// `gba_savestate_load` compares the ROM fingerprint and refuses a state that
// belongs to another cartridge, so a slot can never be read into the wrong
// machine even by accident.
//
// The C ABI wants the simulation thread. That is not a new constraint here:
// the Qt menus reach this under `FCEU_WRAPPER_LOCK()`, which the emulation
// thread has to win inside `fceuWrapperUpdate` before it steps anything, and
// the hotkey path arrives on the emulation thread itself. The NES serialiser
// in `FCEUSS_Save` is standing on exactly the same ground.

/// Write the running machine's state to `path`.
///
/// Reports whether anything was written. A false means no GBA session, a path
/// that could not be written, or a machine that would not serialise -- the
/// caller owns the message, because it knows whether this was a slot or a
/// "Save State As".
bool fceu11_gba_savestate_save(const char* path);

/// Report that the state is read back from a real machine, replacing the
/// running one.
///
/// Reports whether the machine took it. A false leaves the running machine
/// exactly as it was: the core checks the cartridge fingerprint before it
/// touches anything, so a state from another game is a refusal rather than a
/// corrupted session.
bool fceu11_gba_savestate_load(const char* path);

// ---- the picture's size on screen (v2.0 S4) -------------------------------
//
// A GBA frame is 240x160 and never any other size, so the only question each
// video driver asks about it is "how big, and where". Both are answered here
// rather than in the drivers, because the three of them would otherwise each
// work it out separately and eventually disagree -- which is exactly what
// happened with the first version, where all three refused to scale past 1:1
// and a 240x160 picture sat in the middle of a 528x506 window looking broken.

/// The size a GBA frame is drawn at inside a `view_w` x `view_h` viewport.
///
/// **Integer multiples only.** A factor of 1.5 or 2.5 resamples 240x160 into
/// pixels that are in no one row or column of the source, and a GBA picture is
/// mostly flat-shaded sprites and crisp text, so it smears where the NES
/// path's non-integer scaling is merely soft. When the viewport cannot hold
/// even 1:1 -- a window narrower than 240 -- the fit falls back to a fractional
/// scale rather than refusing to draw, because a small correct picture beats
/// a clipped one.
///
/// The result is centred by the caller, which is what leaves the letterbox.
struct GbaDrawSize
{
	int width;
	int height;
	/// Which path produced it, for the caller's own comment or assertion. Not
	/// used for drawing.
	bool integral;
};

GbaDrawSize fceu11_gba_draw_size(int view_w, int view_h);

