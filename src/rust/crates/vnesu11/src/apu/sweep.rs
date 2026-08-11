//! Pulse sweep unit (used by Pulse channels 1 & 2).
//!
//! The sweep periodically adjusts the pulse channel's timer period.
//! It can either increase (shift right) or decrease (shift left) the
//! period, with a configurable rate.
//!
//! Reference: NESdev wiki "APU sweep".

#[derive(Debug, Clone, Copy)]
pub struct SweepUnit {
    /// Channel index (0 or 1) — for debugging.
    pub channel: u8,
    /// Enabled flag (from $4001 / $4005 bit 7).
    pub enabled: bool,
    /// Period reload (0..=7) — sweep tick rate.
    pub period: u8,
    /// Shift count (0..=7) — number of bits to shift.
    pub shift: u8,
    /// Direction: false = decrease, true = increase.
    pub negate: bool,
    /// Counter (decrements at quarter-frame rate).
    pub counter: u8,
    /// Reload flag — set by $4001/$4005 write, cleared after one tick.
    pub reload_flag: bool,
    /// Whether to negate channel 1's sweep (the only quirk: ch1
    /// uses one's complement, ch2 uses two's complement).  This is
    /// the famous "sweep quirk" — see NESdev wiki.
    pub ones_complement: bool,
}

impl Default for SweepUnit {
    fn default() -> Self {
        Self::new(0)
    }
}

impl SweepUnit {
    pub const fn new(channel: u8) -> Self {
        Self {
            channel,
            enabled: false,
            period: 0,
            shift: 0,
            negate: false,
            counter: 0,
            reload_flag: false,
            // ch1 uses one's complement for negative sweep result;
            // ch2 uses two's complement. Default = ch2 behavior;
            // channel 1 enables ones_complement.
            ones_complement: channel == 0,
        }
    }

    /// Half-frame tick.  Called by the frame counter.
    pub fn tick(&mut self) {
        if self.counter == 0 && self.enabled && self.shift > 0 {
            // Sweep adjustment happens here.  Caller is expected to
            // query `target_period()` to apply.
        }
        if self.reload_flag {
            self.counter = self.period;
            self.reload_flag = false;
        } else if self.counter > 0 {
            self.counter -= 1;
        }
    }

    /// Compute the swept target period given the current `curfreq`.
    /// Returns `None` if the target period would underflow (caller
    /// should silence the channel).
    pub fn target_period(&self, curfreq: u16) -> Option<u16> {
        if self.shift == 0 || !self.enabled {
            return Some(curfreq);
        }
        let delta = curfreq >> self.shift;
        if self.negate {
            // Sweep DOWN — subtract delta.  Use ones or two's
            // complement depending on channel (sweep quirk).
            let negated = if self.ones_complement {
                (delta ^ 0xFFFF).wrapping_add(1) & 0x7FF // 11-bit
            } else {
                (curfreq.wrapping_sub(delta)) & 0x7FF
            };
            Some(negated)
        } else {
            // Sweep UP — add delta.  If overflow (>0x7FF), channel
            // is silenced.
            let candidate = curfreq.wrapping_add(delta);
            if candidate > 0x7FF {
                None
            } else {
                Some(candidate & 0x7FF)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_target_no_shift() {
        let s = SweepUnit::new(1);
        assert_eq!(s.target_period(0x123), Some(0x123));
    }

    #[test]
    fn sweep_target_positive_shift() {
        let mut s = SweepUnit::new(1);
        s.enabled = true;
        s.shift = 2;
        // 0x400 >> 2 = 0x100.  curfreq 0x400 + 0x100 = 0x500.
        assert_eq!(s.target_period(0x400), Some(0x500));
    }

    #[test]
    fn sweep_target_negative_shift() {
        let mut s = SweepUnit::new(1);
        s.enabled = true;
        s.shift = 2;
        s.negate = true;
        // 0x400 >> 2 = 0x100.  0x400 - 0x100 = 0x300.
        assert_eq!(s.target_period(0x400), Some(0x300));
    }

    #[test]
    fn sweep_target_overflow() {
        let mut s = SweepUnit::new(1);
        s.enabled = true;
        s.shift = 4;
        // curfreq + delta > 0x7FF → None.
        assert!(s.target_period(0x7F0).is_none());
    }
}