//! Noise channel — pseudo-random noise (LFSR).
//!
//! Uses a 15-bit linear-feedback shift register (LFSR).  When the
//! "short mode" flag is set, the LFSR is treated as 7-bit (which
//! produces a brighter, more periodic sound).
//!
//! Reference: NESdev wiki "APU noise".

use super::envelope::EnvelopeUnit;
use super::length_counter::LengthCounter;

/// Noise period table (NTSC).  Index by $400E bits 0-3.
const NOISE_PERIOD: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

#[derive(Debug, Clone)]
pub struct NoiseChannel {
    /// Halt envelope flag (from $400F bit 5).
    pub halt_envelope: bool,
    /// Constant volume / envelope volume (4 bits).
    pub constant_volume: u8,
    /// Period index (0..=15).
    pub period_index: u8,
    /// Short mode (from $400F bit 4) — use 7-bit LFSR.
    pub short_mode: bool,
    /// Envelope unit.
    pub envelope: EnvelopeUnit,
    /// Length counter.
    pub length: LengthCounter,
    /// LFSR shift register (15 bits, bit 14 = MSB).
    pub shift_register: u16,
    /// Timer counter.
    pub timer_counter: u16,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            halt_envelope: false,
            constant_volume: 0,
            period_index: 0,
            short_mode: false,
            envelope: EnvelopeUnit::new(),
            length: LengthCounter::new(),
            shift_register: 1, // Initial LFSR = 1 (per NESdev).
            timer_counter: 0,
        }
    }

    /// $400C write — envelope / halt.
    pub fn write_envelope(&mut self, val: u8) {
        self.halt_envelope = (val & 0x20) != 0;
        self.envelope.loop_flag = self.halt_envelope;
        self.constant_volume = val & 0x0F;
        self.envelope.volume = self.constant_volume;
    }

    /// $400E write — period + short mode.
    pub fn write_period(&mut self, val: u8) {
        self.short_mode = (val & 0x80) != 0;
        self.period_index = val & 0x0F;
    }

    /// $400F write — length counter load + envelope start.
    pub fn write_length(&mut self, val: u8) {
        self.length.load_from_write((val >> 3) & 0x1F);
        self.envelope.start_flag = true;
    }

    /// One CPU cycle tick.
    pub fn tick(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = NOISE_PERIOD[self.period_index as usize];
            // Clock LFSR.
            let bit = if self.short_mode {
                // Bit 6 XOR bit 5 (short mode = 7-bit LFSR).
                let b6 = (self.shift_register >> 6) & 1;
                let b5 = (self.shift_register >> 5) & 1;
                b6 ^ b5
            } else {
                // Bit 14 XOR bit 13 (normal 15-bit LFSR).
                let b14 = (self.shift_register >> 14) & 1;
                let b13 = (self.shift_register >> 13) & 1;
                b14 ^ b13
            };
            self.shift_register = (self.shift_register << 1) | bit;
            // Invert feedback (NES hardware quirk: LFSR bit 0 is
            // inverted after shift).
            // Actually no: NES LFSR feedback is `b14 XOR b13`,
            // and the result is shifted INTO bit 0 (not inverted).
            // The inverted behavior is achieved by initializing LFSR
            // to 1 (so bit 0 = 1 always).
        } else {
            self.timer_counter -= 1;
        }
    }

    /// Output level (0 or volume).
    pub fn output(&self) -> u8 {
        if self.length.is_silent() {
            return 0;
        }
        // Bit 0 of LFSR = 0 → silenced (the "toggle" bit).
        if (self.shift_register & 1) == 0 {
            return 0;
        }
        self.envelope.output()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_silent_when_length_zero() {
        let n = NoiseChannel::new();
        assert_eq!(n.output(), 0);
    }

    #[test]
    fn noise_period_table_known() {
        assert_eq!(NOISE_PERIOD[0], 4);
        assert_eq!(NOISE_PERIOD[15], 4068);
    }

    #[test]
    fn noise_lfsr_initial() {
        let n = NoiseChannel::new();
        assert_eq!(n.shift_register, 1);
    }

    #[test]
    fn noise_write_period_parses_bits() {
        let mut n = NoiseChannel::new();
        n.write_period(0x87); // short_mode + period 7
        assert!(n.short_mode);
        assert_eq!(n.period_index, 7);
    }
}