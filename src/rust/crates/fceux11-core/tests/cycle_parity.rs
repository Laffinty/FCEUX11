//! Per-instruction cycle-accounting parity tests for the Rust 6502 CPU.
//!
//! Phase 4 sub-step 5 of `docs/plans/cpu-rust-v2.md`. These tests pin
//! down the `step()` return value and the `count` accumulator delta for
//! every relevant instruction shape, so that a cycle-accounting drift
//! surfaces as a per-instruction assertion failure instead of a frame-
//! hash mismatch in `rom_regression_rust_smoke`.
//!
//! ## What we assert
//!
//! For each step:
//!   1. `step()` return value == `CycTable[opcode] + extras` (page-cross,
//!      branch-taken, RMW dummy cycles)
//!   2. `state.regs.count` delta == `(CycTable + extras) * 16` (the
//!      `count` accumulator is in 1/16-CPU-cycle units, matching the
//!      C++ `_count += cycles * 16` budget scaling)
//!   3. PC advanced to the expected post-instruction address
//!
//! The tests run on a flat 64 KiB bus with NO IRQ / NMI / RESET firing,
//! so `dispatch_irq` always returns 0. This isolates the per-instruction
//! accounting from the per-frame drift problem (sub-step 5 separate).
//!
//! ## C++ reference math
//!
//! For `X6502_RunDebug(cycles)` in `src/x6502.cpp`:
//!   - `_count += cycles * 16`        (ascending budget, 1/16 units)
//!   - Each instruction: `_count -= CycTable * 48` (decrement in
//!     1/48-CPU-cycle units, i.e. 1/3 of the Rust count unit)
//!   - Loop breaks when `_count <= 0`
//!
//! The Rust `fceux11_cpu_run` does the equivalent with the polarity
//! inverted: ascending `count`, each step adds
//! `(base + extras) * 16`. The unit conversion between the two is
//! folded into the FFI shim's `cycles * 16` scaling — per-instruction,
//! both should accumulate the same count delta for the same
//! instruction stream. These tests pin the Rust side; the C++ side is
//! the gold standard.

use fceux11_core::cpu::{Bus, CpuState, IrqSource, decode::info, run, step};

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct FlatBus {
    mem: [u8; 0x10000],
}

impl FlatBus {
    fn new() -> Self {
        Self { mem: [0; 0x10000] }
    }
    /// Fill [$start, $start+$len) with a repeating byte.
    fn fill(&mut self, start: u16, data: &[u8]) {
        for (i, &b) in data.iter().enumerate() {
            self.mem[start as usize + i] = b;
        }
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

/// Build a CpuState at $4000 with I|U flags, S=$FD, all IRQ bits cleared.
/// `dispatch_irq` always returns 0 for these tests.
fn cpu_at(pc: u16) -> CpuState {
    let mut cpu = CpuState::new();
    cpu.regs.pc = pc;
    cpu.regs.s = 0xFD;
    cpu.regs.p = 0x24; // I_FLAG | U_FLAG (post-RESET P value)
    cpu.regs.moo_pi = cpu.regs.p;
    cpu.regs.irq_low = 0;
    cpu.nmi_fresh = false;
    cpu
}

// ===========================================================================
// 1. Base-cycle parity: no extras
// ===========================================================================

#[test]
fn nop_base_2_no_extras() {
    // $4000: EA (NOP implied, base=2, no extras)
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 2, "NOP base cycle cost");
    assert_eq!(
        cpu.regs.count, -96,
        "count delta = -2 * 48 (C++ _count polarity)"
    );
    assert_eq!(cpu.regs.pc, 0x4001);
    assert!(cpu.regs.jammed == 0);
}

#[test]
fn lda_imm_base_2() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xA9;
    bus.mem[0x4001] = 0x42;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 2, "LDA #imm base");
    assert_eq!(cpu.regs.count, -96);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(cpu.regs.pc, 0x4002);
}

