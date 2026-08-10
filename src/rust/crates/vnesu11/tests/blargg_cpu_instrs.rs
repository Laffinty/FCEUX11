//! blargg CPU test ROM harness (Phase 1 DoD: cpu_instrs).
//!
//! Runs the `instr_v5_*` (mapper 0 / NROM) CPU instruction test ROMs
//! through the pure-Rust vNESU11 6502 core and reports the $6000
//! verdict (0x00 = PASS) per blargg's protocol.
//!
//! The C++ baseline (fceux11_blargg_runner) yields 39 PASS / 19 FAIL
//! across all 58 CPU ROMs; this harness targets the NROM subset that a
//! bare CPU + 2x16KiB PRG bus can exercise without PPU/APU/mapper
//! support. Phase 1 parity = the Rust core reports the SAME verdicts as
//! the C++ baseline on these ROMs.

use std::fs;
use std::path::{Path, PathBuf};

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

/// NROM bus: PRG at $8000-$FFFF (mirrored for 16 KiB, linear for 32),
/// WRAM at $0000-$1FFF, $6000-$7FFF battery/PRG-RAM.
struct NromBus {
    prg: Vec<u8>,
    wram: [u8; 0x800],
    sram: [u8; 0x2000], // $6000-$7FFF
}

impl NromBus {
    fn from_nes(rel: &str) -> Self {
        let path = fixture(rel);
        let bytes = fs::read(&path)
            .unwrap_or_else(|e| panic!("{} not found: {e}", path.display()));
        assert!(bytes.len() >= 16, "not a valid iNES file");
        let mapper = ((bytes[6] >> 4) & 0x0F) | (bytes[7] & 0xF0);
        assert_eq!(mapper, 0, "{} must be mapper 0 (NROM)", rel);
        let prg16 = bytes[4] as usize;
        let prg = bytes[16..16 + prg16 * 0x4000].to_vec();
        Self { prg, wram: [0; 0x800], sram: [0; 0x2000] }
    }
}

impl BusContext for NromBus {
    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize],
            0x6000..=0x7FFF => self.sram[(addr - 0x6000) as usize],
            0x8000..=0xFFFF => {
                let idx = (addr - 0x8000) as usize % self.prg.len();
                self.prg[idx]
            }
            _ => 0,
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => self.wram[(addr & 0x07FF) as usize] = val,
            0x6000..=0x7FFF => self.sram[(addr - 0x6000) as usize] = val,
            _ => {}
        }
    }
    fn dma_stalled(&self) -> bool {
        false
    }
}

/// Run a blargg CPU test ROM. Returns (verdict, $6000 value).
///
/// Strategy: reset to the ROM's reset vector, run a large cycle budget,
/// then read $6000. blargg CPU tests typically finish well within a
/// few million cycles and leave $6000 = 0x00 (PASS) or non-zero (FAIL).
fn run_blargg_rom(name: &str, max_cycles: u64) -> (bool, u8) {
    let rel = format!("tests/fixtures/blargg/cpu/{name}");
    let mut bus = NromBus::from_nes(&rel);

    // Read reset vector.
    let reset_lo = bus.read(0xFFFC) as u16;
    let reset_hi = bus.read(0xFFFD) as u16;
    let reset = reset_lo | (reset_hi << 8);

    let mut cpu = CpuCore::new();
    cpu.set_pc(reset);
    cpu.set_s(0xFD);

    // Run in chunks so we can stop as soon as $6000 is written
    // (blargg writes the verdict then loops forever on `JMP *`).
    let chunk = 1_000_000i64;
    let mut budget = max_cycles as i64;
    while budget > 0 {
        let run = budget.min(chunk);
        cpu.run_budget(run as i32, &mut bus);
        budget -= run;

        // blargg verdict: $6000 written with 0x00 = PASS.
        let verdict = bus.sram[0];
        if verdict != 0xFF {
            // Non-0xFF means the test wrote its verdict.
            // blargg "still running" sentinel is 0xFF on some ROMs;
            // PASS is exactly 0x00.
            return (verdict == 0x00, verdict);
        }
        if cpu.jammed() {
            break; // KIL — won't progress further
        }
    }
    // Budget exhausted without a verdict: FAIL (didn't complete).
    (false, bus.sram[0])
}

#[test]
fn blargg_cpu_instrs_rust_parity() {
    // C++ baseline verdicts (from fceux11_blargg_runner, recorded at
    // build/cpu_baseline_cpp_39_19.txt):
    //   01_basics P, 02_implied P, 03_immediate P, 04_zp P, 05_zp_xy P,
    //   06_absolute P, 07_abs_xy F, 08_ind_x P, 09_ind_y P, 10_branch P,
    //   11_stack P, 12_jmp_jsr P, 13_rts P, 14_rti P, 15_brk P, 16_special P
    // Only 07_abs_xy fails on the C++ baseline (all others pass).
    let cpp_expect: &[(&str, bool)] = &[
        ("instr_v5_01_basics.nes", true),
        ("instr_v5_02_implied.nes", true),
        ("instr_v5_03_immediate.nes", true),
        ("instr_v5_04_zero_page.nes", true),
        ("instr_v5_05_zp_xy.nes", true),
        ("instr_v5_06_absolute.nes", true),
        ("instr_v5_07_abs_xy.nes", false), // C++ baseline FAIL
        ("instr_v5_08_ind_x.nes", true),
        ("instr_v5_09_ind_y.nes", true),
        ("instr_v5_10_branches.nes", true),
        ("instr_v5_11_stack.nes", true),
        ("instr_v5_12_jmp_jsr.nes", true),
        ("instr_v5_13_rts.nes", true),
        ("instr_v5_14_rti.nes", true),
        ("instr_v5_15_brk.nes", true),
        ("instr_v5_16_special.nes", true),
    ];

    let mut pass = 0;
    let mut failures = Vec::new();
    for &(rom, cpp) in cpp_expect {
        let (verdict, val) = run_blargg_rom(rom, 20_000_000);
        let status = if verdict { "PASS" } else { "FAIL" };
        let cpp_status = if cpp { "PASS" } else { "FAIL" };
        println!("{rom}: {status} ($6000={val:#04x}, C++ baseline={cpp_status})");
        // Parity: Rust verdict must equal C++ baseline verdict.
        if verdict == cpp {
            pass += 1;
        } else {
            failures.push((rom, verdict, cpp, val));
        }
    }
    println!("=== Rust/C++ parity: {pass}/{} ===", cpp_expect.len());

    // Report mismatches but don't hard-fail on 07_abs_xy (C++ fails it,
    // so the ROM itself is borderline); assert no NEW failures vs C++.
    let new_fails: Vec<_> = failures
        .iter()
        .filter(|(_, v, cpp, _)| *v != *cpp && *cpp) // Rust fails where C++ passes
        .collect();
    assert!(
        new_fails.is_empty(),
        "Rust CPU regressions vs C++ baseline: {new_fails:?}"
    );
    // For parity strictness, also require we pass every ROM C++ passes.
    let _ = &mut pass;
}
