//! `PpuState` aggregate — registers + OAM + secondary OAM + frame counters.
//!
//! The state machine in [`crate::frame::tick_dot`] advances the dot/scanline
//! counters and dispatches register / OAM / scroll events. The
//! [`Registers`] sub-struct handles the value-level side effects; this
//! struct layers on top:
//! - OAM storage (256 bytes primary, 32 bytes secondary, plus a flag
//!   for the 8-sprite overflow).
//! - Frame counters (`frame` increments every time `scanline == 0 && dot == 0`
//!   rolls over).
//! - Sprite-0 hit latch.
//! - The "vbl suppression this frame" flag, set by `$2002` reads at
//!   sl 241 dot 0 and consumed by the frame state machine.
//! - The odd-frame flag (toggles every pre-render to decide even/odd skip).

use crate::bus::PpuBus;
#[allow(unused_imports)] // status_bits used only by #[cfg(test)] modules.
use crate::registers::{Registers, ctrl_bits, mask_bits, status_bits};

/// 256-byte primary OAM (chip RAM, accessed via `$2004`).
pub const OAM_SIZE: usize = 0x100;
/// 32-byte secondary OAM (8 sprites × 4 bytes).
pub const SECONDARY_OAM_SIZE: usize = 0x20;
/// Maximum sprites per scanline.
pub const MAX_SPRITES_PER_LINE: usize = 8;
/// NTSC scanlines per frame. The state machine indexes the pre-render
/// line as `-1` (hardware scanline 261) and visible/post/VBL lines as
/// `0..=260` — 262 scanlines × 341 dots = 89342 dots per frame, matching
/// the C++ `Cpu::run` frame budget.
///
/// Frame visit sequence: Phase 6.1.e follow-up (§6.4.3, VBL-block-phase
/// alignment) made the layout VBL-first to match the C++ engine:
/// `[241..=260, -1, 0..=240]` (262 scanlines). The wrap fires from sl
/// 240 to sl 241 (`next_sl == 241` — the unique pre-image is sl 240).
/// Phase 5.1 had set the wrap to fire from sl 260 to sl -1
/// (`next_sl >= NTSC_SCANLINES - 1` = 261), which is the pre-render-first
/// layout. The sl index numbers don't change between layouts; only the
/// order in which they're visited changes.
pub const NTSC_SCANLINES: i16 = 262;
/// PPU dots per scanline (visible 256 + hblank 85 = 341, indexed 0..=340).
pub const DOTS_PER_SCANLINE: u16 = 341;

/// Sentinel for [`PpuState::sprite0_hit_dot`] — no sprite 0 hit is
/// pending on the current scanline.
pub const NO_SPRITE0_HIT_DOT: u16 = 0xFFFF;

