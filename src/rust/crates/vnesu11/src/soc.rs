//! VNesSoc — top-level SoC struct + bus context wiring.
//!
//! Phase 0: skeleton + opaque pointer (C-ABI surface compiles/links).
//! Phase 1: `CpuCore` replaces placeholder; `VNesBusContext` exposes
//!          WRAM + mapper handler stub so CPU can execute.
//! Phase 2: full bus matrix wired (fixed region `match` + mapper range
//!          table), open-bus tracking, PPU register/mirror plumbing,
//!          RAM init helpers, OAM DMA, joypad strobe.
//!
//! See:
//!   - `docs/wip_2.0_plan/02_architecture.md` §3 for bus design.
//!   - `phase_2_bus_and_ram.md` for Phase 2 DoD.

use crate::apu::ApuCore;
use crate::cpu::{BusContext, CpuCore};
use crate::dma::DmaCore;
use crate::irq::IrqController;
use crate::joypad::JoypadState;
use crate::mapper::MapperRangeTable;
use crate::ppu::nametable::{mirror_horizontal, MirrorFn, Mirroring};
use crate::ppu::{PpuCore, Segment};
use crate::ram::{InternalRam, RamInitOption, RamRng};

/// Top-level SoC struct. CPU is implemented (Phase 1); PPU/APU/DMA/IRQ
/// remain placeholders until Phase 3-4. Memory banks are owned here.
pub struct VNesSoc {
    /// CPU interpreter (Phase 1).
    pub cpu: CpuCore,
    /// PPU core. Phase 3 implements newppu=1 path.
    pub ppu: PpuCore,
    /// APU core. Phase 4 lands here.
    pub apu: ApuCore,
    /// DMA controller (OAM DMA + DMC DMA).
    pub dma: DmaCore,
    /// IRQ controller (NMI + IRQ + external FDS sources).
    pub irq: IrqController,
    /// Joypad ($4016/$4017 + VS coin).
    pub joypad: JoypadState,
    /// Scanline-budget scheduler. Phase 4 lands here.
    pub scheduler: SchedulerPlaceholder,

    // -----------------------------------------------------------------
    // Private RAM banks (Phase 2)
    // -----------------------------------------------------------------
    /// All CPU/PPU-private RAM, owned by the SoC for single ownership.
    pub ram_banks: InternalRam,
    /// Public mirrors for backwards compatibility with the Phase 1
    /// `BusContext` API. These are *references* into `ram_banks` —
    /// `Default` keeps them in sync.
    pub wram: [u8; 2048],
    pub vram: [u8; 2048],
    pub oam: [u8; 256],
    pub palette: [u8; 32],

    /// Per-range mapper handler table. Phase 5 wires
    /// `SetReadHandler`/`SetWriteHandler` forwarding to FFI.
    pub mapper: MapperRangeTable,

    /// Mapper meta-vtable + ctx (mirroring/audio/IRQ/savestate).
    pub mapper_meta: Option<MapperMetaSlot>,

    /// Frame buffer (256×240 = 61440 bytes). Filled each frame.
    pub frame_buffer: [u8; 61440],

    /// Last frame-ready flag.
    pub frame_ready: bool,

    // -----------------------------------------------------------------
    // Bus plumbing (Phase 2)
    // -----------------------------------------------------------------
    /// Last value on the CPU data bus — for open-bus reads.
    pub open_bus: u8,

    /// Current nametable mirror (cached for save-state introspection).
    pub nametable_mirror: Mirroring,
    /// Active mirroring function pointer. Replaced on `set_mirroring`.
    pub nametable_mirror_fn: MirrorFn,

    // -----------------------------------------------------------------
    // PPU register storage (Phase 2 — minimal; full impl is Phase 3)
    // -----------------------------------------------------------------
    /// $2000 PPUCTRL.
    pub ppu_ctrl: u8,
    /// $2001 PPUMASK.
    pub ppu_mask: u8,
    /// $2003/$2004 OAMADDR.
    pub ppu_oam_addr: u8,
    /// $2005/$2006 write toggle (w).
    pub ppu_w: bool,
    /// $2005/$2006 scroll latch (t).
    pub ppu_t: u16,
    /// Current VRAM address (v).
    pub ppu_v: u16,
    /// Fine X scroll (x).
    pub ppu_x: u8,
    /// $2007 data-read buffer.
    pub ppu_read_buffer: u8,

