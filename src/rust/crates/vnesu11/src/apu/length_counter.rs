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
}

impl Default for LengthCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl LengthCounter {
    pub const fn new() -> Self {
        Self { counter: 0, enabled: false }
    }

    /// Load the counter from a length-index register write
    /// ($4003/$4007/$400B/$400F).  Only takes effect when the channel
    /// is enabled ($4015 bit), matching C++ `SQReload` /
    /// `Write_PSG case 0x3/0x7/0xB/0xF` which gate on `EnabledChannels`.
    #[inline]
    pub fn load_from_write(&mut self, index: u8) {
        if self.enabled {
            self.counter = LENGTH_TABLE[(index & 0x1F) as usize];
        }
    }

    /// Half-frame tick (called by frame counter's half-frame event).
    ///
    /// `halt` is the channel's length-counter halt flag: bit 5 of
    /// $4000/$4004/$400C for pulse/noise, bit 7 of $4008 for triangle.
    /// When set, the counter is held and does not decrement (C++
    /// `FrameSoundStuff` checks `!(PSG[x]&0x20)` / `!(PSG[8]&0x80)`).
    #[inline]
    pub fn tick(&mut self, halt: bool) {
        if !self.enabled {
            // Per NESdev: when disabled, length counter is forced to 0.
            self.counter = 0;
            return;
        }
        if halt {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_counter_load_from_write() {
        let mut lc = LengthCounter::new();
        lc.enabled = true;
        lc.load_from_write(0);
        assert_eq!(lc.counter, 10); // LENGTH_TABLE[0] = 10
        // Disabled channel ignores the load.
        let mut disabled = LengthCounter::new();
        disabled.load_from_write(0);
        assert_eq!(disabled.counter, 0);
    }

    #[test]
    fn length_counter_counts_down() {
        let mut lc = LengthCounter::new();
        lc.enabled = true;
        lc.counter = 5;
        lc.tick(false);
        assert_eq!(lc.counter, 4);
        lc.tick(false);
        assert_eq!(lc.counter, 3);
    }

    #[test]
    fn length_counter_halt_holds() {
        let mut lc = LengthCounter::new();
        lc.enabled = true;
        lc.counter = 5;
        lc.tick(true);
        assert_eq!(lc.counter, 5);
    }

    #[test]
    fn length_counter_disabled_clamps_to_zero() {
        let mut lc = LengthCounter::new();
        lc.enabled = false;
        lc.counter = 5;
        // Per NESdev: when disabled, the length counter is forced to 0.
        lc.tick(false);
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