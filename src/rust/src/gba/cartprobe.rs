//! Research-only diagnostic: run real commercial cartridges through the core and
//! report whether each one is *running*, *wedged*, or *dead*, and — for the
//! wedged ones — whether it is waiting on a DMA trigger that can never fire.
//!
//! # Why this exists
//!
//! The v2.0.1 manual pass found three real cartridges behaving differently:
//!
//! * `GT Championship (EU)(Kemco)` runs smoothly, and **crashes on close**.
//! * `Super Mario Advance 4 (E)(Menace)` and `Milestone` **freeze during the
//!   load stage**.
//!
//! The freeze is the one that needs a root cause. Three candidate explanations
//! were ruled out by reading the code rather than by guessing:
//!
//! * **Not an infinite loop.** `gba_step_frame` bounds a frame to
//!   `16 * 280_896` steps and returns when the bound is hit, so the core cannot
//!   spin forever inside one frame. What a wedged machine actually does is burn
//!   the whole budget *every* frame, which reads on screen as a freeze. That is
//!   why the step count, and not "did it return", is the measurement.
//! * **Not the battery path.** `gba_battery_read` refuses
//!   `cap < data.len()` with `GBA_ERR_CAPACITY` and `gba_battery_write` refuses
//!   when `backup_type == None`, so there is no unchecked copy on a path that
//!   runs every frame.
//! * **Not `gba_load_rom` failing.** That returns an error code and the C++ side
//!   prints it; it does not crash.
//!
//! What is left is the documented-but-unimplemented **DMA start-timing mode 3**:
//! `run_event_dma` is only ever called with `1` (`VBlank`) and `2` (`HBlank`),
//! never `3`. A cartridge that arms a channel with start timing 3 and waits for
//! its transfer would wait forever. `timing 3` is exactly what GBATEK calls
//! "special" — on real hardware `DMA3` with start timing 3 fires when the LCD
//! starts a display, which is how games stream a picture without tying DMA to a
//! scanline.
//!
//! **This probe does not assume that is the answer.** It measures, per
//! cartridge: steps per frame, whether the budget is exhausted, which DMA
//! channels were armed and with which start timing, how much DMA actually ran,
//! whether the picture changes at all, and where the PC spends its time. The
//! claim "this cartridge is waiting for a DMA that never fires" requires the
//! armed flag — a channel observed *enabled with timing 3* — not merely a low
//! DMA count.
//!
//! # Bracketing rules this probe obeys
//!
//! Two of them are the ones that have bitten this project before, so they are
//! worth stating rather than rediscovering:
//!
//! * **The PC is sampled, never used as an exit condition.** The ARM7TDMI fetch
//!   pointer runs speculatively past a not-yet-taken branch, so "PC reached
//!   address X" fires on the first pass through a loop whose body is one
//!   instruction wide. The loop here is bounded by step count alone.
//! * **"Armed" is confirmed, not assumed.** A register is 0 at power-on, so a
//!   channel that has merely never been written looks identical to a channel
//!   that is armed and unserviced. Both the "ever seen" mask and the end-of-run
//!   state are printed so the two cannot be confused.
//!
//! # What it prints
//!
//! Per cartridge: the step profile, how often the frame budget was exhausted,
//! the DMA channels armed per start timing (observed and at end), DMA blocks and
//! units moved, how many *distinct* frame hashes appeared (a frozen screen is
//! one hash), and the hottest PCs.
//!
//! `#[ignore]`d because it prints rather than asserts: these are numbers to read,
//! and a measurement is not a gate. Run it with
//!
//! ```text
//! set FCEUX11_GBA_PROBE_DIR=<folder holding the .gba files>
//! cargo test -p fceux11-rust --no-default-features --features gba --lib -- --ignored --nocapture cartprobe
//! ```

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    use gba_core::gba::Gba;

    use crate::gba::audio::{self, AudioOut};
    use crate::gba::bios;
    use crate::gba::install_swi_hook;

    /// One GBA frame is 280,896 cycles. The bound is deliberately the same
    /// number the production `gba_step_frame` uses, so a step count printed here
    /// is directly comparable with what the emulator is really doing per frame.
    const CYCLE_LIMIT: u64 = 16 * 280_896;

    /// The host rate the Qt layer pushes in the first frame of a GBA session
    /// (`fceuWrapper_sync_gba_audio` -> `fceu11_gba_configure_audio` ->
    /// `gba_set_output_rate`). 44100 is what FCEUX11 asks its device for.
    const DEVICE_RATE: u32 = 44100;

    /// Frames run before any measurement, so power-on transients (BIOS boot,
    /// the first WAITCNT programming, the first sound-FIFO fill) are not counted
    /// as a wedge.
    const WARMUP_FRAMES: usize = 60;

    /// Frames actually reported.
    const MEASURE_FRAMES: usize = 180;

    /// How often the DMA control registers are sampled. Per-step sampling would
    /// be exact and would also dominate the run time; every 64th step is dense
    /// enough that a channel left armed for a whole frame cannot be missed.
    const SAMPLE_EVERY: u64 = 64;

    /// What one frame did.
    struct Frame {
        steps: u64,
        hit_limit: bool,
        dma_blocks: u64,
        /// Per channel: bit `t` set means start timing `t` was seen while the
        /// channel was enabled, plus the high bit means "ever enabled at all".
        seen_timing: [u8; 4],
        hash: u64,
    }

    impl Frame {
        fn new(steps: u64, hit_limit: bool, dma_blocks: u64, seen_timing: [u8; 4], hash: u64) -> Self {
            Self { steps, hit_limit, dma_blocks, seen_timing, hash }
        }
    }

    /// FNV-1a over the LCD's own pixel buffer, so "is the picture changing" is
    /// answered by the same bytes the viewer would draw rather than by a proxy.
    fn frame_hash(gba: &Gba) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for row in gba.cpu.bus.lcd.buffer.iter() {
            for pixel in row.iter() {
                for channel in [pixel.red(), pixel.green(), pixel.blue()] {
                    hash ^= u64::from(channel);
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
        }
        hash
    }

    /// The DMA control-high registers, read through the bus's own I/O path.
    ///
    /// `Bus::dma` is a private field, so the registers are read where the
    /// hardware exposes them rather than through a widened vendor API:
    /// `read_raw` dispatches `0x0400_00B0..=0x0400_00FF` to `read_dma_raw`,
    /// which is a `&self` read with no side effects. Only `CNT_H` is touched --
    /// on real hardware reading a channel's *source* register clears it, so
    /// reading the whole bank would corrupt the cartridge's own setup.
    const DMA_CNT_H: [usize; 4] =
        [0x0400_00BA, 0x0400_00C6, 0x0400_00D2, 0x0400_00DE];

    fn dma_control(gba: &Gba, idx: usize) -> u16 {
        let base = DMA_CNT_H[idx];
        let lo = gba.cpu.bus.read_raw(base);
        let hi = gba.cpu.bus.read_raw(base + 1);
        u16::from(lo) | (u16::from(hi) << 8)
    }

    /// Start timing bits 12-13 of a DMA control register.
    fn timing_of(control: u16) -> u16 {
        (control >> 12) & 0x3
    }

    fn describe_timing(timing: u16) -> &'static str {
        match timing {
            0 => "immediate",
            1 => "VBlank",
            2 => "HBlank",
            3 => "SPECIAL(display start) -- never fired by this core",
            _ => unreachable!(),
        }
    }

    /// Run one frame exactly the way production does, audio included.
    ///
    /// The audio is not decoration. `Machine::new` wires a sample ring and
    /// `gba_step_frame` calls `advance_frame` at the end of every frame, while
    /// the Qt layer drains `render` into the sound device right after — and a
    /// release build compiles with `panic = "abort"`, so a panic anywhere on
    /// that path takes the whole emulator down with no message on screen. A
    /// probe that leaves the ring unwired is therefore not the same program the
    /// user runs, and its "all three cartridges are fine" is not evidence about
    /// the shipping configuration.
    fn run_frame(
        gba: &mut Gba,
        audio_out: &mut AudioOut,
        samples: &mut [i32],
        hottest: &mut BTreeMap<u32, u32>,
    ) -> Frame {
        let mut steps: u64 = 0;
        let mut seen_timing = [0u8; 4];
        let blocks_before = gba.cpu.bus.dma_blocks_run;

        while steps < CYCLE_LIMIT {
            if steps % SAMPLE_EVERY == 0 {
                let pc = gba.cpu.registers.register_at(15);
                *hottest.entry(pc).or_insert(0) += 1;
                for idx in 0..4 {
                    let control = dma_control(gba, idx);
                    if control & 0x8000 != 0 {
                        seen_timing[idx] |= 1 << timing_of(control);
                        // Bit 7: armed at least once, whatever the timing.
                        seen_timing[idx] |= 0x80;
                    }
                }
            }
            if gba.step() {
                break;
            }
            steps += 1;
        }

        // Production order: the frame's audio is clocked forward first, then the
        // caller drains it.
        audio_out.advance_frame();
        let cap = samples.len() as u32;
        let mut written = 0u32;
        // SAFETY: `samples` is a live slice of `cap` writable i32s, which is
        // what `render` copies into, and `written` is a live local.
        let _ = unsafe { audio_out.render(samples.as_mut_ptr(), cap, &mut written) };

        Frame::new(steps, steps >= CYCLE_LIMIT, gba.cpu.bus.dma_blocks_run - blocks_before, seen_timing, frame_hash(gba))
    }

    fn probe_one(path: &Path) {
        let Ok(rom) = std::fs::read(path) else {
            println!("  !! could not read {}", path.display());
            return;
        };

        println!("\n================ {} ================", path.display());
        println!("ROM: {} bytes", rom.len());

        // Built the way the production path builds it: the stub BIOS plus our
        // SWI hook. Audio is left unwired because it cannot affect CPU or DMA
        // scheduling, and leaving it out keeps the probe free of the rtrb ring.
        let mut gba = Gba::new(bios::stub(), &rom);
        install_swi_hook(&mut gba);

        // Audio, wired exactly as `Machine::new` does it: the ring, the volume
        // and the host rate the Qt layer will ask for.
        let boot_rate = audio::configured_rate();
        let rx = gba.init_audio(boot_rate, audio::RING_SLOTS);
        let mut audio_out = AudioOut::new(boot_rate);
        audio_out.set_volume(audio::configured_volume());
        audio_out.attach(rx, boot_rate);
        let mut samples: Vec<i32> = vec![0; 2 * 5 * (DEVICE_RATE as usize / 60).max(1) + 64];

        let mut hottest: BTreeMap<u32, u32> = BTreeMap::new();
        for _ in 0..WARMUP_FRAMES {
            let _ = run_frame(&mut gba, &mut audio_out, &mut samples, &mut hottest);
        }

        // The Qt layer reconfigures on the session's first frame
        // (`fceuWrapper_sync_gba_audio`), which rebuilds the ring at the device
        // rate. Do the same here rather than assuming the boot rate is final.
        audio_out.set_rate(DEVICE_RATE);
        let rx = gba.init_audio(DEVICE_RATE, audio::RING_SLOTS);
        audio_out.attach(rx, DEVICE_RATE);
        println!(
            "audio: boot rate {} -> device rate {}, ring {} slots, buffer {} samples",
            boot_rate,
            DEVICE_RATE,
            audio::RING_SLOTS,
            samples.len()
        );

        let dma_units_at_start = gba.cpu.bus.dma_units_moved;
        let mut seen_timing = [0u8; 4];
        let mut steps_all = Vec::with_capacity(MEASURE_FRAMES);
        let mut limit_hits = 0usize;
        let mut hashes = BTreeMap::new();
        let mut dma_blocks_in_window = 0u64;

        for _ in 0..MEASURE_FRAMES {
            let frame = run_frame(&mut gba, &mut audio_out, &mut samples, &mut hottest);
            steps_all.push(frame.steps);
            if frame.hit_limit {
                limit_hits += 1;
            }
            dma_blocks_in_window += frame.dma_blocks;
            for idx in 0..4 {
                seen_timing[idx] |= frame.seen_timing[idx];
            }
            *hashes.entry(frame.hash).or_insert(0usize) += 1;
        }
        println!("audio underruns: {}", audio_out.underruns());

        let min = steps_all.iter().copied().min().unwrap_or(0);
        let max = steps_all.iter().copied().max().unwrap_or(0);
        let first: Vec<String> = steps_all.iter().take(8).map(u64::to_string).collect();
        let tail: Vec<String> = steps_all.iter().rev().take(4).rev().map(u64::to_string).collect();

        println!("steps/frame  first 8: [{}]", first.join(", "));
        println!("steps/frame  last 4 : [{}]", tail.join(", "));
        println!("steps/frame  min {} / max {} (budget {})", min, max, CYCLE_LIMIT);
        println!("frames that exhausted the budget: {} / {}", limit_hits, MEASURE_FRAMES);
        println!(
            "distinct frame hashes: {} (1 = frozen picture)",
            hashes.len()
        );
        println!(
            "DMA in window: {} blocks, {} units",
            dma_blocks_in_window,
            gba.cpu.bus.dma_units_moved - dma_units_at_start
        );

        println!("-- DMA channels: armed per start timing --");
        for idx in 0..4 {
            let mask = seen_timing[idx];
            if mask == 0 {
                println!("  DMA{}: never armed", idx);
                continue;
            }
            let mut modes = Vec::new();
            for timing in 0..4u16 {
                if mask & (1 << timing) != 0 {
                    modes.push(format!("{} ({})", timing, describe_timing(timing)));
                }
            }
            println!("  DMA{}: armed with timing {}", idx, modes.join(", "));
            let end = dma_control(&gba, idx);
            if end & 0x8000 != 0 {
                println!(
                    "        STILL ENABLED at end of run: control={:#06x} timing={} -- armed and unserviced",
                    end,
                    describe_timing(timing_of(end))
                );
            }
        }

        println!("-- hottest PCs --");
        let mut ranked: Vec<(u32, u32)> = hottest.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        for (pc, count) in ranked.iter().take(8) {
            println!("  {:#010x}  {} samples", pc, count);
        }

        // The verdict, stated as what was measured rather than as a conclusion.
        let wedged = limit_hits * 2 > MEASURE_FRAMES;
        let frozen = hashes.len() <= 1;
        println!(
            "-- verdict: {} / picture {} / DMA3 special {}",
            if wedged { "WEDGED (budget exhausted most frames)" } else { "running" },
            if frozen { "FROZEN" } else { "changing" },
            if seen_timing[3] & 0b1000 != 0 { "ARMED BUT NEVER SERVICED" } else { "not armed" }
        );
    }

    #[test]
    #[ignore = "prints a per-cartridge report; a measurement is not a gate"]
    fn probe_cartridges_for_wedges() {
        let dir = std::env::var("FCEUX11_GBA_PROBE_DIR").unwrap_or_else(|_| {
            panic!("set FCEUX11_GBA_PROBE_DIR to the folder holding the .gba files")
        });
        let dir = PathBuf::from(dir);

        let mut roms: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                (path.extension().map(|e| e.eq_ignore_ascii_case("gba")) == Some(true)).then_some(path)
            })
            .collect();
        roms.sort();

        println!("probing {} cartridge(s) from {}", roms.len(), dir.display());
        for rom in &roms {
            probe_one(rom);
        }
    }
}