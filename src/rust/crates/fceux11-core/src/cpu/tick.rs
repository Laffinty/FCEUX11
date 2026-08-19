//! Opt-in per-instruction cycle tick bridge for mapper / DMC hooks.
//!
//! Phase 4 sub-step 6 part 2 of `docs/plans/cpu-rust-v2.md`.
//!
//! The C++ emulator drives mapper IRQs and DMC steal cycles through a
//! per-instruction hook installed via `g_cpu.set_map_irq_hook(fn)`:
//!
//! ```cpp
//! // X6502_RunDebug loop body (src/x6502.cpp:611-612):
//! temp = _tcount;   // cycles consumed this loop iteration
//! _tcount = 0;
//! if (const auto hook = g_cpu.map_irq_hook()) [[unlikely]] hook(temp);
//! ```
//!
//! `hook(temp)` is invoked once per loop iteration with the cycles
//! consumed that iteration (dispatch ADDCYC(7) + instruction
//! CycTable[b1], since `_tcount` accumulates both). The mapper uses it
//! to advance an internal counter (e.g. MMC3 scanline counter, VRC6
//! clock divider, DMC rate timer). Without this call the Rust CPU
//! under `FCEUX11_RUST_CPU=ON` never advances mapper clocks, which is
//! one source of the per-frame cycle-accounting drift seen in
//! `apu_wav_diff_test` / `golden_savestate_test` /
//! `savestate_regression_rust_smoke` (sub-step 5).
//!
//! This module exposes the same hook as an opt-in FFI callback:
//! `fceux11_cpu_set_tick(fn)` installs an `extern "C" fn(i32)` that
//! [`run_with_tick`] invokes once per executed instruction with the
//! instruction's cycle count (the `step()` return value, which matches
//! C++ `temp` — dispatch + instruction cycles). When no hook is
//! installed the caller should keep using [`run`] (no branch per
//! instruction); `run_with_tick` is only for the mapper-hook case.
//!
//! ## Memory-cycled vs instruction-cycled
//!
//! The C++ `hook(temp)` is instruction-granular (one call per loop
//! iteration, with the iteration's cycle count). It is NOT a
//! per-memory-access tick. A full per-access `tick()` (as in ced-nes
//! `cpu2.rs`) is a larger redesign that would thread a `tick`
//! parameter through every `Bus::read`/`write`; this module
//! deliberately starts with the instruction-granular bridge that
//! matches the existing C++ integration contract. If a future phase
//! needs per-access granularity (e.g. to model DMC steal cycles that
//! land mid-instruction), it can be added on top of this without
//! changing the FFI shape.

use crate::cpu::addressing::{Bus, CpuState};
use crate::cpu::execute::{dispatch_step, execute_step, run};

/// Signature of the installed tick callback. Mirrors the C++ mapper
/// hook: called once per executed instruction with the instruction's
/// total cycle count (base + extras + dispatch, the `step()` return
/// value). The mapper advances its internal counters from it.
pub type TickFn = extern "C" fn(cycles: i32);

/// Installed tick callback. `None` when no hook is installed (the
/// C++ side is driven by `g_cpu.set_map_irq_hook`, which registers
/// per-mapper).
///
/// Single-threaded, mirrors `g_cpu`'s `std::function`-based slot and
/// the rest of the FFI's `static mut` slots in `cpu/bus.rs`.
static mut TICK_FN: Option<TickFn> = None;

/// Install / replace the tick callback. Mirrors
/// `Cpu::set_map_irq_hook`. Pass the sentinel
/// [`fceux11_cpu_set_tick_null`] to disable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_set_tick(f: TickFn) {
    unsafe {
        TICK_FN = Some(f);
    }
}

/// Disable the tick callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_set_tick_null() {
    unsafe {
        TICK_FN = None;
    }
}

/// Internal: call the installed hook with `cycles`, if any.
#[inline]
fn tick_instruction(cycles: i32) {
    unsafe {
        if let Some(f) = TICK_FN {
            f(cycles);
        }
    }
}

