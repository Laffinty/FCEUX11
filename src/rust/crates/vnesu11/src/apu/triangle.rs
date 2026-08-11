//! Triangle channel — 32-step triangle wave generator.
//!
//! The triangle has 32 amplitude steps.  Its timer is clocked at
//! half-CPU rate (i.e., one tick every other CPU cycle).
//!
//! Reference: NESdev wiki "APU triangle".

use super::length_counter::LengthCounter;
use super::linear_counter::LinearCounter;

/// Triangle sequence (32 steps).
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

#[derive(Debug, Clone)]
pub struct TriangleChannel {
    /// Linear counter.
    pub linear_counter: LinearCounter,
    /// Length counter.
    pub length: LengthCounter,
    /// Timer period reload (0..=0x7FF).
    pub timer_period: u16,
    /// Timer counter.
    pub timer_counter: u16,
    /// Linear counter reload value (from $4008).
    pub linear_reload: u8,
    /// Sequence step (0..=31).
    pub seq_step: u8,
    /// Half-clock toggle (triangle advances every other CPU cycle).
    pub alt: u8,
}

impl Default for TriangleChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl TriangleChannel {
    pub fn new() -> Self {
        Self {
            linear_counter: LinearCounter::new(),
            length: LengthCounter::new(),
            timer_period: 0,
            timer_counter: 0,
            linear_reload: 0,
            seq_step: 0,
            alt: 0,
        }
    }

    /// $4008 write — linear counter control + reload.
    pub fn write_control(&mut self, val: u8) {
        self.linear_counter.control_flag = (val & 0x80) != 0;
        self.linear_reload = val & 0x7F;
        self.linear_counter.reload_flag = true;
    }

    /// $400A write — timer low.
    pub fn write_timer_lo(&mut self, val: u8) {
        self.timer_period = (self.timer_period & 0x700) | val as u16;
    }

    /// $400B write — timer high + length load.
    pub fn write_timer_hi(&mut self, val: u8) {
        self.timer_period = (self.timer_period & 0xFF) | (((val & 0x07) as u16) << 8);
        self.length.load((val >> 3) & 0x1F);
        if self.length.enabled {
            self.length.apply_load();
        }
        // Also reload linear counter on $400B write.
        self.linear_counter.reload_flag = true;
    }

    /// One CPU cycle tick (but triangle advances every 2 cycles).
    /// We call this every CPU cycle but skip the actual timer clock
    /// on odd cycles.
    pub fn tick(&mut self) {
        self.alt ^= 1;
        if self.alt == 0 {
            self.tick_timer();
        }
    }

    fn tick_timer(&mut self) {
        if self.linear_counter.is_silent() || self.length.is_silent() {
            return;
        }
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            self.seq_step = (self.seq_step + 1) & 0x1F;
        } else {
            self.timer_counter -= 1;
        }
    }

    pub fn output(&self) -> u8 {
        if self.linear_counter.is_silent() || self.length.is_silent() {
            return 0;
        }
        TRIANGLE_SEQUENCE[self.seq_step as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_silent_when_length_zero() {
        let t = TriangleChannel::new();
        assert_eq!(t.output(), 0);
    }

    #[test]
    fn triangle_silent_when_linear_counter_zero() {
        let mut t = TriangleChannel::new();
        t.length.enabled = true;
        t.length.counter = 100;
        t.linear_counter.control_flag = true;
        t.linear_counter.counter = 0;
        assert_eq!(t.output(), 0);
    }

    #[test]
    fn triangle_sequence_steps() {
        let mut t = TriangleChannel::new();
        t.length.enabled = true;
        t.length.counter = 100;
        t.linear_counter.control_flag = true;
        t.linear_counter.reload_value = 50;
        t.linear_counter.reload_flag = true;
        t.linear_counter.tick(); // counter = 50
        // Initial seq_step = 0 → output 15
        assert_eq!(t.output(), 15);
        t.seq_step = 16;
        // seq_step 16 → output 0 (bottom of triangle)
        assert_eq!(t.output(), 0);
        t.seq_step = 31;
        // seq_step 31 → output 15 (top again)
        assert_eq!(t.output(), 15);
    }
}