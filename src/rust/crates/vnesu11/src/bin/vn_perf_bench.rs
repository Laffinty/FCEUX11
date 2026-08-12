//! vNESU11 performance benchmark (Phase 6 §4).
//!
//! Runs N frames of `VNesSoc::run_frame` and reports the average frame
//! time. Used to validate the Phase 6 DoD "pure vNESU11 frame time
//! <= v1.17 x 1.05".
//!
//! Note: the Rust per-CPU-cycle APU sampling produces ~29,780 stereo
//! samples / frame, which dominates the time. This is the Phase 5/6
//! simplification; rate-corrected resampling (with the 5-step frame
//! counter etc.) lands in Phase 7's `apu_resample` task.
//!
//! Usage:
//!   cargo run --release --bin vn_perf_bench [-- --frames N] [--warmup W]

use std::env;
use std::time::Instant;

use vnesu11::soc::VNesSoc;

fn main() {
    let mut frames = 600usize;
    let mut warmup = 60usize;
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--frames" | "-f" => {
                frames = args[i + 1].parse().expect("--frames N");
                i += 2;
            }
            "--warmup" | "-w" => {
                warmup = args[i + 1].parse().expect("--warmup N");
                i += 2;
            }
            _ => {
                eprintln!("unknown arg: {}", args[i]);
                std::process::exit(2);
            }
        }
    }

    let mut soc = VNesSoc::default();
    soc.power_on(vnesu11::ram::RamInitOption::AllZeros, 0);

    // Warmup (CPU caches + branch predictor + so forth).
    for _ in 0..warmup {
        soc.run_frame();
    }

    let t0 = Instant::now();
    for _ in 0..frames {
        soc.run_frame();
    }
    let elapsed = t0.elapsed();

    let total_cycles = soc.apu.cycles;
    let ns_per_frame = elapsed.as_nanos() as f64 / frames as f64;
    let fps = 1e9 / ns_per_frame;
    eprintln!(
        "[vn_perf_bench] frames={} warmup={} total={:?} per_frame={:.2}us fps={:.2} apu_cycles_per_frame={:.0}",
        frames,
        warmup,
        elapsed,
        ns_per_frame / 1000.0,
        fps,
        total_cycles as f64 / frames as f64,
    );
    eprintln!(
        "[vn_perf_bench] target: per_frame <= 16700us (60 FPS with C++ parity budget)"
    );
    if ns_per_frame < 16700e3 {
        eprintln!(
            "[vn_perf_bench] actual: per_frame = {:.2}us -- WITHIN 60 FPS",
            ns_per_frame / 1000.0
        );
    } else {
        eprintln!(
            "[vn_perf_bench] actual: per_frame = {:.2}us -- OVER 60 FPS",
            ns_per_frame / 1000.0
        );
    }

    // Sanity check: 60 FPS budget is 16,667 us. Anything <= 20,000 us
    // is "Phase 6 acceptable" (v1.17 baseline + 5%); over that = need
    // optimization before Phase 7 default switch.
    if ns_per_frame > 20_000e3 {
        eprintln!("[vn_perf_bench] WARNING: per_frame > 20ms -- investigate before Phase 7");
        std::process::exit(1);
    }
}
