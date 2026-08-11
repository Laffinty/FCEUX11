//! Triangle linear counter (triangle channel only).
//!
//! When the linear counter flag is set, the counter reloads and
//! counts down at quarter-frame rate.  When the flag is clear,
//! the counter stays at its current value (it still decrements when
//! the flag is set).  This is the triangle's "halt" control.
//!
//! Reference: NESdev wiki "APU triangle".

#[derive(Debug, Clone, Copy)]
pub struct LinearCounter {
    /// Period reload value (0..=127).
    pub reload_value: u8,
    /// Current counter value (0..=127).
    pub counter: u8,
    /// Control flag: 1 = counter reloads + decrements, 0 = counter
    /// freezes (does NOT decrement).  When clear AND $4008 write,
    /// the counter is silenced.
    pub control_flag: bool,
    /// Reload pending flag (set by $4008 write).
    pub reload_flag: bool,
}

impl Default for LinearCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl LinearCounter {
    pub const fn new() -> Self {
        Self {
            reload_value: 0,
            counter: 0,
            control_flag: false,
            reload_flag: false,
        }
    }

    /// Quarter-frame tick.
    pub fn tick(&mut self) {
        if self.reload_flag {
            self.counter = self.reload_value;
            self.reload_flag = false;
        } else if self.counter > 0 && self.control_flag {
            self.counter -= 1;
        }
    }

    /// Silenced when counter is 0 OR control flag is clear.
    #[inline]
    pub fn is_silent(&self) -> bool {
        self.counter == 0 || !self.control_flag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reload_sets_counter() {
        let mut c = LinearCounter::new();
        c.reload_value = 50;
        c.reload_flag = true;
        c.tick();
        assert_eq!(c.counter, 50);
        assert!(!c.reload_flag);
    }

    #[test]
    fn tick_decrements_when_control_set() {
        let mut c = LinearCounter::new();
        c.control_flag = true;
        c.reload_value = 10;
        c.reload_flag = true;
        c.tick();
        assert_eq!(c.counter, 10);
        c.tick();
        assert_eq!(c.counter, 9);
    }

    #[test]
    fn tick_does_not_decrement_when_control_clear() {
        let mut c = LinearCounter::new();
        c.control_flag = false;
        c.reload_value = 10;
        c.reload_flag = true;
        c.tick();
        assert_eq!(c.counter, 10);
        c.tick();
        assert_eq!(c.counter, 10);
    }

    #[test]
    fn is_silent_when_zero() {
        let mut c = LinearCounter::new();
        c.control_flag = true;
        c.reload_value = 0;
        c.reload_flag = true;
        c.tick();
        assert!(c.is_silent());
    }

    #[test]
    fn is_silent_when_control_clear() {
        let c = LinearCounter::new();
        assert!(c.is_silent());
    }
}