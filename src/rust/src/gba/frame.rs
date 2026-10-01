//! The lifecycle and frame-buffer C ABI (v2.0 S2-a).
//!
//! # Shape, and why it is this shape
//!
//! Every function here follows the convention this repository already uses for
//! moving a large value across `extern "C"` (see
//! `fceux11-core/src/state_file.rs`): **out-parameters, plus an explicit size
//! query, plus an explicit free.** No function returns a machine state by
//! value, and none hands a pointer to memory Rust still owns.
//!
//! That last point is not style. `Arm7tdmi` is 82,032 bytes — measured, see
//! `gba_probe_cpu_size` — and a value that size returned across an FFI boundary
//! has to be materialised on the *caller's* stack, which the caller does not
//! control and, in the general case, cannot be told about. So the state lives
//! in a `static mut` on this side and the C++ side never sees it.
//!
//! # Threading
//!
//! Everything here is simulation-thread only (plan section 4.1). The static is
//! not synchronised, and deliberately so: a mutex would imply the state is
//! safe to touch from two threads, which is a stronger promise than the plan
//! makes and stronger than any caller needs.
//!
//! # What is not here
//!
//! Audio (plan section 4.2's fractional sampling) and savestates (section 4.1)
//! are separate surfaces. This file is the frame and the lifecycle, which is
//! the part jsmolka needs in order to run at all.

use std::sync::{Mutex, OnceLock};

use gba_core::gba::Gba;

use crate::gba::audio::{self, AudioOut};
use crate::gba::bios;
use crate::gba::overlay::{self, Overlay};
use crate::gba::rtc;
use crate::gba::save::{self, SaveError, WireState};
use crate::gba::swi;
use crate::gba::swi::wait::IntrWaitRequest;

/// Error codes, matching plan section 4.1's enum.
pub const GBA_OK: i32 = 0;
/// No ROM is loaded.
pub const GBA_ERR_NO_ROM: i32 = 1;
/// The ROM could not be read or is not a GBA cartridge.
pub const GBA_ERR_BAD_ROM: i32 = 2;
/// A BIOS problem: neither a user BIOS nor the built-in stub could be used.
pub const GBA_ERR_BIOS: i32 = 3;
/// The request names something this build does not implement: a savestate
/// written by a newer version, or a file that is not a GBA savestate at all.
///
/// Distinct from [`GBA_ERR_STATE`] because the caller's next move differs. A
/// state that is not ours will never load, whatever the caller does; a state
/// that is ours but does not fit *this* machine — the wrong cartridge — is
/// worth a different ROM.
pub const GBA_ERR_UNSUPPORTED: i32 = 4;
/// The request is not legal in this state — including trying to turn the
/// `BETA` watermark off in a release build, and restoring a state onto a
/// cartridge it was not taken from.
pub const GBA_ERR_STATE: i32 = 5;
/// The caller's buffer is too small. Never returned by a function that does
/// not also report the required size.
pub const GBA_ERR_CAPACITY: i32 = 6;

/// A GBA cartridge is rejected below this size.
///
/// The core's header parser slices up to `0xE4`, and in a release build a short
/// slice is a panic rather than an error. An `extern "C"` entry point that
/// aborts the emulator is worse than one that returns a code, so the length is
/// checked before the core ever sees the slice.
const MIN_CARTRIDGE: usize = 0x200;

/// The machine, owned by this side of the FFI boundary.
///
/// Wrapped rather than a bare `static mut` so that "there is no machine" is a
/// representable state instead of a null check at every call site.
pub(crate) struct Machine {
    pub(crate) gba: Box<Gba>,
    overlay: Overlay,
    /// The audio sink and its per-frame accounting (S2-b1). Owned here for the
    /// same reason the machine is: the ring's consumer end has to be stored on
    /// this side, and it has to die with the machine.
    pub(crate) audio: AudioOut,
}

impl Machine {
    fn new(rom: &[u8]) -> Self {
        let mut gba = Box::new(Gba::new(bios::stub(), rom));
        crate::gba::install_swi_hook(&mut gba);
        // The host rate is whatever `gba_set_output_rate` last recorded. Read
        // it here rather than storing it on the machine, so a rate set before
        // the ROM was loaded still lands on this machine.
        let rate = audio::configured_rate();
        let rx = gba.init_audio(rate, audio::RING_SLOTS);
        let mut audio = AudioOut::new(rate);
        audio.set_volume(audio::configured_volume());
        audio.attach(rx, rate);
        Self {
            gba,
            // A debug build may turn the watermark off; a release build may
            // not. See plan section 7.1 and invariant 5.
            overlay: Overlay::for_build(cfg!(not(debug_assertions))),
            audio,
        }
    }
}

/// The one machine this process has, behind a mutex.
///
/// # Why a mutex, given the plan says simulation-thread only
///
/// The plan's rule is about the *C++ caller*: it promises not to call these
/// functions from two threads at once. It does not make the state sound on
/// its own, and Rust's test harness runs tests in parallel -- several of them
/// load and unload a machine, and a `Box<Gba>` being dropped on one thread
/// while another installs one is a double free, not a race the caller can
/// prevent. The first version of this file used a bare `static mut` and the
/// suite died with `STATUS_HEAP_CORRUPTION`.
///
/// The lock is uncontended in real use, so it costs nothing on the frame path,
/// and it makes "simulation thread only" a promise the code keeps rather than
/// one it merely asserts.
fn machine_slot() -> &'static Mutex<Option<Machine>> {
    static SLOT: OnceLock<Mutex<Option<Machine>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Run `body` against the machine, or report that none is loaded.
///
/// # Visibility
/// `pub(crate)` because `audio.rs` has to reach the same machine: the ring's
/// consumer end and the frame accounting both live on it, and duplicating the
/// lock here would put two guards on one state.
///
/// A poisoned lock means a previous call panicked while holding it. The state
/// is then of unknown validity, so it is dropped rather than reused: an
/// emulator that keeps going on a half-torn-down machine is worse than one
/// that reports "no ROM".
pub(crate) fn with_machine<R>(body: impl FnOnce(&mut Machine) -> R) -> Result<R, i32> {
    let mut guard = match machine_slot().lock() {
        Ok(guard) => guard,
        Err(_) => {
            // Take the state out and let it drop, so the next call starts clean.
            if let Ok(mut slot) = machine_slot().try_lock() {
                *slot = None;
            }
            return Err(GBA_ERR_STATE);
        }
    };
    guard.as_mut().map(body).ok_or(GBA_ERR_NO_ROM)
}

/// Install a machine, replacing whatever was there.
///
/// Dropping the old machine while holding the lock is what keeps the two from
/// overlapping; a `Box<Gba>` moved out and freed on another thread is exactly
/// what produced the heap corruption.
fn set_machine(machine: Machine) {
    if let Ok(mut slot) = machine_slot().lock() {
        *slot = Some(machine);
    }
}

/// Prepare the GBA side. Idempotent, and safe to call before any ROM is
/// loaded.
///
/// Returns `GBA_OK`. Kept for the plan's `gba_init` and because a caller
/// should not have to guess whether a previous teardown happened.
#[unsafe(no_mangle)]
pub extern "C" fn gba_init() -> i32 {
    GBA_OK
}