#[test]
fn lda_zp_base_3() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xA5;
    bus.mem[0x4001] = 0x10;
    bus.mem[0x0010] = 0x77;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 3, "LDA zp base");
    assert_eq!(cpu.regs.count, -144);
    assert_eq!(cpu.regs.a, 0x77);
}

#[test]
fn lda_abs_base_4() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xAD;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2000] = 0xAB;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 4, "LDA abs base");
    assert_eq!(cpu.regs.count, -192);
    assert_eq!(cpu.regs.a, 0xAB);
}

#[test]
fn brk_base_7_no_dispatch() {
    // BRK is opcode 0x00, dispatch should NOT fire (no IRQ pending) and
    // step() returns the base cycle cost (7). PC ends up at the BRK
    // vector = $FFFE/$FFFF contents.
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x00; // BRK
    bus.mem[0x4001] = 0xAA; // BRK signature byte
    bus.mem[0xFFFE] = 0x00;
    bus.mem[0xFFFF] = 0x50;
    bus.mem[0x5000] = 0xEA; // NOP at the IRQ vector (next step's job)
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 7, "BRK base cycle cost");
    assert_eq!(cpu.regs.count, -336, "count delta = -7 * 48");
    assert_eq!(cpu.regs.pc, 0x5000, "PC = BRK vector");
    assert_eq!(cpu.regs.s, 0xFA, "3 pushes (PCH/PCL/P|B)");
}

// ===========================================================================
// 2. Extras parity: page-cross / branch-taken / RMW
// ===========================================================================

#[test]
fn lda_abs_x_no_page_cross_base_4() {
    // $4000: BD 00 20 (LDA $2000,X), X=0 → no page cross
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x00;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xBD;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2000] = 0xCD;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 4, "LDA abs,X base when no page cross (base=4, no extra)");
    assert_eq!(cpu.regs.count, -192);
    assert_eq!(cpu.regs.a, 0xCD);
}

#[test]
fn lda_abs_x_page_cross_extra_1() {
    // LDA $20FF,X with X=1 → effective $2100, page cross → +1 cycle
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xBD;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2100] = 0xEE;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 4 + 1, "LDA abs,X with page cross");
    assert_eq!(cpu.regs.count, (4 + 1) * -48);
    assert_eq!(cpu.regs.a, 0xEE);
}

#[test]
fn branch_taken_no_page_cross_extra_1() {
    // BEQ +5 from $4000 (target $4007, same page)
    let mut cpu = cpu_at(0x4000);
    cpu.regs.p = 0x24 | 0x02; // I|U|Z set, branch taken
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xF0;
    bus.mem[0x4001] = 0x05;
    bus.mem[0x4007] = 0xEA; // NOP at branch target
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 2 + 1, "BEQ taken, no page cross (base=2 + taken=1)");
    assert_eq!(cpu.regs.count, (2 + 1) * -48);
    assert_eq!(cpu.regs.pc, 0x4007);
}

#[test]
fn branch_taken_page_cross_extra_2() {
    // BCC at $4080 with displacement +$7E (signed = +126):
    //   pre = $4082 (PC after fetch)
    //   target = $4082 + $7E = $4100 (page crossed: $40 → $41)
    let mut cpu = cpu_at(0x4080);
    cpu.regs.p = 0x24; // I|U, C=0, BCC taken
    let mut bus = FlatBus::new();
    bus.mem[0x4080] = 0x90; // BCC
    bus.mem[0x4081] = 0x7E; // +$7E → $4100 (page crossed)
    bus.mem[0x4100] = 0xEA;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(
        c,
        2 + 1 + 1,
        "BCC taken, page crossed (base=2 + taken=1 + page=1)"
    );
    assert_eq!(cpu.regs.count, (2 + 1 + 1) * -48);
    assert_eq!(cpu.regs.pc, 0x4100);
}

