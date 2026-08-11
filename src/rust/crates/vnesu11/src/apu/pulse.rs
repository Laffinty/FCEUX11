//! Pulse channel (1 of 2) — square-wave generator with envelope
//! + sweep + length counter.
//!
//! Period table is fixed at 11 bits (0..=0x7FF).  The actual
//! frequency is `CPU_RATE_HZ / (16 * (period + 1))` Hz.
//!
//! Duty cycle table (0%, 25%, 50%, 75%) per NESdev wiki:
//! - 0: 01000000
//! - 1: 01100000
//! - 2: 01111000
//! - 3: 10011111
//!
//! Reference: `src/apu.cpp` (FCEUX upstream).

use super::envelope::EnvelopeUnit;
use super::length_counter::LengthCounter;
use super::sweep::SweepUnit;

/// Pulse duty cycles — 8-step sequence per cycle.
const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 75% (inverted 25%)
];

/// Pulse channel state.
#[derive(Debug, Clone)]
pub struct PulseChannel {
    /// Channel index (0 = $4000-$4003, 1 = $4004-$4007).
    pub channel: u8,
    /// Duty cycle (0..=3).
    pub duty: u8,
    /// Halt envelope (when envelope's loop flag is set).
    pub halt_envelope: bool,
    /// Constant volume (4 bits, when envelope disabled).
    pub constant_volume: u8,
    /// Sweep unit.
    pub sweep: SweepUnit,
    /// Envelope unit.
    pub envelope: EnvelopeUnit,
    /// Length counter.
    pub length: LengthCounter,
    /// Current timer period (0..=0x7FF).
    pub timer_period: u16,
    /// Current timer value (down-counter, decrement at CPU rate).
    pub timer_counter: u16,
    /// Sequence step (0..=7).
    pub seq_step: u8,
}

impl Default for PulseChannel {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PulseChannel {
    pub fn new(channel: u8) -> Self {
        Self {
            channel,
            duty: 0,
            halt_envelope: false,
            constant_volume: 0,
            sweep: SweepUnit::new(channel),
            envelope: EnvelopeUnit::new(),
            length: LengthCounter::new(),
            timer_period: 0,
            timer_counter: 0,
            seq_step: 0,
        }
    }

    /// $4000 / $4004 write.
    pub fn write_control(&mut self, val: u8) {
        self.duty = (val >> 6) & 0x03;
        self.halt_envelope = (val & 0x20) != 0;
        self.envelope.loop_flag = self.halt_envelope;
        self.constant_volume = val & 0x0F;
        self.envelope.volume = self.constant_volume;
    }

    /// $4001 / $4005 write — sweep.
    pub fn write_sweep(&mut self, val: u8) {
        self.sweep.enabled = (val & 0x80) != 0;
        self.sweep.period = (val >> 4) & 0x07;
        self.sweep.negate = (val & 0x08) != 0;
        self.sweep.shift = val & 0x07;
        self.sweep.reload_flag = true;
    }

    /// $4002 / $4006 write — timer low.
    pub fn write_timer_lo(&mut self, val: u8) {
        self.timer_period = (self.timer_period & 0x700) | val as u16;
    }

    /// $4003 / $4007 write — timer high + length counter load.
    pub fn write_timer_hi(&mut self, val: u8) {
        self.timer_period = (self.timer_period & 0xFF) | (((val & 0x07) as u16) << 8);
        // Reset phase + length counter.
        if self.length.enabled {
            self.length.apply_load();
        }
        self.seq_step = 0;
    }

    /// One CPU cycle tick.
    #[inline]
    pub fn tick(&mut self) {
        // Timer clocked at half-CPU-rate (1 tick every 2 CPU cycles
        // when period is 0).  Simplified: decrement by 2 each call,
        // wrapping at 16 (i.e., timer_period + 1).
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            self.seq_step = (self.seq_step + 1) & 0x07;
        } else {
            self.timer_counter -= 1;
        }
    }

    /// Output level (0..=15).  Returns 0 when silenced.
    pub fn output(&self) -> u8 {
        if self.length.is_silent() {
            return 0;
        }
        let duty_byte = DUTY_TABLE[self.duty as usize][self.seq_step as usize];
        if duty_byte == 0 {
            return 0;
        }
        self.envelope.output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_output_silent_when_length_zero() {
        let mut p = PulseChannel::new(0);
        p.length.enabled = false;
        assert_eq!(p.output(), 0);
    }

    #[test]
    fn pulse_output_duty_cycle() {
        let mut p = PulseChannel::new(0);
        p.length.enabled = true;
        p.length.counter = 255;
        p.duty = 2; // 50%
        p.envelope.start_flag = true;
        p.envelope.volume = 10;
        p.envelope.tick(); // sets output to 10
        // Sequence at step 0: DUTY_TABLE[2][0] = 0 → output 0
        assert_eq!(p.output(), 0);
        p.seq_step = 1;
        // DUTY_TABLE[2][1] = 1 → output 10
        assert_eq!(p.output(), 10);
    }

    #[test]
    fn pulse_write_timer_hi_resets_phase() {
        let mut p = PulseChannel::new(0);
        p.seq_step = 5;
        p.length.enabled = true;
        p.length.counter = 10;
        p.write_timer_hi(0x40);
        // After write, sequence resets to 0.
        assert_eq!(p.seq_step, 0);
        // Length counter is NOT reloaded (apply_load only if counter == 0).
        assert_eq!(p.length.counter, 10);
    }

    #[test]
    fn pulse_write_timer_hi_reloads_when_counter_zero() {
        let mut p = PulseChannel::new(0);
        p.length.enabled = true;
        p.length.counter = 0;
        p.length.load(5);
        p.write_timer_hi(0x40);
        // Counter was 0, so apply_load loads from LENGTH_TABLE[5].
        assert_eq!(p.length.counter, super::super::length_counter::LENGTH_TABLE[5]);
    }
}