/// PPU state aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpuState {
    /// `$2000`-`$2007`, `$4014`, scroll latches, open-bus buffer.
    pub registers: Registers,
    /// Primary OAM — 256 bytes.
    pub oam: [u8; OAM_SIZE],
    /// Secondary OAM — 32 bytes (8 sprites × 4 bytes per sprite).
    pub secondary_oam: [u8; SECONDARY_OAM_SIZE],
    /// Number of sprites in `secondary_oam` (0..=8).
    pub secondary_oam_count: u8,
    /// Sprite overflow during current scanline eval (drives PPU[2] bit 5
    /// at the right dot in Phase 4).
    pub sprite_overflow: bool,
    /// Sprite 0 hit latch — set when sprite 0 overlaps non-transparent BG.
    pub sprite0_hit: bool,
    /// Phase 6.6 (Session A): the dot on the current scanline at which
    /// the sprite 0 hit should latch (pixel `x` outputs at dot `x + 1`),
    /// recorded by the batch sprite render. The frame state machine
    /// sets `sprite0_hit` when it reaches this dot. Transient —
    /// recomputed every visible scanline, cleared at the pre-render
    /// line, NOT part of the RPU1 payload (pinned to
    /// [`NO_SPRITE0_HIT_DOT`] on load, like `ppudead`).
    pub sprite0_hit_dot: u16,
    /// Current scanline. Phase 6.1.e follow-up (VBL-first layout): the
    /// frame visit sequence is `[241..=260, -1, 0..=240]` so `scanline`
    /// still indexes pre-render as `-1` and visible as `0..=239`, but
    /// the frame start is `(241, 0)` (see `PpuState::new`) and the
    /// frame wrap goes `sl 240 → sl 241` instead of `sl 260 → sl -1`.
    pub scanline: i16,
    /// Current dot within scanline: 0..=340.
    pub dot: u16,
    /// Frame counter — wraps only by convention (u64 is plenty).
    pub frame: u64,
    /// Set by `$2002` read at sl 241 dot 0; consulted by frame state
    /// machine to suppress VBL flag set + NMI for this frame.
    pub vbl_suppressed_this_frame: bool,
    /// Phase 5.1: NMI line latch. Set by the per-dot driver when the
    /// frame state machine asserts NMI (sl 241 dot 1 with NMI enabled
    /// and no VBL suppression); consumed (taken-and-cleared) by the
    /// C++ bridge via `fceux11_ppu_take_nmi_pending`, which forwards
    /// it to the canonical C++ `TriggerNMI()` latch. Without this the
    /// PPU NMI never reaches the CPU and any ROM that waits for a VBL
    /// interrupt (nestest's IRQ-wait at $C28F, blargg vbl tests) spins
    /// forever.
    pub nmi_pending: bool,
    /// Toggles every pre-render to drive even/odd skip.
    pub odd_frame: bool,
    /// Tracks whether the most recent CPU write was `$2005`/`$2006`
    /// (echoed into PPU[2] bits 4..=0 until next read of `$2002` or
    /// write to `$2004`/`$2007`). Mirrors `Registers::write_toggle` but
    /// kept here so the state machine can flip it without touching
    /// registers when the frame loop's BG fetch needs to.
    pub last_was_2005_or_2006: bool,
    /// Set when CPU issues `$4014`; consumed by the frame state machine
    /// to drive the 256-cycle DMA pump. Phase 3's
    /// [`crate::scheduler::NesScheduler`] drives the async pump one
    /// byte per CPU cycle; Phase 1/2's [`start_oam_dma`] performs the
    /// full 256-byte copy synchronously.
    pub oam_dma_pending: bool,
    /// OAM DMA source page (the byte written to `$4014`). Valid only
    /// when `oam_dma_pending` is true.
    pub oam_dma_page: u8,
    /// OAM DMA byte counter (0..=256). Each CPU cycle advances by 1;
    /// when it reaches 256 the DMA completes and `oam_dma_pending`
    /// clears.
    pub oam_dma_counter: u16,

    // -----------------------------------------------------------------
    // Phase 4: rendering state. Used by the per-tile fetch + per-pixel
    // output in `crate::rendering::render_scanline`. These fields are
    // pre-loaded at the start of each visible scanline and advanced
    // tile-by-tile during the visible region.
    // -----------------------------------------------------------------

    /// 16-bit shift registers for the two pattern bit planes (BG).
    /// `pshift[0]` = plane 0 (low bit), `pshift[1]` = plane 1 (high bit).
    /// Pre-loaded with two tiles' worth of pattern data at the start
    /// of each visible scanline. The render loop shifts left by 8
    /// each tile fetch and ORs in the newly fetched pattern bytes.
    pub bg_pshift: [u16; 2],
    /// 4-bit attribute latch for the current tile. Holds the palette
    /// quadrant (2 bits) replicated to bits 0..=3 of a 4-bit value.
    /// `atlatch` is shifted right by 2 every tile and ORed with the new
    /// quadrant's bits in the upper 2 bits.
    pub bg_atlatch: u8,
    /// Latches for the next tile (4 bytes: name table, attribute,
    /// pattern low, pattern high). Loaded by the per-tile fetch and
    /// committed to the shift registers + atlatch on the next tile.
    pub bg_next_nt: u8,
    /// Next attribute byte.
    pub bg_next_at: u8,
    /// Next pattern low byte (bit plane 0).
    pub bg_next_pattern_lo: u8,
    /// Next pattern high byte (bit plane 1).
    pub bg_next_pattern_hi: u8,
    /// True if at least one tile has been fetched since the start of
    /// the current visible scanline. The pre-load "tile -2" and
    /// "tile -1" populate the shift register with zero so the first
    /// 16 pixels use the zeros (rather than garbage from a previous
    /// scanline).
    pub bg_primed: bool,
    /// True if the rendering engine is in the visible part of a
    /// visible scanline (i.e. tile fetches are happening). Set by
    /// the dot driver at sl 0..239 dot 0, cleared at sl 240.
    pub bg_active: bool,
    /// Sprite pattern shift registers: 8 sprites × 2 planes × 8 bits.
    /// `sprite_shift[i][0]` = plane 0, `sprite_shift[i][1]` = plane 1.
    /// Each sprite has an 8-bit shift register, shifted left by 1
    /// each visible dot.
    pub sprite_shift: [[u8; 2]; 8],
    /// Sprite attribute latches: 8 sprites × 1 byte (palette + flags).
    /// Bit 5 of the attribute is the priority (0 = front, 1 = behind
    /// BG). Bits 0..1 are the palette quadrant.
    pub sprite_attr: [u8; 8],
    /// Sprite X position counters: 8 sprites × 1 byte. Decremented
    /// each visible dot. When the counter reaches 0, the sprite's
    /// pixels are output (until the next sprite or end of tile).
    pub sprite_x: [u8; 8],
    /// Sprite 0 is in range (set during sprite eval if sprite 0 is
    /// one of the 8 sprites for the current scanline). Used for
    /// sprite 0 hit detection.
    pub sprite0_in_range: bool,
    /// True if sprite eval already happened for the current scanline
    /// (so we don't re-evaluate every dot).
    pub sprite_eval_done: bool,
    /// Phase 6.4: C++ `ppudead` mirror (src/ppu.cpp:90). The C++ new
    /// PPU's FIRST frame after process start runs a different layout —
    /// VBL flag set at frame dot 0, VBL window = the first 20
    /// scanlines, no sl-241 set (ppu_rendering.cpp:1626-1655) — and
    /// blargg ROMs sync to 2 VBLs at init, so missing this frame shifts
    /// the whole test timeline by one VBL occurrence (trace-diff:
    /// first VBL-set read cycle 27516 Rust vs 29786 C++, §6.3.a.4
    /// follow-up). 1 only for the process's first frame: PpuState is
    /// created once per bridge init and reused across game loads, and
    /// `power()`/`reset()` must NOT re-set it (mirrors the C++ static,
    /// which Power/Reset don't touch either). Not part of the RPU1
    /// savestate payload; `read_payload` pins it to 0 (restores are
    /// always past boot).
    pub ppudead: u8,
}

