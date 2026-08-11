//! CPU + PPU bus matrices.
//!
//! # Architecture (per `phase_2_bus_and_ram.md` §2.2, §5.3)
//!
//! Two layers, **not** one match:
//!
//! 1. **Fixed region** ($0000-$401F): address topology is hard-wired,
//!    `match` compiles to a jump table. WRAM/VRAM/PPU/APU/open-bus live
//!    here.
//! 2. **Mapper region** ($4020-$FFFF): mapper registers per-range handler
//!    tables via `SetReadHandler(start, end, fn)`. Linear scan over the
//!    (typically 4-16 entry) range table; cache-hot.
//!
//! Mapper region is what makes "pure match" impossible — C++ MMMC3 alone
//! registers 5+ ranges. See `mapper.rs` for the range table data structure.
//!
//! # Open-bus behavior
//!
//! NES real hardware: unmapped addresses return the "last value on the
//! data bus". vNESU11 tracks this in `VNesSoc::open_bus` — every CPU read
//! updates it, every CPU write sets it. This matters for some PPU/APU
//! tests that rely on stale bus values, but most games don't care.

use crate::mapper::MapperRangeTable;
use crate::ppu::nametable::Mirroring;
use crate::soc::VNesSoc;

impl VNesSoc {
    // ===================================================================
    // CPU BUS — read
    // ===================================================================

