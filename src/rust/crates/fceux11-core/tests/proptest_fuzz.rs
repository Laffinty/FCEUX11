//! Property-based fuzz tests for the Rust 6502 CPU.
//!
//! Phase 4 sub-step 6 part 1 of `docs/plans/cpu-rust-v2.md`.
//!
//! Generates random CPU states + random ROM bytes and asserts the
//! invariants that must hold for ANY valid 6502 execution:
//!
//! 1. `step()` never panics and always returns a sane cycle count
//!    (<= 16 for a single instruction + dispatch).
//! 2. PC stays within $0000-$FFFF (u16 wrap).
//! 3. The stack pointer stays within $00-$FF.
//! 4. RESET / NMI bits in `irq_low` are consumed by dispatch.
//! 5. `run()` terminates within a bounded budget (no infinite loop).
//!
//! These are safety/robustness invariants — they do NOT check semantic
//! equivalence with the C++ CPU (that's the blargg / ctest gate).
//! Think of this as the "no crash, no hang, no out-of-range state"
//! fuzz fence.
//!
//! We use the lower-level `TestRunner::run` API rather than the
//! `proptest!` macro because the macro's named-argument grammar only
//! accepts `Arbitrary` types; custom strategies need explicit
//! `TestRunner` plumbing.

use fceux11_core::cpu::{
    run, step, Bus, CpuState, IrqSource,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};

#[derive(Clone)]
struct FlatBus {
    // Box keeps the 64 KiB array on the heap: proptest's nested
    // generation would otherwise exhaust the test thread's stack
    // (64 KiB * several in-flight frames > default 2 MiB stack).
    mem: Box<[u8; 0x10000]>,
}

// Debug shows only a summary (proptest prints the failing input via
// Debug; a full 4 KiB mem dump makes failures unreadable).
impl std::fmt::Debug for FlatBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FlatBus")
            .field("len", &self.mem.len())
            .field("first16", &&self.mem[0..16])
            .field("vector_fffa", &&self.mem[0xFFFA..0x10000])
            .finish()
    }
}

