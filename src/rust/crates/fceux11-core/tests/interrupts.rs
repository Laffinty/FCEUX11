//! Unit tests for 6502 interrupt dispatch (IRQ / NMI / BRK / RESET).
//!
//! Phase 4 sub-step 2 of `docs/plans/cpu-rust-v2.md`. These tests pin
//! down the local CPU semantics that blargg `cpu_interrupts_v2` and
//! `cpu_timing_test6` exercise end-to-end, so that a single-opcode
//! dispatch bug surfaces as a one-line cargo failure instead of a
//! 10-minute ROM diff.
//!
//! ## Semantics under test (matches `X6502_RunDebug` in src/x6502.cpp)
//!
//! | Trigger      | Vector    | Cycles | Push P bits | Sets I | Cleared when |
//! | ------------ | --------- | ------ | ----------- | ------ | ------------ |
//! | RESET        | $FFFC/D   | 0*     | (none)      | yes    | dispatch     |
//! | NMI          | $FFFA/B   | 7      | U only      | yes    | dispatch     |
//! | IRQ (masked) | $FFFE/F   | 7      | U only      | yes    | dispatch     |
//! | BRK          | $FFFE/F   | 7      | U + B       | yes    | n/a (intrinsic)|
//!
//! *RESET does not consume any cycles itself; the cycle cost comes from
//! the instruction the CPU executes next (typically the JMP at the
//! reset vector).
//!
//! NMI additional: asserted with `nmi_fresh` so the dispatch is deferred
//! by one boundary (the latch was set at/after the current boundary;
//! 6502 samples the line at instruction end). `nmi_fresh` is cleared on
//! the defer; the NMI bit in `irq_low` stays set until the dispatch runs.

use fceux11_core::cpu::{
    step, Bus, CpuState, Flags, IrqSource,
};

// ---------------------------------------------------------------------------
// Test harness — local FlatBus (the one in cpu/bus.rs is #[cfg(test)]
// so it's only visible inside the crate, not from these integration
// tests).
// ---------------------------------------------------------------------------

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

/// Build a CpuState at a known PC and P (with `moo_pi = P` so the first
/// `step()` call's dispatch_irq sees a consistent view). S = $FD
/// (post-RESET hardware stack top), `irq_low` cleared, `nmi_fresh`
/// cleared.
fn cpu_at(pc: u16, p: u8) -> CpuState {
    let mut cpu = CpuState::new();
    cpu.regs.pc = pc;
    cpu.regs.s = 0xFD;
    cpu.regs.p = p;
    cpu.regs.moo_pi = p;
    cpu.regs.irq_low = 0;
    cpu.nmi_fresh = false;
    cpu
}

/// Simulate `fceux11_cpu_trigger_nmi`: assert the NMI line with a fresh
/// edge, so dispatch_irq will defer by one boundary.
fn trigger_nmi(cpu: &mut CpuState) {
    cpu.regs.irq_low |= IrqSource::NMI.bits();
    cpu.nmi_fresh = true;
}

// ===========================================================================
// Group A: RESET (4 tests)
// ===========================================================================

/// After `power()`, the first `step()` consumes the RESET bit (loads PC
/// from $FFFC/$FFFD) without charging any cycles itself, then executes
/// the instruction at the reset vector. P ends up with exactly I set
/// (`_PI=_P=I_FLAG` in the C++ reference) — U is cleared, not set.
///
/// This is the regression test for the Phase 4.1 cycle-count fix:
/// `dispatch_irq` must return 0 for RESET, not 7. The 7 cycles were
/// double-counted before, and a non-JMP opcode at the reset vector
/// would have blown up at `unreachable!()` in `step()`.
#[test]
fn reset_does_not_consume_cycles_then_executes_vector() {
    let mut cpu = CpuState::new();
    cpu.power();
    let mut bus = FlatBus::new();
    // Reset vector -> NOP at $C000.
    bus.mem[0xC000] = 0xEA; // NOP
    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0xC0;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cycles, 0 + 2, "RESET dispatch (0) + NOP (2)");
    assert_eq!(cpu.regs.pc, 0xC001);
    assert_eq!(
        cpu.regs.irq_low & IrqSource::RESET.bits(),
        0,
        "RESET bit must be cleared after dispatch",
    );
    assert_eq!(
        cpu.regs.p,
        Flags::IRQ_DIS.bits(),
        "P must be exactly I_FLAG after RESET (C++ `_PI=_P=I_FLAG`)",
    );
}

