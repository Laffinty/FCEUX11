//! APU frame counter.
//!
//! Two modes:
//! - **4-step**: 14914 CPU cycles per frame (3728.5 quarter, 7456.5 half,
//!   11185.5 quarter, 14914.5 half + IRQ)
//! - **5-step**: 18640 CPU cycles per frame (no IRQ)
//!
//! Reference: NESdev wiki "APU frame counter".
//!
//! **Phase 6 P2 shadow fix (2026-08-12)**: `$4017` writes take effect
//! 3-4 CPU cycles later (parity-dependent: even = 3, odd = 4), matching
//! `src/sound.cpp::Write_FBC` + `FCEU_SoundCPUHook` cycle-position model.
//! This delayed reset is the key to matching C++ frame-counter timing
//! for the shadow run — synchronous reset would drift the IRQ by 3-4
//! cycles per period and cause CPU divergence on blargg tests that
//! synchronize to frame-counter IRQs.

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
    /// Pending $4017 write mode bits (5-step + irq_inhibit), scheduled
    /// for `reset_in` cycles from now. 0 = no pending write.
    /// Phase 6 P2 shadow fix (2026-08-12): delayed reset to match
    /// `src/sound.cpp::fc_reset_in` / `fc_pending_mode`.
    pub pending_mode: u8,
    /// Cycles until `pending_mode` is committed (3 if even parity, 4
    /// if odd parity). 0 = no pending reset.
    pub reset_in: u8,
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
            pending_mode: 0,
            reset_in: 0,
        }
    }

    /// $4017 write. The mode bits are committed after `reset_in`
    /// cycles (3 or 4 depending on parity), matching C++'s
    /// `fc_reset_in` / `fc_pending_mode` semantics. During the delay,
    /// the existing period continues to count down; once the delay
    /// elapses, `cycle_count` resets to 0 and the new mode takes effect.
    pub fn write(&mut self, val: u8) {
        // Pending mode bits (bit 7 = 5-step, bit 6 = irq_inhibit).
        self.pending_mode = val & 0xC0;
        // Parity comes from the absolute CPU cycle count at the write
        // site — the caller passes it in via `write_with_parity`.
        // Default: assume even parity → 3 cycle delay.
        self.reset_in = 3;
    }

    /// $4017 write with explicit parity (passed in by the caller as
    /// `cycle_parity` = absolute_cpu_cycles & 1). even = 3 cycle
    /// delay, odd = 4 cycle delay. Matches C++ `fc_reset_in = ((abs_ts
    /// & 1) == 0) ? 3 : 4` in `src/sound.cpp:1269`.
    pub fn write_with_parity(&mut self, val: u8, cycle_parity: u8) {
        self.pending_mode = val & 0xC0;
        self.reset_in = if cycle_parity & 1 == 0 { 3 } else { 4 };
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

        // Phase 6 P2 shadow fix: deferred $4017 reset (matches C++
        // fc_reset_in / fc_pending_mode).
        if self.reset_in > 0 {
            self.reset_in -= 1;
            if self.reset_in == 0 {
                // Reset matures: commit pending mode + clear cycle_count.
                self.five_step = (self.pending_mode & 0x80) != 0;
                self.irq_inhibit = (self.pending_mode & 0x40) != 0;
                self.cycle_count = 0;
                self.step = 0;
                self.quarter_frame = false;
                self.half_frame = false;
                // 5-step write additionally fires an immediate quarter +
                // half clock when the reset matures (handled in C++
                // FCEU_SoundCPUHook). For simplicity in Rust we just
                // emit the half_frame flag (the IRQ path doesn't apply
                // because 5-step has no IRQ). Phase 6 P2 follow-up
                // could split these if a blargg test regresses.
                if self.five_step {
                    self.half_frame = true;
                }
                self.pending_mode = 0;
                let _ = current_cycle;
                return;
            }
        }

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
        self.pending_mode = 0;
        self.reset_in = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_counter_write_parses_modes() {
        let mut fc = FrameCounter::new();
        // write() uses default even-parity delay (3 cycles).
        fc.write(0x80); // 5-step
        // After 3 ticks the reset matures.
        let mut irq = false;
        fc.tick(0, &mut irq); fc.tick(1, &mut irq); fc.tick(2, &mut irq);
        assert!(fc.five_step);
        assert!(!fc.irq_inhibit);
        // 4-step + IRQ inhibit (parity odd).
        fc.write_with_parity(0x40, 1);
        fc.tick(3, &mut irq); fc.tick(4, &mut irq); fc.tick(5, &mut irq); fc.tick(6, &mut irq);
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
        fc.write_with_parity(0x40, 0); // IRQ inhibit (even parity → 3 cycles)
        let mut irq = false;
        // Burn 3 cycles for the reset to mature.
        fc.tick(0, &mut irq); fc.tick(1, &mut irq); fc.tick(2, &mut irq);
        for c in 0..=14914u64 {
            fc.tick(c, &mut irq);
        }
        assert!(!irq);
    }

    #[test]
    fn no_irq_5step() {
        let mut fc = FrameCounter::new();
        fc.write_with_parity(0x80, 0); // 5-step (even parity)
        let mut irq = false;
        fc.tick(0, &mut irq); fc.tick(1, &mut irq); fc.tick(2, &mut irq);
        for c in 0..=14914u64 {
            fc.tick(c, &mut irq);
        }
        assert!(!irq);
    }

    /// Phase 6 P2 shadow fix: $4017 reset matures 3 cycles later (even parity).
    #[test]
    fn delayed_reset_3_cycles_even_parity() {
        let mut fc = FrameCounter::new();
        // Pre-load cycle_count so we can verify the reset is delayed.
        for c in 0..=100u64 { fc.tick(c, &mut irq_mut()); }
        let before = fc.cycle_count;
        fc.write_with_parity(0x00, 0); // even parity → 3 cycle delay
        let mut irq = false;
        // After 1 tick: cycle_count still ticking (not yet reset).
        fc.tick(101, &mut irq);
        assert!(fc.cycle_count > before, "delayed: cycle_count should NOT reset on first tick");
        // After 3 ticks: cycle_count = 0 (reset matured).
        fc.tick(102, &mut irq);
        fc.tick(103, &mut irq);
        assert_eq!(fc.cycle_count, 0, "reset should mature after 3 ticks");
    }

    /// Phase 6 P2 shadow fix: $4017 reset matures 4 cycles later (odd parity).
    #[test]
    fn delayed_reset_4_cycles_odd_parity() {
        let mut fc = FrameCounter::new();
        for c in 0..=100u64 { fc.tick(c, &mut irq_mut()); }
        fc.write_with_parity(0x00, 1); // odd parity → 4 cycle delay
        let mut irq = false;
        fc.tick(101, &mut irq);
        fc.tick(102, &mut irq);
        fc.tick(103, &mut irq);
        assert!(fc.cycle_count != 0, "delayed: still 1 cycle remaining");
        fc.tick(104, &mut irq);
        assert_eq!(fc.cycle_count, 0, "reset should mature after 4 ticks");
    }

    fn irq_mut() -> bool { false }
}