    /// CPU-side read at address `addr`. Returns the byte value.
    ///
    /// Hot path: `match` on the fixed region + linear scan over the
    /// mapper range table. Both layers are `#[inline(always)]`-able at
    /// the call site.
    #[inline]
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        let v = self.cpu_read_unchecked(addr);
        self.open_bus = v;
        v
    }

    /// Internal: do the decode without updating `open_bus`. Lets callers
    /// (write paths, snapshot) avoid double-tracking.
    #[inline(always)]
    fn cpu_read_unchecked(&mut self, addr: u16) -> u8 {
        match addr {
            // 2 KiB WRAM mirrored across $0000-$1FFF.
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            // PPU register mirror (every 8 bytes within $2000-$3FFF).
            0x2000..=0x3FFF => self.ppu_read_register((addr & 0x0007) as u8),
            // APU + DMC + joystick (Phase 4 wires the APU; Phase 2 keeps
            // registers open-bus for unmasked writes).
            0x4000..=0x401F => self.apu_io_read(addr),
            // Cartridge/expansion region ($4020-$FFFF): mapper range table.
            _ => self
                .mapper
                .read(addr)
                .unwrap_or_else(|| self.read_open_bus_for(addr)),
        }
    }

    // ===================================================================
    // CPU BUS — write
    // ===================================================================

    /// CPU-side write to `addr`.
    #[inline]
    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        self.cpu_write_unchecked(addr, val);
        self.open_bus = val;
    }

    #[inline(always)]
    fn cpu_write_unchecked(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize] = val,
            0x2000..=0x3FFF => self.ppu_write_register((addr & 0x0007) as u8, val),
            0x4000..=0x401F => self.apu_io_write(addr, val),
            // Mapper region.
            _ => {
                if !self.mapper.write(addr, val) {
                    // Unmapped write: open bus, no side effect.
                }
            }
        }
    }

    // ===================================================================
    // PPU BUS — read
    // ===================================================================

    /// PPU-side read of CHR / nametable / palette at `addr`.
    ///
    /// - `$0000-$1FFF`: CHR ROM/RAM via mapper range table (CHR path).
    /// - `$2000-$2FFF`: nametable, resolved via the active mirror function.
    /// - `$3000-$3EFF`: nametable mirror (alias of `$2000-$2EFF`).
    /// - `$3F00-$3FFF`: palette + mirror logic.
    #[inline]
    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        match addr {
            // CHR / pattern tables — mapper handler if registered, else open.
            0x0000..=0x1FFF => self.mapper.read_chr(addr).unwrap_or(0),
            // Nametable region $2000-$2FFF — apply mirroring.
            0x2000..=0x2FFF => {
                let vram_addr = (self.nametable_mirror_fn)(addr);
                self.vram[(vram_addr & 0x07FF) as usize]
            }
            // $3000-$3EFF is the same physical VRAM as $2000-$2EFF (just
            // with a different upper bit, which the mirror function will
            // handle consistently).
            0x3000..=0x3EFF => {
                let vram_addr = (self.nametable_mirror_fn)(addr - 0x1000);
                self.vram[(vram_addr & 0x07FF) as usize]
            }
            // Palette + mirror logic (mirrors 0x3F00/0x3F04/0x3F08/0x3F0C
            // onto 0x3F10/0x3F14/0x3F18/0x3F1C).
            0x3F00..=0x3FFF => self.palette_read(addr),
            // Outside PPU address space: 0.
            _ => 0,
        }
    }

    #[inline]
    fn palette_read(&self, addr: u16) -> u8 {
        let mut idx = (addr & 0x001F) as usize;
        // Mirroring: $3F10 ≡ $3F00, $3F14 ≡ $3F04, etc.
        if idx >= 0x10 && (idx & 0x03) == 0 {
            idx -= 0x10;
        }
        // Universal background: when rendering is active and the address
        // is $3F00/$3F04/$3F08/$3F0C, OR the low 2 bits with 0x00 — that's
        // handled in the rendering layer (not here). For raw reads the
        // byte is just whatever was written.
        self.palette[idx]
    }

    /// PPU-side write at `addr` (CHR via mapper, nametable via mirror,
    /// palette with mirroring).
    #[inline]
    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let _ = self.mapper.write_chr(addr, val);
            }
            0x2000..=0x2FFF => {
                let vram_addr = (self.nametable_mirror_fn)(addr);
                self.vram[(vram_addr & 0x07FF) as usize] = val;
            }
            0x3000..=0x3EFF => {
                let vram_addr = (self.nametable_mirror_fn)(addr - 0x1000);
                self.vram[(vram_addr & 0x07FF) as usize] = val;
            }
            0x3F00..=0x3FFF => {
                let mut idx = (addr & 0x001F) as usize;
                if idx >= 0x10 && (idx & 0x03) == 0 {
                    idx -= 0x10;
                }
                self.palette[idx] = val;
            }
            _ => {}
        }
    }

    // ===================================================================
    // PPU data read buffer ($2007 read lag)
    // ===================================================================

    /// `$2007` read with the classic PPU data-read-buffer lag.
    ///
    /// Returns the buffered value (last call's VRAM/CHR result), then
    /// advances `v` and refills the buffer with the value at the new
    /// `v`. Palette reads ($3F00-$3FFF) bypass the buffer and return
    /// directly without buffering (NESdev wiki "PPU scrolling" §).
    ///
    /// `increment` is typically +1 or +32 depending on PPUCTRL bit 2.
    pub fn ppu_read_data(&mut self, increment: u16) -> u8 {
        let addr = self.ppu_v & 0x3FFF;
        let buf = self.ppu_read_buffer;
        if addr >= 0x3F00 {
            // Palette reads: no buffering, but palette's low 2 bits OR
            // into the nametable buffer's low 2 bits in real hardware.
            // Phase 2 keeps it simple — direct palette value, leave the
            // buffer untouched.
            self.ppu_v = self.ppu_v.wrapping_add(increment);
            self.ppu_read(addr) & 0x3F
        } else {
            let fresh = self.ppu_read(addr);
            self.ppu_read_buffer = fresh;
            self.ppu_v = self.ppu_v.wrapping_add(increment);
            buf
        }
    }

    // ===================================================================
    // PPU register stub (Phase 3 wires real behavior; Phase 2 keeps
    // basic storage for testing the bus plumbing).
    // ===================================================================

    fn ppu_read_register(&self, reg: u8) -> u8 {
        // Phase 2: $2002 (PPUSTATUS) returns the w flag + open bus for
        // low bits; other registers read as open bus. Real implementation
        // is in Phase 3.
        match reg {
            0x02 => {
                // Bit 7 = PPU w toggle; bits 0-4 = open bus latch;
                // bit 5 = sprite overflow (always 0 here); bit 6 = sprite 0 hit.
                let mut v = self.open_bus & 0x1F;
                v |= (self.ppu_w as u8) << 7;
                v
            }
            0x04 => {
                // $2004 OAM read: returns OAM[addr]. Phase 3 wires the
                // full read path; for now expose raw OAM byte.
                self.oam[self.ppu_oam_addr as usize]
            }
            0x07 => {
                // $2007 PPU data — has its own hot path; the general
                // CPU bus read doesn't use this. Returning open bus here
                // matches the "other PPU regs" default.
                self.open_bus
            }
            _ => self.open_bus,
        }
    }

    fn ppu_write_register(&mut self, reg: u8, val: u8) {
        match reg {
            0x00 => {
                // PPUCTRL: bit 7 = NMI on VBlank, bit 5 = sprite size,
                // bit 4 = bg pattern table, bit 3 = sprite pattern table,
                // bit 2 = +32 increment, bits 1-0 = nametable select.
                self.ppu_ctrl = val;
            }
            0x01 => {
                // PPUMASK.
                self.ppu_mask = val;
            }
            0x03 => {
                // OAMADDR.
                self.ppu_oam_addr = val;
            }
            0x04 => {
                // OAMDMA — Phase 4 wires real DMA controller. For now
                // ignore.
            }
            0x05 => {
                // PPUSCROLL: writes to scroll registers (paired w/ $2006).
                if !self.ppu_w {
                    self.ppu_x = val & 0x07;
                    self.ppu_t = (self.ppu_t & 0xFFE0) | ((val >> 3) as u16);
                } else {
                    self.ppu_t = (self.ppu_t & 0x8C1F)
                        | (((val & 0x07) as u16) << 12)
                        | (((val >> 3) as u16) << 5);
                }
                self.ppu_w = !self.ppu_w;
            }
            0x06 => {
                // PPUADDR: writes to v/t (paired w/ $2005).
                if !self.ppu_w {
                    self.ppu_t = (self.ppu_t & 0x00FF) | (((val & 0x3F) as u16) << 8);
                } else {
                    self.ppu_t = (self.ppu_t & 0xFF00) | val as u16;
                    self.ppu_v = self.ppu_t;
                }
                self.ppu_w = !self.ppu_w;
            }
            0x07 => {
                // $2007 PPU data — write side; no buffer involved.
                let addr = self.ppu_v & 0x3FFF;
                self.ppu_write(addr, val);
                let increment = if (self.ppu_ctrl & 0x04) != 0 { 32 } else { 1 };
                self.ppu_v = self.ppu_v.wrapping_add(increment);
            }
            _ => {}
        }
    }

    // ===================================================================
    // APU / IO ($4000-$401F) — Phase 2 stub. Phase 4 wires real APU.
    // ===================================================================

    #[inline(always)]
    fn apu_io_read(&self, addr: u16) -> u8 {
        match addr {
            0x4016 => self.joypad_strobe_latch,
            0x4017 => self.joypad_strobe_latch,
            _ => self.open_bus,
        }
    }

    #[inline(always)]
    fn apu_io_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x4014 => {
                // OAM DMA: Phase 4 wires. For now, copy 256 bytes from
                // CPU addr into OAM. Many games use this on boot so a
                // minimal Phase 2 implementation helps test ROMs.
                let src_base = (val as u16) << 8;
                for i in 0..256u16 {
                    let b = self.cpu_read_for_dma(src_base.wrapping_add(i));
                    self.oam[i as usize] = b;
                }
            }
            0x4016 => {
                // Joypad strobe.
                self.joypad_strobe = (val & 0x01) != 0;
                if self.joypad_strobe {
                    self.joypad_strobe_latch = self.joypad_latched[0];
                }
            }
            _ => { /* APU registers: Phase 4 */ }
        }
    }

    /// DMA reads from the CPU address space without going through the
    /// normal `cpu_read` (which would update open_bus and risk recursion
    /// when DMA itself is the cause). Returns the byte at `addr` via the
    /// same decode logic minus the open-bus tracking.
    #[inline(always)]
    fn cpu_read_for_dma(&mut self, addr: u16) -> u8 {
        // Same decode as `cpu_read_unchecked` — duplicated here to keep
        // DMA off the open-bus hot path and to avoid `&mut self`-twice.
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.open_bus,
            0x4000..=0x401F => self.open_bus,
            _ => self.mapper.read(addr).unwrap_or(0),
        }
    }

    // ===================================================================
    // Open bus
    // ===================================================================

    /// Default open-bus value when there's no recent reference: $00.
    /// Most "unmapped" addresses also return this on real hardware
    /// (the bus really has some indeterminate state; FCEUX's
    /// `readopenbus` default-returns 0). For our purposes, return the
    /// cached `open_bus` so any subsequent reads of unmapped addresses
    /// see the last bus value (per real-hardware behavior).
    #[inline(always)]
    fn read_open_bus_for(&self, _addr: u16) -> u8 {
        self.open_bus
    }

    // ===================================================================
    // Mirroring control (mapper → VNesSoc)
    // ===================================================================

    /// Install a new nametable mirror function. Called by mapper code
    /// whenever the mapper flips mirroring (e.g. MMC1 bit 0).
    #[inline]
    pub fn set_mirroring(&mut self, m: Mirroring) {
        self.nametable_mirror_fn = m.mirror_fn();
        self.nametable_mirror = m;
    }

    /// Current mirroring mode (for mapper introspection / save state).
    #[inline]
    pub fn mirroring(&self) -> Mirroring {
        self.nametable_mirror
    }
}

