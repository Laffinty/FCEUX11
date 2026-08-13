//! APU module — Ricoh 2A03 audio processing unit.
//!
//! Phase 4 deliverable per `docs/wip_2.0_plan/phase_4_apu_dma.md`. The
//! APU runs at CPU master clock and produces a stereo audio stream
//! (pulse×2 + triangle + noise + DMC).  Mixer uses the standard
//! nonlinear lookup tables (pulse + TND).
//!
//! Reference: `src/apu.cpp` (FCEUX upstream APU implementation) +
//! NESdev wiki "APU".
//!
//! # Submodules
//!
//! - [`mod@envelope`] — Volume envelope (used by pulse, noise)
//! - [`mod@length_counter`] — Length counter (used by all channels)
//! - [`mod@sweep`] — Pulse sweep unit
//! - [`mod@linear_counter`] — Triangle linear counter
//! - [`mod@pulse`] — Pulse channels (2)
//! - [`mod@triangle`] — Triangle channel
//! - [`mod@noise`] — Noise channel (LFSR)
//! - [`mod@dmc`] — DMC channel with sample fetch
//! - [`mod@frame_counter`] — 4-step / 5-step frame counter
//! - [`mod@mixer`] — Non-linear mixer (pulse + TND tables)

pub mod dmc;
pub mod envelope;
pub mod frame_counter;
pub mod length_counter;
pub mod linear_counter;
pub mod mixer;
pub mod noise;
pub mod pulse;
pub mod sweep;
pub mod triangle;

pub use dmc::{DmcChannel, DmcDmaOutcome};
pub use envelope::EnvelopeUnit;
pub use frame_counter::{FrameCounter, FRAME_IRQ_PERIOD_CYCLES};
pub use length_counter::LengthCounter;
pub use linear_counter::LinearCounter;
pub use mixer::{mixer, PULSE_TABLE, TND_TABLE};
pub use noise::NoiseChannel;
pub use pulse::PulseChannel;
pub use sweep::SweepUnit;
pub use triangle::TriangleChannel;

/// APU master cycle counter.
///
/// Each APU tick corresponds to **1 CPU cycle** = 1/3 PPU dot.
/// `cycles` is a monotonically-increasing u64 used by frame counter
/// (which fires at quarter-frame / half-frame cycle counts).
#[derive(Debug, Clone, Copy, Default)]
pub struct ApuCycle(pub u64);

/// APU IRQ source bitmask (consumed by `IrqController`).
///
/// `IRQ_FCOUNT` is the frame counter IRQ; Phase 4 also includes DMC
/// IRQ (which we OR with `IRQ_FCOUNT` for unified semantics — Phase 5
/// mapper hookup can override via the meta vtable).
pub const IRQ_FCOUNT: u32 = 0x200;
pub const IRQ_DMC: u32 = 0x100;

/// Top-level APU state (2A03 equivalent).
///
/// Holds the 5 channels + frame counter + output buffer.  `tick()`
/// advances one CPU cycle and pushes one stereo sample.
#[derive(Debug)]
pub struct ApuCore {
    /// Pulse channel 1 ($4000-$4003).
    pub pulse1: PulseChannel,
    /// Pulse channel 2 ($4004-$4007).
    pub pulse2: PulseChannel,
    /// Triangle channel ($4008-$400B).
    pub triangle: TriangleChannel,
    /// Noise channel ($400C-$400F).
    pub noise: NoiseChannel,
    /// DMC channel ($4010-$4013).
    pub dmc: DmcChannel,
    /// Frame counter ($4017).
    pub frame_counter: FrameCounter,

    /// Frame counter IRQ pending.
    pub frame_irq_pending: bool,
    /// DMC IRQ pending.
    pub dmc_irq_pending: bool,

    /// Master cycle counter.
    pub cycles: u64,
    /// Output audio buffer (interleaved stereo i16 samples).
    pub output_buffer: Vec<i16>,
}

impl Default for ApuCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ApuCore {
    pub fn new() -> Self {
        Self {
            pulse1: PulseChannel::new(0),
            pulse2: PulseChannel::new(1),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            frame_counter: FrameCounter::new(),
            frame_irq_pending: false,
            dmc_irq_pending: false,
            cycles: 0,
            output_buffer: Vec::with_capacity(8192),
        }
    }

    /// Power-on: reset all channels + frame counter + buffer. Frame
    /// counter starts in mode 0 ($4017 = $00) at phase `fhcnt=4`,
    /// matching `FCEUSND_Reset(true)`.
    pub fn power_cycle(&mut self) {
        self.pulse1 = PulseChannel::new(0);
        self.pulse2 = PulseChannel::new(1);
        self.triangle = TriangleChannel::new();
        self.noise = NoiseChannel::new();
        self.dmc = DmcChannel::new();
        self.frame_counter = FrameCounter::new();
        self.frame_counter.power_reset();
        self.frame_irq_pending = false;
        self.dmc_irq_pending = false;
        self.cycles = 0;
        self.output_buffer.clear();
    }

    /// Soft reset (`FCEUSND_Reset(false)`): preserve the last $4017
    /// mode, reset the frame-counter phase to `fhcnt=4`, and clear the
    /// length counters / IRQ flags. Channel register values ($4000-
    /// $4013) are left intact because the ROM's reset handler re-writes
    /// them after the reset vector runs.
    pub fn soft_reset(&mut self) {
        self.frame_counter.soft_reset();
        self.frame_irq_pending = false;
        self.dmc_irq_pending = false;
        self.cycles = 0;
        self.pulse1.length.set_enabled(false);
        self.pulse2.length.set_enabled(false);
        self.triangle.length.set_enabled(false);
        self.noise.length.set_enabled(false);
        // C++ FCEUSND_Reset also clears the sweep enable and the DMC
        // latches/counters; the raw PSG registers are preserved.
        self.pulse1.sweep.enabled = false;
        self.pulse2.sweep.enabled = false;
        self.dmc = DmcChannel::new();
        self.output_buffer.clear();
    }

