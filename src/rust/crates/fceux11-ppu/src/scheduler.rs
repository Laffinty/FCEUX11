//! `NesScheduler` — Phase 3 of `docs/plans/v2.1_ppu_rust_refactor_plan.md`.
//!
//! Unified CPU + PPU driver. Holds both halves of the emulator state
//! and drives them in lockstep at the 3 PPU dots / 1 CPU cycle ratio
//! for NTSC (and the equivalent rational fraction for PAL/Dendy).
//!
//! ## Granularity
//!
//! Phase 3 uses **per-instruction** CPU granularity —
//! [`crate::stepper`] wraps [`crate::frame::tick_dot`]-per-PPU-dot
//! advancement and runs the CPU one whole instruction per interleave,
//! matching the C++ `X6502_Run(1)` call pattern in
//! `src/ppu_rendering.cpp:1711-2170`. The plan §6.2 documents this as
//! the acceptable Phase 3 trade-off; a future phase can swap the
//! stepper for a true per-cycle state machine without changing this
//! scheduler's public surface.
//!
//! ## OAM DMA
//!
//! The scheduler drives the OAM DMA pump one byte per CPU cycle
//! (256 cycles = 768 PPU dots), matching the hardware behaviour. The
//! C++ reference calls `X6502_DMW()` 256 times in a tight loop
//! inside `FCEUPPU_Loop`; here we interleave the DMA reads with the
//! PPU dot clock so the bus contention is observable at the right
//! dot.
//!
//! ## Mapper event hooks
//!
//! The scheduler fires the [`PpuBus`] trait's `notify_*` callbacks at
//! the natural PPU-side boundaries:
//!
//! - `notify_hblank2` fires when entering the post-render hblank
//!   (sl 240+, dot 320).
//! - `notify_scanline(sl)` fires on every scanline boundary change.
//! - `notify_a12_rising` fires on **filtered** PPU A12 rising edges
//!   (v2.1.3 batch 1): the fetch-address model in [`crate::a12`]
//!   presents every fetch address to the watcher inside
//!   [`crate::frame::tick_dot`], and the CPU access path reports the
//!   `v`-driven transitions while rendering is off. This replaces the
//!   v2.1.2 per-scanline approximation (which fired `notify_hblank`
//!   once per visible scanline at dot 256) — `notify_hblank` is no
//!   longer fired by this scheduler.
//! - `notify_vblank(asserted)` fires on VBL flag transitions.

use crate::bus::PpuBus;
use crate::frame::{TickOutcome, tick_dot};
use crate::state::PpuState;
use crate::video_system::VideoSystem;

/// NTSC frame budget (PPU dots per frame). Kept as a named constant for
/// the existing call sites; the authoritative table lives in
/// [`crate::video_system`] (Step B.5-2a).
pub const NTSC_CPU_CYCLES_PER_FRAME: u32 =
    VideoSystem::Ntsc.timings().dots_per_frame;
/// PAL/Dendy frame budget (PPU dots per frame).
pub const PAL_CPU_CYCLES_PER_FRAME: u32 =
    VideoSystem::Pal.timings().dots_per_frame;
/// PPU dots per CPU cycle for the NTSC/Dendy ratio (1 CPU cycle = 3 PPU
/// dots). PAL uses 16 dots per 5 CPU cycles (3.2) - wiring that into the
/// interleave loop is Step B.5-2c; until then this constant is what all
/// call sites use. See `crate::video_system::VideoSystemTimings`.
pub const PPU_DOTS_PER_CPU_CYCLE: u32 = 3;

/// Unified scheduler state.
#[derive(Debug, Default)]
pub struct NesScheduler {
    /// Total CPU cycles consumed since last frame start. Used to
    /// advance the PPU proportionally after each CPU instruction.
    cpu_cycles_consumed: u32,
    /// Total PPU dots consumed since last frame start. Used to decide
    /// when the frame is complete.
    ppu_dots_consumed: u32,
    /// Frame budget (PPU dots per frame) - from the region table.
    frame_cycles: u32,
    /// Region timings this scheduler was configured for (B.5-2a).
    video_system: VideoSystem,
    /// Last scanline seen — used to fire `notify_scanline` only on
    /// transitions.
    last_scanline: i16,
    /// Cached VBL state — used to fire `notify_vblank` only on
    /// transitions.
    vbl_asserted: bool,
}

