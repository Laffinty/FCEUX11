//! PPU bus trait and the [`FlatBus`] test stub.
//!
//! The trait surface mirrors `docs/plans/v2.1_ppu_rust_refactor_plan.md` §6.3
//! (the C ABI hooks that `ppu_rust_bridge.cpp` will install in Phase 2). In
//! Phase 1 only `read`/`write`/`peek_chr` are exercised; the notify_* hooks
//! are no-ops that Phase 3 will route through the unified scheduler.
//!
//! [`FlatBus`] is the test stub. It models a flat 64 KiB CPU address space
//! plus a 8 KiB CHR address space, with no mirroring or mapper logic —
//! enough to drive every register-write / scroll-state / OAM-DMA assertion
//! in `tests/` without dragging in a real cartridge.

/// Bus interface seen by the PPU state machine.
///
/// All methods take `&mut self` so the implementor can mutate per-tick
/// state (mapper bank-switch, A12 edge counters, hblank hooks). The
/// `notify_*` hooks have default no-op implementations so test stubs
/// only override the few they care about.
pub trait PpuBus {
    /// CPU / PPU data-bus read. The PPU routes `$2007` reads here.
    /// Address is the full 16-bit PPU bus address ($0000..=$FFFF).
    fn read(&mut self, addr: u16) -> u8;

    /// CPU / PPU data-bus write. The PPU routes `$2007` writes here.
    fn write(&mut self, addr: u16, val: u8);

    /// Peek CHR without driving the side-effects of a real read. Used
    /// during BG fetch when we want the tile byte but don't want to
    /// tick mapper IRQ counters. Phase 1 uses this in tests; Phase 3
    /// will gate the hot fetch path on it.
    fn peek_chr(&mut self, addr: u16) -> u8 {
        let _ = addr;
        0
    }

    /// Rising edge on PPU A12 (PPU address bit 12). MMC3 uses this to
    /// clock its IRQ counter. Phase 1 leaves it as a no-op; Phase 4
    /// wires the MMC3 mapper through it.
    fn notify_a12_rising(&mut self) {}

    /// HBlank hook (PPU enters the visible scanline's hblank region).
    fn notify_hblank(&mut self) {}

    /// Secondary HBlank hook — some mappers (VRC IRQ) need a different
    /// timing for the second copy of the hook.
    fn notify_hblank2(&mut self) {}

    /// Called once per scanline boundary, with the new scanline index.
    fn notify_scanline(&mut self, _sl: i16) {}

    /// Called when VBlank asserts/deasserts. `asserted=true` on enter,
    /// `asserted=false` on exit (sl 261 for NTSC).
    fn notify_vblank(&mut self, _asserted: bool) {}
}

// ---------------------------------------------------------------------------
// FlatBus — minimal test stub.
//
// 64 KiB CPU RAM + 8 KiB CHR ROM. No mirroring, no PRG banking. The OAM
// DMA test uses this directly to populate the source page before issuing
// $4014; the register tests use it to verify $2007 read/write round-trip.
// ---------------------------------------------------------------------------

/// 64 KiB flat bus for unit tests. `cpu` is read/written for any address
/// `$0000..=$FFFF`; `chr` is read-only for `$0000..=$1FFF`. Out-of-range
/// reads return 0; out-of-range writes are dropped.
#[derive(Debug, Clone)]
pub struct FlatBus {
    /// CPU address space, 64 KiB.
    pub cpu: [u8; 0x10000],
    /// CHR address space, 8 KiB (peek/read both consult this).
    pub chr: [u8; 0x2000],
}

impl FlatBus {
    /// Build an empty bus — all zeros.
    pub fn new() -> Self {
        Self {
            cpu: [0u8; 0x10000],
            chr: [0u8; 0x2000],
        }
    }

    /// Pre-fill `cpu[page << 8 .. page << 8 | 0xFF]` with a deterministic
    /// pattern: byte at offset `i` is `(page as u8).wrapping_add(i as u8)`.
    /// Used by the OAM-DMA test to lay out the source page.
    pub fn fill_cpu_page(&mut self, page: u8, base_value: u8) {
        let p = (page as usize) << 8;
        for i in 0..0x100usize {
            self.cpu[p + i] = base_value.wrapping_add(i as u8);
        }
    }
}

impl Default for FlatBus {
    fn default() -> Self {
        Self::new()
    }
}

impl PpuBus for FlatBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.cpu[addr as usize]
    }

    fn write(&mut self, addr: u16, val: u8) {
        self.cpu[addr as usize] = val;
    }

    fn peek_chr(&mut self, addr: u16) -> u8 {
        self.chr[(addr & 0x1FFF) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_bus_round_trip() {
        let mut bus = FlatBus::new();
        bus.write(0x2007, 0xAB);
        assert_eq!(bus.read(0x2007), 0xAB);
        assert_eq!(bus.read(0x0000), 0x00);
        assert_eq!(bus.peek_chr(0x0000), 0x00);
    }

    #[test]
    fn flat_bus_fill_cpu_page_is_deterministic() {
        let mut bus = FlatBus::new();
        bus.fill_cpu_page(0x02, 0x10);
        assert_eq!(bus.cpu[0x0200], 0x10);
        assert_eq!(bus.cpu[0x0201], 0x11);
        assert_eq!(bus.cpu[0x02FF], 0x10u8.wrapping_add(0xFF));
    }

    #[test]
    fn notify_hooks_default_to_noop() {
        // PpuBus's default impls for the notify_* hooks must not panic
        // when the test stub doesn't override them — Phase 3 will wire
        // real mapper implementations through them.
        let mut bus = FlatBus::new();
        bus.notify_a12_rising();
        bus.notify_hblank();
        bus.notify_hblank2();
        bus.notify_scanline(7);
        bus.notify_vblank(true);
        bus.notify_vblank(false);
    }
}
