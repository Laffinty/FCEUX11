//! Unofficial 6502 opcode coverage tests.
//!
//! Phase 4 sub-step 3 of `docs/plans/cpu-rust-v2.md`. These tests
//! exercise the 109 unofficial 6502 opcodes grouped by behaviour
//! family:
//!
//! | Family            | Opcodes                            | Behaviour |
//! |-------------------|------------------------------------|-----------|
//! | KIL/JAM           | 02,12,22,32,42,52,62,72,92,B2,D2,F2 | CPU halts |
//! | Read-NOP          | 04,0C,14,1C,34,3C,44,54,5C,64,74,7C,80,82,89,C2,D4,DC,E2,F4,FC | 2-byte reads, no state change |
//! | DOP/0A/4A/6A      | 04/44/64 are read-modify-write NOPs | mem read+write unchanged |
//! | SLO (ASL+ORA)     | 03,07,0F,13,17,1B,1F              | ASL mem, ORA mem into A |
//! | RLA (ROL+AND)     | 23,27,2F,33,37,3B,3F              | ROL mem, AND mem into A |
//! | SRE (LSR+EOR)     | 43,47,4F,53,57,5B,5F              | LSR mem, EOR mem into A |
//! | RRA (ROR+ADC)     | 63,67,6F,73,77,7B,7F              | ROR mem, ADC mem into A |
//! | SAX (A&X→mem)     | 83,87,8F,97                       | Store A & X |
//! | LAX (mem→A,X)     | A3,A7,AB,AF,B3,B7,BF              | Load A and X with same value |
//! | DCP (DEC+CMP)     | C3,C7,CF,D3,D7,DB,DF              | DEC mem, CMP mem with A |
//! | ISC (INC+SBC)     | E3,E7,EF,F3,F7,FB,FF              | INC mem, SBC mem from A |
//! | ANC (AND imm + C) | 0B,2B                             | AND imm into A, C = bit 7 of A |
//! | ALR (AND imm + LSR)| 4B                                | AND imm into A, LSR A |
//! | ARR (AND imm + ROR)| 6B                                | AND imm into A, ROR A, special C/V |
//! | XAA (TXA + AND imm)| 8B                                | TXA, AND imm into A |
//! | AXS (A&X − imm)   | CB                                | X = (A & X) - imm, sets NZC |
//! | AHX (A&X&H+1→mem) | 93,9F                             | Store A & X & (H+1) |
//! | SHX (X&H+1→mem)   | 9E                                | Store X & (H+1) |
//! | SHY (Y&H+1→mem)   | 9C                                | Store Y & (H+1) |
//! | TAS (S=A&X,store) | 9B                                | S = A&X; Store S & (H+1) |
//! | LAS (mem&S→A,X,S)| BB                                | A=X=S = mem & S |
//! | SBC unofficial dup | EB                                | undocumented duplicate of E9 |
//!
//! Per the NESdev CPU unofficial opcodes matrix
//! (https://www.nesdev.org/wiki/CPU_unofficial_opcodes).

use fceux11_core::cpu::{step, Bus, CpuState, Flags};

struct FlatBus {
    mem: [u8; 0x10000],
}

impl FlatBus {
    fn new() -> Self {
        Self { mem: [0; 0x10000] }
    }
}

impl Bus for FlatBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val;
    }
}

fn cpu_at(pc: u16) -> CpuState {
    let mut cpu = CpuState::new();
    cpu.regs.pc = pc;
    cpu.regs.s = 0xFD;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
    cpu.regs.moo_pi = cpu.regs.p;
    cpu.regs.irq_low = 0;
    cpu.nmi_fresh = false;
    cpu
}

// ===========================================================================
// KIL / JAM — opcodes 02/12/22/32/42/52/62/72/92/B2/D2/F2
// ===========================================================================