#[test]
fn reset_loads_pc_from_fffc_vector() {
    let mut cpu = CpuState::new();
    cpu.power();
    let mut bus = FlatBus::new();
    // Vector points to $BEEF, where LDA #imm sits.
    bus.mem[0xBEEF] = 0xA9;
    bus.mem[0xBEF0] = 0x42;
    bus.mem[0xFFFC] = 0xEF;
    bus.mem[0xFFFD] = 0xBE;
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cycles, 0 + 2);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(cpu.regs.pc, 0xBEF1);
}

/// After RESET, P must have I set and B clear. U is set per datasheet
/// (and per the Rust dispatch; the C++ is slightly different — it omits
/// U after RESET — but the difference doesn't matter because nothing
/// pushes P during a RESET handler).
#[test]
fn reset_sets_i_flag_and_clears_b() {
    let mut cpu = CpuState::new();
    cpu.regs.p = 0xFF; // start with all flags set, including B
    cpu.power();
    let mut bus = FlatBus::new();
    bus.mem[0xC000] = 0xEA; // NOP at the reset vector
    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0xC0;
    step(&mut cpu, &mut bus);
    assert_ne!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag must be set after RESET",
    );
    assert_eq!(
        cpu.regs.p & Flags::BREAK.bits(),
        0,
        "B flag must be clear after RESET",
    );
}

/// RESET is a one-shot: once dispatched, it must not re-dispatch at the
/// next instruction boundary. The CPU just runs the next sequential
/// instruction normally.
#[test]
fn reset_consumed_at_next_boundary_does_not_re_dispatch() {
    let mut cpu = CpuState::new();
    cpu.power();
    let mut bus = FlatBus::new();
    // Two NOPs after the reset vector.
    bus.mem[0xC000] = 0xEA;
    bus.mem[0xC001] = 0xEA;
    bus.mem[0xFFFC] = 0x00;
    bus.mem[0xFFFD] = 0xC0;
    step(&mut cpu, &mut bus); // dispatch + first NOP, PC = $C001
    assert_eq!(cpu.regs.pc, 0xC001);
    // Next step: no RESET dispatch; just executes the second NOP.
    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cycles, 2, "no extra dispatch cycles, just NOP");
    assert_eq!(cpu.regs.pc, 0xC002);
    assert_eq!(
        cpu.regs.irq_low & IrqSource::RESET.bits(),
        0,
        "RESET bit must remain clear",
    );
}

// ===========================================================================
// Group B: NMI (5 tests)
// ===========================================================================

/// NMI dispatch sequence:
///   trigger_nmi -> step()#1: defer (return 0) + execute current inst
///   step()#2: dispatch (7 cycles, push PCH/PCL/P|U, set I, jmp via $FFFA)
///             + execute instruction at NMI vector
#[test]
fn nmi_loads_pc_from_fffa_vector() {
    let mut cpu = cpu_at(0x4000, Flags::IRQ_DIS.bits() | Flags::UNUSED.bits());
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA; // NOP at the current PC (executed during defer)
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xA9; // LDA #$42 at the NMI vector
    bus.mem[0x5001] = 0x42;

    trigger_nmi(&mut cpu);
    // step()#1: defer + execute NOP at $4000
    assert_eq!(step(&mut cpu, &mut bus), 2, "defer + NOP");
    assert_eq!(cpu.regs.pc, 0x4001);
    assert!(!cpu.nmi_fresh, "nmi_fresh cleared after defer");
    assert_ne!(
        cpu.regs.irq_low & IrqSource::NMI.bits(),
        0,
        "NMI bit must remain set until dispatch",
    );

    // step()#2: dispatch + execute LDA #$42 at $5000
    assert_eq!(step(&mut cpu, &mut bus), 7 + 2, "NMI dispatch + LDA imm");
    assert_eq!(cpu.regs.pc, 0x5002);
    assert_eq!(cpu.regs.a, 0x42);
    assert_eq!(
        cpu.regs.irq_low & IrqSource::NMI.bits(),
        0,
        "NMI bit cleared after dispatch",
    );
}

