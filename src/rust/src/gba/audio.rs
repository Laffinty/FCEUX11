//! GBAEUX11 audio (v2.0 S2-b1).
//!
//! # What the core already does, and what it does not
//!
//! The vendored core resamples to the host rate itself: `Sound::step`
//! accumulates `cycles * output_rate` and emits a sample each time the
//! accumulator passes `CPU_CLOCK` (`sound.rs`). That accumulator is
//! cycle-accurate and carries its own residual, so **draining the ring is
//! already drift-free**. There is no need to implement a sample-rate converter
//! here, and plan section 4.2 says so explicitly.
//!
//! So why a second accumulator? Because it answers a question the core's cannot.
//! The core knows how many samples it *produced*; it has no notion of how many
//! the frame *should have* carried, and when the ring is full it drops samples
//! silently (`if out.slots() >= 2` -- no counter, no log, nothing). A consumer
//! that drains the ring cannot tell a healthy frame from one that silently lost
//! a third of its audio.
//!
//! [`SampleClock`] supplies the missing half: the exact per-frame count for the
//! configured rate, with the residual carried forward. Comparing the two turns
//! an invisible loss into a number, which is what plan section 8's performance
//! criterion ("audio underrun count is 0") is measured against.
//!
//! # The rate, and why it is not 738
//!
//! A GBA frame is 228 scanlines of 1232 cycles, so 280,896 cycles. The host
//! consumes `rate` samples per second, so one frame carries
//!
//! ```text
//! samples_per_frame = rate * 280896 / 16777216 = rate * 4389 / 262144
//! ```
//!
//! which is 738.3534 at 44.1 kHz -- not an integer, and the fraction is the
//! whole point. Rounding it to 738 loses 0.35 samples per frame, about 21
//! samples per second, which is a pitch-and-length drift the host cannot
//! correct. Hardcoding the integer part is exactly the mistake R12 describes.
//!
//! There is a trap in this repository that makes the mistake look reasonable:
//! `sdl-sound.cpp:263` computes
//!
//! ```c
//! samplesPerFrame = (int)((double)s_SampleRate / getBaseFrameRate());
//! ```
//!
//! which is the same quantity, truncated. It is *not* the per-frame sample
//! count -- it is a buffer-size hint, used only to raise `spec.samples` to
//! 1024 when the value is large. The authoritative count on the NES side comes
//! from `FlushEmulateSound`'s `end`, which carries four fractional bits and
//! hands the remainder to the next frame through `soundtsoffs`. Copying the
//! truncated line instead of the mechanism is the bug.
//!
//! # Why this is provably drift-free
//!
//! The accumulator adds `rate * 4389` per frame and emits `phase >> 18`,
//! keeping the remainder. Over `2^18` frames the phase has grown by
//! `2^18 * rate * 4389`, so the total emitted is
//! `(2^18 * rate * 4389) >> 18` = `rate * 4389` -- exactly, with no remainder
//! left over. And `2^18` frames is `262144 * 280896 / 16777216` = 4389 seconds.
//! So over any span of 4389 seconds the output is `rate` samples per second.
//! There is no accumulated error because the denominator is a power of two and
//! the identity closes exactly. `no_drift_over_the_exact_cycle_is_the_point`
//! checks this for every rate the SDL device offers.
//!
//! # Shortfalls are padded, not dropped
//!
//! If the ring held fewer samples than the clock asked for, the frame is
//! padded with silence and the shortfall is added to the underrun count. The
//! frame still carries its full length. Dropping the missing samples instead
//! would be *quieter* and would reintroduce exactly the drift this module
//! exists to prevent: one short frame in a thousand is inaudible, a
//! systematically short frame is a slow tape.
//!
//! # The first frame is short, and that is not drift
//!
//! Emulation starts at the top of a frame, but the core reports VBlank at
//! scanline 160 of 228 -- so the first drain happens after 197,120 cycles
//! rather than a full 280,896, and the clock asks for 738 samples where only
//! 518 exist. That is a 220-sample shortfall at 44.1 kHz: about five
//! milliseconds, once per ROM load, and it is *constant*, not a rate error.
//!
//! Measured over 100 frames at 44.1 kHz, `underruns` reads 220 at frame 1, 221
//! by frame 20, and does not move again; the ring settles at two slots. A
//! drift of the kind R12 describes would grow in proportion to the frame count
//! instead, and would empty the ring within minutes. This one does not.
//!
//! # Threading
//!
//! Simulation thread only, like the rest of the FFI surface (plan section 4.1).
//! The ring is drained inside `gba_step_frame`; `gba_render_audio` only copies
//! out of a buffer that is already finished. Nothing here is safe to call from
//! an SDL audio callback, and nothing tries to be.