#[test]
fn jam_halts_cpu_with_pc_rolled_back() {
    // Per C++ 0x02 handler: stop CPU, roll PC back so the same opcode
    // byte stays at PC (so re-running the loop re-jams).
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x02; // JAM (KIL)
    step(&mut cpu, &mut bus);
    assert_ne!(cpu.regs.jammed, 0, "CPU must be jammed");
    assert_eq!(cpu.regs.pc, 0x4000, "PC rolled back to the JAM opcode");
}

#[test]
fn jam_all_variants_halt_cpu() {
    // Every opcode in {02,12,22,32,42,52,62,72,92,B2,D2,F2} should jam.
    let variants: &[u8] = &[
        0x02, 0x12, 0x22, 0x32, 0x42, 0x52, 0x62, 0x72, 0x92, 0xB2, 0xD2, 0xF2,
    ];
    for &op in variants {
        let mut cpu = cpu_at(0x4000);
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        step(&mut cpu, &mut bus);
        assert_ne!(cpu.regs.jammed, 0, "opcode ${:02X} should jam CPU", op);
        assert_eq!(cpu.regs.pc, 0x4000, "opcode ${:02X}: PC rolled back", op);
    }
}

// ===========================================================================
// SLO (ASL mem; ORA mem) — opcodes 03,07,0F,13,17,1B,1F
// ===========================================================================

#[test]
fn slo_zp_shifts_left_then_oras_into_a() {
    // SLO zp: M <<= 1; A |= M; sets NZC from M's ASL result
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x0F;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x07; // SLO zp
    bus.mem[0x4001] = 0x10; // zp addr
    bus.mem[0x0010] = 0x21; // M = 0x21 → after ASL: 0x42, C=0
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x42, "M shifted left to 0x42");
    assert_eq!(cpu.regs.a, 0x4F, "A = 0x0F | 0x42 = 0x4F");
    assert_eq!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N flag from 0x42 (bit 7 not set)");
    assert_eq!(cpu.regs.p & Flags::ZERO.bits(), 0, "Z not set");
    assert_eq!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=0 from 0x21 << 1");
}

#[test]
fn slo_zp_carry_from_asl() {
    // SLO ZP with M = 0x81 → ASL → 0x02, C=1
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x00;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x07; // SLO zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x81;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x02);
    assert_eq!(cpu.regs.a, 0x02);
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 from M=0x81 << 1");
}

#[test]
fn slo_abs_shifts_left_then_oras_into_a() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x0F; // SLO abs
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2000] = 0x40;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x2000], 0x80, "M shifted left");
    assert_eq!(cpu.regs.a, 0xFF, "A = 0xFF | 0x80 = 0xFF");
    assert_ne!(cpu.regs.p & Flags::NEGATIVE.bits(), 0);
}

// ===========================================================================
// RLA (ROL mem; AND mem) — opcodes 23,27,2F,33,37,3B,3F
// ===========================================================================

#[test]
fn rla_zp_rotates_left_then_ands_into_a() {
    // ROL with C=0: M = (M << 1) | 0; AND into A
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits(); // C=0
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x27; // RLA zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x0F; // M = 0x0F → ROL(C=0) → 0x1E
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x1E, "M rotated left to 0x1E");
    assert_eq!(cpu.regs.a, 0x1E, "A = 0xFF & 0x1E = 0x1E");
    assert_eq!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=0 (bit 7 of 0x1E not set)");
    assert_eq!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=0 (bit 7 of M=0x0F was 0)");
}

#[test]
fn rla_zp_rotates_left_with_carry_in() {
    // ROL with C=1: M = (M << 1) | 1
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits() | Flags::CARRY.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x27; // RLA zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x40; // → ROL(C=1) → 0x81
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x81);
    assert_eq!(cpu.regs.a, 0x81);
    assert_eq!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=0 (bit 7 of M was 0)");
}

// ===========================================================================
// SRE (LSR mem; EOR mem) — opcodes 43,47,4F,53,57,5B,5F
// ===========================================================================

