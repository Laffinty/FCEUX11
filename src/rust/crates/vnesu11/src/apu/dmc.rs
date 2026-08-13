//! DMC (Delta Modulation Channel) — sample playback.
//!
//! The DMC reads 1-bit PCM data from PRG-ROM (via DMA) at a
//! configurable rate. Each bit is fed through a delta-sigma DAC; the
//! 7-bit output level moves +/-2 toward the decoded bit and clamps at
//! the rail by restoring the previous level. An optional IRQ fires when
//! a non-looping sample finishes.
//!
//! The model mirrors `src/sound.cpp` (FCEUX upstream):
//! - `acc`/`bit_count` are free-running (they advance even when no
//!   sample is loaded), exactly like C++ `DMCacc`/`DMCBitCount`.
//! - The DMA byte fetch (`complete_dma`) is driven by the SoC at each
//!   CPU-instruction boundary, mirroring C++ `DMCDMA()` in
//!   `FCEU_SoundCPUHook`.
//!
//! Reference: NESdev wiki "APU DMC".

/// DMC rate table (NTSC). Index by $4010 bits 0-3.
const DMC_RATE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214,
    190, 160, 142, 128, 106, 84, 72, 54,
];

/// Outcome of a DMC DMA byte fetch (`complete_dma`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DmcDmaOutcome {
    /// Sample still running.
    Running,
    /// Non-looping sample finished with IRQ enabled → assert DMC IRQ.
    IrqRequested,
    /// Non-looping sample finished with IRQ disabled.
    Done,
}

