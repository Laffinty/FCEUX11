//! DMA module — OAM DMA + DMC DMA.
//!
//! Phase 4 deliverable. The DMA controllers manage the 513/514 cycle
//! OAM DMA (triggered by $4014 write) and DMC sample DMA (periodically
//! stealing cycles from the CPU).
//!
//! Reference: `src/apu.cpp` (FCEUX upstream DMA logic).
//!
//! Phase 4 ships a **skeleton** with the OAM DMA cycle count
//! ($4014 trigger → 513 or 514 CPU-cycle stall depending on alignment).
//! Full DMC DMA arbitration lands in Phase 6 shadow-run integration.

pub mod oam_dma;

/// DMA controller — owns OAM DMA state + DMC DMA state.
#[derive(Debug, Default, Clone)]
pub struct DmaCore {
    /// OAM DMA state.
    pub oam: oam_dma::OamDma,
    /// DMC DMA active flag.
    pub dmc_active: bool,
    /// DMC DMA pending stall (0 = no stall).
    pub dmc_stall_cycles: u8,
}

impl DmaCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all DMA state.
    pub fn reset(&mut self) {
        self.oam = oam_dma::OamDma::default();
        self.dmc_active = false;
        self.dmc_stall_cycles = 0;
    }

    /// Power-cycle reset.
    pub fn power_cycle(&mut self) {
        self.reset();
    }

    /// Total CPU-cycle stall count (used by scheduler to stretch CPU run).
    pub fn total_stall_cycles(&self) -> i32 {
        let oam = if self.oam.active { self.oam.stall_cycles as i32 } else { 0 };
        let dmc = self.dmc_stall_cycles as i32;
        oam + dmc
    }

    /// True if any DMA is currently stalling the CPU.
    pub fn is_stalling(&self) -> bool {
        self.oam.active || self.dmc_stall_cycles > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dma_default_state() {
        let d = DmaCore::new();
        assert!(!d.oam.active);
        assert_eq!(d.oam.remaining, 0);
        assert!(!d.dmc_active);
        assert_eq!(d.dmc_stall_cycles, 0);
        assert!(!d.is_stalling());
    }

    #[test]
    fn dma_reset_clears_state() {
        let mut d = DmaCore::new();
        d.dmc_active = true;
        d.dmc_stall_cycles = 3;
        d.reset();
        assert!(!d.dmc_active);
        assert_eq!(d.dmc_stall_cycles, 0);
    }

    #[test]
    fn total_stall_cycles_sums_oam_and_dmc() {
        let mut d = DmaCore::new();
        d.oam.active = true;
        d.oam.stall_cycles = 100;
        d.dmc_stall_cycles = 2;
        assert_eq!(d.total_stall_cycles(), 102);
    }
}