    /// Advance APU by `cpu_cycles` CPU cycles.  Each cycle:
    /// 1. Quarter / half-frame counter tick (if at boundary).
    /// 2. Each channel tick.
    /// 3. Mix output → push stereo sample to `output_buffer`.
    pub fn tick(&mut self, cpu_cycles: u32) {
        for _ in 0..cpu_cycles {
            self.tick_one();
        }
    }

    /// Advance APU by one CPU cycle.
    #[inline]
    fn tick_one(&mut self) {
        // 1. Frame counter (quarter / half / IRQ)
        self.frame_counter
            .tick(self.cycles, &mut self.frame_irq_pending);

        // 2. Channels (each advances its timer at appropriate rate).
        //    C++ `FrameSoundStuff` clocks envelope decay + the triangle
        //    linear counter on BOTH quarter and half frame steps; length
        //    counters + pulse sweep only on half steps. Length counters
        //    are held when the channel's halt flag is set (pulse/noise
        //    bit 5, triangle bit 7 of $4008).
        if self.frame_counter.quarter_frame || self.frame_counter.half_frame {
            self.pulse1.envelope.tick();
            self.pulse2.envelope.tick();
            self.triangle.linear_counter.tick();
            self.noise.envelope.tick();
        }
        if self.frame_counter.half_frame {
            self.pulse1.length.tick(self.pulse1.halt_envelope);
            self.pulse2.length.tick(self.pulse2.halt_envelope);
            self.triangle
                .length
                .tick(self.triangle.linear_counter.control_flag);
            self.noise.length.tick(self.noise.halt_envelope);
            self.pulse1.sweep.tick();
            self.pulse2.sweep.tick();
        }

        // 3. Channel waveform ticks (half-clock for triangle, full for others)
        self.pulse1.tick();
        self.pulse2.tick();
        self.triangle.tick();
        self.noise.tick();
        self.dmc.tick();

        // 4. Mixer → push stereo sample (L, R are same on mono console)
        let (l, r) = mixer(
            self.pulse1.output(),
            self.pulse2.output(),
            self.triangle.output(),
            self.noise.output(),
            self.dmc.output(),
        );
        self.output_buffer.push(l);
        self.output_buffer.push(r);

        // 5. Master cycle counter
        self.cycles = self.cycles.wrapping_add(1);
    }

    /// Return the pending IRQ bitmask (DMC + frame counter) WITHOUT
    /// clearing the latches. The frame IRQ flag stays set until $4015
    /// is READ (C++ `SIRQStat` bit 6); the DMC IRQ flag stays set
    /// until $4015 is WRITTEN (C++ `StatusWrite` clears bit 7). The
    /// SoC re-asserts these into the `IrqController` on every poll,
    /// which is the level-triggered semantics the hardware uses.
    pub fn take_irq(&mut self) -> u32 {
        let mut mask = 0;
        if self.frame_irq_pending {
            mask |= IRQ_FCOUNT;
        }
        if self.dmc_irq_pending {
            mask |= IRQ_DMC;
        }
        mask
    }

    /// Clear all pending APU IRQs (after CPU has serviced them).
    pub fn clear_irqs(&mut self) {
        self.frame_irq_pending = false;
        // DMC IRQ is level-triggered — stays set until $4015 bit 7 cleared.
        // Phase 6 will wire the $4015 write path.
    }

    /// Total number of stereo sample frames in output buffer.
    pub fn output_frames(&self) -> usize {
        self.output_buffer.len() / 2
    }

    /// Drain output buffer (returns Vec, leaves buffer empty).
    pub fn drain_output(&mut self) -> Vec<i16> {
        core::mem::take(&mut self.output_buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apu_power_cycle_resets_state() {
        let mut a = ApuCore::new();
        a.tick(100);
        assert!(a.cycles >= 100);
        assert!(!a.output_buffer.is_empty());
        a.power_cycle();
        assert_eq!(a.cycles, 0);
        assert!(a.output_buffer.is_empty());
    }

    #[test]
    fn apu_tick_produces_stereo_samples() {
        let mut a = ApuCore::new();
        a.tick(10);
        assert_eq!(a.output_buffer.len(), 20, "10 cycles × 2 channels = 20 samples");
        assert_eq!(a.output_frames(), 10);
    }

    #[test]
    fn apu_silent_channels_produce_silence() {
        // With all channels disabled (default after power), output
        // should be a constant silence value (no pulse + no TND).
        let mut a = ApuCore::new();
        a.tick(4);
        let samples: Vec<i16> = a.drain_output();
        // Both channels silent: pulse_out = PULSE_TABLE[0], tnd_out = TND_TABLE[0].
        // PULSE_TABLE[0] = 0.0 (silence for pulse out).
        // TND_TABLE[0] = 0.0 (silence for TND).
        assert!(samples.iter().all(|&s| s == 0));
    }

    #[test]
    fn apu_take_irq_returns_zero_by_default() {
        let a = ApuCore::new();
        let mask = { let mut x = a; x.take_irq() };
        assert_eq!(mask, 0);
    }
}