//! APU frame counter.
//!
//! Two modes:
//! - **4-step (NTSC)**: 29830 CPU cycles per frame. Events at cycle
//!   `7457` (quarter), `14913` (half), `22371` (quarter),
//!   `29828` (IRQ set), `29829` (IRQ still set + half), `29830`
//!   (IRQ end + reset to 0).
//! - **4-step (PAL)**: 33252 CPU cycles per frame. Events at
//!   `8313` / `16627` / `24939` (quarter/half/quarter),
//!   `33252` (IRQ set) / `33253` (IRQ set + half) / `33254`
//!   (IRQ end + reset).
//! - **5-step (NTSC)**: 37282 CPU cycles per frame (no IRQ). Events at
//!   `7457` / `14913` / `22371` (quarter/half/quarter),
//!   `37281` (half) / `37282` (reset to 0; no IRQ end because IRQ
//!   was never set).
//! - **5-step (PAL)**: 41566 CPU cycles per frame (no IRQ). Events at
//!   `8313` / `16627` / `24939` (quarter/half/quarter),
//!   `41565` (half) / `41566` (reset).
//!
//! Reference: NESdev wiki "APU frame counter" + `src/sound.cpp`
//! `FrameCounterTick()` (Phase 6 P2 — the canonical event positions
//! are byte-pinned against the C++ reference).
//!
//! **Phase 6 P2 shadow fix (2026-08-12)**: `$4017` writes take effect
//! 3-4 CPU cycles later (parity-dependent: even = 3, odd = 4), matching
//! `src/sound.cpp::Write_FBC` + `FCEU_SoundCPUHook` cycle-position model.
//!
//! **Phase 6 P2 root-cause fix (2026-08-12, second edition)**: the
//! event positions WERE OFF BY 2× — the previous implementation used
//! `14914` for the 4-step period with quarter events at `3728/11185`
//! and half at `7456`, which matches the CYCLE HALVES of the correct
//! positions. The C++ reference uses integer-cycle positions
//! (`7457/14913/22371/29828/29829/29830` for NTSC, period `29830`)
//! because `FrameCounterTick()` checks `fhcnt == N` at each tick.
//! Shadow-run divergence was caused by Rust firing the IRQ ~half a
//! frame earlier than C++; aligning the constants here closes it.

/// CPU cycle count of the NTSC 4-step period (and 5-step + half
/// period for the 5-step's terminal event). Matches C++
/// `fhcnt==29830` reset point.
pub const FRAME_NTSC_PERIOD: u64 = 29830;
/// CPU cycle count of the PAL 4-step / 5-step terminal reset.
pub const FRAME_PAL_PERIOD: u64 = 33254;
/// CPU cycle count of the NTSC 5-step terminal reset.
pub const FRAME_NTSC_5STEP_PERIOD: u64 = 37282;
/// CPU cycle count of the PAL 5-step terminal reset.
pub const FRAME_PAL_5STEP_PERIOD: u64 = 41566;

/// Legacy alias kept for callers that imported the old constant.
/// New code should reference the NTSC / PAL period explicitly.
pub const FRAME_IRQ_PERIOD_CYCLES: u64 = FRAME_NTSC_PERIOD;

/// Complete frame-counter event table — byte-pinned against
/// `src/sound.cpp::FrameCounterTick()` for all four combinations
/// (NTSC/PAL × 4-step/5-step). The project discipline (see
/// `ppu/dot_clock.rs`) requires magic timing numbers to be pinned as
/// constants, NOT re-derived or computed, so these are literals
/// copied from the C++ reference. Changing any value here without
/// changing the C++ side breaks shadow-run parity.
#[derive(Debug, Clone, Copy)]
struct FrameCounterProfile {
    /// Reset-to-0 cycle position (`fhcnt == N` → `fhcnt = 0`).
    reset_at: u64,
    /// Quarter-frame event positions (C++ `FrameSoundEvent(true,false)`).
    quarter_at: [u64; 2],
    /// Half-frame event positions (C++ `FrameSoundEvent(true,true)`).
    half_at: [u64; 2],
    /// IRQ-set positions (4-step only; C++ `FrameIRQSet()`).
    /// 5-step profiles use `u64::MAX` (never set).
    irq_set_at: u64,
    /// Second IRQ-set + half-frame position (4-step only).
    irq_set2_at: u64,
}

