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
    /// CPU stall cycles remaining (513 or 514 → 0). Phase 5: the
    /// `$4014` write stalls the CPU for the full DMA duration; each
    /// byte transfer consumes 2 cycles and the trailing alignment
    /// consumes the residual.
    pub stall_cycles: u16,
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
        self.stall_cycles = if cycle_odd { 513 } else { 514 };
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

    /// Advance one step of the DMA transfer.
    ///
    /// While bytes remain, each call consumes **one byte** (the SoC
    /// layer performs the actual bus read + OAM write and feeds the
    /// value back; the `Some(0)` placeholder is kept for the pure
    /// cycle-tracking API used by unit tests). After the last byte,
    /// residual alignment cycles (513/514 − 512) are drained one at a
    /// time before the DMA deactivates.
    ///
    /// Returns `Some(_)` while a byte slot is being transferred,
    /// `None` during the trailing alignment drain or when idle.
    pub fn step(&mut self) -> Option<u8> {
        if !self.active {
            return None;
        }
        if self.remaining > 0 {
            // One byte transfer consumes 2 CPU cycles (1 read + 1 write).
            self.remaining -= 1;
            self.stall_cycles = self.stall_cycles.saturating_sub(2);
            if self.remaining == 0 && self.stall_cycles == 0 {
                self.active = false;
            }
            Some(0) // placeholder byte value
        } else {
            // Drain residual alignment cycles (1 for odd, 2 for even
            // trigger). No data moves during these.
            self.stall_cycles = self.stall_cycles.saturating_sub(1);
            if self.stall_cycles == 0 {
                self.active = false;
            }
            None
        }
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
        assert_eq!(d.stall_cycles, 513);
    }

    #[test]
    fn oam_dma_start_even_cycle_514() {
        let mut d = OamDma::default();
        assert!(d.start(0x02, false));
        assert_eq!(d.total_cycles(), 514);
        assert_eq!(d.stall_cycles, 514);
    }

    #[test]
    fn oam_dma_double_start_is_noop() {
        let mut d = OamDma::default();
        assert!(d.start(0x02, true));
        assert!(!d.start(0x04, false)); // re-start rejected
        assert_eq!(d.page, 0x02); // unchanged
    }

    #[test]
    fn oam_dma_step_transfers_256_bytes_then_drains() {
        let mut d = OamDma::default();
        d.start(0x00, true);
        assert_eq!(d.remaining, 256);
        let mut transfers = 0;
        let mut total_steps = 0;
        while d.active {
            if d.step().is_some() {
                transfers += 1;
            }
            total_steps += 1;
            assert!(total_steps <= 300, "DMA never deactivated");
        }
        assert_eq!(transfers, 256, "all 256 bytes transferred");
        // Odd trigger: 513 cycles = 256 bytes × 2 + 1 alignment.
        assert_eq!(total_steps, 257);
        assert!(!d.active);
        assert_eq!(d.remaining, 0);
        assert_eq!(d.stall_cycles, 0);
    }

    #[test]
    fn oam_dma_even_trigger_drains_two_alignment_cycles() {
        let mut d = OamDma::default();
        d.start(0x00, false); // even → 514 cycles
        let mut total_steps = 0;
        while d.active {
            d.step();
            total_steps += 1;
            assert!(total_steps <= 300);
        }
        // 256 bytes × 2 + 2 alignment = 514.
        assert_eq!(total_steps, 258);
    }
}