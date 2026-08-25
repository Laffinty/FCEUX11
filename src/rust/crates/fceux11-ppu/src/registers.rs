//! `$2000`-`$2007` and `$4014` register file, scroll latches, open-bus buffer.
//!
//! The state machine in [`crate::frame::tick_dot`] drives timing events
//! (scanline/dot, VBlank flag set/clear, NMI sampling, scroll copy on
//! rendering scanlines). The methods on [`Registers`] are *value-level*
//! side-effects: each method commits exactly the bits the spec says a CPU
//! write or read triggers, without worrying about which dot the CPU is at.
//!
//! Bit layout reference: <https://www.nesdev.org/wiki/PPU_registers>.
//! Open-bus / buffered-read semantics: <https://www.nesdev.org/wiki/PPU_scrolling#PPU_internal_registers>.

use crate::bus::PpuBus;

/// Bit indices in `$2000` PPUCTRL.
pub mod ctrl_bits {
    pub const NMI_ENABLE: u8 = 7;
    pub const SPRITE_SIZE: u8 = 5; // 0 = 8x8, 1 = 8x16
    pub const BG_PATTERN: u8 = 4; // 0 = $0000, 1 = $1000
    pub const SPRITE_PATTERN: u8 = 3;
    pub const VRAM_INCREMENT: u8 = 2; // 0 = +1, 1 = +32
    pub const NAMETABLE_LO: u8 = 0;
    pub const NAMETABLE_HI: u8 = 1;
}

/// Bit indices in `$2001` PPUMASK.
pub mod mask_bits {
    pub const EMPH_BLUE: u8 = 7;
    pub const EMPH_GREEN: u8 = 6;
    pub const EMPH_RED: u8 = 5;
    pub const SHOW_SPRITES: u8 = 4;
    pub const SHOW_BG: u8 = 3;
    pub const SPRITE_LEFT8: u8 = 2;
    pub const BG_LEFT8: u8 = 1;
    pub const GRAYSCALE: u8 = 0;
}

/// Bit indices in `$2002` PPUSTATUS.
pub mod status_bits {
    pub const VBL: u8 = 7;
    pub const SPRITE0_HIT: u8 = 6;
    pub const SPRITE_OVERFLOW: u8 = 5;
    /// bits 4..=0 — last CPU write was to $2005 or $2006 (clear after
    /// $2002 read or $2004/$2007 access). The exact value is the
    /// "second-write toggle" before the read.
    pub const WRITE_TOGGLE: u8 = 4;
}

/// Mask covering bits 4..=0 of `$2002` (the open-bus / write-toggle echo).
pub const STATUS_WRITE_TOGGLE_MASK: u8 = 0x1F;

