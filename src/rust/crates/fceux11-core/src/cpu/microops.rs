//! Batch 3c.2 increment i (v2.1.4 plan §5.1) — the unified grant model's
//! micro-op framework, first slice: ZP/Abs × Load/Store/Compare/Bit.
//!
//! # The grant model
//!
//! On hardware the 6502 performs exactly one bus access (or one internal
//! cycle) per CPU cycle, and the PPU advances 3 dots per CPU cycle with
//! phase-locked clocks. The unified grant model makes the emulator's
//! loop grant **exactly one CPU cycle per 3 PPU dots** (NTSC) and turns
//! each instruction into a *micro-op program* continued across grants —
//! the plan's §5.1 state-machine shape, authored from the 6502 datasheet
//! cycle sequences (no external emulator code referenced).
//!
//! The cadence falls out of the existing budget arithmetic with NO
//! interleave-loop restructuring: the per-dot loop adds 16 count units
//! per dot and every granted cycle charges 48 units, so after a grant
//! `count ∈ (−48, −32]` and the next grant lands exactly 3 dots later
//! (PAL: 15 units/dot → 3-4 dot spacing averaging 3.2, i.e. exactly
//! 5 cycles per 16 dots). A per-cycle proof: for a back-to-back
//! instruction stream the instruction START dots are identical under the
//! atomic model (full `base*48` charged at fetch) and the grant model
//! (48 charged per micro-op) — both equal `start + 3*Σbase` dots. Only
//! the *intra-instruction* access dots move: an access at cycle k lands
//! at `start + 3(k−1)` dots instead of `start`. That delta is exactly
//! the smear the plan's E1/E2/vbl_02 evidence attributes the CPU-PPU
//! phase jitter to.
//!
//! # Continuation state
//!
//! [`MicroCont`] (stored on `CpuState::micro`) is **runtime-only**: it is
//! not part of the 64-byte savestate blob. Frame-boundary integrity is
//! preserved by *draining* any pending continuation at the end of the
//! interleave loop's frame budget (`drain` / FFI
//! `fceux11_cpu_drain_micro_continuation`): the remaining micro-ops run
//! back-to-back with the PPU frozen, which reproduces the atomic model's
//! count-overdraw residual exactly and keeps "frame boundary ==
//! instruction boundary" true (savestates and per-frame Lua observers
//! never see mid-instruction state).
//!
//! # Per-instruction contract (C++ parity preserved)
//!
//! At micro-op 1 (the opcode fetch) the same prologue as
//! `execute_step`'s runs: `LAST_FETCH_BASE` publish, `tcount += base`,
//! `tick_post_body(base)` (the C++ `add_cycles(CycTable)` convention —
//! `timestamp_` is pre-charged with the instruction's full base BEFORE
//! the body runs, the R2-fix contract), `tick_pre_body` (the mapper/APU
//! per-instruction hook fires once per instruction at fetch, unchanged).
//! The only difference is the `count` charge: 48 (cycle 1) instead of
//! `base*48` — the remaining cycles charge 48 each as their micro-ops
//! are granted. After the instruction completes, the total charge is
//! identical to the atomic model.
//!
//! `count` therefore reflects "cycles actually consumed"
//! mid-instruction (the blob write-back via `sync_db_to_blob` mirrors
//! this per access) — the plan's "每微操作后回写等价值" item.
//!
//! # Scope (increment i)
//!
//! Only ZP/Abs × Load/Store/Compare/Bit (20 opcodes) go through micro-op
//! templates; every other opcode falls back to the atomic
//! `execute_step` body inside the same grant. Dispatch sequences, RMW,
//! branches, stack ops, DMA and the 3b write-landing deferral are later
//! increments (§5.1 increment table).
//!
//! # Flag
//!
//! Opt-in via `FCEUX11_UNIFIED_GRANT=1` (read by the root crate's
//! interleave loop, which calls [`set_micro_grant_enabled`]). Default
//! OFF = the 147-PASS blargg baseline, byte for byte. Mutually exclusive
//! with the increment-1 bus-access hook (`FCEUX11_BUS_ACCESS_HOOK`):
//! the hook's job (advancing the PPU mid-instruction) is what the grant
//! model does structurally, so the root loop does not install the hook
//! when the grant model is on.

use crate::cpu::addressing::AddrMode;
use crate::cpu::addressing::{Bus, CpuState};
use crate::cpu::alu::{bit, cmp, load_reg};
use crate::cpu::decode::{OpKind, info};
use crate::cpu::execute::{CYCLES_PER_CPU_CYCLE, fetch, load_reg_for_load, store_reg};

