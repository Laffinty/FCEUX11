//! PPU module — 2C02 (newppu=1) scanline-segment driven core.
//!
//! Phase 3 deliverable per `docs/wip_2.0_plan/phase_3_ppu.md`. This is
//! the heaviest phase; it implements the PPU master clock + CPU budget
//! timing model (decision A) used by FCEUX since time immemorial.
//!
//! # Submodules
//!
//! - [`mod@registers`] — $2000-$2007 + v/t/x/w internal registers
//! - [`mod@dot_clock`] — 341×262 timing constants
//! - [`mod@background`] — Background fetch + shifter pipeline
//! - [`mod@sprite`] — OAM evaluation + sprite rendering
//! - [`mod@oam`] — Primary + secondary OAM
//! - [`mod@sprite_lut`] — 512 KiB sprite LUT (`LazyLock`)
//! - [`mod@tile_fetch`] — Templated tile fetchers
//! - [`mod@compositing`] — BG+sprite multiplexing + sprite 0 hit
//! - [`mod@nmi`] — VBlank NMI dispatch
//! - [`mod@idle`] — NSF "no PPU" stub (`system_type=NSF`)
//!
//! # Segment-driven execution (decision A, ADR-008)
//!
//! The PPU master produces **scanline segments** to the `Scheduler`:
//! each segment carries a CPU cycle budget that the scheduler hands to
//! the CPU.  Within a segment, the PPU advances dot-by-dot.  This
//! mirrors `FCEUPPU_Loop` exactly — segment boundaries are CPU budget
//! boundaries, which is the natural shadow-run compare point.

pub mod background;
pub mod compositing;
pub mod dot_clock;
pub mod idle;
pub mod nametable;
pub mod nmi;
pub mod oam;
pub mod registers;
pub mod sprite;
pub mod sprite_lut;
pub mod tile_fetch;

use crate::ppu::background::BackgroundRenderer;

pub use background::BackgroundState;
pub use compositing::Compositor;
pub use dot_clock::{DOTS_PER_SCANLINE, PRELINE, SCANLINES_PER_FRAME};
pub use nmi::NmiController;
pub use oam::OamState;
pub use registers::{PpuRegisters, VramAddr};
pub use sprite::SpriteState;

use crate::mapper::MapperRangeTable;

/// Scanline segment produced by `PpuCore::next_segment`.
///
/// Each segment carries a CPU cycle budget. The scheduler calls
/// `cpu.run_budget(budget)` between segments, matching `X6502_Run`
/// semantics in the C++ code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    /// Visible scanline, CPU gets 256 cycles (background rendered).
    Visible { cpu_budget: i32 },
    /// Sprite evaluation + render at end of scanline, CPU gets 69 cycles.
    SpriteEval { cpu_budget: i32 },
    /// Pre-render scanline (scanline -1).
    PreRender { cpu_budget: i32 },
    /// VBlank interval (scanline 241+), CPU gets a long idle budget.
    VBlank { cpu_budget: i32 },
    /// NSF "no PPU" idle (system_type=NSF).
    Idle { cpu_budget: i32 },
    /// Frame complete.
    FrameComplete,
}

impl Segment {
    pub fn cpu_budget(self) -> i32 {
        match self {
            Self::Visible { cpu_budget } => cpu_budget,
            Self::SpriteEval { cpu_budget } => cpu_budget,
            Self::PreRender { cpu_budget } => cpu_budget,
            Self::VBlank { cpu_budget } => cpu_budget,
            Self::Idle { cpu_budget } => cpu_budget,
            Self::FrameComplete => 0,
        }
    }
}