#[test]
fn sre_zp_shifts_right_then_eors_into_a() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x0F;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x47; // SRE zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x05; // M = 0x05 → LSR → 0x02, C=1
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x02, "M shifted right to 0x02");
    assert_eq!(cpu.regs.a, 0x0D, "A = 0x0F ^ 0x02 = 0x0D");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 from M=0x05 >> 1");
}

// ===========================================================================
// RRA (ROR mem; ADC mem) — opcodes 63,67,6F,73,77,7B,7F
// ===========================================================================

#[test]
fn rra_zp_rotates_right_then_adcs_into_a() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x05;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits(); // C=0
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x67; // RRA zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x04; // M = 0x04 → ROR(C=0) → 0x02, C=0
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x02);
    assert_eq!(cpu.regs.a, 0x07, "A = 0x05 + 0x02 = 0x07");
    assert_eq!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=0 + 0x02 + 0x05 = 0x07, no carry");
}

// ===========================================================================
// SAX (Store A & X) — opcodes 83,87,8F,97
// ===========================================================================

#[test]
fn sax_zp_stores_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xF0;
    cpu.regs.x = 0x0F;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x87; // SAX zp
    bus.mem[0x4001] = 0x10;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x00, "M = A & X = 0xF0 & 0x0F = 0x00");
}

#[test]
fn sax_abs_stores_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xAA;
    cpu.regs.x = 0x55;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x8F; // SAX abs
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x2000], 0x00, "A=0xAA & X=0x55 = 0x00");
}

#[test]
fn sax_zpy_stores_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0xFF;
    cpu.regs.y = 0x10;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x97; // SAX zpy
    bus.mem[0x4001] = 0x20; // zp addr = $20 + Y
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0030], 0xFF, "M = A & X = 0xFF & 0xFF = 0xFF");
}

// ===========================================================================
// LAX (Load A and X with same value) — opcodes A3,A7,AB,AF,B3,B7,BF
// ===========================================================================

#[test]
fn lax_zp_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xA7; // LAX zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x42;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(cpu.regs.x, 0x42);
    assert_eq!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=0 from 0x42 (bit 7 not set)");
    assert_eq!(cpu.regs.p & Flags::ZERO.bits(), 0);
}

#[test]
fn lax_imm_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xAB; // LAX imm
    bus.mem[0x4001] = 0x00;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x00);
    assert_eq!(cpu.regs.x, 0x00);
    assert_ne!(cpu.regs.p & Flags::ZERO.bits(), 0);
}

#[test]
fn lax_abs_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xAF; // LAX abs
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2000] = 0xFF;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0xFF);
    assert_eq!(cpu.regs.x, 0xFF);
    assert_ne!(cpu.regs.p & Flags::NEGATIVE.bits(), 0);
}

#[test]
fn lax_indy_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.y = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xB3; // LAX (ind),Y
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x00;
    bus.mem[0x0011] = 0x30; // ptr = $3000
    bus.mem[0x3005] = 0x99; // M[3000+Y=5]
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x99);
    assert_eq!(cpu.regs.x, 0x99);
}

// ===========================================================================
// DCP (DEC mem; CMP mem) — opcodes C3,C7,CF,D3,D7,DB,DF
// ===========================================================================

#[test]
fn dcp_zp_decrements_then_compares() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xC7; // DCP zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x04; // M=4 → DEC → 3. CMP 3 vs A=5: A > M, C=1, Z=0, N=0
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x03, "M decremented to 3");
    assert_eq!(cpu.regs.a, 0x05, "A unchanged");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 (A>=M)");
    assert_eq!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=0 (A-M=2 >0)");
    assert_eq!(cpu.regs.p & Flags::ZERO.bits(), 0, "Z=0");
}

#[test]
fn dcp_zp_wraps_around_on_zero() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xC7;
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x00; // M=0 → DEC → 0xFF
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0xFF, "DEC wraps 0 → 0xFF");
    // CMP 0xFF vs A=0xFF: equal, Z=1, N=0
    assert_ne!(cpu.regs.p & Flags::ZERO.bits(), 0);
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0);
}