/// Count-budget units consumed by one granted CPU cycle. The per-dot
/// loop adds 16 units per dot (NTSC); one CPU cycle = 3 dots = 48 units.
/// Same math as `execute.rs`'s `dot(cycles) * 3` for `cycles == 1`.
const UNITS_PER_CPU_CYCLE: i32 = CYCLES_PER_CPU_CYCLE * 3;

/// Opt-in flag for the unified grant model. Default OFF (the atomic
/// per-instruction model, byte-identical to the v2.1.3 semantics).
static MICRO_GRANT_ENABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Enable / disable the unified grant model. Called by the root crate's
/// interleave loop from `FCEUX11_UNIFIED_GRANT`; pure-Rust tests call it
/// directly (serialize with [`MICRO_TEST_LOCK`]).
pub fn set_micro_grant_enabled(on: bool) {
    MICRO_GRANT_ENABLED.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the unified grant model is active.
#[inline]
pub fn micro_grant_enabled() -> bool {
    MICRO_GRANT_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

/// Serializes tests that toggle [`MICRO_GRANT_ENABLED`] (a process-global
/// static; cargo test runs tests in parallel). Test-only.
#[cfg(test)]
pub(crate) static MICRO_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

// ---------------------------------------------------------------------------
// Micro-op program state
// ---------------------------------------------------------------------------

/// Continuation state for an in-flight instruction (runtime-only; NOT in
/// the 64-byte savestate blob). One instruction at a time can be
/// in-flight; `None` means the CPU is at an instruction boundary.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct MicroCont {
    /// The executing instruction's opcode (fetched at micro-op 1).
    pub opcode: u8,
    /// Next micro-op phase (see the `PHASE_*` constants).
    pub phase: u8,
    /// Effective-address scratch (the template's PatchAddr slot): ZP
    /// holds the operand byte after phase 2; Abs holds lo after phase 2
    /// and `lo | hi<<8` after phase 3.
    pub eff: u16,
}

// Phase numbering = the cycle the micro-op executes in.
/// Cycle 2: fetch the operand byte at PC (ZP: the address; Abs: lo).
pub(crate) const PHASE_OPERAND_LO: u8 = 2;
/// Cycle 3 (Abs only): fetch the high operand byte at PC+1.
pub(crate) const PHASE_OPERAND_HI: u8 = 3;
/// Final cycle: the data access + semantic commit (load/store/compare/bit).
pub(crate) const PHASE_FINAL: u8 = 4;

/// The increment-i micro-op class: ZP / Abs × Load / Store / Compare / Bit.
/// 20 opcodes. Cycle sequences (6502 datasheet):
/// * ZP (3 cycles): fetch, operand, data access
/// * Abs (4 cycles): fetch, lo, hi, data access
/// None of these classes has extra cycles (no page-cross on unindexed
/// ZP/Abs, no RMW internals, no branch/stack internals).
pub(crate) const fn is_micro_eligible(opcode: u8) -> bool {
    matches!(
        opcode,
        // Load: LDA / LDX / LDY, zp + abs
        0xA5 | 0xAD | 0xA6 | 0xAE | 0xA4 | 0xAC
            // Store: STA / STX / STY, zp + abs
            | 0x85 | 0x8D | 0x86 | 0x8E | 0x84 | 0x8C
            // Compare: CMP / CPX / CPY, zp + abs
            | 0xC5 | 0xCD | 0xE4 | 0xEC | 0xC4 | 0xCC
            // Bit: BIT zp + abs
            | 0x24 | 0x2C
    )
}

/// True when an instruction continuation is pending (the CPU is
/// mid-instruction and the next grant must run [`micro_op_step`]).
#[inline]
pub(crate) fn has_pending(state: &CpuState) -> bool {
    state.micro.is_some()
}

// ---------------------------------------------------------------------------
// Grant entry points
// ---------------------------------------------------------------------------

/// Begin the instruction at PC as one grant: fetch the opcode, then
/// either set up the micro-op continuation (eligible class — the fetch
/// IS cycle 1) or run the atomic body (fallback class — exactly the
/// pre-existing `execute_step` behaviour).
///
/// Called by the run loops in place of `execute_step` when the grant
/// model is enabled. Returns the CPU cycles consumed by this grant
/// (1 for the micro path; the full instruction cost for the fallback).
pub(crate) fn instruction_begin<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) -> u8 {
    // Each instruction body is its own access phase (mirrors
    // execute_step / dispatch_step; the per-access hook is not installed
    // in grant mode, so this is bookkeeping only).
    state.cycle_in_phase = 0;
    let opcode = fetch(state, bus);
    if is_micro_eligible(opcode) {
        let base = info(opcode).base_cycles;
        // Publish the base cost for the 3b landing observers (same
        // contract as execute_step; the landing deferral is not combined
        // with grant mode in production).
        crate::cpu::execute::LAST_FETCH_BASE.store(base, std::sync::atomic::Ordering::Relaxed);
        // Charge cycle 1 only; cycles 2..N charge 48 each as granted.
        state.regs.count = state.regs.count.saturating_sub(UNITS_PER_CPU_CYCLE);
        // C++ ADDCYC(CycTable) convention, preserved (see module docs):
        // tcount ledger and the timestamp pre-charge both see the full
        // base at fetch, exactly like the atomic model.
        state.regs.tcount = state.regs.tcount.saturating_add(base as i32);
        crate::cpu::tick::tick_post_body(base as i32);
        crate::cpu::tick::tick_pre_body(state);
        state.micro = Some(MicroCont {
            opcode,
            phase: PHASE_OPERAND_LO,
            eff: 0,
        });
        state.cycles_in_run = state.cycles_in_run.saturating_add(1);
        1
    } else {
        execute_step_fetched(state, bus, opcode)
    }
}

