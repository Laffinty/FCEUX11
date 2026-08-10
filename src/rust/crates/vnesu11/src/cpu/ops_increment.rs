//! Increment / decrement instructions: INC, DEC (memory), INX, INY,
//! DEX, DEY (registers).

use super::flags::set_zn;
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // -----------------------------------------------------------------
    // INC (memory) — RMW, no carry, sets Z/N. 6 modes.
    // -----------------------------------------------------------------

    #[inline(always)]
    fn inc_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let r = v.wrapping_add(1);
        bus.write(addr, r);
        self.p = set_zn(self.p, r);
    }

    #[inline(always)] pub(crate) fn inc_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.inc_at(a, bus); }
    #[inline(always)] pub(crate) fn inc_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.inc_at(a, bus); }
    #[inline(always)] pub(crate) fn inc_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.inc_at(a, bus); }
    #[inline(always)] pub(crate) fn inc_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, c) = self.absx(bus);
        if c { self.count -= 1; } // RMW abs,X always +1
        self.inc_at(a, bus);
    }

    // -----------------------------------------------------------------
    // DEC (memory) — RMW, no carry, sets Z/N. 6 modes.
    // -----------------------------------------------------------------

    #[inline(always)]
    fn dec_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let r = v.wrapping_sub(1);
        bus.write(addr, r);
        self.p = set_zn(self.p, r);
    }

    #[inline(always)] pub(crate) fn dec_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.dec_at(a, bus); }
    #[inline(always)] pub(crate) fn dec_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.dec_at(a, bus); }
    #[inline(always)] pub(crate) fn dec_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.dec_at(a, bus); }
    #[inline(always)] pub(crate) fn dec_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, c) = self.absx(bus);
        if c { self.count -= 1; } // RMW abs,X always +1
        self.dec_at(a, bus);
    }

    // -----------------------------------------------------------------
    // INX / INY / DEX / DEY (register)
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn inx(&mut self) {
        self.x = self.x.wrapping_add(1);
        self.p = set_zn(self.p, self.x);
    }

    #[inline(always)]
    pub(crate) fn iny(&mut self) {
        self.y = self.y.wrapping_add(1);
        self.p = set_zn(self.p, self.y);
    }

    #[inline(always)]
    pub(crate) fn dex(&mut self) {
        self.x = self.x.wrapping_sub(1);
        self.p = set_zn(self.p, self.x);
    }

    #[inline(always)]
    pub(crate) fn dey(&mut self) {
        self.y = self.y.wrapping_sub(1);
        self.p = set_zn(self.p, self.y);
    }
}