use std::sync::atomic::{AtomicU32, Ordering};

use crate::gba::frame::{GBA_ERR_CAPACITY, GBA_ERR_NO_ROM, GBA_ERR_STATE, GBA_OK, with_machine};

/// CPU cycles in one GBA video frame: 228 scanlines of 1232 cycles each.
///
/// Written as the product so the arithmetic is checkable rather than asserted.
/// The core states the same figure at `lcd.rs:27` ("228 lines x 308 pixels =
/// 280,896 cycles/frame"), and `frame.rs` uses it for its step bound.
const FRAME_CYCLES: u64 = 228 * 1232;

/// The core's CPU clock, as `sound.rs` defines it.
const CPU_CLOCK: u64 = 16_777_216;

/// `samples_per_frame = rate * FRAME_CYCLES / CPU_CLOCK`.
///
/// Both sides are divided by 64 so the denominator stays a power of two, which
/// is what lets [`SampleClock::advance`] use a shift instead of a division.
/// `FRAME_CYCLES / 64` is 4389 and `CPU_CLOCK / 64` is 262144 = 2^18.
const FRAME_NUM: u64 = FRAME_CYCLES / 64;
const FRAME_DEN_SHIFT: u32 = 18;

// The reduction above is only exact if 64 really divides both sides. These are
// checked by the compiler rather than by a test, because a violation would make
// every sample count wrong in a way no runtime assertion would catch in time.
const _: () = assert!(FRAME_CYCLES % 64 == 0, "FRAME_NUM is a truncated division");
const _: () = assert!(CPU_CLOCK % 64 == 0, "FRAME_DEN_SHIFT assumes CPU_CLOCK / 64");
const _: () = assert!((1u64 << FRAME_DEN_SHIFT) * 64 == CPU_CLOCK);

/// `FSettings.SoundVolume` is a 0-150 scale -- `fceu.cpp:612` sets it to 150
/// with the comment "0-150 scale" -- so 150 is nominal full scale.
const VOLUME_NOMINAL: f32 = 150.0;

/// The volume a fresh machine starts at: full scale.
const VOLUME_DEFAULT: u32 = 150;

/// The host rate a fresh machine starts at.
///
/// Provisional. The SDL device rate is user-selectable across
/// {11025, 22050, 44100, 48000, 96000} (`sdl-sound.cpp:60`) and 44100 is what
/// that code falls back to when the configured value is unsupported
/// (`sdl-sound.cpp:245`), so it is the right guess until the C++ side pushes
/// the real one down in S2-b3.
const RATE_DEFAULT: u32 = 44_100;

/// The largest rate the SDL device offers, used only to size the ring.
const RATE_MAX: u32 = 96_000;

/// Stereo frames one video frame carries at [`RATE_MAX`].
const MAX_FRAMES_PER_VIDEO_FRAME: usize =
    (RATE_MAX as u64 * FRAME_NUM >> FRAME_DEN_SHIFT) as usize;

/// Ring capacity, in `f32` slots -- two per stereo frame.
pub(crate) const RING_SLOTS: usize = 16_384;

// Five video frames of headroom at the highest rate the device offers, so a
// caller that skips a few `gba_step_frame` calls still finds samples waiting.
const _: () = assert!(RING_SLOTS >= 2 * 5 * MAX_FRAMES_PER_VIDEO_FRAME);

/// Full scale for the core's samples, which arrive in [-1, 1].
const SAMPLE_SCALE: f32 = 32_768.0;

/// The saturation ceiling, and the reason it is not 32768.
///
/// `WriteSound`'s contract is +/-32768 (plan section 4.2), but the SDL callback
/// narrows to `AUDIO_S16SYS`. 32768 does not fit in an `i16` and wraps to
/// -32768, flipping the polarity of a full-scale sample -- a click once per
/// frame. The floor stays -32768 because that one *is* representable.
const SAMPLE_CEILING: i32 = 32_767;
const SAMPLE_FLOOR: i32 = -32_768;

/// The host rate and volume to use for a machine that does not exist yet.
///
/// `gba_set_output_rate` and `gba_set_volume` may be called before a ROM is
/// loaded, and the setting has to survive until there is a machine to apply it
/// to. Relaxed ordering is enough: these are configuration, not
/// synchronisation, and the plan's threading rule already forbids calling them
/// from two threads at once.
static PENDING_RATE: AtomicU32 = AtomicU32::new(RATE_DEFAULT);
static PENDING_VOLUME: AtomicU32 = AtomicU32::new(VOLUME_DEFAULT);