/// Run the CPU for `cycles` 1/16-dot units, invoking the installed
/// tick callback once per iteration with the iteration's total cycle
/// count (dispatch cost + base + extras), matching the C++ `temp =
/// _tcount; hook(temp);` at `src/x6502.cpp:607-615`.
///
/// Mirrors `X6502_RunDebug`'s structure (`src/x6502.cpp:519-624`):
/// the loop first runs `dispatch_step`. If dispatch fires AND its
/// `7`-cycle cost exhausts the caller's budget, the loop exits
/// BEFORE running the follow-up instruction AND BEFORE firing the
/// tick — matching the C++ early-return at `src/x6502.cpp:586-588`.
/// This is the cycle-drift fix documented in
/// `docs/plans/phase4-dispatch-budget-fix-2026-08-XX.md`.
///
/// Semantically identical to [`run`] plus the mapper-hook bridge.
/// Kept separate so the plain [`run`] hot path has zero per-instruction
/// branch overhead when no mapper hook is active.
pub fn run_with_tick<B: Bus + ?Sized>(
    state: &mut CpuState,
    bus: &mut B,
    cycles: i32,
) -> i32 {
    let mut executed_cycles = 0i32;
    let target = state.regs.count.saturating_add(cycles);
    loop {
        // Phase 1: dispatch. If dispatch exhausted budget, no
        // instruction + no tick (C++ matches this).
        let dc = dispatch_step(state, bus);
        let dc_plus_ic: u8;
        if dc != 0 {
            executed_cycles = executed_cycles.saturating_add(dc as i32);
            if state.regs.jammed != 0 {
                break;
            }
            if state.regs.count >= target {
                // Dispatch exhausted the budget — no follow-up
                // instruction, no tick. This is the cycle-drift fix.
                break;
            }
            let ic = execute_step(state, bus, dc);
            executed_cycles = executed_cycles.saturating_add(ic as i32);
            dc_plus_ic = dc.saturating_add(ic);
        } else {
            let ic = execute_step(state, bus, 0);
            executed_cycles = executed_cycles.saturating_add(ic as i32);
            dc_plus_ic = ic;
        }
        // Single tick per iteration with the iteration's total cycle
        // count. Matches C++ `temp = _tcount; hook(temp);` at
        // `src/x6502.cpp:607-612`, where `_tcount` accumulates both
        // the dispatch's `ADDCYC(7)` and the opcode's `ADDCYC(CycTable)`
        // and is reset to 0 only AFTER the hook call.
        tick_instruction(dc_plus_ic as i32);
        if state.regs.jammed != 0 {
            break;
        }
        if state.regs.count >= target {
            break;
        }
    }
    executed_cycles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::state::IrqSource;
    use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

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

    static TICK_COUNT: AtomicUsize = AtomicUsize::new(0);
    static TICK_SUM: AtomicI32 = AtomicI32::new(0);
    // Serialize the tick tests: they share the static TICK_FN slot and
    // the atomic counters, so running them in parallel (Rust test
    // harness default) would cross-contaminate.
    static TICK_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    extern "C" fn counting_tick(cycles: i32) {
        TICK_COUNT.fetch_add(1, Ordering::SeqCst);
        TICK_SUM.fetch_add(cycles, Ordering::SeqCst);
    }

    fn cpu_at(pc: u16) -> CpuState {
        let mut cpu = CpuState::new();
        cpu.regs.pc = pc;
        cpu.regs.s = 0xFD;
        cpu.regs.p = 0x24; // I_FLAG | U_FLAG
        cpu.regs.moo_pi = cpu.regs.p;
        cpu.regs.irq_low = 0;
        cpu.nmi_fresh = false;
        cpu
    }

    #[test]
    fn tick_called_once_per_instruction_with_cycle_count() {
        let _guard = TICK_TEST_LOCK.lock().unwrap();
        TICK_COUNT.store(0, Ordering::SeqCst);
        TICK_SUM.store(0, Ordering::SeqCst);
        unsafe { fceux11_cpu_set_tick(counting_tick); }

        let mut cpu = cpu_at(0x4000);
        let mut bus = FlatBus::new();
        // NOP sled: each NOP is 2 cycles.
        bus.mem[0x4000..0x4080].fill(0xEA);

        // Budget: 32 cycles -> 32*16 = 512 1/16-units.
        // Loop terminates when sum(step_cycles) >= 32/3 ≈ 10.67,
        // i.e. after ~6 NOPs (6*2 = 12 >= 10.67).
        let consumed = run_with_tick(&mut cpu, &mut bus, 32 * 16);
        assert!(consumed >= 8, "at least ~4 instructions executed, got {}", consumed);

        // One tick per executed instruction; NOPs are 2 cycles, so
        // instruction count = consumed / 2.
        let expected_instructions = consumed / 2;
        assert_eq!(
            TICK_COUNT.load(Ordering::SeqCst) as i32,
            expected_instructions,
            "tick must be called once per executed instruction"
        );
        assert_eq!(
            TICK_SUM.load(Ordering::SeqCst),
            consumed,
            "sum of ticked cycles must equal total consumed"
        );

        unsafe { fceux11_cpu_set_tick_null(); }
    }

    #[test]
    fn no_hook_installed_is_noop() {
        let _guard = TICK_TEST_LOCK.lock().unwrap();
        unsafe { fceux11_cpu_set_tick_null(); }
        let mut cpu = cpu_at(0x4000);
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = 0xEA;
        // Should not panic and should behave like plain run().
        let consumed = run_with_tick(&mut cpu, &mut bus, 16);
        let mut cpu2 = cpu_at(0x4000);
        let mut bus2 = FlatBus::new();
        bus2.mem[0x4000] = 0xEA;
        let plain = run(&mut cpu2, &mut bus2, 16);
        assert_eq!(consumed, plain);
    }

    #[test]
    fn dispatch_cycles_are_included_in_tick() {
        let _guard = TICK_TEST_LOCK.lock().unwrap();
        TICK_COUNT.store(0, Ordering::SeqCst);
        TICK_SUM.store(0, Ordering::SeqCst);
        unsafe { fceux11_cpu_set_tick(counting_tick); }

        let mut cpu = cpu_at(0x4000);
        let mut bus = FlatBus::new();
        bus.mem[0x4000] = 0xEA;
        bus.mem[0xFFFA] = 0x00;
        bus.mem[0xFFFB] = 0x50;
        bus.mem[0x5000] = 0xEA;
        // NMI pending with fresh edge, budget = 8 cycles (128 1/16-units):
        //   Step 1: dispatch defers (0 cycles) + NOP runs (2 cycles),
        //           tick fired once with the iteration's 2-cycle total.
        //           state.count = 96. 96 < 128, continue.
        //   Step 2: dispatch fires (7 cycles), state.count = 432.
        //           432 >= 128 -> EARLY EXIT (mirrors C++
        //           `src/x6502.cpp:586-588`). No follow-up NOP, no tick.
        cpu.regs.irq_low = IrqSource::NMI.bits();
        cpu.nmi_fresh = true;

        let _ = run_with_tick(&mut cpu, &mut bus, 8 * 16);
        assert_eq!(
            TICK_COUNT.load(Ordering::SeqCst),
            1,
            "exactly one tick fires (defer+NOP iteration); dispatch exhausted budget -> no tick on iteration 2"
        );
        assert_eq!(
            TICK_SUM.load(Ordering::SeqCst),
            2,
            "tick sum = defer+NOP iteration's 2 cycles"
        );
        assert_eq!(
            cpu.regs.pc, 0x5000,
            "PC advanced to NMI vector $5000; follow-up NOP at $5000 NOT executed"
        );

        unsafe { fceux11_cpu_set_tick_null(); }
    }
}
