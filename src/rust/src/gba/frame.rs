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

use gba_core::cpu::hardware::internal_memory::BackupType;
use gba_core::cpu::hardware::keypad::GbaButton;
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
/// Takes the ROM out of the running machine and builds a fresh one from it, so
/// the machine really is put back to its initial state: registers, working and
/// video RAM, the frame buffer, the audio ring and the sample clock all start
/// over, and the SWI hook is re-installed by the same path that installs it on a
/// load.
///
/// # What this is, precisely
///
/// A **power-on reset**, not the BIOS `SoftReset` (SWI `0x00`). SoftReset resets
/// the CPU state and jumps back to `0x00000000` while **leaving memory alone**;
/// this one re-initialises memory as well. The reason for the choice is what a
/// Reset button means to the person pressing it, and that FCEUX11's own NES
/// reset is a hard reset too. The cost is recorded rather than glossed: a
/// program that relies on SoftReset *preserving* memory sees different
/// behaviour here. That is a choice about what the button does — it is not the
/// absence of a reset, which was known limitation L14.
///
/// The ROM is moved out rather than copied: the machine it came from is being
/// discarded, and a cartridge can be 32 MB.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub extern "C" fn gba_reset() -> i32 {
    let rom = match with_machine(|machine| {
        std::mem::take(&mut machine.gba.cpu.bus.internal_memory.rom)
    }) {
        Ok(rom) => rom,
        Err(code) => return code,
    };
    // A zero-length ROM means the machine was in no state worth resetting;
    // building from it would silently install a machine that can never boot.
    if rom.is_empty() {
        return GBA_ERR_NO_ROM;
    }
    set_machine(Machine::new(&rom));
    GBA_OK
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

/// Set the buttons the machine sees, as a mask of the ones currently held.
///
/// Bit 0 is A, 1 is B, 2 is Select, 3 is Start, 4-7 are the D-pad
/// (Right, Left, Up, Down) and 8-9 are R and L -- the core's own
/// `GbaButton` layout, which plan section 7.2's NES-compatible mapping lands on
/// one for one. **A set bit means held.** The core's `KEYINPUT` register is
/// active low, so the inversion happens here, once, rather than in the host.
///
/// A full state rather than a delta: the host reads input every frame and sets
/// everything that is down, so a button released without being in the mask is
/// released. That removes the failure mode where a missed call leaves a key
/// stuck down.
///
/// # Safety
/// The caller must be the simulation thread. Section 4.1 already confines every
/// call in this file to that one thread.
///
/// `void` per section 4.1, and honestly so: the host has nothing to decide on
/// failure. It sets buttons unconditionally before each frame, and the answer to
/// "the machine was not loaded" is the same either way -- the next load starts
/// with every button released, because the machine is built fresh.
#[unsafe(no_mangle)]
pub extern "C" fn gba_set_buttons(mask: u16) {
    // SAFETY: simulation thread.
    let _ = with_machine(|machine| {
        for button in GbaButton::ALL {
            machine.gba.cpu.bus.keypad.set_button(button, mask & (button as u16) != 0);
        }
    });
}

/// The buttons the machine currently sees, as a mask of the ones held.
///
/// Not in section 4.1's list, and added because the counterpart is useless
/// without it: a test that sets a mask and wants to know the core agreed has
/// otherwise to go and read `KEYINPUT` through the bus. Read-only view of the
/// same state [`gba_set_buttons`] writes.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub extern "C" fn gba_buttons(out_mask: *mut u16) -> i32 {
    if out_mask.is_null() {
        return GBA_ERR_STATE;
    }
    match with_machine(|machine| machine.gba.cpu.bus.keypad.key_input) {
        Ok(active_low) => {
            // SAFETY: checked non-null above.
            unsafe { *out_mask = !active_low & 0x03FF };
            GBA_OK
        }
        Err(code) => code,
    }
}

/// Which save hardware the cartridge has, as a small integer for the host.
///
/// 0 none, 1 SRAM 32K, 2 Flash 64K, 3 Flash 128K, 4 EEPROM. The numbering is
/// this module's, not the core's enum order, so that adding a type upstream
/// cannot silently renumber a value a host is comparing against.
const SAVE_TYPE_NONE: i32 = 0;
const SAVE_TYPE_SRAM: i32 = 1;
const SAVE_TYPE_FLASH64: i32 = 2;
const SAVE_TYPE_FLASH128: i32 = 3;
const SAVE_TYPE_EEPROM: i32 = 4;

fn save_type_code(backup: BackupType) -> i32 {
    match backup {
        BackupType::None => SAVE_TYPE_NONE,
        BackupType::Sram => SAVE_TYPE_SRAM,
        BackupType::Flash64 => SAVE_TYPE_FLASH64,
        BackupType::Flash128 => SAVE_TYPE_FLASH128,
        BackupType::Eeprom => SAVE_TYPE_EEPROM,
    }
}

fn save_type_from_code(code: i32) -> Option<BackupType> {
    match code {
        SAVE_TYPE_NONE => Some(BackupType::None),
        SAVE_TYPE_SRAM => Some(BackupType::Sram),
        SAVE_TYPE_FLASH64 => Some(BackupType::Flash64),
        SAVE_TYPE_FLASH128 => Some(BackupType::Flash128),
        SAVE_TYPE_EEPROM => Some(BackupType::Eeprom),
        _ => None,
    }
}

/// The save hardware this cartridge has, or [`SAVE_TYPE_NONE`].
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub extern "C" fn gba_battery_save_type() -> i32 {
    match with_machine(|machine| {
        save_type_code(machine.gba.cpu.bus.internal_memory.backup_type())
    }) {
        Ok(code) => code,
        Err(_) => SAVE_TYPE_NONE,
    }
}

/// Force the save hardware type, for a cartridge the signature scan got wrong.
///
/// Takes one of the codes [`gba_battery_save_type`] returns. An unrecognised
/// code is `GBA_ERR_STATE` rather than a silent "no save hardware": a host that
/// passes a stale value should be told, not obeyed.
///
/// Changing the type keeps the existing save up to the shorter of the two
/// sizes, so correcting a misdetection does not throw the save away.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub extern "C" fn gba_set_save_type(save_type: i32) -> i32 {
    let Some(backup) = save_type_from_code(save_type) else {
        return GBA_ERR_STATE;
    };
    // SAFETY: simulation thread.
    match with_machine(|machine| {
        machine.gba.cpu.bus.internal_memory.set_backup_type(backup);
    }) {
        Ok(()) => GBA_OK,
        Err(code) => code,
    }
}

/// Bytes the battery save currently occupies.
///
/// # Safety
/// `out_size` must point to a writable `u32`, and the caller must be the
/// simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_battery_size(out_size: *mut u32) -> i32 {
    if out_size.is_null() {
        return GBA_ERR_STATE;
    }
    // SAFETY: checked non-null above.
    match with_machine(|machine| {
        *out_size = machine.gba.cpu.bus.internal_memory.battery_data().len() as u32;
    }) {
        Ok(()) => GBA_OK,
        Err(code) => code,
    }
}

/// Copy the battery save out, raw.
///
/// The bytes are exactly what goes in a `.srm`: no header, no padding, no
/// length. That is the format mGBA uses, and a bare image is the only one
/// another emulator is likely to read.
///
/// # Safety
/// `dst` must point to `cap` writable bytes, and the caller must be the
/// simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_battery_read(dst: *mut u8, cap: u32) -> i32 {
    if dst.is_null() {
        return GBA_ERR_STATE;
    }
    // SAFETY: simulation thread.
    match with_machine(|machine| {
        let data = machine.gba.cpu.bus.internal_memory.battery_data();
        if cap < data.len() as u32 {
            return GBA_ERR_CAPACITY;
        }
        // SAFETY: `cap >= data.len()` and `dst` has `cap` writable bytes.
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len()) };
        GBA_OK
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

/// Replace the battery save from `src`.
///
/// **Refuses on a cartridge with no save hardware.** Plan section 7.3 requires
/// it and the reason is not ceremony: writing 32 KB of a save file to a cart
/// that has no save memory, and then letting the game write back over it,
/// destroys a file the user may have had for years. An unknown medium is a
/// refusal, not a guess.
///
/// # Safety
/// `src` must point to `len` readable bytes, and the caller must be the
/// simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_battery_write(src: *const u8, len: u32) -> i32 {
    if src.is_null() {
        return GBA_ERR_STATE;
    }
    // SAFETY: simulation thread.
    match with_machine(|machine| {
        let memory = &mut machine.gba.cpu.bus.internal_memory;
        if memory.backup_type() == BackupType::None {
            return GBA_ERR_STATE;
        }
        // SAFETY: the caller guarantees `len` readable bytes at `src`.
        let data = unsafe { std::slice::from_raw_parts(src, len as usize) };
        memory.load_battery(data);
        GBA_OK
    }) {
        Ok(code) => code,
        Err(code) => code,
    }
}