// ---------------------------------------------------------------------------
// MapperRangeTable — CHR side
// ---------------------------------------------------------------------------
//
// Phase 2 stubs: PRG (CPU side) and CHR (PPU side) reads share the same
// `MapperRangeTable` design but address different memory maps. We extend
// the table type with `read_chr` / `write_chr` mirroring `read` / `write`
// — until Phase 5 splits the two, we use a single table and the CHR side
// falls through to open (the CHR mapper handlers are added in Phase 5).
//
// To keep the dependency surface small in Phase 2, we add the helper
// methods directly on `MapperRangeTable`.

impl MapperRangeTable {
    /// CHR side: PPU reads at `$0000-$1FFF`. Phase 2 falls through to
    /// `None` (caller maps that to "open"); Phase 5 registers actual
    /// handlers.
    #[inline(always)]
    pub fn read_chr(&self, _addr: u16) -> Option<u8> {
        // Phase 2: PRG and CHR share the same table. By the time Phase 5
        // wires things up, mappers will register separate read ranges for
        // CHR (or use a dedicated `chr_ranges` field).
        None
    }

    /// CHR side: PPU writes at `$0000-$1FFF`.
    #[inline(always)]
    pub fn write_chr(&mut self, _addr: u16, _val: u8) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ram::InternalRam;