/// Top-level PPU state (2C02 equivalent).
///
/// Phase 3 implements **newppu=1** path only.  Old PPU
/// (`ppu.cpp` legacy path) is preserved in C++ as a movie-compat
/// fallback per ADR-009 (decision ③).
#[derive(Debug)]
pub struct PpuCore {
    // ===== Frame timing =====
    /// Current scanline.  Pre-render = -1; visible = 0..=239; vblank = 240+.
    pub scanline: i16,
    /// Current dot within scanline (0..=340).
    pub dot: u16,
    /// Even/odd frame toggle for odd-frame skip-dot.
    pub odd_frame: bool,
    /// Whether the system is NTSC, PAL, or Dendy. Phase 3 hardcodes NTSC.
    pub timing: TimingMode,
    /// NSF "no PPU" mode.
    pub idle: bool,

    // ===== Registers ($2000-$2007) =====
    pub regs: PpuRegisters,

    // ===== Rendering pipeline =====
    /// Background pipeline (shifters + next-tile latches).
    pub bg: BackgroundState,
    /// Sprite state (secondary OAM + shifters + position counters).
    pub spr: SpriteState,
    /// Primary + secondary OAM.
    pub oam: OamState,
    /// Output compositor (multiplexes BG + sprites).
    pub compositor: Compositor,
    /// VBlank NMI dispatch.
    pub nmi: NmiController,

    // ===== Output =====
    /// 256×240 = 61440-byte frame buffer (palette indices).
    pub frame_buffer: Box<[u8; 61440]>,
    /// Current frame is complete and ready for read-out.
    pub frame_ready: bool,
    /// Address of mapper CHR access (so the bus layer can route).
    pub mapper: MapperRangeTable,

    // ===== CHR / nametable caches (Phase 3 (a)) =====
    //
    // These caches hold the data that the background renderer reads
    // each scanline.  Phase 5 mapper integration will populate them
    // via FFI; for now, tests fill them directly via the public
    // fields to verify the rendering pipeline.
    //
    /// Active nametable visible portion (32 cols × 30 rows = 960
    /// bytes).  In production, populated from `vram[base..base+0x3C0]`
    /// where `base = regs.nametable_base()`.
    pub nametable: Box<[u8; 960]>,
    /// Active attribute table (8×8 quadrants = 64 bytes).  In
    /// production, populated from `vram[base + 0x3C0..base + 0x400]`.
    pub attribute: Box<[u8; 64]>,
    /// Both background pattern tables, low plane (8 KiB).  Index:
    /// `(tile_id * 16) + row` for the low byte of the pixel pair.
    pub pattern_lo: Box<[u8; 8192]>,
    /// Both background pattern tables, high plane (8 KiB).  Index:
    /// `(tile_id * 16) + row` for the high byte.
    pub pattern_hi: Box<[u8; 8192]>,
    /// Coarse X scroll (5 bits, 0..=31).
    pub scroll_coarse_x: u8,
    /// Coarse Y scroll (5 bits, 0..=31).
    pub scroll_coarse_y: u8,
    /// Fine X scroll (3 bits, 0..=7).
    pub fine_x: u8,

    // ===== Counters =====
    /// Cycles consumed since last `run_frame` entry (segment-internal).
    pub cycles_this_segment: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingMode {
    Ntsc,
    Pal,
    Dendy,
}

impl Default for TimingMode {
    fn default() -> Self {
        Self::Ntsc
    }
}

impl Default for PpuCore {
    fn default() -> Self {
        Self::new()
    }
}

impl PpuCore {
    pub fn new() -> Self {
        Self {
            scanline: PRELINE,
            dot: 0,
            odd_frame: false,
            timing: TimingMode::Ntsc,
            idle: false,

            regs: PpuRegisters::new(),

            bg: BackgroundState::new(),
            spr: SpriteState::new(),
            oam: OamState::new(),
            compositor: Compositor::new(),
            nmi: NmiController::new(),

            frame_buffer: Box::new([0u8; 61440]),
            frame_ready: false,
            mapper: MapperRangeTable::default(),

            nametable: Box::new([0u8; 960]),
            attribute: Box::new([0u8; 64]),
            pattern_lo: Box::new([0u8; 8192]),
            pattern_hi: Box::new([0u8; 8192]),
            scroll_coarse_x: 0,
            scroll_coarse_y: 0,
            fine_x: 0,

            cycles_this_segment: 0,
        }
    }

