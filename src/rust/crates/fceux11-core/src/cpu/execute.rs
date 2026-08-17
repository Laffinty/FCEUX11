//! Top-level instruction execution loop.
//!
//! Phase 1 goal (per `docs/plans/cpu-rust-v2.md` §4 Phase 1 gate):
//! `cargo build -p fceux11-core` succeeds and `fceux11_cpu_table_test`
//! walks all 256 opcodes without panicking.
//!
//! Phase 1 implements:
//! * `fetch()` — read the opcode byte and advance PC.
//! * `decode_addr()` — dispatch the addressing-mode function.
//! * `step()` — execute one instruction. For Phase 1, every opcode
//!   goes through a **stub** that just adds the base cycle count and
//!   advances PC by the instruction size. Real per-opcode semantics
//!   arrive in Phase 2.
//!
//! Phase 2 will swap the stubs in `phase1_stub_*` for the actual ALU /
//! load / store / branch / RMW implementations per the NESdev wiki.

use crate::cpu::addressing::{
    abs_x_read, abs_y_read, absolute, imm, implied, ind_x, ind_y_read, indirect, relative, zp,
    zpx, zpy, AddrMode, Bus, CpuState, ModeResult,
};
// Phase 2 will route write/RMW ops through abs_x_write / abs_y_write /
// ind_y_write; kept in scope so the dispatcher swap is a one-line
// change.
#[allow(unused_imports)]
use crate::cpu::addressing::{abs_x_write, abs_y_write, ind_y_write};
use crate::cpu::decode::{info, OpKind};
use crate::cpu::state::{Flags, IrqSource};

/// Per-cycle residual set by the dispatch helper. Mirrors
/// `_tcount` in the C++ code.
const CYCLES_PER_CPU_CYCLE: i32 = 16; // matches PAL ? 15 : 16; we hardcode NTSC for now.

/// Fetch one byte at PC and advance PC.
#[inline]
fn fetch<B: Bus + ?Sized>(s: &mut CpuState, bus: &mut B) -> u8 {
    let pc = s.regs.pc;
    let op = s.rd(bus, pc);
    s.regs.pc = pc.wrapping_add(1);
    op
}

/// Dispatch the addressing mode and return the result. Panics only on
/// the impossible case of an unknown variant — the table builder is
/// `const`, so the variant list is exhaustive.
fn decode_addr<B: Bus + ?Sized>(
    mode: AddrMode,
    s: &mut CpuState,
    bus: &mut B,
) -> ModeResult {
    match mode {
        AddrMode::Implied | AddrMode::Accum => implied(s, bus),
        AddrMode::Imm => imm(s, bus),
        AddrMode::ZP => zp(s, bus),
        AddrMode::ZPX => zpx(s, bus),
        AddrMode::ZPY => zpy(s, bus),
        AddrMode::Abs => absolute(s, bus),
        // Phase 1: default to the read variant for store ops. The real
        // Phase 2 dispatcher will choose read vs write based on OpKind.
        AddrMode::AbsX => abs_x_read(s, bus),
        AddrMode::AbsY => abs_y_read(s, bus),
        AddrMode::Rel => relative(s, bus),
        AddrMode::Ind => indirect(s, bus),
        AddrMode::IndX => ind_x(s, bus),
        AddrMode::IndY => ind_y_read(s, bus),
    }
}

/// Step one instruction. Returns the number of *CPU cycles* consumed
/// (i.e. `CycTable[op] / 1`; the internal 1/16-dot residual is left in
/// `state.regs.tcount` for the FCEUX-style sound hook).
///
/// Phase 1: stubs every opcode. Phase 2: full per-opcode logic.
pub fn step<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) -> u8 {
    // Mirror C++'s loop-top: _PI = _P at every instruction boundary so
    // IRQ-line sampling observes the *current* flag state.
    state.regs.moo_pi = state.regs.p;

    // IRQ / NMI / RESET dispatch — full implementation lands in Phase 3.
    // For Phase 1 we only consume the RESET bit so the CPU advances
    // cleanly into the reset vector.
    if state.regs.irq_low & IrqSource::RESET.bits() != 0 {
        let lo = state.rd(bus, 0xFFFC);
        let hi = state.rd(bus, 0xFFFD);
        state.regs.pc = ((hi as u16) << 8) | lo as u16;
        state.regs.jammed = 0;
        state.regs.p = Flags::RESET.bits();
        state.regs.moo_pi = state.regs.p;
        state.regs.irq_low &= !IrqSource::RESET.bits();
        state.regs.irq_low &= !IrqSource::TEMP.bits();
        // The reset vector load itself counts as 7 cycles per the C++ loop.
        state.regs.count = state.regs.count.saturating_add(7 * CYCLES_PER_CPU_CYCLE);
        return 7;
    }

    let opcode = fetch(state, bus);
    let op_info = info(opcode);

    let addr = decode_addr(op_info.mode, state, bus);

    // Phase 1 stub: do the bare minimum the gate requires (no panic,
    // correct cycle cost, correct PC advancement). Real per-opcode
    // semantics arrive in Phase 2.
    phase1_stub(state, bus, opcode, op_info.kind, addr);

    // Total cycle cost = base + addressing-mode extra.
    let cycles = op_info.base_cycles.saturating_add(addr.extra_cycles);
    state.regs.count = state
        .regs
        .count
        .saturating_add(cycles as i32 * CYCLES_PER_CPU_CYCLE);
    cycles
}

