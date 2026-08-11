//! VBlank NMI dispatch.
//!
//! NES PPU triggers an NMI to the CPU at scanline 241 dot 1 (after VBlank
//! flag is set) IF PPUCTRL bit 7 is set.  The NMI fires once per
//! frame; the CPU services it before clearing the flag.
//!
//! Reference: `src/ppu.cpp::FCEUPPU_Loop` (VBlank / NMI block) +
//! `src/x6502.cpp::X6502_IRQBegin`.

/// VBlank NMI dispatch state.
#[derive(Debug, Default, Clone)]
pub struct NmiController {
    /// NMI has been armed (VBlank set + NMI enabled). CPU hasn't yet
    /// seen this NMI.
    pending_flag: bool,
    /// The CPU has been notified and should process this NMI.  After
    /// notifying, this clears so we don't double-fire.
    notified: bool,
    /// Scanline at which the NMI was armed (debug info).
    armed_scanline: i16,
    /// Dot at which the NMI was armed (debug info).
    armed_dot: u16,
}

impl NmiController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Arm the NMI (called from PpuCore when VBlank is set + NMI enabled).
    pub fn arm(&mut self, scanline: i16, dot: u16) {
        self.pending_flag = true;
        self.armed_scanline = scanline;
        self.armed_dot = dot;
    }

    /// Take the pending NMI — returns true exactly once per arming.
    pub fn take(&mut self) -> bool {
        if self.pending_flag {
            self.pending_flag = false;
            self.notified = true;
            true
        } else {
            false
        }
    }

    /// Peek at pending state (CPU polls before take).
    pub fn pending(&self) -> bool {
        self.pending_flag
    }

    /// Whether the CPU has been notified of this frame's NMI.
    pub fn notified(&self) -> bool {
        self.notified
    }

    /// Reset state (per-frame or on PPU reset).
    pub fn reset(&mut self) {
        self.pending_flag = false;
        self.notified = false;
        self.armed_scanline = 0;
        self.armed_dot = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmi_arm_then_take_fires_once() {
        let mut n = NmiController::new();
        assert!(!n.pending());
        n.arm(241, 1);
        assert!(n.pending());
        assert!(n.take());
        assert!(!n.pending());
        // Second take = no-op.
        assert!(!n.take());
    }

    #[test]
    fn nmi_records_arm_position() {
        let mut n = NmiController::new();
        n.arm(241, 1);
        assert_eq!(n.armed_scanline, 241);
        assert_eq!(n.armed_dot, 1);
    }

    #[test]
    fn nmi_reset_clears() {
        let mut n = NmiController::new();
        n.arm(241, 1);
        n.take();
        n.reset();
        assert!(!n.pending());
        assert!(!n.notified());
    }
}