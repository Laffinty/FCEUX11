//! Arithmetic & logical instructions: ADC, SBC, AND, ORA, EOR, CMP.

use super::flags::{set_zn, C_FLAG, D_FLAG, V_FLAG};
use super::BusContext;
use super::CpuCore;

impl CpuCore {
    // -----------------------------------------------------------------
    // ADC — add with carry (8 modes), decimal mode supported
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn adc_common(&mut self, val: u8) {
        if self.p & D_FLAG != 0 {
            self.adc_decimal(val);
        } else {
            self.adc_binary(val);
        }
    }

    #[inline(always)]
    fn adc_binary(&mut self, val: u8) {
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

    #[inline(always)]
    fn adc_decimal(&mut self, val: u8) {
        let a = self.a;
        let carry = (self.p & C_FLAG != 0) as u16;
        // Binary result for V flag (NES behavior).
        let bin_sum = a as u16 + val as u16 + carry;
        let v = ((a ^ val) & 0x80 == 0) && ((a ^ (bin_sum as u8)) & 0x80 != 0);

        // Decimal-adjusted sum.
        let lo = (a & 0x0F) as u16 + (val & 0x0F) as u16 + carry;
        let hi = (a >> 4) as u16 + (val >> 4) as u16;
        let mut sum = lo + (hi << 4);
        if lo > 0x09 {
            sum += 0x06;
        }
        let mut carry_out = false;
        if sum > 0x99 {
            sum += 0x60;
            carry_out = true;
        }
        self.p = set_zn(self.p, sum as u8);
        self.p = if v { self.p | V_FLAG } else { self.p & !V_FLAG };
        self.p = if carry_out { self.p | C_FLAG } else { self.p & !C_FLAG };
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
    // SBC — subtract with borrow (8 modes), decimal mode supported
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn sbc_common(&mut self, val: u8) {
        if self.p & D_FLAG != 0 {
            self.sbc_decimal(val);
        } else {
            // SBC = A - val - (1 - C). In binary mode this is exactly
            // equivalent to ADC with the complement (~val) and the same
            // carry, because: A + !val + C = A + (255-val) + C
            //   = A - val + 255 + C ≡ A - val - 1 + C (mod 256)
            //   = A - val - (1 - C). ✓
            self.adc_binary(!val);
        }
    }

    /// Decimal-mode SBC. The NES 6502 decimal adjust is NOT the same as
    /// ADC-of-complement; it needs its own correction rules:
    ///   temp = A - val - (1-C)
    ///   if (A & 0x0F) < (val & 0x0F) + (1-C):  temp -= 0x06
    ///   if (temp & 0xF0) > 0x90:               temp -= 0x60
    #[inline(always)]
    fn sbc_decimal(&mut self, val: u8) {
        let a = self.a;
        let c = self.p & C_FLAG != 0;
        let borrow: u8 = if c { 0 } else { 1 };

        let mut temp = a.wrapping_sub(val).wrapping_sub(borrow);

        if (a & 0x0F) < (val & 0x0F).wrapping_add(borrow) {
            temp = temp.wrapping_sub(0x06);
        }
        if temp & 0xF0 > 0x90 {
            temp = temp.wrapping_sub(0x60);
        }

        // V flag from binary subtraction overflow.
        let v = ((a ^ val) & 0x80 != 0) && ((a ^ temp) & 0x80 != 0);
        // Carry = 1 if no borrow out (A >= val + borrow).
        let no_borrow = (a as u16) >= (val as u16) + borrow as u16;

        self.p = set_zn(self.p, temp);
        self.p = if v { self.p | V_FLAG } else { self.p & !V_FLAG };
        self.p = if no_borrow { self.p | C_FLAG } else { self.p & !C_FLAG };
        self.a = temp;
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