#[test]
fn branch_not_taken_no_extra() {
    // BEQ +5 from $4000 with Z=0 → not taken
    let mut cpu = cpu_at(0x4000);
    cpu.regs.p = 0x24; // I|U, Z=0, branch not taken
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xF0;
    bus.mem[0x4001] = 0x05;
    bus.mem[0x4002] = 0xEA;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 2, "BEQ not taken, no extra");
    assert_eq!(cpu.regs.count, -96);
    assert_eq!(cpu.regs.pc, 0x4002);
}

// ===========================================================================
// 3. Stream parity: 256 NOPs sum to 256 * 2 cycles
// ===========================================================================

#[test]
fn stream_256_nops_sums_to_512_cycles() {
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.fill(0x4000, &[0xEA; 256]);
    let mut total_cycles: u32 = 0;
    for _ in 0..256 {
        let c = step(&mut cpu, &mut bus);
        total_cycles = total_cycles.saturating_add(c as u32);
    }
    assert_eq!(total_cycles, 512, "256 NOPs @ 2 cycles each = 512");
    assert_eq!(
        cpu.regs.count,
        -(512 * 48),
        "count delta = -512 * 48 = -24576"
    );
    assert_eq!(cpu.regs.pc, 0x4100, "PC advanced 256 bytes from $4000");
}

// ===========================================================================
// 4. Run() budget parity: each X6502_Run(n) call consumes ~ n/3 cycles
// ===========================================================================

/// Pin the `run()` cycle-budget math. For a NOP stream with
/// `scaled_cycles = cycles * 16` (1/16-CPU-cycle units):
///   - `count` starts as `start + scaled_cycles` (budget added, like
///     C++ `_count += cycles*16`)
///   - each NOP decrements `count` by `dot(2) * 3 = 96`
///   - the loop exits when `count <= 0` (C++ `while (_count > 0)`)
///   - N NOPs needed: N*96 >= scaled_cycles
#[test]
fn run_cycles_consumes_proportional_to_budget() {
    use fceux11_core::cpu::run;
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.fill(0x4000, &[0xEA; 4096]);
    let start_count = cpu.regs.count;

    // The FFI passes scaled_cycles = cycles * 16 to run(). Budget is
    // added, then each NOP decrements 96. For scaled = 1536, the loop
    // runs 16 NOPs (16*96 = 1536) and returns count to its start value.
    let scaled = 96 * 16; // 96 cycles in 1/16 units
    let consumed = run(&mut cpu, &mut bus, scaled);
    let inst_count = cpu.regs.count - start_count;
    assert_eq!(
        inst_count, 0,
        "16 NOPs * -96 exactly exhaust the 1536 budget"
    );
    assert_eq!(inst_count % 96, 0, "count delta must be a multiple of -96");
    let n_nops = scaled / 96;
    assert_eq!(consumed, n_nops * 2, "consumed cycles = 2 per NOP");
    assert_eq!(n_nops, 16);
    assert_eq!(cpu.regs.pc, 0x4010, "16 bytes of NOPs consumed");
}

// ===========================================================================
// 5. OpKind dispatch: every OpKind returns the expected base + extras
// ===========================================================================

