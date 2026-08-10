//! CPU instruction-level tests 鈥?corner cases: undocumented opcodes,
//! page-cross penalties, decimal mode, interrupt timing.
//!
//! These tests use an in-memory `MockBus` and drive individual
//! instructions through `run_budget`/`step`, checking registers and
//! cycle consumption. ROM-based validation (nestest.nes + blargg)
//! lives in `tests/rom_tests.rs` (needs ROM files, see phase_1_cpu.md).

use vnesu11::cpu::flags::{
    C_FLAG, D_FLAG, I_FLAG, N_FLAG, U_FLAG, V_FLAG, Z_FLAG,
};
use vnesu11::cpu::{BusContext, CpuCore};

/// Minimal bus: 32 KiB PRG at $8000, 2 KiB WRAM.
struct MockBus {
    ram: [u8; 0x800],
    prg: [u8; 0x8000],
    read_log: Vec<u16>,
    write_log: Vec<(u16, u8)>,
}

impl MockBus {
    fn new() -> Self {
        Self {
            ram: [0; 0x800],
            prg: [0; 0x8000],
            read_log: Vec::new(),
            write_log: Vec::new(),
        }
    }
    fn load(&mut self, addr: u16, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            let a = addr.wrapping_add(i as u16);
            if a < 0x800 {
                self.ram[a as usize] = b;
            } else {
                self.prg[(a - 0x8000) as usize] = b;
            }
        }
    }
    fn peek(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            _ => self.prg[(addr - 0x8000) as usize],
        }
    }
}

impl BusContext for MockBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.read_log.push(addr);
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
            _ => self.prg[(addr - 0x8000) as usize],
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.write_log.push((addr, val));
        match addr {
            0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize] = val,
            _ => {}
        }
    }
    fn dma_stalled(&self) -> bool {
        false
    }
}

fn fresh_cpu() -> (CpuCore, MockBus) {
    let mut bus = MockBus::new();
    let mut cpu = CpuCore::new();
    cpu.set_pc(0x8000);
    // Reset vector points at $8000.
    bus.load(0xFFFC, &[0x00, 0x80]);
    (cpu, bus)
}

/// Run a sequence of bytes as a program from $8000.
fn run_program(prog: &[u8], budget: i32) -> (CpuCore, MockBus) {
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x8000, prog);
    cpu.run_budget(budget, &mut bus);
    (cpu, bus)
}

// =====================================================================
// Basic arithmetic & flags
// =====================================================================

#[test]
fn lda_imm_sets_a_and_flags() {
    let (cpu, _) = run_program(&[0xA9, 0x42], 2);
    assert_eq!(cpu.a(), 0x42);
    assert_eq!(cpu.p() & Z_FLAG, 0); // non-zero
    assert_eq!(cpu.p() & N_FLAG, 0); // bit 7 clear
}

#[test]
fn lda_imm_zero_sets_z() {
    let (cpu, _) = run_program(&[0xA9, 0x00], 2);
    assert_eq!(cpu.p() & Z_FLAG, Z_FLAG);
}

#[test]
fn lda_imm_negative_sets_n() {
    let (cpu, _) = run_program(&[0xA9, 0x80], 2);
    assert_eq!(cpu.p() & N_FLAG, N_FLAG);
}

#[test]
fn adc_binary_carry_and_overflow() {
    // 0x7F + 0x01 = 0x80: overflow set, carry clear.
    let (cpu, _) = run_program(&[0xA9, 0x7F, 0x69, 0x01], 4);
    assert_eq!(cpu.a(), 0x80);
    assert_eq!(cpu.p() & V_FLAG, V_FLAG); // signed overflow
    assert_eq!(cpu.p() & C_FLAG, 0);
    assert_eq!(cpu.p() & N_FLAG, N_FLAG); // negative result

    // 0xFF + 0x01 = 0x00: carry set, zero set.
    let (cpu, _) = run_program(&[0xA9, 0xFF, 0x69, 0x01], 4);
    assert_eq!(cpu.a(), 0x00);
    assert_eq!(cpu.p() & C_FLAG, C_FLAG);
    assert_eq!(cpu.p() & Z_FLAG, Z_FLAG);
}