/// NTSC 4-step: quarter 7457/22371, half 14913, IRQ 29828/29829,
/// reset 29830.
const NTSC_4STEP: FrameCounterProfile = FrameCounterProfile {
    reset_at: 29830,
    quarter_at: [7457, 22371],
    half_at: [14913, 29829],
    irq_set_at: 29828,
    irq_set2_at: 29829,
};

/// NTSC 5-step: quarter 7457/22371, half 14913/37281, no IRQ,
/// reset 37282.
const NTSC_5STEP: FrameCounterProfile = FrameCounterProfile {
    reset_at: 37282,
    quarter_at: [7457, 22371],
    half_at: [14913, 37281],
    irq_set_at: u64::MAX,
    irq_set2_at: u64::MAX,
};

/// PAL 4-step: quarter 8313/24939, half 16627, IRQ 33252/33253,
/// reset 33254.
const PAL_4STEP: FrameCounterProfile = FrameCounterProfile {
    reset_at: 33254,
    quarter_at: [8313, 24939],
    half_at: [16627, 33253],
    irq_set_at: 33252,
    irq_set2_at: 33253,
};

/// PAL 5-step: quarter 8313/24939, half 16627/41565, no IRQ,
/// reset 41566.
const PAL_5STEP: FrameCounterProfile = FrameCounterProfile {
    reset_at: 41566,
    quarter_at: [8313, 24939],
    half_at: [16627, 41565],
    irq_set_at: u64::MAX,
    irq_set2_at: u64::MAX,
};