#[test]
fn every_opkind_returns_documented_cycles() {
    // Spot-check: for each OpKind, place a representative opcode at
    // $4000 with operand bytes, then assert step() == CycTable + extras.
    // This catches regressions in the dispatcher's per-kind cycle math.
    struct Case {
        opcode: u8,
        operands: &'static [u8],
        mem_setup: &'static [(u16, &'static [u8])],
        expected_cycles: u8,
        kind_name: &'static str,
    }
    let cases = &[
        Case {
            opcode: 0xEA,
            operands: &[],
            mem_setup: &[],
            expected_cycles: 2, // NOP
            kind_name: "NopRead",
        },
        Case {
            opcode: 0xA9,
            operands: &[0x42],
            mem_setup: &[],
            expected_cycles: 2, // LDA #imm
            kind_name: "Load",
        },
        Case {
            opcode: 0x8D,
            operands: &[0x00, 0x20],
            mem_setup: &[],
            expected_cycles: 4, // STA abs
            kind_name: "Store",
        },
        Case {
            opcode: 0xE6,
            operands: &[0x10],
            mem_setup: &[(0x0010, &[0x00])],
            expected_cycles: 5, // INC zp (RMW)
            kind_name: "Rmw",
        },
        Case {
            opcode: 0x69,
            operands: &[0x01],
            mem_setup: &[],
            expected_cycles: 2, // ADC #imm
            kind_name: "AluA",
        },
        Case {
            opcode: 0xC9,
            operands: &[0x10],
            mem_setup: &[],
            expected_cycles: 2, // CMP #imm
            kind_name: "Compare",
        },
        Case {
            opcode: 0x24,
            operands: &[0x10],
            mem_setup: &[(0x0010, &[0xC0])],
            expected_cycles: 3, // BIT zp
            kind_name: "Bit",
        },
        Case {
            opcode: 0x4C,
            operands: &[0x00, 0x50],
            mem_setup: &[],
            expected_cycles: 3, // JMP abs
            kind_name: "Jump",
        },
    ];

    for c in cases {
        let mut cpu = cpu_at(0x4000);
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = c.opcode;
        for (i, &b) in c.operands.iter().enumerate() {
            bus.mem[0x4001 + i] = b;
        }
        for (addr, data) in c.mem_setup {
            for (i, &b) in data.iter().enumerate() {
                bus.mem[*addr as usize + i] = b;
            }
        }
        let info = info(c.opcode);
        assert_eq!(
            info.mnemonic,
            match c.kind_name {
                "NopRead" => "NOP",
                "Load" => "LDA",
                "Store" => "STA",
                "Rmw" => "INC",
                "AluA" => "ADC",
                "Compare" => "CMP",
                "Bit" => "BIT",
                "Jump" => "JMP",
                _ => panic!("unknown kind_name {}", c.kind_name),
            },
            "opcode ${:02X} decoded as wrong mnemonic",
            c.opcode
        );

        let actual = step(&mut cpu, &mut bus);
        assert_eq!(
            actual, c.expected_cycles,
            "{} (${:02X}) expected {} cycles, got {}",
            c.kind_name, c.opcode, c.expected_cycles, actual
        );
    }
}

// ===========================================================================
// 6. State flag consistency: PC and SP advance correctly across extras
// ===========================================================================

#[test]
fn ldx_abs_y_with_page_cross_consistent_count() {
    // LDX $20FF,Y (opcode $BE) with Y=1 → $2100, page crossed
    let mut cpu = cpu_at(0x4000);
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xBE; // LDX abs,Y
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x20;
    bus.mem[0x2100] = 0xAB;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 4 + 1, "LDX abs,Y page-crossed");
    assert_eq!(cpu.regs.x, 0xAB);
    assert_eq!(cpu.regs.pc, 0x4003);
    assert_eq!(cpu.regs.count, (4 + 1) * -48);
}

// ===========================================================================
// 7. Multi-instruction cycle accounting: dispatch-budget early-exit
//    Mirrors the C++ `X6502_RunDebug` early-exit at
//    `src/x6502.cpp:586-588`. The cycle-drift fix closes this:
//    when dispatch consumes the entire remaining per-call budget, the
//    follow-up instruction must NOT execute (matches C++ behaviour
//    to the byte).
// ===========================================================================