    /// Produce the next scanline segment.  The scheduler consumes the
    /// CPU budget and then calls `advance_to_next_segment()` to tick
    /// the PPU's dot clock forward by the segment's worth of dots.
    ///
    /// Per-scanline total budget (must match `src/ppu_rendering.cpp::DoLine`):
    /// - Visible scanline (sl < 240): **341** = 256 (visible dots) +
    ///   6 + 63 + 16 (sprite eval + hblank). Emitted as two segments
    ///   (`Visible` 256, `SpriteEval` 85) so the scheduler can render
    ///   background before sprite composition (mirrors C++ order).
    /// - Post-render + VBlank scanlines (sl >= 240): **341** = 256+69+16
    ///   (mirrors C++ DoLine branch `if (sl >= 240)`).
    pub fn next_segment(&mut self) -> Segment {
        if self.idle {
            // NSF "no PPU" mode — produce one big idle budget per
            // "scanline" worth of cycles, frame complete after 262 of
            // them.  This keeps the scheduler logic identical.
            return Segment::Idle { cpu_budget: 113 }; // ~341/3 dots per CPU cycle
        }

        match self.scanline {
            sl if sl == PRELINE => Segment::PreRender {
                // **Phase 6 P2 shadow fix (2026-08-12, third edition)**:
                // budget 0 — this scanline is a Rust-internal artifact
                // (C++ FCEUPPU_Loop starts at sl=0 and has no separate
                // pre-render scanline; its pre-render is sl=261, which
                // already gets the full 341 budget below). Giving it a
                // nonzero budget added ~85 CPU cycles/frame that C++
                // never runs, drifting the APU frame counter phase and
                // misaligning the frame-counter IRQ. The pre-render
                // PPU work (clear VBlank etc.) still runs in
                // `tick_preline_segment`; only the CPU budget is 0.
                cpu_budget: 0,
            },
            sl if (0..=239).contains(&sl) => {
                // Visible scanline: 256 visible dots (background fetch
                // + render). Sprite eval / hblank follows as a second
                // segment below — see `sprite_phase`.
                Segment::Visible {
                    cpu_budget: dot_clock::CPU_BUDGET_VISIBLE,
                }
            }
            sl if sl == 240 => Segment::VBlank {
                cpu_budget: dot_clock::CPU_BUDGET_VBLANK_LINE,
            },
            sl if sl == dot_clock::VBLANK_SCANLINE => Segment::VBlank {
                // **Phase 6 P2 shadow fix (2026-08-12)**: was `1` here
                // which left the CPU with only 1 cycle to enter the NMI
                // handler. C++ DoLine gives scanline 241 a full 341
                // budget (256+69+16), so the NMI entry sequence (7 cycles
                // for push + vector) completes cleanly.  Without this
                // fix the shadow PC drifts ~3 cycles/frame and diverges
                // after frame 3.
                cpu_budget: dot_clock::CPU_BUDGET_VBLANK_LINE,
            },
            sl if (242..=260).contains(&sl) => Segment::VBlank {
                // **Phase 6 P2 shadow fix (2026-08-12, third edition)**:
                // was CPU_BUDGET_GB_HBLANK (85) here. C++ DoLine's
                // `sl >= 240` branch gives EVERY post-render / VBlank
                // scanline a full 341 budget (X6502_Run(256+69) +
                // X6502_Run(16)); Rust was under-budgeting these 19
                // idle VBlank lines by 256 cycles each. Net effect:
                // the Rust CPU ran ~5,120 fewer cycles per frame than
                // C++, so the APU frame counter (ticked per instruction)
                // drifted ~5k cycles per frame → the frame-counter IRQ
                // fired at a different phase → shadow CPU divergence
                // from frame 3 onward even with full state sync.
                cpu_budget: dot_clock::CPU_BUDGET_VBLANK_LINE,
            },
            sl if sl == 261 => Segment::PreRender {
                // **Phase 6 P2 shadow fix (2026-08-12, third edition)**:
                // was CPU_BUDGET_GB_HBLANK (85). The pre-render scanline
                // is `sl >= 240` in C++ DoLine, so it also gets the full
                // 341 budget.
                cpu_budget: dot_clock::CPU_BUDGET_VBLANK_LINE,
            },
            _ => Segment::FrameComplete,
        }
    }

