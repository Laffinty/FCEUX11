//! Best-of-N CPU dispatch measurement (reduces turbo/noise variance).
//! Reports best / median / worst for the nestest 60-frame CPU workload.
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use vnesu11::cpu::{BusContext, CpuCore};

fn fixture(rel: &str) -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).and_then(|p| p.parent())
        .map(|root| root.join(rel)).unwrap_or_else(|| PathBuf::from(rel))
}

struct Bus { prg: Vec<u8>, wram: [u8; 0x800] }
impl BusContext for Bus {
    #[inline(always)]
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x8000..=0xFFFF => self.prg[(addr - 0x8000) as usize % self.prg.len()],
            _ => 0,
        }
    }
    fn write(&mut self, addr: u16, v: u8) { if addr < 0x2000 { self.wram[(addr & 0x7FF) as usize] = v; } }
    fn dma_stalled(&self) -> bool { false }
}

fn run_once(prg: &[u8], pc: u16, cycles: i64) -> f64 {
    let mut bus = Bus { prg: prg.to_vec(), wram: [0; 0x800] };
    let mut cpu = CpuCore::new();
    cpu.set_pc(pc); cpu.set_s(0xFD);
    cpu.run_budget((cycles/20) as i32, &mut bus);
    let t0 = Instant::now();
    let mut rem = cycles;
    while rem > 0 { let r = rem.min(1_000_000) as i32; cpu.run_budget(r, &mut bus); rem -= r as i64; }
    t0.elapsed().as_secs_f64() * 1e3
}

fn stats(name: &str, prg: &[u8], pc: u16, cycles: i64) {
    let mut ts = Vec::new();
    for _ in 0..9 { ts.push(run_once(prg, pc, cycles)); }
    ts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let best = ts[0]; let med = ts[4]; let worst = ts[8];
    println!("{name}: best={best:.3} med={med:.3} worst={worst:.3} ms (60f CPU) => best {:.3} ms/frame",
             best / 60.0);
}

fn main() {
    let cycles = 107_386_380i64;
    let path = fixture("tests/fixtures/nestest.nes");
    let bytes = fs::read(&path).unwrap();
    let prg = bytes[16..16 + 0x4000].to_vec();
    stats("nestest", &prg, 0xC000, cycles);
    stats("nop", &vec![0xEA; 0x8000], 0x8000, cycles);
    println!("\nC++ full-emu baseline (fceux11_bench_x6502_exec, best-of-5): 43.441 ms / 0.724 ms/frame");
}