impl NesScheduler {
    pub const fn new() -> Self {
        Self {
            cpu_cycles_consumed: 0,
            ppu_dots_consumed: 0,
            frame_cycles: NTSC_CPU_CYCLES_PER_FRAME,
            video_system: VideoSystem::Ntsc,
            last_scanline: -1,
            vbl_asserted: false,
        }
    }

    /// Configure the scheduler for a region. Step B.5-2a: the frame
    /// budget now comes from the [`VideoSystem`] timing table; the
    /// raster/interleave still run the NTSC path until B.5-2b/c.
    pub fn set_video_system(&mut self, system: VideoSystem) {
        self.video_system = system;
        self.frame_cycles = system.timings().dots_per_frame;
    }

    /// Region this scheduler was configured for.
    #[inline]
    pub fn video_system(&self) -> VideoSystem {
        self.video_system
    }

    /// Reset the per-frame accumulators (called at the start of every
    /// `emulate_frame`).
    pub fn begin_frame(&mut self) {
        self.cpu_cycles_consumed = 0;
        self.ppu_dots_consumed = 0;
        self.last_scanline = -2; // sentinel — fires once on first tick
        self.vbl_asserted = false;
    }

    /// Phase 5.1: reset the per-frame accumulators after the frame
    /// wrap detected by the per-dot driver
    /// (`ffi::fceux11_ppu_tick_dots`). Unlike [`Self::begin_frame`],
    /// this does NOT re-arm the scanline sentinel: the transition into
    /// the pre-render line already fired inside the wrap tick, and
    /// re-arming would double-fire `notify_scanline(-1)` on the first
    /// tick of the new frame.
    pub fn reset_frame_counters(&mut self) {
        self.cpu_cycles_consumed = 0;
        self.ppu_dots_consumed = 0;
        self.vbl_asserted = false;
    }

    /// Total CPU cycles consumed by the current frame so far.
    #[inline]
    pub fn cpu_cycles_consumed(&self) -> u32 {
        self.cpu_cycles_consumed
    }

    /// Total PPU dots consumed by the current frame so far.
    #[inline]
    pub fn ppu_dots_consumed(&self) -> u32 {
        self.ppu_dots_consumed
    }

    /// Frame cycle budget.
    #[inline]
    pub fn frame_cycles(&self) -> u32 {
        self.frame_cycles
    }

    /// Advance the PPU by exactly one dot, firing mapper event hooks
    /// as appropriate (scanline transitions, VBL transitions,
    /// hblank). Returns the [`TickOutcome`] so the caller can route
    /// NMI / VBL events.
    pub fn tick_one_ppu_dot<B: PpuBus + ?Sized>(
        &mut self,
        state: &mut PpuState,
        bus: &mut B,
    ) -> TickOutcome {
        let outcome = tick_dot(state, bus);
        self.ppu_dots_consumed += 1;

        // Scanline transition → notify_scanline. The PPU transitions
        // scanline once per frame wrap (sl 261 → -1) and once per dot
        // 340 wrap; we only fire when the scanline actually changes.
        if state.scanline != self.last_scanline {
            bus.notify_scanline(state.scanline);
            self.last_scanline = state.scanline;
        }

        // HBlank hooks fire at the natural dot boundaries:
        // - notify_hblank2: scanline enters the very-end hblank
        //   (dot 320+) — VRC IRQ hook.
        // (v2.1.3 batch 1: notify_hblank at dot 256 was the v2.1.2
        // per-scanline MMC3 IRQ approximation; the filtered A12 path in
        // `crate::frame::tick_dot` supersedes it, so it no longer
        // fires — routing both would double-clock MMC3 counters.)
        if state.dot == 320 && state.scanline >= 0 {
            bus.notify_hblank2();
        }

        // VBlank transition. The PPU state machine sets the VBL flag
        // at sl 241 dot 1; we observe `outcome.vbl_entered` and fire
        // notify_vblank on the rising edge.
        // Step B.5-2b: VBL window from the region table (NTSC 241..=260,
        // PAL 241..=310, Dendy 291..=310).
        let timings = state.video_system.timings();
        let vbl_now = outcome.vbl_entered
            || (state.scanline >= timings.vbl_set_scanline
                && state.scanline <= timings.vbl_end_scanline);
        if vbl_now != self.vbl_asserted {
            self.vbl_asserted = vbl_now;
            bus.notify_vblank(vbl_now);
        }

        // (v2.1.3 batch 1: the dot-256 notify_a12_rising firing — an
        // unfiltered once-per-scanline approximation — is gone; real
        // filtered edges now come from the fetch-address watcher.)

        outcome
    }

