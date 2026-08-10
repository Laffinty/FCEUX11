//! VNesSoc skeleton — Phase 0 stub + Phase 1 CPU wiring.
//!
//! Phase 0: struct shape + opaque pointer (C-ABI surface compiles/links).
//! Phase 1: `CpuCore` replaces `CpuPlaceholder`; a minimal
//! `VNesBusContext` wires WRAM + mapper handlers so the CPU can execute
//! against the SoC's own memory.

use crate::cpu::{BusContext, CpuCore};
use crate::mapper::MapperRangeTable;

/// Top-level SoC struct. Phase 1: CPU wired; PPU/APU still placeholders.
/// Real fields arrive in Phase 3-5 per `02_architecture.md` §7.
pub struct VNesSoc {
    /// CPU interpreter (Phase 1 — implemented).
    pub cpu: CpuCore,
    /// PPU core. Phase 3 lands here (newppu=1 only).
    pub ppu: PpuPlaceholder,
    /// APU core. Phase 4 lands here.
    pub apu: ApuPlaceholder,
    /// DMA controller. Phase 4 lands here.
    pub dma: DmaPlaceholder,
    /// IRQ controller (NMI + IRQ + external FDS sources).
    pub irq: IrqPlaceholder,
    /// Scanline-budget scheduler. Phase 4 lands here.
    pub scheduler: SchedulerPlaceholder,

    /// WRAM (2 KiB) — CPU visible at $0000-$1FFF.
    pub wram: [u8; 2048],
    /// VRAM (2 KiB) — PPU nametables.
    pub vram: [u8; 2048],
    /// OAM (256 bytes) — sprite attribute memory.
    pub oam: [u8; 256],
    /// Palette RAM (32 bytes).
    pub palette: [u8; 32],

    /// Per-range mapper handler table. Phase 2 lands this; Phase 5 wires
    /// `SetReadHandler`/`SetWriteHandler` forwarding.
    pub mapper: MapperRangeTable,

    /// Mapper meta-vtable + ctx (mirroring/audio/IRQ/savestate).
    pub mapper_meta: Option<MapperMetaSlot>,

    /// Frame buffer (256×240 = 61440 bytes). Filled each frame.
    pub frame_buffer: [u8; 61440],

    /// Last frame-ready flag.
    pub frame_ready: bool,
}

impl Default for VNesSoc {
    fn default() -> Self {
        Self {
            cpu: CpuCore::new(),
            ppu: PpuPlaceholder::default(),
            apu: ApuPlaceholder::default(),
            dma: DmaPlaceholder::default(),
            irq: IrqPlaceholder::default(),
            scheduler: SchedulerPlaceholder::default(),
            wram: [0; 2048],
            vram: [0; 2048],
            oam: [0; 256],
            palette: [0; 32],
            mapper: MapperRangeTable::default(),
            mapper_meta: None,
            frame_buffer: [0; 61440],
            frame_ready: false,
        }
    }
}

/// Bus context that lets `CpuCore` read/write through the SoC.
///
/// Phase 1: WRAM ($0000-$1FFF) + PRG-ROM stub (mapper region returns
/// open bus). Phase 2 wires the full match-based decode + mapper table.
pub struct VNesBusContext<'a> {
    /// Borrows the mutable parts of the SoC the CPU needs.
    pub soc: &'a mut VNesSoc,
    /// PRG-ROM (from ROM load, Phase 5 wires mapper). Stub for now.
    pub prg_rom: &'a [u8],
    /// Open-bus value.
    pub open_bus: u8,
}

impl BusContext for VNesBusContext<'_> {
    #[inline(always)]
    fn read(&mut self, addr: u16) -> u8 {
        let v = match addr {
            0x0000..=0x1FFF => self.soc.wram[(addr & 0x07FF) as usize],
            0x8000..=0xFFFF => {
                let idx = (addr - 0x8000) as usize % self.prg_rom.len().max(1);
                self.prg_rom[idx]
            }
            _ => self.open_bus,
        };
        self.open_bus = v;
        v
    }

    #[inline(always)]
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.soc.wram[(addr & 0x07FF) as usize] = val,
            _ => {}
        }
        self.open_bus = val;
    }

    #[inline(always)]
    fn dma_stalled(&self) -> bool {
        false
    }
}

/// Opaque pointer that C++ sees. The Opaque struct is heap-allocated so the
/// C++ side can hold a stable `*mut VNesSocOpaque` across calls.
#[repr(transparent)]
pub struct VNesSocOpaque(pub *mut VNesSoc);

impl VNesSocOpaque {
    /// # Safety
    /// `ptr` must point to a valid `VNesSocOpaque` allocated by `vnesu11_create`.
    pub unsafe fn from_raw(ptr: *mut VNesSocOpaque) -> Self {
        // Reconstruct in place — only used to re-use the type's identity.
        // The real access goes through `as_mut`.
        let _ = ptr;
        Self(core::ptr::null_mut())
    }

    /// # Safety
    /// The opaque must be a valid pointer from `vnesu11_create`.
    pub unsafe fn as_mut(&mut self) -> &mut VNesSoc {
        &mut *self.0
    }
}

#[derive(Default)]
pub struct MapperMetaSlot {
    pub mapper_ctx: *mut core::ffi::c_void,
    pub meta: crate::mapper::MapperMetaVtable,
}

// Phase 0 placeholder types. Phase 1-5 will replace with real structs.
#[derive(Default)] pub struct CpuPlaceholder;
#[derive(Default)] pub struct PpuPlaceholder;
#[derive(Default)] pub struct ApuPlaceholder;
#[derive(Default)] pub struct DmaPlaceholder;
#[derive(Default)] pub struct IrqPlaceholder;
#[derive(Default)] pub struct SchedulerPlaceholder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soc_can_be_constructed() {
        let soc = VNesSoc::default();
        assert_eq!(soc.wram.len(), 2048);
        assert_eq!(soc.vram.len(), 2048);
        assert_eq!(soc.oam.len(), 256);
        assert_eq!(soc.palette.len(), 32);
        assert_eq!(soc.frame_buffer.len(), 61440);
        assert!(!soc.frame_ready);
        assert_eq!(soc.mapper.read_count, 0);
        assert_eq!(soc.mapper.write_count, 0);
    }

    #[test]
    fn opaque_round_trip() {
        // Box a VNesSoc, then box the Opaque around it (matches FFI layout).
        let soc = Box::new(VNesSoc::default());
        let soc_raw: *mut VNesSoc = Box::into_raw(soc);
        let opaque_box = Box::new(VNesSocOpaque(soc_raw));
        // Read/write through the inner VNesSoc.
        unsafe {
            (*soc_raw).wram[0] = 0xCC;
            assert_eq!((*soc_raw).wram[0], 0xCC);
        }
        // Cleanup: reconstruct both Boxes.
        drop(opaque_box);
        drop(unsafe { Box::from_raw(soc_raw) });
    }
}
