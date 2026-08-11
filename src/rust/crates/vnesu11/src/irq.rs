//! IRQ controller — NMI (edge) + IRQ (level) + external sources.
//!
//! Phase 4 deliverable.  Aggregates IRQ sources from:
//! - PPU VBlank NMI (edge-sensitive)
//! - APU frame counter IRQ (level)
//! - APU DMC IRQ (level)
//! - Mapper IRQ (level)
//! - External IRQ sources EXT / EXT2 (FDS disk, level) — `vnesu11_set_external_irq`
//!
//! Reference: `src/x6502.cpp::X6502_IRQBegin` + `src/fceu.cpp::FCEU_IQEXT`.
//!
//! Phase 4 ships a **skeleton** that wires the CPU IRQ source bitmask
//! (`CpuCore::IRQ_NMI`, `IRQ_FCOUNT`, `IRQ_DMC`, `IRQ_EXT`, `IRQ_EXT2`).

/// External IRQ source index (FDS uses index 0 = EXT, 1 = EXT2).
pub const EXT_IRQ_SOURCE_INDEX: [u32; 2] = [0, 1];

/// IRQ controller state.
#[derive(Debug, Default, Clone)]
pub struct IrqController {
    /// NMI edge pending (set by PPU VBlank, cleared after CPU takes).
    pub nmi_edge: bool,
    /// NMI has been notified to CPU (consumed flag).
    pub nmi_notified: bool,
    /// IRQ sources OR'd bitmask.
    pub irq_sources: u32,
    /// External IRQ sources (2 sources: EXT, EXT2 for FDS).
    pub external: [bool; 2],
}

impl IrqController {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all IRQ state.
    pub fn reset(&mut self) {
        self.nmi_edge = false;
        self.nmi_notified = false;
        self.irq_sources = 0;
        self.external = [false; 2];
    }

    /// Assert NMI (edge-sensitive). Called by PPU VBlank.
    #[inline]
    pub fn assert_nmi(&mut self) {
        self.nmi_edge = true;
    }

    /// Clear NMI edge (called after CPU services the NMI).
    #[inline]
    pub fn clear_nmi(&mut self) {
        self.nmi_edge = false;
        self.nmi_notified = false;
    }

    /// Take the NMI: returns true if NMI is pending, then clears it.
    pub fn take_nmi(&mut self) -> bool {
        if self.nmi_edge && !self.nmi_notified {
            self.nmi_notified = true;
            true
        } else {
            false
        }
    }

    /// Assert an IRQ source (mapper, APU frame counter, etc.).
    #[inline]
    pub fn assert_irq(&mut self, source: u32) {
        self.irq_sources |= source;
    }

    /// De-assert an IRQ source.
    #[inline]
    pub fn deassert_irq(&mut self, source: u32) {
        self.irq_sources &= !source;
    }

    /// Poll (read-and-clear) — returns true if any IRQ source is active.
    pub fn poll(&mut self) -> bool {
        self.irq_sources != 0
            || self.external[0]
            || self.external[1]
    }

    /// Set external IRQ source (FDS uses this via `vnesu11_set_external_irq`).
    #[inline]
    pub fn set_external(&mut self, source: usize, on: bool) {
        if source < 2 {
            self.external[source] = on;
        }
    }

    /// Aggregate IRQ source bitmask (for CPU `irq_begin`).
    pub fn aggregate_mask(&self) -> u32 {
        let mut m = self.irq_sources;
        if self.external[0] {
            m |= 0x001; // IRQ_EXT
        }
        if self.external[1] {
            m |= 0x002; // IRQ_EXT2
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nmi_edge_arms_and_takes() {
        let mut i = IrqController::new();
        assert!(!i.take_nmi());
        i.assert_nmi();
        assert!(i.take_nmi());
        // Second take = no-op (already notified).
        assert!(!i.take_nmi());
    }

    #[test]
    fn irq_sources_or() {
        let mut i = IrqController::new();
        i.assert_irq(0x200); // FCOUNT
        i.assert_irq(0x100); // DMC
        assert!(i.poll());
        assert_eq!(i.aggregate_mask(), 0x300);
    }

    #[test]
    fn external_sources_aggregate() {
        let mut i = IrqController::new();
        i.set_external(0, true);
        i.set_external(1, false);
        assert!(i.poll());
        assert_eq!(i.aggregate_mask() & 0x001, 0x001);
        assert_eq!(i.aggregate_mask() & 0x002, 0);
    }

    #[test]
    fn reset_clears_all() {
        let mut i = IrqController::new();
        i.assert_nmi();
        i.assert_irq(0x200);
        i.set_external(0, true);
        i.reset();
        assert!(!i.nmi_edge);
        assert_eq!(i.irq_sources, 0);
        assert!(!i.external[0]);
    }
}