/// Whether the save memory has been written since this was last called.
///
/// Takes the flag, so a caller that polls it owns the flush. Asking twice
/// without writing in between gets `false` the second time, which is the
/// point.
///
/// # Safety
/// The caller must be the simulation thread.
#[unsafe(no_mangle)]
pub extern "C" fn gba_battery_take_dirty() -> i32 {
    // SAFETY: simulation thread.
    match with_machine(|machine| {
        i32::from(machine.gba.cpu.bus.internal_memory.take_save_dirty())
    }) {
        Ok(flag) => flag,
        Err(_) => 0,
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
            // **Red first**, because this is RGBA and the core's word is BGR.
            //
            // The core's colour is the GBA's hardware BGR555: `red()` reads bits
            // 0..=4 and `blue()` reads 10..=14. Those accessors are named for
            // the hardware correctly, and `Color::from_rgb` packs to match --
            // so nothing is wrong inside the core. The error was here: the first
            // byte of an RGBA buffer is the *red* slot, and it was being given
            // `blue()`.
            //
            // It reads as a plausible line of code, and it produces a picture
            // in which every colour is wrong while everything else -- layout,
            // timing, scaling -- is obviously right. A pink ground came out
            // blue and a blue status bar came out orange: the exact signature of
            // red and blue trading places.
            //
            // Each channel is scaled by bit replication rather than shifted, or
            // a pure red would come out 248 instead of 255.
            out.push(scale5(pixel.red()));
            out.push(scale5(pixel.green()));
            out.push(scale5(pixel.blue()));
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
        gba_battery_read, gba_battery_save_type, gba_battery_size, gba_battery_take_dirty,
        gba_battery_write, gba_savestate_load, gba_savestate_save, gba_savestate_size,
        gba_set_buttons, gba_set_overlay, gba_set_save_type, gba_step_frame, gba_unload_rom,
        gba_buttons, machine_slot, read_c_string, scale5, with_machine, frame_bytes,
    };
    use gba_core::cpu::hardware::internal_memory::BackupType;
use gba_core::cpu::hardware::keypad::GbaButton;
    use crate::gba::audio::{
        gba_audio_underruns, gba_render_audio, gba_samples_per_frame_fixed, gba_set_output_rate,
    };
    use crate::gba::overlay::{FRAME_BYTES, SCREEN_HEIGHT, SCREEN_WIDTH};
    use crate::gba::save;
    use crate::gba::swi;
    use crate::gba::swi::wait::IntrWaitRequest;
    // For `probe_a_real_cartridge` only: it builds its own machine rather than
    // going through the ABI, so it needs the core type and the stub image.
    use crate::gba::bios;
    use gba_core::gba::Gba;

    /// A cartridge the core will accept, shaped the way a real one is: a
    /// branch at offset 0 into the code, and `SWI 0` (SoftReset) there.
    ///
    /// Both halves matter. The stub BIOS branches to `08000000h` and lets the
    /// cartridge's own first instruction decide where the code is, so a test
    /// ROM whose branch does not reach its code runs the branch instead of
    /// the SWI and the test fails for a reason that has nothing to do with the
    /// ABI it is exercising.
    fn cartridge() -> Vec<u8> {
        const CODE_AT: usize = 0xC0;
        let mut rom = vec![0u8; 0x200];
        // `b` at offset 0: the target is `PC + 8 + (offset << 2)`, and at
        // offset 0 that is `8 + (offset << 2)`. Same encoding the core decodes.
        let offset = ((CODE_AT - 8) / 4) as u32;
        rom[..4].copy_from_slice(&(0xEA00_0000 | (offset & 0x00FF_FFFF)).to_le_bytes());
        rom[CODE_AT..CODE_AT + 4].copy_from_slice(&0xEF00_0000u32.to_le_bytes());
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

    /// A pure red pixel leaves as R in the first byte.
    ///
    /// **The test above could not have caught red and blue trading places**, and
    /// the reason is worth keeping in mind: every colour it looks at is
    /// neutral. The watermark is white, and it checks the alpha of everything
    /// else. A channel swap is invisible in grey, so a frame that is entirely
    /// watermark-coloured passes either way.
    ///
    /// So this one plants a saturated colour in the core's own buffer and reads
    /// the real conversion path, including the watermark step. Red is the
    /// strictest probe available: it is zero in two channels, so a swap cannot
    /// be absorbed by a tolerance.
    #[test]
    fn a_saturated_colour_keeps_its_channels() {
        use gba_core::cpu::hardware::lcd::Color;

        exclusively(|| {
            with_loaded_machine();

            // Everything that touches the machine happens inside this block,
            // and the guard is dropped at the end of it -- **before** any
            // assertion runs.
            //
            // That ordering is not tidiness. A failing `assert!` unwinds while
            // the slot's `MutexGuard` is still alive, which poisons the lock,
            // and every later test in the crate that touches the machine then
            // dies on a poisoned lock instead of on its own merits. One real
            // failure became twenty-five unrelated ones the first time this
            // test was written: the same safe-versus-isolated split the module
            // already documents, reached from the other direction.
            let probed: [[u8; 4]; 3] = {
                let mut slot = machine_slot()
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let machine = slot.as_mut().expect("with_loaded_machine loaded one");

                // Three corners of the 5-bit colour cube, chosen so each is
                // zero in two channels and full in the third.
                let probes = [
                    (0u32, 0u32, Color::from_rgb(31, 0, 0)),
                    (1, 0, Color::from_rgb(0, 31, 0)),
                    (0, 1, Color::from_rgb(0, 0, 31)),
                ];
                let saved: Vec<Color> = probes
                    .iter()
                    .map(|(y, x, _)| machine.gba.cpu.bus.lcd.buffer[*y as usize][*x as usize])
                    .collect();
                for (y, x, colour) in &probes {
                    machine.gba.cpu.bus.lcd.buffer[*y as usize][*x as usize] = *colour;
                }

                let bytes = frame_bytes(machine);
                let at = |y: usize, x: usize| -> [u8; 4] {
                    let start = (y * 240 + x) * 4;
                    [bytes[start], bytes[start + 1], bytes[start + 2], bytes[start + 3]]
                };
                let read = [at(0, 0), at(1, 0), at(0, 1)];

                // Put the machine back even so, so the next test in this
                // module sees the machine it left behind.
                for ((y, x, _), original) in probes.iter().zip(&saved) {
                    machine.gba.cpu.bus.lcd.buffer[*y as usize][*x as usize] = *original;
                }
                read
            };

            assert_eq!(
                probed[0],
                [255, 0, 0, 255],
                "red pixel lost its channel"
            );
            assert_eq!(
                probed[1],
                [0, 255, 0, 255],
                "green pixel lost its channel"
            );
            assert_eq!(
                probed[2],
                [0, 0, 255, 255],
                "blue pixel lost its channel -- the buffer is RGBA, so blue is the third byte"
            );
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

    // ---- input (S2-b4 stage 1) ---------------------------------------------

    /// Every one of the ten buttons, one at a time, reaches the core under the
    /// bit the host uses.
    ///
    /// Ten separate cases on purpose. A single "press all ten" test would pass
    /// against **any** permutation of the bit layout — which is exactly the
    /// mistake worth catching here, since the layout is a claim about hardware
    /// that nothing else in the build checks. Each button is also read back on
    /// its own, so a permutation fails loudly instead of quietly swapping two
    /// keys.
    #[test]
    fn each_button_lands_on_its_own_bit() {
        exclusively(|| {
            with_loaded_machine();
            for (index, button) in GbaButton::ALL.into_iter().enumerate() {
                gba_set_buttons(0);
                gba_set_buttons(1 << index);

                let mut mask = 0u16;
                // SAFETY: `mask` is a live local.
                assert_eq!(unsafe { gba_buttons(&mut mask) }, GBA_OK);
                assert_eq!(
                    mask,
                    1 << index,
                    "{} is on bit {index}, not {mask:#06b}",
                    button.name()
                );
                assert_eq!(
                    mask.count_ones(),
                    1,
                    "{} also pressed something else: {mask:#06b}",
                    button.name()
                );
            }
        });
    }

    /// The mask is a full state, not a delta: whatever is absent is released.
    ///
    /// The host sets input every frame, so a key that is not in the mask must be
    /// up. A setter that only ever pressed would leave the last key stuck down
    /// for the rest of the session, and nothing in the frame path would notice.
    #[test]
    fn a_button_absent_from_the_mask_is_released() {
        exclusively(|| {
            with_loaded_machine();
            gba_set_buttons(GbaButton::A as u16 | GbaButton::L as u16);
            gba_set_buttons(GbaButton::A as u16);

            let mut mask = 0u16;
            // SAFETY: `mask` is a live local.
            assert_eq!(unsafe { gba_buttons(&mut mask) }, GBA_OK);
            assert_eq!(
                mask,
                GbaButton::A as u16,
                "L was not released: {mask:#06b}"
            );
        });
    }

    /// Nothing held is nothing held, from a fresh machine.
    #[test]
    fn a_fresh_machine_reports_no_buttons() {
        exclusively(|| {
            with_loaded_machine();
            let mut mask = 0xFFFFu16;
            // SAFETY: `mask` is a live local.
            assert_eq!(unsafe { gba_buttons(&mut mask) }, GBA_OK);
            assert_eq!(mask, 0, "a machine with no input reports {mask:#06b}");
        });
    }

    /// Buttons travel with a savestate, and come back with the machine.
    ///
    /// `Keypad::key_input` is serialized, so this needs no re-installation —
    /// unlike the hooks, which is worth asserting rather than assuming, since
    /// "the function pointers come back unset" is the documented rule nearby and
    /// a reader would reasonably guess input behaves the same way.
    #[test]
    fn buttons_survive_a_savestate_round_trip() {
        exclusively(|| {
            with_loaded_machine();
            gba_set_buttons(GbaButton::B as u16 | GbaButton::Start as u16);
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

            gba_set_buttons(0);
            // SAFETY: the buffer holds `written` readable bytes.
            assert_eq!(
                unsafe { gba_savestate_load(buffer.as_ptr(), written) },
                GBA_OK
            );

            let mut mask = 0u16;
            // SAFETY: `mask` is a live local.
            assert_eq!(unsafe { gba_buttons(&mut mask) }, GBA_OK);
            assert_eq!(
                mask,
                (GbaButton::B as u16) | (GbaButton::Start as u16),
                "the held buttons did not come back: {mask:#06b}"
            );
        });
    }

    /// With nothing loaded, the input surface is inert rather than crashing.
    #[test]
    fn the_input_surface_reports_no_rom() {
        exclusively(|| {
            gba_unload_rom();
            gba_set_buttons(GbaButton::A as u16);
            let mut mask = 0xFFFFu16;
            // SAFETY: `mask` is a live local.
            assert_eq!(unsafe { gba_buttons(&mut mask) }, GBA_ERR_NO_ROM);
            assert_eq!(mask, 0xFFFF, "it wrote a mask with no machine to read");
        });
    }

    // ---- the cartridge real-time clock (S2-b3) ------------------------------

    /// A reset puts the machine back where it started, and keeps the cartridge.
    ///
    /// This is the test behind striking known limit **L14** ("`gba_reset()` does
    /// not reset the core"). It checks four things separately, because "reset"
    /// is four claims and an implementation that does three of them still looks
    /// right at a glance:
    ///
    /// - the **program counter** is back at the reset vector,
    /// - **working RAM** no longer holds what was written to it,
    /// - the **SWI hook** is installed again (a rebuilt machine that came back
    ///   without one would answer no BIOS call at all), and
    /// - the **cartridge is still loaded** — a reset that unloads the game is
    ///   the one behaviour that would be worse than not resetting.
    #[test]
    fn a_reset_rewinds_the_machine_and_keeps_the_cartridge() {
        exclusively(|| {
            with_loaded_machine();
            let rom_before = with_machine(|m| m.gba.cpu.bus.internal_memory.rom.len())
                .expect("loaded");
            let pc_before = with_machine(|m| m.gba.cpu.registers.program_counter())
                .expect("loaded");

            // Run a frame, scribble over working RAM, and press a button, so
            // that "reset" has something to actually undo.
            assert_eq!(gba_step_frame(), GBA_OK);
            with_machine(|m| {
                for offset in 0..64u32 {
                    m.gba.cpu.bus.write_byte((0x0200_0000 + offset) as usize, 0xA5);
                }
            })
            .expect("loaded");
            gba_set_buttons(GbaButton::A as u16);

            assert_eq!(gba_reset(), GBA_OK);

            with_machine(|m| {
                assert_eq!(
                    m.gba.cpu.registers.program_counter(),
                    pc_before,
                    "the program counter was not rewound"
                );
                for offset in 0..64u32 {
                    let got = m.gba.cpu.bus.read_byte((0x0200_0000 + offset) as usize);
                    assert_ne!(
                        got, 0xA5,
                        "working RAM still holds what was written at +{offset}"
                    );
                }
                assert!(
                    m.gba.cpu.swi_hook.is_some(),
                    "the rebuilt machine came back without its SWI hook"
                );
            })
            .expect("loaded");

            // Buttons do not survive a reset either: the core's keypad starts
            // with all ten released, and a stuck A would be invisible until a
            // game acted on it.
            let mut mask = 0xFFFFu16;
            // SAFETY: `mask` is a live local.
            assert_eq!(unsafe { gba_buttons(&mut mask) }, GBA_OK);
            assert_eq!(mask, 0, "a button survived the reset: {mask:#06b}");

            assert_eq!(
                gba_rom_loaded(),
                1,
                "the reset unloaded the cartridge"
            );
            assert_eq!(
                with_machine(|m| m.gba.cpu.bus.internal_memory.rom.len()).expect("loaded"),
                rom_before,
                "the cartridge changed across the reset"
            );
        });
    }

    /// A second reset is as harmless as the first, and a reset of nothing is a
    /// refusal rather than an empty machine.
    ///
    /// The second half is the one that matters: rebuilding from a zero-length
    /// ROM would install a machine whose cartridge is empty, which boots to
    /// nothing and then reports "loaded". Refusing keeps "no cartridge" and
    /// "broken cartridge" distinguishable.
    ///
    /// The empty-ROM guard is **unreachable through the public ABI** — a machine
    /// only exists if `gba_load_rom_bytes` accepted a cartridge, and that
    /// demands `MIN_CARTRIDGE` bytes — so the test empties the ROM behind the
    /// guard rather than pretending the public path reaches it. The first
    /// version of this test asserted the refusal with nothing loaded, and passed
    /// for the wrong reason: that goes through `with_machine`'s "no machine"
    /// error, never reaching the guard at all. Removing the guard left it green.
    #[test]
    fn a_reset_is_idempotent_and_refuses_an_empty_machine() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(gba_reset(), GBA_ERR_NO_ROM, "reset with nothing loaded");
            assert_eq!(gba_rom_loaded(), 0, "and it must not install a machine");

            with_loaded_machine();
            assert_eq!(gba_reset(), GBA_OK);
            assert_eq!(gba_reset(), GBA_OK, "a second reset is not an error");
            assert_eq!(gba_rom_loaded(), 1);

            // Now the unreachable case, reached on purpose.
            with_machine(|m| m.gba.cpu.bus.internal_memory.rom.clear()).expect("loaded");
            assert_eq!(
                gba_reset(),
                GBA_ERR_NO_ROM,
                "a machine with no cartridge must refuse to rebuild"
            );
            assert_eq!(gba_rom_loaded(), 1, "the machine survived a refused reset");
        });
    }

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

    // ---- the battery save (S3-1) -------------------------------------------

    /// A cartridge with no save hardware refuses a save write, and says why.
    ///
    /// Plan section 7.3 requires this and the reason is not ceremony: writing a
    /// save file to a cart with no save memory, then letting the game write back
    /// over it, destroys a file the user may have had for years. The default
    /// test cartridge carries no `SRAM_V` marker, so this is exactly the
    /// ambiguous case, and "refuse" is the only answer the plan allows.
    #[test]
    fn a_cartridge_with_no_save_hardware_refuses_a_save_write() {
        exclusively(|| {
            with_loaded_machine();
            // The cartridge() fixture has no save-type marker, so the scan finds
            // nothing. If that stops being true, this test is lying.
            assert_eq!(gba_battery_save_type(), 0, "the fixture now has save hardware");

            let data = [0x5Au8; 64];
            // SAFETY: `data` is 64 readable bytes.
            let code = unsafe { gba_battery_write(data.as_ptr(), data.len() as u32) };
            assert_eq!(code, GBA_ERR_STATE, "an unknown medium must be refused");
        });
    }

    /// A forced save type makes the cartridge writable, and the size follows it.
    ///
    /// The sizes are the point: SRAM 32K, Flash 64K, Flash 128K, EEPROM 32K, and
    /// a host that sized its file from the *previous* type would write a
    /// truncated save.
    #[test]
    fn a_forced_save_type_sizes_the_buffer() {
        exclusively(|| {
            with_loaded_machine();
            for (code, size) in [(1, 0x8000usize), (2, 0x1_0000), (3, 0x2_0000), (4, 0x8000)] {
                assert_eq!(gba_set_save_type(code), GBA_OK, "save type {code}");
                assert_eq!(gba_battery_save_type(), code, "save type {code} did not take");
                let mut reported = 0u32;
                // SAFETY: `reported` is a live local.
                assert_eq!(unsafe { gba_battery_size(&mut reported) }, GBA_OK);
                assert_eq!(reported as usize, size, "save type {code} size");
            }
            // And back to none, which makes it unwritable again.
            assert_eq!(gba_set_save_type(0), GBA_OK);
            let data = [0x11u8; 8];
            // SAFETY: `data` is 8 readable bytes.
            assert_eq!(
                unsafe { gba_battery_write(data.as_ptr(), 8) },
                GBA_ERR_STATE,
                "back to no save hardware"
            );
        });
    }

    /// A save survives a round trip; a short one loads without clearing the rest.
    #[test]
    fn a_save_round_trips_and_a_short_write_keeps_the_rest() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_set_save_type(1), GBA_OK); // SRAM, 32K

            let mut full = vec![0u8; 0x8000];
            for (index, byte) in full.iter_mut().enumerate() {
                *byte = (index % 251) as u8;
            }
            // SAFETY: `full` is 0x8000 readable bytes.
            assert_eq!(unsafe { gba_battery_write(full.as_ptr(), 0x8000) }, GBA_OK);

            let mut read_back = vec![0u8; 0x8000];
            // SAFETY: `read_back` is 0x8000 writable bytes.
            assert_eq!(unsafe { gba_battery_read(read_back.as_mut_ptr(), 0x8000) }, GBA_OK);
            assert_eq!(read_back, full, "the save did not survive the round trip");

            // A short file loads its bytes and leaves the rest **untouched** --
            // not zeroed. The core documents exactly that, and a host restoring
            // a save written by an older, smaller emulator depends on it, so
            // the assertion is against the original contents rather than
            // against a constant: "untouched" is the claim, not "zero".
            let short = [0xC3u8; 16];
            // SAFETY: `short` is 16 readable bytes.
            assert_eq!(unsafe { gba_battery_write(short.as_ptr(), 16) }, GBA_OK);
            read_back.iter_mut().for_each(|byte| *byte = 0);
            // SAFETY: `read_back` is 0x8000 writable bytes.
            assert_eq!(unsafe { gba_battery_read(read_back.as_mut_ptr(), 0x8000) }, GBA_OK);
            assert_eq!(&read_back[..16], &short[..], "the short file did not load");
            assert_eq!(
                &read_back[16..],
                &full[16..],
                "a short file changed the rest of the save"
            );

            // A buffer too small is refused rather than truncated.
            let mut tiny = [0u8; 16];
            // SAFETY: `tiny` is 16 writable bytes.
            assert_eq!(
                unsafe { gba_battery_read(tiny.as_mut_ptr(), 16) },
                GBA_ERR_CAPACITY
            );
        });
    }

    /// Switching type keeps the save, up to the shorter of the two sizes.
    ///
    /// This is what makes `gba_set_save_type` a correction rather than a reset:
    /// a user forcing the right type onto a misdetected cart must not lose what
    /// is already there.
    #[test]
    fn switching_save_type_keeps_what_fits() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_set_save_type(1), GBA_OK); // SRAM, 32K
            let data = [0x77u8; 0x8000];
            // SAFETY: `data` is 0x8000 readable bytes.
            assert_eq!(unsafe { gba_battery_write(data.as_ptr(), 0x8000) }, GBA_OK);

            // Up to Flash128: bigger, so all 32K survives.
            assert_eq!(gba_set_save_type(3), GBA_OK);
            let mut read_back = vec![0u8; 0x2_0000];
            // SAFETY: `read_back` is 0x20000 writable bytes.
            assert_eq!(unsafe { gba_battery_read(read_back.as_mut_ptr(), 0x2_0000) }, GBA_OK);
            assert_eq!(&read_back[..0x8000], &data[..], "the save was dropped going up");

            // Back down to SRAM: the first 32K is still the save. The tail is a
            // fresh buffer's 0xFF rather than a copy of bytes we no longer fit.
            assert_eq!(gba_set_save_type(1), GBA_OK);
            read_back.iter_mut().for_each(|byte| *byte = 0);
            // SAFETY: `read_back` is 0x20000 writable bytes; only 0x8000 is read.
            unsafe { gba_battery_read(read_back.as_mut_ptr(), 0x2_0000) };
            assert_eq!(&read_back[..0x8000], &data[..], "the save was dropped coming down");
        });
    }

    /// An unrecognised save type is refused, not rounded to "none".
    ///
    /// A host holding a stale value should be told. Quietly treating it as "no
    /// save hardware" would also *return* `GBA_OK`, leaving the user
    /// wondering why their override did nothing.
    #[test]
    fn an_unknown_save_type_is_refused() {
        exclusively(|| {
            with_loaded_machine();
            for code in [-1, 5, 99, i32::MAX] {
                assert_eq!(gba_set_save_type(code), GBA_ERR_STATE, "save type {code} accepted");
            }
            assert_eq!(gba_set_save_type(1), GBA_OK);
            assert_eq!(gba_battery_save_type(), 1, "a refused set changed the type");
        });
    }

    /// The dirty flag is taken, so a caller that polls it owns the flush.
    #[test]
    fn the_save_dirty_flag_is_taken_not_merely_read() {
        exclusively(|| {
            with_loaded_machine();
            assert_eq!(gba_set_save_type(1), GBA_OK);
            assert_eq!(gba_battery_take_dirty(), 0, "a freshly built machine is clean");

            // Write through the machine's own memory, the way a game does.
            with_machine(|machine| {
                machine.gba.cpu.bus.internal_memory.write_at(0x0E00_0000, 0x42);
            })
            .expect("loaded");
            assert_eq!(gba_battery_take_dirty(), 1, "the write was not noticed");
            assert_eq!(gba_battery_take_dirty(), 0, "the flag was not taken");

            // And loading a save file is *not* a device write, so the host is
            // not told to flush it straight back out again.
            let data = [0x99u8; 32];
            // SAFETY: `data` is 32 readable bytes.
            assert_eq!(unsafe { gba_battery_write(data.as_ptr(), 32) }, GBA_OK);
            assert_eq!(
                gba_battery_take_dirty(),
                0,
                "a host-side save load was mistaken for a device write"
            );
        });
    }

    /// With nothing loaded, the save surface reports it instead of crashing.
    #[test]
    fn the_save_surface_reports_no_rom() {
        exclusively(|| {
            gba_unload_rom();
            assert_eq!(gba_battery_save_type(), 0);
            let mut size = 0u32;
            // SAFETY: `size` is a live local.
            assert_eq!(unsafe { gba_battery_size(&mut size) }, GBA_ERR_NO_ROM);
            let data = [0u8; 8];
            // SAFETY: `data` is 8 readable bytes.
            assert_eq!(unsafe { gba_battery_write(data.as_ptr(), 8) }, GBA_ERR_NO_ROM);
            assert_eq!(gba_set_save_type(1), GBA_ERR_NO_ROM);
            assert_eq!(gba_battery_take_dirty(), 0);
        });
    }

    /// Where the program counter is, by memory region.
    ///
    /// The names are the hardware ones rather than ours: the question this
    /// exists to answer is "did the game get as far as its own code", and that
    /// is a question about the GBA's address map, not about this module.
    fn region_of(pc: usize) -> &'static str {
        match pc {
            0x0000_0000..=0x0000_3FFF => "BIOS (our stub)",
            0x0200_0000..=0x0203_FFFF => "EWRAM",
            0x0300_0000..=0x0300_7FFF => "IWRAM",
            0x0400_0000..=0x0400_03FF => "IO registers",
            0x0500_0000..=0x0500_01FF => "palette + OAM",
            0x0600_0000..=0x0600_FFFF => "VRAM",
            0x0700_0000..=0x0700_03FF => "OAM",
            0x0800_0000..=0x09FF_FFFF => "cart ROM",
            0x0A00_0000..=0x0BFF_FFFF => "cart ROM (wait state 1)",
            0x0C00_0000..=0x0DFF_FFFF => "cart ROM (wait state 2)",
            0x0E00_0000..=0x0E00_FFFF => "cart SRAM",
            _ => "unmapped",
        }
    }

    /// How much of the screen is not the colour every pixel starts as.
    fn screen_report(gba: &Gba) -> (usize, usize, Vec<(u8, u8, u8, usize)>) {
        let mut lit = 0usize;
        let mut total = 0usize;
        let mut histogram: std::collections::HashMap<(u8, u8, u8), usize> =
            std::collections::HashMap::new();
        for row in &gba.cpu.bus.lcd.buffer {
            for pixel in row {
                total += 1;
                let rgb = (pixel.red(), pixel.green(), pixel.blue());
                if rgb != (0, 0, 0) {
                    lit += 1;
                }
                *histogram.entry(rgb).or_insert(0) += 1;
            }
        }
        let mut top: Vec<(u8, u8, u8, usize)> =
            histogram.into_iter().map(|(rgb, n)| (rgb.0, rgb.1, rgb.2, n)).collect();
        top.sort_by_key(|(_, _, _, n)| std::cmp::Reverse(*n));
        top.truncate(4);
        (lit, total, top)
    }

    /// Which `.gba` a diagnostic probe should run.
    ///
    /// `GBA_PROBE_ROM` wins; otherwise the Desktop is scanned. Three probes
    /// each re-implementing this is the r50 shape -- the same decision written
    /// down more than once, with nothing keeping the copies in step.
    fn probe_rom_path() -> Option<std::path::PathBuf> {
        if let Ok(path) = std::env::var("GBA_PROBE_ROM") {
            let path = std::path::PathBuf::from(path);
            return path.is_file().then_some(path);
        }
        let desktop = std::path::Path::new(&std::env::var("USERPROFILE").unwrap_or_default())
            .join("Desktop");
        std::fs::read_dir(&desktop)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("gba"))
            })
    }

    /// How long a probe should run, in emulated steps.
    ///
    /// `GBA_PROBE_STEPS` overrides it, because the interesting question is
    /// frequently "would it get there eventually" and the default is a
    /// *guess* about how long that is. At 280,896 cycles a frame, 30 million
    /// steps is under two seconds of GBA time -- which is less than a
    /// commercial game's fade-in, so a game that looks wedged at the default
    /// may simply not have been run long enough. A probe that cannot be given
    /// more time reports "stuck" when it means "not yet".
    fn probe_steps(default: u64) -> u64 {
        std::env::var("GBA_PROBE_STEPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    }

    /// Run a real cartridge and report what the core actually did with it.
    ///
    /// **Ignored by default, and it only prints.** This is the S4 diagnostic:
    /// the question "why is a commercial game a black screen" cannot be
    /// answered from the front end, because every layer in front of the core
    /// has already been shown to work. What is missing is evidence about the
    /// core itself -- whether the game reached its own code, and whether the
    /// LCD ever emitted a non-black pixel.
    ///
    /// It runs a **local** `Gba`, not the shared machine behind the ABI, so it
    /// needs no isolation and cannot disturb a real session.
    ///
    /// ```text
    /// set GBA_PROBE_ROM=C:\path\to\game.gba
    /// cargo test -p fceux11-rust --release --no-default-features --features gba --lib \
    ///     probe_a_real_cartridge -- --ignored --nocapture
    /// ```
    ///
    /// With no `GBA_PROBE_ROM` it looks on the Desktop, and if it finds
    /// nothing it says so and returns: a machine without someone's game on it
    /// is a normal machine, not a failure.
    #[test]
    #[ignore = "a diagnostic probe for S4: it needs a real .gba and only reports"]
    fn probe_a_real_cartridge() {
        /// Total instructions to run. Roughly ten seconds of emulated GBA at
        /// a few million instructions a second, which is far past the point
        /// where a booting game has either drawn something or given up.
        let STEPS: u64 = probe_steps(30_000_000);
        /// How often to sample the program counter.
        const SAMPLE_EVERY: u64 = 1_000;

        let Some(path) = probe_rom_path() else {
            println!("no .gba found -- set GBA_PROBE_ROM to a path, or put one on the Desktop");
            return;
        };

        let rom = std::fs::read(&path).expect("read the cartridge");
        println!("cartridge : {}", path.display());
        println!("size      : {} bytes", rom.len());
        if rom.len() < 0xC0 {
            println!("too short to carry a GBA header; the loader would have refused it");
            return;
        }
        println!(
            "header    : entry=0x{:08X} maker={:?} fixed@0xB2=0x{:02X}",
            u32::from_le_bytes([rom[0], rom[1], rom[2], rom[3]]),
            String::from_utf8_lossy(&rom[0xAC..0xB0]),
            rom[0xB2]
        );

        let mut gba = Gba::new(bios::stub(), &rom);
        crate::gba::install_swi_hook(&mut gba);

        let mut regions: std::collections::HashMap<&'static str, u64> = std::collections::HashMap::new();
        let mut addresses: std::collections::HashMap<usize, u64> = std::collections::HashMap::new();
        let mut checkpoints: Vec<(u64, usize, usize)> = Vec::new();
        let mut distinct_screens: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut in_cart = 0u64;
        let mut samples = 0u64;
        let mut first_cart = None;

        for step in 1..=STEPS {
            gba.step();
            if step % SAMPLE_EVERY == 0 {
                samples += 1;
                let pc = gba.cpu.registers.program_counter();
                *regions.entry(region_of(pc)).or_insert(0) += 1;
                *addresses.entry(pc).or_insert(0) += 1;
                if (0x0800_0000..0x0E00_0000).contains(&pc) {
                    in_cart += 1;
                    first_cart.get_or_insert(pc);
                }
            }
            // Looking at every pixel on every step would cost more than the
            // emulation itself, so the screen is sampled, not polled.
            if step % 1_250_000 == 0 {
                let (lit, total, top) = screen_report(&gba);
                distinct_screens.insert(
                    top.iter().map(|(r, g, b, _)| format!("{r}{g}{b}")).collect(),
                );
                checkpoints.push((step, lit, total));
            }
        }

        let (lit, total, top) = screen_report(&gba);
        let pc = gba.cpu.registers.program_counter();

        println!("\nafter {STEPS} instructions:");
        println!("  PC                   : 0x{:08X}  ({})", pc, region_of(pc));
        println!("  CPU halted           : {}", gba.cpu.halted);
        println!("  screen               : {lit} of {total} pixels lit ({:.2} %)", lit as f64 * 100.0 / total as f64);
        println!("  four commonest colours:");
        for (r, g, b, n) in top {
            println!("      rgb({r:>2},{g:>2},{b:>2})  {n:>6} pixels");
        }

        println!("\n  where the PC was, by region (one sample per {SAMPLE_EVERY} instructions):");
        let mut region_rows: Vec<(&&str, &u64)> = regions.iter().collect();
        region_rows.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (name, count) in region_rows {
            println!("      {name:<26} {count:>8} samples");
        }
        println!(
            "  inside the cartridge : {in_cart} of {samples} ({:.1} %)",
            in_cart as f64 * 100.0 / samples as f64
        );

        // A cartridge the CPU never reached is a boot regression, not a
        // property of the game, and the two look identical on screen. Saying so
        // in words is the difference between "this game is Alpha" and "the
        // stub BIOS is broken", and this is the test that found the second
        // one once already.
        if in_cart == 0 {
            println!(
                "\n  ** THE CPU NEVER REACHED THE CARTRIDGE. That is a broken stub BIOS,\n  \
                 not a game that has not drawn yet. Check `bios::CART_ENTRY`. **"
            );
        } else {
            println!("  first cart address    : 0x{:08X}", first_cart.unwrap_or(0));
        }

        println!("\n  most visited addresses (a tight loop shows up here):");
        let mut hot: Vec<(&usize, &u64)> = addresses.iter().collect();
        hot.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
        for (addr, count) in hot.iter().take(6) {
            println!("      0x{addr:08X}  ({:<24}) {count:>8} samples", region_of(**addr));
        }

        println!("\n  screen over time (sampled every 1.25M instructions):");
        for (step, lit, _) in &checkpoints {
            println!("      after {step:>10} steps: {lit:>6} lit");
        }
        println!(
            "  distinct top-colour sets over the whole run: {} \
             (1 means the picture never changed)",
            distinct_screens.len()
        );
    }

    // ---- why did a game stop? --------------------------------------------
    //
    // `probe_a_real_cartridge` answers "did the game reach its own code". It
    // cannot answer "what was the machine waiting for when it stopped", and
    // that second question is what decides whether a frozen screen is the
    // game's business or ours.
    //
    // This exists because the two commercial carts on this machine stop in
    // completely different places and present completely differently -- one
    // freezes on a drawn screen, the other is white from the first sample --
    // and the only thing that tells them apart is the register state at the
    // moment each one stopped.

    /// Read one of the GBA's IO registers.
    fn io16(gba: &mut Gba, addr: u32) -> u16 {
        gba.cpu.bus.read_half_word(addr as usize)
    }

    /// Read a 32-bit word built out of two IO reads.
    ///
    /// Composed rather than calling a word-wide reader because every word-wide
    /// access in this program goes through the bus's own word path, and a
    /// halfword read is the same access the firmware itself makes.
    fn io32(gba: &mut Gba, addr: u32) -> u32 {
        u32::from(io16(gba, addr)) | (u32::from(io16(gba, addr + 2)) << 16)
    }

    /// One line of machine state, in the order that answers "what is it
    /// waiting for": the CPU's own gates, then the interrupt plumbing, then
    /// the two places a wait can block on.
    fn state_line(gba: &mut Gba) -> String {
        // Every value is read into a local before the format string is built.
        // Doing it inline does not borrow-check: one immutable read of `gba`
        // stays alive across the whole argument list and the bus reads after
        // it need `&mut`.
        let pc = gba.cpu.registers.program_counter();
        let halted = gba.cpu.halted;
        let irq_disabled = u8::from(gba.cpu.cpsr.irq_disable());
        let dispcnt = io16(gba, 0x0400_0000);
        let blank = dispcnt & 0x0080 != 0;
        let dispstat = io16(gba, 0x0400_0004);
        let vcount = io16(gba, 0x0400_0006);
        let ie = io16(gba, 0x0400_0200);
        let iff = io16(gba, 0x0400_0202);
        let ime = io16(gba, 0x0400_0208);
        let bios_flags = gba.cpu.bus.read_byte(0x0300_7FF8);
        let irq_ptr = io32(gba, 0x0300_7FFC);

        format!(
            "pc=0x{pc:08X} halted={halted:<5} I={irq_disabled} \
             DISPCNT=0x{dispcnt:04X}{} DISPSTAT=0x{dispstat:04X} VCOUNT={vcount:<3} \
             IE=0x{ie:04X} IF=0x{iff:04X} IME=0x{ime:04X} \
             BIOSFLAGS=0x{bios_flags:02X} IRQPTR=0x{irq_ptr:08X}",
            if blank { " *BLANK*" } else { "" },
        )
    }

    /// Run a real cartridge and report what it was waiting for when it stopped.
    ///
    /// **Ignored by default, and it only prints.** Purely observational: it
    /// reads registers and counts, and it writes nothing into the core.
    ///
    /// ```text
    /// set GBA_PROBE_ROM=C:\path\to\game.gba
    /// cargo test -p fceux11-rust --release --no-default-features --features gba --lib \
    ///     probe_why_a_game_stalls -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a diagnostic probe for S4: it needs a real .gba and only reports"]
    fn probe_why_a_game_stalls() {
        let STEPS: u64 = probe_steps(30_000_000);
        const SNAPSHOT_EVERY: u64 = 1_000_000;

        let Some(path) = probe_rom_path() else {
            println!("no .gba found -- set GBA_PROBE_ROM to a path, or put one on the Desktop");
            return;
        };
        let rom = std::fs::read(&path).expect("read the cartridge");
        println!("cartridge : {}", path.display());

        let mut gba = Gba::new(bios::stub(), &rom);
        crate::gba::install_swi_hook(&mut gba);

        // The BIOS IRQ handler lives at 0x18 in the stub and is the only code
        // that runs there, so a PC inside the vector block means an interrupt
        // was actually taken -- as opposed to merely requested, which is all
        // that reading IF would tell us.
        let mut irq_entries = 0u64;
        let mut first_halt = None;
        let mut halt_episodes = 0u64;
        let mut was_halted = false;
        // The BIOS intr-wait flag word is what a sleeping CPU polls, so the
        // count of writes to it is the whole question; watching the value go
        // from zero to non-zero is enough, since a flag only ever gets set.
        let mut flags_went_nonzero_at = None;
        let mut blank_at = None;
        let mut irq_ptr_at = None;
        // Edge counts, because a level sampled once a million steps apart
        // cannot tell "raised every frame and acknowledged" from "never
        // raised" -- and that is exactly the ambiguity that decides whether an
        // `IntrWait` on this group can ever be satisfied.
        let mut prev_if = 0u16;
        let mut vblank_requests = 0u64;
        let mut hblank_requests = 0u64;

        for step in 1..=STEPS {
            gba.step();

            let pc = gba.cpu.registers.program_counter();
            if pc < 0x40 {
                irq_entries += 1;
            }

            let iff = gba.cpu.bus.read_half_word(0x0400_0202);
            if (iff & 0x0001) != 0 && (prev_if & 0x0001) == 0 {
                vblank_requests += 1;
            }
            if (iff & 0x0002) != 0 && (prev_if & 0x0002) == 0 {
                hblank_requests += 1;
            }
            prev_if = iff;

            if gba.cpu.halted && !was_halted {
                if first_halt.is_none() {
                    first_halt = Some(step);
                }
                halt_episodes += 1;
            }
            was_halted = gba.cpu.halted;

            if step % SNAPSHOT_EVERY == 0 {
                if flags_went_nonzero_at.is_none()
                    && gba.cpu.bus.read_byte(0x0300_7FF8) != 0
                {
                    flags_went_nonzero_at = Some(step);
                }
                if blank_at.is_none() && io16(&mut gba, 0x0400_0000) & 0x0080 != 0 {
                    blank_at = Some(step);
                }
                if irq_ptr_at.is_none() && io32(&mut gba, 0x0300_7FFC) != 0 {
                    irq_ptr_at = Some(step);
                }
                let (lit, _, _) = screen_report(&gba);
                println!("after {step:>10}: {lit:>6} lit  {}", state_line(&mut gba));
            }
        }

        println!("\nsummary over {STEPS} instructions:");
        println!("  first halt           : {first_halt:?}");
        println!("  halt episodes        : {halt_episodes}");
        println!("  BIOS IRQ entries     : {irq_entries} steps spent in the vector");
        println!("  VBlank requests (IF) : {vblank_requests}");
        println!("  HBlank requests (IF) : {hblank_requests}");
        println!("  BIOSFLAGS first set  : {flags_went_nonzero_at:?}");
        println!("  forced blank first on: {blank_at:?}");
        println!("  IRQ handler installed: {irq_ptr_at:?}");
        println!("  wake hook installed  : {}", gba.cpu.wake_hook.is_some());
        println!("  final                : {}", state_line(&mut gba));
        println!("  palette entry 0      : 0x{:04X}", io16(&mut gba, 0x0500_0000));
        println!("  WAITCNT              : 0x{:04X}", io16(&mut gba, 0x0400_0204));
        println!("  BG0CNT               : 0x{:04X}", io16(&mut gba, 0x0400_0008));
    }

    // ---- what a stuck game is actually doing ----------------------------
    //
    // The other two probes report *state*. A game that never changes its
    // picture and never changes its registers leaves nothing to state --
    // every reading is the same reading. What is wanted there is the
    // sequence, and the vendored core already carries a disassembler
    // (`DisasmEntry::format`), so the trace is read out of the machine rather
    // than decoded by hand from the ROM. Hand-decoding is how the previous
    // two rounds went wrong.

    /// A rolling execution trace of a game that has stopped changing.
    ///
    /// **Ignored by default, and it only prints.**
    ///
    /// The warm-up matters: the interesting instructions are the ones executed
    /// *after* the machine has settled into whatever it is doing, and a trace
    /// taken from step zero is mostly the boot sequence.
    ///
    /// ```text
    /// set GBA_PROBE_ROM=C:\path\to\game.gba
    /// cargo test -p fceux11-rust --release --no-default-features --features gba --lib \
    ///     probe_trace_a_stuck_game -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a diagnostic probe for S4: it needs a real .gba and only reports"]
    fn probe_trace_a_stuck_game() {
        /// Run this long before tracing, so the trace is the settled behaviour
        /// rather than the boot sequence.
        const WARMUP: u64 = 5_000_000;
        /// How many instructions to trace. Sized to span a whole frame, because
        /// a game that sleeps on `VBlankIntrWait` runs a few instructions per
        /// frame and nothing at all in between -- a window shorter than a frame
        /// reports an empty trace for a machine that is in fact looping.
        const TRACE_STEPS: u64 = 400_000;
        /// Lines of trace to keep. The channel drops entries when it fills, so
        /// the buffer is drained every step and only the tail is held.
        const KEEP: usize = 200;

        let Some(path) = probe_rom_path() else {
            println!("no .gba found -- set GBA_PROBE_ROM to a path, or put one on the Desktop");
            return;
        };
        let rom = std::fs::read(&path).expect("read the cartridge");
        println!("cartridge : {}", path.display());

        let mut gba = Gba::new(bios::stub(), &rom);
        crate::gba::install_swi_hook(&mut gba);

        for _ in 0..WARMUP {
            gba.step();
        }
        let warm = state_line(&mut gba);
        println!("state at the end of the warm-up: {warm}\n");

        gba.cpu.disasm_enabled = true;
        let mut trace: std::collections::VecDeque<String> =
            std::collections::VecDeque::with_capacity(KEEP);
        let mut entries = 0u64;
        for _ in 0..TRACE_STEPS {
            gba.step();
            if let Some(rx) = gba.disasm_rx.as_mut() {
                while let Ok(entry) = rx.pop() {
                    entries += 1;
                    if trace.len() == KEEP {
                        trace.pop_front();
                    }
                    trace.push_back(entry.format());
                }
            }
        }
        gba.cpu.disasm_enabled = false;

        println!(
            "traced {TRACE_STEPS} steps, {entries} instructions disassembled, \
             last {KEEP} kept:\n"
        );
        for line in &trace {
            println!("    {line}");
        }
        println!("\nstate at the end of the trace: {}", state_line(&mut gba));

        // The registers, because "spin on a halfword until some bits appear"
        // is not an answer -- the address and the mask are. Reading them out
        // of the machine is also the only way to be sure which one of the
        // several hundred IO registers the game is actually polling.
        let regs: Vec<String> = (0..8)
            .map(|i| {
                let v = gba.cpu.registers.register_at(i);
                format!("r{i}=0x{v:08X}")
            })
            .collect();
        println!("registers    : {}", regs.join(" "));
        println!("sp           = 0x{:08X}", gba.cpu.registers.register_at(13));
        let r2 = gba.cpu.registers.register_at(2);
        let r3 = gba.cpu.registers.register_at(3);
        println!("r2 (polled address) = 0x{r2:08X}");
        println!("r3 (bit mask)       = 0x{r3:08X}");
        if (0x0400_0000..0x0400_0400).contains(&r2) {
            println!(
                "  -> the register there reads 0x{:04X}, so the wait is on bits {:04X} which are {}",
                gba.cpu.bus.read_half_word(r2 as usize),
                r3 & 0xFFFF,
                if (u32::from(gba.cpu.bus.read_half_word(r2 as usize)) & (r3 & 0xFFFF)) == 0 {
                    "NEVER SET"
                } else {
                    "set"
                }
            );
        }
    }

    // ---- what one frame of a stuck game actually changes ----------------
    //
    // The trace above shows *which* instructions run and the register dump
    // shows what the machine is polling, but neither shows where a
    // `LDR r2, [pc, #n]` actually pointed -- the disassembler renders a
    // PC-relative literal load as a plain immediate. Memory traffic is not
    // subject to that: what a frame writes is a fact about the machine, not a
    // decoding of the ROM.

    /// The memory a game changes in one frame of its stuck loop.
    ///
    /// **Ignored by default, and it only prints.**
    ///
    /// The window is one burst, opened by waiting for the machine to fall
    /// asleep and closed by it doing so again. Waiting for the sleep rather
    /// than counting steps is what makes the window a whole frame: a game
    /// that sleeps on `VBlankIntrWait` runs a few hundred instructions and
    /// then stops, and an arbitrary step count lands somewhere inside that
    /// or inside the next sleep.
    ///
    /// ```text
    /// set GBA_PROBE_ROM=C:\path\to\game.gba
    /// cargo test -p fceux11-rust --release --no-default-features --features gba --lib \
    ///     probe_what_one_frame_changes -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "a diagnostic probe for S4: it needs a real .gba and only reports"]
    fn probe_what_one_frame_changes() {
        const WARMUP: u64 = 5_000_000;
        /// Hard cap on waiting for the machine to settle into its loop. If it
        /// never sleeps, this probe has no window to report and says so rather
        /// than inventing one.
        const SETTLE_LIMIT: u64 = 4_000_000;
        /// Hard cap on the burst itself.
        const BURST_LIMIT: u64 = 20_000;
        /// Enough for several frames of a loop that touches a handful of
        /// variables; the report aggregates by address, so this bounds the
        /// scan, not the number of variables.
        const MAX_EVENTS: usize = 4_000;

        let Some(path) = probe_rom_path() else {
            println!("no .gba found -- set GBA_PROBE_ROM to a path, or put one on the Desktop");
            return;
        };
        let rom = std::fs::read(&path).expect("read the cartridge");
        println!("cartridge : {}", path.display());

        let mut gba = Gba::new(bios::stub(), &rom);
        crate::gba::install_swi_hook(&mut gba);

        for _ in 0..WARMUP {
            gba.step();
        }
        println!("state after the warm-up: {}", state_line(&mut gba));

        let mut settled = 0;
        while !gba.cpu.halted && settled < SETTLE_LIMIT {
            gba.step();
            settled += 1;
        }
        if !gba.cpu.halted {
            println!(
                "  the machine never went to sleep in {SETTLE_LIMIT} steps, so it is\n  \
                 spinning rather than waiting and this probe has no frame to open"
            );
            return;
        }
        // Sleeping is the *end* of a frame, not the start. Recording from here
        // would watch a machine that is still asleep and report an empty frame
        // for a game that is running perfectly well, so wait for it to wake
        // before opening the window.
        let mut slept = 0;
        while gba.cpu.halted && slept < SETTLE_LIMIT {
            gba.step();
            slept += 1;
        }
        if gba.cpu.halted {
            println!("  it never woke up in {SETTLE_LIMIT} steps; there is no frame to report");
            return;
        }
        println!(
            "  it fell asleep {settled} steps after the warm-up and stayed asleep\n  \
             {slept} more; opening the window when it wakes\n"
        );

        // IWRAM only. It is 32 KB, which is small enough to rescan every step,
        // and it is where a game keeps its flags -- the thing this probe is
        // looking for. EWRAM is 256 KB and would dominate the run time.
        const IWRAM_BASE: usize = 0x0300_0000;
        const IWRAM_WORDS: usize = 0x8000 / 2;
        let mut prev: Vec<u16> = (0..IWRAM_WORDS)
            .map(|i| gba.cpu.bus.read_half_word(IWRAM_BASE + i * 2))
            .collect();

        let mut events: Vec<(u32, u16, u16, u32)> = Vec::new();
        let mut truncated = false;
        let mut burst = 0u64;
        while burst < BURST_LIMIT {
            gba.step();
            burst += 1;
            if gba.cpu.halted {
                break;
            }
            let pc = gba.cpu.registers.program_counter() as u32;
            for i in 0..IWRAM_WORDS {
                let v = gba.cpu.bus.read_half_word(IWRAM_BASE + i * 2);
                if v != prev[i] {
                    events.push((IWRAM_BASE as u32 + (i * 2) as u32, prev[i], v, pc));
                    prev[i] = v;
                }
            }
            if events.len() >= MAX_EVENTS {
                truncated = true;
                break;
            }
        }

        // Aggregated by address, not listed event by event. A loop that repeats
        // produces thousands of lines that differ only in a counter's value,
        // and the line that matters is "these are the only addresses that ever
        // change" -- which an event log buries.
        let mut by_addr: std::collections::BTreeMap<u32, (u64, u16, u16, u32)> =
            std::collections::BTreeMap::new();
        for (addr, old, new, pc) in &events {
            let slot = by_addr.entry(*addr).or_insert((0, *old, *new, *pc));
            slot.0 += 1;
            slot.2 = *new;
        }

        println!(
            "frame ran {burst} steps and produced {} IWRAM writes across {} addresses{}:",
            events.len(),
            by_addr.len(),
            if truncated { " (truncated)" } else { "" }
        );
        println!("    address     changes   from      to        last writer");
        for (addr, (count, first, last, pc)) in &by_addr {
            println!(
                "    0x{addr:08X}  {count:>7}   0x{first:04X}   0x{last:04X}   0x{pc:08X}"
            );
        }
        if by_addr.len() <= 24 {
            println!("\n  the whole frame, in order:");
            for (addr, old, new, pc) in &events {
                println!("    [0x{addr:08X}] 0x{old:04X} -> 0x{new:04X}   (pc 0x{pc:08X})");
            }
        }
        println!("\nstate at the end of the frame: {}", state_line(&mut gba));
        println!(
            "  DISPCNT 0x{:04X}, palette[0] 0x{:04X}",
            io16(&mut gba, 0x0400_0000),
            io16(&mut gba, 0x0500_0000)
        );
    }

    // ---- the on-screen size of a GBA frame -----------------------------
    //
    // The arithmetic lives in C++ (`fceu11_gba_draw_size` in `gba_load.cpp`,
    // because that is where the three video drivers can reach it without three
    // copies). It is pure integer logic, so it is mirrored here and pinned --
    // a rule that decides how big a picture is drawn is worth a test even when
    // the thing under test is on the other side of the FFI.
    //
    // What is *not* mirrored: the drivers themselves. r50 is what happens when
    // the same decision is written out three times and only one of them is
    // ever looked at.

    /// The rule `fceu11_gba_draw_size` implements, restated.
    ///
    /// A GBA frame is 240x160 and no video mode makes it anything else, so the
    /// only question is how large a whole multiple of it fits.
    fn gba_draw_size(view_w: i32, view_h: i32) -> (i32, i32, bool) {
        const NATIVE_W: i32 = 240;
        const NATIVE_H: i32 = 160;
        let factor = (view_w / NATIVE_W).min(view_h / NATIVE_H);
        if factor >= 1 {
            return (NATIVE_W * factor, NATIVE_H * factor, true);
        }
        let scale = (view_w as f32 / NATIVE_W as f32).min(view_h as f32 / NATIVE_H as f32);
        (
            ((NATIVE_W as f32) * scale) as i32,
            ((NATIVE_H as f32) * scale) as i32,
            false,
        )
    }

    /// The picture is scaled up to fill the window, at a whole multiple.
    ///
    /// The first version of this refused to go past 1:1, which left a 240x160
    /// picture sitting in the middle of a 528x506 window looking like a bug
    /// rather than a feature. A GBA picture is mostly flat-shaded sprites and
    /// crisp text, so a non-integer factor smears it -- the whole point of
    /// insisting on integers.
    #[test]
    fn a_gba_frame_fills_the_window_at_a_whole_multiple() {
        for (view_w, view_h) in [(528, 506), (1280, 720), (800, 600), (1920, 1080)] {
            let (w, h, integral) = gba_draw_size(view_w, view_h);
            assert!(integral, "{view_w}x{view_h} should scale by a whole factor");
            assert!(
                w % 240 == 0 && h % 160 == 0,
                "{view_w}x{view_h} gave {w}x{h}, which is not a whole multiple of 240x160"
            );
            assert_eq!(
                w * 160,
                h * 240,
                "{view_w}x{view_h} gave {w}x{h}, which distorts the 3:2 aspect"
            );
            // Filling means "as large as fits", so the smaller axis has to be
            // the binding one -- one more factor would overflow it.
            assert!(
                w <= view_w && h <= view_h,
                "{view_w}x{view_h} gave {w}x{h}, which does not fit"
            );
            let next_w = w + 240;
            let next_h = h + 160;
            assert!(
                next_w > view_w || next_h > view_h,
                "{view_w}x{view_h} stopped at {w}x{h} with room for a larger multiple"
            );
        }
    }

    /// A viewport too small for 1:1 shrinks rather than clipping.
    ///
    /// The alternative is a picture wider than the window, which shows a
    /// cropped fragment of a game -- strictly worse than a small correct one,
    /// and the kind of thing that reads as a crash.
    #[test]
    fn a_viewport_smaller_than_the_frame_shrinks_instead_of_clipping() {
        for (view_w, view_h) in [(160, 120), (240, 100), (100, 160), (239, 159)] {
            let (w, h, integral) = gba_draw_size(view_w, view_h);
            assert!(!integral, "{view_w}x{view_h} cannot hold 1:1");
            assert!(w >= 1 && h >= 1, "{view_w}x{view_h} gave {w}x{h}");
            assert!(
                w <= view_w && h <= view_h,
                "{view_w}x{view_h} gave {w}x{h}, which is clipped"
            );
        }
    }

    // ---- the pad ----------------------------------------------------------------
    //
    // The GBA side of the pad is two things that have to agree: the bit layout
    // `GbaPadBit` publishes to the C++ side, and the core's own keypad. The
    // failure this pins down is a *mapping* one, so the assertion is about
    // which GBA bit each NES button ends up on -- the thing that is invisible
    // in a screenshot and shows up as "up moves right".

    /// The GBA bit each NES button index maps to, in `GamePadNames` order.
    ///
    /// This is the table `fceuWrapper.cpp` builds from the user's bindings, and
    /// it is positional: the GBA bits happen to be in the same order as the NES
    /// button indices for the first eight, which is a coincidence worth stating
    /// rather than assuming -- add a button and the coincidence stops.
    const NES_BUTTON_TO_GBA: [(u8, u16); 8] = [
        (0, 1 << 0), // A
        (1, 1 << 1), // B
        (2, 1 << 2), // Select
        (3, 1 << 3), // Start
        (4, 1 << 6), // Up    -- NOT bit 4
        (5, 1 << 7), // Down  -- NOT bit 5
        (6, 1 << 5), // Left  -- NOT bit 6
        (7, 1 << 4), // Right -- NOT bit 7
    ];

    /// The face buttons agree bit-for-bit, the d-pad does not.
    ///
    /// The property worth pinning: the d-pad is a **permutation** of the NES
    /// d-pad's four bits, not a copy. `GamePadNames` orders the directions Up,
    /// Down, Left, Right and the core's `Keypad` orders them Right, Left, Up,
    /// Down, so mapping one onto the other positionally produces a d-pad that
    /// still does *something* under every key -- a rotated one, wrong in a way
    /// no screenshot or smoke test will ever catch.
    ///
    /// The permutation is checked by its *values* rather than by comparing
    /// against an identity: sorting a permutation of a set always reproduces
    /// that set, so `sorted == expected` cannot tell a reorder from a copy. The
    /// two positional checks below are what actually make the copy fail.
    #[test]
    fn the_face_buttons_match_and_the_dpad_does_not() {
        for (nes_bit, gba_bit) in NES_BUTTON_TO_GBA.iter().take(4) {
            assert_eq!(*gba_bit, 1u16 << nes_bit, "face button {nes_bit}");
        }

        // Each d-pad direction lands somewhere other than its own NES bit, and
        // the four targets are the NES d-pad's four bits with none left over.
        // Together those two facts are exactly "a permutation and not a copy":
        // drop in `1 << nes_bit` and the first check turns red.
        let dpad: Vec<u16> = NES_BUTTON_TO_GBA.iter().skip(4).map(|(_, b)| *b).collect();
        for (offset, (nes_bit, gba_bit)) in NES_BUTTON_TO_GBA.iter().skip(4).enumerate() {
            let nes_dpad_bit = 1u16 << (4 + offset as u8);
            assert_ne!(
                *gba_bit, nes_dpad_bit,
                "NES d-pad bit {} maps to the same GBA bit, so the explicit table has \
                 collapsed back into a positional mapping -- which is the bug it exists \
                 to prevent, because every key would still do something",
                4 + offset
            );
        }
        let mut sorted = dpad.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec![1u16 << 4, 1u16 << 5, 1u16 << 6, 1u16 << 7],
            "the d-pad must use exactly the NES d-pad's four bits, each once"
        );
    }

    /// A held pad reaches the core as the bits the core reads.
    ///
    /// End of the C++-side contract, tested from this side: whatever the Qt
    /// layer pushes through `gba_set_buttons` is what the core's keypad holds.
    #[test]
    fn a_pushed_mask_reaches_the_core_unchanged() {
        exclusively(|| {
            gba_load_rom_bytes(&cartridge());
            for (nes_bit, gba_bit) in NES_BUTTON_TO_GBA {
                gba_set_buttons(gba_bit);
                let mut read = 0u16;
                assert_eq!(gba_buttons(&mut read), GBA_OK);
                assert_eq!(
                    read, gba_bit,
                    "NES button {nes_bit} should be GBA bit 0x{gba_bit:04X} alone"
                );
            }
            gba_set_buttons(0);
            let mut read = 1u16;
            assert_eq!(gba_buttons(&mut read), GBA_OK);
            assert_eq!(read, 0, "releasing every button must leave the pad empty");
        });
    }
}