/// Execute exactly ONE micro-op of the pending instruction continuation
/// (one granted CPU cycle: a bus access here; internal cycles arrive in
/// increment iii). Charges 48 count units.
///
/// Must only be called when [`has_pending`] is true (the run loops
/// guarantee it). Returns the cycles consumed by this grant (always 1).
pub(crate) fn micro_op_step<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) -> u8 {
    let cont = match state.micro {
        Some(c) => c,
        // Defensive: no pending continuation — nothing to do. The run
        // loops never call this in that state.
        None => return 0,
    };
    let mode = info(cont.opcode).mode;
    match cont.phase {
        PHASE_OPERAND_LO => {
            // Mirrors `zp` / `absolute`'s first operand read.
            let pc = state.regs.pc;
            let lo = state.rd(bus, pc);
            state.regs.pc = pc.wrapping_add(1);
            let next = match mode {
                AddrMode::ZP => Some((PHASE_FINAL, lo as u16)),
                AddrMode::Abs => Some((PHASE_OPERAND_HI, lo as u16)),
                _ => None,
            };
            match next {
                Some((phase, eff)) => {
                    state.micro = Some(MicroCont {
                        opcode: cont.opcode,
                        phase,
                        eff,
                    });
                }
                None => unreachable!("micro operand-lo with mode {:?}", mode),
            }
        }
        PHASE_OPERAND_HI => {
            // Mirrors `absolute`'s second operand read.
            debug_assert_eq!(mode, AddrMode::Abs);
            let pc = state.regs.pc;
            let hi = state.rd(bus, pc);
            state.regs.pc = pc.wrapping_add(1);
            let eff = ((hi as u16) << 8) | (cont.eff & 0x00FF);
            state.micro = Some(MicroCont {
                opcode: cont.opcode,
                phase: PHASE_FINAL,
                eff,
            });
        }
        PHASE_FINAL => {
            execute_final(state, bus, cont.opcode, cont.eff);
            state.micro = None;
        }
        _ => unreachable!("micro phase {}", cont.phase),
    }
    state.regs.count = state.regs.count.saturating_sub(UNITS_PER_CPU_CYCLE);
    state.cycles_in_run = state.cycles_in_run.saturating_add(1);
    1
}

/// The final-cycle data access + semantic commit. Reuses the existing
/// ALU helpers so the micro path is semantically IDENTICAL to the
/// atomic arms in `execute.rs` (same flag math, same DB latch via
/// `rd`/`wr`).
fn execute_final<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B, opcode: u8, eff: u16) {
    match info(opcode).kind {
        OpKind::Load => {
            let v = state.rd(bus, eff);
            load_reg(state, bus, load_reg_for_load(opcode), v);
        }
        OpKind::Store => {
            let v = store_reg(state, opcode);
            state.wr(bus, eff, v);
        }
        OpKind::Compare => {
            let m = state.rd(bus, eff);
            let r = match opcode {
                0xC5 | 0xCD => state.regs.a,
                0xE4 | 0xEC => state.regs.x,
                0xC4 | 0xCC => state.regs.y,
                _ => unreachable!("micro compare opcode ${:02X}", opcode),
            };
            cmp(state, bus, r, m);
        }
        OpKind::Bit => {
            let m = state.rd(bus, eff);
            bit(state, bus, m);
        }
        _ => unreachable!("micro final for non-eligible opcode ${:02X}", opcode),
    }
}

/// Run the remaining micro-ops of a pending continuation back-to-back
/// (no budget check, no PPU advancement). Called at the interleave
/// loop's frame end so a frame boundary is always an instruction
/// boundary — the count trajectory then matches the atomic model's
/// cross-frame overdraw residual exactly.
pub fn drain<B: Bus + ?Sized>(state: &mut CpuState, bus: &mut B) {
    while state.micro.is_some() {
        micro_op_step(state, bus);
    }
}

