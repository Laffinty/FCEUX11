//! 6502 ALU primitives.
//!
//! Each helper takes the CPU register file (via [`CpuState`]) and the
//! operand byte read from the addressing-mode helper, performs the
//! operation, and updates `A` / `X` / `Y` / flags in place.
//!
//! Every primitive is a literal translation of the C++ macro in
//! `src/x6502.cpp:147–225`. We keep them as Rust functions so the
//! borrow checker verifies aliasing on `state.regs.p`.
//!
//! ## Flag-update rules (matching the C++ implementation)
//!
//! * `Flags::ZERO` / `Flags::NEGATIVE` are precomputed in the [`ZN_TABLE`]
//!   for every byte value; load / store / compare helpers OR them in.
//! * `Flags::CARRY` / `Flags::OVERFLOW` are arithmetic-driven and computed
//!   per instruction.
//! * `Flags::BREAK` / `Flags::UNUSED` / `Flags::DECIMAL` / `Flags::IRQ_DIS`
//!   are not touched by ALU primitives (set only by control flow).

use crate::cpu::addressing::CpuState;
use crate::cpu::state::{Flags, ZN_TABLE};

/// ADC — Add with Carry. `A = A + M + C`. Sets N, V, Z, C.
#[inline]
pub fn adc<B>(state: &mut CpuState, _bus: &mut B, m: u8)
where
    B: ?Sized,
{
    let a = state.regs.a;
    let c = if state.regs.p & Flags::CARRY.bits() != 0 { 1u16 } else { 0u16 };
    let l = a as u16 + m as u16 + c;
    let result = l as u8;
    // Overflow: signed carry-in differs from signed carry-out.
    // V = ((A ^ M) & 0x80 == 0) AND ((A ^ result) & 0x80 != 0)
    //     — C++ form: (((_A ^ x) & 0x80) ^ 0x80) & ((_A ^ l) & 0x80)) >> 1
    let v = (((a ^ m) as u16 ^ 0x80) & (a as u16 ^ l) & 0x80) != 0;
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::CARRY.bits() | Flags::NEGATIVE.bits() | Flags::OVERFLOW.bits());
    p |= (l >> 8) as u8 & Flags::CARRY.bits();
    if v {
        p |= Flags::OVERFLOW.bits();
    }
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
    state.regs.a = result;
}

/// SBC — Subtract with Carry. `A = A - M - (1 - C)`. Sets N, V, Z, C.
#[inline]
pub fn sbc<B>(state: &mut CpuState, _bus: &mut B, m: u8)
where
    B: ?Sized,
{
    let a = state.regs.a;
    let c = if state.regs.p & Flags::CARRY.bits() != 0 { 1i16 } else { 0i16 };
    // Subtraction = addition of one's complement plus carry-in.
    let l = (a as i16) - (m as i16) - (1 - c);
    let result = l as u8;
    // V: signed overflow on subtraction.
    let v = ((a as i16 ^ result as i16) & (a as i16 ^ m as i16) & 0x80) != 0;
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::CARRY.bits() | Flags::NEGATIVE.bits() | Flags::OVERFLOW.bits());
    // Carry is **inverted** on subtraction: C set means no borrow.
    p |= (((l >> 8) & 1) as u8) ^ Flags::CARRY.bits();
    if v {
        p |= Flags::OVERFLOW.bits();
    }
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
    state.regs.a = result;
}

/// AND — `A &= M`. Sets N, Z.
#[inline]
pub fn and<B>(state: &mut CpuState, _bus: &mut B, m: u8)
where
    B: ?Sized,
{
    let a = state.regs.a & m;
    state.regs.a = a;
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    p |= ZN_TABLE[a as usize];
    state.regs.p = p;
}

/// ORA — `A |= M`. Sets N, Z.
#[inline]
pub fn ora<B>(state: &mut CpuState, _bus: &mut B, m: u8)
where
    B: ?Sized,
{
    let a = state.regs.a | m;
    state.regs.a = a;
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    p |= ZN_TABLE[a as usize];
    state.regs.p = p;
}

/// EOR — `A ^= M`. Sets N, Z.
#[inline]
pub fn eor<B>(state: &mut CpuState, _bus: &mut B, m: u8)
where
    B: ?Sized,
{
    let a = state.regs.a ^ m;
    state.regs.a = a;
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    p |= ZN_TABLE[a as usize];
    state.regs.p = p;
}