impl FlatBus {
    fn new() -> Self {
        Self {
            mem: Box::new([0; 0x10000]),
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

/// A random but valid CpuState. The stack pointer is forced to
/// $00-$FD so pushes stay on the stack page $0100-$01FF.
#[derive(Debug, Clone)]
struct RandomCpu {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    s: u8,
    p: u8,
    irq_low: u32,
    nmi_fresh: bool,
}

impl RandomCpu {
    fn to_state(&self) -> CpuState {
        let mut cpu = CpuState::new();
        cpu.regs.pc = self.pc;
        cpu.regs.a = self.a;
        cpu.regs.x = self.x;
        cpu.regs.y = self.y;
        cpu.regs.s = self.s;
        cpu.regs.p = self.p;
        cpu.regs.moo_pi = self.p;
        cpu.regs.irq_low = self.irq_low;
        cpu.nmi_fresh = self.nmi_fresh;
        cpu
    }
}

fn random_cpu() -> impl Strategy<Value = RandomCpu> {
    (
        any::<u16>(),  // pc
        any::<u8>(),   // a
        any::<u8>(),   // x
        any::<u8>(),   // y
        0x00u8..=0xFD, // s
        any::<u8>(),   // p
        any::<u32>(),  // irq_low
        any::<bool>(), // nmi_fresh
    )
        .prop_map(|(pc, a, x, y, s, p, irq_low, nmi_fresh)| RandomCpu {
            pc,
            a,
            x,
            y,
            s,
            p,
            irq_low,
            nmi_fresh,
        })
}

/// ROM filled with random bytes; vectors pre-seeded from the same
/// random slice so dispatch targets are well-defined.
///
/// Size is intentionally tiny (256 bytes into $FF00-$FFFF): proptest's
/// vec strategy + shrinker recurses deeply enough to blow the
/// test-thread stack for vecs larger than a few hundred elements on
/// Windows (4 KiB and 32 KiB both overflowed). 256 bytes still covers
/// every vector read ($FFFA-$FFFF) and keeps any instruction fetch in
/// a well-defined window; the fuzz intent is robustness of `step()`
/// against arbitrary opcodes, not ROM-size realism.
fn random_rom() -> impl Strategy<Value = FlatBus> {
    prop::collection::vec(any::<u8>(), 0x0100).prop_map(|data| {
        let mut bus = FlatBus::new();
        bus.mem[0xFF00..0x10000].copy_from_slice(&data[..]);
        // Vectors read from $FFFA-$FFFF (inside the populated window).
        bus.mem[0xFFFA] = data[0x00FA];
        bus.mem[0xFFFB] = data[0x00FB];
        bus.mem[0xFFFC] = data[0x00FC];
        bus.mem[0xFFFD] = data[0x00FD];
        bus.mem[0xFFFE] = data[0x00FE];
        bus.mem[0xFFFF] = data[0x00FF];
        bus
    })
}

#[test]
fn step_never_panics_and_returns_sane_cycles() {
    let mut runner = TestRunner::new(Config::with_cases(1024));
    let result = runner.run(&(random_cpu(), random_rom()), |(cpu_in, bus)| {
        let mut cpu = cpu_in.to_state();
        let mut bus = bus;
        let c = step(&mut cpu, &mut bus);
        prop_assert!(c <= 16, "step() returned {} cycles (max expected 16)", c);
        prop_assert!(cpu.regs.pc as u32 <= 0xFFFF, "PC out of range: {:04X}", cpu.regs.pc);
        Ok(())
    });
    assert!(result.is_ok(), "fuzz failure: {:?}", result);
}

#[test]
fn run_terminates_within_budget() {
    let mut runner = TestRunner::new(Config::with_cases(256));
    let result = runner.run(
        &(random_cpu(), random_rom(), 1i32..=4096),
        |(cpu_in, bus, cycles)| {
            let mut cpu = cpu_in.to_state();
            let mut bus = bus;
            // fceux11_cpu_run passes cycles * 16 as the 1/16-dot target.
            // The loop terminates when sum(step_cycles) >= cycles / 3
            // (each step adds step_cycles * 48 to count). Each step can
            // be up to ~16 cycles (7 dispatch + 8 instr + extras), so
            // the max consumed is ~(cycles/3 + 1) * 16 ≈ cycles * 5.3.
            // `cycles * 8` is a generous but still bounded ceiling that
            // catches a runaway / non-terminating loop while tolerating
            // NMI storms and the "at least one instruction per call"
            // invariant (C++ X6502_Run(1) also executes one instruction
            // even though 1 < its cost).
            let consumed = run(&mut cpu, &mut bus, cycles * 16);
            prop_assert!(
                consumed >= 0 && consumed <= cycles * 8,
                "run() consumed {} cycles for budget {} (unexpectedly large)",
                consumed,
                cycles
            );
            prop_assert!(cpu.regs.pc as u32 <= 0xFFFF);
            // s is u8; range is guaranteed by the type.
            Ok(())
        },
    );
    assert!(result.is_ok(), "fuzz failure: {:?}", result);
}

#[test]
fn reset_consumes_bit_and_loads_vector() {
    let mut runner = TestRunner::new(Config::with_cases(512));
    let result = runner.run(&(random_rom(), any::<u16>()), |(bus, reset_vector)| {
        let mut cpu = CpuState::new();
        cpu.regs.pc = 0x0000;
        cpu.regs.s = 0xFD;
        cpu.regs.p = 0xFF;
        cpu.regs.moo_pi = 0xFF;
        cpu.regs.irq_low = IrqSource::RESET.bits();
        cpu.nmi_fresh = false;
        let mut bus = bus;
        bus.mem[0xFFFC] = reset_vector as u8;
        bus.mem[0xFFFD] = (reset_vector >> 8) as u8;
        let _ = step(&mut cpu, &mut bus);
        prop_assert_eq!(
            cpu.regs.irq_low & IrqSource::RESET.bits(),
            0,
            "RESET bit not consumed"
        );
        prop_assert!(cpu.regs.pc as u32 <= 0xFFFF);
        Ok(())
    });
    assert!(result.is_ok(), "fuzz failure: {:?}", result);
}

#[test]
fn nmi_fresh_coalesces_and_fires_once() {
    let mut runner = TestRunner::new(Config::with_cases(512));
    let result = runner.run(&(random_cpu(), random_rom()), |(cpu_in, bus)| {
        let mut cpu = cpu_in.to_state();
        let mut bus = bus;
        // Overwrite irq_low (NOT OR): the random state may carry
        // RESET / NMI2 bits which outrank NMI in dispatch_irq priority
        // (RESET > NMI2 > NMI), so OR-ing NMI in would let a random
        // RESET/NMI2 win the first boundary and the defer assertion
        // below would be testing the wrong dispatch.
        cpu.regs.irq_low = IrqSource::NMI.bits();
        cpu.nmi_fresh = true;
        // First step: defer (clears nmi_fresh) + execute one instruction.
        let _ = step(&mut cpu, &mut bus);
        prop_assert!(!cpu.nmi_fresh, "nmi_fresh not cleared by defer");
        // Second step: dispatch NMI (clears NMI bit).
        let _ = step(&mut cpu, &mut bus);
        prop_assert_eq!(
            cpu.regs.irq_low & IrqSource::NMI.bits(),
            0,
            "NMI bit not consumed by dispatch"
        );
        prop_assert!(cpu.regs.pc as u32 <= 0xFFFF);
        Ok(())
    });
    assert!(result.is_ok(), "fuzz failure: {:?}", result);
}

/// Sanity check that the strategies produce valid buses.
#[test]
fn random_rom_strategy_produces_valid_bus() {
    let mut runner = TestRunner::new(Default::default());
    let result = runner.run(&random_rom(), |bus| {
        assert_eq!(bus.mem.len(), 0x10000);
        Ok(())
    });
    assert!(result.is_ok(), "strategy failure: {:?}", result);
}
