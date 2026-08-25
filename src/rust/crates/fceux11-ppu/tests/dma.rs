//! Integration tests for OAM DMA (`$4014`).
//!
//! Phase 1 implements the DMA as a synchronous 256-byte copy: the
//! PPU reads `(page << 8)..=(page << 8) | 0xFF` through the bus and
//! writes the bytes into primary OAM, then resets `oam_addr` to 0.
//! Phase 3 will replace this with the per-cycle asynchronous pump
//! driven by the unified scheduler (plan §6.4).
//!
//! Reference: <https://www.nesdev.org/wiki/PPU_registers#OAM_DMA_($4014)_>.

use fceux11_ppu::{FlatBus, PpuState};

#[test]
fn dma_copies_256_bytes_from_source_page() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    // Lay out source page $02 with a deterministic pattern: byte at
    // offset i = 0x40 + i (with wrap).
    bus.fill_cpu_page(0x02, 0x40);
    // Sanity: sentinel in OAM to be overwritten.
    s.oam[0] = 0xFF;
    s.oam[0xFF] = 0xFF;

    s.start_oam_dma(&mut bus, 0x02);

    assert_eq!(s.oam[0], 0x40, "first byte of source page");
    assert_eq!(s.oam[0x10], 0x40u8.wrapping_add(0x10));
    assert_eq!(s.oam[0x80], 0x40u8.wrapping_add(0x80));
    assert_eq!(s.oam[0xFF], 0x40u8.wrapping_add(0xFF));
}

#[test]
fn dma_resets_oam_addr_to_zero() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    s.registers.oam_addr = 0x80;
    s.start_oam_dma(&mut bus, 0x02);
    assert_eq!(s.registers.oam_addr, 0, "DMA sets oam_addr = 0");
}

#[test]
fn dma_writes_to_primary_oam_not_secondary() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    bus.fill_cpu_page(0x02, 0x40);
    // Pre-fill secondary OAM with a sentinel pattern.
    for i in 0..s.secondary_oam.len() {
        s.secondary_oam[i] = 0xAA;
    }
    s.start_oam_dma(&mut bus, 0x02);
    // Primary OAM should be overwritten with the source page contents.
    assert_eq!(s.oam[0], 0x40);
    assert_eq!(s.oam[0xFF], 0x40u8.wrapping_add(0xFF));
    // Secondary OAM should be untouched.
    for i in 0..s.secondary_oam.len() {
        assert_eq!(
            s.secondary_oam[i], 0xAA,
            "secondary_oam[{i}] must not be clobbered by DMA"
        );
    }
}

#[test]
fn dma_reads_through_provided_bus() {
    // The PPU must consult the bus for the source page — not a private
    // copy. We exercise this by routing reads through a custom PpuBus.
    use fceux11_ppu::PpuBus;

    struct CountingBus {
        read_count: u32,
        source: [u8; 0x100],
    }
    impl PpuBus for CountingBus {
        fn read(&mut self, addr: u16) -> u8 {
            self.read_count += 1;
            self.source[((addr >> 8) << 8 | (addr & 0xFF)) as usize & 0xFF]
        }
        fn write(&mut self, _addr: u16, _val: u8) {}
    }

    let mut bus = CountingBus {
        read_count: 0,
        source: [0u8; 0x100],
    };
    // Lay out a unique pattern.
    for i in 0..0x100 {
        bus.source[i] = 0xA0u8.wrapping_add(i as u8);
    }

    let mut s = PpuState::new();
    s.start_oam_dma(&mut bus, 0x03);
    assert_eq!(bus.read_count, 256, "DMA must issue exactly 256 bus reads");
    assert_eq!(s.oam[0], 0xA0);
    assert_eq!(s.oam[0x10], 0xA0u8.wrapping_add(0x10));
    assert_eq!(s.oam[0xFF], 0xA0u8.wrapping_add(0xFF));
}

#[test]
fn dma_overwrites_prior_oam_contents_completely() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    // Fill OAM with garbage.
    for i in 0..s.oam.len() {
        s.oam[i] = 0xFE;
    }
    // Fill source page $04 with another pattern.
    bus.fill_cpu_page(0x04, 0x10);
    s.start_oam_dma(&mut bus, 0x04);
    for i in 0..s.oam.len() {
        assert_eq!(s.oam[i], 0x10u8.wrapping_add(i as u8));
    }
}

#[test]
fn secondary_oam_is_independent_of_dma_state() {
    let mut s = PpuState::new();
    let mut bus = FlatBus::new();
    bus.fill_cpu_page(0x02, 0x40);
    s.start_oam_dma(&mut bus, 0x02);
    // DMA does not clear secondary_oam_count — that field tracks the
    // eval-time sprite roster. We just confirm the *byte buffer* is
    // unchanged by DMA.
    for &b in s.secondary_oam.iter() {
        assert_eq!(b, 0, "DMA leaves secondary_oam bytes untouched");
    }
}