// ===========================================================================
// ISC (INC mem; SBC mem) — opcodes E3,E7,EF,F3,F7,FB,FF
// ===========================================================================

#[test]
fn isc_zp_increments_then_sbcs_from_a() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x05;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits() | Flags::CARRY.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xE7; // ISC zp
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x02; // M=2 → INC → 3. SBC 5 - 3 = 2
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x03, "M incremented to 3");
    assert_eq!(cpu.regs.a, 0x02, "A = 5 - 3 = 2");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 (no borrow)");
    assert_eq!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=0 (positive result)");
}

// ===========================================================================
// ANC (AND imm; C = bit 7 of result) — opcodes 0B,2B
// ===========================================================================

#[test]
fn anc_imm_ands_and_sets_carry_from_high_bit() {
    // ANC #$80: A = A & $80; C = bit 7 of result (=1)
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x0B; // ANC #imm
    bus.mem[0x4001] = 0x80;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x80, "A = FF & 80 = 80");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 (bit 7 of result)");
    assert_ne!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=1");
}

#[test]
fn anc_imm_ands_and_clears_carry_when_bit7_clear() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits() | Flags::CARRY.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x0B;
    bus.mem[0x4001] = 0x7F; // → A = FF & 7F = 7F, bit 7 = 0
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x7F);
    assert_eq!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=0");
}

// ===========================================================================
// ALR (AND imm; LSR A) — opcode 4B
// ===========================================================================

#[test]
fn alr_ands_then_lsrs() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x7F;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x4B; // ALR #imm
    bus.mem[0x4001] = 0xFF; // → A = 7F & FF = 7F → LSR A → 0x3F, C=1
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x3F);
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1");
}

// ===========================================================================
// ARR (AND imm; ROR A with special C/V) — opcode 6B
// ===========================================================================

#[test]
fn arr_ands_then_rors() {
    // ARR #$80: A = A & $80, then ROR A. With C=0 and value 0x80:
    // result = 0x40. Per Visual6502 ARR formula (binary mode):
    //   C = bit 6 of result
    //   V = bit 6 of result ^ bit 5 of result
    // bit 6 of 0x40 = 1, bit 5 of 0x40 = 0 -> C = 1, V = 1.
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x6B; // ARR #imm
    bus.mem[0x4001] = 0x80;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x40, "A = 0xFF & 0x80 = 0x80, ROR = 0x40");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 (bit 6 of result 0x40)");
    assert_ne!(cpu.regs.p & Flags::OVERFLOW.bits(), 0, "V=1 (bit 6 ^ bit 5 of result)");
}

// ===========================================================================
// XAA (TXA; AND imm) — opcode 8B
// ===========================================================================

#[test]
fn xaa_txa_then_ands_imm() {
    // XAA #$00: A = X & 0 = 0
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x42;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x8B;
    bus.mem[0x4001] = 0x00;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x00, "A = X & 0 = 0");
    assert_ne!(cpu.regs.p & Flags::ZERO.bits(), 0);
    assert_eq!(cpu.regs.x, 0x42, "X unchanged");
}

// ===========================================================================
// AXS (X = (A & X) - imm) — opcode CB
// ===========================================================================

#[test]
fn axs_subtracts_imm_from_a_and_x() {
    // AXS #$02: X = (A & X) - imm
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xCB;
    bus.mem[0x4001] = 0x02;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.x, 0xFD, "X = (A & X) - imm = 0xFF - 0x02 = 0xFD");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 (no borrow)");
    assert_ne!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=1 (high bit set)");
}

#[test]
fn axs_with_borrow() {
    // AXS #$80: X = 0x10 - 0x80 = wraps with borrow
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x10;
    cpu.regs.x = 0x10;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xCB;
    bus.mem[0x4001] = 0x80;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.x, 0x90, "X = (A & X) - imm = 0x10 - 0x80 = 0x90 (wrapping)");
    assert_eq!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=0 (borrow)");
    assert_ne!(cpu.regs.p & Flags::NEGATIVE.bits(), 0, "N=1");
}