impl Default for PpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl PpuState {
    /// Cold-state init — everything zero, scanline/dot at frame start.
    ///
    /// Phase 6.1.e follow-up (VBL-block-phase alignment, see
    /// `docs/history/v2.1_phase6_batch_compat.md` §6.4.3): the frame
    /// layout is now VBL-first to match the C++ engine's frame
    /// boundary at sl 240 → sl 241. The frame visit sequence is
    /// `[241..=260, -1, 0..=240]` (262 scanlines × 341 dots = 89342
    /// dots per frame, matching `kNtscCpuCyclesPerFrame`). Frame start
    /// sits at `(sl 241, dot 0)` so the VBL flag set event at
    /// `(sl 241, dot 1)` lands in dot 1 of the frame, matching the
    /// C++ `PPU_status |= 0x80; ppur.status.sl = 241; runppu(20 *
    /// kLineTime)` sequence.
    pub const fn new() -> Self {
        Self {
            registers: Registers::new(),
            oam: [0u8; OAM_SIZE],
            secondary_oam: [0u8; SECONDARY_OAM_SIZE],
            secondary_oam_count: 0,
            sprite_overflow: false,
            sprite0_hit: false,
            sprite0_hit_dot: NO_SPRITE0_HIT_DOT,
            scanline: 241,
            dot: 0,
            frame: 0,
            vbl_suppressed_this_frame: false,
            nmi_pending: false,
            odd_frame: false,
            last_was_2005_or_2006: false,
            oam_dma_pending: false,
            oam_dma_page: 0,
            oam_dma_counter: 0,
            bg_pshift: [0u16; 2],
            bg_atlatch: 0,
            bg_next_nt: 0,
            bg_next_at: 0,
            bg_next_pattern_lo: 0,
            bg_next_pattern_hi: 0,
            bg_primed: false,
            bg_active: false,
            sprite_shift: [[0u8; 2]; 8],
            sprite_attr: [0u8; 8],
            sprite_x: [0u8; 8],
            sprite0_in_range: false,
            sprite_eval_done: false,
            ppudead: 1,
        }
    }