/// Atomic body for a fetched opcode (the pre-existing `execute_step`
/// prologue + body, minus the fetch itself). Used by
/// [`instruction_begin`] for non-eligible opcodes; the body itself is
/// shared with `execute_step` (see `execute::execute_body`).
pub(crate) fn execute_step_fetched<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    opcode: u8,
) -> u8 {
    let op_info = info(opcode);
    let base = op_info.base_cycles;
    crate::cpu::execute::LAST_FETCH_BASE.store(base, std::sync::atomic::Ordering::Relaxed);
    state.regs.count = state
        .regs
        .count
        .saturating_sub((base as i32) * UNITS_PER_CPU_CYCLE);
    state.regs.tcount = state.regs.tcount.saturating_add(base as i32);
    crate::cpu::tick::tick_post_body(base as i32);
    crate::cpu::tick::tick_pre_body(state);
    crate::cpu::execute::execute_body(state, bus, opcode, op_info, base)
}

// ---------------------------------------------------------------------------
// Tests. The per-dot harness simulates the production interleave loop's
// CPU side exactly: one `run_with_tick(state, bus, 16)` call per PPU
// dot (NTSC), no PPU, no NMI. Grants therefore land on dots 1, 4, 7, 10…
// (one grant every 3rd dot).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::execute::step;
    use crate::cpu::state::Flags;
    use crate::cpu::tick::run_with_tick;
    use std::sync::MutexGuard;

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

    /// Records one `(addr, is_write, dot)` triple per bus access, where
    /// `dot` is the 1-based per-dot call the access landed in.
    struct RecordingBus {
        mem: [u8; 0x10000],
        log: Vec<(u16, bool, u32)>,
        dot: u32,
    }
    impl RecordingBus {
        fn new() -> Self {
            Self {
                mem: [0; 0x10000],
                log: Vec::new(),
                dot: 0,
            }
        }
        fn accesses(&self) -> Vec<(u16, bool)> {
            self.log.iter().map(|&(a, w, _)| (a, w)).collect()
        }
    }
    impl Bus for RecordingBus {
        fn read(&mut self, addr: u16) -> u8 {
            let v = self.mem[addr as usize];
            self.log.push((addr, false, self.dot));
            v
        }
        fn write(&mut self, addr: u16, val: u8) {
            self.log.push((addr, true, self.dot));
            self.mem[addr as usize] = val;
        }
    }

    /// One per-dot CPU call — mirrors the interleave loop's
    /// `fceux11_cpu_run_ticks(cpu, ticks_per_dot)` with NTSC's 16.
    fn run_one_dot(state: &mut CpuState, bus: &mut RecordingBus) {
        bus.dot += 1;
        let _ = run_with_tick(state, bus, 16);
    }

    /// Run `n` per-dot calls.
    fn run_dots(state: &mut CpuState, bus: &mut RecordingBus, n: u32) {
        for _ in 0..n {
            run_one_dot(state, bus);
        }
    }

    fn cpu_at(pc: u16) -> CpuState {
        let mut c = CpuState::new();
        c.regs.pc = pc;
        c.regs.s = 0xFD;
        c.regs.p = Flags::IRQ_DIS.bits() | Flags::UNUSED.bits();
        c.regs.moo_pi = c.regs.p;
        c.regs.irq_low = 0;
        c.nmi_fresh = false;
        c
    }

    /// RAII guard: set the grant flag to a desired value for the guard's
    /// lifetime, then restore the previous value. Holds
    /// [`MICRO_TEST_LOCK`] so concurrent tests cannot observe or clobber
    /// each other's flag state (the static is process-global).
    struct FlagGuard(MutexGuard<'static, ()>, bool);
    impl FlagGuard {
        fn set(on: bool) -> Self {
            let guard = MICRO_TEST_LOCK.lock().unwrap();
            let prev = micro_grant_enabled();
            set_micro_grant_enabled(on);
            FlagGuard(guard, prev)
        }
    }
    impl Drop for FlagGuard {
        fn drop(&mut self) {
            set_micro_grant_enabled(self.1);
        }
    }

    // -----------------------------------------------------------------
    // Eligibility table
    // -----------------------------------------------------------------

    #[test]
    fn eligible_set_is_exactly_zp_abs_load_store_compare_bit() {
        const ELIGIBLE: [u8; 20] = [
            0xA5, 0xAD, 0xA6, 0xAE, 0xA4, 0xAC, // LDA/LDX/LDY
            0x85, 0x8D, 0x86, 0x8E, 0x84, 0x8C, // STA/STX/STY
            0xC5, 0xCD, 0xE4, 0xEC, 0xC4, 0xCC, // CMP/CPX/CPY
            0x24, 0x2C, // BIT
        ];
        for op in 0..=255u8 {
            let expect = ELIGIBLE.contains(&op);
            assert_eq!(
                is_micro_eligible(op),
                expect,
                "eligibility mismatch for ${:02X}",
                op
            );
        }
        for &op in &ELIGIBLE {
            let i = info(op);
            assert!(
                matches!(i.mode, AddrMode::ZP | AddrMode::Abs),
                "${:02X} mode",
                op
            );
            assert!(
                matches!(
                    i.kind,
                    OpKind::Load | OpKind::Store | OpKind::Compare | OpKind::Bit
                ),
                "${:02X} kind",
                op
            );
        }
    }

    // -----------------------------------------------------------------
    // Grant cadence — the core timing property of the whole batch.
    // NTSC grants land on dots 1, 4, 7, 10, … (every 3rd dot).
    // -----------------------------------------------------------------

    #[test]
    fn lda_abs_accesses_land_3_dots_apart() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        let mut bus = RecordingBus::new();
        // LDA $F000 (4 cycles): fetch, lo, hi, read.
        bus.mem[0x4000] = 0xAD;
        bus.mem[0x4001] = 0x00;
        bus.mem[0x4002] = 0xF0;
        bus.mem[0xF000] = 0x7F;

        // 12 dots: grants at 1,4,7,10 complete the instruction; dots
        // 11-12 add budget without granting (count ≤ 0 until dot 13).
        run_dots(&mut s, &mut bus, 12);

        // Cycle k must land on dot 3(k−1)+1.
        assert_eq!(
            bus.log,
            vec![
                (0x4000, false, 1),  // opcode fetch  (cycle 1)
                (0x4001, false, 4),  // operand lo    (cycle 2)
                (0x4002, false, 7),  // operand hi    (cycle 3)
                (0xF000, false, 10), // data read    (cycle 4)
            ]
        );
        assert_eq!(s.regs.a, 0x7F);
        assert_eq!(s.regs.pc, 0x4003);
        assert!(s.micro.is_none(), "instruction completed");
        assert_eq!(s.regs.p & Flags::NEGATIVE.bits(), 0);
        assert_eq!(s.regs.p & Flags::ZERO.bits(), 0);
        assert_eq!(s.regs.count, 0, "12 dots of budget exactly repaid");
    }

    #[test]
    fn sta_abs_write_lands_on_cycle_4_dot() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        s.regs.a = 0x5A;
        let mut bus = RecordingBus::new();
        // STA $2001 (4 cycles) — the $2001 rendering-write class from E1.
        bus.mem[0x4000] = 0x8D;
        bus.mem[0x4001] = 0x01;
        bus.mem[0x4002] = 0x20;

        run_dots(&mut s, &mut bus, 12);

        assert_eq!(
            bus.log,
            vec![
                (0x4000, false, 1),
                (0x4001, false, 4),
                (0x4002, false, 7),
                (0x2001, true, 10), // the WRITE lands at cycle 4 = dot 10
            ]
        );
        assert_eq!(bus.mem[0x2001], 0x5A);
    }

    #[test]
    fn lda_zp_completes_in_three_grants() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        let mut bus = RecordingBus::new();
        // LDA $10 (3 cycles): grants at dots 1, 4, 7.
        bus.mem[0x4000] = 0xA5;
        bus.mem[0x4001] = 0x10;
        bus.mem[0x0010] = 0x80;

        run_dots(&mut s, &mut bus, 9);

        assert_eq!(
            bus.log,
            vec![(0x4000, false, 1), (0x4001, false, 4), (0x0010, false, 7)]
        );
        assert_eq!(s.regs.a, 0x80);
        assert!(s.regs.p & Flags::NEGATIVE.bits() != 0);
        assert_eq!(s.regs.count, 0);
    }

    // -----------------------------------------------------------------
    // Semantic equivalence vs the atomic model, for every eligible
    // opcode: registers / flags / memory / PC / DB / access sequence
    // identical after the instruction completes, and the total count
    // charge identical (48 per granted cycle == base*48 for the whole
    // instruction).
    // -----------------------------------------------------------------

    /// Drive one instruction atomically (grant model OFF, `step`) and
    /// via per-dot grants (ON), compare everything observable. One flag
    /// guard is held across BOTH phases so concurrent tests never
    /// observe the intermediate OFF state.
    fn assert_equivalent(code: &[u8], data: &[(u16, u8)], setup: impl Fn(&mut CpuState)) {
        let _g = FlagGuard::set(false);
        // --- atomic reference ---
        let mut s_ref = cpu_at(0x4000);
        let mut b_ref = RecordingBus::new();
        for (i, &b) in code.iter().enumerate() {
            b_ref.mem[0x4000 + i] = b;
        }
        for &(a, v) in data {
            b_ref.mem[a as usize] = v;
        }
        setup(&mut s_ref);
        let cost = step(&mut s_ref, &mut b_ref) as i32;

        // --- grant model ---
        set_micro_grant_enabled(true);
        let mut s_mic = cpu_at(0x4000);
        let mut b_mic = RecordingBus::new();
        for (i, &b) in code.iter().enumerate() {
            b_mic.mem[0x4000 + i] = b;
        }
        for &(a, v) in data {
            b_mic.mem[a as usize] = v;
        }
        setup(&mut s_mic);
        // Exactly 3*cost dots: grants at 1,4,…,3(cost−1)+1 complete
        // the instruction; the trailing two dots add budget without
        // granting, so exactly ONE instruction executes on each side.
        // (The FlagGuard from the atomic phase is still held: concurrent
        // tests never observe the flag flip between the two phases.)
        run_dots(&mut s_mic, &mut b_mic, (3 * cost) as u32);
        drop(_g);
        assert!(s_mic.micro.is_none(), "continuation must be drained");

        // Observable state identical.
        assert_eq!(s_mic.regs.pc, s_ref.regs.pc, "pc {:02X}", code[0]);
        assert_eq!(s_mic.regs.a, s_ref.regs.a, "a {:02X}", code[0]);
        assert_eq!(s_mic.regs.x, s_ref.regs.x, "x {:02X}", code[0]);
        assert_eq!(s_mic.regs.y, s_ref.regs.y, "y {:02X}", code[0]);
        assert_eq!(s_mic.regs.s, s_ref.regs.s, "s {:02X}", code[0]);
        assert_eq!(s_mic.regs.p, s_ref.regs.p, "p {:02X}", code[0]);
        assert_eq!(s_mic.regs.db, s_ref.regs.db, "db {:02X}", code[0]);
        assert_eq!(b_mic.mem, b_ref.mem, "memory {:02X}", code[0]);
        assert_eq!(
            b_mic.accesses(),
            b_ref.accesses(),
            "access sequence {:02X}",
            code[0]
        );

        // Charge sanity: atomic charged cost*48 from count 0; the grant
        // model added 16 per dot and charged 48 per cycle — after exactly
        // 3*cost dots the residual is 16*3*cost − 48*cost = 0.
        assert_eq!(
            s_ref.regs.count,
            -(cost * 48),
            "atomic charge {:02X}",
            code[0]
        );
        assert_eq!(s_mic.regs.count, 0, "grant charge {:02X}", code[0]);
    }

    #[test]
    fn grant_model_is_semantically_equivalent_to_atomic_for_all_eligible_opcodes() {
        // Loads: value lands in the register, flags from the value.
        assert_equivalent(&[0xA5, 0x10], &[(0x0010, 0x00)], |_| {}); // LDA zp → Z set
        assert_equivalent(&[0xAD, 0x00, 0xF0], &[(0xF000, 0x80)], |_| {}); // LDA abs → N set
        assert_equivalent(&[0xA6, 0x20], &[(0x0020, 0x7F)], |_| {}); // LDX zp
        assert_equivalent(&[0xAE, 0x00, 0xF0], &[(0xF000, 0x01)], |_| {}); // LDX abs
        assert_equivalent(&[0xA4, 0x30], &[(0x0030, 0xFF)], |_| {}); // LDY zp
        assert_equivalent(&[0xAC, 0x00, 0xF0], &[(0xF000, 0x42)], |_| {}); // LDY abs
        // Stores: register value lands in memory (incl. PPU-range addrs).
        assert_equivalent(&[0x85, 0x40], &[], |s: &mut CpuState| s.regs.a = 0x11);
        assert_equivalent(&[0x8D, 0x01, 0x20], &[], |s: &mut CpuState| s.regs.a = 0x1E);
        assert_equivalent(&[0x86, 0x41], &[], |s: &mut CpuState| s.regs.x = 0x22);
        assert_equivalent(&[0x8E, 0x02, 0x20], &[], |s: &mut CpuState| s.regs.x = 0x08);
        assert_equivalent(&[0x84, 0x42], &[], |s: &mut CpuState| s.regs.y = 0x33);
        assert_equivalent(&[0x8C, 0x03, 0x20], &[], |s: &mut CpuState| s.regs.y = 0x78);
        // Compares: NZC from reg − mem (equal / less / greater).
        assert_equivalent(&[0xC5, 0x10], &[(0x0010, 0x42)], |s: &mut CpuState| {
            s.regs.a = 0x42
        });
        assert_equivalent(
            &[0xCD, 0x00, 0xF0],
            &[(0xF000, 0x80)],
            |s: &mut CpuState| s.regs.a = 0x7F,
        );
        assert_equivalent(&[0xE4, 0x20], &[(0x0020, 0x10)], |s: &mut CpuState| {
            s.regs.x = 0x0F
        });
        assert_equivalent(
            &[0xEC, 0x00, 0xF0],
            &[(0xF000, 0x01)],
            |s: &mut CpuState| s.regs.x = 0x01,
        );
        assert_equivalent(&[0xC4, 0x30], &[(0x0030, 0xFF)], |s: &mut CpuState| {
            s.regs.y = 0x00
        });
        assert_equivalent(
            &[0xCC, 0x00, 0xF0],
            &[(0xF000, 0x40)],
            |s: &mut CpuState| s.regs.y = 0x40,
        );
        // BIT: N/V from memory, Z from A&M.
        assert_equivalent(&[0x24, 0x10], &[(0x0010, 0xC0)], |s: &mut CpuState| {
            s.regs.a = 0x40
        });
        assert_equivalent(
            &[0x2C, 0x00, 0xF0],
            &[(0xF000, 0x00)],
            |s: &mut CpuState| s.regs.a = 0xFF,
        );
    }

    // -----------------------------------------------------------------
    // Fallback: ineligible opcodes still execute atomically inside one
    // per-dot call under grant mode.
    // -----------------------------------------------------------------

    #[test]
    fn ineligible_opcodes_fall_back_to_atomic() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        s.regs.x = 0x03;
        let mut bus = RecordingBus::new();
        // LDA $10,X (B5, ZPX — increment ii scope).
        bus.mem[0x4000] = 0xB5;
        bus.mem[0x4001] = 0x10;
        bus.mem[0x0013] = 0x99;

        run_one_dot(&mut s, &mut bus);
        // All accesses landed in dot 1 (atomic body, single grant).
        assert_eq!(
            bus.accesses(),
            vec![(0x4000, false), (0x4001, false), (0x0013, false)]
        );
        assert!(bus.log.iter().all(|&(_, _, d)| d == 1));
        assert_eq!(s.regs.a, 0x99);
        assert!(s.micro.is_none());
    }

    // -----------------------------------------------------------------
    // Drain: a mid-instruction continuation finishes back-to-back and
    // the count trajectory matches the atomic overdraw.
    // -----------------------------------------------------------------

    #[test]
    fn drain_completes_pending_continuation() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        s.regs.a = 0x66;
        let mut bus = RecordingBus::new();
        bus.mem[0x4000] = 0x8D; // STA $2002
        bus.mem[0x4001] = 0x02;
        bus.mem[0x4002] = 0x20;

        // Exactly one dot: fetch only, continuation pending.
        run_one_dot(&mut s, &mut bus);
        assert!(s.micro.is_some());
        assert_eq!(bus.accesses(), vec![(0x4000, false)]);

        // Drain (frame-end semantics): remaining cycles run with no
        // further dots — capture them in a fresh log.
        let pre_drain_mem = bus.mem;
        let mut drained = RecordingBus {
            mem: pre_drain_mem,
            log: Vec::new(),
            dot: 0,
        };
        drain(&mut s, &mut drained);
        assert!(s.micro.is_none());
        assert_eq!(
            drained.accesses(),
            vec![(0x4001, false), (0x4002, false), (0x2002, true)]
        );
        assert_eq!(drained.mem[0x2002], 0x66);
        // Count: the fetch dot added 16 and charged 48 (−32); the drain
        // charged 3×48 more → 16 − 4*48 = −176, exactly the atomic
        // model's post-instruction overdraw for a 16-unit budget.
        assert_eq!(s.regs.count, 16 - 4 * 48);
    }

    // -----------------------------------------------------------------
    // Timestamp / mapper-hook convention (R2): a mid-instruction store
    // observes `timestamp` already pre-charged with the full base at
    // fetch — identical to the atomic model.
    // -----------------------------------------------------------------

    #[test]
    fn timestamp_convention_preserved_under_grant_model() {
        use crate::cpu::tick::{
            TICK_SLOT_LOCK, fceux11_cpu_set_tick_cycles, fceux11_cpu_set_tick_null,
        };
        use std::sync::atomic::{AtomicI32, Ordering};

        let _g = FlagGuard::set(true);
        let _tick_guard = TICK_SLOT_LOCK.lock().unwrap();
        static TICKED: AtomicI32 = AtomicI32::new(0);
        TICKED.store(0, Ordering::SeqCst);
        extern "C" fn counting_tick_cycles(cycles: i32) {
            TICKED.fetch_add(cycles, Ordering::SeqCst);
        }
        unsafe { fceux11_cpu_set_tick_cycles(counting_tick_cycles) };

        let mut s = cpu_at(0x4000);
        s.regs.a = 0x2A;
        let mut bus = RecordingBus::new();
        bus.mem[0x4000] = 0x8D; // STA $2001, 4 cycles
        bus.mem[0x4001] = 0x01;
        bus.mem[0x4002] = 0x20;

        // Dots 1-7: grants at 1 (fetch), 4 (lo), 7 (hi). The write has
        // NOT happened yet; the timestamp was pre-charged with base=4
        // at the fetch grant.
        run_dots(&mut s, &mut bus, 7);
        assert_eq!(TICKED.load(Ordering::SeqCst), 4, "fetch pre-charged base=4");
        assert_eq!(bus.accesses().len(), 3, "write not yet issued");

        // Dots 8-10: the write grant lands at dot 10; no further
        // timestamp charge (eligible classes have no extras).
        run_dots(&mut s, &mut bus, 3);
        assert_eq!(
            TICKED.load(Ordering::SeqCst),
            4,
            "no extra timestamp charge"
        );
        assert_eq!(bus.log.last(), Some(&(0x2001, true, 10u32)));

        unsafe { fceux11_cpu_set_tick_null() };
    }

    // -----------------------------------------------------------------
    // A grant with a large budget (the E1 NMI-freeze shape: 8 cycles =
    // 128 units) runs multiple micro-ops in one call.
    // -----------------------------------------------------------------

    #[test]
    fn large_budget_grant_runs_multiple_micro_ops() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        let mut bus = RecordingBus::new();
        bus.mem[0x4000] = 0xAD; // LDA $F000 (4 cycles)
        bus.mem[0x4001] = 0x00;
        bus.mem[0x4002] = 0xF0;
        bus.mem[0xF000] = 0x3C;

        // 128 units = 8 CPU cycles: fetch(48) + lo(48) + hi(48) fit;
        // the read is one grant short (count hits ≤ 0 after 3 cycles).
        let _ = run_with_tick(&mut s, &mut bus, 128);
        assert_eq!(bus.accesses().len(), 3);
        assert!(s.micro.is_some(), "read still pending");

        // Two dots later the budget renews and the read grants.
        run_dots(&mut s, &mut bus, 2);
        assert_eq!(bus.accesses().len(), 4);
        assert_eq!(s.regs.a, 0x3C);
        assert!(s.micro.is_none());
    }

    // -----------------------------------------------------------------
    // Flag OFF: zero behavior change (the continuation never engages).
    // -----------------------------------------------------------------

    #[test]
    fn flag_off_matches_plain_step() {
        // Force OFF under the test lock: concurrent tests toggle the
        // process-global flag, and this test pins the OFF behaviour.
        let _g = FlagGuard::set(false);
        let mut s = cpu_at(0x4000);
        let mut bus = RecordingBus::new();
        bus.mem[0x4000] = 0xAD;
        bus.mem[0x4001] = 0x00;
        bus.mem[0x4002] = 0xF0;
        bus.mem[0xF000] = 0x21;
        // Flag is OFF by default; a single run_with_tick call executes
        // the whole instruction atomically.
        bus.dot += 1;
        let consumed = run_with_tick(&mut s, &mut bus, 16);
        assert_eq!(consumed, 4);
        assert_eq!(bus.accesses().len(), 4);
        assert!(bus.log.iter().all(|&(_, _, d)| d == 1), "atomic: one dot");
        assert_eq!(s.regs.a, 0x21);
        assert!(s.micro.is_none());
    }

    // -----------------------------------------------------------------
    // FlatBus sanity: the grant path against a plain bus too (compile
    // surface — drain/micro ops are generic over Bus).
    // -----------------------------------------------------------------

    #[test]
    fn grant_model_works_over_generic_bus() {
        let _g = FlagGuard::set(true);
        let mut s = cpu_at(0x4000);
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = 0xA9; // LDA #imm — NOT eligible (increment ii)
        bus.mem[0x4001] = 0x55;
        bus.mem[0x4002] = 0xAD; // LDA abs — eligible
        bus.mem[0x4003] = 0x00;
        bus.mem[0x4004] = 0xF0;
        bus.mem[0xF000] = 0x10;

        // Dot 1: the ineligible imm executes atomically even in grant
        // mode (charges 2*48; the next grant needs 6 dots of budget).
        let _ = run_with_tick(&mut s, &mut bus, 16);
        assert_eq!(s.regs.a, 0x55);
        assert!(s.micro.is_none());
        // Dots 2-16: LDA abs grants at dots 7 (fetch), 10 (lo),
        // 13 (hi), 16 (read).
        for _ in 0..15 {
            let _ = run_with_tick(&mut s, &mut bus, 16);
        }
        assert_eq!(s.regs.a, 0x10);
        assert!(s.micro.is_none());
    }
}