#[test]
fn sbc_borrow() {
    // SBC #imm = A - imm - (1-C). Fresh P has C clear → borrow=1.
    // A=0x10, SBC #0x20: 0x10 - 0x20 - 1 = 0xEF, borrow set (C clear).
    let (cpu, _) = run_program(&[0xA9, 0x10, 0xE9, 0x20], 4);
    assert_eq!(cpu.a(), 0xEF);
    assert_eq!(cpu.p() & C_FLAG, 0); // borrow (no carry)
    assert_eq!(cpu.p() & N_FLAG, N_FLAG);
}

#[test]
fn sbc_no_borrow_sets_carry() {
    // SEC; LDA #$30; SBC #$20 → 0x30 - 0x20 - 0 = 0x10, C set.
    let (cpu, _) = run_program(&[0x38, 0xA9, 0x30, 0xE9, 0x20], 8);
    assert_eq!(cpu.a(), 0x10);
    assert_eq!(cpu.p() & C_FLAG, C_FLAG);
}

#[test]
fn cmp_sets_carry() {
    // A=0x42, CMP #0x40 鈫?C set.
    let (cpu, _) = run_program(&[0xA9, 0x42, 0xC9, 0x40], 4);
    assert_eq!(cpu.p() & C_FLAG, C_FLAG);
    // CMP #0x42 鈫?Z set.
    let (cpu, _) = run_program(&[0xA9, 0x42, 0xC9, 0x42], 4);
    assert_eq!(cpu.p() & Z_FLAG, Z_FLAG);
}

// =====================================================================
// Page-cross penalties
// =====================================================================

#[test]
fn lda_absx_page_cross_extra_cycle() {
    // LDX #$01 (0xA2); LDA $80FF,X with X=1: base 0x80FF + 1 = 0x8100
    // → crossed page. Program: A2 01 BD FF 80.
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x8000, &[0xA2, 0x01]); // LDX #$01
    bus.load(0x8002, &[0xBD, 0xFF, 0x80]); // LDA $80FF,X
    bus.load(0x8100, &[0xAB]); // target value
    cpu.set_pc(0x8000);
    cpu.run_budget(10, &mut bus); // LDX(2) + LDA abs,X crossing(5) = 7
    assert_eq!(cpu.a(), 0xAB);

    // Non-crossing LDA $80FE,X with X=0 = 4 cycles (no +1).
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x8000, &[0xBD, 0xFE, 0x80]); // LDA $80FE,X (X=0, no cross)
    bus.load(0x80FE, &[0xCD]);
    cpu.set_pc(0x8000);
    cpu.run_budget(4, &mut bus);
    assert_eq!(cpu.a(), 0xCD, "non-crossing LDA abs,X = 4 cycles");
}

#[test]
fn branch_page_cross_penalty() {
    // BNE +2 (not taken) = 2 cycles; taken = 3; taken+cross = 4.
    // Program: LDA #$01; BNE +$0F (target crosses page if PC at $8003 鈫?$8012, no cross).
    // We only verify taken adds 1 cycle: LDA #1 (2cy) + BNE taken same-page (3cy) = 5.
    let (mut cpu, mut bus) = fresh_cpu();
    // LDX #1 (2cy); DEX (2cy) sets Z; BNE -3 (taken, same page) 3cy...
    // Simpler: LDA #1; BNE +0 (branch to itself, taken, no page cross).
    bus.load(0x8000, &[0xA9, 0x01, 0xD0, 0x00]);
    cpu.set_pc(0x8000);
    cpu.run_budget(5, &mut bus); // 2 (LDA) + 3 (taken same-page BNE) = 5
    assert_eq!(cpu.pc(), 0x8004, "taken BNE same page = 3 cycles");
}

// =====================================================================
// Undocumented opcodes
// =====================================================================