/// NMI pushes PC and P with the B flag CLEAR (B is set only on BRK / PHP).
/// The push order is PCH, PCL, P|U (top of stack = PCH).
#[test]
fn nmi_pushes_pch_pcl_p_with_b_clear() {
    let mut cpu = cpu_at(0x4001, 0xEF); // N | V | U | D | I | Z | C (B is bit 4 = 0x10, not set in 0xEF)
    let mut bus = FlatBus::new();
    bus.mem[0x4001] = 0xEA;
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA;

    trigger_nmi(&mut cpu);
    step(&mut cpu, &mut bus); // defer + NOP at $4001
    step(&mut cpu, &mut bus); // dispatch + NOP at $5000

    // After dispatch, S was 0xFD; three pushes -> S = 0xFA.
    assert_eq!(cpu.regs.s, 0xFA);
    assert_eq!(bus.mem[0x01FD], 0x40, "PCH pushed first");
    assert_eq!(bus.mem[0x01FC], 0x02, "PCL pushed second (PC was $4002 post-NOP)");
    // Pushed P: per NMI dispatch in execute.rs, push value is
    // `(p | U) & ~B`. With p = 0xEF the push value is 0xEF (U already set).
    // The sibling test below covers the case where B was set in P.
    let pushed_p = bus.mem[0x01FB];
    assert_eq!(
        pushed_p & Flags::BREAK.bits(),
        0,
        "B flag must be clear in NMI-pushed P (got ${:02X})",
        pushed_p
    );
    assert_ne!(
        pushed_p & Flags::UNUSED.bits(),
        0,
        "U flag must be set in pushed P",
    );
}

/// Same scenario as above but starting with B explicitly set in P. After
/// the NMI dispatch, the pushed P must have B clear (NMI is not BRK).
#[test]
fn nmi_push_clears_b_flag_even_if_p_already_has_b() {
    let mut cpu = cpu_at(0x4001, 0xFF); // all 8 flags set, including B
    let mut bus = FlatBus::new();
    bus.mem[0x4001] = 0xEA;
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA;

    trigger_nmi(&mut cpu);
    step(&mut cpu, &mut bus); // defer + NOP at $4001
    step(&mut cpu, &mut bus); // dispatch + NOP at $5000

    let pushed_p = bus.mem[0x01FB];
    assert_eq!(
        pushed_p & Flags::BREAK.bits(),
        0,
        "NMI dispatch must clear B from pushed P (got ${:02X})",
        pushed_p
    );
}

/// NMI sets the I flag in P (the next maskable IRQ will be blocked).
#[test]
fn nmi_sets_i_flag_in_p() {
    let mut cpu = cpu_at(0x4000, 0x20); // U only, I clear
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA;
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA;

    trigger_nmi(&mut cpu);
    step(&mut cpu, &mut bus); // defer + NOP
    step(&mut cpu, &mut bus); // dispatch + NOP

    assert_ne!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag must be set after NMI dispatch",
    );
}