    /// Power-on reset (cold). Clears OAM, secondary OAM, scanline, dot;
    /// resets registers. The frame counter does NOT reset (matches NES
    /// hardware, where the PPU clock runs continuously).
    pub fn power(&mut self) {
        self.registers = Registers::new();
        self.oam = [0u8; OAM_SIZE];
        self.secondary_oam = [0u8; SECONDARY_OAM_SIZE];
        self.secondary_oam_count = 0;
        self.sprite_overflow = false;
        self.sprite0_hit = false;
        self.sprite0_hit_dot = NO_SPRITE0_HIT_DOT;
        // Phase 6.1.e follow-up (VBL-block-phase alignment): frame
        // start is (sl 241, dot 0). See `PpuState::new` for rationale.
        self.scanline = 241;
        self.dot = 0;
        self.vbl_suppressed_this_frame = false;
        self.odd_frame = false;
        self.last_was_2005_or_2006 = false;
        self.oam_dma_pending = false;
        self.oam_dma_page = 0;
        self.oam_dma_counter = 0;
        self.bg_pshift = [0u16; 2];
        self.bg_atlatch = 0;
        self.bg_next_nt = 0;
        self.bg_next_at = 0;
        self.bg_next_pattern_lo = 0;
        self.bg_next_pattern_hi = 0;
        self.bg_primed = false;
        self.bg_active = false;
        self.sprite_shift = [[0u8; 2]; 8];
        self.sprite_attr = [0u8; 8];
        self.sprite_x = [0u8; 8];
        self.sprite0_in_range = false;
        self.sprite_eval_done = false;
    }

