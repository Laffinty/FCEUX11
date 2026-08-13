//! CPU module — 6502 interpreter (Phase 1).
//!
//! Replicates the C++ `X6502`/`x6502.cpp` semantics **exactly**:
//! - `run_budget(cycles)` mirrors `X6502_Run` (decision A / ADR-008)
//! - `step()` performs interrupt sampling + fetch + dispatch per
//!   instruction, with `count` decremented by base cycles then by
//!   per-instruction extras (page-cross, RMW, taken branch).
//! - Undocumented opcodes are implemented (nestest-compatible).
//!
//! Internal field layout is NOT the savestate layout — that is
//! `regs::CpuRegsLayout` (64-byte `#[repr(C)]` mirror of `X6502`).
//! Conversion between the two happens in the savestate path (Phase 6).

pub mod addressing;
pub mod cycles;
pub mod decoder;
pub mod flags;
pub mod interrupt;
pub mod ops_arith;
pub mod ops_branch;
pub mod ops_flag;
pub mod ops_increment;
pub mod ops_load_store;
pub mod ops_shift;
pub mod ops_stack;
pub mod ops_system;
pub mod ops_transfer;
pub mod regs;

use crate::cpu::cycles::BASE_CYCLES;

/// Memory bus interface used by the CPU.
///
/// Private to this crate; all implementations are monomorphized (no
/// vtable) so the hot path stays inline-able. `VNesSoc` provides the
/// real implementation (Phase 2); tests provide mock buses.
pub trait BusContext {
    /// Read one byte from CPU address space.
    fn read(&mut self, addr: u16) -> u8;
    /// Write one byte to CPU address space.
    fn write(&mut self, addr: u16, val: u8);
    /// True if a DMA controller is stalling the CPU this cycle.
    fn dma_stalled(&self) -> bool;
}

/// Ricoh 2A03 CPU core (6502).
///
/// Phase 1: full instruction set + interrupts + budget-driven execution.
#[derive(Debug, Clone)]
pub struct CpuCore {
    /// Program counter.
    pub(crate) pc: u16,
    /// Accumulator.
    pub(crate) a: u8,
    /// X index.
    pub(crate) x: u8,
    /// Y index.
    pub(crate) y: u8,
    /// Stack pointer (0x0100 + S).
    pub(crate) s: u8,
    /// Processor status (P register).
    pub(crate) p: u8,
    /// Data-bus "cache" (open-bus behavior for read-modify-write).
    pub(crate) db: u8,
    /// mooPI: last pushed P (used by RTI to restore P).
    pub(crate) moo_pi: u8,
    /// Jammed (KIL) — CPU halted on illegal $02.
    pub(crate) jammed: bool,
    /// Pending interrupt sources (FCEU_IQ* bitmask).
    pub(crate) irq_pending: u32,
    /// Remaining cycle budget for current `run_budget`.
    pub(crate) count: i32,
    /// Cycles consumed since last IRQ hook (C++ `_tcount`).
    pub(crate) tcount: i32,
}

impl Default for CpuCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuCore {
    /// Power-on state: PC = 0, P = U|I, others 0 (matches `X6502_Power`).
    pub fn new() -> Self {
        Self {
            pc: 0,
            a: 0,
            x: 0,
            y: 0,
            s: 0,
            p: flags::INITIAL_P,
            db: 0,
            moo_pi: 0,
            jammed: false,
            irq_pending: 0,
            count: 0,
            tcount: 0,
        }
    }

    /// Reset: PC = reset vector, P = U|I. `S` is NOT touched here —
    /// this matches the C++ `X6502_Reset()` (it only sets
    /// `_IRQlow = FCEU_IQRESET`); the stack pointer is set to 0xFD at
    /// power-on by the caller, mirroring `X6502_Power()`.
    pub fn reset<BC: BusContext>(&mut self, bus: &mut BC) {
        self.pc = self.read16(bus, 0xFFFC);
        self.p = flags::INITIAL_P;
        self.moo_pi = flags::INITIAL_P;
        self.jammed = false;
        self.irq_pending = 0;
        self.tcount = 0;
        self.count = 0;
    }