/// The rate a new machine should start at.
pub(crate) fn configured_rate() -> u32 {
    PENDING_RATE.load(Ordering::Relaxed)
}

/// The volume a new machine should start at.
pub(crate) fn configured_volume() -> u32 {
    PENDING_VOLUME.load(Ordering::Relaxed)
}

/// Per-frame sample accounting with the residual carried forward.
///
/// See the module docs for why a second accumulator exists when the core has a
/// cycle-accurate one of its own.
#[derive(Debug, Clone)]
pub struct SampleClock {
    /// Remainder, always below `1 << FRAME_DEN_SHIFT`.
    phase: u64,
    rate: u32,
}

impl SampleClock {
    /// A clock running at `rate` Hz with no accumulated residual.
    #[must_use]
    pub const fn new(rate: u32) -> Self {
        Self { phase: 0, rate }
    }

    /// The rate this clock counts for.
    #[must_use]
    pub const fn rate(&self) -> u32 {
        self.rate
    }

    /// Samples this frame must carry. The remainder stays in `phase`.
    pub fn advance(&mut self) -> u32 {
        self.phase += u64::from(self.rate) * FRAME_NUM;
        let whole = (self.phase >> FRAME_DEN_SHIFT) as u32;
        self.phase -= u64::from(whole) << FRAME_DEN_SHIFT;
        whole
    }

    /// The exact per-frame rate as a reduced fraction.
    ///
    /// Published by `gba_samples_per_frame_fixed` so a caller can size its own
    /// buffer without assuming an integer frame length. The denominator is a
    /// power of two, so the common factor is too and the reduction is exact.
    #[must_use]
    pub fn ratio(&self) -> (u32, u32) {
        let mut num = u64::from(self.rate) * FRAME_NUM;
        let mut den = 1u64 << FRAME_DEN_SHIFT;
        let shift = num.trailing_zeros().min(FRAME_DEN_SHIFT);
        num >>= shift;
        den >>= shift;
        (num as u32, den as u32)
    }

    /// Drop the residual, so the next frame starts from zero again.
    pub const fn reset(&mut self) {
        self.phase = 0;
    }
}

/// Downmix, apply volume, and saturate into the int32 range `WriteSound` wants.
///
/// The downmix happens first, before any gain (plan section 4.3): a same-phase
/// stereo pair sums to twice the channel level, so amplifying first would clip
/// every time both sides are hot.
///
/// Truncation rather than rounding, deliberately. It biases toward zero by at
/// most one LSB out of 32768 -- about -96 dB, far below the noise floor -- and
/// it keeps the range argument trivial: `f32 as i32` saturates, so nothing
/// out there can wrap.
fn to_int32(left: f32, right: f32, volume: u32) -> i32 {
    let mono = (left + right) * 0.5;
    let scaled = mono * SAMPLE_SCALE * (volume as f32 / VOLUME_NOMINAL);
    // `f32::clamp` propagates NaN, and a NaN cast to i32 is 0, so the ceiling
    // and floor are compared explicitly rather than handed to clamp. The
    // fall-through cast is reached only for values known to be in range, plus
    // NaN, which becomes silence.
    if scaled >= SAMPLE_CEILING as f32 {
        SAMPLE_CEILING
    } else if scaled <= SAMPLE_FLOOR as f32 {
        SAMPLE_FLOOR
    } else {
        scaled as i32
    }
}

/// Everything on this side of the audio boundary.
///
/// Owned by the `Machine` in `frame.rs`, so it is created and destroyed with the
/// machine and never crosses `extern "C"` -- the same arrangement `frame.rs`
/// uses for the frame buffer, and for the same reason (the 82 KB state must
/// not be materialised on the caller's stack).
pub struct AudioOut {
    /// The consumer end of the ring. `None` until a rate is installed.
    rx: Option<rtrb::Consumer<f32>>,
    clock: SampleClock,
    /// One video frame of mono int32, finished and ready to hand out.
    frame: Vec<i32>,
    volume: u32,
    underruns: u64,
}

impl AudioOut {
    /// A silent sink at `rate`, with no ring attached yet.
    #[must_use]
    pub fn new(rate: u32) -> Self {
        Self {
            rx: None,
            clock: SampleClock::new(rate),
            frame: Vec::new(),
            volume: VOLUME_DEFAULT,
            underruns: 0,
        }
    }

    /// Take the consumer end of a ring the core has just started filling.
    ///
    /// Installing a ring restarts the clock: `phase` is counted in units of
    /// `rate * 4389`, so a residual left over from a different rate means
    /// nothing.
    pub fn attach(&mut self, rx: rtrb::Consumer<f32>, rate: u32) {
        self.rx = Some(rx);
        self.clock = SampleClock::new(rate);
        self.frame.clear();
    }