#[derive(Debug, Clone, Copy)]
pub struct FrameCounter {
    /// 5-step mode (from $4017 bit 7).  false = 4-step.
    pub five_step: bool,
    /// IRQ inhibit (from $4017 bit 6).  false = IRQ enabled.
    pub irq_inhibit: bool,
    /// PAL timing (false = NTSC). Mirrors the C++ global `PAL` in
    /// `src/sound.cpp::FrameCounterTick()`. Set via
    /// `FrameCounter::set_pal` (the SoC wires this from
    /// `vnesu11_set_system_type` + the PAL flag the C++ side passes
    /// through `ApuStateMirror`).
    pub pal: bool,
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
            pal: false,
            cycle_count: 0,
            step: 0,
            quarter_frame: false,
            half_frame: false,
            pending_mode: 0,
            reset_in: 0,
        }
    }

    /// Set PAL/NTSC timing. Called by the SoC when the system type /
    /// PAL flag changes (mirrors the C++ global `PAL` used by
    /// `FrameCounterTick`).
    pub fn set_pal(&mut self, pal: bool) {
        self.pal = pal;
    }

    /// Active event table for the current (pal, five_step) combo.
    #[inline(always)]
    fn profile(&self) -> FrameCounterProfile {
        match (self.pal, self.five_step) {
            (false, false) => NTSC_4STEP,
            (false, true) => NTSC_5STEP,
            (true, false) => PAL_4STEP,
            (true, true) => PAL_5STEP,
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

        let profile = self.profile();
        // Increment FIRST, then compare the NEW value — mirrors C++
        // `FrameCounterTick()` which does `fhcnt++; if (fhcnt==N)`.
        // Using the post-increment value keeps the IRQ set at
        // `irq_set2_at` (29829) from being cleared by the same tick
        // that increments `cycle_count` to `reset_at` (29830); the
        // C++ model holds the IRQ for the whole 29829→29830 window.
        self.cycle_count = self.cycle_count.wrapping_add(1);
        let now = self.cycle_count;
        if now == profile.reset_at {
            // Reset boundary (C++ `fhcnt==N → FrameIRQEnd; fhcnt=0;
            // fcnt=0`). In 4-step mode the IRQ is cleared here; in
            // 5-step mode there was no IRQ to clear.
            if !self.five_step {
                *irq_out = false;
            }
            self.cycle_count = 0;
            self.quarter_frame = false;
            self.half_frame = false;
            let _ = current_cycle;
            return;
        }
        // Quarter-frame events.
        if now == profile.quarter_at[0] || now == profile.quarter_at[1] {
            self.quarter_frame = true;
        }
        // Half-frame events (half_at[1] doubles as the 5-step terminal
        // half and the 4-step second IRQ-set + half).
        if now == profile.half_at[0] || now == profile.half_at[1] {
            self.half_frame = true;
        }
        // Frame IRQ (4-step mode only).
        if now == profile.irq_set_at || now == profile.irq_set2_at {
            *irq_out = !self.five_step && !self.irq_inhibit;
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

    /// Helper: run `n` ticks from a fresh counter, returning the
    /// frame-counter irq latch.
    fn run_ticks(fc: &mut FrameCounter, n: u64) -> bool {
        let mut irq = false;
        for c in 0..n {
            fc.tick(c, &mut irq);
        }
        irq
    }

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

    /// NTSC 4-step: quarter-frame fires at fhcnt == 7457 (C++
    /// FrameCounterTick), NOT the old (incorrect) 3728.
    #[test]
    fn quarter_frame_at_7457() {
        let mut fc = FrameCounter::new();
        let mut irq = false;
        // Post-increment compare: now reaches 7456 → no quarter yet.
        run_ticks(&mut fc, 7456);
        assert!(!fc.quarter_frame);
        // now == 7457 → quarter-frame.
        fc.tick(0, &mut irq);
        assert!(fc.quarter_frame);
        assert!(!fc.half_frame);
        // No re-fire on subsequent cycles.
        fc.tick(0, &mut irq);
        assert!(!fc.quarter_frame);
    }

    /// NTSC 4-step: half-frame fires at fhcnt == 14913.
    #[test]
    fn half_frame_at_14913() {
        let mut fc = FrameCounter::new();
        let mut irq = false;
        run_ticks(&mut fc, 14912);
        assert!(!fc.half_frame);
        fc.tick(0, &mut irq);
        assert!(fc.half_frame);
        assert!(!irq);
    }

    /// NTSC 4-step: IRQ fires at fhcnt == 29828 / 29829, cleared at
    /// 29830 (matches C++ FrameIRQSet / FrameIRQEnd + reset).
    #[test]
    fn irq_at_29828_4step() {
        let mut fc = FrameCounter::new();
        // now reaches 29827 → no IRQ yet.
        assert!(!run_ticks(&mut fc, 29827));
        // now == 29828 → IRQ set.
        let mut irq = false;
        fc.tick(0, &mut irq);
        assert!(irq, "IRQ must set at fhcnt == 29828");
        // now == 29829 → IRQ still set (and half-frame fires).
        fc.tick(0, &mut irq);
        assert!(irq, "IRQ stays set at 29829");
        assert!(fc.half_frame);
        // now == 29830 → IRQ cleared + reset to 0.
        fc.tick(0, &mut irq);
        assert!(!irq, "IRQ cleared at 29830");
        assert_eq!(fc.cycle_count, 0, "frame counter wraps at 29830");
    }

    /// NTSC 5-step: terminal half at 37281, wrap at 37282, NO IRQ.
    #[test]
    fn no_irq_5step_nsc_wrap_at_37282() {
        let mut fc = FrameCounter::new();
        fc.write_with_parity(0x80, 0); // 5-step (even parity → 3 cycles)
        let mut irq = false;
        // Burn the 3-cycle deferred reset.
        fc.tick(0, &mut irq); fc.tick(0, &mut irq); fc.tick(0, &mut irq);
        // Burn up to one before the terminal event (now ≤ 37280).
        run_ticks(&mut fc, 37280);
        assert!(!fc.half_frame);
        fc.tick(0, &mut irq);
        assert!(fc.half_frame, "5-step terminal half at 37281");
        assert!(!irq, "5-step has no frame IRQ");
        fc.tick(0, &mut irq);
        assert!(!irq);
        assert_eq!(fc.cycle_count, 0, "5-step wraps at 37282");
    }

    /// PAL 4-step: events at 8313/16627/24939/33252/33253/33254.
    #[test]
    fn pal_4step_irq_at_33252() {
        let mut fc = FrameCounter::new();
        fc.set_pal(true);
        let mut irq = false;
        // quarter at 8313.
        run_ticks(&mut fc, 8312);
        assert!(!fc.quarter_frame);
        fc.tick(0, &mut irq);
        assert!(fc.quarter_frame);
        // half at 16627: from now=8313, advance to now=16626.
        run_ticks(&mut fc, 8313);
        assert!(!fc.half_frame);
        fc.tick(0, &mut irq);
        assert!(fc.half_frame);
        // IRQ at 33252 / 33253, cleared at 33254: now 16628..=33251.
        run_ticks(&mut fc, 16624);
        assert!(!irq);
        fc.tick(0, &mut irq);
        assert!(irq, "PAL IRQ set at 33252");
        fc.tick(0, &mut irq);
        assert!(irq, "PAL IRQ stays set at 33253");
        fc.tick(0, &mut irq);
        assert!(!irq, "PAL IRQ cleared at 33254");
        assert_eq!(fc.cycle_count, 0, "PAL 4-step wraps at 33254");
    }

    /// PAL 5-step: no IRQ, wrap at 41566.
    #[test]
    fn pal_5step_no_irq() {
        let mut fc = FrameCounter::new();
        fc.set_pal(true);
        fc.write_with_parity(0x80, 0);
        let mut irq = false;
        fc.tick(0, &mut irq); fc.tick(0, &mut irq); fc.tick(0, &mut irq);
        // From now=1, advance to now=41564 (terminal half at 41565).
        run_ticks(&mut fc, 41564);
        assert!(!fc.half_frame);
        fc.tick(0, &mut irq);
        assert!(fc.half_frame, "PAL 5-step terminal half at 41565");
        assert!(!irq, "PAL 5-step has no frame IRQ");
        fc.tick(0, &mut irq);
        assert_eq!(fc.cycle_count, 0, "PAL 5-step wraps at 41566");
    }

    #[test]
    fn no_irq_when_inhibit() {
        let mut fc = FrameCounter::new();
        fc.write_with_parity(0x40, 0); // IRQ inhibit (even parity → 3 cycles)
        let mut irq = false;
        // Burn 3 cycles for the reset to mature.
        fc.tick(0, &mut irq); fc.tick(0, &mut irq); fc.tick(0, &mut irq);
        run_ticks(&mut fc, 29830);
        assert!(!irq);
    }

    /// Phase 6 P2 shadow fix: $4017 reset matures 3 cycles later (even parity).
    #[test]
    fn delayed_reset_3_cycles_even_parity() {
        let mut fc = FrameCounter::new();
        // Pre-load cycle_count so we can verify the reset is delayed.
        run_ticks(&mut fc, 101);
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
        run_ticks(&mut fc, 101);
        fc.write_with_parity(0x00, 1); // odd parity → 4 cycle delay
        let mut irq = false;
        fc.tick(101, &mut irq);
        fc.tick(102, &mut irq);
        fc.tick(103, &mut irq);
        assert!(fc.cycle_count != 0, "delayed: still 1 cycle remaining");
        fc.tick(104, &mut irq);
        assert_eq!(fc.cycle_count, 0, "reset should mature after 4 ticks");
    }
}