/// CMP / CPX / CPY — compare register with memory.
/// Sets Z if equal, N from the result byte, C if register >= memory.
#[inline]
pub fn cmp<B>(state: &mut CpuState, _bus: &mut B, r: u8, m: u8)
where
    B: ?Sized,
{
    let t = (r as i16) - (m as i16);
    let result = (t & 0xFF) as u8;
    let mut p = state.regs.p;
    p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    // C: borrow inversion — C set iff r >= m (no borrow).
    p |= (((t >> 8) & 1) as u8) ^ Flags::CARRY.bits();
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
}

/// BIT — Test bits in memory against accumulator.
/// * N = bit 7 of M
/// * V = bit 6 of M
/// * Z = (A & M) == 0 ? 1 : 0
///
/// M is read but **not** consumed.
#[inline]
pub fn bit<B>(state: &mut CpuState, _bus: &mut B, m: u8)
where
    B: ?Sized,
{
    let a = state.regs.a;
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::OVERFLOW.bits() | Flags::NEGATIVE.bits());
    // Z: ZN_TABLE[(A & M) & 0xFF] gives ZERO bit only if AND is zero.
    p |= ZN_TABLE[(a & m) as usize] & Flags::ZERO.bits();
    // N and V from M directly.
    p |= m & (Flags::NEGATIVE.bits() | Flags::OVERFLOW.bits());
    state.regs.p = p;
}

/// ASL — Arithmetic Shift Left on memory.
/// Operates on `m` in place; the caller is responsible for storing the
/// result back if the instruction is a memory RMW.
#[inline]
pub fn asl(state: &mut CpuState, m: u8) -> u8 {
    let mut p = state.regs.p;
    p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    let result = m << 1;
    p |= (m >> 7) & Flags::CARRY.bits();
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
    result
}

/// LSR — Logical Shift Right on memory.
#[inline]
pub fn lsr(state: &mut CpuState, m: u8) -> u8 {
    let mut p = state.regs.p;
    p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    let result = m >> 1;
    p |= m & Flags::CARRY.bits();
    // N is always cleared on LSR (bit 7 = 0 after shift).
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
    result
}

/// ROL — Rotate Left on memory (9-bit rotation through C).
#[inline]
pub fn rol(state: &mut CpuState, m: u8) -> u8 {
    let c_in = if state.regs.p & Flags::CARRY.bits() != 0 { 1 } else { 0 };
    let c_out = m >> 7;
    let result = (m << 1) | c_in;
    let mut p = state.regs.p;
    p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    p |= c_out;
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
    result
}