    /// Execute until the cycle budget is exhausted — mirrors `X6502_Run`.
    ///
    /// Budget semantics: `count` accumulates `cycles`, then instructions
    /// run until `count <= 0`. Interrupt sampling happens at the start of
    /// each instruction (penultimate-cycle sampling), so an NMI/IRQ may
    /// push + vector in the middle of the budget (C++ `X6502_RunDebug`).
    ///
    /// Performance note: the hot loop is written so `count` stays in a
    /// register across instructions (LLVM keeps the `while` induction
    /// variable live); `step_inner` is `#[inline(always)]` and does the
    /// fetch + dispatch with `count` as a local, so the common case
    /// (no pending IRQ) compiles to a tight loop with no per-instruction
    /// function-call boundary.
    #[inline(always)]
    pub fn run_budget<BC: BusContext>(&mut self, cycles: i32, bus: &mut BC) {
        self.count += cycles;
        let mut remaining = self.count;
        while remaining > 0 {
            // DMA stall (Phase 4 wires real behavior; Phase 1 honors the
            // flag by pausing the CPU).
            if bus.dma_stalled() {
                break;
            }
            // Interrupt sampling (penultimate cycle). Fast path: no
            // pending sources → pure fetch+dispatch.
            if self.irq_pending == 0 {
                remaining = self.step_inner(remaining, bus);
                continue;
            }
            // Slow path: poll interrupts (may push + vector, consuming
            // cycles and possibly exhausting the budget).
            self.count = remaining;
            self.poll_interrupts(bus);
            remaining = self.count;
            if remaining <= 0 {
                break; // C++ returns here after IRQ handling.
            }
            remaining = self.step_inner(remaining, bus);
        }
        self.count = remaining;
    }

    /// Step ONE instruction, decrementing `remaining` by the
    /// instruction's cost. Returns `(new_remaining, tcount)` where
    /// `tcount` is the total cycle cost of this instruction (base +
    /// extras). The caller can use `tcount` to tick the APU per
    /// instruction (matching C++ `FCEU_SoundCPUHook(temp)`) and call
    /// the mapper IRQ hook with sub-segment granularity.
    ///
    /// **Phase 6 P2 shadow fix (2026-08-12)**: callers should now use
    /// this primitive in their own loop so they can hook the APU per
    /// instruction (the previous per-segment APU tick had sub-instruction
    /// timing drift in the frame counter events at 7457/14913/22371/
    /// 29828-30).
    pub fn step_one<BC: BusContext>(&mut self, mut remaining: i32, bus: &mut BC) -> (i32, i32) {
        if bus.dma_stalled() {
            return (remaining, 0);
        }
        // Snapshot P for the NEXT instruction's IRQ sample (C++ `_PI=_P`).
        self.moo_pi = self.p;

        // Fetch + advance PC.
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        // Base cycle cost.
        let base = BASE_CYCLES[opcode as usize] as i32;
        let prev_remaining = remaining;
        remaining -= base;
        self.count = remaining;
        // Dispatch (may decrement `remaining` for page-cross / RMW /
        // taken branch). Handlers write through `self.count` for the
        // per-instruction extras, so sync back after.
        decoder::execute(self, opcode, bus);
        let new_remaining = self.count;
        // tcount = base + extras (any `self.count -= 1` in the
        // handlers above is captured via `prev_remaining - new_remaining`).
        let tcount = prev_remaining - new_remaining;
        self.tcount = tcount;
        (new_remaining, tcount)
    }

    /// Fetch + dispatch one instruction, decrementing `remaining` by the
    /// instruction's cost. Assumes no pending IRQ (fast path).
    #[inline(always)]
    fn step_inner<BC: BusContext>(&mut self, mut remaining: i32, bus: &mut BC) -> i32 {
        // Snapshot P for the NEXT instruction's IRQ sample (C++ `_PI=_P`).
        self.moo_pi = self.p;

        // Fetch + advance PC.
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);

        // Base cycle cost.
        remaining -= BASE_CYCLES[opcode as usize] as i32;
        self.tcount = 0;