/// NMI is non-maskable: it fires even when I=1. The dispatch sets I=1
/// again (no change) and proceeds normally.
#[test]
fn nmi_unaffected_by_i_flag() {
    let mut cpu = cpu_at(0x4000, Flags::IRQ_DIS.bits() | Flags::UNUSED.bits()); // I=1, U=1
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA;
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA;

    trigger_nmi(&mut cpu);
    step(&mut cpu, &mut bus); // defer
    step(&mut cpu, &mut bus); // dispatch

    assert_eq!(cpu.regs.pc, 0x5001, "PC should be at NMI vector + 1");
    assert_eq!(
        cpu.regs.irq_low & IrqSource::NMI.bits(),
        0,
        "NMI bit cleared post-dispatch (sanity)",
    );
}

// ===========================================================================
// Group C: NMI edge detection (2 tests)
// ===========================================================================

/// Two consecutive `trigger_nmi` calls between instruction boundaries
/// coalesce into a single NMI dispatch — the second trigger is the same
/// edge as the first, not a fresh one.
#[test]
fn nmi_edge_two_consecutive_triggers_yield_one_dispatch() {
    let mut cpu = cpu_at(0x4000, Flags::IRQ_DIS.bits() | Flags::UNUSED.bits());
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA;
    bus.mem[0x4001] = 0xEA;
    bus.mem[0x5001] = 0xEA; // NOP after the NMI vector (otherwise the uninitialised
                             // 0x00 byte is BRK, which jumps PC to the IRQ vector)
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA;

    // Two back-to-back triggers before any step().
    trigger_nmi(&mut cpu);
    trigger_nmi(&mut cpu);
    assert!(cpu.nmi_fresh);

    step(&mut cpu, &mut bus); // #1: defer + NOP at $4000
    assert_eq!(cpu.regs.pc, 0x4001);
    assert!(!cpu.nmi_fresh);
    assert_ne!(cpu.regs.irq_low & IrqSource::NMI.bits(), 0);

    step(&mut cpu, &mut bus); // #2: dispatch + NOP at $5000
    assert_eq!(cpu.regs.pc, 0x5001, "exactly one NMI dispatched");
    assert_eq!(
        cpu.regs.irq_low & IrqSource::NMI.bits(),
        0,
        "NMI bit cleared after the single dispatch",
    );

    step(&mut cpu, &mut bus); // #3: no NMI, just NOP at $5001
    assert_eq!(cpu.regs.pc, 0x5002);
}

/// After a dispatch, a fresh trigger re-arms the edge: another NMI can
/// fire on a later boundary.
#[test]
fn nmi_edge_rearms_after_dispatch() {
    let mut cpu = cpu_at(0x4000, Flags::IRQ_DIS.bits() | Flags::UNUSED.bits());
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA;
    bus.mem[0x4001] = 0xEA;
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.mem[0x5000] = 0xEA;
    bus.mem[0x5001] = 0xEA;
    bus.mem[0x5002] = 0xEA; // NOP after the NMI vector for the second dispatch

    trigger_nmi(&mut cpu);
    step(&mut cpu, &mut bus); // #1: defer + NOP
    step(&mut cpu, &mut bus); // #2: dispatch + NOP at $5000
    assert_eq!(cpu.regs.pc, 0x5001);

    // Second NMI after the first has been serviced.
    trigger_nmi(&mut cpu);
    step(&mut cpu, &mut bus); // #3: defer + NOP at $5001
    assert_eq!(cpu.regs.pc, 0x5002);

    step(&mut cpu, &mut bus); // #4: dispatch + NOP at $5000 (jumped back)
    assert_eq!(cpu.regs.pc, 0x5001);
}

// ===========================================================================
// Group D: IRQ (3 tests)
// ===========================================================================