/// Phase 1 stub for every opcode. Only the cycle cost and PC advancement
/// are correct; registers / memory / flags are not modified. Phase 2
/// replaces this with per-opcode real logic.
fn phase1_stub<B: Bus + ?Sized>(
    state: &mut CpuState,
    _bus: &mut B,
    opcode: u8,
    kind: OpKind,
    _addr: ModeResult,
) {
    match kind {
        OpKind::Jam => {
            // KIL / STP halts the CPU until reset. Mirror C++ 0x02 handler.
            state.regs.jammed = 1;
            state.regs.pc = state.regs.pc.wrapping_sub(1);
        }
        OpKind::NopRead | OpKind::NopReadWrite | OpKind::AluA | OpKind::Load
        | OpKind::Store | OpKind::Rmw | OpKind::Unofficial | OpKind::Compare | OpKind::Bit => {
            // The addressing-mode helper has already advanced PC past
            // the operand. For Phase 1 we do nothing else.
            let _ = opcode;
        }
        OpKind::Register | OpKind::Flag | OpKind::Jump | OpKind::Branch => {
            // Phase 1: branch / jump target resolution is implemented
            // in Phase 2. For now we leave the addressing-mode helper's
            // (relative / immediate / etc.) side effect in place.
            let _ = opcode;
        }
    }
}

// ---------------------------------------------------------------------------
// Cycle-quantised run loop. Mirrors the C++ X6502_RunDebug top loop.
// Phase 1 only handles `cycles = 0` (no-op) and the simplest path. The
// full loop with IRQ dispatch / debug hooks lands in Phase 3.
// ---------------------------------------------------------------------------

/// Run the CPU until `cycles` 1/16-dot units of budget are consumed.
/// Returns the number of instructions executed.
pub fn run<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B, cycles: i32) -> i32 {
    let mut executed = 0i32;
    let target = state.regs.count.saturating_add(cycles);
    while state.regs.count < target {
        step(state, bus);
        executed += 1;
        if state.regs.jammed != 0 {
            // JAM halts the CPU — no more instructions advance until reset.
            break;
        }
    }
    executed
}

// The Phase 2 wiring for write/RMW addressing variants lives in
// `abs_x_write`, `abs_y_write`, `ind_y_write`. They're imported here so
// the wiring is ready when Phase 2 lands; see the dispatcher stub in
// `decode_addr` for the read/write selection logic.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::addressing::Bus;

    /// Trivial flat 64 KiB bus.
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
    fn power_sets_s_to_fd_and_resets_irqlow() {
        let mut s = CpuState::new();
        s.power();
        assert_eq!(s.regs.s, 0xFD);
        assert_ne!(s.regs.irq_low & IrqSource::RESET.bits(), 0);
        assert_eq!(s.regs.jammed, 0);
    }

    #[test]
    fn reset_consumes_reset_irq_and_loads_vector() {
        let mut s = CpuState::new();
        s.power();
        let mut bus = FlatBus::new();
        bus.mem[0xFFFC] = 0x00;
        bus.mem[0xFFFD] = 0xC0;
        // After power() s.regs.count is 0; we'll spend 7 cycles on reset.
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 7);
        assert_eq!(s.regs.pc, 0xC000);
        assert_eq!(s.regs.irq_low & IrqSource::RESET.bits(), 0);
        assert_eq!(s.regs.p & Flags::IRQ_DIS.bits(), Flags::IRQ_DIS.bits());
    }

    #[test]
    fn nop_consumes_two_cycles() {
        // NOP = $EA, base cycles = 2.
        let mut s = CpuState::new();
        let mut bus = FlatBus::new();
        bus.mem[0x0000] = 0xEA;
        s.regs.pc = 0;
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 2);
        assert_eq!(s.regs.pc, 1); // only the opcode byte
    }

    #[test]
    fn jam_halts_cpu() {
        // STP / KIL $02 — base cycles 2, PC rolls back one.
        let mut s = CpuState::new();
        let mut bus = FlatBus::new();
        bus.mem[0x0000] = 0x02;
        s.regs.pc = 0;
        let cycles = step(&mut s, &mut bus);
        assert_eq!(cycles, 2);
        assert_eq!(s.regs.jammed, 1);
        assert_eq!(s.regs.pc, 0);
        // A subsequent step does nothing (jam guards the loop in run()).
        let again = step(&mut s, &mut bus);
        assert_eq!(again, 2);
    }

    #[test]
    fn run_loop_returns_instruction_count() {
        let mut s = CpuState::new();
        let mut bus = FlatBus::new();
        for i in 0..16 {
            bus.mem[i] = 0xEA; // 16 × NOP
        }
        s.regs.pc = 0;
        // 16 NOPs × 2 cycles each, scaled by CYCLES_PER_CPU_CYCLE=16 to
        // match the C++ `_count = cycles * 16` model.
        let budget = 16 * 2 * 16;
        let n = run(&mut s, &mut bus, budget);
        assert_eq!(n, 16);
        assert_eq!(s.regs.pc, 16);
    }
}