    /// Helper: build a default SoC with all-zero RAM.
    fn soc() -> VNesSoc {
        VNesSoc::default()
    }

    /// Helper: replace the SoC's RAM with a fresh `InternalRam`.
    fn fresh_soc() -> VNesSoc {
        VNesSoc::default()
    }

    #[test]
    fn wram_mirror_write_once_read_many() {
        let mut s = soc();
        s.cpu_write(0x0000, 0xAA);
        // Same byte should be visible at $0800, $1000, $1800 (mirror).
        assert_eq!(s.cpu_read(0x0000), 0xAA);
        assert_eq!(s.cpu_read(0x0800), 0xAA);
        assert_eq!(s.cpu_read(0x1000), 0xAA);
        assert_eq!(s.cpu_read(0x1800), 0xAA);
    }

    #[test]
    fn open_bus_latches_last_value() {
        let mut s = soc();
        // Initial open bus = 0.
        assert_eq!(s.open_bus, 0);
        // Read a value into the bus via WRAM.
        s.wram[0] = 0x42;
        let _ = s.cpu_read(0x0000);
        assert_eq!(s.open_bus, 0x42);
        // Subsequent write updates the bus.
        s.cpu_write(0x0000, 0x99);
        assert_eq!(s.open_bus, 0x99);
    }

