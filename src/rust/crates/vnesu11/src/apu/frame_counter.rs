//! APU frame counter.
//!
//! Two modes:
//! - **4-step**: 14914 CPU cycles per frame (3728.5 quarter, 7456.5 half,
//!   11185.5 quarter, 14914.5 half + IRQ)
//! - **5-step**: 18640 CPU cycles per frame (no IRQ)
//!
//! Reference: NESdev wiki "APU frame counter".

/// CPU cycle counts for the 4-step frame counter's events.
/// Values are half-cycle counts; we double them below for CPU cycles.
pub const FRAME_IRQ_PERIOD_CYCLES: u64 = 14914;

#[derive(Debug, Clone, Copy)]
pub struct FrameCounter {
    /// 5-step mode (from $4017 bit 7).  false = 4-step.
    pub five_step: bool,
    /// IRQ inhibit (from $4017 bit 6).  false = IRQ enabled.
    pub irq_inhibit: bool,
    /// Cycle counter (monotonically increments; reset on $4017 write).
    pub cycle_count: u64,
    /// Step counter (0..=3 for 4-step, 0..=4 for 5-step).
    pub step: u8,
    /// Quarter-frame flag (cleared each tick; set when event fires).
    pub quarter_frame: bool,
    /// Half-frame flag (cleared each tick; set when event fires).
    pub half_frame: bool,
}

impl Default for FrameCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameCounter {
    pub fn new() -> Self {
        Self {
            five_step: false,
            irq_inhibit: false,
            cycle_count: 0,
            step: 0,
            quarter_frame: false,
            half_frame: false,
        }
    }

    /// $4017 write.
    pub fn write(&mut self, val: u8) {
        self.five_step = (val & 0x80) != 0;
        self.irq_inhibit = (val & 0x40) != 0;
        // Reset sequence on write.
        self.cycle_count = 0;
        self.step = 0;
        self.quarter_frame = false;
        self.half_frame = false;
    }

    /// Tick the frame counter by one CPU cycle.
    ///
    /// `current_cycle` is the absolute APU master cycle (incremented
    /// once per CPU cycle by `ApuCore`). Events are checked against
    /// `cycle_count` — the phase within the current period — which is
    /// reset by `$4017` writes. This fires each event exactly once per
    /// period (the original implementation compared against the
    /// absolute cycle and re-fired the IRQ every cycle once the master
    /// counter passed the period; Phase 5 wiring surfaced that).
    ///
    /// `irq_out` is set to true on IRQ events (4-step mode only).
    pub fn tick(&mut self, current_cycle: u64, irq_out: &mut bool) {
        // Reset transient flags.
        self.quarter_frame = false;
        self.half_frame = false;
        let period = if self.five_step { 18640u64 } else { 14914u64 };
        // Phase within the current period.
        let phase = self.cycle_count;
        self.cycle_count = self.cycle_count.wrapping_add(1);
        if self.cycle_count >= period {
            // Period boundary: final half-frame event + (4-step) IRQ.
            self.cycle_count = 0;
            *irq_out = !self.five_step && !self.irq_inhibit;
            self.half_frame = true;
            self.quarter_frame = false;
            let _ = current_cycle;
            return;
        }
        // Quarter-frame at 3728.5 / 11185.5, half-frame at 7456.5
        // (and 14914.5 in 5-step mode, where the period is longer).
        if phase == 3728 || phase == 11185 {
            self.quarter_frame = true;
        }
        if phase == 7456 || phase == 14914 {
            self.half_frame = true;
        }
        let _ = current_cycle;
    }

    /// Reset for power-on / reset.
    pub fn reset(&mut self) {
        self.cycle_count = 0;
        self.step = 0;
        self.quarter_frame = false;
        self.half_frame = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_counter_write_parses_modes() {
        let mut fc = FrameCounter::new();
        fc.write(0x80); // 5-step
        assert!(fc.five_step);
        assert!(!fc.irq_inhibit);
        fc.write(0x40); // 4-step + IRQ inhibit
        assert!(!fc.five_step);
        assert!(fc.irq_inhibit);
    }

    #[test]
    fn quarter_frame_at_3728() {
        let mut fc = FrameCounter::new();
        let mut irq = false;
        // Dense ticks: event fires on the call whose phase is 3728.
        for c in 0..=3728u64 {
            fc.tick(c, &mut irq);
        }
        assert!(fc.quarter_frame);
        assert!(!fc.half_frame);
        // No re-fire on subsequent cycles.
        fc.tick(3729, &mut irq);
        assert!(!fc.quarter_frame);
    }

    #[test]
    fn half_frame_at_7456() {
        let mut fc = FrameCounter::new();
        let mut irq = false;
        for c in 0..=7456u64 {
            fc.tick(c, &mut irq);
        }
        assert!(fc.half_frame);
        assert!(!irq);
    }

    #[test]
    fn irq_at_14914_4step() {
        let mut fc = FrameCounter::new();
        let mut irq = false;
        // The boundary fires on the call whose phase count reaches the
        // period (call #14914, 0-indexed phase 14913).
        for c in 0..=14913u64 {
            fc.tick(c, &mut irq);
        }
        assert!(fc.half_frame);
        assert!(irq);
        // Bug guard (Phase 5): the IRQ must NOT re-fire on every cycle
        // past the period boundary.
        irq = false;
        fc.tick(14914, &mut irq);
        assert!(!irq, "IRQ must fire once per period, not every cycle");
        fc.tick(14915, &mut irq);
        assert!(!irq);
    }

    #[test]
    fn no_irq_when_inhibit() {
        let mut fc = FrameCounter::new();
        fc.write(0x40); // IRQ inhibit
        let mut irq = false;
        for c in 0..=14914u64 {
            fc.tick(c, &mut irq);
        }
        assert!(!irq);
    }

    #[test]
    fn no_irq_5step() {
        let mut fc = FrameCounter::new();
        fc.write(0x80); // 5-step
        let mut irq = false;
        for c in 0..=14914u64 {
            fc.tick(c, &mut irq);
        }
        assert!(!irq);
    }
}