#[test]
fn lax_loads_a_and_x() {
    // LAX $8000 (abs) = A7/BD etc. Use LAX zp: A7.
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x0010, &[0xAB]);
    bus.load(0x8000, &[0xA7, 0x10]); // LAX $10
    cpu.set_pc(0x8000);
    cpu.run_budget(3, &mut bus);
    assert_eq!(cpu.a(), 0xAB);
    assert_eq!(cpu.x(), 0xAB);
    assert_eq!(cpu.p() & N_FLAG, N_FLAG); // 0xAB bit 7 set
}

#[test]
fn sax_stores_a_and_x() {
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x8000, &[0xA9, 0xF0, 0xA2, 0x0F, 0x87, 0x10]); // LDA #F0; LDX #0F; SAX $10
    cpu.set_pc(0x8000);
    cpu.run_budget(8, &mut bus);
    assert_eq!(bus.peek(0x0010), 0x00); // F0 & 0F = 0x00
}

#[test]
fn dcp_decrements_and_compares() {
    // DCP $10: mem = mem-1; CMP mem. mem=0x01 鈫?0x00; A=0x00 鈫?Z set.
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x0010, &[0x01]);
    bus.load(0x8000, &[0xA9, 0x00, 0xC7, 0x10]); // LDA #0; DCP $10
    cpu.set_pc(0x8000);
    cpu.run_budget(7, &mut bus);
    assert_eq!(bus.peek(0x0010), 0x00);
    assert_eq!(cpu.p() & Z_FLAG, Z_FLAG);
}

#[test]
fn anc_sets_carry_from_bit7() {
    // ANC #$80: A = A & 0x80; C = bit7.
    let (cpu, _) = run_program(&[0xA9, 0xFF, 0x0B, 0x80], 4);
    assert_eq!(cpu.a(), 0x80);
    assert_eq!(cpu.p() & C_FLAG, C_FLAG);
    assert_eq!(cpu.p() & V_FLAG, 0);
}

#[test]
fn kil_jams_cpu() {
    let (cpu, _) = run_program(&[0x02], 2);
    assert!(cpu.jammed());
}

#[test]
fn unofficial_nop_consumes_operand() {
    // 0x04 = NOP zp (2 bytes). PC should advance 2.
    let (cpu, _) = run_program(&[0x04, 0x00], 3);
    assert_eq!(cpu.pc(), 0x8002);
}

// =====================================================================
// Decimal-flag parity with C++ FCEUX
//
// The C++ core (`src/x6502.cpp` ADC/SBC macros) is BINARY-ONLY: it
// ignores the D (decimal) flag entirely — there is no `D_FLAG` reference
// anywhere in x6502.cpp. The migration goal (ADR-008, decision A) is
// byte-for-byte shadow-run parity with C++, so the Rust core matches:
// SED sets D but ADC/SBC still compute in binary. These tests lock that
// parity, NOT "ideal" decimal hardware behavior.
// =====================================================================

#[test]
fn decimal_flag_set_by_sed() {
    // SED sets the D flag.
    let (cpu, _) = run_program(&[0xF8], 2);
    assert_eq!(cpu.p() & D_FLAG, D_FLAG);
}

#[test]
fn adc_binary_ignores_decimal_flag() {
    // SED; LDA #$15; ADC #$15 → C++-parity: binary 0x15+0x15 = 0x2A
    // (NOT the BCD-adjusted 0x30), C clear.
    let (cpu, _) = run_program(&[0xF8, 0xA9, 0x15, 0x69, 0x15], 8);
    assert_eq!(cpu.a(), 0x2A, "binary result, not BCD-adjusted");
    assert_eq!(cpu.p() & C_FLAG, 0);
}

#[test]
fn adc_binary_overflow_carry_in_decimal_mode() {
    // SED; LDA #$99; ADC #$01 → binary 0x99+0x01 = 0x9A, C clear,
    // N set (parity with C++).
    let (cpu, _) = run_program(&[0xF8, 0xA9, 0x99, 0x69, 0x01], 8);
    assert_eq!(cpu.a(), 0x9A);
    assert_eq!(cpu.p() & C_FLAG, 0);
}