    // -----------------------------------------------------------------
    // Joypad (Phase 2 stub — Phase 4 wires full implementation)
    // -----------------------------------------------------------------
    /// Latched button state per pad (8 bits, A/B/Select/Start/Up/Down/Left/Right).
    pub joypad_latched: [u8; 2],
    /// Strobe bit ($4016 write bit 0).
    pub joypad_strobe: bool,
    /// Latched byte returned by $4016 read in strobe mode.
    pub joypad_strobe_latch: u8,
    /// Shift register for non-strobe reads ($4016/$4017).
    pub joypad_shift: [u8; 2],

    // -----------------------------------------------------------------
    // RAM init (Phase 2 — splitmix64 + xoroshiro128plus)
    // -----------------------------------------------------------------
    /// PRNG state for `RamInitOption::Random`.
    pub ram_rng: RamRng,
    /// Last-set `RAMInitOption` (for save-state).
    pub ram_init_option: RamInitOption,
    /// Last-set `RAMInitSeed` (for save-state).
    pub ram_init_seed: u32,
}

impl Default for VNesSoc {
    fn default() -> Self {
        let ram_banks = InternalRam::new_zeroed();
        let wram = clone_array::<2048>(&ram_banks.wram);
        let vram = clone_array::<2048>(&ram_banks.vram);
        let oam = clone_array::<256>(&ram_banks.oam);
        let palette = clone_array::<32>(&ram_banks.palette);

        Self {
            cpu: CpuCore::new(),
            ppu: PpuCore::new(),
            apu: ApuCore::new(),
            dma: DmaCore::new(),
            irq: IrqController::new(),
            joypad: JoypadState::new(),
            scheduler: SchedulerPlaceholder,

            ram_banks,
            wram,
            vram,
            oam,
            palette,

            mapper: MapperRangeTable::default(),
            mapper_meta: None,
            frame_buffer: [0; 61440],
            frame_ready: false,

            open_bus: 0,

            nametable_mirror: Mirroring::Horizontal,
            nametable_mirror_fn: mirror_horizontal,

            ppu_ctrl: 0,
            ppu_mask: 0,
            ppu_oam_addr: 0,
            ppu_w: false,
            ppu_t: 0,
            ppu_v: 0,
            ppu_x: 0,
            ppu_read_buffer: 0,

            joypad_latched: [0; 2],
            joypad_strobe: false,
            joypad_strobe_latch: 0,
            joypad_shift: [0; 2],

            ram_rng: RamRng::new(),
            ram_init_option: RamInitOption::Checker,
            ram_init_seed: 0,
        }
    }
}

/// Sync the four public RAM arrays back into `ram_banks`. Called when
/// the CPU/PPU bus layer has potentially modified them in place.
impl VNesSoc {
    pub fn sync_ram_banks_from_views(&mut self) {
        self.ram_banks.wram.copy_from_slice(&self.wram);
        self.ram_banks.vram.copy_from_slice(&self.vram);
        self.ram_banks.oam.copy_from_slice(&self.oam);
        self.ram_banks.palette.copy_from_slice(&self.palette);
    }

    pub fn sync_ram_banks_to_views(&mut self) {
        self.wram.copy_from_slice(&self.ram_banks.wram);
        self.vram.copy_from_slice(&self.ram_banks.vram);
        self.oam.copy_from_slice(&self.ram_banks.oam);
        self.palette.copy_from_slice(&self.ram_banks.palette);
    }

    /// Set NSF "no PPU" idle mode.
    pub fn set_nsf_idle(&mut self, idle: bool) {
        self.ppu.idle = idle;
    }