#[test]
fn dispatch_just_exhausts_budget_does_not_execute_followup_instruction() {
    // $4000: NOP, $5000: NOP (the would-be follow-up instruction).
    // $FFFA/B points to $5000. NMI is pending with a fresh edge so
    // the second boundary dispatches.
    //
    // Budget = 8 cycles -> 8 * 16 = 128 1/16-dot-units.
    //
    // Phase 1 (loop iter 1): dispatch defers (NMI fresh, 0 cycles),
    //   then NOP runs (2 cycles). state.count = 96 < 128. Continue.
    // Phase 2 (loop iter 2): dispatch fires (7 cycles). state.count
    //   = 96 + 336 = 432. 432 >= 128 -> EARLY EXIT.
    //
    // The follow-up NOP at $5000 must NOT have been executed.
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA; // NOP
    bus.mem[0xFFFA] = 0x00; // NMI vector LSB
    bus.mem[0xFFFB] = 0x50; // NMI vector MSB
    bus.mem[0x5000] = 0xEA; // follow-up NOP (must NOT execute)

    cpu.regs.irq_low = IrqSource::NMI.bits();
    cpu.nmi_fresh = true;

    let consumed = run(&mut cpu, &mut bus, 8 * 16);

    // Budget added first: count = 0 + 128 = 128.
    // Phase 1 consumed: defer (0) + NOP (2 cycles) → count -= 96 → 32.
    // Phase 2 dispatch consumed: 7 cycles → count -= 336 → -304 ≤ 0
    //   → EARLY EXIT. Follow-up NOP did NOT run.
    assert_eq!(
        consumed, 9,
        "consumed = defer+NOP (2 cycles) + dispatch (7 cycles); follow-up NOP suppressed"
    );
    assert_eq!(
        cpu.regs.pc, 0x5000,
        "PC must land at NMI vector $5000, NOT past the follow-up NOP at $5000"
    );
    assert_eq!(
        cpu.regs.count,
        128 - 432,
        "state.count = budget 128 - (NOP 96 + dispatch 336) = -304"
    );
}

#[test]
fn dispatch_does_not_suppress_followup_when_budget_is_ample() {
    // Same setup as the early-exit test above, but with a budget
    // large enough that the follow-up instruction fits comfortably.
    // Locks the regression case where the loop would under-shoot by
    // suppressing the follow-up when it shouldn't.
    let mut cpu = cpu_at(0x4000);
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA; // NOP
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA; // follow-up NOP (must execute here)
    // Pad $5001.. with NOPs so the loop doesn't run BRK / HLT / an
    // arbitrary opcode off the end of the budget run.
    bus.mem[0x5001..0x5080].fill(0xEA);

    cpu.regs.irq_low = IrqSource::NMI.bits();
    cpu.nmi_fresh = true;

    // Budget: 100 cycles -> 1600 1/16-units. Plenty for defer+NOP
    // (96) + dispatch+NOP (432) + many follow-up NOPs.
    let consumed = run(&mut cpu, &mut bus, 100 * 16);

    // The follow-up NOP at $5000 must have executed. In the early-
    // exit bug, PC would still be at $5000 (the NMI vector). With
    // the fix, PC advances past $5000 to at least $5001.
    assert!(
        cpu.regs.pc > 0x5000,
        "PC must advance past the follow-up NOP at $5000 (got ${:04X})",
        cpu.regs.pc
    );
    assert!(
        consumed > 9,
        "consumed must include the defer (0) + NOP (2) + dispatch (7) + at least the follow-up NOP (2) = 11+"
    );
}

// ===========================================================================
// 7. No-dispatch IRQ: I=1, EXTERNAL pending → returns base only, no +7
// ===========================================================================

#[test]
fn blocked_irq_returns_base_cycles_no_dispatch_cycles() {
    // Regression test for Phase 4.2 bug #2: before the fix, dispatch_irq
    // returned 7 unconditionally when irq_low had the EXTERNAL bit set,
    // even if I=1 blocked the actual dispatch. step() then took the
    // "irq_cycles != 0" branch and added 7 spurious cycles.
    let mut cpu = cpu_at(0x4000);
    cpu.regs.p = 0x24; // I=1, blocks maskable IRQ
    cpu.regs.irq_low = IrqSource::EXTERNAL.bits(); // IRQ pending
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA; // NOP
    let start_count = cpu.regs.count;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(
        c, 2,
        "NOP must return only its base 2 cycles; no spurious +7 from blocked IRQ"
    );
    assert_eq!(cpu.regs.count - start_count, -96);
    assert_eq!(cpu.regs.pc, 0x4001);
    assert_ne!(
        cpu.regs.irq_low & IrqSource::EXTERNAL.bits(),
        0,
        "EXTERNAL bit must remain pending",
    );
}