    /// Move to a new rate. The caller re-creates the ring and calls
    /// [`AudioOut::attach`] afterwards.
    pub fn set_rate(&mut self, rate: u32) {
        self.clock = SampleClock::new(rate);
    }

    /// Set the output volume on the 0-150 scale.
    pub fn set_volume(&mut self, volume: u32) {
        self.volume = volume;
    }

    /// The current volume.
    #[must_use]
    pub const fn volume(&self) -> u32 {
        self.volume
    }

    /// Samples the next completed frame will carry.
    #[must_use]
    pub fn peek(&self) -> u32 {
        let rate = self.clock.rate();
        (u64::from(rate) * FRAME_NUM >> FRAME_DEN_SHIFT) as u32
    }

    /// The per-frame rate as a reduced fraction.
    #[must_use]
    pub fn ratio(&self) -> (u32, u32) {
        self.clock.ratio()
    }

    /// Samples the ring has produced but nothing has consumed.
    #[must_use]
    pub fn pending_slots(&self) -> usize {
        self.rx.as_ref().map_or(0, rtrb::Consumer::slots)
    }

    /// Cumulative samples the core had to drop, or that were not yet ready.
    ///
    /// Plan section 8's performance criterion is this number staying at zero.
    #[must_use]
    pub const fn underruns(&self) -> u64 {
        self.underruns
    }

    /// Zero the counter, for a caller that wants a per-run figure.
    pub const fn clear_underruns(&mut self) {
        self.underruns = 0;
    }

    /// Restart the frame accounting, keeping the ring.
    pub fn reset(&mut self) {
        self.clock.reset();
        self.frame.clear();
        self.underruns = 0;
    }

    /// Convert one completed video frame of core output into [`Self::frame`].
    ///
    /// `want` samples are always produced. A shortfall is padded with silence
    /// and counted, never dropped -- see the module docs.
    pub fn advance_frame(&mut self) {
        let want = self.clock.advance() as usize;
        self.frame.clear();
        self.frame.resize(want, 0);

        let Some(rx) = self.rx.as_mut() else {
            self.underruns += want as u64;
            return;
        };

        // Whole stereo pairs only. The core pushes left and right together or
        // not at all (`sound.rs`), so the ring always holds an even number of
        // slots at the head; reading an odd count would leave the halves
        // offset and swap the channels for the rest of playback.
        let take = (rx.slots() / 2).min(want);
        if take == 0 {
            self.underruns += want as u64;
            return;
        }

        // `ReadChunk::into_iter` yields in ring order -- the head, then the
        // wrapped remainder -- and releases the slots as it goes. Iterating it
        // is therefore correct even when the read spans the wrap point, which
        // `as_slices` is not: that pair of slices is cut at the wrap boundary,
        // not in half, so indexing it as (lefts, rights) would swap the
        // channels on exactly those frames.
        let Ok(chunk) = rx.read_chunk(take * 2) else {
            self.underruns += want as u64;
            return;
        };

        let mut iter = chunk.into_iter();
        let mut produced = 0usize;
        while let (Some(l), Some(r)) = (iter.next(), iter.next()) {
            self.frame[produced] = to_int32(l, r, self.volume);
            produced += 1;
        }
        self.underruns += (want - produced) as u64;
    }

    /// Copy the finished frame out to the caller.
    ///
    /// # Safety
    /// `dst` must be valid for `cap` writes, and `written` must be valid for
    /// one write or null.
    pub unsafe fn render(&self, dst: *mut i32, cap: u32, written: *mut u32) -> i32 {
        let need = self.frame.len() as u32;
        if dst.is_null() {
            return GBA_ERR_STATE;
        }
        // The required size is reported even on failure, so a caller can size
        // its buffer from one call rather than by trial and error.
        if !written.is_null() {
            unsafe { *written = need };
        }
        if cap < need {
            return GBA_ERR_CAPACITY;
        }
        // SAFETY: checked non-null and `cap >= need`, which is the length of
        // the copy below.
        unsafe { std::ptr::copy_nonoverlapping(self.frame.as_ptr(), dst, self.frame.len()) };
        GBA_OK
    }
}