// ===========================================================================
// AHX (A & X & H+1 → mem) — opcodes 93, 9F
// ===========================================================================

#[test]
fn ahx_indy_stores_a_and_x_and_h_plus_1() {
    // AHX (ind),Y at $93 with Y=0: ptr=$3000 → A&X&(H+1)
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xF0;
    cpu.regs.x = 0x0F;
    cpu.regs.y = 0x00;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x93;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x0000] = 0x00;
    bus.mem[0x0001] = 0x30;
    step(&mut cpu, &mut bus);
    // A=0xF0 & X=0x0F = 0x00, then & (H+1=0x30+1=0x31) = 0x00
    assert_eq!(bus.mem[0x3000], 0x00);
}

// ===========================================================================
// SHX (X & H+1 → mem) — opcode 9E
// ===========================================================================

#[test]
fn shx_absx_stores_x_and_h_plus_1() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x42;
    cpu.regs.y = 0x00;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9E;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x30; // abs = $3000; abs+X = $3042
    step(&mut cpu, &mut bus);
    // X=0x42 & (H+1=0x31) = 0x00
    assert_eq!(bus.mem[0x3042], 0x00, "X=0x42 & 0x31 = 0x00");
}

// ===========================================================================
// SHY (Y & H+1 → mem) — opcode 9C
// ===========================================================================

#[test]
fn shy_absx_stores_y_and_h_plus_1() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x00;
    cpu.regs.y = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9C;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x40; // abs = $40FF
    step(&mut cpu, &mut bus);
    // Y=0xFF & (H+1=0x41) = 0x41
    assert_eq!(bus.mem[0x40FF], 0x41);
}

// ===========================================================================
// TAS (S = A & X; store S & H+1) — opcode 9B
// ===========================================================================

#[test]
fn tas_absy_sets_s_and_stores() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xF0;
    cpu.regs.x = 0x0F;
    cpu.regs.y = 0x00;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9B;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x50; // abs = $5000
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.s, 0x00, "S = A & X = 0x00");
    assert_eq!(bus.mem[0x5000], 0x00, "stored S & (H+1) = 0x00");
}

// ===========================================================================
// LAS (A = X = S = mem & S) — opcode BB
// ===========================================================================

#[test]
fn las_absy_loads_a_x_s_with_mem_and_s() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.s = 0xF0;
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xBB;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x60; // abs = $6000
    bus.mem[0x6001] = 0x0F; // mem[abs+Y]
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x00, "A = mem & S = 0x0F & 0xF0 = 0x00");
    assert_eq!(cpu.regs.x, 0x00, "X = mem & S = 0x00");
    assert_eq!(cpu.regs.s, 0x00, "S = mem & S = 0x00");
}

// ===========================================================================
// Read-NOP variants (no state change, just consume operand)
// ===========================================================================

#[test]
fn nop_zp_does_not_change_memory_or_registers() {
    // 0x04 (NOP zp) and 0x44 (NOP zp read) and 0x64 (NOP zp rmw)
    // all read a byte from zp and do nothing else.
    let variants: &[u8] = &[0x04, 0x44, 0x64];
    for &op in variants {
        let mut cpu = cpu_at(0x4000);
        cpu.regs.a = 0x42;
        cpu.regs.x = 0x33;
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        bus.mem[0x4001] = 0x10;
        bus.mem[0x0010] = 0xAB;
        let pc_before = cpu.regs.pc;
        let a_before = cpu.regs.a;
        let x_before = cpu.regs.x;
        step(&mut cpu, &mut bus);
        assert_eq!(cpu.regs.pc, pc_before + 2, "PC advanced by 2 (opcode + operand)");
        assert_eq!(cpu.regs.a, a_before, "A unchanged by opcode ${:02X}", op);
        assert_eq!(cpu.regs.x, x_before, "X unchanged by opcode ${:02X}", op);
        // 0x44 and 0x64 are read+write; mem unchanged.
        assert_eq!(bus.mem[0x0010], 0xAB, "M unchanged by opcode ${:02X}", op);
    }
}