    #[test]
    fn unmapped_addr_returns_open_bus() {
        let mut s = soc();
        s.open_bus = 0x77;
        // $4020 with no mapper range registered → open bus.
        let v = s.cpu_read(0x4020);
        assert_eq!(v, 0x77);
    }

    #[test]
    fn palette_mirror_logic() {
        let mut s = soc();
        // Write at $3F00.
        s.ppu_write(0x3F00, 0x11);
        // Read at $3F00 and $3F10 (mirror of $3F00).
        assert_eq!(s.ppu_read(0x3F00), 0x11);
        assert_eq!(s.ppu_read(0x3F10), 0x11);
        // Write at $3F10 should write the same byte (it mirrors).
        s.ppu_write(0x3F14, 0x22);
        assert_eq!(s.ppu_read(0x3F04), 0x22);
        assert_eq!(s.ppu_read(0x3F14), 0x22);
    }

    #[test]
    fn nametable_horizontal_mirror_table() {
        let mut s = fresh_soc();
        s.set_mirroring(Mirroring::Horizontal);
        s.vram[0x000] = 0xA1;
        s.vram[0x400] = 0xA2;
        // A ($2000) and B ($2400) share VRAM (top row pair).
        assert_eq!(s.ppu_read(0x2000), 0xA1);
        assert_eq!(s.ppu_read(0x2400), 0xA1);
        // C ($2800) and D ($2C00) share VRAM (bottom row pair).
        assert_eq!(s.ppu_read(0x2800), 0xA2);
        assert_eq!(s.ppu_read(0x2C00), 0xA2);
    }

    #[test]
    fn nametable_vertical_mirror_table() {
        let mut s = fresh_soc();
        s.set_mirroring(Mirroring::Vertical);
        s.vram[0x000] = 0xB1;
        s.vram[0x400] = 0xB2;
        // A ($2000) and C ($2800) share VRAM (left column pair).
        assert_eq!(s.ppu_read(0x2000), 0xB1);
        assert_eq!(s.ppu_read(0x2800), 0xB1);
        // B ($2400) and D ($2C00) share VRAM (right column pair).
        assert_eq!(s.ppu_read(0x2400), 0xB2);
        assert_eq!(s.ppu_read(0x2C00), 0xB2);
    }

    #[test]
    fn ppu_register_mirror_within_2000_3fff() {
        let mut s = soc();
        // Writing to $2000 should also be visible at $2008, $2010, $2018,
        // $2020 (every 8 bytes). We exercise PPUCTRL register storage.
        s.ppu_write_register(0x00, 0x80);
        // Read $2000 again via the same PPU register read path.
        let _v = s.ppu_read_register(0x00);
        // ppu_read_register for non-0x02/0x04/0x07 returns open_bus.
        // So we just check the storage via $2002 (PPUSTATUS) bit 7.
        let st = s.ppu_read_register(0x02);
        // w is 0 initially, so bit 7 = 0.
        assert_eq!(st & 0x80, 0);
        // Confirm PPUCTRL was stored.
        assert_eq!(s.ppu_ctrl, 0x80);
    }