// ===========================================================================
// 8. Unofficial store/RMW cycle parity (Phase 7 preflight)
//
// SHX/SHY/AHX/TAS/LAS all use WRITE-mode addressing on the C++ side
// (`GetABIWR` / `GetIYWR` — `src/x6502.cpp:255-263, 312-325`), which
// never adds a page-cross cycle. The base CycTable value is the total
// cycle cost whether or not the effective address crosses a page.
// LAS (0xBB) is a read-modify-write (`RMW_ABY`, `src/x6502.cpp:334`)
// with the same write-mode addressing: base CycTable[0xBB] = 4, no
// extra even on a page-cross.
// ===========================================================================

#[test]
fn las_absy_page_cross_is_4_cycles() {
    // base $60FF, Y=1 → eff $6100 (page cross). CycTable[0xBB] = 4;
    // RMW write-mode adds no page-cross penalty and no write-back cost.
    let mut cpu = cpu_at(0x4000);
    cpu.regs.s = 0x0F;
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xBB;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x60; // abs = $60FF; eff = $6100
    bus.mem[0x6100] = 0xAA;
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 4, "LAS base 4, no page-cross extra (write-mode RMW)");
    assert_eq!(cpu.regs.count, -192, "count delta = -4 * 48");
    assert_eq!(cpu.regs.pc, 0x4003);
    assert_eq!(cpu.regs.a, 0x0A, "A = 0xAA & 0x0F");
}

#[test]
fn shx_absy_page_cross_is_5_cycles() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x42;
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9E;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x30; // abs = $30FF; eff = $3100 (page cross)
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 5, "SHX base 5, write-mode (no page-cross extra)");
    assert_eq!(cpu.regs.count, -240, "count delta = -5 * 48");
    assert_eq!(cpu.regs.pc, 0x4003);
}

#[test]
fn shy_absx_page_cross_is_5_cycles() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.x = 0x01;
    cpu.regs.y = 0xFF;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9C;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x40; // abs = $40FF; eff = $4100 (page cross)
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 5, "SHY base 5, write-mode (no page-cross extra)");
    assert_eq!(cpu.regs.count, -240, "count delta = -5 * 48");
    assert_eq!(cpu.regs.pc, 0x4003);
}

#[test]
fn tas_absy_page_cross_is_5_cycles() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x0F;
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9B;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x60; // abs = $60FF; eff = $6100 (page cross)
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 5, "TAS base 5, write-mode (no page-cross extra)");
    assert_eq!(cpu.regs.count, -240, "count delta = -5 * 48");
    assert_eq!(cpu.regs.pc, 0x4003);
}

#[test]
fn ahx_absy_page_cross_is_5_cycles() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x0F;
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x9F;
    bus.mem[0x4001] = 0xFF;
    bus.mem[0x4002] = 0x60; // abs = $60FF; eff = $6100 (page cross)
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 5, "AHX abs,Y base 5, write-mode (no page-cross extra)");
    assert_eq!(cpu.regs.count, -240, "count delta = -5 * 48");
    assert_eq!(cpu.regs.pc, 0x4003);
}

#[test]
fn ahx_indy_page_cross_is_6_cycles() {
    let mut cpu = cpu_at(0x4000);
    cpu.regs.a = 0xFF;
    cpu.regs.x = 0x0F;
    cpu.regs.y = 0x01;
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0x93;
    bus.mem[0x4001] = 0x00;
    bus.mem[0x0000] = 0xFF;
    bus.mem[0x0001] = 0x60; // ptr = $60FF; eff = $6100 (page cross)
    let c = step(&mut cpu, &mut bus);
    assert_eq!(c, 6, "AHX (ind),Y base 6, write-mode (no page-cross extra)");
    assert_eq!(cpu.regs.count, -288, "count delta = -6 * 48");
    assert_eq!(cpu.regs.pc, 0x4002);
}