        // Dispatch (may decrement `remaining` for page-cross / RMW /
        // taken branch). Handlers write through `self.count` for the
        // per-instruction extras, so sync back after.
        self.count = remaining;
        decoder::execute(self, opcode, bus);
        self.count
    }

    /// Read 16-bit little-endian from the bus.
    #[inline(always)]
    pub(crate) fn read16<BC: BusContext>(&self, bus: &mut BC, addr: u16) -> u16 {
        let lo = bus.read(addr) as u16;
        let hi = bus.read(addr.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    // -----------------------------------------------------------------
    // Stack helpers (stack at 0x0100 + S).
    // -----------------------------------------------------------------

    #[inline(always)]
    pub(crate) fn push<BC: BusContext>(&mut self, bus: &mut BC, val: u8) {
        // 6502 PUSH: write at $100+S, THEN decrement S (C++
        // `WrRAM(0x100+_S, V); _S--;`). Writing first matters for
        // blargg instr_v5 set_test 5 ("PHA stores at $100+S").
        bus.write(0x0100 | self.s as u16, val);
        self.s = self.s.wrapping_sub(1);
    }

    #[inline(always)]
    pub(crate) fn pop<BC: BusContext>(&mut self, bus: &mut BC) -> u8 {
        // 6502 POP: increment S, THEN read at $100+S (C++
        // `RdRAM(0x100+(++_S))`).
        self.s = self.s.wrapping_add(1);
        bus.read(0x0100 | self.s as u16)
    }

    // -----------------------------------------------------------------
    // IRQ source bitmask helpers (match src/x6502.h FCEU_IQ*).
    // -----------------------------------------------------------------
    pub const IRQ_EXT: u32 = 0x001;
    pub const IRQ_EXT2: u32 = 0x002;
    pub const IRQ_RESET: u32 = 0x020;
    pub const IRQ_NMI2: u32 = 0x040;
    pub const IRQ_NMI: u32 = 0x080;
    pub const IRQ_DPCM: u32 = 0x100;
    pub const IRQ_FCOUNT: u32 = 0x200;
    pub const IRQ_TEMP: u32 = 0x800;

    /// Assert an IRQ source (edge-sensitive for NMI).
    #[inline(always)]
    pub fn irq_begin(&mut self, source: u32) {
        self.irq_pending |= source;
    }

    /// De-assert an IRQ source.
    #[inline(always)]
    pub fn irq_end(&mut self, source: u32) {
        self.irq_pending &= !source;
    }

    /// Trigger an NMI (edge-sensitive).
    #[inline(always)]
    pub fn trigger_nmi(&mut self) {
        self.irq_begin(Self::IRQ_NMI);
    }

    // -----------------------------------------------------------------
    // Register accessors (used by debugger / tests / savestate).
    // -----------------------------------------------------------------
    pub fn pc(&self) -> u16 { self.pc }
    pub fn a(&self) -> u8 { self.a }
    pub fn x(&self) -> u8 { self.x }
    pub fn y(&self) -> u8 { self.y }
    pub fn s(&self) -> u8 { self.s }
    pub fn p(&self) -> u8 { self.p }
    pub fn jammed(&self) -> bool { self.jammed }

    pub fn set_pc(&mut self, v: u16) { self.pc = v; }
    pub fn set_a(&mut self, v: u8) { self.a = v; }
    pub fn set_x(&mut self, v: u8) { self.x = v; }
    pub fn set_y(&mut self, v: u8) { self.y = v; }
    pub fn set_s(&mut self, v: u8) { self.s = v; }
    pub fn set_p(&mut self, v: u8) { self.p = v; }

    /// Snapshot of register state (for shadow-run diffing).
    pub fn snapshot(&self) -> CpuRegsSnapshot {
        CpuRegsSnapshot {
            pc: self.pc,
            a: self.a,
            x: self.x,
            y: self.y,
            s: self.s,
            p: self.p,
            db: self.db,
            jammed: self.jammed,
            tcount: self.tcount,
            count: self.count,
        }
    }

    /// Test-only: overwrite the full register file (debugger/unit tests).
    ///
    /// Exposed so integration tests can set up interrupt-sampling
    /// scenarios (e.g. asserting the penultimate-cycle sample uses the
    /// P from before the current instruction). Normal savestate restore
    /// uses `regs::CpuRegsLayout` (Phase 6).
    #[doc(hidden)]
    pub fn set_regs_for_test(&mut self, pc: u16, a: u8, x: u8, y: u8, s: u8, p: u8) {
        self.pc = pc;
        self.a = a;
        self.x = x;
        self.y = y;
        self.s = s;
        self.p = p;
        self.moo_pi = p;
    }
}

/// Lightweight register snapshot for tests / shadow-run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuRegsSnapshot {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub s: u8,
    pub p: u8,
    pub db: u8,
    pub jammed: bool,
    pub tcount: i32,
    pub count: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny RAM + ROM bus for unit tests.
    pub struct TestBus {
        pub ram: [u8; 0x800],
        pub prg: [u8; 0x8000], // 32 KiB at $8000-$FFFF
        pub stall: bool,
    }

    impl Default for TestBus {
        fn default() -> Self {
            Self {
                ram: [0; 0x800],
                prg: [0; 0x8000],
                stall: false,
            }
        }
    }

    impl TestBus {
        pub fn load(&mut self, addr: u16, bytes: &[u8]) {
            for (i, &b) in bytes.iter().enumerate() {
                let a = addr.wrapping_add(i as u16);
                if a < 0x800 {
                    self.ram[a as usize] = b;
                } else {
                    self.prg[(a - 0x8000) as usize] = b;
                }
            }
        }
    }

    impl BusContext for TestBus {
        fn read(&mut self, addr: u16) -> u8 {
            match addr {
                0x0000..=0x1FFF => self.ram[(addr & 0x07FF) as usize],
                0x8000..=0xFFFF => self.prg[(addr - 0x8000) as usize],
                _ => 0,
            }
        }
        fn write(&mut self, addr: u16, val: u8) {
            if addr < 0x800 {
                self.ram[addr as usize] = val;
            }
        }
        fn dma_stalled(&self) -> bool { self.stall }
    }

    #[test]
    fn new_state_matches_power_on() {
        let cpu = CpuCore::new();
        assert_eq!(cpu.p, flags::INITIAL_P);
        assert_eq!(cpu.pc, 0);
        assert_eq!(cpu.s, 0);
        assert!(!cpu.jammed);
        assert_eq!(cpu.irq_pending, 0);
    }

    #[test]
    fn reset_sets_pc_and_p_but_not_s() {
        let mut bus = TestBus::default();
        bus.load(0xFFFC, &[0x34, 0x12]);
        let mut cpu = CpuCore::new();
        cpu.s = 0x20; // pre-existing S must be preserved across reset
        cpu.reset(&mut bus);
        assert_eq!(cpu.pc, 0x1234);
        assert_eq!(cpu.s, 0x20); // C++ X6502_Reset does NOT touch S
        assert_eq!(cpu.p, flags::INITIAL_P);
    }

    #[test]
    fn push_pop_round_trip() {
        let mut bus = TestBus::default();
        let mut cpu = CpuCore::new();
        cpu.s = 0xFE;
        cpu.push(&mut bus, 0xAB);
        assert_eq!(cpu.s, 0xFD);
        // 6502 PUSH writes at $100+S *before* decrementing: with S=0xFE
        // the byte lands at $01FE (not $01FD). Maps to ram[0x1FE & 0x7FF].
        assert_eq!(bus.ram[0x1FE], 0xAB);
        assert_eq!(bus.read(0x01FE), 0xAB);
        let v = cpu.pop(&mut bus);
        assert_eq!(v, 0xAB);
        assert_eq!(cpu.s, 0xFE);
    }

    #[test]
    fn run_budget_nop_sled() {
        let mut bus = TestBus::default();
        // NOP sled starting at $8000.
        for i in 0..32 {
            bus.load(0x8000 + i as u16, &[0xEA]);
        }
        let mut cpu = CpuCore::new();
        cpu.pc = 0x8000;
        cpu.run_budget(64, &mut bus); // 64 cycles / 2 per NOP = 32 NOPs
        assert!(cpu.count <= 0);
        assert_eq!(cpu.pc, 0x8000 + 32);
    }

    #[test]
    fn dma_stall_halts_cpu() {
        let mut bus = TestBus::default();
        bus.stall = true;
        bus.load(0x8000, &[0xEA]);
        let mut cpu = CpuCore::new();
        cpu.pc = 0x8000;
        cpu.run_budget(64, &mut bus);
        assert_eq!(cpu.pc, 0x8000); // never advanced
    }
}
