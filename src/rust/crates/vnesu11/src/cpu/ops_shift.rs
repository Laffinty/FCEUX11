//! Shift / rotate instructions: ASL, ROL, LSR, ROR (accumulator + memory).

use super::flags::{set_zn, C_FLAG};
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // -----------------------------------------------------------------
    // ASL — arithmetic shift left. 5 modes (A + 4 memory).
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn asl_a(&mut self) {
        let old_c = self.p & C_FLAG;
        let r = self.a << 1;
        self.p = if self.a & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        let _ = old_c;
        self.a = r;
        self.p = set_zn(self.p, r);
    }

    #[inline(always)]
    fn asl_mem<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let r = v << 1;
        self.p = if v & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        bus.write(addr, r);
        self.p = set_zn(self.p, r);
    }

    #[inline(always)] pub(crate) fn asl_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.asl_mem(a, bus); }
    #[inline(always)] pub(crate) fn asl_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.asl_mem(a, bus); }
    #[inline(always)] pub(crate) fn asl_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.asl_mem(a, bus); }
    #[inline(always)] pub(crate) fn asl_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absx(bus);
        self.asl_mem(a, bus);
    }

    // -----------------------------------------------------------------
    // ROL — rotate left through carry. 5 modes.
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn rol_a(&mut self) {
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (self.a << 1) | old_c;
        self.p = if self.a & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.a = r;
        self.p = set_zn(self.p, r);
    }

    #[inline(always)]
    fn rol_mem<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (v << 1) | old_c;
        self.p = if v & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        bus.write(addr, r);
        self.p = set_zn(self.p, r);
    }

    #[inline(always)] pub(crate) fn rol_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.rol_mem(a, bus); }
    #[inline(always)] pub(crate) fn rol_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.rol_mem(a, bus); }
    #[inline(always)] pub(crate) fn rol_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.rol_mem(a, bus); }
    #[inline(always)] pub(crate) fn rol_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absx(bus);
        self.rol_mem(a, bus);
    }

    // -----------------------------------------------------------------
    // LSR — logical shift right. 5 modes.
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn lsr_a(&mut self) {
        self.p = if self.a & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        let r = self.a >> 1;
        self.a = r;
        self.p = set_zn(self.p, r);
    }

    #[inline(always)]
    fn lsr_mem<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        self.p = if v & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        let r = v >> 1;
        bus.write(addr, r);
        self.p = set_zn(self.p, r);
    }

    #[inline(always)] pub(crate) fn lsr_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.lsr_mem(a, bus); }
    #[inline(always)] pub(crate) fn lsr_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.lsr_mem(a, bus); }
    #[inline(always)] pub(crate) fn lsr_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.lsr_mem(a, bus); }
    #[inline(always)] pub(crate) fn lsr_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absx(bus);
        self.lsr_mem(a, bus);
    }

    // -----------------------------------------------------------------
    // ROR — rotate right through carry. 5 modes.
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn ror_a(&mut self) {
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (self.a >> 1) | (old_c << 7);
        self.p = if self.a & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.a = r;
        self.p = set_zn(self.p, r);
    }

    #[inline(always)]
    fn ror_mem<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (v >> 1) | (old_c << 7);
        self.p = if v & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        bus.write(addr, r);
        self.p = set_zn(self.p, r);
    }

    #[inline(always)] pub(crate) fn ror_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.ror_mem(a, bus); }
    #[inline(always)] pub(crate) fn ror_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.ror_mem(a, bus); }
    #[inline(always)] pub(crate) fn ror_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.ror_mem(a, bus); }
    #[inline(always)] pub(crate) fn ror_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absx(bus);
        self.ror_mem(a, bus);
    }
}