/// ROR — Rotate Right on memory.
#[inline]
pub fn ror(state: &mut CpuState, m: u8) -> u8 {
    let c_in = if state.regs.p & Flags::CARRY.bits() != 0 { 0x80 } else { 0 };
    let c_out = m & 1;
    let result = (m >> 1) | c_in;
    let mut p = state.regs.p;
    p &= !(Flags::CARRY.bits() | Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    p |= c_out;
    p |= ZN_TABLE[result as usize];
    state.regs.p = p;
    result
}

/// Load register from memory. Sets N, Z.
#[inline]
pub fn load_reg<B>(state: &mut CpuState, _bus: &mut B, reg: LoadReg, m: u8)
where
    B: ?Sized,
{
    let val = m;
    match reg {
        LoadReg::A => state.regs.a = val,
        LoadReg::X => state.regs.x = val,
        LoadReg::Y => state.regs.y = val,
    }
    let mut p = state.regs.p;
    p &= !(Flags::ZERO.bits() | Flags::NEGATIVE.bits());
    p |= ZN_TABLE[val as usize];
    state.regs.p = p;
}

/// Target register for a load / store / transfer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LoadReg {
    A,
    X,
    Y,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::addressing::Bus;

    struct NoBus;
    impl Bus for NoBus {
        fn read(&mut self, _addr: u16) -> u8 {
            0
        }
        fn write(&mut self, _addr: u16, _val: u8) {
        }
    }

    #[test]
    fn adc_basic_sets_carry() {
        let mut s = CpuState::new();
        s.regs.a = 0xFF;
        s.regs.p = Flags::CARRY.bits();
        let mut bus = NoBus;
        adc(&mut s, &mut bus, 0x01);
        assert_eq!(s.regs.a, 0x01);
        assert!(s.regs.p & Flags::CARRY.bits() != 0); // 0xFF + 0x01 + 1 = 0x101, C set
    }

    #[test]
    fn adc_overflow_set() {
        // 0x50 + 0x50 = 0xA0, signed overflow (positive + positive = negative)
        let mut s = CpuState::new();
        s.regs.a = 0x50;
        s.regs.p = 0;
        let mut bus = NoBus;
        adc(&mut s, &mut bus, 0x50);
        assert_eq!(s.regs.a, 0xA0);
        assert!(s.regs.p & Flags::OVERFLOW.bits() != 0);
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
        assert_eq!(s.regs.p & Flags::CARRY.bits(), 0);
    }

    #[test]
    fn sbc_basic() {
        // A=0x50, M=0x30, C=1: 0x50 - 0x30 - 0 = 0x20
        let mut s = CpuState::new();
        s.regs.a = 0x50;
        s.regs.p = Flags::CARRY.bits();
        let mut bus = NoBus;
        sbc(&mut s, &mut bus, 0x30);
        assert_eq!(s.regs.a, 0x20);
        assert!(s.regs.p & Flags::CARRY.bits() != 0);
    }

    #[test]
    fn sbc_borrow_inverts_carry() {
        // A=0x10, M=0x20, C=1: 0x10 - 0x20 - 0 = -0x10 → borrow, C clear
        let mut s = CpuState::new();
        s.regs.a = 0x10;
        s.regs.p = Flags::CARRY.bits();
        let mut bus = NoBus;
        sbc(&mut s, &mut bus, 0x20);
        assert_eq!(s.regs.a, 0xF0);
        assert_eq!(s.regs.p & Flags::CARRY.bits(), 0); // borrow → C clear
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
    }

    #[test]
    fn cmp_equal_sets_z() {
        let mut s = CpuState::new();
        s.regs.a = 0x42;
        s.regs.p = 0;
        let mut bus = NoBus;
        cmp(&mut s, &mut bus, 0x42, 0x42);
        assert!(s.regs.p & Flags::ZERO.bits() != 0);
        assert!(s.regs.p & Flags::CARRY.bits() != 0);
    }

    #[test]
    fn cmp_greater_sets_c() {
        let mut s = CpuState::new();
        s.regs.a = 0x80;
        s.regs.p = 0;
        let mut bus = NoBus;
        cmp(&mut s, &mut bus, 0x80, 0x40);
        assert_eq!(s.regs.p & Flags::ZERO.bits(), 0);
        assert!(s.regs.p & Flags::CARRY.bits() != 0);
    }

    #[test]
    fn cmp_less_clears_c() {
        let mut s = CpuState::new();
        s.regs.a = 0x40;
        s.regs.p = Flags::CARRY.bits();
        let mut bus = NoBus;
        cmp(&mut s, &mut bus, 0x40, 0x80);
        assert_eq!(s.regs.p & Flags::CARRY.bits(), 0);
    }

    #[test]
    fn bit_zero_result() {
        let mut s = CpuState::new();
        s.regs.a = 0x00;
        s.regs.p = 0;
        let mut bus = NoBus;
        bit(&mut s, &mut bus, 0xC0);
        // A & M = 0 → Z set; N from bit 7 of M (1); V from bit 6 of M (1)
        assert!(s.regs.p & Flags::ZERO.bits() != 0);
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
        assert!(s.regs.p & Flags::OVERFLOW.bits() != 0);
    }

    #[test]
    fn asl_shift_left() {
        let mut s = CpuState::new();
        s.regs.p = 0;
        let mut bus = NoBus;
        let r = asl(&mut s, 0x80);
        assert_eq!(r, 0x00);
        assert!(s.regs.p & Flags::CARRY.bits() != 0);
        assert!(s.regs.p & Flags::ZERO.bits() != 0);
    }

    #[test]
    fn lsr_shift_right() {
        let mut s = CpuState::new();
        s.regs.p = 0;
        let mut bus = NoBus;
        let r = lsr(&mut s, 0x01);
        assert_eq!(r, 0x00);
        assert!(s.regs.p & Flags::CARRY.bits() != 0);
        assert!(s.regs.p & Flags::ZERO.bits() != 0);
    }

    #[test]
    fn rol_through_carry() {
        let mut s = CpuState::new();
        s.regs.p = Flags::CARRY.bits();
        let mut bus = NoBus;
        let r = rol(&mut s, 0x00);
        assert_eq!(r, 0x01);
        assert_eq!(s.regs.p & Flags::CARRY.bits(), 0); // rotated out
    }
}