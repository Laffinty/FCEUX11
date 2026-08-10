//! System instructions: BRK, NOP, BIT + undocumented ("illegal") opcodes.

use super::flags::{self, C_FLAG, N_FLAG, V_FLAG, Z_FLAG};
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // =================================================================
    // Official
    // =================================================================

    /// BRK — force interrupt, push PC+1 + P (B set), vector 0xFFFE.
    #[inline(always)]
    pub(crate) fn brk<BC: BusContext>(&mut self, bus: &mut BC) {
        let ret = self.pc.wrapping_add(1);
        self.push(bus, (ret >> 8) as u8);
        self.push(bus, (ret & 0xFF) as u8);
        self.push(bus, self.p | flags::U_FLAG | flags::B_FLAG);
        self.p |= flags::I_FLAG;
        self.pc = self.read16(bus, 0xFFFE);
    }

    /// NOP — no-op (implied).
    #[inline(always)]
    pub(crate) fn nop(&mut self) {}

    /// NOP that consumes the immediate operand byte (unofficial NOPs).
    #[inline(always)]
    pub(crate) fn nop_imm<BC: BusContext>(&mut self, bus: &mut BC) {
        let _ = self.imm(bus);
    }

    /// BIT — test bits, sets Z/N/V. 2 modes.
    #[inline(always)]
    pub(crate) fn bit_common(&mut self, val: u8) {
        let r = self.a & val;
        self.p = if r == 0 { self.p | Z_FLAG } else { self.p & !Z_FLAG };
        self.p = if val & N_FLAG != 0 { self.p | N_FLAG } else { self.p & !N_FLAG };
        self.p = if val & V_FLAG != 0 { self.p | V_FLAG } else { self.p & !V_FLAG };
    }
    #[inline(always)] pub(crate) fn bit_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.bit_common(v); }
    #[inline(always)] pub(crate) fn bit_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.bit_common(v); }

    // =================================================================
    // Undocumented (unofficial) opcodes
    // =================================================================

    /// LAX — load A and X from memory (read, sets Z/N).
    #[inline(always)]
    pub(crate) fn lax_common(&mut self, v: u8) {
        self.a = v;
        self.x = v;
        self.p = flags::set_zn(self.p, v);
    }

    /// DCP — DEC memory then CMP. RMW.
    #[inline(always)]
    fn dcp_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let r = v.wrapping_sub(1);
        bus.write(addr, r);
        let cmp = self.a.wrapping_sub(r);
        self.p = flags::set_zn(self.p, cmp);
        self.p = if self.a >= r { self.p | C_FLAG } else { self.p & !C_FLAG };
    }

    /// ISB (ISC) — INC memory then SBC. RMW.
    #[inline(always)]
    fn isb_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let r = v.wrapping_add(1);
        bus.write(addr, r);
        self.sbc_common(r);
    }

    /// SLO — ASL memory then ORA. RMW.
    #[inline(always)]
    fn slo_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let r = v << 1;
        self.p = if v & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        bus.write(addr, r);
        self.a |= r;
        self.p = flags::set_zn(self.p, self.a);
    }

    /// RLA — ROL memory then AND. RMW.
    #[inline(always)]
    fn rla_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (v << 1) | old_c;
        self.p = if v & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        bus.write(addr, r);
        self.a &= r;
        self.p = flags::set_zn(self.p, self.a);
    }

    /// SRE — LSR memory then EOR. RMW.
    #[inline(always)]
    fn sre_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        self.p = if v & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        let r = v >> 1;
        bus.write(addr, r);
        self.a ^= r;
        self.p = flags::set_zn(self.p, self.a);
    }

    /// RRA — ROR memory then ADC. RMW.
    #[inline(always)]
    fn rra_at<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let v = bus.read(addr);
        bus.write(addr, v); // RMW dummy write
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (v >> 1) | (old_c << 7);
        self.p = if v & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        bus.write(addr, r);
        self.adc_common(r);
    }

    /// ANC — AND #imm then set C from bit 7, clear V. (0x0B/0x2B)
    #[inline(always)]
    pub(crate) fn anc<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.imm(bus);
        self.a &= v;
        self.p = flags::set_zn(self.p, self.a);
        self.p = if self.a & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.p &= !V_FLAG;
    }

    /// ARR — AND #imm then ROR A, with NES-specific V flag. (0x4B)
    #[inline(always)]
    pub(crate) fn arr<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.imm(bus);
        self.a &= v;
        let old_c = (self.p & C_FLAG != 0) as u8;
        let r = (self.a >> 1) | (old_c << 7);
        self.a = r;
        let v6 = (r >> 6) & 1;
        let v5 = (r >> 5) & 1;
        self.p = if v6 == 1 { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.p = if v6 != v5 { self.p | V_FLAG } else { self.p & !V_FLAG };
        self.p = flags::set_zn(self.p, r);
    }

    /// XAA — A = X & imm. (0x8B) — nestest-accepted behavior.
    #[inline(always)]
    pub(crate) fn xaa<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.imm(bus);
        self.a = self.x & v;
        self.p = flags::set_zn(self.p, self.a);
    }

    /// LAS — LDA & TSX from memory (0xBB): A = X = S = mem & S.
    #[inline(always)]
    pub(crate) fn las<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.read_absy(bus);
        let r = v & self.s;
        self.a = r;
        self.x = r;
        self.s = r;
        self.p = flags::set_zn(self.p, r);
    }

    /// AXS (SBX) — X = (A & X) - imm. (0xCB)
    #[inline(always)]
    pub(crate) fn axs<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.imm(bus);
        let t = self.a & self.x;
        let r = t.wrapping_sub(v);
        self.x = r;
        self.p = flags::set_zn(self.p, r);
        self.p = if t >= v { self.p | C_FLAG } else { self.p & !C_FLAG };
    }

    /// SHA (AHX) — store A & X & (high+1).
    #[inline(always)]
    pub(crate) fn sha_write<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let hi = ((addr >> 8) as u8).wrapping_add(1);
        let v = self.a & self.x & hi;
        bus.write(addr, v);
    }

    /// SHX (XAS) — store X & (high+1).
    #[inline(always)]
    pub(crate) fn shx_write<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let hi = ((addr >> 8) as u8).wrapping_add(1);
        bus.write(addr, self.x & hi);
    }

    /// SHY — store Y & (high+1).
    #[inline(always)]
    pub(crate) fn shy_write<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        let hi = ((addr >> 8) as u8).wrapping_add(1);
        bus.write(addr, self.y & hi);
    }

    /// TAS (SHS) — S = A & X; store S & (high+1).
    #[inline(always)]
    pub(crate) fn tas<BC: BusContext>(&mut self, addr: u16, bus: &mut BC) {
        self.s = self.a & self.x;
        let hi = ((addr >> 8) as u8).wrapping_add(1);
        bus.write(addr, self.s & hi);
    }

    /// KIL ($02 etc.) — jam the CPU.
    #[inline(always)]
    pub(crate) fn kil(&mut self) {
        self.jammed = true;
        self.pc = self.pc.wrapping_sub(1);
    }

    // =================================================================
    // Addressing-mode wrappers for undocumented ops
    // =================================================================

    // LAX
    #[inline(always)] pub(crate) fn lax_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.lax_common(v); }
    #[inline(always)] pub(crate) fn lax_zpy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpy(bus); self.lax_common(v); }
    #[inline(always)] pub(crate) fn lax_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.lax_common(v); }
    #[inline(always)] pub(crate) fn lax_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.lax_common(v); }
    #[inline(always)] pub(crate) fn lax_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.lax_common(v); }
    #[inline(always)] pub(crate) fn lax_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.lax_common(v); }

    // SAX
    #[inline(always)] pub(crate) fn sax_zp<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zp(self.a & self.x, bus); }
    #[inline(always)] pub(crate) fn sax_zpy<BC: BusContext>(&mut self, bus: &mut BC) { self.write_zpy(self.a & self.x, bus); }
    #[inline(always)] pub(crate) fn sax_abs<BC: BusContext>(&mut self, bus: &mut BC) { self.write_abs(self.a & self.x, bus); }
    #[inline(always)] pub(crate) fn sax_izx<BC: BusContext>(&mut self, bus: &mut BC) { self.write_izx(self.a & self.x, bus); }

    // DCP
    #[inline(always)] pub(crate) fn dcp_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_absx<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absx(bus); if c { self.count -= 1; } self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_absy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absy(bus); if c { self.count -= 1; } self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_izy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.izy(bus); if c { self.count -= 1; } self.dcp_at(a, bus); }

    // ISB
    #[inline(always)] pub(crate) fn isb_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_absx<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absx(bus); if c { self.count -= 1; } self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_absy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absy(bus); if c { self.count -= 1; } self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_izy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.izy(bus); if c { self.count -= 1; } self.isb_at(a, bus); }

    // SLO
    #[inline(always)] pub(crate) fn slo_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_absx<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absx(bus); if c { self.count -= 1; } self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_absy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absy(bus); if c { self.count -= 1; } self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_izy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.izy(bus); if c { self.count -= 1; } self.slo_at(a, bus); }

    // RLA
    #[inline(always)] pub(crate) fn rla_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_absx<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absx(bus); if c { self.count -= 1; } self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_absy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absy(bus); if c { self.count -= 1; } self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_izy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.izy(bus); if c { self.count -= 1; } self.rla_at(a, bus); }

    // SRE
    #[inline(always)] pub(crate) fn sre_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_absx<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absx(bus); if c { self.count -= 1; } self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_absy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absy(bus); if c { self.count -= 1; } self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_izy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.izy(bus); if c { self.count -= 1; } self.sre_at(a, bus); }

    // RRA
    #[inline(always)] pub(crate) fn rra_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_absx<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absx(bus); if c { self.count -= 1; } self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_absy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.absy(bus); if c { self.count -= 1; } self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_izy<BC: BusContext>(&mut self, bus: &mut BC) { let (a, c) = self.izy(bus); if c { self.count -= 1; } self.rra_at(a, bus); }

    // =================================================================
    // SHA / SHX / SHY / TAS — undocumented stores with page-cross
    // behaviour. These use write helpers so the borrow checker accepts
    // them (each computes the address, then writes in a separate stmt).
    // =================================================================

    #[inline(always)]
    pub(crate) fn sha_absy<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, _) = self.absy(bus);
        self.sha_write(a, bus);
    }
    #[inline(always)]
    pub(crate) fn sha_izy<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, _) = self.izy(bus);
        self.sha_write(a, bus);
    }
    #[inline(always)]
    pub(crate) fn shx_absy<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, _) = self.absy(bus);
        self.shx_write(a, bus);
    }
    #[inline(always)]
    pub(crate) fn shy_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, _) = self.absx(bus);
        self.shy_write(a, bus);
    }
    #[inline(always)]
    pub(crate) fn tas_absy<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, _) = self.absy(bus);
        self.tas(a, bus);
    }
}
