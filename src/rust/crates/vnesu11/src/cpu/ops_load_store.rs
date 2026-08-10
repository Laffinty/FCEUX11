//! Load / store instructions: LDA, LDX, LDY, STA, STX, STY.

use super::flags::set_zn;
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // -----------------------------------------------------------------
    // LDA (load accumulator) — 8 addressing modes
    // -----------------------------------------------------------------

    #[inline(always)] pub(crate) fn lda_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.a = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn lda_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.a = v; self.p = set_zn(self.p, v); }

    // -----------------------------------------------------------------
    // LDX — 5 modes
    // -----------------------------------------------------------------

    #[inline(always)] pub(crate) fn ldx_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.x = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldx_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.x = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldx_zpy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpy(bus); self.x = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldx_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.x = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldx_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.x = v; self.p = set_zn(self.p, v); }

    // -----------------------------------------------------------------
    // LDY — 5 modes
    // -----------------------------------------------------------------

    #[inline(always)] pub(crate) fn ldy_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.y = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldy_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.y = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldy_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.y = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldy_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.y = v; self.p = set_zn(self.p, v); }
    #[inline(always)] pub(crate) fn ldy_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.y = v; self.p = set_zn(self.p, v); }

    // -----------------------------------------------------------------
    // STA — 7 modes (no flags; no page-cross penalty on write)
    // -----------------------------------------------------------------

    #[inline(always)] pub(crate) fn sta_zp<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zp(self.a, bus); }
    #[inline(always)] pub(crate) fn sta_zpx<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zpx(self.a, bus); }
    #[inline(always)] pub(crate) fn sta_abs<BC: BusContext>(&mut self, bus: &mut BC) { self.write_abs(self.a, bus); }
    #[inline(always)] pub(crate) fn sta_absx<BC: BusContext>(&mut self, bus: &mut BC) { self.write_absx(self.a, bus); }
    #[inline(always)] pub(crate) fn sta_absy<BC: BusContext>(&mut self, bus: &mut BC) { self.write_absy(self.a, bus); }
    #[inline(always)] pub(crate) fn sta_izx<BC: BusContext>(&mut self, bus: &mut BC) { self.write_izx(self.a, bus); }
    #[inline(always)] pub(crate) fn sta_izy<BC: BusContext>(&mut self, bus: &mut BC) { self.write_izy(self.a, bus); }

    // -----------------------------------------------------------------
    // STX — 3 modes
    // -----------------------------------------------------------------

    #[inline(always)] pub(crate) fn stx_zp<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zp(self.x, bus); }
    #[inline(always)] pub(crate) fn stx_zpy<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zpy(self.x, bus); }
    #[inline(always)] pub(crate) fn stx_abs<BC: BusContext>(&mut self, bus: &mut BC) { self.write_abs(self.x, bus); }

    // -----------------------------------------------------------------
    // STY — 3 modes
    // -----------------------------------------------------------------

    #[inline(always)] pub(crate) fn sty_zp<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zp(self.y, bus); }
    #[inline(always)] pub(crate) fn sty_zpx<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zpx(self.y, bus); }
    #[inline(always)] pub(crate) fn sty_abs<BC: BusContext>(&mut self, bus: &mut BC) { self.write_abs(self.y, bus); }
}