/// Whether a cartridge is loaded.
#[unsafe(no_mangle)]
pub extern "C" fn gba_rom_loaded() -> i32 {
    // A count, not a status: `with_machine` hands back whatever the closure
    // returned, so the closure is what decides. Returning `GBA_OK` here would
    // report "loaded" as 0 and "not loaded" as 1 -- backwards.
    // SAFETY: simulation thread.
    match with_machine(|_| 1i32) {
        Ok(loaded) => loaded,
        Err(_) => 0,
    }
}

/// Drop the machine, if there is one. Returns `GBA_OK` either way, so a
/// caller can tear down without first asking whether it is loaded.
#[unsafe(no_mangle)]
pub extern "C" fn gba_unload_rom() -> i32 {
    set_machine_unload();
    GBA_OK
}

/// Drop the machine, if there is one.
fn set_machine_unload() {
    if let Ok(mut slot) = machine_slot().lock() {
        *slot = None;
    }
}

/// Last error text, for the C++ side to show.
///
/// Deliberately thin: the codes carry the meaning, and a message that could
/// disagree with the code is worse than no message. Returns an empty string
/// rather than a null, so the caller never has to check.
#[unsafe(no_mangle)]
pub extern "C" fn gba_last_error(_dst: *mut u8, _cap: u32) -> i32 {
    // Filled in when an operation has something worth saying. The signature is
    // already fixed by the plan so the C++ side can be written against it now.
    GBA_OK
}

/// Load a cartridge from a file.
///
/// `path` is UTF-8 and NUL-terminated, per plan section 4.1.
///
/// # Safety
/// `path` must point to a valid NUL-terminated string, and the caller must be
/// the simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_load_rom(path: *const u8) -> i32 {
    if path.is_null() {
        return GBA_ERR_BAD_ROM;
    }
    let Some(bytes) = read_c_string(path) else {
        return GBA_ERR_BAD_ROM;
    };
    // SAFETY: `path` is non-null and NUL-terminated; `read_c_string` stops at
    // the terminator.
    let Ok(rom) = std::fs::read(unsafe { std::str::from_utf8_unchecked(bytes) }) else {
        return GBA_ERR_BAD_ROM;
    };
    gba_load_rom_bytes(&rom)
}

/// The part of loading that does not touch the filesystem, so the rule about
/// short cartridges can be tested without a file.
fn gba_load_rom_bytes(rom: &[u8]) -> i32 {
    if rom.len() < MIN_CARTRIDGE {
        return GBA_ERR_BAD_ROM;
    }
    set_machine(Machine::new(rom));
    GBA_OK
}

/// Reset the machine, keeping the cartridge.
///
/// Note that the *core* reset is still a stub -- this has always returned
/// `GBA_OK` without touching the CPU, and a real reset needs `SoftReset`
/// (`0x00`) semantics that belong with the savestate work in S2-b2. What it
/// does do is clear the audio frame accounting, because a residual carried
/// across a reset is a frame boundary from the previous run, and the underrun
/// total should be per-run.
#[unsafe(no_mangle)]
pub extern "C" fn gba_reset() -> i32 {
    // SAFETY: simulation thread.
    with_machine(|machine| {
        machine.audio.reset();
        GBA_OK
    })
    .unwrap_or(GBA_ERR_NO_ROM)
}

/// Bytes one frame occupies: 240x160 RGBA, watermark included.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_frame_buffer_size(out_size: *mut u32) -> i32 {
    if out_size.is_null() {
        return GBA_ERR_STATE;
    }
    // SAFETY: checked non-null above.
    unsafe { *out_size = overlay::FRAME_BYTES as u32 };
    GBA_OK
}

/// Copy the current frame into `dst` as RGBA.
///
/// `dst` must be at least [`gba_frame_buffer_size`] bytes. The `BETA`
/// watermark is drawn on the way out, so it lands in every copy this function
/// produces — window, screenshot and recording alike, because there is only
/// one place it is drawn.
///
/// # Safety
/// `dst` must point to `cap` writable bytes, and the caller must be the
/// simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_frame_buffer(dst: *mut u8, cap: u32) -> i32 {
    if dst.is_null() || cap < overlay::FRAME_BYTES as u32 {
        return GBA_ERR_CAPACITY;
    }
    // SAFETY: simulation thread; `dst` is non-null and at least FRAME_BYTES.
    unsafe {
        with_machine(|machine| {
            let mut frame = vec![0u8; overlay::FRAME_BYTES];
            frame.copy_from_slice(&frame_bytes(machine));
            if machine.overlay.enabled() {
                overlay::draw(&mut frame);
            }
            std::ptr::copy_nonoverlapping(frame.as_ptr(), dst, overlay::FRAME_BYTES);
            GBA_OK
        })
    }
    .unwrap_or(GBA_ERR_NO_ROM)
}

/// Ask for the `BETA` watermark to be turned off.
///
/// A release build refuses with `GBA_ERR_STATE` and leaves it on. A caller
/// that checked the return value is therefore never misled about whether the
/// mark is present.
#[unsafe(no_mangle)]
pub extern "C" fn gba_set_overlay(enable: i32) -> i32 {
    let want = enable != 0;
    with_machine(|machine| {
        let (after, allowed) = machine.overlay.set_enabled(want);
        machine.overlay = after;
        if allowed { GBA_OK } else { GBA_ERR_STATE }
    })
    .unwrap_or(GBA_ERR_NO_ROM)
}

/// Run until the next vertical blank, then return.
///
/// One frame of emulation, per the plan's `gba_step_frame`. `Gba::step`
/// returns `true` on entering VBlank, which is the frame boundary.
#[unsafe(no_mangle)]
pub extern "C" fn gba_step_frame() -> i32 {
    with_machine(|machine| {
        // Bounded so a machine wedged before it ever reaches VBlank -- a
        // game looping on an unclaimed SWI, say -- returns instead of
        // hanging the emulator. The bound is far above a real frame: one
        // GBA frame is 280,896 cycles.
        const CYCLE_LIMIT: u64 = 16 * 280_896;
        let mut cycles = 0u64;
        while cycles < CYCLE_LIMIT {
            if machine.gba.step() {
                break;
            }
            cycles += 1;
        }
        // The audio for this frame is whatever the core pushed while the CPU
        // ran, so the drain belongs here and not in `gba_render_audio`: by the
        // time a caller asks for samples the core may already be half a frame
        // into the next one. Draining on a wedged machine is still correct --
        // it produces the silence the clock asks for, and counts it, rather
        // than leaving last frame's samples to be replayed.
        machine.audio.advance_frame();
        GBA_OK
    })
    .unwrap_or(GBA_ERR_NO_ROM)
}

// ---- savestates (v2.0 S2-b2) ------------------------------------------------
//
// The shape is section 4.1's: the caller owns the buffer. There is no
// `gba_savestate_free` here, and that is deliberate — section 4.1's archive
// list does not have one either, because `gba_savestate_save` writes into a
// buffer the caller already owns. Plan r31 ③ had proposed a `free` for a
// shape where we allocate; r37 ⑤ resolved the conflict in favour of the
// section, and with it goes the `cap` round-trip r31 ④ was about.
//
// The stack: encoding runs here, on the caller's thread, where the measurement
// says 48 KB is enough. Decoding does not fit there -- it needs 2 MB and a
// `QThread` has 1 -- so `gba_savestate_load` hands it to a worker with its
// own. See `save.rs` for the numbers.

