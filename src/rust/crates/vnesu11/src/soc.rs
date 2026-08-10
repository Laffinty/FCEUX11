//! VNesSoc skeleton — Phase 0 stub.
//!
//! The real implementation lands across Phase 1-5. Phase 0 ships the struct
//! shape + opaque pointer so the C-ABI surface compiles and links.

use crate::mapper::MapperRangeTable;

/// Top-level SoC struct. Phase 0: all fields are placeholders.
/// Real fields arrive in Phase 1-5 per `02_architecture.md` §7.
pub struct VNesSoc {
    /// CPU interpreter. Phase 1 lands here.
    pub cpu: CpuPlaceholder,
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
            cpu: CpuPlaceholder::default(),
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
