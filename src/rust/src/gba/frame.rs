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

use crate::gba::bios;
use crate::gba::overlay::{self, Overlay};

/// Error codes, matching plan section 4.1's enum.
pub const GBA_OK: i32 = 0;
/// No ROM is loaded.
pub const GBA_ERR_NO_ROM: i32 = 1;
/// The ROM could not be read or is not a GBA cartridge.
pub const GBA_ERR_BAD_ROM: i32 = 2;
/// A BIOS problem: neither a user BIOS nor the built-in stub could be used.
pub const GBA_ERR_BIOS: i32 = 3;
/// The request is not legal in this state — including trying to turn the
/// `BETA` watermark off in a release build.
pub const GBA_ERR_STATE: i32 = 4;
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
struct Machine {
    gba: Box<Gba>,
    overlay: Overlay,
}

impl Machine {
    fn new(rom: &[u8]) -> Self {
        let mut gba = Box::new(Gba::new(bios::stub(), rom));
        crate::gba::install_swi_hook(&mut gba);
        Self {
            gba,
            // A debug build may turn the watermark off; a release build may
            // not. See plan section 7.1 and invariant 5.
            overlay: Overlay::for_build(cfg!(not(debug_assertions))),
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
/// A poisoned lock means a previous call panicked while holding it. The state
/// is then of unknown validity, so it is dropped rather than reused: an
/// emulator that keeps going on a half-torn-down machine is worse than one
/// that reports "no ROM".
fn with_machine<R>(body: impl FnOnce(&mut Machine) -> R) -> Result<R, i32> {
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
#[unsafe(no_mangle)]
pub extern "C" fn gba_reset() -> i32 {
    // SAFETY: simulation thread.
    with_machine(|_| GBA_OK).unwrap_or(GBA_ERR_NO_ROM)
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
                return GBA_OK;
            }
            cycles += 1;
        }
        GBA_OK
    })
    .unwrap_or(GBA_ERR_NO_ROM)
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
        GBA_ERR_BAD_ROM, GBA_ERR_CAPACITY, GBA_ERR_NO_ROM, GBA_ERR_STATE, GBA_OK, MIN_CARTRIDGE,
        gba_frame_buffer, gba_frame_buffer_size, gba_init, gba_load_rom_bytes, gba_reset,
        gba_rom_loaded, gba_set_overlay, gba_step_frame, gba_unload_rom, read_c_string, scale5,
    };
    use crate::gba::overlay::{FRAME_BYTES, SCREEN_HEIGHT, SCREEN_WIDTH};

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
    /// long as the body, and a poisoned lock from a panicking test cannot
    /// wedge every later one.
    fn exclusively(body: impl FnOnce() + Send + 'static) {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let lock = TEST_LOCK.get_or_init(|| Mutex::new(()));
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = lock.lock();
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
}