/// The whole state as bytes, or an error code.
fn encode_machine(machine: &Machine) -> Result<Vec<u8>, i32> {
    let wire = save::save(
        &machine.gba.cpu,
        &save::RomFingerprint::of(&machine.gba),
        save::AudioState::capture(&machine.audio),
        swi::pending_intr_wait().map(save::PendingWait::of),
    )
    .map_err(|_| GBA_ERR_STATE)?;
    let mut bytes = Vec::with_capacity(wire.header.len() + wire.payload.len());
    bytes.extend_from_slice(&wire.header);
    bytes.extend_from_slice(&wire.payload);
    Ok(bytes)
}

/// Put a decoded state back into the one machine this process has.
///
/// Four things did not travel with the state and have to be rebuilt here,
/// which is the whole content of this function:
///
/// 1. **the ROM and the BIOS** — the core marks them `#[serde(skip)]`, so the
///    decoded machine has neither. [`Gba::new`] takes the cartridge back and
///    rebuilds the header, which is a plain struct with no `Serialize` derive
///    and therefore could not have been in the payload either;
/// 2. **the SWI hook**, a function pointer the derive cannot carry;
/// 3. **the audio ring**, a channel rather than data — and with it the
///    residual, which is only meaningful if the rate has not changed;
/// 4. **a pending `IntrWait`**, which lives in a `thread_local` and is
///    invisible to serde entirely.
///
/// The swap is `mem::swap` rather than an assignment on purpose: an assignment
/// would move 82 KB through this frame, and the whole arrangement of this
/// module exists so that no machine-sized value ever sits on a caller's stack.
fn apply_state(machine: &mut Machine, mut wire: WireState) -> i32 {
    // The ROM is not in the payload, so a state only means anything next to
    // the cartridge it was taken from. Checking this first means a refusal
    // leaves the running machine untouched.
    if save::RomFingerprint::of(&machine.gba) != wire.fingerprint {
        return GBA_ERR_STATE;
    }

    let rom = std::mem::take(&mut machine.gba.cpu.bus.internal_memory.rom);
    let mut fresh = Box::new(Gba::new(bios::stub(), &rom));
    std::mem::swap(&mut fresh.cpu, wire.cpu.as_mut());
    machine.gba = fresh;

    crate::gba::install_swi_hook(&mut machine.gba);
    let pending: Option<IntrWaitRequest> = wire.pending_wait.map(save::PendingWait::to_request);
    swi::restore_intr_wait(pending, &mut machine.gba.cpu);

    // A new ring, then the residual on top of the clock `attach` just reset.
    // The watermark is a build-level setting rather than machine state, so it
    // is left alone; the volume is the host's, so it is left alone too.
    let rate = audio::configured_rate();
    let rx = machine.gba.init_audio(rate, audio::RING_SLOTS);
    machine.audio.attach(rx, rate);
    machine.audio.clear_underruns();
    if wire.audio.rate() == rate {
        machine.audio.restore_clock(wire.audio.phase(), rate);
    }
    // A residual counted in another rate's units is not a residual, and the
    // alternative -- honouring it -- is the sub-sample-per-frame drift section
    // 4.2 exists to rule out. `attach` has already left the clock at zero,
    // which is the only correct answer in that case.

    GBA_OK
}

/// Bytes one savestate occupies, header included.
///
/// Reports the real number by writing one and measuring it. The payload is
/// JSON, so its length is not something that can be computed without doing the
/// work, and an estimate that is too small would make `gba_savestate_save` fail
/// in a way the caller cannot tell from a corrupt state.
///
/// A caller that would rather not pay for the extra encode can skip this and
/// pass a generous buffer, then read the size `gba_savestate_save` reports when
/// it returns `GBA_ERR_CAPACITY`.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_savestate_size(out_size: *mut u32) -> i32 {
    if out_size.is_null() {
        return GBA_ERR_STATE;
    }
    match with_machine(|machine| encode_machine(machine)) {
        Ok(Ok(bytes)) => {
            let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
            // SAFETY: checked non-null above.
            unsafe { *out_size = len };
            GBA_OK
        }
        Ok(Err(code)) => code,
        Err(code) => code,
    }
}

/// Write the current state into `dst`.
///
/// `dst` must be at least [`gba_savestate_size`] bytes. If it is not, nothing
/// is written and `*out_written` receives the size that would have worked, so
/// one retry is enough.
///
/// # Safety
/// `dst` must point to `cap` writable bytes, `out_written` to a writable
/// `u32`, and the caller must be the simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_savestate_save(
    dst: *mut u8,
    cap: u32,
    out_written: *mut u32,
) -> i32 {
    if dst.is_null() || out_written.is_null() {
        return GBA_ERR_STATE;
    }
    let bytes = match with_machine(|machine| encode_machine(machine)) {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(code)) | Err(code) => return code,
    };
    let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    // SAFETY: non-null pointers checked above; the copy is bounded by `cap`.
    unsafe {
        if cap < len {
            *out_written = len;
            return GBA_ERR_CAPACITY;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        *out_written = len;
    }
    GBA_OK
}

/// Restore a state, replacing whatever the machine is doing.
///
/// The state's cartridge must be the one that is loaded; a mismatch is
/// `GBA_ERR_STATE` and the running machine is left alone.
///
/// # Safety
/// `src` must point to `len` readable bytes, and the caller must be the
/// simulation thread. Section 4.1 already confines every call in this file to
/// that one thread, and this one depends on it: the pending `IntrWait` is a
/// `thread_local`, so a load on another thread would arm the wait somewhere the
/// CPU will never look.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_savestate_load(src: *const u8, len: u32) -> i32 {
    if src.is_null() {
        return GBA_ERR_STATE;
    }
    // Judge the machine before the file. A caller with nothing loaded has a
    // bigger problem than a bad file, and every other function in this file
    // answers `GBA_ERR_NO_ROM` first — returning "unsupported" here would make
    // a caller's "no cartridge" look like "your save is from the future".
    if with_machine(|_| ()).is_err() {
        return GBA_ERR_NO_ROM;
    }
    // SAFETY: the caller guarantees `len` readable bytes at `src`.
    let bytes = unsafe { std::slice::from_raw_parts(src, len as usize) }.to_vec();
    let wire = match save::load_on_worker(bytes) {
        None => return GBA_ERR_STATE,
        Some(Err(SaveError::NotOurs | SaveError::FutureVersion(_))) => return GBA_ERR_UNSUPPORTED,
        Some(Err(SaveError::Payload)) => return GBA_ERR_STATE,
        Some(Ok(wire)) => wire,
    };
    with_machine(|machine| apply_state(machine, wire)).unwrap_or(GBA_ERR_NO_ROM)
}

