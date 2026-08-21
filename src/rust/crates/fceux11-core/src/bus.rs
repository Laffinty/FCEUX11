//! Minimal Bus prototype for performance validation.
//!
//! This module implements a bare-bones NES bus with WRAM + PRG ROM
//! and no mapper, PPU or APU.  Its sole purpose is to prove that a
//! Rust `match`-based address decode can match (or beat) the
//! performance of the C++ `switch` / function-pointer dispatch used
//! in `src/fceu.cpp`.

use std::time::Instant;

/// A minimal bus: 2 KiB WRAM + PRG ROM.
///
/// Address decode uses a single `match` expression.  The Rust
/// compiler is expected to lower this to a jump table for the
/// contiguous ranges.
pub struct SimpleBus {
    pub wram: [u8; 0x800],
    pub prg_rom: Vec<u8>,
}

impl SimpleBus {
    pub fn new(prg_rom: Vec<u8>) -> Self {
        Self {
            wram: [0; 0x800],
            prg_rom,
        }
    }

    #[inline(always)]
    pub fn read_u8(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => 0, // PPU stub
            0x4000..=0x4017 => 0, // APU / input stub
            0x4018..=0x5FFF => 0, // expansion stub
            0x6000..=0x7FFF => 0, // PRG RAM stub
            0x8000..=0xFFFF => {
                if self.prg_rom.is_empty() {
                    0
                } else {
                    self.prg_rom[(addr as usize - 0x8000) % self.prg_rom.len()]
                }
            }
        }
    }

    #[inline(always)]
    pub fn write_u8(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => { /* PPU stub */ }
            0x4000..=0x4017 => { /* APU stub */ }
            0x4018..=0x5FFF => { /* expansion stub */ }
            0x6000..=0x7FFF => { /* PRG RAM stub */ }
            0x8000..=0xFFFF => { /* ROM read-only */ }
        }
    }
}

/// Benchmark the simple bus against a trivial workload.
///
/// We perform 1 billion sequential reads/writes and report the
/// elapsed time.  This gives a rough lower-bound on the cost of
/// the `match` decode itself.
pub fn bench_simple_bus(iterations: usize) -> std::time::Duration {
    let rom = vec![0u8; 32 * 1024]; // 32 KiB PRG ROM
    let mut bus = SimpleBus::new(rom);

    // Warm up
    for i in 0..1000 {
        bus.write_u8((i & 0x7FF) as u16, (i & 0xFF) as u8);
    }

    let start = Instant::now();
    let mut acc: u8 = 0;
    for i in 0..iterations {
        let addr = ((i * 7 + 0x2000) & 0xFFFF) as u16;
        acc = acc.wrapping_add(bus.read_u8(addr));
        bus.write_u8(addr & 0x7FF, acc);
    }
    // Prevent the compiler from eliminating the loop.
    std::hint::black_box(acc);
    start.elapsed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wram_mirror() {
        let mut bus = SimpleBus::new(vec![0u8; 32 * 1024]);
        bus.write_u8(0x0000, 0xAB);
        assert_eq!(bus.read_u8(0x0000), 0xAB);
        assert_eq!(bus.read_u8(0x0800), 0xAB);
        assert_eq!(bus.read_u8(0x1000), 0xAB);
        assert_eq!(bus.read_u8(0x1800), 0xAB);
    }

    #[test]
    fn test_prg_rom_read() {
        let mut rom = vec![0u8; 32 * 1024];
        rom[0] = 0xDE;
        rom[1] = 0xAD;
        rom[2] = 0xBE;
        rom[3] = 0xEF;
        rom[0x3FFF] = 0xCA; // index at $BFFF
        let bus = SimpleBus::new(rom);
        assert_eq!(bus.read_u8(0x8000), 0xDE);
        assert_eq!(bus.read_u8(0x8001), 0xAD);
        assert_eq!(bus.read_u8(0xBFFF), 0xCA); // $BFFF - $8000 = $3FFF
    }

    #[test]
    fn test_prg_rom_wrap() {
        let mut rom = vec![0u8; 16 * 1024]; // 16 KiB → mirrors at $8000-$BFFF and $C000-$FFFF
        rom[0] = 0x42;
        let bus = SimpleBus::new(rom);
        assert_eq!(bus.read_u8(0x8000), 0x42);
        assert_eq!(bus.read_u8(0xC000), 0x42);
    }

    #[test]
    fn test_bus_decode_stub_regions() {
        let mut bus = SimpleBus::new(vec![0u8; 32 * 1024]);
        // PPU region should be a read stub (returns 0, no panic)
        assert_eq!(bus.read_u8(0x2002), 0);
        // APU region should be a write stub (no panic)
        bus.write_u8(0x4000, 0xFF);
        // Expansion region
        assert_eq!(bus.read_u8(0x5000), 0);
    }

    #[test]
    fn bench_runs_without_panic() {
        let dur = bench_simple_bus(1_000_000);
        // On a modern x86_64, 1M iterations should finish in < 1 ms.
        // This is a sanity check, not a strict performance gate.
        assert!(
            dur.as_millis() < 100,
            "bus benchmark unexpectedly slow: {:?}",
            dur
        );
    }
}