#[test]
fn nop_imm_does_not_change_a() {
    // 0x80, 0x82, 0x89, 0xC2, 0xE2: NOP imm (just consume operand byte)
    let variants: &[u8] = &[0x80, 0x82, 0x89, 0xC2, 0xE2];
    for &op in variants {
        let mut cpu = cpu_at(0x4000);
        cpu.regs.a = 0x42;
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        bus.mem[0x4001] = 0xAB;
        step(&mut cpu, &mut bus);
        assert_eq!(cpu.regs.pc, 0x4002, "PC advanced by 2");
        assert_eq!(cpu.regs.a, 0x42, "A unchanged by opcode ${:02X}", op);
    }
}

#[test]
fn nop_implied_does_not_change_state() {
    // 0x1A, 0x3A, 0x5A, 0x7A, 0xDA, 0xFA: NOP implied
    let variants: &[u8] = &[0x1A, 0x3A, 0x5A, 0x7A, 0xDA, 0xFA];
    for &op in variants {
        let mut cpu = cpu_at(0x4000);
        cpu.regs.a = 0x42;
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        step(&mut cpu, &mut bus);
        assert_eq!(cpu.regs.pc, 0x4001, "PC advanced by 1");
        assert_eq!(cpu.regs.a, 0x42);
    }
}

#[test]
fn nop_zpx_consumes_zp_and_x_offset() {
    // 0x14, 0x34, 0x54, 0x74, 0xD4, 0xF4: NOP zpx (read byte from zp+X)
    let variants: &[u8] = &[0x14, 0x34, 0x54, 0x74, 0xD4, 0xF4];
    for &op in variants {
        let mut cpu = cpu_at(0x4000);
        cpu.regs.a = 0x42;
        cpu.regs.x = 0x05;
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        bus.mem[0x4001] = 0x10; // zp = $10, +X = $15
        bus.mem[0x0015] = 0xEE;
        step(&mut cpu, &mut bus);
        assert_eq!(cpu.regs.pc, 0x4002);
        assert_eq!(cpu.regs.a, 0x42);
    }
}

#[test]
fn nop_abs_consumes_full_address() {
    // 0x0C: NOP abs (read byte from abs)
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x42;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x0C;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2000] = 0xEE;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.pc, 0x4003);
    assert_eq!(cpu.regs.a, 0x42);
}

#[test]
fn nop_absx_consumes_full_address_plus_x() {
    // 0x1C, 0x3C, 0x5C, 0x7C, 0xDC, 0xFC: NOP abs,X
    let variants: &[u8] = &[0x1C, 0x3C, 0x5C, 0x7C, 0xDC, 0xFC];
    for &op in variants {
        let mut cpu = cpu_at(0x4000);
        cpu.regs.x = 0x05;
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        bus.mem[0x4001] = 0x00;
        bus.mem[0x4002] = 0x20; // abs = $2000 + X = $2005
        bus.mem[0x2005] = 0xEE;
        step(&mut cpu, &mut bus);
        assert_eq!(cpu.regs.pc, 0x4003);
    }
}

// ===========================================================================
// SBC unofficial duplicate — opcode EB (same as E9)
// ===========================================================================

#[test]
fn sbc_imm_eb_is_identical_to_e9() {
    // EB is an undocumented duplicate of E9 (SBC #imm).
    let run_sbc = |op: u8| {
        let mut cpu = cpu_at(0x4000);
        cpu.regs.a = 0x10;
        cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits() | Flags::CARRY.bits();
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = op;
        bus.mem[0x4001] = 0x05;
        step(&mut cpu, &mut bus);
        (cpu.regs.a, cpu.regs.p)
    };
    let (a_e9, p_e9) = run_sbc(0xE9);
    let (a_eb, p_eb) = run_sbc(0xEB);
    assert_eq!(a_e9, a_eb, "EB and E9 produce the same A");
    assert_eq!(p_e9, p_eb, "EB and E9 produce the same P");
    assert_eq!(a_eb, 0x0B, "A = 0x10 - 0x05 = 0x0B");
}