    /// Emit a follow-up sprite-eval / hblank segment after the visible
    /// segment for scanlines 0..=239. Returns `None` if no follow-up is
    /// needed (i.e. scanline has advanced past 239). Called by the
    /// scheduler **after** `advance_to_next_segment()` runs the
    /// per-segment render — keeping the sprite composition on the
    /// correct scanline so CHR patterns land at the right dots.
    ///
    /// **Phase 6 P2 shadow fix (2026-08-12)**: previously the visible
    /// scanline emitted a single 256-budget segment, leaving the CPU
    /// 85 cycles short per scanline vs C++ (256 + 6 + 63 + 16 = 341).
    /// Over 240 scanlines that was a ~20k-cycle deficit per frame and
    /// the CPU drifted significantly behind.
    pub fn sprite_eval_segment(&mut self) -> Option<Segment> {
        if self.idle {
            return None;
        }
        // Only meaningful for scanlines that have just finished the
        // visible (256-cycle) segment. The scheduler bumps scanline
        // inside `advance_to_next_segment()`, so by the time this is
        // called the scanline counter is already +1.
        let just_finished = self.scanline - 1;
        if (0..=239).contains(&just_finished) {
            // 85 = 6 (start HBLANK) + 63 (main hblank) + 16 (final
            // sprite eval slot) — matches ppu_rendering.cpp::DoLine.
            Some(Segment::SpriteEval {
                cpu_budget: dot_clock::CPU_BUDGET_HBLANK_TOTAL,
            })
        } else {
            None
        }
    }

    /// Advance the PPU dot clock after the scheduler consumed the
    /// segment's CPU budget.  Performs the per-segment rendering work
    /// (background fetch, sprite eval, etc.).
    ///
    /// Returns `true` if a full frame just completed.
    pub fn advance_to_next_segment(&mut self) -> bool {
        // Step the dot clock forward by one full segment's worth of
        // dots.  Each segment is "341 dots" worth of PPU cycles.
        let dots_per_segment: u16 = DOTS_PER_SCANLINE;

        if self.idle {
            // NSF idle: just advance dot clock; no rendering.
            self.dot += dots_per_segment;
            if self.dot >= DOTS_PER_SCANLINE {
                self.dot = 0;
                self.scanline += 1;
                if self.scanline >= SCANLINES_PER_FRAME - 1 {
                    self.scanline = PRELINE;
                    self.odd_frame = !self.odd_frame;
                    self.frame_ready = true;
                    return true;
                }
            }
            return false;
        }

        // Per-segment work:
        match self.scanline {
            sl if sl == PRELINE => self.tick_preline_segment(),
            sl if (0..=239).contains(&sl) => self.tick_visible_segment(),
            sl if sl == 240 => self.tick_postline_segment(),
            sl if sl == dot_clock::VBLANK_SCANLINE => self.tick_vblank_set_segment(),
            sl if (242..=260).contains(&sl) => {} // idle
            sl if sl == 261 => self.tick_prerender_segment(),
            _ => {}
        }

        // Phase 6 P2 shadow fix (2026-08-12): the PRELINE scanline is
        // a Rust-internal extra (C++ FCEUPPU_Loop starts at sl=0; its
        // pre-render is sl=261, which advances 341 dots like every
        // other scanline). Giving PRELINE its own 341-dot advance made
        // the Rust frame 263×341 dots vs C++'s 262×341, shifting the
        // VBlank-set dot by one scanline → the $2002 wait loop exited
        // one iteration later → ~2 extra instructions + ~7 cycles of
        // frame-counter phase drift per frame. Do the preline PPU work
        // (clear VBlank etc.) but do NOT advance the dot clock, keeping
        // the frame at exactly 262×341 dots.
        if self.scanline == PRELINE {
            self.scanline = 0;
            return false;
        }

        // Advance to next scanline.
        self.dot = self.dot.wrapping_add(dots_per_segment);
        self.scanline += 1;

        // Odd-frame skip dot at end of pre-render (scanline 261 → -1).
        if self.scanline > 261 {
            self.scanline = PRELINE;
            self.odd_frame = !self.odd_frame;
            if self.odd_frame && self.bg_rendering_enabled() {
                // Skip one dot on odd frames (the famous 5/341 NES
                // quirk).  In the budget model, this is "the next
                // visible segment's 256-cycle budget starts one dot
                // earlier".
                self.dot = 1;
            }
            self.frame_ready = true;
            return true;
        }

        false
    }

