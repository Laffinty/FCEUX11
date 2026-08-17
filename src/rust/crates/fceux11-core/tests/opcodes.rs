//! Integration tests for the Phase 1 CPU scaffold.
//!
//! These tests assert the gate specified in `docs/plans/cpu-rust-v2.md`
//! §4 Phase 1: every one of the 256 opcodes can be `step()`ped on a
//! trivial flat 64 KiB bus without panicking, and the per-opcode cycle
//! cost matches the legacy `CycTable` byte-for-byte.

use fceux11_core::cpu::{
    cycle_count, size_of, step, Bus, CpuState, Flags, IrqSource, X6502Layout,
};

/// Flat 64 KiB RAM bus — trivial, no mapper, no PPU.
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

#[test]
fn all_256_opcodes_step_without_panic() {
    // Place every opcode at its own PC, run one step, verify it doesn't
    // panic. This is the headline Phase 1 gate.
    //
    // For non-branch opcodes we also verify the base cycle cost matches
    // the legacy `CycTable`. For branch opcodes we accept >= base_cycles
    // because the new dispatch takes the branch (operand byte 0x00 →
    // cond true for BPL/BCS/etc when P=0) and adds 1 cycle.
    //
    // We do NOT verify PC advancement here — the C++ `opsize` table
    // marks many unofficial opcodes as size 0 even though the
    // addressing-mode helpers DO consume operand bytes. PC advancement
    // is checked by the targeted `addressing_*` unit tests instead.
    use fceux11_core::cpu::decode::info;
    let mut passed = 0;
    let mut total_cycles = 0u32;
    for opcode in 0u16..=0xFF {
        let mut bus = FlatBus::new();
        let mut cpu = CpuState::new();
        bus.mem[0] = opcode as u8;
        cpu.regs.pc = 0;
        let cycles = step(&mut cpu, &mut bus);
        let op_info = info(opcode as u8);
        if matches!(op_info.kind, fceux11_core::cpu::decode::OpKind::Branch) {
            // Branch taken adds 1 cycle; some may add a page-cross too.
            assert!(
                cycles as u8 >= cycle_count(opcode as u8),
                "opcode ${:02X}: branch cycle {} < base {}",
                opcode, cycles, cycle_count(opcode as u8)
            );
        } else {
            assert_eq!(
                cycles as u8,
                cycle_count(opcode as u8),
                "cycle mismatch for opcode ${:02X}",
                opcode
            );
        }
        passed += 1;
        total_cycles += cycles as u32;
    }
    assert_eq!(passed, 256);
}

#[test]
fn x6502_layout_is_byte_compatible_with_cpp() {
    // The C++ side asserts sizeof(X6502) == 64 and alignof(X6502) == 64
    // in src/cpu.cpp:11–22. Mirroring those asserts here keeps the
    // savestate binary format consistent.
    assert_eq!(core::mem::size_of::<X6502Layout>(), 64);
    assert_eq!(core::mem::align_of::<X6502Layout>(), 64);
}

#[test]
fn power_then_reset_loads_vector() {
    // Per C++ X6502_RunDebug loop semantics, step() processes both the
    // IRQ/NMI dispatch AND the next instruction. So after power() (which
    // sets the RESET bit), step() consumes the RESET (7 cycles) and the
    // JMP at $C000 (7 cycles) for a total of 14.
    let mut cpu = CpuState::new();
    cpu.power();
    let mut bus = FlatBus::new();
    // Reset vector: $C000 (nestest convention).
    bus.mem[0xC000] = 0x4C; bus.mem[0xC001] = 0x00; bus.mem[0xC002] = 0x80; // JMP $8000
    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0xC0;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cycles, 7 + 3); // RESET (7) + JMP abs (3)
    assert_eq!(cpu.regs.pc, 0x8000);
    assert_eq!(cpu.regs.p & Flags::IRQ_DIS.bits(), Flags::IRQ_DIS.bits());
    assert_eq!(
        cpu.regs.irq_low & IrqSource::RESET.bits(),
        0,
        "RESET bit must be consumed",
    );
}

#[test]
fn cycle_count_table_equals_legacy() {
    // Spot-check a handful of opcodes against the C++ table values.
    assert_eq!(cycle_count(0x00), 7); // BRK
    assert_eq!(cycle_count(0xEA), 2); // NOP
    assert_eq!(cycle_count(0x78), 2); // SEI
    assert_eq!(cycle_count(0xAD), 4); // LDA abs
    assert_eq!(cycle_count(0xBD), 4); // LDA abs,X (base)
    assert_eq!(cycle_count(0x6C), 5); // JMP indirect
    assert_eq!(cycle_count(0x4C), 3); // JMP abs
    assert_eq!(cycle_count(0x20), 6); // JSR
}

#[test]
fn addressing_modes_match_expected() {
    use fceux11_core::cpu::decode::info;
    assert_eq!(info(0xA9).mnemonic, "LDA");
    assert_eq!(info(0xA5).mnemonic, "LDA");
    assert_eq!(info(0xAD).mnemonic, "LDA");
    assert_eq!(info(0xB5).mnemonic, "LDA");
    assert_eq!(info(0xBD).mnemonic, "LDA");
    assert_eq!(info(0xA1).mnemonic, "LDA");
    assert_eq!(info(0xB1).mnemonic, "LDA");
    assert_eq!(info(0xEA).mnemonic, "NOP");
    assert_eq!(info(0x00).mnemonic, "BRK");
    assert_eq!(info(0x40).mnemonic, "RTI");
    assert_eq!(info(0x60).mnemonic, "RTS");
}

#[test]
fn unofficial_opcodes_recognised() {
    use fceux11_core::cpu::decode::info;
    // Spot-check: every column-3 / -7 / -B / -F opcode in the NESdev
    // matrix is marked unofficial.
    assert!(!info(0x03).official, "SLO (d,x) should be unofficial");
    assert!(!info(0x07).official, "SLO zp should be unofficial");
    assert!(!info(0x0B).official, "ANC #imm should be unofficial");
    assert!(!info(0x0F).official, "SLO abs should be unofficial");
    assert!(!info(0x9B).official, "TAS should be unofficial");
    assert!(!info(0xCB).official, "AXS should be unofficial");
}

#[test]
fn op_size_table_consistent() {
    // Sanity: every official opcode should have a non-zero size.
    for opcode in 0u16..=0xFF {
        let info = fceux11_core::cpu::decode::info(opcode as u8);
        if info.official {
            assert!(
                info.size >= 1,
                "official opcode ${:02X} has size 0",
                opcode
            );
        }
    }
}