// ===========================================================================
// SLO / RLA / SRE / RRA address-mode coverage (Ind,IndY,ZP,ZPX,Abs,AbsY,AbsX)
// ===========================================================================

#[test]
fn slo_indexed_indirect() {
    // SLO (ind,X): M = (zp + X) → ASL M; A |= M
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x01;
    cpu.regs.x = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x03; // SLO (ind,X)
    bus.mem[0x4001] = 0x20; // zp byte; zp+X = $25
    bus.mem[0x0025] = 0x10; // ptr low byte
    bus.mem[0x0026] = 0x00; // ptr high byte → ptr = $0010
    bus.mem[0x0010] = 0x03; // M at ptr; → ASL → 0x06
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x06, "M[ptr] = 0x03 ASL'd to 0x06");
    assert_eq!(cpu.regs.a, 0x07, "A = 0x01 | 0x06 = 0x07");
}

#[test]
fn slo_indirect_indexed_y() {
    // SLO (ind),Y: M = (zp) + Y → ASL M; A |= M
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x00;
    cpu.regs.y = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x13; // SLO (ind),Y
    bus.mem[0x4001] = 0x20; // zp = $20 → ptr = $3000 (some addr)
    bus.mem[0x0020] = 0x00;
    bus.mem[0x0021] = 0x30; // ptr = $3000
    bus.mem[0x3005] = 0x40; // M = ptr[Y=5]
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x3005], 0x80, "M ASL'd to 0x80");
    assert_eq!(cpu.regs.a, 0x80, "A = 0x00 | 0x80 = 0x80");
}

#[test]
fn slo_zpx() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x01;
    cpu.regs.x = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x17; // SLO zpx
    bus.mem[0x4001] = 0x10; // zp = $10 + X = $15
    bus.mem[0x0015] = 0x40; // → ASL → 0x80
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0015], 0x80);
    assert_eq!(cpu.regs.a, 0x81);
}

#[test]
fn slo_absy() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x00;
    cpu.regs.y = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x1B; // SLO abs,Y
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20; // abs = $2000 + Y = $2005
    bus.mem[0x2005] = 0xFF; // → ASL → 0xFE, C=1
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x2005], 0xFE);
    assert_eq!(cpu.regs.a, 0xFE);
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1");
}

#[test]
fn slo_absx() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x00;
    cpu.regs.x = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x1F; // SLO abs,X
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20; // abs = $2000 + X = $2005
    bus.mem[0x2005] = 0x01; // → ASL → 0x02
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x2005], 0x02);
    assert_eq!(cpu.regs.a, 0x02);
}

#[test]
fn slo_abs() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x0F; // SLO abs
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2000] = 0x80; // → ASL → 0x00, C=1
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x2000], 0x00);
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0);
    assert_eq!(cpu.regs.a, 0xFF, "A unchanged (already 0xFF)");
}

// ===========================================================================
// Coverage matrix for RMW opcodes (per-mode sanity check)
// ===========================================================================

#[test]
fn lax_indx_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xA3; // LAX (ind,X)
    bus.mem[0x4001] = 0x10; // zp byte; zp+X = $15
    bus.mem[0x0015] = 0x00;
    bus.mem[0x0016] = 0x30; // ptr = $3000
    bus.mem[0x3000] = 0x77; // M at ptr (NOT ptr+X)
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x77);
    assert_eq!(cpu.regs.x, 0x77);
}

#[test]
fn lax_zpy_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.y = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xB7; // LAX zp,Y
    bus.mem[0x4001] = 0x10; // zp = $10 + Y = $15
    bus.mem[0x0015] = 0x99;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0x99);
    assert_eq!(cpu.regs.x, 0x99);
}