/// With I=1 (post-RESET, post-NMI, post-SEI), a pending maskable IRQ is
/// blocked at this boundary and the CPU just executes the next
/// instruction normally. `step()` must return only the instruction's
/// cycle cost, not 7 extra from a non-existent dispatch.
#[test]
fn irq_blocked_when_i_flag_set_does_not_dispatch() {
    let mut cpu = cpu_at(0x4000, Flags::IRQ_DIS.bits() | Flags::UNUSED.bits()); // I=1
    cpu.regs.irq_low = IrqSource::EXTERNAL.bits(); // IRQ pending
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA; // NOP
    bus.mem[0xFFFE] = 0x00;
    bus.mem[0xFFFE + 1] = 0x50;

    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(
        cycles, 2,
        "no spurious 7-cycle dispatch when I blocks IRQ (got {})",
        cycles
    );
    assert_eq!(cpu.regs.pc, 0x4001, "PC should advance normally");
    assert_ne!(
        cpu.regs.irq_low & IrqSource::EXTERNAL.bits(),
        0,
        "EXTERNAL bit must remain pending until cleared",
    );
    assert_ne!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag must remain set",
    );
}

/// After CLI clears the I flag, a pending maskable IRQ fires at the next
/// instruction boundary (deferred by one step to match the C++ semantics:
/// `_PI = _P` happens AFTER the dispatch check).
#[test]
fn irq_unmasked_after_cli_fires_at_next_boundary() {
    // Start with I=1, EXTERNAL pending.
    let mut cpu = cpu_at(0x4000, Flags::IRQ_DIS.bits() | Flags::UNUSED.bits()); // I=1
    cpu.regs.irq_low = IrqSource::EXTERNAL.bits();
    let mut bus = FlatBus::new();
    // $4000: CLI (0x58, 2 cycles) ; then a NOP at $4001
    bus.mem[0x4000] = 0x58;
    bus.mem[0x4001] = 0xEA;
    bus.mem[0xFFFE] = 0x00;
    bus.mem[0xFFFE + 1] = 0x50;
    bus.mem[0x5000] = 0xEA;

    // step()#1: CLI runs, I becomes 0 in P, but moo_pi is updated to
    // the pre-CLI P (which still has I=1) just after dispatch_irq.
    // The dispatch check used moo_pi = I=1, so IRQ is still blocked
    // this step. CLI itself takes 2 cycles.
    let cycles1 = step(&mut cpu, &mut bus);
    assert_eq!(cycles1, 2, "CLI only");
    assert_eq!(cpu.regs.pc, 0x4001);
    assert_eq!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag clear after CLI",
    );

    // step()#2: moo_pi is still the pre-CLI value (I=1) at the start of
    // this step — `moo_pi = P` happens AFTER `dispatch_irq` but BEFORE
    // the instruction at $4001 runs, so the dispatch check sees I=1
    // and the IRQ is still blocked. The NOP at $4001 executes normally.
    let cycles2 = step(&mut cpu, &mut bus);
    assert_eq!(cycles2, 2, "NOP at $4001 (moo_pi still has I=1)");
    assert_eq!(cpu.regs.pc, 0x4002);

    // step()#3: moo_pi was updated to P (I=0) at the END of step()#2,
    // so this boundary's dispatch sees the post-CLI I flag. The maskable
    // IRQ dispatches: pushes PC, sets I, jmps to $5000; then executes
    // the NOP there.
    let cycles3 = step(&mut cpu, &mut bus);
    assert_eq!(cycles3, 7 + 2, "IRQ dispatch (7) + NOP (2)");
    assert_eq!(cpu.regs.pc, 0x5001);
    assert_ne!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag set again by IRQ dispatch",
    );
}

