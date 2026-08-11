//! Length counter (used by all 5 APU channels).
//!
//! When enabled, the length counter counts down at half-frame rate
//! and silences the channel when it reaches 0.  When disabled
//! (`enabled == false`), the counter is held at 0 and the channel
//! is silenced.
//!
//! Reference: NESdev wiki "APU length counter".

/// Length counter table — values loaded by writing $4003 / $4007 /
/// $400B / $400F bits 3-7 (5-bit index).  Each entry is the duration
/// in (NTSC) frames; APU converts to half-frame ticks internally.
pub const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

#[derive(Debug, Clone, Copy)]
pub struct LengthCounter {
    /// Counter value (0 = channel silenced).
    pub counter: u8,
    /// Enabled flag (set by $4015 bit, cleared by status write).
    pub enabled: bool,
    /// Loaded on $4003/$4007/etc. write (5-bit index into table).
    pub load_index: u8,
}

impl Default for LengthCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl LengthCounter {
    pub const fn new() -> Self {
        Self { counter: 0, enabled: false, load_index: 0 }
    }

    /// Load counter from $4003/$4007/$400B/$400F write.
    #[inline]
    pub fn load(&mut self, index: u8) {
        self.load_index = index & 0x1F;
    }

    /// Half-frame tick (called by frame counter's half-frame event).
    #[inline]
    pub fn tick(&mut self) {
        if !self.enabled {
            // Per NESdev: when disabled, length counter is forced to 0.
            self.counter = 0;
            return;
        }
        if self.counter > 0 {
            self.counter -= 1;
        }
    }

    /// Update enabled flag from $4015 write.
    #[inline]
    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.counter = 0;
        }
    }

    /// Channel is silent when length counter is 0.
    #[inline]
    pub fn is_silent(&self) -> bool {
        self.counter == 0
    }

    /// Apply the load_index to the counter (called when the channel
    /// is *not* silenced by other means — e.g. when $4003 write
    /// happens).  This is a separate call so the channel logic can
    /// short-circuit (e.g. triangle ulock flag).
    #[inline]
    pub fn apply_load(&mut self) {
        if self.counter == 0 {
            self.counter = LENGTH_TABLE[self.load_index as usize];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_counter_load() {
        let mut lc = LengthCounter::new();
        lc.load(0);
        assert_eq!(lc.load_index, 0);
        lc.apply_load();
        assert_eq!(lc.counter, 10); // LENGTH_TABLE[0] = 10
    }

    #[test]
    fn length_counter_counts_down() {
        let mut lc = LengthCounter::new();
        lc.enabled = true;
        lc.counter = 5;
        lc.tick();
        assert_eq!(lc.counter, 4);
        lc.tick();
        assert_eq!(lc.counter, 3);
    }

    #[test]
    fn length_counter_disabled_clamps_to_zero() {
        let mut lc = LengthCounter::new();
        lc.enabled = false;
        lc.counter = 5;
        // Per NESdev: when disabled, the length counter is forced to 0.
        lc.tick();
        assert_eq!(lc.counter, 0);
    }

    #[test]
    fn length_counter_set_enabled_clears_counter() {
        let mut lc = LengthCounter::new();
        lc.enabled = true;
        lc.counter = 5;
        lc.set_enabled(false);
        assert_eq!(lc.counter, 0);
    }

    #[test]
    fn length_counter_disabled_clears() {
        let mut lc = LengthCounter::new();
        lc.enabled = true;
        lc.counter = 5;
        lc.set_enabled(false);
        assert_eq!(lc.counter, 0);
    }

    #[test]
    fn length_table_known_values() {
        // Pin a few canonical entries from NESdev.
        assert_eq!(LENGTH_TABLE[0], 10);
        assert_eq!(LENGTH_TABLE[1], 254);
        assert_eq!(LENGTH_TABLE[31], 30);
    }
}