#[test]
fn lax_absy_loads_a_and_x() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.y = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xBF; // LAX abs,Y
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20; // abs = $2000 + Y = $2005
    bus.mem[0x2005] = 0xCD;
    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.a, 0xCD);
    assert_eq!(cpu.regs.x, 0xCD);
}

#[test]
fn dcp_ind_decrements_then_compares() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x10;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xC3; // DCP (ind,X) — X defaults to 0
    bus.mem[0x4001] = 0x20;
    bus.mem[0x0020] = 0x00;
    bus.mem[0x0021] = 0x50; // ptr = $5000
    bus.mem[0x5000] = 0x10; // M = 16 → DEC → 15. CMP 15 vs A=16: A > M, C=1
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x5000], 0x0F, "M decremented to 15");
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0, "C=1 (A>=M)");
}

#[test]
fn isc_ind_increments_then_sbcs() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x10;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits() | Flags::CARRY.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xE3; // ISC (ind,X) — X defaults to 0
    bus.mem[0x4001] = 0x20;
    bus.mem[0x0020] = 0x00;
    bus.mem[0x0021] = 0x50; // ptr = $5000
    bus.mem[0x5000] = 0x03; // → INC → 0x04. SBC 0x10 - 0x04 = 0x0C
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x5000], 0x04);
    assert_eq!(cpu.regs.a, 0x0C);
    assert_ne!(cpu.regs.p & Flags::CARRY.bits(), 0);
}

#[test]
fn rla_ind_and_others_address_modes() {
    // RLA (ind,X) — sanity check
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x05;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x23; // RLA (ind,X)
    bus.mem[0x4001] = 0x20; // zp byte; zp+X = $25
    bus.mem[0x0025] = 0x10; // ptr low
    bus.mem[0x0026] = 0x00; // ptr high → ptr = $0010 (the EFFECTIVE address)
    bus.mem[0x0010] = 0xAA; // M at the effective address
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x0010], 0x54, "M ROL(C=0) → 0x54");
    assert_eq!(cpu.regs.a, 0x54, "A = 0xFF & 0x54 = 0x54");
}

#[test]
fn sre_ind_address_mode() {
    // SRE (ind,X)
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x0F;
    cpu.regs.x = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x43; // SRE (ind,X)
    bus.mem[0x4001] = 0x20; // zp = $20 + X = $25
    bus.mem[0x0025] = 0x00;
    bus.mem[0x0026] = 0x50; // ptr = $5000
    bus.mem[0x5000] = 0x06; // M → LSR → 0x03, C=0
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x5000], 0x03);
    assert_eq!(cpu.regs.a, 0x0C, "A = 0x0F ^ 0x03 = 0x0C");
}

#[test]
fn rra_ind_address_mode() {
    // RRA (ind,X)
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0x05;
    cpu.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
    cpu.regs.x = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x63; // RRA (ind,X)
    bus.mem[0x4001] = 0x20; // zp = $20 + X = $25
    bus.mem[0x0025] = 0x00;
    bus.mem[0x0026] = 0x50; // ptr = $5000
    bus.mem[0x5000] = 0x02; // M → ROR(C=0) → 0x01. ADC 5+1=6
    step(&mut cpu, &mut bus);
    assert_eq!(bus.mem[0x5000], 0x01);
    assert_eq!(cpu.regs.a, 0x06);
}

// ===========================================================================
// AHX abs,Y = 9F (separate from AHX (ind),Y = 93)
// ===========================================================================

#[test]
fn ahx_absy_stores_a_and_x_and_h_plus_1() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x0F;
    cpu.regs.y = 0x05;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9F; // AHX abs,Y
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x60; // abs = $6000 + Y = $6005
    step(&mut cpu, &mut bus);
    // A=0xFF & X=0x0F = 0x0F, & (H+1=0x61) = 0x01
    assert_eq!(bus.mem[0x6005], 0x01);
}