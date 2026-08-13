//! 6502 addressing modes — effective-address computation + page-cross.
//!
//! Each method advances `pc` past the operand bytes and returns the
//! effective address (or value for immediate). Page-cross flags are
//! returned where the CPU adds a +1 cycle penalty:
//!
//! * reads via abs,X / abs,Y / (ind),Y
//! * taken branches whose target crosses a page
//! * RMW via abs,X / abs,Y always (+1)
//!
//! Zero-page indexed modes wrap within the zero page (no penalty).
//!
//! The `read_*` helpers combine address computation + memory read in
//! SEPARATE statements so Rust's borrow checker accepts the pattern
//! (avoiding E0499 from nesting `bus.read(self.zp(bus))` in one expr).

use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // -----------------------------------------------------------------
    // Immediate / implied (no address computation)
    // -----------------------------------------------------------------

    /// Read the immediate byte and advance PC.
    #[inline(always)]
    pub(crate) fn imm<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let b = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        b
    }

    // -----------------------------------------------------------------
    // Zero-page (no page-cross; wraps within 0x00-0xFF)
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn zp<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let a = bus.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        a
    }

    #[inline(always)]
    pub(crate) fn zpx<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let a = bus.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        a.wrapping_add(self.x as u16) & 0xFF
    }

    #[inline(always)]
    pub(crate) fn zpy<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let a = bus.read(self.pc) as u16;
        self.pc = self.pc.wrapping_add(1);
        a.wrapping_add(self.y as u16) & 0xFF
    }

    // -----------------------------------------------------------------
    // Absolute (16-bit; may page-cross with X/Y)
    // -----------------------------------------------------------------

    /// Absolute address, no penalty (used for writes and JMP).
    #[inline(always)]
    pub(crate) fn abs<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let lo = bus.read(self.pc) as u16;
        let hi = bus.read(self.pc.wrapping_add(1)) as u16;
        self.pc = self.pc.wrapping_add(2);
        lo | (hi << 8)
    }

    /// Absolute,X — returns (addr, page_crossed).
    #[inline(always)]
    pub(crate) fn absx<BC: BusContext>(&mut self, bus: &mut BC) -> (u16, bool) {
        let base = self.abs(bus);
        let crossed = (base & 0xFF) + self.x as u16 > 0xFF;
        (base.wrapping_add(self.x as u16), crossed)
    }

    /// Absolute,Y — returns (addr, page_crossed).
    #[inline(always)]
    pub(crate) fn absy<BC: BusContext>(&mut self, bus: &mut BC) -> (u16, bool) {
        let base = self.abs(bus);
        let crossed = (base & 0xFF) + self.y as u16 > 0xFF;
        (base.wrapping_add(self.y as u16), crossed)
    }

    /// RMW abs,X effective address: does the C++ `GetABIWR` dummy read of
    /// the OLD-page address (always, not just on page-cross), then
    /// returns the corrected address. RMW abs,X has NO extra page-cross
    /// cycle penalty — the base table already carries the +1.
    #[inline(always)]
    pub(crate) fn rmw_absx<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let (a, crossed) = self.absx(bus);
        let dummy = if crossed { (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF) } else { a };
        let _ = bus.read(dummy);
        a
    }

    /// RMW abs,Y effective address (same dummy-read semantics as abs,X).
    #[inline(always)]
    pub(crate) fn rmw_absy<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let (a, crossed) = self.absy(bus);
        let dummy = if crossed { (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF) } else { a };
        let _ = bus.read(dummy);
        a
    }

    /// RMW (indirect),Y effective address. C++ `GetIYWR`: dummy-read the
    /// OLD-page address and add NO page-cross cycle (the base table
    /// already carries the RMW timing). The regular `izy` helper is for
    /// reads and DOES add the +1 penalty — RMW must not.
    #[inline(always)]
    pub(crate) fn rmw_izy<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let (a, crossed) = self.izy(bus);
        let dummy = if crossed { (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF) } else { a };
        let _ = bus.read(dummy);
        a
    }

    // -----------------------------------------------------------------
    // Indirect (JMP only)
    // -----------------------------------------------------------------

    /// (absolute indirect) — includes the famous 6502 page-wrap bug:
    /// if `base & 0xFF == 0xFF`, the high byte is read from the same
    /// page (base & 0xFF00), NOT base+1 across the page boundary.
    #[inline(always)]
    pub(crate) fn indirect<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let base = self.abs(bus);
        let lo = bus.read(base) as u16;
        // 6502 bug: high byte address wraps within the page.
        let hi_addr = (base & 0xFF00) | ((base.wrapping_add(1)) & 0x00FF);
        let hi = bus.read(hi_addr) as u16;
        lo | (hi << 8)
    }

    // -----------------------------------------------------------------
    // Indexed indirect (zero-page vector)
    // -----------------------------------------------------------------

    /// (Indirect,X): vector at (zp + X), wraps in zero page.
    #[inline(always)]
    pub(crate) fn izx<BC: BusContext>(&mut self, bus: &mut BC) -> u16 {
        let zp_base = self.zpx(bus); // already & 0xFF
        self.read_vector(bus, zp_base)
    }

    /// (Indirect),Y: vector at zp, add Y. Returns (addr, page_crossed).
    #[inline(always)]
    pub(crate) fn izy<BC: BusContext>(&mut self, bus: &mut BC) -> (u16, bool) {
        let zp_base = self.zp(bus);
        let base = self.read_vector(bus, zp_base);
        let crossed = (base & 0xFF) + self.y as u16 > 0xFF;
        (base.wrapping_add(self.y as u16), crossed)
    }

    /// Read a 16-bit vector at a zero-page address (wraps within page 0).
    #[inline(always)]
    fn read_vector<BC: BusContext>(&mut self, bus: &mut BC, zp_addr: u16) -> u16 {
        let lo = bus.read(zp_addr) as u16;
        let hi = bus.read((zp_addr.wrapping_add(1)) & 0xFF) as u16;
        lo | (hi << 8)
    }

    // -----------------------------------------------------------------
    // Relative (branches)
    // -----------------------------------------------------------------

    /// Read the branch offset, compute target, and advance PC past it.
    /// Returns (target, crossed_page).
    #[inline(always)]
    pub(crate) fn relative<BC: BusContext>(&mut self, bus: &mut BC) -> (u16, bool) {
        let off = self.imm(bus) as i8;
        let base = self.pc;
        let target = base.wrapping_add(off as i16 as u16);
        let crossed = (base & 0xFF00) != (target & 0xFF00);
        (target, crossed)
    }

    // -----------------------------------------------------------------
    // Read helpers — combine address computation + bus read in separate
    // statements (avoids E0499). Page-cross penalty applied inline.
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn read_zp<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let a = self.zp(bus);
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_zpx<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let a = self.zpx(bus);
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_zpy<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let a = self.zpy(bus);
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_abs<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let a = self.abs(bus);
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_absx<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let (a, crossed) = self.absx(bus);
        if crossed {
            self.count -= 1;
            // Page-cross dummy read: the 6502 first reads the address
            // with the OLD (pre-carry) page, then re-reads the correct
            // page. The value is discarded but the open-bus side effect
            // is observable (blargg instr_misc_03_dummy).
            let dummy = (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF);
            let _ = bus.read(dummy);
        }
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_absy<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let (a, crossed) = self.absy(bus);
        if crossed {
            self.count -= 1;
            let dummy = (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF);
            let _ = bus.read(dummy);
        }
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_izx<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let a = self.izx(bus);
        bus.read(a)
    }

    #[inline(always)]
    pub(crate) fn read_izy<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        let (a, crossed) = self.izy(bus);
        if crossed {
            self.count -= 1;
            let dummy = (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF);
            let _ = bus.read(dummy);
        }
        bus.read(a)
    }

    // -----------------------------------------------------------------
    // Write helpers — compute address + bus write (no page-cross
    // penalty on writes).
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn write_zp<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let a = self.zp(bus);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_zpx<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let a = self.zpx(bus);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_zpy<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let a = self.zpy(bus);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_abs<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let a = self.abs(bus);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_absx<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let (a, crossed) = self.absx(bus);
        // STA abs,X always does a dummy read before the write. On a
        // page-cross the dummy targets the OLD (pre-carry) page; the
        // write then lands on the correct page.
        let dummy = if crossed { (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF) } else { a };
        let _ = bus.read(dummy);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_absy<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let (a, crossed) = self.absy(bus);
        let dummy = if crossed { (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF) } else { a };
        let _ = bus.read(dummy);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_izx<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let a = self.izx(bus);
        bus.write(a, val);
    }

    #[inline(always)]
    pub(crate) fn write_izy<BC: BusContext>(&mut self, val: u8, bus: &mut BC) {
        let (a, crossed) = self.izy(bus);
        let dummy = if crossed { (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF) } else { a };
        let _ = bus.read(dummy);
        bus.write(a, val);
    }
}
