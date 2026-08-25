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
/// NTSC scanlines per frame, indexed `-1` (pre-render) ..= `261`
/// (post-render / VBL clear) — 263 indices total. `NTSC_SCANLINES` is
/// the count exclusive of the pre-render: `next_sl >= NTSC_SCANLINES`
/// triggers a frame wrap.
pub const NTSC_SCANLINES: i16 = 262;
/// PPU dots per scanline (visible 256 + hblank 85 = 341, indexed 0..=340).
pub const DOTS_PER_SCANLINE: u16 = 341;

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
    /// Current scanline: -1 (pre-render) .. 260 (post-render).
    pub scanline: i16,
    /// Current dot within scanline: 0..=340.
    pub dot: u16,
    /// Frame counter — wraps only by convention (u64 is plenty).
    pub frame: u64,
    /// Set by `$2002` read at sl 241 dot 0; consulted by frame state
    /// machine to suppress VBL flag set + NMI for this frame.
    pub vbl_suppressed_this_frame: bool,
    /// Toggles every pre-render to drive even/odd skip.
    pub odd_frame: bool,
    /// Tracks whether the most recent CPU write was `$2005`/`$2006`
    /// (echoed into PPU[2] bits 4..=0 until next read of `$2002` or
    /// write to `$2004`/`$2007`). Mirrors `Registers::write_toggle` but
    /// kept here so the state machine can flip it without touching
    /// registers when the frame loop's BG fetch needs to.
    pub last_was_2005_or_2006: bool,
    /// Set when CPU issues `$4014`; consumed by the frame state machine
    /// to drive the 256-cycle DMA pump (Phase 3 will turn this into
    /// a per-cycle async pump — Phase 1 performs the copy synchronously
    /// inside `start_oam_dma`).
    pub oam_dma_pending: bool,
}

impl Default for PpuState {
    fn default() -> Self {
        Self::new()
    }
}

impl PpuState {
    /// Cold-state init — everything zero, scanline/dot at frame start.
    pub const fn new() -> Self {
        Self {
            registers: Registers::new(),
            oam: [0u8; OAM_SIZE],
            secondary_oam: [0u8; SECONDARY_OAM_SIZE],
            secondary_oam_count: 0,
            sprite_overflow: false,
            sprite0_hit: false,
            scanline: -1,
            dot: 0,
            frame: 0,
            vbl_suppressed_this_frame: false,
            odd_frame: false,
            last_was_2005_or_2006: false,
            oam_dma_pending: false,
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
        self.scanline = -1;
        self.dot = 0;
        self.vbl_suppressed_this_frame = false;
        self.odd_frame = false;
        self.last_was_2005_or_2006 = false;
        self.oam_dma_pending = false;
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
    /// Phase 3 will turn this into a per-cycle asynchronous pump driven
    /// by the unified scheduler.
    pub fn start_oam_dma<B: PpuBus + ?Sized>(&mut self, bus: &mut B, page: u8) {
        let base = (page as u16) << 8;
        for i in 0..OAM_SIZE as u16 {
            self.oam[i as usize] = bus.read(base | i);
        }
        self.registers.start_oam_dma();
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
        assert_eq!(s.scanline, -1);
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