/// PPU register file + the hidden latches the CPU can't see but the
/// rendering state machine relies on.
///
/// `v` and `t` are 15-bit VRAM addresses laid out as
/// `yyy NN YYYYY XXXXX` (bits 14..=12 = fine Y, 11 = nametable Y,
/// 10 = nametable X, 9..=5 = coarse Y, 4..=0 = coarse X). `fine_x`
/// holds bits 2..=0 of the X scroll (the rest of the first `$2005`
/// write goes into coarse X bits of `t`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registers {
    /// `$2000` PPUCTRL.
    pub ctrl: u8,
    /// `$2001` PPUMASK.
    pub mask: u8,
    /// `$2002` PPUSTATUS (VBL / sprite0 hit / overflow + write-toggle echo).
    pub status: u8,
    /// `$2003` OAMADDR.
    pub oam_addr: u8,
    /// `$2005`/`$2006` second-write toggle. False = next write is the
    /// first, true = next write is the second.
    pub write_toggle: bool,
    /// Current VRAM address (15 bits used).
    pub v: u16,
    /// Temporary VRAM address (15 bits used) — the scroll latches.
    pub t: u16,
    /// Fine X scroll (3 bits).
    pub fine_x: u8,
    /// Buffered read result for `$2007` open-bus behaviour. Palette reads
    /// don't update this; everything else does.
    pub vram_buffer: u8,
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    /// Cold-state reset (after power-on): all registers cleared, latches
    /// cleared, open-bus buffer cleared.
    pub const fn new() -> Self {
        Self {
            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            write_toggle: false,
            v: 0,
            t: 0,
            fine_x: 0,
            vram_buffer: 0,
        }
    }

    /// Warm reset (post power-on, post soft reset): keep `vram_buffer` so
    /// a game reading `$2007` during boot still gets the last garbage byte
    /// from the previous frame. Match the legacy FCEUX behaviour.
    pub fn warm_reset(&mut self) {
        self.ctrl = 0;
        self.mask = 0;
        self.status = 0;
        self.oam_addr = 0;
        self.write_toggle = false;
        self.v = 0;
        self.t = 0;
        self.fine_x = 0;
    }

    // -- control / mask ------------------------------------------------

    /// `$2000` write. `nmi_enable_changed` is set if bit 7 transitioned.
    pub fn write_ctrl(&mut self, val: u8) -> bool {
        let old_nmi = (self.ctrl >> ctrl_bits::NMI_ENABLE) & 1;
        self.ctrl = val;
        let new_nmi = (val >> ctrl_bits::NMI_ENABLE) & 1;
        old_nmi != new_nmi
    }

    /// `$2001` write.
    pub fn write_mask(&mut self, val: u8) {
        self.mask = val;
    }

    // -- status --------------------------------------------------------

    /// `$2002` read — returns the status byte and clears the VBL flag.
    /// The bit-4..=0 echo is the previous `write_toggle` *before* this
    /// access; the read then flips `write_toggle` to false.
    pub fn read_status(&mut self) -> u8 {
        let prev_toggle = self.write_toggle;
        let visible_mask = (1 << status_bits::VBL)
            | (1 << status_bits::SPRITE0_HIT)
            | (1 << status_bits::SPRITE_OVERFLOW);
        let mut v = self.status & visible_mask;
        if prev_toggle {
            v |= STATUS_WRITE_TOGGLE_MASK;
        }
        // Reading $2002 clears bit 7 (VBL) and resets the write toggle
        // (which controls the bits 4..=0 echo).
        self.status &= !(1 << status_bits::VBL);
        self.write_toggle = false;
        v
    }

    /// Set the VBL flag from outside (the frame state machine calls this
    /// at sl 241 dot 1, *unless* the early-read suppression already
    /// cleared it).
    pub fn set_vbl_flag(&mut self) {
        self.status |= 1 << status_bits::VBL;
    }

    /// Clear the VBL flag.
    pub fn clear_vbl_flag(&mut self) {
        self.status &= !(1 << status_bits::VBL);
    }

    /// Mark the VBL flag as suppressed for this frame — `$2002` was read
    /// at sl 241 dot 0 (one PPU dot before the working-config set
    /// boundary). The frame state machine consults this before set.
    pub fn suppress_vbl_for_frame(&mut self) {
        self.status &= !(1 << status_bits::VBL);
    }

    /// Set the sprite-0-hit flag (used by sprite eval).
    pub fn set_sprite0_hit(&mut self) {
        self.status |= 1 << status_bits::SPRITE0_HIT;
    }

    /// Clear sprite-0-hit (called when rendering turns off).
    pub fn clear_sprite0_hit(&mut self) {
        self.status &= !(1 << status_bits::SPRITE0_HIT);
    }

    /// Set sprite overflow flag.
    pub fn set_sprite_overflow(&mut self) {
        self.status |= 1 << status_bits::SPRITE_OVERFLOW;
    }

    /// Clear sprite overflow flag.
    pub fn clear_sprite_overflow(&mut self) {
        self.status &= !(1 << status_bits::SPRITE_OVERFLOW);
    }

    // -- OAM DMA / direct ----------------------------------------------

    /// `$4014` OAM DMA — write triggers a 256-byte copy from
    /// `(page << 8)..=(page << 8 | 0xFF)` into OAM. The PPU state is
    /// responsible for actually driving the read loop through the bus;
    /// `Registers` only stores `oam_addr = 0` afterwards (matches C++
    /// behaviour).
    pub fn start_oam_dma(&mut self) {
        self.oam_addr = 0;
    }

    /// `$2003` write.
    pub fn write_oam_addr(&mut self, val: u8) {
        self.oam_addr = val;
    }

    /// `$2004` write — increments `oam_addr` (mod 256).
    pub fn increment_oam_addr(&mut self) {
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    // -- scroll / addr latches ----------------------------------------

    /// `$2005` write — double-write scroll. First write stores coarse X
    /// in `t` and fine X in `fine_x`; second stores fine Y and coarse Y.
    pub fn write_scroll(&mut self, val: u8) {
        if !self.write_toggle {
            // First write: t.coarse_x = val >> 3; fine_x = val & 7.
            self.t = (self.t & !0x001F) | ((val as u16) >> 3);
            self.fine_x = val & 0x07;
        } else {
            // Second write: t.fine_y   = val & 0x07;
            //             t.coarse_y = val >> 3.
            // Bit layout of t: yyy NN YYYYY XXXXX
            //  coarse_y is bits 9..=5 → shift val >> 3 by 5
            //  fine_y   is bits 14..=12 → shift val & 0x07 by 12
            self.t = (self.t & !0x7BE0)
                | ((((val as u16) >> 3) & 0x1F) << 5)
                | (((val as u16) & 0x07) << 12);
        }
        self.write_toggle = !self.write_toggle;
    }

    /// `$2006` write — double-write VRAM address.
    pub fn write_addr(&mut self, val: u8) {
        if !self.write_toggle {
            // First write: high 6 bits into bits 13..=8 of t.
            self.t = (self.t & !0x3F00) | (((val as u16) & 0x3F) << 8);
        } else {
            // Second write: low 8 bits, then v = t.
            self.t = (self.t & !0x00FF) | ((val as u16) & 0x00FF);
            self.v = self.t;
        }
        self.write_toggle = !self.write_toggle;
    }

    /// Copy the horizontal bits of `t` into `v` — called by the state
    /// machine at the start of each visible scanline's visible dots.
    pub fn copy_horizontal(&mut self) {
        // v.coarse_x and v.nametable_x come from t.
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    /// Copy vertical bits of `t` into `v` — called at the end of the
    /// visible portion of each scanline (around dot 257).
    pub fn copy_vertical(&mut self) {
        // v.coarse_y, v.nametable_y, v.fine_y come from t.
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    /// Increment coarse X by 1 with horizontal nametable flip at the
    /// coarse-X rollover. Triggered by the rendering BG fetch at the
    /// appropriate dot.
    pub fn increment_coarse_x(&mut self) {
        let coarse_x = (self.v & 0x001F) + 1;
        if coarse_x & 0x20 != 0 {
            // Rollover: coarse_x = 0; nametable_x flips.
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v = (self.v & !0x001F) | (coarse_x & 0x001F);
        }
    }

    /// Increment fine Y (3 bits) with carry into coarse Y / nametable Y.
    /// Triggered at the end of the visible portion of each scanline.
    pub fn increment_fine_y(&mut self) {
        let mut fine_y = (self.v >> 12) & 0x07;
        let mut coarse_y = (self.v >> 5) & 0x1F;
        let mut nametable_y = (self.v >> 11) & 0x01;

        fine_y = (fine_y + 1) & 0x07;
        if fine_y == 0 {
            // Carry into coarse_y; rollover flips nametable_y.
            coarse_y = (coarse_y + 1) & 0x1F;
            if coarse_y == 30 {
                coarse_y = 0;
                nametable_y ^= 1;
            }
        }

        // v layout: yyy NN YYYYY XXXXX = bits 14..12 / 11..10 / 9..5 / 4..0.
        // We clear those bits and rebuild from the freshly-incremented
        // fields. The other bits (which encode things like the BG/sprite
        // pattern-table selects in some PPU variants) are untouched.
        let v = self.v & 0x8C1F_u16;
        self.v = v | (coarse_y << 5) | (nametable_y << 11) | (fine_y << 12);
    }

    // -- $2007 data ----------------------------------------------------

    /// `$2007` read. Returns the buffered read if `v` is below the
    /// palette range; for palette reads (`v >= 0x3F00`) returns the
    /// real bus read and does *not* update the buffer.
    ///
    /// After returning, `v` is incremented by 1 (or 32 if `ctrl` bit 2
    /// is set).
    pub fn read_data<B: PpuBus + ?Sized>(&mut self, bus: &mut B, ctrl: u8) -> u8 {
        let v = self.v;
        let addr = self.mirror_data_addr(v);
        let result = if (v & 0x3FFF) < 0x3F00 {
            // Non-palette: return the buffered byte from the *last* read;
            // this read returns the real value into the buffer.
            let buffered = self.vram_buffer;
            self.vram_buffer = bus.read(addr);
            buffered
        } else {
            // Palette range: return real value, leave buffer alone.
            bus.read(addr)
        };
        self.increment_v(ctrl);
        result
    }

    /// `$2007` write. Writes through `v`; updates the open-bus buffer
    /// only if `v` is below the palette range.
    pub fn write_data<B: PpuBus + ?Sized>(&mut self, bus: &mut B, ctrl: u8, val: u8) {
        let v = self.v;
        let addr = self.mirror_data_addr(v);
        bus.write(addr, val);
        if (v & 0x3FFF) < 0x3F00 {
            self.vram_buffer = val;
        }
        self.increment_v(ctrl);
    }

    /// Apply the `$2007` increment to `v` based on `ctrl` bit 2.
    fn increment_v(&mut self, ctrl: u8) {
        if ctrl & (1 << ctrl_bits::VRAM_INCREMENT) != 0 {
            self.v = self.v.wrapping_add(32) & 0x7FFF;
        } else {
            self.v = self.v.wrapping_add(1) & 0x7FFF;
        }
    }

    /// Apply the address-space mirroring that the real PPU uses for
    /// `$2007` access: $3F00-$3FFF mirror down to $3F00-$3FFF
    /// (palette aliases $3F00/$3F04/$3F08/$3F0C collapse to 0x00/0x04/0x08/0x0C).
    fn mirror_data_addr(&self, v: u16) -> u16 {
        let lo = v & 0x3FFF;
        if lo < 0x3F00 {
            lo
        } else {
            // Palette mirrors: $3F00/$3F04/$3F08/$3F0C are the same entry;
            // $3F10/$3F14/$3F18/$3F1C alias to $3F00/$3F04/$3F08/$3F0C.
            0x3F00 | (lo & 0x000F)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::FlatBus;

    #[test]
    fn new_clears_all_latches() {
        let r = Registers::new();
        assert_eq!(r.ctrl, 0);
        assert_eq!(r.mask, 0);
        assert_eq!(r.status, 0);
        assert_eq!(r.oam_addr, 0);
        assert!(!r.write_toggle);
        assert_eq!(r.v, 0);
        assert_eq!(r.t, 0);
        assert_eq!(r.fine_x, 0);
        assert_eq!(r.vram_buffer, 0);
    }

    #[test]
    fn ctrl_write_reports_nmi_edge() {
        let mut r = Registers::new();
        assert!(r.write_ctrl(0x80)); // 0 -> 0x80: rising edge
        assert!(r.write_ctrl(0x00)); // 0x80 -> 0x00: falling edge
        assert!(r.write_ctrl(0x80)); // 0 -> 0x80 again: rising edge
        assert!(!r.write_ctrl(0x80)); // 0x80 -> 0x80: hold
    }

    #[test]
    fn status_read_echoes_write_toggle_and_clears_vbl() {
        let mut r = Registers::new();
        r.status = 1 << status_bits::VBL | 1 << status_bits::SPRITE0_HIT;
        r.write_toggle = true;
        let v = r.read_status();
        // VBL bit is returned (set at read time) AND the stored flag is
        // cleared so the next read returns 0.
        assert_ne!(v & (1 << status_bits::VBL), 0, "VBL is returned");
        assert_ne!(
            v & (1 << status_bits::SPRITE0_HIT),
            0,
            "sprite0 hit preserved"
        );
        assert_eq!(v & STATUS_WRITE_TOGGLE_MASK, STATUS_WRITE_TOGGLE_MASK);
        assert!(!r.write_toggle, "$2002 read resets write toggle");
        let second = r.read_status();
        assert_eq!(
            second & (1 << status_bits::VBL),
            0,
            "VBL cleared after first read"
        );
        assert_ne!(
            second & (1 << status_bits::SPRITE0_HIT),
            0,
            "sprite0 hit preserved across reads"
        );
    }

    #[test]
    fn scroll_first_write_sets_coarse_x_and_fine_x() {
        let mut r = Registers::new();
        r.write_scroll(0xA5); // 0b1010_0101 → coarse_x=0x14, fine_x=5
        assert_eq!(r.t & 0x001F, 0x14);
        assert_eq!(r.fine_x, 5);
        assert!(r.write_toggle);
    }

    #[test]
    fn scroll_second_write_sets_coarse_y_and_fine_y() {
        let mut r = Registers::new();
        r.write_scroll(0xFF); // first — toggle to true
        r.write_scroll(0x7B); // second: fine_y=3, coarse_y=0x0F
        assert_eq!((r.t >> 5) & 0x1F, 0x0F);
        assert_eq!((r.t >> 12) & 0x07, 0x03);
        assert!(!r.write_toggle);
    }

    #[test]
    fn addr_double_write_copies_t_to_v() {
        let mut r = Registers::new();
        r.write_addr(0x21); // high 6 bits into t bits 13..8
        assert!(r.write_toggle);
        assert_eq!((r.t >> 8) & 0x3F, 0x21);
        r.write_addr(0xCA); // low 8 bits, v = t
        assert!(!r.write_toggle);
        assert_eq!(r.v, 0x21CA & 0x7FFF);
        assert_eq!(r.v, r.t);
    }

    #[test]
    fn coarse_x_increment_rolls_over_and_flips_nametable() {
        let mut r = Registers::new();
        r.v = 0x0400 | 0x1F; // nametable_x=1, coarse_x=31
        r.increment_coarse_x();
        assert_eq!(r.v & 0x001F, 0, "coarse_x wrapped to 0");
        assert_eq!(r.v & 0x0400, 0, "nametable_x flipped to 0");
    }

    #[test]
    fn vram_buffer_round_trip_through_data_register() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        bus.write(0x2000, 0xAA);
        r.v = 0x2000;
        let first = r.read_data(&mut bus, r.ctrl);
        assert_eq!(first, 0, "first read returns the prior buffer (0)");
        let second = r.read_data(&mut bus, r.ctrl);
        assert_eq!(second, 0xAA, "second read returns what was buffered");
    }

    #[test]
    fn palette_read_does_not_update_buffer() {
        let mut r = Registers::new();
        r.vram_buffer = 0xCD;
        let mut bus = FlatBus::new();
        bus.write(0x3F00, 0x12);
        r.v = 0x3F00;
        let v = r.read_data(&mut bus, r.ctrl);
        assert_eq!(v, 0x12, "palette returns the real bus value");
        assert_eq!(
            r.vram_buffer, 0xCD,
            "palette read does not touch the buffer"
        );
    }

    #[test]
    fn oam_dma_resets_oam_addr() {
        let mut r = Registers::new();
        r.oam_addr = 0x80;
        r.start_oam_dma();
        assert_eq!(r.oam_addr, 0);
    }

    #[test]
    fn v_increment_uses_ctrl_bit2() {
        let mut r = Registers::new();
        r.v = 0x1000;
        r.read_data(&mut FlatBus::new(), 0); // +1
        assert_eq!(r.v, 0x1001);
        r.v = 0x1000;
        r.read_data(&mut FlatBus::new(), 1 << ctrl_bits::VRAM_INCREMENT); // +32
        assert_eq!(r.v, 0x1020);
    }
}