    /// Whether the background should be rendered (PPUCTRL + PPUMASK bits).
    fn bg_rendering_enabled(&self) -> bool {
        // $2001 bits 3 (BG enable) + 4 (sprites enable).
        (self.regs.ppumask & 0x18) != 0
    }

    fn tick_preline_segment(&mut self) {
        // Pre-render (scanline -1): clear VBlank, sprite 0 hit, sprite
        // overflow. Reset v/t from t.
        self.regs.status &= !0x80; // clear VBlank
        self.regs.status &= !0x20; // clear sprite overflow
        self.regs.status &= !0x40; // clear sprite 0 hit
        // v ← t (PPUADDR is latched at end of pre-render).
        // In the newppu model this happens at dot 257 of pre-render;
        // for the segment-driven abstraction we just do it at the
        // start of the segment.
        // Source: src/ppu.cpp / ppu_rendering.cpp (PPU_address_latch).
        if self.dot >= 257 {
            self.regs.v.copy_from(&self.regs.t);
        }
    }

    fn tick_visible_segment(&mut self) {
        // Background: render pixels into frame_buffer for this scanline.
        // Phase 3 (a) — the per-pixel pipeline is now wired up.
        if (self.regs.ppumask & 0x08) != 0 {
            self.render_background_scanline();
        }
        // Sprites: compose on top of the background at end of scanline.
        // Phase 5 stage 0 — real CHR pattern fetch + frame_buffer write
        // (sprite.rs reads the PpuCore CHR caches).
        if (self.regs.ppumask & 0x10) != 0 {
            self.spr.render_scanline(
                self.scanline as u8,
                &mut self.frame_buffer,
                &self.oam,
                self.regs.ppuctrl,
                self.regs.ppumask,
                &self.pattern_lo,
                &self.pattern_hi,
                &mut self.compositor,
            );
        }
    }

    /// Render the background into the frame buffer for the current
    /// scanline.  Uses `BackgroundRenderer::render_line` with the
    /// PpuCore's CHR / nametable caches + scroll registers.
    ///
    /// This is the Phase 3 (a) hook-up: the rendering pipeline now
    /// produces real pixel output (not placeholder zeros).
    fn render_background_scanline(&mut self) {
        let scanline = self.scanline as u8;
        let target_offset = (scanline as usize) * 256;
        // Coarse Y for the current scanline, taking the per-tile row
        // into account (every 8 scanlines = 1 tile row).
        let effective_coarse_y = self.scroll_coarse_y.wrapping_add(scanline >> 3);
        let row_in_tile = scanline & 0x07;
        // Output buffer slice for this scanline (256 bytes).
        let output = &mut self.frame_buffer[target_offset..target_offset + 256];

        BackgroundRenderer::render_line(
            &*self.nametable,
            &*self.attribute,
            &*self.pattern_lo,
            &*self.pattern_hi,
            self.scroll_coarse_x,
            effective_coarse_y,
            self.fine_x,
            row_in_tile,
            self.regs.bg_pattern_base(),
            output,
        );
    }

    fn tick_postline_segment(&mut self) {
        // Scanline 240: post-render, idle. No rendering, just frame
        // boundary work if needed.
    }