/// Pin the cartridge real-time clock to a fixed moment, or release it.
///
/// `unix_secs` is seconds since the Unix epoch, the same unit
/// [`gba_rtc_time`] reports. `enable` is the switch: zero releases the pin and
/// the machine follows the host clock again, and any other value pins it.
///
/// The switch is a parameter rather than a sentinel value on purpose. An
/// earlier draft used `0` to mean "release", which quietly made the Unix epoch
/// itself impossible to pin -- and the epoch is the one instant a test wants to
/// be able to state, because its expected date is computable by hand.
///
/// A pinned clock does not advance. That is the point of it: a lock test needs
/// a date it can state, and someone debugging a game's clock wants the date they
/// picked. A game that wants time to pass has the host clock, which is the
/// default.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub extern "C" fn gba_rtc_set_time(unix_secs: i64, enable: i32) -> i32 {
    // SAFETY: simulation thread.
    with_machine(|machine| {
        if enable != 0 {
            rtc::set_time_override(&mut machine.gba, unix_secs);
        } else {
            rtc::clear_time_override(&mut machine.gba);
        }
        GBA_OK
    })
    .unwrap_or(GBA_ERR_NO_ROM)
}

/// The moment the cartridge's real-time clock reports right now.
///
/// Whatever [`gba_rtc_set_time`] pinned, or the host clock when nothing is
/// pinned. This is the value the game would read over GPIO, not the state of
/// the pin — the two differ exactly when the clock is following the host, and
/// a caller that wants to know which it is has the value either way.
///
/// # Safety
/// `out_unix_secs` must point to a writable `i64`, and the caller must be the
/// simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_rtc_time(out_unix_secs: *mut i64) -> i32 {
    if out_unix_secs.is_null() {
        return GBA_ERR_STATE;
    }
    match with_machine(|machine| machine.gba.cpu.bus.internal_memory.rtc().time_override()) {
        Ok(pinned) => {
            let now = pinned.unwrap_or_else(rtc::now_unix_secs);
            // SAFETY: checked non-null above.
            unsafe { *out_unix_secs = now };
            GBA_OK
        }
        Err(code) => code,
    }
}

/// Read a NUL-terminated byte string, or `None` if it is not valid UTF-8.
fn read_c_string<'a>(ptr: *const u8) -> Option<&'a [u8]> {
    let mut len = 0usize;
    // SAFETY: the caller guarantees a NUL terminator exists; we stop at the
    // first one, so we never read past it.
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        Some(std::slice::from_raw_parts(ptr, len))
    }
}

/// Expand the core's 5-5-5 BGR words into the RGBA bytes the ABI promises.
fn frame_bytes(machine: &Machine) -> Vec<u8> {
    let buffer = &machine.gba.cpu.bus.lcd.buffer;
    let mut out = Vec::with_capacity(overlay::FRAME_BYTES);
    for row in buffer.iter() {
        for pixel in row.iter() {
            // The core's 15-bit colour is BGR with five bits per channel and
            // a clear top bit; the ABI promises 8-bit RGBA, so each channel is
            // scaled by bit replication rather than shifted, or a pure red
            // would come out 248 instead of 255.
            out.push(scale5(pixel.blue()));
            out.push(scale5(pixel.green()));
            out.push(scale5(pixel.red()));
            out.push(0xFF);
        }
    }
    out
}

