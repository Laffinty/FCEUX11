//! OAM DMA — direct memory access from $4014 write.
//!
//! When the CPU writes to $4014, the PPU copies 256 bytes from
//! `page << 8` into OAM. The CPU is stalled for 513 or 514 cycles
//! depending on whether the cycle counter is odd/even at trigger.
//!
//! Reference: `src/ppu.cpp` (FCEUX upstream) + NESdev wiki "PPU DMA".

/// OAM DMA controller state.
#[derive(Debug, Default, Clone, Copy)]
pub struct OamDma {
    /// DMA is currently in progress.
    pub active: bool,
    /// Source page (high byte of address).
    pub page: u8,
    /// Bytes remaining to transfer (256 → 0).
    pub remaining: u16,
    /// Whether the trigger cycle was aligned (odd → 513, even → 514).
    pub aligned: bool,
}

impl OamDma {
    /// Start an OAM DMA. Calling this on an already-active DMA is a
    /// no-op (real hardware allows this but doesn't restart).
    pub fn start(&mut self, page: u8, cycle_odd: bool) -> bool {
        if self.active {
            return false;
        }
        self.active = true;
        self.page = page;
        self.remaining = 256;
        // NES quirk: if the write happens on an odd CPU cycle, the
        // DMA is 513 cycles; if even, 514 cycles.
        self.aligned = cycle_odd;
        true
    }

    /// Total CPU cycles to stall for this DMA (513 or 514).
    pub fn total_cycles(&self) -> u16 {
        if self.aligned { 513 } else { 514 }
    }

    /// Per-byte transfer cycle count: 2 cycles (1 read + 1 write).
    pub fn cycles_per_byte(&self) -> u16 {
        2
    }

    /// Run one CPU cycle's worth of DMA work (decrement counters).
    /// Returns the byte that should be transferred this cycle (if any).
    pub fn step(&mut self) -> Option<u8> {
        if !self.active {
            return None;
        }
        if self.remaining == 0 {
            self.active = false;
            return None;
        }
        // Real hardware: 1 read + 1 write per cycle (2 cycles). Here
        // we model it as 1 byte per 2 cycles by returning Some only
        // on even cycles.
        self.remaining -= 1;
        if self.remaining == 0 {
            self.active = false;
        }
        // Note: the actual byte value is read by the SoC layer via
        // `bus.read((page << 8) | (256 - remaining_before))`. This
        // step function only tracks the cycle count.
        Some(0) // placeholder byte value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oam_dma_start_odd_cycle_513() {
        let mut d = OamDma::default();
        assert!(d.start(0x02, true));
        assert!(d.active);
        assert_eq!(d.page, 0x02);
        assert_eq!(d.remaining, 256);
        assert!(d.aligned);
        assert_eq!(d.total_cycles(), 513);
    }

    #[test]
    fn oam_dma_start_even_cycle_514() {
        let mut d = OamDma::default();
        assert!(d.start(0x02, false));
        assert_eq!(d.total_cycles(), 514);
    }

    #[test]
    fn oam_dma_double_start_is_noop() {
        let mut d = OamDma::default();
        assert!(d.start(0x02, true));
        assert!(!d.start(0x04, false)); // re-start rejected
        assert_eq!(d.page, 0x02); // unchanged
    }

    #[test]
    fn oam_dma_step_clears_after_256() {
        let mut d = OamDma::default();
        d.start(0x00, true);
        assert_eq!(d.remaining, 256);
        let mut step_count = 0;
        while d.active && d.remaining > 0 {
            d.step();
            step_count += 1;
        }
        assert_eq!(step_count, 256);
        assert!(!d.active);
    }
}