    /// Plan §0.8 step 1D.1: apply the NESdev PPU frame timing
    /// suppression window for a CPU-side `$2002` read.
    ///
    /// Per <https://www.nesdev.org/wiki/PPU_frame_timing>:
    /// * **(sl 240, dot 340) — 1 PPU clock before VBL set**:
    ///   the read sees VBL=0, and the PPU must *not* set the VBL
    ///   flag or generate an NMI for the entire frame.
    /// * **(sl 241, dot 0) — same alignment as above** (the sl→241
    ///   wrap puts dot 0 and the previous dot 340 on the same PPU
    ///   tick). The PPU programmer reference is explicit that the
    ///   "1 clock before" suppression extends here, so the read
    ///   must also suppress VBL+NMI for the entire frame.
    /// * **(sl 241, dot 1 or 2) — same dot or 1 dot later than
    ///   VBL set**: the read sees VBL=1 and clears it (handled by
    ///   `Registers::read_status`), and the read pulls `/NMI` back
    ///   up before the CPU samples it, so the pending NMI must
    ///   be canceled. (VBL was already set at (241, 1), so the
    ///   flag is set on read; this is the "1 dot later" window
    ///   per the PPU programmer reference's "may even be
    ///   suppressed by reads landing on the following dot or two".)
    /// * **Other dots**: no side effect.
    ///
    /// The method is on `PpuState` (not on `Registers::read_status`)
    /// because the suppression decision needs `(sl, dot)` which the
    /// value-level `Registers` API cannot see. The FFI wrapper in
    /// `ffi.rs::fceux11_ppu_cpu_read` calls this for `$2002` reads;
    /// unit tests in `tests/vbl_nmi.rs` call it directly.
    ///
    /// Phase 6.6.quater stage 3 (= plan §0.8 step 1) fix: the
    /// previous implementation used `sl == 241 && dot <= 1` as the
    /// second branch, which (a) failed to set `vbl_suppressed_this_frame`
    /// at (241, 0) and (b) skipped the (241, 2) "1 dot later" cancel
    /// window. Both were non-spec deviations. They are corrected
    /// here so the suppression window matches NESdev frame timing
    /// and `ppu_vbl_nmi 02-vbl_set_time` can pass.
    pub fn apply_a2002_suppression(&mut self) {
        let sl = self.scanline;
        let dot = self.dot;
        match (sl, dot) {
            (240, 340) | (241, 0) => {
                // 1 PPU clock before VBL set: suppress VBL+NMI for
                // this frame entirely. The frame state machine in
                // `frame.rs` checks `vbl_suppressed_this_frame` at
                // (sl 241, dot 1) and skips both `set_vbl_flag` and
                // `nmi_asserted`. The nmi_pending clear is belt-and-
                // suspenders; at (240, 340) nmi_pending is normally
                // false, and at (241, 0) the per-dot interleave has
                // not yet set it.
                self.vbl_suppressed_this_frame = true;
                self.nmi_pending = false;
            }
            (241, 1) | (241, 2) => {
                // Same dot or 1 dot later than VBL set. The flag WAS
                // set at (241, 1) (the state machine ran before the
                // CPU read landed); `Registers::read_status` will
                // clear it for the return-value semantics. We also
                // cancel the pending NMI latch so the per-cycle
                // interleave doesn't fire `TriggerNMI()` for the
                // suppressed frame.
                self.nmi_pending = false;
            }
            _ => {}
        }
    }

    /// Soft reset — like power but keeps `frame` and `odd_frame`.
    /// Distinct from power in the legacy FCEUX `FCEUPPU_Reset` semantics.
    pub fn reset(&mut self) {
        let frame = self.frame;
        let odd = self.odd_frame;
        self.power();
        self.frame = frame;
        self.odd_frame = odd;
    }

    // -----------------------------------------------------------------
    // Convenience accessors
    // -----------------------------------------------------------------

    /// True if rendering is enabled (BG or sprites visible per mask).
    #[inline]
    pub fn rendering_enabled(&self) -> bool {
        (self.registers.mask & ((1 << mask_bits::SHOW_BG) | (1 << mask_bits::SHOW_SPRITES))) != 0
    }

    /// NMI enable bit from `$2000`.
    #[inline]
    pub fn nmi_enabled(&self) -> bool {
        (self.registers.ctrl >> ctrl_bits::NMI_ENABLE) & 1 != 0
    }

    // -----------------------------------------------------------------
    // OAM eval
    // -----------------------------------------------------------------

    /// Sprite evaluation for the upcoming scanline. Phase 1 implements
    /// the minimum behaviour: copy up to 8 sprites whose Y range
    /// contains the current scanline into secondary OAM, marking
    /// overflow if a 9th sprite would have qualified.
    ///
    /// `sprite_height` is 8 or 16 (from `$2000` bit 5).
    pub fn eval_sprites(&mut self, sprite_height: u8) {
        let sl = self.scanline;
        let height = sprite_height as i16; // caller passes 8 or 16.
        self.secondary_oam = [0u8; SECONDARY_OAM_SIZE];
        self.secondary_oam_count = 0;
        self.sprite_overflow = false;

        for i in 0..64usize {
            let y = self.oam[i * 4];
            // Sprite Y of $FF means "off scanline" (legacy convention).
            // Y values 0..=238 are valid visible positions; >239 means
            // the sprite starts below the screen.
            //
            // Note: `sl >= y && sl < y + height` is parsed as
            // `(sl >= y) && (sl < y + height)` only when parenthesised —
            // the chained form `a <= b < c` in Rust is `(a <= b) < c`
            // (boolean < integer), always true.
            let in_range = y < 0xFF && (y as i16) <= sl && sl < (y as i16) + height;
            if !in_range {
                continue;
            }
            if (self.secondary_oam_count as usize) < MAX_SPRITES_PER_LINE {
                let dst = (self.secondary_oam_count as usize) * 4;
                self.secondary_oam[dst..dst + 4].copy_from_slice(&self.oam[i * 4..i * 4 + 4]);
                self.secondary_oam_count += 1;
            } else {
                // 9th+ in-range sprite: overflow. (A real PPU has
                // dot-by-dot sprite eval quirks; Phase 4 sharpens this.)
                self.sprite_overflow = true;
                if self.sprite_overflow {
                    self.registers.set_sprite_overflow();
                }
                break;
            }
        }
    }

