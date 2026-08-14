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
        // 6502 does a dummy read of the byte after the opcode (64doc BRK
        // cycle 2) and advances PC past it, then pushes that address.
        // The read has bus side effects (same as RTS/RTI dummy reads).
        let _ = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        let ret = self.pc;
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

    /// NOP absolute (0x0C) — reads the operand address and discards it.
    #[inline(always)]
    pub(crate) fn nop_abs<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.abs(bus);
        let _ = bus.read(a);
    }

    /// NOP absolute,X (0x1C/0x3C/0x5C/0x7C/0xDC/0xFC) — reads the
    /// operand address (page-cross penalty applies) and discards it.
    /// On a page cross the 6502 also performs a dummy read of the OLD
    /// (pre-carry) page before reading the effective address (C++
    /// `GetABIRD`).
    #[inline(always)]
    pub(crate) fn nop_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let (a, crossed) = self.absx(bus);
        if crossed {
            self.count -= 1;
            let dummy = (a & 0xFF00).wrapping_sub(0x100) | (a & 0xFF);
            let _ = bus.read(dummy);
        }
        let _ = bus.read(a);
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

    /// ANC — AND #imm then set C from bit 7. (0x0B/0x2B)
    ///
    /// C++ `ops.inc`: `LD_IM(AND;_P&=~C_FLAG;_P|=_A>>7)`. N/Z come
    /// from the AND, C from bit 7 of the result; the V flag is NOT
    /// modified (the previous Rust implementation wrongly cleared it).
    #[inline(always)]
    pub(crate) fn anc<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.imm(bus);
        self.a &= v;
        self.p = flags::set_zn(self.p, self.a);
        self.p = if self.a & 0x80 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
    }

    /// ARR — AND #imm then ROR A, with NES-specific V flag. (0x6B)
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

    /// ALR (ASR) — AND #imm then LSR A. (0x4B)
    ///
    /// C++ `ops.inc`: `LD_IM(AND;LSRA)` — C gets bit 0 of the AND
    /// result, then A shifts right (N cleared, Z from the result).
    #[inline(always)]
    pub(crate) fn alr<BC: BusContext>(&mut self, bus: &mut BC) {
        let v = self.imm(bus);
        self.a &= v;
        self.p = if self.a & 0x01 != 0 { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.a >>= 1;
        self.p = flags::set_zn(self.p, self.a);
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

    /// SHA (AHX) — store A & X & (base_high+1). C++ uses the BASE
    /// address high byte (before the index), not the effective one.
    #[inline(always)]
    pub(crate) fn sha_write<BC: BusContext>(&mut self, addr: u16, base_hi: u8, bus: &mut BC) {
        let v = self.a & self.x & base_hi.wrapping_add(1);
        bus.write(addr, v);
    }

    /// SXA (0x9E) — write X & (EA_high+1) to the quirk address
    /// `(value_hi << 8) | EA_low`. C++ `ops.inc` builds the value then
    /// writes its high byte to the value-as-address.
    #[inline(always)]
    pub(crate) fn shx_write<BC: BusContext>(&mut self, ea: u16, bus: &mut BC) {
        let hi = ((ea >> 8) as u8).wrapping_add(1);
        let val_hi = self.x & hi;
        let addr = ((val_hi as u16) << 8) | (ea & 0xFF);
        bus.write(addr, val_hi);
    }

    /// SYA (0x9C) — write Y & (EA_high+1) to the quirk address
    /// `(value_hi << 8) | EA_low`.
    #[inline(always)]
    pub(crate) fn shy_write<BC: BusContext>(&mut self, ea: u16, bus: &mut BC) {
        let hi = ((ea >> 8) as u8).wrapping_add(1);
        let val_hi = self.y & hi;
        let addr = ((val_hi as u16) << 8) | (ea & 0xFF);
        bus.write(addr, val_hi);
    }

    /// TAS/XAS (0x9B) — S = A & X; store S & (base_high+1).
    #[inline(always)]
    pub(crate) fn tas<BC: BusContext>(&mut self, addr: u16, base_hi: u8, bus: &mut BC) {
        self.s = self.a & self.x;
        bus.write(addr, self.s & base_hi.wrapping_add(1));
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
    #[inline(always)] pub(crate) fn lax_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.lax_common(v); }
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
    #[inline(always)] pub(crate) fn dcp_absx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absx(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_absy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absy(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.dcp_at(a, bus); }
    #[inline(always)] pub(crate) fn dcp_izy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_izy(bus); self.dcp_at(a, bus); }

    // ISB
    #[inline(always)] pub(crate) fn isb_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_absx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absx(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_absy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absy(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.isb_at(a, bus); }
    #[inline(always)] pub(crate) fn isb_izy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_izy(bus); self.isb_at(a, bus); }

    // SLO
    #[inline(always)] pub(crate) fn slo_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_absx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absx(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_absy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absy(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.slo_at(a, bus); }
    #[inline(always)] pub(crate) fn slo_izy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_izy(bus); self.slo_at(a, bus); }

    // RLA
    #[inline(always)] pub(crate) fn rla_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_absx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absx(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_absy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absy(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.rla_at(a, bus); }
    #[inline(always)] pub(crate) fn rla_izy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_izy(bus); self.rla_at(a, bus); }

    // SRE
    #[inline(always)] pub(crate) fn sre_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_absx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absx(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_absy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absy(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.sre_at(a, bus); }
    #[inline(always)] pub(crate) fn sre_izy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_izy(bus); self.sre_at(a, bus); }

    // RRA
    #[inline(always)] pub(crate) fn rra_zp<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zp(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.zpx(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_abs<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.abs(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_absx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absx(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_absy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_absy(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_izx<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.izx(bus); self.rra_at(a, bus); }
    #[inline(always)] pub(crate) fn rra_izy<BC: BusContext>(&mut self, bus: &mut BC) { let a = self.rmw_izy(bus); self.rra_at(a, bus); }

    // =================================================================
    // SHA / SHX / SHY / TAS — undocumented stores with page-cross
    // behaviour. These use write helpers so the borrow checker accepts
    // them (each computes the address, then writes in a separate stmt).
    // =================================================================

    #[inline(always)]
    pub(crate) fn sha_absy<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absy(bus);
        let base_hi = (a.wrapping_sub(self.y as u16) >> 8) as u8;
        self.sha_write(a, base_hi, bus);
    }
    #[inline(always)]
    pub(crate) fn sha_izy<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_izy(bus);
        let base_hi = (a.wrapping_sub(self.y as u16) >> 8) as u8;
        self.sha_write(a, base_hi, bus);
    }
    #[inline(always)]
    pub(crate) fn shx_absy<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absy(bus);
        self.shx_write(a, bus);
    }
    #[inline(always)]
    pub(crate) fn shy_absx<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absx(bus);
        self.shy_write(a, bus);
    }
    #[inline(always)]
    pub(crate) fn tas_absy<BC: BusContext>(&mut self, bus: &mut BC) {
        let a = self.rmw_absy(bus);
        let base_hi = (a.wrapping_sub(self.y as u16) >> 8) as u8;
        self.tas(a, base_hi, bus);
    }
}
