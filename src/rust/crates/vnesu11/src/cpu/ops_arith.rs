//! Arithmetic & logical instructions: ADC, SBC, AND, ORA, EOR, CMP.

use super::flags::{set_zn, C_FLAG, V_FLAG};
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // -----------------------------------------------------------------
    // ADC — add with carry (8 modes)
    //
    // NOTE (parity decision): the C++ FCEUX core (`x6502.cpp` ADC macro)
    // treats ADC/SBC as **binary-only**, ignoring the D (decimal) flag
    // entirely — there is no `D_FLAG` reference anywhere in x6502.cpp.
    // NES commercial software never uses decimal mode, and the migration
    // goal (decision A / ADR-008) is byte-for-byte shadow-run parity with
    // the C++ core. A "more correct" decimal implementation would break
    // parity and red-flag every shadow-run diff. So we match C++ exactly:
    // binary arithmetic regardless of D. If a future phase decides to
    // implement decimal mode (FCEUX upstream is binary-only too), it must
    // be a deliberate, separately-gated change.
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn adc_common(&mut self, val: u8) {
        let a = self.a;
        let carry = (self.p & C_FLAG != 0) as u16;
        let sum = a as u16 + val as u16 + carry;

        // V flag: signed overflow.
        let v = ((a ^ val) & 0x80 == 0) && ((a ^ (sum as u8)) & 0x80 != 0);
        self.p = set_zn(self.p, sum as u8);
        self.p = if v { self.p | V_FLAG } else { self.p & !V_FLAG };
        self.p = if sum > 0xFF { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.a = sum as u8;
    }

    // ADC addressing-mode wrappers
    #[inline(always)] pub(crate) fn adc_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.adc_common(v); }
    #[inline(always)] pub(crate) fn adc_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.adc_common(v); }

    // -----------------------------------------------------------------
    // SBC — subtract with borrow (8 modes)
    //
    // Parity decision (see ADC above): binary-only, matching C++ FCEUX
    // exactly. SBC = A - val - (1 - C), implemented as ADC of the
    // complement with the same carry:
    //   A + !val + C = A + (255-val) + C ≡ A - val - 1 + C (mod 256)
    //                = A - val - (1 - C). ✓
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn sbc_common(&mut self, val: u8) {
        self.adc_common(!val);
    }

    #[inline(always)] pub(crate) fn sbc_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.sbc_common(v); }
    #[inline(always)] pub(crate) fn sbc_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.sbc_common(v); }

    // -----------------------------------------------------------------
    // AND / ORA / EOR — logical ops (8 modes each)
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn and_common(&mut self, val: u8) {
        self.a &= val;
        self.p = set_zn(self.p, self.a);
    }
    #[inline(always)] pub(crate) fn and_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.and_common(v); }
    #[inline(always)] pub(crate) fn and_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.and_common(v); }

    #[inline(always)]
    pub(crate) fn ora_common(&mut self, val: u8) {
        self.a |= val;
        self.p = set_zn(self.p, self.a);
    }
    #[inline(always)] pub(crate) fn ora_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.ora_common(v); }
    #[inline(always)] pub(crate) fn ora_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.ora_common(v); }

    #[inline(always)]
    pub(crate) fn eor_common(&mut self, val: u8) {
        self.a ^= val;
        self.p = set_zn(self.p, self.a);
    }
    #[inline(always)] pub(crate) fn eor_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.eor_common(v); }
    #[inline(always)] pub(crate) fn eor_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.eor_common(v); }

    // -----------------------------------------------------------------
    // CPX / CPY — compare X or Y with memory (3 modes each)
    // -----------------------------------------------------------------

    #[inline(always)]
    fn cpx_common(&mut self, val: u8) {
        let r = self.x.wrapping_sub(val);
        self.p = set_zn(self.p, r);
        self.p = if self.x >= val { self.p | C_FLAG } else { self.p & !C_FLAG };
    }
    #[inline(always)] pub(crate) fn cpx_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.cpx_common(v); }
    #[inline(always)] pub(crate) fn cpx_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.cpx_common(v); }
    #[inline(always)] pub(crate) fn cpx_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.cpx_common(v); }

    #[inline(always)]
    fn cpy_common(&mut self, val: u8) {
        let r = self.y.wrapping_sub(val);
        self.p = set_zn(self.p, r);
        self.p = if self.y >= val { self.p | C_FLAG } else { self.p & !C_FLAG };
    }
    #[inline(always)] pub(crate) fn cpy_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.cpy_common(v); }
    #[inline(always)] pub(crate) fn cpy_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.cpy_common(v); }
    #[inline(always)] pub(crate) fn cpy_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.cpy_common(v); }

    // -----------------------------------------------------------------
    // CMP — compare A with memory (8 modes)
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn cmp_common(&mut self, val: u8) {
        let r = self.a.wrapping_sub(val);
        self.p = set_zn(self.p, r);
        self.p = if self.a >= val { self.p | C_FLAG } else { self.p & !C_FLAG };
    }
    #[inline(always)] pub(crate) fn cmp_imm<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.imm(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_zp<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zp(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_zpx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_zpx(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_abs<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_abs(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_absx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absx(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_absy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_absy(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_izx<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izx(bus); self.cmp_common(v); }
    #[inline(always)] pub(crate) fn cmp_izy<BC: BusContext>(&mut self, bus: &mut BC) { let v = self.read_izy(bus); self.cmp_common(v); }

}