    // -----------------------------------------------------------------
    // OAM DMA
    // -----------------------------------------------------------------

    /// Synchronous OAM DMA copy. Reads 256 bytes from `(page << 8)`..
    /// `(page << 8) | 0xFF` via the bus and writes them into OAM. Sets
    /// `oam_addr = 0` (matches C++ behaviour).
    ///
    /// Phase 3 prefers the per-cycle asynchronous pump: the CPU writes
    /// `$4014` to call [`Self::begin_oam_dma`] which sets
    /// `oam_dma_pending = true`, then [`crate::scheduler::NesScheduler`]
    /// calls [`Self::tick_oam_dma`] once per CPU cycle until 256 bytes
    /// are transferred. This synchronous method is kept as a fallback
    /// for callers that don't drive the async pump (e.g. the Phase 1/2
    /// integration tests).
    pub fn start_oam_dma<B: PpuBus + ?Sized>(&mut self, bus: &mut B, page: u8) {
        let base = (page as u16) << 8;
        for i in 0..OAM_SIZE as u16 {
            self.oam[i as usize] = bus.read_cpu(base | i);
        }
        self.registers.start_oam_dma();
    }

    /// Phase 3: begin an async OAM DMA. Sets `oam_dma_pending = true`,
    /// records the source page, and resets the byte counter. The
    /// scheduler calls [`Self::tick_oam_dma`] once per CPU cycle to
    /// transfer bytes one at a time.
    pub fn begin_oam_dma(&mut self, page: u8) {
        self.oam_dma_pending = true;
        self.oam_dma_page = page;
        self.oam_dma_counter = 0;
        // `oam_addr` is reset to 0 by the DMA completion path, not
        // here — the C++ reference does the reset on cycle 1 of the
        // pump (`src/ppu.cpp` X6502_DMW). We mirror that behaviour by
        // zeroing it inside `tick_oam_dma`'s first-cycle branch.
    }