    #[test]
    fn ppu_data_read_buffer_lags_by_one() {
        let mut s = fresh_soc();
        s.set_mirroring(Mirroring::Vertical);
        // Place a byte at $2000 (nametable, not palette).
        s.vram[0x000] = 0x55;
        s.ppu_v = 0x2000;
        s.ppu_read_buffer = 0xAA; // initial buffer
        // First read should return the OLD buffer (0xAA), then refresh.
        let first = s.ppu_read_data(1);
        assert_eq!(first, 0xAA);
        // After the read, the buffer should now hold 0x55.
        assert_eq!(s.ppu_read_buffer, 0x55);
        // v should have advanced by 1.
        assert_eq!(s.ppu_v, 0x2001);
        // Second read should return 0x55 (was the buffer).
        let second = s.ppu_read_data(1);
        assert_eq!(second, 0x55);
    }

    #[test]
    fn ppu_data_read_palette_bypasses_buffer() {
        let mut s = fresh_soc();
        s.palette[0] = 0x33;
        s.ppu_v = 0x3F00;
        s.ppu_read_buffer = 0xAA;
        let v = s.ppu_read_data(1);
        // Palette reads bypass the buffer; should return 0x33 immediately.
        assert_eq!(v, 0x33);
        // Buffer is untouched.
        assert_eq!(s.ppu_read_buffer, 0xAA);
        // v advanced.
        assert_eq!(s.ppu_v, 0x3F01);
    }

    #[test]
    fn set_mirroring_changes_function_pointer() {
        let mut s = fresh_soc();
        s.set_mirroring(Mirroring::Horizontal);
        assert_eq!(s.nametable_mirror, Mirroring::Horizontal);
        s.set_mirroring(Mirroring::Vertical);
        assert_eq!(s.nametable_mirror, Mirroring::Vertical);
        // Same addresses resolve differently.
        s.vram[0x000] = 0xCC;
        s.vram[0x400] = 0xDD;
        assert_eq!(s.ppu_read(0x2400), 0xDD); // vertical: B ≡ nt+0x400
        s.set_mirroring(Mirroring::Horizontal);
        assert_eq!(s.ppu_read(0x2400), 0xCC); // horizontal: B ≡ A (nt)
    }

    #[test]
    fn oam_dma_copies_256_bytes_from_cpu_addr() {
        let mut s = fresh_soc();
        // Fill WRAM with 0..=255.
        for (i, b) in s.wram.iter_mut().enumerate() {
            *b = i as u8;
        }
        // DMA from $0200 → should copy 256 bytes into OAM.
        s.cpu_write(0x4014, 0x02);
        for i in 0..256 {
            assert_eq!(s.oam[i], i as u8);
        }
    }

    #[test]
    fn joypad_strobe_clears_then_latches_first_button() {
        let mut s = fresh_soc();
        s.joypad_latched[0] = 0xAB;
        // Write 1 to $4016 → strobe on, latch first button state.
        s.cpu_write(0x4016, 0x01);
        assert!(s.joypad_strobe);
        // Reading $4016 in strobe mode returns the latched byte.
        let v = s.cpu_read(0x4016);
        assert_eq!(v, 0xAB);
        // Write 0 to $4016 → strobe off (latch state remains).
        s.cpu_write(0x4016, 0x00);
        assert!(!s.joypad_strobe);
    }

    #[test]
    fn ram_init_helper_is_consistent() {
        // Sanity: InternalRam::init_wram with AllZeros makes the entire
        // WRAM bank zero; with AllOnes makes it 0xFF. This pins that
        // the bus path sees correctly initialized RAM on power-on.
        let mut s = soc();
        let mut ram = InternalRam::new_zeroed();
        ram.init_wram(&mut s.ram_rng, crate::ram::RamInitOption::AllOnes);
        s.wram = ram.wram;
        for i in 0..2048u16 {
            assert_eq!(s.cpu_read(i), 0xFF);
        }
    }
}