/// Widen a 5-bit channel to 8 bits by replicating the high bits.
///
/// `0b00000 -> 0`, `0b11111 -> 255`, and everything in between stays
/// monotonic -- a plain `<< 3` would map 31 to 248 and leave the brightest
/// value short of white, which is visible on a saturated screen.
fn scale5(value: u8) -> u8 {
    debug_assert!(value < 32);
    (value << 3) | (value >> 2)
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{
        GBA_ERR_BAD_ROM, GBA_ERR_BIOS, GBA_ERR_CAPACITY, GBA_ERR_NO_ROM, GBA_ERR_STATE,
        GBA_ERR_UNSUPPORTED, GBA_OK, MIN_CARTRIDGE, gba_frame_buffer, gba_frame_buffer_size,
        gba_init, gba_load_rom_bytes, gba_reset, gba_rom_loaded, gba_rtc_set_time, gba_rtc_time,
        gba_savestate_load,
        gba_savestate_save, gba_savestate_size, gba_set_overlay, gba_step_frame, gba_unload_rom,
        read_c_string, scale5, with_machine,
    };
    use crate::gba::audio::{
        gba_audio_underruns, gba_render_audio, gba_samples_per_frame_fixed, gba_set_output_rate,
    };
    use crate::gba::overlay::{FRAME_BYTES, SCREEN_HEIGHT, SCREEN_WIDTH};
    use crate::gba::save;
    use crate::gba::swi;
    use crate::gba::swi::wait::IntrWaitRequest;

    /// A cartridge the core will accept: a real entry point, and a `SWI 0`
    /// reset at the start so the machine has something to do.
    fn cartridge() -> Vec<u8> {
        let mut rom = vec![0u8; 0x200];
        // Branch at 0x00 to 0xC0, where a `SWI 0` (SoftReset) sits.
        rom[0] = 0x04;
        rom[1] = 0x00;
        rom[2] = 0x00;
        rom[3] = 0xEA;
        for (i, word) in rom[0xC0..0xC4].chunks_exact_mut(4).enumerate() {
            let _ = i;
            word.copy_from_slice(&0xEF00_0000u32.to_le_bytes());
        }
        rom
    }

    /// Run a test body with exclusive use of the machine.
    ///
    /// The mutex in [`machine_slot`] makes concurrent access *safe*, but these
    /// tests also need it to be *isolated*: they load, unload and reconfigure
    /// the one machine this process has, and the harness runs them in
    /// parallel. Without this a test that switches the watermark off finds
    /// another test's frame in the buffer, and fails for a reason that has
    /// nothing to do with the code.
    ///
    /// The lock is taken on a dedicated thread so the guard lives exactly as
    /// long as the body.
    ///
    /// A poisoned lock — a test that panicked while holding it — is recovered
    /// from rather than propagated, because the alternative is worse than the
    /// failure it hides: a panicking test poisons the mutex, every later test's
    /// `lock()` returns `Err`, and if that is ignored the tests run *without*
    /// isolation and fail for reasons that have nothing to do with them. That
    /// is not hypothetical — it is what happened the first time this suite ran
    /// a test that failed, and one real failure turned into fourteen.
    fn exclusively(body: impl FnOnce() + Send + 'static) {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let lock = TEST_LOCK.get_or_init(|| Mutex::new(()));
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                    body();
                })
                .join()
                .expect("the test body must not have panicked");
        });
    }

    /// Load a machine and leave it loaded for the test that follows.
    fn with_loaded_machine() {
        gba_unload_rom();
        assert_eq!(gba_load_rom_bytes(&cartridge()), GBA_OK);
    }

    /// The lifecycle reports its own state, and unload is idempotent.
    #[test]
    fn the_lifecycle_reports_whether_a_rom_is_loaded() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(gba_rom_loaded(), 0, "nothing is loaded yet");
            assert_eq!(gba_reset(), GBA_ERR_NO_ROM, "and reset says so rather than crashing");
            // SAFETY: single-threaded test, per the module's threading rule.
            assert_eq!(gba_step_frame(), GBA_ERR_NO_ROM);
            assert_eq!(gba_unload_rom(), GBA_OK, "unloading nothing is not an error");
            assert_eq!(gba_unload_rom(), GBA_OK, "and is idempotent");

            with_loaded_machine();
            assert_eq!(gba_rom_loaded(), 1);
            assert_eq!(gba_init(), GBA_OK, "init is idempotent");
            assert_eq!(gba_reset(), GBA_OK);
        });
    }

    /// The status and the boolean are not the same value.
    ///
    /// `gba_rom_loaded` is a count, while every other function here returns a
    /// status where `GBA_OK` is zero. A body that returns `GBA_OK` from a
    /// shared `with_machine` helper therefore reports "loaded" as **0** and
    /// "not loaded" as **1** — backwards, and silently, because both are
    /// plausible-looking integers. This test states the convention outright so
    /// the next person does not reintroduce it.
    #[test]
    fn loaded_reports_one_and_ok_reports_zero() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(gba_rom_loaded(), 0, "not loaded is 0");
            assert_eq!(gba_init(), GBA_OK, "and GBA_OK is 0");
            with_loaded_machine();
            assert_eq!(
                gba_rom_loaded(),
                1,
                "loaded is 1 — not GBA_OK, which is 0"
            );
        });
    }

    /// A cartridge shorter than the header is refused, not handed to the core.
    ///
    /// The core's parser slices to `0xE4` and a short slice panics in a
    /// release build — which, across `extern "C"`, takes the emulator with it.
    #[test]
    fn a_short_cartridge_is_refused_before_the_core_sees_it() {
        exclusively(|| {
            gba_unload_rom();
            for length in [0usize, 1, 0xC0, MIN_CARTRIDGE - 1] {
                assert_eq!(
                    gba_load_rom_bytes(&vec![0u8; length]),
                    GBA_ERR_BAD_ROM,
                    "a {length}-byte cartridge must be refused"
                );
            }
            assert_eq!(gba_load_rom_bytes(&vec![0u8; MIN_CARTRIDGE]), GBA_OK);
        });
    }

    /// The frame is 240x160 RGBA, and that is what the size query says.
    #[test]
    fn the_frame_size_is_answered_before_the_frame_is_copied() {
        exclusively(|| {
            let mut size = 0u32;
            assert_eq!(
                unsafe { gba_frame_buffer_size(&raw mut size) },
                GBA_OK,
                "the size must be answerable even with no ROM"
            );
            assert_eq!(size as usize, FRAME_BYTES);
            assert_eq!(size as usize, SCREEN_WIDTH * SCREEN_HEIGHT * 4);

            with_loaded_machine();
            let mut out = vec![0u8; FRAME_BYTES];
            assert_eq!(unsafe { gba_frame_buffer(out.as_mut_ptr(), size) }, GBA_OK);
        });
    }

    /// A buffer that is too small is refused rather than overrun.
    #[test]
    fn a_short_frame_buffer_is_refused() {
        exclusively(|| {
            with_loaded_machine();
            let mut small = vec![0u8; 16];
            assert_eq!(
                unsafe { gba_frame_buffer(small.as_mut_ptr(), 16) },
                GBA_ERR_CAPACITY
            );
            // A null pointer is refused the same way, not dereferenced.
            assert_eq!(
                unsafe { gba_frame_buffer_size(std::ptr::null_mut()) },
                GBA_ERR_STATE
            );
            assert_eq!(
                unsafe { gba_frame_buffer(std::ptr::null_mut(), FRAME_BYTES as u32) },
                GBA_ERR_CAPACITY
            );
        });
    }

    /// The frame comes out RGBA, and the game's own pixels are fully opaque.
    ///
    /// The watermark is the one exception: it is written at alpha 240 so it
    /// reads as an overlay rather than as a hole. That is why the two are
    /// told apart here rather than by a blanket "every alpha is 255".
    #[test]
    fn the_frame_is_rgba_with_opaque_pixels() {
        exclusively(|| {
            with_loaded_machine();
            let mut out = vec![0u8; FRAME_BYTES];
            // SAFETY: single-threaded test, per the module's threading rule.
            unsafe {
                assert_eq!(
                    gba_frame_buffer(out.as_mut_ptr(), FRAME_BYTES as u32),
                    GBA_OK
                );
            }
            for (i, pixel) in out.chunks_exact(4).enumerate() {
                assert!(
                    pixel[3] == 0xFF || pixel[3] == 240,
                    "pixel {i} has an alpha that is neither opaque nor watermark: {pixel:?}"
                );
                // The watermark is white at 240; everything else is opaque.
                if pixel[3] == 240 {
                    assert_eq!(&pixel[..3], &[255, 255, 255], "a watermark pixel at {i}");
                }
            }
            // And the overwhelming majority is opaque, i.e. this is not a frame
            // that is somehow all watermark.
            let opaque = out.chunks_exact(4).filter(|p| p[3] == 0xFF).count();
            assert!(opaque > FRAME_BYTES / 4 / 2, "only {opaque} opaque pixels");
        });
    }

    /// 5-bit channels widen to the full 8-bit range.
    #[test]
    fn a_channel_scales_to_the_full_byte_range() {
        assert_eq!(scale5(0), 0);
        assert_eq!(scale5(31), 255, "full white must be 255, not 248");
        // Monotonic, and never a wrap.
        let mut previous = 0u8;
        for value in 0..32u8 {
            let got = scale5(value);
            assert!(got >= previous, "scaling is not monotonic at {value}");
            assert_eq!(got, (value << 3) | (value >> 2));
            previous = got;
        }
    }

    /// Stepping a frame advances the machine.
    #[test]
    fn stepping_a_frame_returns_and_advances() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_step_frame(), GBA_OK);
            // And it is repeatable: a second frame also completes.
            assert_eq!(gba_step_frame(), GBA_OK);
        });
    }

    /// The watermark reaches the caller's buffer.
    ///
    /// This is the only assertion that proves the overlay is on the frame path
    /// at all: a build that compiled the overlay but never called it would pass
    /// every other test here.
    #[test]
    fn the_watermark_reaches_the_caller_buffer() {
        exclusively(|| {
            with_loaded_machine();
            let mut out = vec![0u8; FRAME_BYTES];
            // SAFETY: single-threaded test, per the module's threading rule.
            unsafe {
                assert_eq!(
                    gba_frame_buffer(out.as_mut_ptr(), FRAME_BYTES as u32),
                    GBA_OK
                );
            }
            // The watermark is the semi-transparent white; the game's own pixels
            // are opaque. Anything that is neither is a bug, covered above.
            let watermark = out.chunks_exact(4).filter(|p| p[3] == 240).count();
            assert!(watermark > 0, "no watermark pixel reached the caller's buffer");
        });
    }

    /// Whether the watermark can be turned off depends on the build, and the
    /// return value is what tells the caller.
    #[test]
    fn turning_the_watermark_off_answers_honestly() {
        exclusively(|| {
            with_loaded_machine();
            let result = gba_set_overlay(0);
            if result == GBA_OK {
                // A test build: the mark really is gone from the next frame.
                let mut out = vec![0u8; FRAME_BYTES];
                assert_eq!(
                    unsafe { gba_frame_buffer(out.as_mut_ptr(), FRAME_BYTES as u32) },
                    GBA_OK
                );
                assert!(
                    out.chunks_exact(4).all(|p| p[3] == 0xFF),
                    "the caller was told the watermark is off, and it is not"
                );
            } else {
                // A release build: it refused, and the mark is still there.
                assert_eq!(result, GBA_ERR_STATE, "an unexplained refusal");
                let mut out = vec![0u8; FRAME_BYTES];
                assert_eq!(
                    unsafe { gba_frame_buffer(out.as_mut_ptr(), FRAME_BYTES as u32) },
                    GBA_OK
                );
                assert!(
                    out.chunks_exact(4).any(|p| p[3] != 0xFF),
                    "a release build must keep the watermark"
                );
            }
            // Put it back so later tests are not order-dependent.
            assert_eq!(gba_set_overlay(1), GBA_OK);
        });
    }

    /// A NUL-terminated path is read up to the terminator and no further.
    #[test]
    fn a_c_string_stops_at_its_terminator() {
        let bytes = b"roms/game.gba\0trailing garbage that must not be read";
        let got = read_c_string(bytes.as_ptr()).expect("valid");
        assert_eq!(std::str::from_utf8(got).expect("utf8"), "roms/game.gba");
    }

    // ---- audio, across the machine (S2-b1) ---------------------------------
    //
    // These live here rather than in `audio.rs` because they touch the one
    // global machine, and `exclusively` is the only thing that makes that
    // safe *and* isolated. A second lock in `audio.rs` would be a second
    // guard on the same state, which is the mistake r32 already made once.

    /// The per-frame ratio, cross-multiplied against the unreduced one.
    ///
    /// `rate * 280896 / 16777216`, kept unreduced so this is an independent
    /// check on the reduction rather than a restatement of it.
    fn assert_ratio_is(rate: u32, num: u32, den: u32) {
        assert_eq!(
            u64::from(num) * 16_777_216,
            u64::from(den) * u64::from(rate) * 280_896,
            "{num}/{den} is not {rate} * 280896 / 16777216"
        );
    }

    /// A rate set with no ROM loaded still reaches the machine built later.
    ///
    /// The C++ side initialises audio before it knows what it is loading, so
    /// this ordering is the normal one rather than an edge case.
    #[test]
    fn a_rate_set_before_a_rom_is_loaded_reaches_the_machine_that_loads_afterwards() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(
                gba_set_output_rate(22_050),
                GBA_OK,
                "a rate set with no machine is remembered, not refused"
            );
            with_loaded_machine();
            let (num, den) = with_machine(|machine| machine.audio.ratio()).expect("loaded");
            assert_ratio_is(22_050, num, den);
            gba_set_output_rate(44_100);
        });
    }

    /// The first frame is short by construction, and nothing drifts after it.
    ///
    /// This is the end-to-end pass: the real core, the real ring, the real
    /// clock. It checks the *plumbing and the count*, not that a game makes a
    /// sound -- the synthetic cartridge never programs `SOUNDCNT_L`, so the
    /// samples are silence. The number is the thing under test.
    ///
    /// The startup transient is real and is not a fault. Emulation begins at
    /// the top of a frame, but `Gba::step` reports VBlank at scanline 160 of
    /// 228, so the first drain happens after 197,120 cycles rather than a full
    /// 280,896. At 44.1 kHz that is 518 samples against the 738 the clock asks
    /// for: a 220-sample shortfall, about five milliseconds, once per ROM load.
    ///
    /// What matters is that it is a *constant* offset rather than a rate error.
    /// Measured over 100 frames at 44.1 kHz, `underruns` goes 220 at frame 1,
    /// 221 by frame 20, and then does not move again -- and the ring settles at
    /// two slots. A drift of the kind R12 describes would instead grow in
    /// proportion to the frame count, and that is what the second half of this
    /// test rules out. The 30-minute run of plan section 8 remains a manual
    /// exit criterion; a unit test covering 100 frames is what a unit test can
    /// honestly assert.
    #[test]
    fn the_startup_transient_is_a_constant_offset_and_does_not_drift() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_set_output_rate(44_100), GBA_OK);

            for _ in 0..20 {
                assert_eq!(gba_step_frame(), GBA_OK);
            }
            let settled = gba_audio_underruns();
            assert!(
                (200..=260).contains(&settled),
                "the startup transient is about 220 samples, got {settled}"
            );

            for _ in 0..80 {
                assert_eq!(gba_step_frame(), GBA_OK);
            }
            let later = gba_audio_underruns();
            assert!(
                later - settled <= 2,
                "the shortfall grew from {settled} to {later} over 80 frames: that is drift"
            );
        });
    }

    /// A drained frame has the length the clock's own fraction says it has.
    #[test]
    fn a_stepped_frame_delivers_audio_of_the_calculated_length() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_set_output_rate(44_100), GBA_OK);
            // Two frames: the first is the startup transient described above,
            // and the second is a whole one.
            assert_eq!(gba_step_frame(), GBA_OK);
            assert_eq!(gba_step_frame(), GBA_OK);

            let (mut num, mut den) = (0u32, 0u32);
            // SAFETY: both out-pointers are live locals.
            assert_eq!(
                unsafe { gba_samples_per_frame_fixed(&mut num, &mut den) },
                GBA_OK
            );
            assert_ratio_is(44_100, num, den);
            let lo = num / den;
            let hi = lo + 1;

            let mut buffer = vec![0i32; 8_192];
            let mut written = 0u32;
            // SAFETY: the buffer has 8192 elements, well over one frame.
            let status =
                unsafe { gba_render_audio(buffer.as_mut_ptr(), buffer.len() as u32, &mut written) };
            assert_eq!(status, GBA_OK);
            assert!((lo..=hi).contains(&written), "{written} is not {lo} or {hi}");
            assert!(written > 0, "a frame carries audio, not nothing");
        });
    }

    /// Reading audio with nothing loaded says so, rather than returning zeroes
    /// that look like a working stream.
    #[test]
    fn render_audio_without_a_rom_reports_no_rom() {
        exclusively(|| {
            gba_unload_rom();
            let mut buffer = vec![0i32; 64];
            let mut written = 0u32;
            // SAFETY: the buffer has 64 elements and `written` is a local.
            let status = unsafe {
                gba_render_audio(buffer.as_mut_ptr(), buffer.len() as u32, &mut written)
            };
            assert_eq!(status, GBA_ERR_NO_ROM);
            assert_eq!(gba_audio_underruns(), 0);
        });
    }

    /// A zero rate is refused instead of silently producing an empty stream.
    #[test]
    fn a_zero_sample_rate_is_refused() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(
                gba_set_output_rate(0),
                GBA_ERR_STATE,
                "the core's own resampler drops every sample at rate 0"
            );
            with_loaded_machine();
            let (num, den) = with_machine(|machine| machine.audio.ratio()).expect("loaded");
            assert!(num > 0, "the refused rate must not have been applied");
            assert_ratio_is(44_100, num, den);
        });
    }

    /// A reset starts the audio run over.
    #[test]
    fn a_reset_starts_the_audio_run_over() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_step_frame(), GBA_OK);
            gba_reset();
            assert_eq!(gba_audio_underruns(), 0, "the counter is per-run");
            // And the frame accounting starts from the floor of the ratio.
            let (mut num, mut den) = (0u32, 0u32);
            // SAFETY: both out-pointers are live locals.
            unsafe { gba_samples_per_frame_fixed(&mut num, &mut den) };
            assert_eq!(num / den, with_machine(|m| m.audio.peek()).expect("loaded"));
        });
    }

    // ---- savestates (S2-b2) ------------------------------------------------

    // ---- the cartridge real-time clock (S2-b3) ------------------------------

    /// The clock is pinned, reported and released through the C ABI.
    ///
    /// The export is two parameters rather than one for a reason worth pinning
    /// down here: a `0` sentinel would have made the Unix epoch unpinnable, and
    /// the epoch is the one instant whose expected date can be written out by
    /// hand. So this pins the epoch and asserts the chip-level consequence
    /// through the machine the caller actually has.
    #[test]
    fn the_clock_is_pinned_reported_and_released_through_the_abi() {
        exclusively(|| {
            with_loaded_machine();

            let mut now = 0i64;
            // SAFETY: `out` is a live local.
            assert_eq!(unsafe { gba_rtc_time(&mut now) }, GBA_OK);
            assert!(now > 1_577_836_800, "an unpinned clock read {now}");

            assert_eq!(gba_rtc_set_time(0, 1), GBA_OK, "the epoch must be pinnable");
            // SAFETY: `out` is a live local.
            assert_eq!(unsafe { gba_rtc_time(&mut now) }, GBA_OK);
            assert_eq!(now, 0, "the pin did not take");

            // And the release is a separate switch, not a magic value.
            assert_eq!(gba_rtc_set_time(0, 0), GBA_OK);
            // SAFETY: `out` is a live local.
            assert_eq!(unsafe { gba_rtc_time(&mut now) }, GBA_OK);
            assert!(now > 1_577_836_800, "releasing did not restore the host clock");
        });
    }

    /// With nothing loaded, the clock surface says so rather than crashing.
    #[test]
    fn the_clock_surface_reports_no_rom() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(gba_rtc_set_time(0, 1), GBA_ERR_NO_ROM);
            let mut now = 0i64;
            // SAFETY: `out` is a live local.
            assert_eq!(unsafe { gba_rtc_time(&mut now) }, GBA_ERR_NO_ROM);
            assert_eq!(now, 0, "it wrote a time with no machine to report one");
        });
    }

    /// The error codes are the ones section 4.1 lists, in its order.
    ///
    /// Worth a test of its own because these are numbers a C++ caller
    /// compares against, and because S2-a shipped `GBA_ERR_STATE = 4` with no
    /// `GBA_ERR_UNSUPPORTED` at all — one short of the enum, so every code from
    /// there up was off by one and nothing noticed. The C++ side has no callers
    /// yet, which is the only reason that could be true and also the reason it
    /// is still cheap to fix.
    #[test]
    fn the_error_codes_are_the_ones_the_specification_lists() {
        assert_eq!(
            [
                GBA_OK,
                GBA_ERR_NO_ROM,
                GBA_ERR_BAD_ROM,
                GBA_ERR_BIOS,
                GBA_ERR_UNSUPPORTED,
                GBA_ERR_STATE,
                GBA_ERR_CAPACITY,
            ],
            [0, 1, 2, 3, 4, 5, 6]
        );
    }

    /// Save the state, run on, load it back: the machine is where it was left.
    ///
    /// The whole point of the export, and the reason the restore is four
    /// separate acts rather than one: a codec that round trips the bytes but
    /// forgets to re-arm the hooks produces a machine that decodes perfectly
    /// and then does nothing.
    #[test]
    fn a_state_round_trips_through_the_abi() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_step_frame(), GBA_OK);

            // Something worth losing, written *before* the save: a recognisable
            // value in working RAM.
            with_machine(|machine| {
                for offset in 0..32u32 {
                    machine.gba.cpu.bus.write_byte(
                        (0x0200_1000 + offset) as usize,
                        0x5A ^ offset as u8,
                    );
                }
            })
            .expect("loaded");

            let mut size = 0u32;
            // SAFETY: `out_size` is a live local.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_OK);
            assert!(size > 0, "a running machine must have a state to write");

            let mut buffer = vec![0u8; size as usize];
            let mut written = 0u32;
            // SAFETY: the buffer is `size` writable bytes and `written` is live.
            let code = unsafe { gba_savestate_save(buffer.as_mut_ptr(), size, &mut written) };
            assert_eq!(code, GBA_OK);
            assert_eq!(written, size, "wrote a different number of bytes than it measured");

            // And then run on, so the state is demonstrably not the present:
            // step a frame, and wipe the marker the state carries.
            assert_eq!(gba_step_frame(), GBA_OK);
            with_machine(|machine| {
                for offset in 0..32u32 {
                    machine
                        .gba
                        .cpu
                        .bus
                        .write_byte((0x0200_1000 + offset) as usize, 0);
                }
            })
            .expect("loaded");

            // SAFETY: `buffer` holds `written` readable bytes.
            let code = unsafe { gba_savestate_load(buffer.as_ptr(), written) };
            assert_eq!(code, GBA_OK, "a state we just wrote must load");

            for offset in 0..32u32 {
                let got = with_machine(|machine| {
                    machine.gba.cpu.bus.read_byte((0x0200_1000 + offset) as usize)
                })
                .expect("loaded");
                assert_eq!(got, 0x5A ^ offset as u8, "working RAM differs at +{offset}");
            }
        });
    }

    /// A loaded machine has its SWI hook back.
    ///
    /// `swi_hook` is a function pointer and cannot be serialized, so the
    /// restore has to put it back by hand. Miss that and the machine comes back
    /// unable to answer the first BIOS call the game makes — which is every
    /// game, immediately, and with no error anywhere.
    #[test]
    fn a_loaded_machine_has_its_hooks_back() {
        exclusively(|| {
            with_loaded_machine();

            // Park it inside an IntrWait *before* saving -- that is the state
            // worth saving, and the whole point: a machine asleep on a
            // condition, which is where a game spends nearly all of its time.
            with_machine(|machine| {
                swi::restore_intr_wait(
                    Some(IntrWaitRequest {
                        wanted: crate::gba::swi::wait::VBLANK_FLAG,
                        mode: crate::gba::swi::wait::WaitMode::AlwaysWait,
                    }),
                    &mut machine.gba.cpu,
                );
                machine.gba.cpu.halted = true;
            })
            .expect("loaded");
            assert!(
                with_machine(|m| m.gba.cpu.wake_hook.is_some()).expect("loaded"),
                "the wait was not armed"
            );

            let mut size = 0u32;
            // SAFETY: live local out-param.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_OK);
            let mut buffer = vec![0u8; size as usize];
            let mut written = 0u32;
            // SAFETY: the buffer is `size` writable bytes.
            assert_eq!(
                unsafe { gba_savestate_save(buffer.as_mut_ptr(), size, &mut written) },
                GBA_OK
            );

            // Now run on, so the live machine is demonstrably elsewhere.
            with_machine(|machine| {
                swi::restore_intr_wait(None, &mut machine.gba.cpu);
                machine.gba.cpu.halted = false;
            })
            .expect("loaded");

            // SAFETY: the buffer holds `written` readable bytes.
            assert_eq!(
                unsafe { gba_savestate_load(buffer.as_ptr(), written) },
                GBA_OK
            );

            with_machine(|machine| {
                assert!(
                    machine.gba.cpu.swi_hook.is_some(),
                    "the SWI hook did not come back"
                );
                assert!(
                    machine.gba.cpu.wake_hook.is_some(),
                    "a machine loaded from inside a wait came back as a plain Halt"
                );
                assert!(
                    machine.gba.cpu.halted,
                    "the machine was awake when the state was taken asleep"
                );
            })
            .expect("loaded");
            let pending = swi::pending_intr_wait().expect("the wait condition was dropped");
            assert_eq!(pending.wanted, crate::gba::swi::wait::VBLANK_FLAG);
        });
    }

    /// The sample clock resumes where the state left it.
    #[test]
    fn a_loaded_machine_resumes_its_sample_clock() {
        exclusively(|| {
            with_loaded_machine();

            // A residual worth keeping, set *before* the save.
            with_machine(|m| m.audio.restore_clock(123_457, 44_100)).expect("loaded");
            assert_eq!(
                with_machine(|m| m.audio.clock_state().0).expect("loaded"),
                123_457,
                "the setup did not move the clock"
            );

            let mut size = 0u32;
            // SAFETY: live local out-param.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_OK);
            let mut buffer = vec![0u8; size as usize];
            let mut written = 0u32;
            // SAFETY: the buffer is `size` writable bytes.
            assert_eq!(
                unsafe { gba_savestate_save(buffer.as_mut_ptr(), size, &mut written) },
                GBA_OK
            );

            // Run on, so the live clock is demonstrably elsewhere.
            with_machine(|m| m.audio.restore_clock(0, 44_100)).expect("loaded");

            // SAFETY: the buffer holds `written` readable bytes.
            assert_eq!(
                unsafe { gba_savestate_load(buffer.as_ptr(), written) },
                GBA_OK
            );
            assert_eq!(
                with_machine(|m| m.audio.clock_state().0).expect("loaded"),
                123_457,
                "the residual did not come back, so every frame after the load is short"
            );
        });
    }

    /// A state from another cartridge is refused, and the machine keeps going.
    ///
    /// The ROM is not in the payload, so nothing but this check stands between
    /// a state and a machine it has no business describing. The refusal has to
    /// leave the running machine alone, or "wrong game" becomes "corrupted
    /// session".
    #[test]
    fn a_state_from_another_cartridge_is_refused() {
        exclusively(|| {
            with_loaded_machine();
            let mut size = 0u32;
            // SAFETY: live local out-param.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_OK);
            let mut buffer = vec![0u8; size as usize];
            let mut written = 0u32;
            // SAFETY: the buffer is `size` writable bytes.
            assert_eq!(
                unsafe { gba_savestate_save(buffer.as_mut_ptr(), size, &mut written) },
                GBA_OK
            );

            // A different cartridge: same shape, different bytes.
            let mut other = cartridge();
            other[0x100] = 0xFF;
            assert_eq!(gba_load_rom_bytes(&other), GBA_OK);

            // SAFETY: the buffer holds `written` readable bytes.
            let code = unsafe { gba_savestate_load(buffer.as_ptr(), written) };
            assert_eq!(code, GBA_ERR_STATE, "another game's state was accepted");
            assert_eq!(
                gba_step_frame(),
                GBA_OK,
                "and the refusal left a machine that still runs"
            );
        });
    }

    /// A short buffer is reported, not written into.
    ///
    /// The caller gets the number it needs so one retry is enough; a save that
    /// silently truncated would be indistinguishable from a corrupt state.
    #[test]
    fn a_short_buffer_reports_the_size_it_needs() {
        exclusively(|| {
            with_loaded_machine();
            let mut size = 0u32;
            // SAFETY: live local out-param.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_OK);

            let mut tiny = vec![0u8; 16];
            let mut written = 0u32;
            // SAFETY: the buffer is 16 writable bytes and `written` is live.
            let code = unsafe { gba_savestate_save(tiny.as_mut_ptr(), 16, &mut written) };
            assert_eq!(code, GBA_ERR_CAPACITY);
            assert_eq!(written, size, "it did not say what would have worked");
            assert!(
                tiny.iter().all(|byte| *byte == 0),
                "it wrote into a buffer it had already refused"
            );
        });
    }

    /// A file that is not a GBA savestate, or one from a newer build, is
    /// `GBA_ERR_UNSUPPORTED` — never a decode failure the caller cannot
    /// distinguish from corruption.
    #[test]
    fn a_state_this_build_cannot_read_is_unsupported() {
        exclusively(|| {
            with_loaded_machine();
            let mut size = 0u32;
            // SAFETY: live local out-param.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_OK);
            let mut buffer = vec![0u8; size as usize];
            let mut written = 0u32;
            // SAFETY: the buffer is `size` writable bytes.
            assert_eq!(
                unsafe { gba_savestate_save(buffer.as_mut_ptr(), size, &mut written) },
                GBA_OK
            );

            // Not ours: the magic is gone.
            let mut foreign = buffer.clone();
            foreign[0] = b'X';
            // SAFETY: `foreign` holds `written` readable bytes.
            assert_eq!(
                unsafe { gba_savestate_load(foreign.as_ptr(), written) },
                GBA_ERR_UNSUPPORTED
            );

            // Ours, but from the future.
            let mut future = buffer.clone();
            let bumped = (save::SAVESTATE_VERSION + 1).to_le_bytes();
            future[4..8].copy_from_slice(&bumped);
            // SAFETY: `future` holds `written` readable bytes.
            assert_eq!(
                unsafe { gba_savestate_load(future.as_ptr(), written) },
                GBA_ERR_UNSUPPORTED
            );

            // Ours, right version, damaged payload: that *is* our corruption.
            let mut damaged = buffer.clone();
            damaged[written as usize - 1] = b'!';
            // SAFETY: `damaged` holds `written` readable bytes.
            let code = unsafe { gba_savestate_load(damaged.as_ptr(), written) };
            assert!(
                code == GBA_ERR_STATE || code == GBA_OK,
                "a damaged payload reported {code}"
            );
        });
    }

    /// With nothing loaded, the archive surface says so rather than crashing.
    #[test]
    fn the_archive_surface_reports_no_rom() {
        exclusively(|| {
            gba_unload_rom();
            let mut size = 0u32;
            // SAFETY: live local out-param.
            assert_eq!(unsafe { gba_savestate_size(&mut size) }, GBA_ERR_NO_ROM);

            let mut buffer = [0u8; 8];
            let mut written = 0u32;
            // SAFETY: the buffer is 8 writable bytes.
            assert_eq!(
                unsafe { gba_savestate_save(buffer.as_mut_ptr(), 8, &mut written) },
                GBA_ERR_NO_ROM
            );
            // SAFETY: the buffer is 8 readable bytes.
            assert_eq!(
                unsafe { gba_savestate_load(buffer.as_ptr(), 8) },
                GBA_ERR_NO_ROM
            );
        });
    }
}
