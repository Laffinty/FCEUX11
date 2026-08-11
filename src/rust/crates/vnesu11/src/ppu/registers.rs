//! PPU registers ($2000-$2007) + internal v/t/x/w.
//!
//! Reference: NESdev wiki "PPU scrolling" + `src/ppu.cpp` (PPU[] array
//! indices) + `src/ppu_rendering.cpp` (register side effects).
//!
//! # Register summary
//!
//! | Reg | Name    | Writable bits | Read-only bits |
//! |-----|---------|--------------|----------------|
//! | $2000 | PPUCTRL  | 8             | -              |
//! | $2001 | PPUMASK  | 8             | -              |
//! | $2002 | PPUSTATUS| -             | 8 (V/S/O/R)    |
//! | $2003 | OAMADDR  | 8             | -              |
//! | $2004 | OAMDATA  | 8             | -              |
//! | $2005 | PPUSCROLL| 8             | -              |
//! | $2006 | PPUADDR  | 8             | -              |
//! | $2007 | PPUDATA  | 8             | -              |
//!
//! # Internal v/t/x/w
//!
//! - `v` (15 bits): current VRAM address (NN YYYYY XXXXX)
//! - `t` (15 bits): temporary VRAM address (NN YYYYY XXXXX)
//! - `x` (3 bits): fine X scroll
//! - `w` (1 bit): write toggle (first/second write)

/// $2000 PPUCTRL bits.
pub const PPUCTRL_NAMETABLE1: u8 = 0x01;
pub const PPUCTRL_NAMETABLE2: u8 = 0x02;
pub const PPUCTRL_INCREMENT: u8 = 0x04; // 0 = +1, 1 = +32
pub const PPUCTRL_SPRITE_TABLE: u8 = 0x08; // 0=$0000, 1=$1000
pub const PPUCTRL_BG_TABLE: u8 = 0x10; // 0=$0000, 1=$1000
pub const PPUCTRL_SPRITE_SIZE: u8 = 0x20; // 0=8x8, 1=8x16
pub const PPUCTRL_MASTER_SLAVE: u8 = 0x40; // unused
pub const PPUCTRL_NMI_ENABLE: u8 = 0x80;

/// $2001 PPUMASK bits.
pub const PPUMASK_GREYSCALE: u8 = 0x01;
pub const PPUMASK_SHOW_LEFT_BG: u8 = 0x02;
pub const PPUMASK_SHOW_LEFT_SPR: u8 = 0x04;
pub const PPUMASK_SHOW_BG: u8 = 0x08;
pub const PPUMASK_SHOW_SPR: u8 = 0x10;
pub const PPUMASK_EMPHASIS_R: u8 = 0x20;
pub const PPUMASK_EMPHASIS_G: u8 = 0x40;
pub const PPUMASK_EMPHASIS_B: u8 = 0x80;

/// $2002 PPUSTATUS bits.
pub const PPUSTATUS_UNUSED: u8 = 0x1F;
pub const PPUSTATUS_SPRITE_OVERFLOW: u8 = 0x20;
pub const PPUSTATUS_SPRITE_ZERO_HIT: u8 = 0x40;
pub const PPUSTATUS_VBLANK: u8 = 0x80;

/// Internal VRAM address (`v` or `t` register). 15 bits:
///
/// bit 0-4: coarse X scroll (X X X X X)
/// bit 5-9: coarse Y scroll (Y Y Y Y Y)
/// bit 10-11: nametable select (NN)
/// bit 12-14: fine Y scroll (F F F)
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VramAddr(pub u16);

impl VramAddr {
    /// Coarse X scroll (5 bits, bits 0-4 of `v`).
    #[inline(always)]
    pub fn coarse_x(&self) -> u16 {
        self.0 & 0x001F
    }
    /// Coarse Y scroll (5 bits, bits 5-9 of `v`).
    #[inline(always)]
    pub fn coarse_y(&self) -> u16 {
        (self.0 >> 5) & 0x001F
    }
    /// Nametable select (2 bits, bits 10-11 of `v`).
    #[inline(always)]
    pub fn nametable(&self) -> u16 {
        (self.0 >> 10) & 0x0003
    }
    /// Fine Y scroll (3 bits, bits 12-14 of `v`).
    #[inline(always)]
    pub fn fine_y(&self) -> u16 {
        (self.0 >> 12) & 0x0007
    }
    /// Set coarse X scroll (preserve other bits).
    #[inline(always)]
    pub fn set_coarse_x(&mut self, x: u16) {
        self.0 = (self.0 & !0x001F) | (x & 0x001F);
    }
    /// Set coarse Y scroll.
    #[inline(always)]
    pub fn set_coarse_y(&mut self, y: u16) {
        self.0 = (self.0 & !0x03E0) | ((y & 0x001F) << 5);
    }
    /// Set nametable select.
    #[inline(always)]
    pub fn set_nametable(&mut self, nt: u16) {
        self.0 = (self.0 & !0x0C00) | ((nt & 0x0003) << 10);
    }
    /// Set fine Y scroll.
    #[inline(always)]
    pub fn set_fine_y(&mut self, fy: u16) {
        self.0 = (self.0 & !0x7000) | ((fy & 0x0007) << 12);
    }
    /// Set fine X scroll (only used on `t`/`v` writes; X is stored
    /// separately in `x`).
    #[inline(always)]
    pub fn copy_from(&mut self, other: &VramAddr) {
        self.0 = other.0;
    }
    /// Increment by `n` (PPUDATA $2007 wrap at 0x4000).
    #[inline(always)]
    pub fn increment(&mut self, n: u16) {
        // 14-bit effective address space (mirrors $4000-$3FFF fold back).
        self.0 = self.0.wrapping_add(n) & 0x7FFF;
    }
    /// Get the 14-bit address (bits 0-13, mirrors folded).
    #[inline(always)]
    pub fn addr(&self) -> u16 {
        self.0 & 0x3FFF
    }
}