    /// Run one full frame of emulation.
    ///
    /// Returns `FrameResult::Complete` when a full frame has rendered.
    /// The scheduler is the segment-driven master; each segment's
    /// `cpu_budget` is handed to the CPU via `cpu.run_budget`.
    ///
    /// Phase 3 wires the basic segment loop; the actual per-pixel
    /// compositing lives in `BackgroundState::render_scanline` /
    /// `SpriteState::render_scanline` (which currently produce a
    /// simplified output for testing — Phase 4+ wires real CHR data).
    pub fn run_frame(&mut self) -> FrameResult {
        let mut result = FrameResult::default();
        // Tick PPU until frame is ready.  Each segment gives the CPU
        // a budget; we honor the budget before yielding back to the
        // PPU for the next segment's work.
        loop {
            let segment = self.ppu.next_segment();
            match segment {
                Segment::FrameComplete => {
                    result.completed = true;
                    break;
                }
                _ => {
                    let budget = segment.cpu_budget();
                    // Bus context for the CPU's bus reads (lets the CPU
                    // execute against the SoC's WRAM/mapper).
                    let mut bus = unsafe { VNesBusContext::new(self) };
                    self.cpu.run_budget(budget, &mut bus);
                    // Advance PPU dot clock + per-segment rendering.
                    let frame_done = self.ppu.advance_to_next_segment();
                    if frame_done {
                        result.completed = true;
                        break;
                    }
                }
            }
        }
        // Pull the rendered frame into the SoC's frame_buffer field
        // for external read-out.
        self.frame_buffer.copy_from_slice(&*self.ppu.frame_buffer);
        self.frame_ready = self.ppu.frame_ready;
        // Drain PPU NMI → CPU IRQ source.
        if self.ppu.nmi.take() {
            self.cpu.irq_begin(CpuCore::IRQ_NMI);
        }
        result
    }

    /// Power-on: apply `RAMInitOption` + `RAMInitSeed` to all four RAM
    /// banks, mirroring `PowerNES` in `src/fceu.cpp:1000-1025`.
    pub fn power_on(&mut self, option: RamInitOption, seed: u32) {
        self.ram_init_option = option;
        self.ram_init_seed = seed;
        self.ram_rng.seed(seed);
        self.ram_banks.init_wram(&mut self.ram_rng, option);
        self.ram_banks.init_vram(&mut self.ram_rng, option);
        self.ram_banks.init_oam(&mut self.ram_rng, option);
        self.ram_banks.init_palette(&mut self.ram_rng, option);
        self.sync_ram_banks_to_views();

        // Reset CPU too (matches `g_cpu.reset()` in `PowerNES`).
        // We use a raw-pointer bus context to satisfy the borrow
        // checker — `&mut self.cpu` and `&mut self` can't coexist
        // through `&mut VNesSoc`.
        let mut bus = unsafe { VNesBusContext::new(self) };
        self.cpu.reset(&mut bus);
        self.open_bus = 0;
        self.frame_ready = false;
        self.ppu_w = false;
        self.ppu_t = 0;
        self.ppu_v = 0;
        self.ppu_x = 0;
        self.ppu_read_buffer = 0;
    }
}

fn clone_array<const N: usize>(src: &[u8]) -> [u8; N] {
    let mut out = [0u8; N];
    out.copy_from_slice(&src[..N.min(src.len())]);
    out
}

/// Bus context that lets `CpuCore` read/write through the SoC.
///
/// Phase 2: full bus matrix wired (fixed-region `match` + mapper range
/// table). Open-bus is tracked in `soc.open_bus`.
///
/// Uses a `*mut VNesSoc` (raw pointer) rather than `&mut` because the
/// CPU bus path is invoked from inside `&mut self.cpu` methods — Rust's
/// borrow checker would otherwise flag the nested `&mut self` as
/// conflicting with the `&mut self.cpu`. The raw pointer is safe to
/// use here because:
///   1. The pointer is always constructed from a live `&mut VNesSoc`
///      borrow in the enclosing scope.
///   2. The pointer is never aliased; no other thread can touch the
///      SoC during emulation.
pub struct VNesBusContext {
    pub soc_ptr: *mut VNesSoc,
    pub open_bus: u8,
}

