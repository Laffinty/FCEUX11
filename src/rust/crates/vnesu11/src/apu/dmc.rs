//! DMC (Delta Modulation Channel) — sample playback.
//!
//! The DMC reads 1-bit PCM data from PRG-ROM (via DMA) at a
//! configurable rate.  Each bit is fed through a delta-sigma
//! DAC.  An optional IRQ fires when the sample is finished.
//!
//! Reference: NESdev wiki "APU DMC".

/// DMC rate table (NTSC).  Index by $4010 bits 0-3.
const DMC_RATE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214,
    190, 160, 142, 128, 106, 84, 72, 54,
];

#[derive(Debug, Clone)]
pub struct DmcChannel {
    /// IRQ enabled (from $4010 bit 7).
    pub irq_enabled: bool,
    /// Loop sample (from $4010 bit 6).
    pub loop_sample: bool,
    /// Rate index (0..=15).
    pub rate_index: u8,
    /// Sample address ($4011-$4012: high 6 bits of 16-bit address).
    pub sample_address: u16,
    /// Sample length in bytes ($4013).
    pub sample_length: u16,
    /// DMA active (currently fetching a byte).
    pub dma_active: bool,
    /// DMA buffer (last fetched byte).
    pub dma_buffer: u8,
    /// Output level (7-bit, 0..=127).
    pub output_level: u8,
    /// Sample shift register.
    pub shift_register: u8,
    /// Bits remaining in shift register.
    pub bits_remaining: u8,
    /// Timer counter.
    pub timer_counter: u16,
    /// Bytes remaining in current sample.
    pub bytes_remaining: u16,
    /// Current sample address (mutable copy).
    pub current_address: u16,
    /// IRQ pending (set when sample completes if `irq_enabled`).
    pub irq_pending: bool,
    /// Output silence flag (set when sample_buffer empty).
    pub sample_buffer_empty: bool,
}

impl Default for DmcChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DmcChannel {
    pub fn new() -> Self {
        Self {
            irq_enabled: false,
            loop_sample: false,
            rate_index: 0,
            sample_address: 0,
            sample_length: 0,
            dma_active: false,
            dma_buffer: 0,
            output_level: 0,
            shift_register: 0,
            bits_remaining: 0,
            timer_counter: 0,
            bytes_remaining: 0,
            current_address: 0,
            irq_pending: false,
            sample_buffer_empty: true,
        }
    }

    /// $4010 write — IRQ / loop / rate.
    pub fn write_control(&mut self, val: u8) {
        self.irq_enabled = (val & 0x80) != 0;
        self.loop_sample = (val & 0x40) != 0;
        self.rate_index = val & 0x0F;
    }

    /// $4011 write — direct load (7-bit DAC).
    pub fn write_load(&mut self, val: u8) {
        self.output_level = val & 0x7F;
    }

    /// $4012 write — sample address high.
    pub fn write_address(&mut self, val: u8) {
        self.sample_address = 0xC000 | ((val as u16) << 6);
    }

    /// $4013 write — sample length.
    pub fn write_length(&mut self, val: u8) {
        self.sample_length = (val as u16) << 4 | 1;
        // Writing length restarts the sample.
        if self.bytes_remaining == 0 {
            self.current_address = self.sample_address;
            self.bytes_remaining = self.sample_length;
        }
    }

    /// One CPU cycle tick.
    pub fn tick(&mut self) {
        if !self.sample_buffer_empty {
            // Clock the shift register.
            if self.timer_counter == 0 {
                self.timer_counter = DMC_RATE[self.rate_index as usize];
                // Output the next bit.
                if (self.shift_register & 1) == 0 {
                    if self.output_level > 1 {
                        self.output_level -= 2;
                    }
                } else if self.output_level < 126 {
                    self.output_level += 2;
                }
                self.shift_register >>= 1;
                self.bits_remaining -= 1;
                if self.bits_remaining == 0 {
                    self.bits_remaining = 8;
                    self.sample_buffer_empty = true;
                    self.dma_active = true;
                }
            } else {
                self.timer_counter -= 1;
            }
        }
    }

    /// Output level (0..=127).  Note: range differs from pulse/noise
    /// (which are 0..=15) — mixer handles the difference.
    pub fn output(&self) -> u8 {
        self.output_level
    }

    /// Take any pending IRQ (clears the bit).
    pub fn take_irq(&mut self) -> bool {
        let v = self.irq_pending;
        self.irq_pending = false;
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dmc_rate_table_known() {
        assert_eq!(DMC_RATE[0], 428);
        assert_eq!(DMC_RATE[15], 54);
    }

    #[test]
    fn dmc_write_address_sets_c000_base() {
        let mut d = DmcChannel::new();
        d.write_address(0x40);
        assert_eq!(d.sample_address, 0xD000);
    }

    #[test]
    fn dmc_write_load_clears_high_bit() {
        let mut d = DmcChannel::new();
        d.write_load(0xFF);
        assert_eq!(d.output_level, 0x7F);
    }

    #[test]
    fn dmc_take_irq_clears() {
        let mut d = DmcChannel::new();
        d.irq_pending = true;
        assert!(d.take_irq());
        assert!(!d.take_irq());
    }

    #[test]
    fn dmc_write_length_restarts_sample() {
        let mut d = DmcChannel::new();
        d.bytes_remaining = 0;
        d.write_address(0x40);
        d.write_length(0x10);
        assert_eq!(d.sample_length, 0x101);
        assert_eq!(d.current_address, 0xD000);
        assert_eq!(d.bytes_remaining, 0x101);
    }
}