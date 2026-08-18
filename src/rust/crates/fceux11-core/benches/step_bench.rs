//! Throughput microbenchmark for the Rust 6502 `step()` function.
//!
//! Phase 4 sub-step 6 part 3 of `docs/plans/cpu-rust-v2.md`.
//!
//! Drives `step()` in a tight loop on a minimal FlatBus and reports
//! instructions/second. Use `cargo bench -p fceux11-core` to run.
//!
//! Note: this is a *micro* benchmark — it does NOT include the bus
//! dispatch overhead, PPU frame setup, or DMC/mapper hooks that the
//! full emulator benchmark (`fceux11_bench_x6502_exec`) measures.
//! Use both together: the micro for the inner-loop ceiling, the
//! full for the realistic hot-path.

use fceux11_core::cpu::{step, Bus, CpuState, IrqSource};

struct FlatBus {
    mem: [u8; 0x10000],
}

impl FlatBus {
    fn new() -> Self {
        Self { mem: [0; 0x10000] }
    }
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

/// Standard 6502 "NOP sled": 256 bytes of $EA (NOP implied, 2 cycles
/// each). Running this measures the cheapest possible `step()` cost
/// (no memory access beyond the opcode fetch, no RMW, no dispatch).
fn bench_step_nop_sled(c: &mut criterion::Criterion) {
    let mut bus = FlatBus::new();
    bus.fill(0x4000, &[0xEA; 256]);
    let mut cpu = CpuState::new();
    cpu.regs.pc = 0x4000;
    cpu.regs.s = 0xFD;
    cpu.regs.p = 0x24; // I_FLAG | U_FLAG
    cpu.regs.moo_pi = cpu.regs.p;
    cpu.regs.irq_low = 0;
    cpu.nmi_fresh = false;

    c.bench_function("step/nop_sled/256", |b| {
        b.iter(|| {
            cpu.regs.pc = 0x4000;
            let _ = step(&mut cpu, &mut bus);
        });
    });
}

/// LDA abs ($AD, base 4 cycles) — exercises a memory read on every
/// instruction. Bus accesses: 1 opcode fetch + 1 operand read.
fn bench_step_lda_abs(c: &mut criterion::Criterion) {
    let mut bus = FlatBus::new();
    bus.fill(0x4000, &[0xAD; 256]);
    // Place a sentinel byte at every page-aligned address used by LDA abs.
    for i in 0..256u16 {
        bus.mem[(0x4000 + i * 4) as usize] = 0xAD; // opcode
        bus.mem[(0x4000 + i * 4 + 1) as usize] = 0x00; // addr lo
        bus.mem[(0x4000 + i * 4 + 2) as usize] = 0x80; // addr hi
        bus.mem[0x8000 + i as usize] = (i & 0xFF) as u8; // mem value
    }
    let mut cpu = CpuState::new();
    cpu.regs.pc = 0x4000;
    cpu.regs.s = 0xFD;
    cpu.regs.p = 0x24;
    cpu.regs.moo_pi = cpu.regs.p;
    cpu.regs.irq_low = 0;
    cpu.nmi_fresh = false;

    c.bench_function("step/lda_abs/256", |b| {
        b.iter(|| {
            cpu.regs.pc = 0x4000;
            let _ = step(&mut cpu, &mut bus);
        });
    });
}

/// Benchmark that includes IRQ dispatch overhead. Sets `irq_low =
/// FCEU_IQNMI` so every step() does NMI dispatch (push 3 bytes, jump
/// via $FFFA/$FFFB) before the follow-up instruction. This is the
/// "step with dispatch" ceiling.
fn bench_step_with_nmi_dispatch(c: &mut criterion::Criterion) {
    let mut bus = FlatBus::new();
    bus.fill(0x4000, &[0xEA; 256]);
    bus.mem[0xFFFA] = 0x00;
    bus.mem[0xFFFB] = 0x50;
    bus.fill(0x5000, &[0xEA; 64]); // NMI handler: 64 NOPs
    let mut cpu = CpuState::new();
    cpu.regs.pc = 0x4000;
    cpu.regs.s = 0xFD;
    cpu.regs.p = 0x24;
    cpu.regs.moo_pi = cpu.regs.p;
    cpu.regs.irq_low = IrqSource::NMI.bits();
    cpu.nmi_fresh = true;

    c.bench_function("step/with_nmi_dispatch", |b| {
        b.iter(|| {
            cpu.regs.pc = 0x4000;
            cpu.regs.irq_low = IrqSource::NMI.bits();
            cpu.nmi_fresh = true;
            let _ = step(&mut cpu, &mut bus);
        });
    });
}

criterion::criterion_group!(benches,
    bench_step_nop_sled,
    bench_step_lda_abs,
    bench_step_with_nmi_dispatch,
);
criterion::criterion_main!(benches);