/// The maskable IRQ loads PC from $FFFE/$FFFF (same vector as BRK).
/// Sets I in P (push order: PCH, PCL, P|U with B clear).
#[test]
fn irq_loads_pc_from_fffe_vector_and_sets_i() {
    let mut cpu = cpu_at(0x4000, Flags::UNUSED.bits()); // I clear, just U
    cpu.regs.irq_low = IrqSource::EXTERNAL.bits();
    let mut bus = FlatBus::new();
    bus.mem[0x4000] = 0xEA;
    bus.mem[0xFFFE] = 0x00;
    bus.mem[0xFFFE + 1] = 0x50;
    bus.mem[0x5000] = 0xA9; // LDA #$77
    bus.mem[0x5001] = 0x77;

    step(&mut cpu, &mut bus);
    assert_eq!(cpu.regs.pc, 0x5002);
    assert_eq!(cpu.regs.a, 0x77);
    assert_ne!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag must be set after IRQ dispatch",
    );
    assert_eq!(cpu.regs.s, 0xFA, "3 pushes -> S = $FD - 3");
    assert_eq!(bus.mem[0x01FD], 0x40, "PCH pushed first");
    // PCL pushed second. The dispatch happens BEFORE fetch in step(),
    // so the pushed PC is the pre-fetch value ($4000); the post-dispatch
    // fetch at $5000 then advances PC to $5002 after LDA #imm completes.
    assert_eq!(bus.mem[0x01FC], 0x00, "PCL pushed second (pre-fetch PC=$4000)");
    let pushed_p = bus.mem[0x01FB];
    assert_eq!(
        pushed_p & Flags::BREAK.bits(),
        0,
        "B flag must be clear in IRQ-pushed P",
    );
}

// ===========================================================================
// Group E: BRK (1 test)
// ===========================================================================

/// BRK is the software interrupt: opcode $00. Pushes PC and P with B
/// flag SET (distinguishing it from IRQ). Then loads PC from $FFFE/$FFFF
/// (same vector as IRQ). Sets I in P.
#[test]
fn brk_pushes_pc_and_p_with_b_set_and_loads_irq_vector() {
    let mut cpu = cpu_at(0x4000, Flags::UNUSED.bits()); // I clear, just U
    let mut bus = FlatBus::new();
    // $4000: BRK ($00, 7 cycles), $4001: BRK signature byte
    bus.mem[0x4000] = 0x00;
    bus.mem[0x4001] = 0xAA; // signature byte pushed by BRK after PCL
    bus.mem[0xFFFE] = 0x00;
    bus.mem[0xFFFE + 1] = 0x50;
    bus.mem[0x5000] = 0xEA;

    let cycles = step(&mut cpu, &mut bus);
    assert_eq!(cycles, 7, "BRK base cycle cost");
    // BRK itself is the instruction step() executes (no IRQ dispatch
    // path — BRK is a regular opcode $00). After BRK, PC = $5000 (the
    // IRQ vector). The instruction at $5000 (a NOP) is the NEXT
    // step()'s job.
    assert_eq!(cpu.regs.pc, 0x5000);
    assert_ne!(
        cpu.regs.p & Flags::IRQ_DIS.bits(),
        0,
        "I flag set after BRK",
    );
    assert_eq!(cpu.regs.s, 0xFA);

    // Stack order: PCH, PCL, P with B set. BRK pushes PC+2 (the byte
    // AFTER the signature byte): with PC=$4000, pushed PC = $4002.
    assert_eq!(bus.mem[0x01FD], 0x40, "PCH");
    assert_eq!(bus.mem[0x01FC], 0x02, "PCL (PC+2)");
    let pushed_p = bus.mem[0x01FB];
    assert_ne!(
        pushed_p & Flags::BREAK.bits(),
        0,
        "B flag MUST be set in BRK-pushed P (got ${:02X})",
        pushed_p
    );
    assert_ne!(
        pushed_p & Flags::UNUSED.bits(),
        0,
        "U flag set in pushed P",
    );

    // The next step() runs the IRQ-vector instruction (NOP), advancing
    // PC by 1. This verifies the refactored step() always executes
    // exactly one instruction per call, even after a JMP-class opcode.
    let cycles2 = step(&mut cpu, &mut bus);
    assert_eq!(cycles2, 2, "follow-up NOP at the IRQ vector");
    assert_eq!(cpu.regs.pc, 0x5001);
}