#[derive(Debug, Clone)]
pub struct DmcChannel {
    /// IRQ enabled (from $4010 bit 7).
    pub irq_enabled: bool,
    /// Loop sample (from $4010 bit 6).
    pub loop_sample: bool,
    /// Rate index (0..=15).
    pub rate_index: u8,
    /// Sample address ($4011-$4012: high 6 bits of 16-bit address).
    /// Stored in full CPU-space form ($C000 | (val << 6)); the DMA
    /// offset into $8000-$FFFF is `sample_address & 0x7FFF`.
    pub sample_address: u16,
    /// Sample length in bytes ($4013 → `(val << 4) + 1`).
    pub sample_length: u16,
    /// Output level (7-bit, 0..=127). C++ `RawDALatch`.
    pub output_level: u8,
    /// Current sample address offset into $8000-$FFFF (0..=0x7FFF).
    /// C++ `DMCAddress`.
    pub current_address: u16,
    /// Bytes remaining in the current sample. C++ `DMCSize`; the value
    /// reported as $4015 bit 4 while non-zero.
    pub bytes_remaining: u16,
    /// Output-unit accumulator (free-running). C++ `DMCacc`.
    pub acc: i32,
    /// 3-bit bit counter (free-running). C++ `DMCBitCount`; when it
    /// wraps to 0 the shift register is empty and the DMA buffer (if
    /// any) is moved into the shift register.
    pub bit_count: u8,
    /// Shift register of 1-bit deltas. C++ `DMCShift`.
    pub shift_register: u8,
    /// True when the shift register holds at least one bit to output.
    /// C++ `DMCHaveSample`.
    pub have_sample: bool,
    /// Fetched DMA byte waiting to be moved into the shift register.
    pub dma_buffer: u8,
    /// True when a fetched DMA byte is pending. C++ `DMCHaveDMA`.
    pub have_dma: bool,
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
            // C++ reset: DMCAddressLatch=0 → sample base $C000; the
            // default sample length is 1 byte (`(0 << 4) + 1`).
            sample_address: 0xC000,
            sample_length: 1,
            output_level: 0,
            current_address: 0x4000,
            bytes_remaining: 0,
            acc: 1,
            bit_count: 0,
            shift_register: 0,
            have_sample: false,
            dma_buffer: 0,
            have_dma: false,
        }
    }

    /// $4010 write — IRQ / loop / rate. IRQ-flag follow behaviour
    /// (C++ `Write_DMCRegs` case 0x00) is handled by the caller in
    /// `bus.rs`, because it needs access to the CPU IRQ line.
    pub fn write_control(&mut self, val: u8) {
        self.irq_enabled = (val & 0x80) != 0;
        self.loop_sample = (val & 0x40) != 0;
        self.rate_index = val & 0x0F;
    }

    /// $4011 write — direct load (7-bit DAC).
    pub fn write_load(&mut self, val: u8) {
        self.output_level = val & 0x7F;
    }

    /// $4012 write — sample address high 6 bits. Only latches; the
    /// running sample is not restarted (C++ `Write_DMCRegs` case 0x02).
    pub fn write_address(&mut self, val: u8) {
        self.sample_address = 0xC000 | ((val as u16) << 6);
    }

    /// $4013 write — sample length. Only latches; the running sample is
    /// not restarted (C++ `Write_DMCRegs` case 0x03).
    pub fn write_length(&mut self, val: u8) {
        self.sample_length = ((val as u16) << 4) | 1;
    }

    /// Prepare a sample for playback: reset the DMA offset and byte
    /// count from the latched address/length. C++ `PrepDPCM()`.
    pub fn prep_dpcm(&mut self) {
        self.current_address = self.sample_address & 0x7FFF;
        self.bytes_remaining = self.sample_length;
    }

    /// Start playback if currently stopped. Called by a $4015 write with
    /// bit 4 set when no sample is running (C++ `StatusWrite`).
    pub fn enable_if_stopped(&mut self) {
        if self.bytes_remaining == 0 {
            self.prep_dpcm();
        }
    }

    /// Stop the running sample ($4015 bit 4 clear). The DMA buffer and
    /// shift-register state are intentionally left intact so a retained
    /// byte survives a stop/start cycle (blargg `dmc_buffer_retained`).
    pub fn stop_sample(&mut self) {
        self.bytes_remaining = 0;
    }

    /// True when the SoC should fetch the next DMA byte (C++ `DMCDMA()`
    /// guard: `DMCSize && !DMCHaveDMA`).
    pub fn needs_dma(&self) -> bool {
        self.bytes_remaining > 0 && !self.have_dma
    }

    /// Consume one fetched DMA byte and advance the sample pointer.
    /// Returns the IRQ/status outcome for the SoC to route.
    pub fn complete_dma(&mut self, byte: u8) -> DmcDmaOutcome {
        debug_assert!(self.bytes_remaining > 0);
        self.dma_buffer = byte;
        self.have_dma = true;
        self.current_address = (self.current_address + 1) & 0x7FFF;
        self.bytes_remaining -= 1;
        if self.bytes_remaining == 0 {
            if self.loop_sample {
                self.prep_dpcm();
                DmcDmaOutcome::Running
            } else if self.irq_enabled {
                DmcDmaOutcome::IrqRequested
            } else {
                DmcDmaOutcome::Done
            }
        } else {
            DmcDmaOutcome::Running
        }
    }

    /// Advance the output unit one CPU cycle.
    ///
    /// `acc` is free-running: it decrements every cycle even when no
    /// sample is loaded, and every wrap advances `bit_count` and shifts
    /// the (possibly empty) shift register. This matches C++
    /// `FCEU_SoundCPUHook`'s `DMCacc` loop and preserves the phase of a
    /// sample started mid-stream.
    pub fn tick(&mut self) {
        self.acc -= 1;
        if self.acc <= 0 {
            if self.have_sample {
                self.output_bit();
            }
            self.acc += DMC_RATE[self.rate_index as usize] as i32;
            self.bit_count = (self.bit_count + 1) & 7;
            self.shift_register >>= 1;
            // C++ `tester()`: when the bit counter wraps to 0 the shift
            // register is empty — move the fetched DMA byte in, or drop
            // the sample.
            if self.bit_count == 0 {
                if !self.have_dma {
                    self.have_sample = false;
                } else {
                    self.have_sample = true;
                    self.shift_register = self.dma_buffer;
                    self.have_dma = false;
                }
            }
        }
    }

    /// Decode one bit from the shift register into the 7-bit output
    /// level. C++: `RawDALatch += ((shift & 1) << 2) - 2`, then restore
    /// the previous level if the result overflowed the 7-bit range.
    #[inline]
    fn output_bit(&mut self) {
        let delta: i16 = if (self.shift_register & 1) != 0 { 2 } else { -2 };
        let prev = self.output_level;
        let new = prev as i16 + delta;
        if new >= 0 && new <= 127 {
            self.output_level = new as u8;
        }
        // else: clamp by keeping the previous level (C++ restores `bah`).
    }

    /// Output level (0..=127). Note: range differs from pulse/noise
    /// (which are 0..=15) — mixer handles the difference.
    pub fn output(&self) -> u8 {
        self.output_level
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
    fn dmc_write_length_latches_only() {
        let mut d = DmcChannel::new();
        d.enable_if_stopped();
        assert_eq!(d.bytes_remaining, 1);
        d.bytes_remaining = 5;
        d.write_length(0x10);
        // Latching length must not restart a running sample.
        assert_eq!(d.bytes_remaining, 5);
        assert_eq!(d.sample_length, 0x101);
    }

    #[test]
    fn dmc_prep_sets_offset_and_size() {
        let mut d = DmcChannel::new();
        d.write_address(0x40); // $D000
        d.write_length(0x10);  // 0x101 bytes
        d.prep_dpcm();
        assert_eq!(d.current_address, 0x5000); // $D000 & 0x7FFF
        assert_eq!(d.bytes_remaining, 0x101);
    }

    #[test]
    fn dmc_complete_dma_advances_and_wraps() {
        let mut d = DmcChannel::new();
        d.current_address = 0x7FFF;
        d.bytes_remaining = 2;
        assert!(d.needs_dma());
        assert_eq!(d.complete_dma(0xAA), DmcDmaOutcome::Running);
        assert_eq!(d.current_address, 0x0000); // wraps within $8000-$FFFF
        assert_eq!(d.bytes_remaining, 1);
        assert!(d.have_dma);
        assert_eq!(d.dma_buffer, 0xAA);
    }

    #[test]
    fn dmc_complete_dma_irq_on_finish() {
        let mut d = DmcChannel::new();
        d.irq_enabled = true;
        d.loop_sample = false;
        d.bytes_remaining = 1;
        assert_eq!(d.complete_dma(0x55), DmcDmaOutcome::IrqRequested);
        assert_eq!(d.bytes_remaining, 0);
    }

    #[test]
    fn dmc_complete_dma_loop_restarts() {
        let mut d = DmcChannel::new();
        d.loop_sample = true;
        d.write_address(0x20); // $C800
        d.write_length(0x02);  // 0x21 bytes
        d.prep_dpcm();
        assert_eq!(d.bytes_remaining, 0x21);
        d.bytes_remaining = 1;
        assert_eq!(d.complete_dma(0x55), DmcDmaOutcome::Running);
        // Loop reload: offset back to sample base, size back to length.
        assert_eq!(d.current_address, 0x4800);
        assert_eq!(d.bytes_remaining, 0x21);
    }

    #[test]
    fn dmc_output_bit_moves_toward_level() {
        let mut d = DmcChannel::new();
        d.output_level = 100;
        d.shift_register = 0b0000_0001; // bit 0 = 1 → +2
        d.output_bit();
        assert_eq!(d.output_level, 102);
        d.shift_register = 0b0000_0000; // bit 0 = 0 → -2
        d.output_bit();
        assert_eq!(d.output_level, 100);
    }

    #[test]
    fn dmc_output_bit_clamps_at_rails() {
        let mut d = DmcChannel::new();
        d.output_level = 127;
        d.shift_register = 1; // +2 would overflow → restore
        d.output_bit();
        assert_eq!(d.output_level, 127);

        d.output_level = 0;
        d.shift_register = 0; // -2 would underflow → restore
        d.output_bit();
        assert_eq!(d.output_level, 0);
    }

    #[test]
    fn dmc_tick_shifts_bits_and_reloads_buffer() {
        let mut d = DmcChannel::new();
        d.rate_index = 15; // period 54
        d.acc = 1;         // first tick wraps
        // Load a sample byte into the shift register by fetching a DMA
        // byte, then simulate the free-running shift counter.
        d.bytes_remaining = 2;
        assert_eq!(d.complete_dma(0b1010_0101), DmcDmaOutcome::Running);
        // Set the bit counter so the next tick wraps to 0 and moves the
        // fetched buffer into the shift register (C++ `tester()`).
        d.bit_count = 7;
        d.tick();
        assert!(d.have_sample);
        assert!(!d.have_dma);
        assert_eq!(d.shift_register, 0b1010_0101);
        // The first output bit is produced on the NEXT wrap (after one
        // period), so immediately after the load no output change yet.
        assert_eq!(d.output_level, 0);
    }

    #[test]
    fn dmc_tick_idle_without_sample_still_free_runs() {
        let mut d = DmcChannel::new();
        d.acc = 1;
        d.tick();
        // acc wrapped and reloaded; bit_count advanced; no crash and no
        // output level change (have_sample false).
        assert!(d.acc > 0);
        assert_eq!(d.bit_count, 1);
        assert_eq!(d.output_level, 0);
    }
}
