//! Volume envelope (used by Pulse and Noise channels).
//!
//! The envelope is a 4-bit down-counter that generates a volume
//! level.  When the counter reaches 0, it either stays at 0
//! (if looping is disabled) or reloads from `volume` (if looping).
//!
//! The envelope clock is driven at quarter-frame rate by the
//! frame counter (in `ApuCore::tick`).
//!
//! Reference: NESdev wiki "APU envelope".

/// Envelope state (4 bits + loop flag).
#[derive(Debug, Clone, Copy)]
pub struct EnvelopeUnit {
    /// Current volume (0..=15).
    pub volume: u8,
    /// Period counter (down-counter, period = `volume + 1`).
    pub decay_count: u8,
    /// Loop flag: 1 = loop on underflow, 0 = clamp at 0.
    pub loop_flag: bool,
    /// Reload value (from $4000/$4004/$400C bits 0-3 = volume).
    pub reload_value: u8,
    /// Start flag: 1 = reload volume + restart.
    pub start_flag: bool,
    /// Current output level (0..=15).
    pub output: u8,
}

impl Default for EnvelopeUnit {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvelopeUnit {
    pub const fn new() -> Self {
        Self {
            volume: 0,
            decay_count: 0,
            loop_flag: false,
            reload_value: 0,
            start_flag: false,
            output: 0,
        }
    }

    /// Half-frame tick.  Called once per APU quarter-frame.
    #[inline]
    pub fn tick(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_count = self.volume + 1;
            self.output = self.volume;
            return;
        }
        if self.decay_count > 0 {
            self.decay_count -= 1;
        }
        if self.decay_count == 0 {
            if self.loop_flag {
                self.decay_count = self.volume + 1;
                self.output = self.volume;
            } else if self.output > 0 {
                self.output -= 1;
                self.decay_count = self.volume + 1;
            }
        }
    }

    /// Output level (0..=15).
    #[inline]
    pub fn output(&self) -> u8 {
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_start_resets_output() {
        let mut e = EnvelopeUnit::new();
        e.volume = 10;
        e.start_flag = true;
        e.tick();
        assert_eq!(e.output, 10);
        assert!(!e.start_flag);
    }

    #[test]
    fn envelope_decay_no_loop() {
        let mut e = EnvelopeUnit::new();
        e.volume = 2; // period = 3
        e.start_flag = true;
        e.tick(); // output = 2
        for _ in 0..3 {
            e.tick();
        }
        // After 3 ticks past the start, output should be 1.
        assert_eq!(e.output, 1);
    }

    #[test]
    fn envelope_loop_at_zero() {
        let mut e = EnvelopeUnit::new();
        e.volume = 1;
        e.loop_flag = true;
        e.start_flag = true;
        e.tick(); // output = 1
        // After enough ticks, output wraps back to volume (1).
        for _ in 0..10 {
            e.tick();
        }
        assert_eq!(e.output, 1);
    }

    #[test]
    fn envelope_clamp_at_zero() {
        let mut e = EnvelopeUnit::new();
        e.volume = 15;
        e.loop_flag = false;
        e.start_flag = true;
        e.tick();
        // After 16*16 ticks, output should be 0.
        for _ in 0..16 * 16 {
            e.tick();
        }
        assert_eq!(e.output, 0);
    }
}