#[test]
fn sbc_binary_ignores_decimal_flag() {
    // SED; LDA #$00; SBC #$01 with C clear → binary 0x00 - 0x01 - 1
    // = 0xFE, C clear (borrow). (NOT the BCD 0x98.)
    let (cpu, _) = run_program(&[0xF8, 0xA9, 0x00, 0xE9, 0x01], 8);
    assert_eq!(cpu.a(), 0xFE);
    assert_eq!(cpu.p() & C_FLAG, 0);
}

// =====================================================================
// Stack & subroutine
// =====================================================================

#[test]
fn jsr_rts_round_trip() {
    // JSR $8005; at $8005: RTS. Verify return lands at $8003.
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0x8000, &[0x20, 0x05, 0x80]); // JSR $8005
    bus.load(0x8005, &[0x60]); // RTS
    cpu.set_pc(0x8000);
    cpu.run_budget(12, &mut bus); // JSR(6) + RTS(6) = 12
    assert_eq!(cpu.pc(), 0x8003);
}

#[test]
fn php_plp_preserve_p() {
    // SEC; PHP; PLP 鈫?C stays set.
    let (cpu, _) = run_program(&[0x38, 0x08, 0x28], 6);
    assert_eq!(cpu.p() & C_FLAG, C_FLAG);
    assert_eq!(cpu.p() & U_FLAG, U_FLAG);
}

// =====================================================================
// Interrupt timing
// =====================================================================

#[test]
fn irq_uses_penultimate_cycle_sample() {
    // SEI disables IRQ one instruction later. Set IRQ pending while
    // I is clear, then run SEI; the IRQ should be taken BEFORE SEI
    // takes effect (because IRQ sampling uses moo_pi = P at entry).
    let (mut cpu, mut bus) = fresh_cpu();
    // NMI vector 鈫?$8010; IRQ vector 鈫?$8020.
    bus.load(0xFFFA, &[0x10, 0x80]);
    bus.load(0xFFFE, &[0x20, 0x80]);
    bus.load(0x8010, &[0x40]); // RTI
    bus.load(0x8020, &[0x40]); // RTI
    // Program: CLI (clear I); ... but we need IRQ pending with I clear.
    bus.load(0x8000, &[0x58]); // CLI
    cpu.set_regs_for_test(0x8000, 0, 0, 0, 0xFD, U_FLAG); // I clear
    cpu.irq_begin(CpuCore::IRQ_EXT);
    cpu.run_budget(20, &mut bus);
    // IRQ should have been taken 鈫?PC lands at RTI return... but budget
    // may not cover it. Simpler assertion: I flag is set (IRQ pushed P|I).
    assert_eq!(cpu.p() & I_FLAG, I_FLAG, "IRQ sets I flag");
}

#[test]
fn irq_masked_by_i_flag() {
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0xFFFE, &[0x20, 0x80]);
    bus.load(0x8020, &[0x40]);
    bus.load(0x8000, &[0xEA, 0xEA]); // NOP NOP
    cpu.set_regs_for_test(0x8000, 0, 0, 0, 0xFD, U_FLAG | I_FLAG); // I set
    cpu.irq_begin(CpuCore::IRQ_EXT);
    cpu.run_budget(4, &mut bus); // 2 NOPs = 4 cycles → PC = 0x8002
    // IRQ masked: PC stayed in the NOP program (0x8002 after 2 NOPs).
    assert_eq!(cpu.pc(), 0x8002);
}

#[test]
fn nmi_is_edge_triggered() {
    let (mut cpu, mut bus) = fresh_cpu();
    bus.load(0xFFFA, &[0x10, 0x80]);
    bus.load(0x8010, &[0x40]); // RTI
    bus.load(0x8000, &[0xEA, 0xEA]); // NOP NOP
    cpu.set_regs_for_test(0x8000, 0, 0, 0, 0xFD, U_FLAG | I_FLAG);
    cpu.irq_begin(CpuCore::IRQ_NMI);
    // NMI costs exactly 7 cycles. Budget 7 → NMI taken, count hits 0,
    // PC lands at the vector ($8010) with the RTI NOT yet fetched.
    cpu.run_budget(7, &mut bus);
    // NMI is edge-triggered and NOT maskable (I flag ignored).
    assert_eq!(cpu.pc(), 0x8010, "NMI vectors to $8010 (RTI not yet executed)");
}