/// PPU register file + internal state.
#[derive(Debug, Default, Clone)]
pub struct PpuRegisters {
    /// $2000 PPUCTRL.
    pub ppuctrl: u8,
    /// $2001 PPUMASK.
    pub ppumask: u8,
    /// $2002 PPUSTATUS.
    pub status: u8,
    /// $2003 OAMADDR.
    pub oam_addr: u8,
    /// OAM DMA page (the high byte of the source page for $4014 writes).
    pub oam_dma_page: u8,

    /// Internal `v` register (current VRAM address).
    pub v: VramAddr,
    /// Internal `t` register (temporary VRAM address).
    pub t: VramAddr,
    /// Internal `x` register (fine X scroll, 3 bits).
    pub x: u8,
    /// Internal `w` register (write toggle).
    pub w: bool,

    /// Data buffer for $2007 reads (lag by one).
    pub read_buffer: u8,
}

impl PpuRegisters {
    pub fn new() -> Self {
        Self {
            ppuctrl: 0,
            ppumask: 0,
            status: 0,
            oam_addr: 0,
            oam_dma_page: 0,
            v: VramAddr::default(),
            t: VramAddr::default(),
            x: 0,
            w: false,
            read_buffer: 0,
        }
    }

    /// Get nametable base address from PPUCTRL bits 0-1.
    #[inline]
    pub fn nametable_base(&self) -> u16 {
        match self.ppuctrl & 0x03 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2C00,
            _ => unreachable!(),
        }
    }

    /// Get background pattern table base from PPUCTRL bit 4.
    #[inline]
    pub fn bg_pattern_base(&self) -> u16 {
        if (self.ppuctrl & PPUCTRL_BG_TABLE) != 0 { 0x1000 } else { 0x0000 }
    }

    /// Get sprite pattern table base from PPUCTRL bit 3.
    #[inline]
    pub fn sprite_pattern_base(&self) -> u16 {
        if (self.ppuctrl & PPUCTRL_SPRITE_TABLE) != 0 { 0x1000 } else { 0x0000 }
    }

    /// PPUDATA increment: +1 or +32 depending on PPUCTRL bit 2.
    #[inline]
    pub fn ppu_data_increment(&self) -> u16 {
        if (self.ppuctrl & PPUCTRL_INCREMENT) != 0 { 32 } else { 1 }
    }

    /// Sprite size: 8x8 or 8x16.
    #[inline]
    pub fn sprite_size(&self) -> SpriteSize {
        if (self.ppuctrl & PPUCTRL_SPRITE_SIZE) != 0 {
            SpriteSize::Size8x16
        } else {
            SpriteSize::Size8x8
        }
    }

    /// NMI enabled (PPUCTRL bit 7).
    #[inline]
    pub fn nmi_enabled(&self) -> bool {
        (self.ppuctrl & PPUCTRL_NMI_ENABLE) != 0
    }

    /// Reset transient status flags (VBlank, sprite 0 hit, sprite
    /// overflow). Called at the start of pre-render.
    pub fn clear_status_flags(&mut self) {
        self.status &= !(PPUSTATUS_VBLANK | PPUSTATUS_SPRITE_ZERO_HIT | PPUSTATUS_SPRITE_OVERFLOW);
    }

    /// Write $2005 (PPUSCROLL).  First write sets coarse X + fine X;
    /// second write sets coarse Y + fine Y.
    pub fn write_scroll(&mut self, val: u8) {
        if !self.w {
            // First write: coarse X, fine X
            self.x = val & 0x07;
            self.t.set_coarse_x((val >> 3) as u16);
            self.w = true;
        } else {
            // Second write: coarse Y, fine Y
            self.t.set_fine_y((val & 0x07) as u16);
            self.t.set_coarse_y((val >> 3) as u16);
            self.w = false;
        }
    }

    /// Write $2006 (PPUADDR).  First write sets high byte (with
    /// $3F mask); second write sets low byte and copies t → v.
    pub fn write_addr(&mut self, val: u8) {
        if !self.w {
            // First write: high 6 bits of v/t (mask $3F to clear bits 14, 15).
            self.t.0 = (self.t.0 & 0x00FF) | (((val & 0x3F) as u16) << 8);
            self.w = true;
        } else {
            // Second write: low byte of t, then t → v.
            self.t.0 = (self.t.0 & 0xFF00) | val as u16;
            self.v.copy_from(&self.t);
            self.w = false;
        }
    }

    /// Read $2002 (PPUSTATUS).  Reading clears the w toggle + VBlank
    /// flag (top 3 bits stay: sprite overflow, sprite 0 hit, VBlank
    /// cleared except reading VBlank clears it).
    pub fn read_status(&mut self) -> u8 {
        let v = self.status;
        self.w = false;
        // Reading PPUSTATUS clears VBlank (bit 7). Source: NESdev wiki.
        self.status &= !PPUSTATUS_VBLANK;
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteSize {
    Size8x8,
    Size8x16,
}

impl SpriteSize {
    pub fn height(self) -> u8 {
        match self {
            Self::Size8x8 => 8,
            Self::Size8x16 => 16,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ppuctrl_bit_decoding() {
        let mut r = PpuRegisters::new();
        // 0b1000_0101 = 0x85: bit0=1, bit2=1, bit7=1. Plus sprite_size
        // bit 5 = 0 (8x8), nametable bits 0-1 = 01 ($2400), +32 increment.
        r.ppuctrl = 0b1000_0101;
        // bits 0-1 = 01 → nametable $2400
        assert_eq!(r.nametable_base(), 0x2400);
        // bit 2 = 1 → +32 increment
        assert_eq!(r.ppu_data_increment(), 32);
        // bit 3 = 0 → sprite table $0000
        assert_eq!(r.sprite_pattern_base(), 0x0000);
        // bit 4 = 0 → bg table $0000
        assert_eq!(r.bg_pattern_base(), 0x0000);
        // bit 5 = 0 → 8x8 sprites
        assert_eq!(r.sprite_size(), SpriteSize::Size8x8);
        // bit 7 = 1 → NMI enabled
        assert!(r.nmi_enabled());
    }

    #[test]
    fn write_scroll_first_then_second() {
        let mut r = PpuRegisters::new();
        // First write: $2005 = 0xAB
        //   x = 0xAB & 0x07 = 3
        //   coarse X (t) = (0xAB >> 3) = 0x15
        r.write_scroll(0xAB);
        assert_eq!(r.x, 0x03);
        assert_eq!(r.t.coarse_x(), 0x15);
        assert!(r.w);

        // Second write: $2005 = 0xCD
        //   fine Y (t) = 0xCD & 0x07 = 5
        //   coarse Y (t) = (0xCD >> 3) = 0x19
        r.write_scroll(0xCD);
        assert_eq!(r.t.fine_y(), 0x05);
        assert_eq!(r.t.coarse_y(), 0x19);
        assert!(!r.w);
    }

    #[test]
    fn write_addr_two_writes() {
        let mut r = PpuRegisters::new();
        // First write: 0x20 (high byte) → t high = (0x20 & 0x3F) << 8 = $2000
        r.write_addr(0x20);
        assert_eq!(r.t.0, 0x2000);
        assert!(r.w);

        // Second write: 0x00 (low byte) → t low = $00, then v ← t = $2000
        r.write_addr(0x00);
        assert_eq!(r.t.0, 0x2000);
        assert_eq!(r.v.0, 0x2000);
        assert!(!r.w);
    }

    #[test]
    fn read_status_clears_vblank() {
        let mut r = PpuRegisters::new();
        r.status = PPUSTATUS_VBLANK | PPUSTATUS_SPRITE_ZERO_HIT;
        let v = r.read_status();
        assert_eq!(v & PPUSTATUS_VBLANK, PPUSTATUS_VBLANK);
        // After read, VBlank must be cleared.
        assert_eq!(r.status & PPUSTATUS_VBLANK, 0);
        // Sprite 0 hit is NOT cleared by reading.
        assert_eq!(r.status & PPUSTATUS_SPRITE_ZERO_HIT, PPUSTATUS_SPRITE_ZERO_HIT);
        // w must be reset.
        assert!(!r.w);
    }

    #[test]
    fn vram_addr_bit_packing() {
        let mut v = VramAddr::default();
        v.set_coarse_x(0x15);
        v.set_coarse_y(0x1A);
        v.set_nametable(2);
        v.set_fine_y(5);
        assert_eq!(v.coarse_x(), 0x15);
        assert_eq!(v.coarse_y(), 0x1A);
        assert_eq!(v.nametable(), 2);
        assert_eq!(v.fine_y(), 5);
        // Verify no overlap: each setter must preserve other bits.
        v.set_coarse_x(0);
        assert_eq!(v.coarse_y(), 0x1A);
        assert_eq!(v.nametable(), 2);
        assert_eq!(v.fine_y(), 5);
    }

    #[test]
    fn clear_status_flags_resets_three_bits() {
        let mut r = PpuRegisters::new();
        r.status = 0xFF;
        r.clear_status_flags();
        // Only bits 5-7 are cleared (VBlank, sprite 0 hit, sprite overflow).
        // Bits 0-4 are PPUSTATUS_UNUSED and remain set.
        assert_eq!(r.status & !0xE0, 0x1F);
        assert_eq!(r.status & 0xE0, 0);
    }
}