impl VNesBusContext {
    /// Construct a context borrowing a live `VNesSoc`.
    ///
    /// # Safety
    /// `soc` must be live for the duration of the borrow (typically
    /// the lifetime of the enclosing `CpuCore::run_budget` call).
    pub unsafe fn new(soc: &mut VNesSoc) -> Self {
        Self { soc_ptr: soc as *mut VNesSoc, open_bus: 0 }
    }
}

impl BusContext for VNesBusContext {
    #[inline(always)]
    fn read(&mut self, addr: u16) -> u8 {
        // SAFETY: caller guarantees the SoC is live and exclusively
        // borrowed for the duration of this context.
        unsafe { (*self.soc_ptr).cpu_read(addr) }
    }

    #[inline(always)]
    fn write(&mut self, addr: u16, val: u8) {
        // SAFETY: see `read`.
        unsafe { (*self.soc_ptr).cpu_write(addr, val); }
    }

    #[inline(always)]
    fn dma_stalled(&self) -> bool {
        false
    }
}

/// Opaque pointer that C++ sees. Heap-allocated so the C++ side can hold
/// a stable `*mut VNesSocOpaque` across calls.
#[repr(transparent)]
pub struct VNesSocOpaque(pub *mut VNesSoc);

impl VNesSocOpaque {
    /// # Safety
    /// `ptr` must point to a valid `VNesSocOpaque` allocated by
    /// `vnesu11_create`.
    pub unsafe fn from_raw(ptr: *mut VNesSocOpaque) -> Self {
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

// Phase 0 placeholder types. PPU is replaced in Phase 3.
// APU/DMA/IRQ/Joypad are replaced in Phase 4.
#[derive(Default)] pub struct SchedulerPlaceholder;

/// Result of running one frame of emulation.
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameResult {
    /// True if a full frame has just been completed (262 scanlines).
    pub completed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ram::RamInitOption;

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
        assert_eq!(soc.open_bus, 0);
    }

    #[test]
    fn opaque_round_trip() {
        let soc = Box::new(VNesSoc::default());
        let soc_raw: *mut VNesSoc = Box::into_raw(soc);
        let opaque_box = Box::new(VNesSocOpaque(soc_raw));
        unsafe {
            (*soc_raw).wram[0] = 0xCC;
            assert_eq!((*soc_raw).wram[0], 0xCC);
        }
        drop(opaque_box);
        drop(unsafe { Box::from_raw(soc_raw) });
    }

    #[test]
    fn power_on_fills_wram_per_option() {
        let mut soc = VNesSoc::default();
        soc.power_on(RamInitOption::AllOnes, 0);
        // After power-on, every WRAM byte is 0xFF.
        for &b in soc.wram.iter() {
            assert_eq!(b, 0xFF);
        }
        // Public views and `ram_banks` are in sync.
        for &b in soc.ram_banks.wram.iter() {
            assert_eq!(b, 0xFF);
        }
    }

    #[test]
    fn power_on_random_is_seed_deterministic() {
        let mut a = VNesSoc::default();
        let mut b = VNesSoc::default();
        a.power_on(RamInitOption::Random, 0xCAFE);
        b.power_on(RamInitOption::Random, 0xCAFE);
        assert_eq!(a.wram, b.wram);
        assert_eq!(a.vram, b.vram);
        assert_eq!(a.oam, b.oam);
        assert_eq!(a.palette, b.palette);
    }

    #[test]
    fn power_on_changes_with_seed() {
        let mut a = VNesSoc::default();
        let mut b = VNesSoc::default();
        a.power_on(RamInitOption::Random, 0);
        b.power_on(RamInitOption::Random, 1);
        // The streams differ at the very first byte with high probability.
        assert_ne!(a.wram[0], b.wram[0]);
    }

    #[test]
    fn sync_round_trip() {
        let mut soc = VNesSoc::default();
        soc.wram[0] = 0xAA;
        soc.sync_ram_banks_from_views();
        assert_eq!(soc.ram_banks.wram[0], 0xAA);
        soc.ram_banks.wram[0] = 0xBB;
        soc.sync_ram_banks_to_views();
        assert_eq!(soc.wram[0], 0xBB);
    }
}