    /// Phase 3: advance the async OAM DMA by one byte. Returns `true`
    /// if a byte was transferred this tick; `false` if no DMA is
    /// pending. When the counter reaches 256 the DMA completes and
    /// `oam_dma_pending` is cleared (and `oam_addr` is reset to 0).
    pub fn tick_oam_dma<B: PpuBus + ?Sized>(&mut self, bus: &mut B) -> bool {
        if !self.oam_dma_pending {
            return false;
        }
        let base = (self.oam_dma_page as u16) << 8;
        let i = self.oam_dma_counter;
        if i == 0 {
            // First byte: reset oam_addr to 0 (C++ `DMW()` starts the
            // pump with `_oam_addr = 0`; Phase 1's `start_oam_dma`
            // does the same synchronously).
            self.registers.start_oam_dma();
        }
        let addr = base | i;
        let v = bus.read_cpu(addr);
        self.oam[i as usize] = v;
        self.oam_dma_counter += 1;
        if self.oam_dma_counter >= OAM_SIZE as u16 {
            self.oam_dma_pending = false;
            self.oam_dma_counter = 0;
            self.registers.start_oam_dma();
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::FlatBus;

    #[test]
    fn power_clears_state() {
        let mut s = PpuState::new();
        s.scanline = 100;
        s.dot = 200;
        s.frame = 5;
        s.oam[0] = 0xAB;
        s.registers.ctrl = 0xFF;
        s.power();
        // Phase 6.1.e follow-up (VBL-first layout): power() places
        // (sl, dot) at (241, 0). Pre-Phase-6.1.e tests expected -1.
        assert_eq!(s.scanline, 241);
        assert_eq!(s.dot, 0);
        assert_eq!(s.frame, 5, "power preserves the frame counter");
        assert_eq!(s.oam[0], 0);
        assert_eq!(s.registers.ctrl, 0);
    }

    #[test]
    fn reset_preserves_frame_and_odd_frame() {
        let mut s = PpuState::new();
        s.frame = 1234;
        s.odd_frame = true;
        s.registers.ctrl = 0xFF;
        s.reset();
        assert_eq!(s.frame, 1234);
        assert!(s.odd_frame);
        assert_eq!(s.registers.ctrl, 0);
    }

    #[test]
    fn rendering_enabled_tracks_show_bg_and_show_sprites() {
        let mut s = PpuState::new();
        assert!(!s.rendering_enabled());
        s.registers.write_mask(1 << mask_bits::SHOW_BG);
        assert!(s.rendering_enabled());
        s.registers.write_mask(1 << mask_bits::SHOW_SPRITES);
        assert!(s.rendering_enabled());
        s.registers.write_mask(0);
        assert!(!s.rendering_enabled());
    }

    #[test]
    fn eval_sprites_finds_three_in_range() {
        let mut s = PpuState::new();
        // Scanline 50, three sprites covering scanlines 45..53.
        s.scanline = 50;
        // Sprite 0: y=45, tile=0x01, attr=0x00, x=10
        s.oam[0..4].copy_from_slice(&[45, 0x01, 0x00, 10]);
        // Sprite 1: y=48, ...
        s.oam[4..8].copy_from_slice(&[48, 0x02, 0x01, 20]);
        // Sprite 2: y=50, ...
        s.oam[8..12].copy_from_slice(&[50, 0x03, 0x02, 30]);
        // Sprite 3: y=80 — out of range
        s.oam[12..16].copy_from_slice(&[80, 0x04, 0x03, 40]);

        s.eval_sprites(8);
        assert_eq!(s.secondary_oam_count, 3);
        // Sprite 0 at offset 0: Y=45.
        assert_eq!(s.secondary_oam[0], 45);
        // Sprite 1 at offset 4: Y=48, tile=0x02.
        assert_eq!(s.secondary_oam[4], 48);
        assert_eq!(s.secondary_oam[5], 0x02);
        // Sprite 2 at offset 8: Y=50.
        assert_eq!(s.secondary_oam[8], 50);
        assert!(!s.sprite_overflow);
    }

    #[test]
    fn eval_sprites_caps_at_eight_and_marks_overflow() {
        let mut s = PpuState::new();
        s.scanline = 30;
        for i in 0..10 {
            s.oam[i * 4] = 25; // all in range
            s.oam[i * 4 + 1] = i as u8;
            s.oam[i * 4 + 2] = 0;
            s.oam[i * 4 + 3] = 0;
        }
        s.eval_sprites(8);
        assert_eq!(s.secondary_oam_count, 8);
        assert!(s.sprite_overflow);
        assert_ne!(s.registers.status & (1 << status_bits::SPRITE_OVERFLOW), 0);
    }

    #[test]
    fn start_oam_dma_copies_256_bytes() {
        let mut s = PpuState::new();
        let mut bus = FlatBus::new();
        bus.fill_cpu_page(0x02, 0x40);
        s.oam[0] = 0xFF; // sentinel
        s.registers.oam_addr = 0x80;
        s.start_oam_dma(&mut bus, 0x02);
        assert_eq!(s.oam[0], 0x40);
        assert_eq!(s.oam[0xFF], 0x40u8.wrapping_add(0xFF));
        assert_eq!(s.registers.oam_addr, 0, "DMA resets oam_addr");
        // First byte of page $02 is index 0 → (0x40 + 0) = 0x40
        // last byte of page $02 is index 0xFF → (0x40 + 0xFF) = 0x3F (wraps)
        assert_eq!(s.oam[0xFF], 0x3F);
    }
}
