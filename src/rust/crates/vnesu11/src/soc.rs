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

    /// Completed frames since power-on (Phase 6 P2 shadow diagnostics).
    pub frame_count: u64,

    /// Executed CPU instructions since power-on (Phase 6 P2 shadow
    /// diagnostics; compared against C++ `g_cpu_instr_count_`).
    pub instr_count: u64,

    /// Dots consumed in the current segment's CPU budget (Phase 6 P2
    /// shadow fix: $2002 VBL-set suppression position tracking). The
    /// PPU dot clock only advances at segment boundaries, so this
    /// recreates the sub-scanline position C++ exposes as
    /// `ppur.status.cycle` during a scanline's CPU run.
    pub segment_dots: u32,

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
    // CHR pages (Phase 6 §1.1) — 8 × 1 KiB mirroring the C++ PPU's
    // vpage_[8] table. The C++ adapter's `Bus::setchr1` hook copies
    // 1 KiB from the current page into `chr_pages[idx]` whenever a
    // mapper bank-switches CHR. `bus.rs::ppu_read` consults this table
    // for $0000-$1FFF.
    // -----------------------------------------------------------------
    pub chr_pages: [[u8; 1024]; 8],

    // -----------------------------------------------------------------
    // Joypad (Phase 4 — `JoypadState` owns $4016/$4017 + VS coin)
    // -----------------------------------------------------------------

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
            frame_count: 0,
            instr_count: 0,
            segment_dots: 0,

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

            chr_pages: [[0u8; 1024]; 8],

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
    /// The PPU is the segment-driven master; each segment's
    /// `cpu_budget` is handed to the CPU via `cpu.run_budget`, the APU
    /// via `apu.tick`, and the DMA/IRQ controllers are driven at
    /// segment boundaries.
    ///
    /// Phase 5 stage 0 (wiring): the main loop now drives
    /// - OAM DMA stall (513/514 CPU cycles, real byte transfer)
    /// - APU channel ticks (frame counter / 5 channels / mixer)
    /// - IRQ routing (PPU VBlank NMI + APU FCOUNT/DMC + mapper + EXT)
    ///
    /// See `docs/wip_2.0_plan/phase_5_mapper_adapter.md` §2.0.1.
    pub fn run_frame(&mut self) -> FrameResult {
        let mut result = FrameResult::default();
        loop {
            let segment = self.ppu.next_segment();
            match segment {
                Segment::FrameComplete => {
                    result.completed = true;
                    break;
                }
                _ => {
                    let budget = segment.cpu_budget();
                    self.run_segment_inner(budget);
                    // 4. PPU renders the segment (background + sprites).
                    let frame_done = self.ppu.advance_to_next_segment();
                    // Phase 6 P2 shadow fix (2026-08-12): visible scanlines
                    // get a follow-up sprite-eval/hblank segment
                    // (85 cycles) matching C++ DoLine's
                    // `X6502_Run(6) + Run(63) + Run(16)`. Without this the
                    // CPU is short ~20k cycles/frame and the shadow PC drifts.
                    if let Some(extra) = self.ppu.sprite_eval_segment() {
                        let extra_budget = extra.cpu_budget();
                        // Phase C (2026-08-13): drive the C++ mapper HBlank
                        // IRQ hook (MMC3 scanline counter, MMC5, mapper 90,
                        // etc.). In Rust-primary mode the C++ PPU never runs,
                        // so this vtable call is the only thing that advances
                        // the mapper scanline IRQ counter. Match the C++
                        // timing: GameHBIRQHook fires after the leading
                        // X6502_Run(6) + X6502_Run(4) = 10 CPU cycles of
                        // HBlank, then the remaining 75 run.
                        self.run_segment_inner(10);
                        if let Some(slot) = &self.mapper_meta {
                            unsafe { (slot.meta.hblank_irq)(slot.mapper_ctx); }
                        }
                        self.run_segment_inner(extra_budget - 10);
                        // The follow-up segment does NOT advance the
                        // scanline — sprite eval happens within the same
                        // scanline as the background render.
                    }
                    // 5. Route interrupts at the segment boundary.
                    self.route_interrupts();
                    if frame_done {
                        self.frame_count += 1;
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
        result
    }

    /// Run one PPU segment: DMA stall → CPU run_budget → APU tick.
    /// Extracted to share the wiring between the primary segment and
    /// the visible-scanline sprite-eval follow-up segment (Phase 6 P2
    /// shadow fix).
    fn run_segment_inner(&mut self, budget: i32) {
        if budget <= 0 {
            return;
        }
        // 1. DMA stall priority: while an OAM DMA is in flight the CPU
        //    is halted (513/514 cycles). Each iteration transfers one
        //    byte (or drains a trailing alignment cycle); the PPU does
        //    not advance during the stall.
        if self.dma.is_stalling() {
            if self.dma.oam.active {
                self.step_oam_dma();
            } else if self.dma.dmc_stall_cycles > 0 {
                // DMC DMA stall (Phase 6 wires the real arbitration;
                // decrement defensively so a pending DMC stall can
                // never spin).
                self.dma.dmc_stall_cycles -= 1;
            }
            return;
        }
        // 2. CPU runs its segment budget (X6502_Run semantics) with
        //    per-instruction APU ticking AND per-instruction IRQ
        //    routing from the APU. The per-instruction APU tick is
        //    needed for sub-instruction frame-counter events (7457
        //    / 14913 / 22371 / 29828-30) so the IRQ fires at the
        //    exact cycle — matching `src/sound.cpp::FCEU_SoundCPUHook`
        //    which advances `fhcnt` one CPU cycle at a time. The
        //    per-instruction IRQ routing is needed because in C++
        //    the APU's `X6502_IRQBegin(FCEU_IQFCOUNT)` takes effect
        //    at the START of the next instruction's IRQ check (i.e.
        //    within 1 instruction of the APU event), whereas Rust's
        //    `route_interrupts` only fires at segment boundaries
        //    (up to 341 cycles late). Without per-instruction routing
        //    the frame counter IRQ can land in the wrong instruction
        //    stream and cause PC divergence after a few frames.
        let mut bus = unsafe { VNesBusContext::new(self) };
        // Phase 6 P2 shadow fix (2026-08-12, fifth edition): unit
        // alignment. The segment budgets (256/85/341) mirror C++
        // DoLine's X6502_Run arguments, whose unit is PPU DOTS: C++
        // credits `_count += cycles*16` and debits `_count -= c*48`
        // per instruction (`Cpu::add_cycles`), i.e. an effective
        // ×3 dots-per-cycle rate. Rust's `step_one` debits `tcount`
        // CPU cycles against a dots budget, so the CPU ran ~3× the
        // instructions C++ does per frame (~89k vs ~29.8k cycles),
        // drifting the APU frame-counter phase. Compensate here by
        // debiting the extra ×2 (1 cycle = 3 dots) at the scheduler
        // layer, leaving the CPU core in cycle units.
        self.cpu.count += budget;
        self.segment_dots = 0;
        let mut remaining = self.cpu.count;
        while remaining > 0 {
            if bus.dma_stalled() {
                break;
            }
            // No pending IRQ: just step one instruction.
            if self.cpu.irq_pending == 0 {
                let (new_remaining, tcount) = self.cpu.step_one(remaining, &mut bus);
                self.instr_count += 1; // Phase 6 P2 shadow diagnostics
                if tcount > 0 {
                    self.segment_dots = self.segment_dots.saturating_add((tcount * 3) as u32);
                    self.apu.tick(tcount as u32);
                    self.route_apu_irqs_to_cpu();
                }
                // Convert cycle debit to dots (×3): step_one already
                // debited tcount (×1); debit the remaining ×2.
                remaining = new_remaining - tcount * 2;
                continue;
            }
            // Pending IRQ: poll, then step.
            self.cpu.count = remaining;
            let count_before_poll = self.cpu.count;
            self.cpu.poll_interrupts(&mut bus);
            remaining = self.cpu.count;
            // Phase 6 P2 shadow fix: the C++ interrupt service path
            // does `ADDCYC(7)` (push PC/P + vector) which lands in
            // `_tcount` and is fed to `FCEU_SoundCPUHook` on the next
            // instruction. Rust's `poll_interrupts` also decrements
            // count by 7 but nothing ticked the APU for it, so the
            // frame-counter phase drifted ~7 cycles per NMI/IRQ vs
            // C++. Feed the consumed cycles to the APU here, and
            // convert the debit to dots (×3: poll debited ×1).
            let poll_consumed = (count_before_poll - remaining).max(0);
            if poll_consumed > 0 {
                self.segment_dots =
                    self.segment_dots.saturating_add((poll_consumed * 3) as u32);
                self.apu.tick(poll_consumed as u32);
                remaining -= poll_consumed * 2;
            }
            if remaining <= 0 {
                break;
            }
            let (new_remaining, tcount) = self.cpu.step_one(remaining, &mut bus);
            self.instr_count += 1; // Phase 6 P2 shadow diagnostics
            if tcount > 0 {
                self.segment_dots = self.segment_dots.saturating_add((tcount * 3) as u32);
                self.apu.tick(tcount as u32);
                self.route_apu_irqs_to_cpu();
            }
            remaining = new_remaining - tcount * 2;
        }
        self.cpu.count = remaining;
    }

    /// Push any IRQs the APU generated this cycle into the CPU's
    /// `irq_pending` mask. Called per-instruction from
    /// `run_segment_inner` to match C++'s sub-instruction IRQ
    /// timing (the C++ APU's `X6502_IRQBegin` lands at the start of
    /// the next instruction's IRQ check).  PPU NMI and mapper IRQ
    /// are still routed only at segment boundaries in
    /// `route_interrupts` (their latency is acceptable since the
    /// PPU asserts NMI at scanline boundaries, not mid-instruction).
    fn route_apu_irqs_to_cpu(&mut self) {
        let mask = self.apu.take_irq();
        if mask != 0 {
            // Phase 6 P2 shadow diagnostics: IRQ re-trigger frequency.
            {
                use std::sync::atomic::{AtomicU32, Ordering};
                static COUNT: AtomicU32 = AtomicU32::new(0);
                let n = COUNT.fetch_add(1, Ordering::Relaxed);
                if n < 10 {
                    let mut stderr = std::io::stderr();
                    use std::io::Write as _;
                    let _ = writeln!(
                        stderr,
                        "[route_apu_irq] n={} mask={:X} pc={:04X} fc={}",
                        n, mask, self.cpu.pc(),
                        self.apu.frame_counter.cycle_count
                    );
                    let _ = stderr.flush();
                }
            }
            // FCOUNT + DMC bits flow through the IrqController for
            // unified level semantics (mapper + EXT can override).
            if mask & crate::apu::IRQ_FCOUNT != 0 {
                self.irq.assert_irq(crate::apu::IRQ_FCOUNT);
            }
            if mask & crate::apu::IRQ_DMC != 0 {
                self.irq.assert_irq(crate::apu::IRQ_DMC);
            }
            let agg = self.irq.aggregate_mask();
            if agg != 0 {
                self.cpu.irq_begin(agg);
            }
        }
    }

    /// Advance one OAM DMA byte: read from the CPU address space and
    /// write into PPU OAM. Keeps the public `oam` view and the
    /// `ram_banks` copy in sync so savestate/snapshot see the data.
    ///
    /// When all 256 bytes have transferred, trailing alignment cycles
    /// (513/514 − 512) are drained without moving data.
    fn step_oam_dma(&mut self) {
        if !self.dma.oam.active {
            return;
        }
        if self.dma.oam.remaining == 0 {
            // Residual alignment drain — no data moves.
            self.dma.oam.step();
            return;
        }
        let offset = 256usize - self.dma.oam.remaining as usize;
        let addr = ((self.dma.oam.page as u16) << 8) | (offset as u16);
        let val = self.cpu_read_for_dma(addr);
        self.ppu.oam.primary[offset] = val;
        self.oam[offset] = val;
        self.ram_banks.oam[offset] = val;
        self.dma.oam.step();
    }

    /// Route interrupts at a segment boundary:
    /// - PPU VBlank NMI (edge) → `IrqController::assert_nmi`
    /// - APU frame-counter / DMC IRQ → `IrqController::assert_irq`
    /// - Mapper IRQ (via `MapperMetaVtable::tick_irq`) → IRQ_EXT
    /// - External FDS sources (EXT/EXT2) already in the controller
    ///
    /// The aggregated mask is then pushed to the CPU as pending IRQ
    /// sources (consumed by the next instruction's interrupt sample).
    fn route_interrupts(&mut self) {
        // PPU VBlank NMI (edge-triggered).
        if self.ppu.nmi.take() {
            self.irq.assert_nmi();
        }
        // APU IRQs (frame counter + DMC).
        let apu_irq = self.apu.take_irq();
        if apu_irq & crate::apu::IRQ_FCOUNT != 0 {
            self.irq.assert_irq(crate::apu::IRQ_FCOUNT);
        }
        if apu_irq & crate::apu::IRQ_DMC != 0 {
            self.irq.assert_irq(crate::apu::IRQ_DMC);
        }
        // Mapper IRQ (meta vtable; FDS disk / mapper timers).
        if let Some(slot) = &self.mapper_meta {
            let mut irq = false;
            unsafe {
                (slot.meta.tick_irq)(slot.mapper_ctx, &mut irq);
            }
            if irq {
                self.irq.assert_irq(CpuCore::IRQ_EXT);
            }
        }
        // Push to the CPU (NMI edge + level IRQ sources).
        if self.irq.take_nmi() {
            self.cpu.irq_begin(CpuCore::IRQ_NMI);
        }
        let mask = self.irq.aggregate_mask();
        if mask != 0 {
            self.cpu.irq_begin(mask);
        }
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
        // C++ X6502_Power sets _S=0xFD before X6502_Reset; the Rust
        // CpuCore::reset() deliberately does NOT touch S (mirroring
        // X6502_Reset), so set the power-on stack pointer here.
        self.cpu.s = 0xFD;
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

        // Phase 5 stage 0: reset APU / DMA / IRQ / joypad to a clean
        // power-on state so a fresh frame starts from silence.
        self.apu.power_cycle();
        self.dma.power_cycle();
        self.irq.reset();
        self.joypad.reset();

        // Phase 7 fix (2026-08-13): the PPU was NOT reset on power-on,
        // so loading a second ROM in Rust-primary mode carried over the
        // previous ROM's scanline/dot/frame counter/registers — a
        // cross-ROM contamination that made the instr_v5 blargg singles
        // non-deterministic. Rebuild the whole PPU core for a clean
        // power-on state.
        self.ppu = crate::ppu::PpuCore::new();
    }

    /// Soft reset (mirrors C++ `ResetNES` in `src/fceu.cpp:959`):
    /// CPU + APU + PPU + DMA + IRQ + joypad are reset, but RAM and the
    /// mapper state are preserved. Previously `vnesu11_reset` only reset
    /// the CPU, so blargg `apu_reset_*` ROMs (which press RESET and then
    /// inspect APU state) saw stale APU state and failed.
    pub fn reset(&mut self) {
        // CPU reset (does NOT touch S; matches X6502_Reset).
        let mut bus = unsafe { VNesBusContext::new(self) };
        self.cpu.reset(&mut bus);
        self.open_bus = 0;
        self.frame_ready = false;
        self.ppu_w = false;
        self.ppu_t = 0;
        self.ppu_v = 0;
        self.ppu_x = 0;
        self.ppu_read_buffer = 0;

        // APU / DMA / IRQ / joypad to a clean reset state.
        self.apu.power_cycle();
        self.dma.power_cycle();
        self.irq.reset();
        self.joypad.reset();

        // PPU reset (soft reset still rebuilds the PPU core; the RAM
        // / VRAM / OAM contents are preserved in ram_banks).
        self.ppu = crate::ppu::PpuCore::new();
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