    /// Advance the PPU by `n_dots` dots without any CPU interleaving.
    /// Used by the synchronous Phase 1/2 path (the
    /// `fceux11_ppu_emulate_frame` FFI's `for _ in 0..n_dots` loop
    /// before Phase 3 replaces it with the unified scheduler).
    pub fn tick_ppu_dots<B: PpuBus + ?Sized>(
        &mut self,
        state: &mut PpuState,
        bus: &mut B,
        n_dots: u32,
    ) -> TickOutcome {
        let mut last = TickOutcome::default();
        for _ in 0..n_dots {
            last = self.tick_one_ppu_dot(state, bus);
        }
        last
    }

    /// Drive one full frame's worth of PPU dots, with the option to
    /// interleave CPU instructions at the 3:1 PPU-dot/CPU-cycle
    /// ratio. The CPU-side interleave is driven by a closure that
    /// runs one CPU instruction and reports the cycle count; the
    /// scheduler advances PPU dots accordingly and routes PPU
    /// NMI/VBL events back to the CPU through the closure.
    ///
    /// The closure signature:
    ///
    /// ```ignore
    /// fn run_one_cpu_instruction() -> u8   // cycles consumed
    /// ```
    ///
    /// After the closure returns, the scheduler:
    ///
    /// 1. Advances PPU by `(cycles * PPU_DOTS_PER_CPU_CYCLE)` dots.
    /// 2. If the most-recently-advanced PPU dot asserted NMI, the
    ///    closure is informed via the `on_nmi_asserted` callback.
    ///
    /// For Phase 3's minimum-viable form, the CPU side stays in
    /// [`crate::stepper::CpuStepper`] (per-instruction) and the
    /// scheduler just keeps the PPU advanced at the natural ratio.
    pub fn emulate_frame_with_cpu<B, F, G>(
        &mut self,
        state: &mut PpuState,
        bus: &mut B,
        mut run_one_instr: F,
        mut on_nmi: G,
    ) -> i32
    where
        B: PpuBus + ?Sized,
        F: FnMut() -> u8,
        G: FnMut(),
    {
        self.begin_frame();
        // Total PPU dots in a frame = frame_cycles * 3.
        let total_dots = self.frame_cycles * PPU_DOTS_PER_CPU_CYCLE;
        let mut nmi_pending = false;
        // Track CPU cycles consumed to know when to call
        // `run_one_instr` next. We aim for 1 instruction per ~3 PPU
        // dots; the actual cycle count of the instruction determines
        // how many dots we credit.
        while self.ppu_dots_consumed < total_dots {
            // Advance PPU dots until the next CPU instruction boundary.
            // We advance `target_dots - current_dots` where
            // `target_dots = cpu_cycles * 3`.
            let target_dots = self.cpu_cycles_consumed * PPU_DOTS_PER_CPU_CYCLE;
            while self.ppu_dots_consumed < target_dots
                && self.ppu_dots_consumed < total_dots
            {
                let outcome = self.tick_one_ppu_dot(state, bus);
                if outcome.nmi_asserted {
                    nmi_pending = true;
                }
                // OAM DMA pump: one byte per CPU cycle. With
                // `target_dots` incrementing in 3-dot steps, the DMA
                // pump fires once every 3 PPU dots. We piggy-back on
                // the natural CPU cycle boundary.
                if state.oam_dma_pending && self.ppu_dots_consumed % PPU_DOTS_PER_CPU_CYCLE == 0 {
                    state.tick_oam_dma(bus);
                }
            }
            // Run one CPU instruction. The closure reports the
            // cycles consumed; we update the credit so the next
            // iteration advances the PPU accordingly.
            let cycles = run_one_instr();
            if cycles == 0 {
                // No instruction completed (e.g. cycle budget
                // exhausted). Break to avoid an infinite loop.
                break;
            }
            self.cpu_cycles_consumed = self.cpu_cycles_consumed.saturating_add(cycles as u32);
            // Drain a pending NMI now that the CPU has reached its
            // next instruction boundary.
            if nmi_pending {
                on_nmi();
                nmi_pending = false;
            }
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::FlatBus;
    use crate::registers::{ctrl_bits, mask_bits};
    use crate::state::DOTS_PER_SCANLINE;

    /// Basic sanity check: `tick_one_ppu_dot` advances dot by 1 and
    /// fires the scanline hook on transitions.
    #[test]
    fn tick_one_ppu_dot_advances_and_fires_scanline() {
        let mut sched = NesScheduler::new();
        let mut ppu = PpuState::new();
        let mut bus = FlatBus::new();
        // Phase 6.1.e follow-up (VBL-first layout): cold start is
        // (sl 241, dot 0); scheduler's initial last_scanline is -1
        // so the first tick fires notify_scanline(241).
        assert_eq!((ppu.scanline, ppu.dot), (241, 0));

        let _ = sched.tick_one_ppu_dot(&mut ppu, &mut bus);
        assert_eq!((ppu.scanline, ppu.dot), (241, 1));
        assert_eq!(sched.ppu_dots_consumed, 1);
        assert_eq!(sched.last_scanline, 241);
    }

    /// Drive to sl 241 dot 1 and confirm the scheduler fires
    /// `notify_vblank` on the VBL transition.
    #[test]
    fn vblank_transition_fires_notify_vblank() {
        let mut sched = NesScheduler::new();
        let mut ppu = PpuState::new();
        // Enable NMI so the state machine asserts NMI at sl 241 dot 1.
        ppu.registers.write_ctrl(1 << ctrl_bits::NMI_ENABLE);
        let mut bus = FlatBus::new();

        // Phase 6.1.e follow-up (VBL-first layout): cold start is
        // (sl 241, dot 0). VBL fires at the (sl 241, dot 1) tick —
        // 2 ticks from start. The ppudead=0 path (normal frame VBL
        // set) is what we test; the natural sl 241 dot 1 VBL set
        // event runs with `state.ppudead == 0` (PpuState::new()
        // initialises ppudead=1, so explicitly clear it).
        ppu.ppudead = 0;
        // Walk 2 dots to land at (sl 241, dot 2).
        for _ in 0..2 {
            let _ = sched.tick_one_ppu_dot(&mut ppu, &mut bus);
        }
        assert_eq!(ppu.scanline, 241);
        assert_eq!(ppu.dot, 2, "should be 2 ticks past (sl 241, dot 1)");
        assert!(sched.vbl_asserted, "vbl_asserted must be true after sl 241 dot 1");
    }

    /// Rendering on + visible scanline: the scheduler itself must NOT
    /// fire any hook at dot 256 anymore. v2.1.3 batch 1 replaced the
    /// v2.1.2 per-scanline MMC3 approximation (notify_hblank +
    /// notify_a12_rising at dot 256) with the filtered fetch-address
    /// watcher inside `tick_dot`; routing both would double-clock MMC3
    /// counters. (Kept as a behaviour record of the retirement.)
    #[test]
    fn scheduler_no_longer_fires_hblank_at_dot_256() {
        let mut sched = NesScheduler::new();
        let mut ppu = PpuState::new();
        ppu.registers.write_mask(1 << mask_bits::SHOW_BG);
        let mut bus = HookCountBus::new();

        // Cold start is (sl 241, dot 0); drive into sl 0 and through
        // dot 256 of the first visible scanline.
        let target_dots = 21 * DOTS_PER_SCANLINE as u32 + 256 - 1;
        for _ in 0..target_dots {
            let _ = sched.tick_one_ppu_dot(&mut ppu, &mut bus);
        }
        assert_eq!(ppu.scanline, 0);
        assert_eq!(ppu.dot, 256);
        assert_eq!(bus.hblank, 0, "dot-256 notify_hblank is retired");
        assert_eq!(bus.a12, 0, "dot-256 notify_a12_rising is retired");
    }

    /// v2.1.3 batch 1: the MMC3-family IRQ clock is the FILTERED A12
    /// rising edge. With rendering off there are no fetches and no
    /// CPU-driven address changes, so no edges fire; with rendering on
    /// and the standard CHR configuration ($2000 = $08: BG $0000,
    /// sprites $1000, empty secondary OAM → dummy $FF fetches from
    /// $1xxx) exactly one edge lands per fetch line — 241 per frame
    /// (pre-render + 240 visible), the blargg 2.Details subtest 7
    /// anchor. (This replaces the v2.1.2
    /// `mapper_irq_clock_requires_rendering_enabled` hblank test.)
    #[test]
    fn a12_edges_require_rendering_and_count_241_per_frame() {
        let frame_dots = DOTS_PER_SCANLINE as u32 * 262;

        let mut sched = NesScheduler::new();
        let mut ppu = PpuState::new();
        ppu.registers.write_mask(0);
        let mut bus = HookCountBus::new();
        for _ in 0..frame_dots {
            let _ = sched.tick_one_ppu_dot(&mut ppu, &mut bus);
        }
        assert_eq!(bus.a12, 0, "rendering off must not produce A12 edges");

        let mut sched = NesScheduler::new();
        let mut ppu = PpuState::new();
        ppu.registers.write_ctrl(0x08); // sprite pattern table $1000
        ppu.registers.write_mask(0x18); // BG + sprites on
        let mut bus = HookCountBus::new();
        for _ in 0..2 * frame_dots {
            let _ = sched.tick_one_ppu_dot(&mut ppu, &mut bus);
        }
        // 241 fetch lines per frame (pre-render + 240 visible) × 2
        // frames; the even-frame dot skip shortens frame 1 without
        // removing its pre-render line.
        assert_eq!(bus.a12, 482, "one filtered A12 edge per fetch line");
    }

    struct HookCountBus {
        inner: FlatBus,
        hblank: u32,
        a12: u32,
    }

    impl HookCountBus {
        fn new() -> Self {
            Self { inner: FlatBus::new(), hblank: 0, a12: 0 }
        }
    }

    impl PpuBus for HookCountBus {
        fn read(&mut self, addr: u16) -> u8 { self.inner.read(addr) }
        fn write(&mut self, addr: u16, val: u8) { self.inner.write(addr, val) }
        fn peek_chr(&mut self, addr: u16) -> u8 { self.inner.peek_chr(addr) }
        fn notify_hblank(&mut self) { self.hblank += 1; }
        fn notify_a12_rising(&mut self) { self.a12 += 1; }
    }

    /// OAM DMA pump transfers one byte per CPU cycle (= 3 PPU dots).
    #[test]
    fn oam_dma_pump_transfers_one_byte_per_cycle() {
        let mut ppu = PpuState::new();
        let mut bus = FlatBus::new();
        // Fill CPU page 0x02 with sentinel values.
        bus.fill_cpu_page(0x02, 0x40);
        ppu.begin_oam_dma(0x02);

        // Pump 256 bytes one at a time.
        for i in 0..256 {
            assert!(ppu.tick_oam_dma(&mut bus), "DMA should be pending");
            assert_eq!(ppu.oam[i as usize], 0x40u8.wrapping_add(i as u8));
        }
        // 257th call should report no DMA pending.
        assert!(!ppu.tick_oam_dma(&mut bus), "DMA should be complete");
        assert!(!ppu.oam_dma_pending);
        assert_eq!(ppu.oam_dma_counter, 0);
        // oam_addr reset to 0 after DMA completion.
        assert_eq!(ppu.registers.oam_addr, 0);
    }
}
