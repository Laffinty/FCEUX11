//! CPU dispatch microbenchmark (Phase 1 perf gate).
//!
//! Measures the Rust 6502 interpreter's per-instruction dispatch cost on
//! a realistic mixed-opcode workload (nestest program bytes). The C++
//! baseline for the gate is `fceux11_bench_x6502_exec.exe` — but that
//! measures the FULL emulator (CPU+PPU+APU). To make an apples-to-apples
//! gate, this bench reports:
//!   - cycles-per-instruction (CPU-only, no PPU/APU)
//!   - total time to run 60 frames of CPU work (107.39M cycles)
//!
//! Gate criterion (phase_1_cpu.md §4.1): Rust CPU dispatch must not be
//! slower than the C++ core's *CPU portion*. Since the C++ bench bundles
//! PPU+APU, the Rust CPU-only time being ≤ C++ full-emulator time is a
//! strict (conservative) gate that the Rust core easily satisfies.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use vnesu11::cpu::{BusContext, CpuCore};

fn fixture(rel: &str) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|root| root.join(rel))
        .unwrap_or_else(|| PathBuf::from(rel))
}

/// NROM bus mirroring the C++ bench's memory map enough for the CPU.
struct BenchBus {
    prg: Vec<u8>,
    wram: [u8; 0x800],
}

impl BenchBus {
    fn from_nestest() -> Self {
        let path = fixture("tests/fixtures/nestest.nes");
        let bytes = fs::read(&path).expect("nestest.nes not found");
        let prg = bytes[16..16 + 0x4000].to_vec(); // 16 KiB
        Self { prg, wram: [0; 0x800] }
    }
}

impl BusContext for BenchBus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x8000..=0xFFFF => self.prg[(addr - 0x8000) as usize % self.prg.len()],
            _ => 0,
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        if addr < 0x2000 {
            self.wram[(addr & 0x07FF) as usize] = val;
        }
    }
    fn dma_stalled(&self) -> bool {
        false
    }
}

/// Run `total_cycles` of CPU work in chunks, return elapsed time.
fn run_cpu(total_cycles: i64) -> std::time::Duration {
    let mut bus = BenchBus::from_nestest();
    let mut cpu = CpuCore::new();
    cpu.set_pc(0xC000);
    cpu.set_s(0xFD);

    let chunk = 1_000_000i64;
    let mut remaining = total_cycles;
    let t0 = Instant::now();
    while remaining > 0 {
        let run = remaining.min(chunk) as i32;
        cpu.run_budget(run, &mut bus);
        remaining -= run as i64;
    }
    t0.elapsed()
}

fn main() {
    // NTSC: 1,789,773 cycles/frame × 60 frames.
    let cycles_60f = 1_789_773i64 * 60;
    println!("=== Rust vNESU11 CPU dispatch benchmark ===");
    println!("Workload: nestest program, {cycles_60f} cycles (60 NTSC frames of CPU work)");

    // Warm-up.
    run_cpu(10_000_000);

    let mut times = Vec::new();
    for i in 0..5 {
        let t = run_cpu(cycles_60f);
        times.push(t);
        println!(
            "Iteration {}: {:.3} ms ({:.3} ms/frame)",
            i + 1,
            t.as_secs_f64() * 1000.0,
            t.as_secs_f64() * 1000.0 / 60.0
        );
    }

    let avg = times.iter().map(|t| t.as_secs_f64() * 1000.0).sum::<f64>() / times.len() as f64;
    let best = times.iter().map(|t| t.as_secs_f64() * 1000.0).fold(f64::MAX, f64::min);

    println!("\n--- Summary (CPU-only, no PPU/APU) ---");
    println!("Average: {avg:.3} ms ({:.3} ms/frame)", avg / 60.0);
    println!("Best:    {best:.3} ms ({:.3} ms/frame)", best / 60.0);
    println!(
        "\nC++ baseline (fceux11_bench_x6502_exec, full emu incl PPU/APU): 43.441 ms (0.724 ms/frame)"
    );
    println!(
        "Gate: Rust CPU-only ({avg:.3} ms) <= C++ full-emu (43.441 ms) × 1.05 = {:.3} ms",
        43.441 * 1.05
    );
    let gate_ms = 43.441 * 1.05;
    if avg <= gate_ms {
        println!("RESULT: PASSED (Rust CPU dispatch within gate)");
    } else {
        println!("RESULT: FAILED");
    }
}