/// Set the host sample rate, in Hz.
///
/// Rebuilds the ring, so any audio buffered under the old rate is discarded.
/// That is the right trade: changing the output rate means the caller has
/// changed output device, and a stream straddling two rates is not a stream.
///
/// Called before a ROM is loaded, the setting is remembered and applied when a
/// machine is created. A rate of zero is refused -- it would make the core's
/// own resampler (`if self.output_rate == 0 { return; }`) produce nothing at
/// all, and would divide the per-frame ratio to nothing.
#[unsafe(no_mangle)]
pub extern "C" fn gba_set_output_rate(rate: u32) -> i32 {
    if rate == 0 {
        return GBA_ERR_STATE;
    }
    PENDING_RATE.store(rate, Ordering::Relaxed);
    // SAFETY: simulation thread.
    match with_machine(|machine| {
        machine.audio.set_rate(rate);
        let rx = machine.gba.init_audio(rate, RING_SLOTS);
        machine.audio.attach(rx, rate);
        GBA_OK
    }) {
        Ok(status) => status,
        // No machine: the setting is recorded above and applied on load.
        Err(GBA_ERR_NO_ROM) => GBA_OK,
        Err(other) => other,
    }
}

/// Set the output volume, on the same 0-150 scale as `FSettings.SoundVolume`.
///
/// `WriteSound` applies no volume of its own (`sdl-sound.cpp:425` copies the
/// sample straight into the ring), so the gain has to be applied here, before
/// the samples reach the driver. A value above 150 is treated as a boost and
/// left to the saturating conversion.
#[unsafe(no_mangle)]
pub extern "C" fn gba_set_volume(volume: u32) -> i32 {
    PENDING_VOLUME.store(volume, Ordering::Relaxed);
    // SAFETY: simulation thread.
    match with_machine(|machine| {
        machine.audio.set_volume(volume);
        GBA_OK
    }) {
        Ok(status) => status,
        Err(GBA_ERR_NO_ROM) => GBA_OK,
        Err(other) => other,
    }
}

/// Copy the finished video frame's audio out as mono int32.
///
/// # Safety
/// `dst` must be valid for `cap` writes, `written` must be valid for one write
/// or null, and the caller must be the simulation thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_render_audio(
    dst: *mut i32,
    cap: u32,
    written: *mut u32,
) -> i32 {
    // SAFETY: simulation thread; `dst`/`written` checked inside `render`.
    unsafe { with_machine(|machine| machine.audio.render(dst, cap, written)) }
        .unwrap_or(GBA_ERR_NO_ROM)
}

/// The per-frame sample rate as a reduced fraction `num / den`.
///
/// Exists so the C++ side can size a buffer without assuming an integer frame
/// length, which is the assumption that costs drift.
///
/// # Safety
/// Both out-pointers must be valid for one write, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn gba_samples_per_frame_fixed(
    num: *mut u32,
    den: *mut u32,
) -> i32 {
    let (n, d) = SampleClock::new(configured_rate()).ratio();
    if !num.is_null() {
        // SAFETY: checked non-null.
        unsafe { *num = n };
    }
    if !den.is_null() {
        // SAFETY: checked non-null.
        unsafe { *den = d };
    }
    GBA_OK
}