    fn tick_vblank_set_segment(&mut self) {
        // Scanline 241, dot 1: set VBlank flag, trigger NMI if enabled.
        self.regs.status |= 0x80; // set VBlank
        if (self.regs.ppuctrl & 0x80) != 0 {
            // NMI enabled.
            self.nmi.arm(self.scanline, self.dot);
        }
    }

    fn tick_prerender_segment(&mut self) {
        // Scanline 261 (last pre-render of next frame): same as preline.
        self.regs.status &= !0x80;
        self.regs.status &= !0x20;
        self.regs.status &= !0x40;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_ppu_starts_at_preline() {
        let p = PpuCore::new();
        assert_eq!(p.scanline, PRELINE);
        assert_eq!(p.scanline, -1);
        assert_eq!(p.dot, 0);
        assert!(!p.frame_ready);
        assert!(!p.odd_frame);
        assert!(!p.idle);
    }

    #[test]
    fn segment_sequence_through_visible() {
        let mut p = PpuCore::new();
        // Pre-render (scanline -1) is first.
        let s = p.next_segment();
        match s {
            Segment::PreRender { .. } => {}
            _ => panic!("expected PreRender at scanline -1, got {:?}", s),
        }
        // Advance to scanline 0.
        p.advance_to_next_segment();
        // Now visible.
        assert!(matches!(p.next_segment(), Segment::Visible { .. }));
        let s = p.next_segment();
        match s {
            Segment::Visible { cpu_budget } => {
                assert_eq!(cpu_budget, dot_clock::CPU_BUDGET_VISIBLE);
            }
            _ => panic!("expected Visible, got {:?}", s),
        }
    }

    #[test]
    fn segment_vblank_at_241() {
        let mut p = PpuCore::new();
        p.scanline = dot_clock::VBLANK_SCANLINE;
        let s = p.next_segment();
        match s {
            Segment::VBlank { cpu_budget } => {
                // VBlank-set scanline budget (Phase 6 P2 shadow fix):
                // matches C++ DoLine X6502_Run(256+69+16) = 341 so the
                // NMI handler entry (7 cycles for push + vector) fits.
                assert_eq!(cpu_budget, dot_clock::CPU_BUDGET_VBLANK_LINE);
            }
            _ => panic!("expected VBlank, got {:?}", s),
        }
    }

    #[test]
    fn idle_segments_skip_rendering() {
        let mut p = PpuCore::new();
        p.idle = true;
        for _ in 0..5 {
            let s = p.next_segment();
            assert!(matches!(s, Segment::Idle { .. }), "got {:?}", s);
            p.advance_to_next_segment();
        }
    }

    #[test]
    fn advance_completes_frame_at_263() {
        let mut p = PpuCore::new();
        // The frame spans scanlines -1..=261 inclusive (263 scanlines).
        // After advancing 263 times, we should be back at PRELINE.
        for _ in 0..263 {
            let _ = p.next_segment();
            p.advance_to_next_segment();
        }
        assert_eq!(p.scanline, PRELINE);
        assert!(p.frame_ready);
    }

    #[test]
    fn vblank_set_at_241_triggers_nmi_when_enabled() {
        let mut p = PpuCore::new();
        p.scanline = dot_clock::VBLANK_SCANLINE;
        p.regs.ppuctrl = 0x80; // NMI enabled
        let _ = p.next_segment();
        p.advance_to_next_segment();
        assert_eq!(p.regs.status & 0x80, 0x80, "VBlank flag must be set");
        assert!(p.nmi.pending(), "NMI must be pending");
    }

    #[test]
    fn vblank_no_nmi_when_disabled() {
        let mut p = PpuCore::new();
        p.scanline = dot_clock::VBLANK_SCANLINE;
        p.regs.ppuctrl = 0x00; // NMI disabled
        let _ = p.next_segment();
        p.advance_to_next_segment();
        assert_eq!(p.regs.status & 0x80, 0x80, "VBlank flag must be set");
        assert!(!p.nmi.pending(), "NMI must NOT be pending");
    }
}