/// Samples the core dropped, or that were not ready, since the last reset.
#[unsafe(no_mangle)]
pub extern "C" fn gba_audio_underruns() -> u32 {
    // SAFETY: simulation thread.
    match with_machine(|machine| machine.audio.underruns() as u32) {
        Ok(count) => count,
        Err(_) => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioOut, CPU_CLOCK, FRAME_CYCLES, FRAME_DEN_SHIFT, MAX_FRAMES_PER_VIDEO_FRAME, RATE_DEFAULT,
        RING_SLOTS, SAMPLE_CEILING, SAMPLE_FLOOR, SAMPLE_SCALE, VOLUME_NOMINAL, SampleClock,
        to_int32,
    };

    /// Every rate the SDL device offers (`sdl-sound.cpp:60`). Hardcoding 44100
    /// would be the bug this module exists to prevent, so the proof runs on all
    /// of them.
    const DEVICE_RATES: [u32; 5] = [11_025, 22_050, 44_100, 48_000, 96_000];

    /// Build a sink over a ring of `slots`, prefilled with `frames` stereo
    /// frames of `pattern`.
    ///
    /// The pattern alternates by frame -- (1, 0) then (0, 1) -- so a correct
    /// pairing always downmixes to the same value, while a pairing that is off
    /// by one slot gives full scale instead. That difference is the only thing
    /// that lets a test tell the two apart.
    fn sink(
        slots: usize,
        frames: usize,
        pattern: impl Fn(usize) -> (f32, f32),
    ) -> (rtrb::Producer<f32>, AudioOut) {
        let (mut tx, rx) = rtrb::RingBuffer::new(slots);
        for i in 0..frames {
            let (l, r) = pattern(i);
            assert!(tx.push(l).is_ok() && tx.push(r).is_ok(), "ring filled up at {i}");
        }
        let mut audio = AudioOut::new(RATE_DEFAULT);
        audio.attach(rx, RATE_DEFAULT);
        (tx, audio)
    }

    /// The alternating pattern described on [`sink`].
    fn alternate(i: usize) -> (f32, f32) {
        if i % 2 == 0 { (1.0, 0.0) } else { (0.0, 1.0) }
    }

    /// `rate * FRAME_CYCLES`, the numerator of the *unreduced* per-frame ratio,
    /// kept unreduced so a test using it is an independent check on the
    /// reduction to `FRAME_NUM / 2^18`.
    fn exact_numerator(rate: u32) -> u128 {
        u128::from(rate) * FRAME_CYCLES as u128
    }

    // ---- the no-drift proof ------------------------------------------------

    /// The accumulator is exactly drift-free over its own cycle.
    ///
    /// `2^18` frames is 4389 seconds, and over that span the output must be
    /// `rate` samples per second with nothing left over. The expected total is
    /// computed from `FRAME_CYCLES / CPU_CLOCK`, so this also proves the
    /// reduction to `FRAME_NUM / 2^18` lost nothing.
    ///
    /// This is the test R12 is about: replacing the shift with a hardcoded
    /// integer frame length -- what `sdl-sound.cpp:263` does -- turns it red.
    #[test]
    fn no_drift_over_the_exact_cycle_is_the_point() {
        for rate in DEVICE_RATES {
            let mut clock = SampleClock::new(rate);
            let mut total: u64 = 0;
            for _ in 0..(1u32 << FRAME_DEN_SHIFT) {
                total += u64::from(clock.advance());
            }
            let expected =
                (u128::from(1u128 << FRAME_DEN_SHIFT) * exact_numerator(rate)) / u128::from(CPU_CLOCK);
            assert_eq!(
                u128::from(total),
                expected,
                "rate {rate}: {total} samples over 2^18 frames, expected {expected}"
            );
            assert_eq!(total % u64::from(rate), 0, "rate {rate}: the residual did not close");
        }
    }

    /// Every frame carries the floor or the ceiling of the exact ratio, and
    /// consecutive frames never differ by more than one.
    ///
    /// A stalled or bursting clock is drift wearing a different hat: the total
    /// can come out right over a long run while every individual frame is the
    /// wrong length, which is audible as a stutter.
    #[test]
    fn each_frame_is_floor_or_ceil_and_never_jumps_by_more_than_one() {
        for rate in DEVICE_RATES {
            let mut clock = SampleClock::new(rate);
            let lo = (exact_numerator(rate) / u128::from(CPU_CLOCK)) as u32;
            let hi = lo + 1;
            let mut previous = clock.advance();
            assert!((lo..=hi).contains(&previous), "rate {rate}: first frame {previous}");

            for frame in 1..1000u32 {
                let n = clock.advance();
                assert!((lo..=hi).contains(&n), "rate {rate}: frame {frame} gave {n}");
                let jump = n.abs_diff(previous);
                assert!(jump <= 1, "rate {rate}: frame {frame} jumped by {jump}");
                previous = n;
            }
        }
    }

    /// The published fraction is reduced, and it is the same number.
    ///
    /// Cross-multiplied against the unreduced ratio, so a `ratio()` that
    /// disagreed with the accumulator by one part in `2^18` would show up here.
    #[test]
    fn the_reduced_fraction_reproduces_the_accumulator() {
        for rate in DEVICE_RATES {
            let (num, den) = SampleClock::new(rate).ratio();
            assert_eq!(
                u128::from(num) * u128::from(CPU_CLOCK),
                u128::from(den) * exact_numerator(rate),
                "rate {rate}: {num}/{den} is not rate*{FRAME_CYCLES}/{CPU_CLOCK}"
            );
            assert_eq!(gcd(num, den), 1, "rate {rate}: {num}/{den} is not reduced");
            assert!(num >= den, "rate {rate}: a frame carries at least one sample");
        }
    }

    // ---- the conversion ----------------------------------------------------

    /// Full scale does not wrap when the driver narrows to S16.
    ///
    /// 32768 does not fit in an `i16` and becomes -32768, inverting a
    /// full-scale sample. A same-phase stereo pair is what reaches the ceiling,
    /// because the downmix is what brings it back to 1.0.
    #[test]
    fn a_full_scale_stereo_pair_lands_below_the_s16_wrap() {
        assert_eq!(to_int32(1.0, 1.0, 150), 32_767, "must be 32767, not 32768");
        assert_ne!(to_int32(1.0, 1.0, 150), SAMPLE_FLOOR, "must not wrap negative");
        // The negative extreme is representable, so it is allowed through.
        assert_eq!(to_int32(-1.0, -1.0, 150), SAMPLE_FLOOR);
    }

    /// No input, at any volume, ever produces the unrepresentable value.
    ///
    /// A sweep rather than a few cases: the ceiling has to hold across the whole
    /// domain, not for the values someone thought to write down.
    #[test]
    fn no_sample_ever_lands_on_the_unrepresentable_value() {
        for step in -100i32..=100 {
            let level = step as f32 / 100.0;
            for volume in [0u32, 1, 75, 150, 300] {
                for (l, r) in [(level, level), (level, -level), (1.0, level)] {
                    let out = to_int32(l, r, volume);
                    assert_ne!(out, 32_768, "level {level} volume {volume} gave 32768");
                    assert!((SAMPLE_FLOOR..=SAMPLE_CEILING).contains(&out));
                }
            }
        }
    }

    /// A NaN sample is silence: not a panic, and not a garbage value.
    ///
    /// `f32::clamp` propagates NaN and a NaN cast to `i32` is 0, so this is
    /// worth pinning rather than assuming.
    #[test]
    fn a_not_a_number_sample_is_silence() {
        assert_eq!(to_int32(f32::NAN, 0.0, 150), 0);
        assert_eq!(to_int32(0.0, f32::NAN, 150), 0);
        assert_eq!(to_int32(f32::INFINITY, 0.0, 150), SAMPLE_CEILING);
    }

    /// Volume scales the sample and nothing else.
    #[test]
    fn volume_scales_the_sample_and_nothing_else() {
        let half = VOLUME_NOMINAL as u32 / 2;
        let at_half_volume_input = to_int32(0.5, 0.5, VOLUME_NOMINAL as u32) as f32;
        assert_eq!(at_half_volume_input, SAMPLE_SCALE * 0.5);
        assert_eq!(to_int32(0.5, 0.5, half) as f32, at_half_volume_input * 0.5);
        assert_eq!(to_int32(0.5, 0.5, 0), 0, "zero volume is silence");
    }

    // ---- the drain ---------------------------------------------------------

    /// A ring that came up short is padded and counted, never shortened.
    ///
    /// The frame keeps the length the clock asked for. Producing fewer samples
    /// would be quieter and would put the drift straight back.
    ///
    /// The ring holds *some* samples and fewer than a frame, on purpose. A ring
    /// with nothing in it takes the early return in `advance_frame` and never
    /// reaches the padding at all -- which is how this test was vacuous the
    /// first time it was written, and how the corresponding mutation slipped
    /// through green.
    #[test]
    fn a_short_ring_is_padded_with_silence_and_counted() {
        const HELD: usize = 50;
        let (mut tx, mut audio) = sink(RING_SLOTS, HELD, alternate);
        let want = SampleClock::new(RATE_DEFAULT).advance() as usize;
        assert!(HELD < want, "the ring must be short or this tests nothing");

        audio.advance_frame();
        assert_eq!(audio.frame.len(), want, "the frame must keep its full length");
        assert_eq!(
            audio.underruns(),
            (want - HELD) as u64,
            "exactly the missing frames are counted"
        );
        let expected = (SAMPLE_SCALE * 0.5) as i32;
        assert!(
            audio.frame[..HELD].iter().all(|&s| s == expected),
            "the samples that did arrive are still converted"
        );
        assert!(
            audio.frame[HELD..].iter().all(|&s| s == 0),
            "and the rest is padded with silence"
        );
        assert!(tx.push(0.0).is_ok(), "the slots we freed are writable again");
    }

    /// A read that spans the wrap keeps the channels paired.
    ///
    /// This is the test for one specific trap. `ReadChunk::as_slices` returns
    /// two slices cut at the ring's wrap boundary, *not* in half, so code that
    /// reads it as (lefts, rights) works perfectly until the read crosses the
    /// end of the buffer -- and then swaps the channels or reads out of range.
    /// The sequence below parks the head near the end of the ring on purpose.
    #[test]
    fn a_read_that_spans_the_wrap_keeps_the_channels_paired() {
        // Sized so the ring holds a little more than one frame at 44.1 kHz.
        let capacity = 1500;
        let (mut tx, mut audio) = sink(capacity, capacity / 2, alternate);
        // First drain starts at the head and stops short of the wrap: 738
        // frames is 1476 slots out of 1500, so the head ends at 1476 with 24
        // slots still to read.
        audio.advance_frame();
        assert_eq!(audio.pending_slots(), capacity - 1476);

        // Refill to the brim. Now the head is at 1476 and the next read wants
        // the same 1476 slots, so it runs off the end: rtrb will hand back
        // 24 slots then 1452, which is the split `as_slices` gets wrong.
        while tx.slots() >= 2 {
            assert!(tx.push(1.0).is_ok() && tx.push(0.0).is_ok());
        }
        assert_eq!(audio.pending_slots(), capacity, "the read below only wraps if this holds");
        audio.advance_frame();

        let expected = (SAMPLE_SCALE * 0.5) as i32;
        assert!(
            audio.frame.iter().all(|&s| s == expected),
            "a mis-paired read would give {}, not {expected}",
            SAMPLE_CEILING
        );
        assert_eq!(audio.underruns(), 0, "the ring was full, so nothing is missing");
    }

    /// Whatever is left in the ring is a whole number of stereo frames.
    ///
    /// The core pushes left and right together, so an odd remainder would mean
    /// the next read takes half a frame and the channels swap for good.
    ///
    /// The ring holds more than a frame wants but less than twice that, which
    /// is the only window where "read `slots / 2`" and "read `slots`" actually
    /// differ. Wider windows make both agree and the test goes vacuous; a
    /// narrower one fails the read outright and the parity check passes
    /// trivially on an untouched ring. Both of those happened here first.
    #[test]
    fn draining_leaves_a_whole_number_of_stereo_frames() {
        const HELD: usize = 500;
        let (mut tx, mut audio) = sink(RING_SLOTS, HELD, alternate);
        let want = SampleClock::new(RATE_DEFAULT).advance() as usize;
        assert!(
            want < 2 * HELD,
            "the ring must hold more than one frame wants, or the two readings agree"
        );

        audio.advance_frame();
        assert_eq!(audio.pending_slots() % 2, 0, "an odd remainder swaps the channels");
        assert_eq!(audio.pending_slots(), 0, "every whole pair was consumed");
        assert_eq!(
            audio.underruns(),
            (want - HELD) as u64,
            "a read that took an odd count would have failed instead"
        );
        assert!(tx.push(0.0).is_ok(), "the slots we freed are writable again");
    }

    /// The ring outlasts several frames at the highest rate offered.
    ///
    /// The constant is asserted at compile time; this checks that `rtrb` really
    /// hands out that capacity, which the compile-time assert cannot see.
    #[test]
    fn the_ring_outlasts_five_frames_at_the_highest_supported_rate() {
        let (mut tx, audio) = sink(RING_SLOTS, 0, alternate);
        let frames = 5 * MAX_FRAMES_PER_VIDEO_FRAME;
        for i in 0..frames {
            let level = f32::from(u8::try_from(i % 2).unwrap_or(0));
            assert!(tx.push(level).is_ok(), "ring too small at frame {i}");
            assert!(tx.push(level).is_ok());
        }
        assert_eq!(audio.pending_slots(), 2 * frames);
        assert!(2 * frames <= RING_SLOTS);
    }

    // ---- the ABI -----------------------------------------------------------

    /// A buffer too small is refused, and nothing is written to it.
    ///
    /// The required size is still reported, so a caller can size its buffer from
    /// one call rather than by trial and error.
    #[test]
    fn a_buffer_smaller_than_the_frame_reports_the_size_and_writes_nothing() {
        let (_tx, mut audio) = sink(RING_SLOTS, 200, alternate);
        audio.advance_frame();
        let need = audio.frame.len() as u32;
        assert!(need > 0);

        const SENTINEL: i32 = 0x5A5A_5A5A;
        let mut dst = vec![SENTINEL; need as usize];
        let mut written = 0u32;
        // SAFETY: `dst` holds `need` elements and `written` is a live local.
        let status = unsafe { audio.render(dst.as_mut_ptr(), need - 1, &mut written) };
        assert_eq!(status, super::GBA_ERR_CAPACITY);
        assert_eq!(written, need, "the required size is reported anyway");
        assert!(dst.iter().all(|&s| s == SENTINEL), "a refused render must not write");

        // And the happy path does write all of it.
        // SAFETY: as above, with a sufficient capacity this time.
        let status = unsafe { audio.render(dst.as_mut_ptr(), need, &mut written) };
        assert_eq!(status, super::GBA_OK);
        assert_eq!(written, need);
        assert_eq!(dst, audio.frame);
    }

    /// Reset drops the residual and the counter, so both are per-run.
    #[test]
    fn reset_clears_the_residual_and_the_counter() {
        let (_tx, mut audio) = sink(4, 0, alternate);
        audio.advance_frame();
        assert!(audio.underruns() > 0, "the setup produced a shortfall");
        audio.reset();
        assert_eq!(audio.underruns(), 0);
        assert!(audio.frame.is_empty(), "last frame's samples must not be replayed");
    }

    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            let t = b;
            b = a % b;
            a = t;
        }
        a
    }
}
