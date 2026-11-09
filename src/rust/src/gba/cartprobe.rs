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
//! the DMA channels armed per start timing judged **per channel** (v2.0.3 plan
//! §1.3: DMA1/DMA2's special timing is the sound FIFO and this core services
//! it; only DMA3's is never fired), DMA blocks and units moved, how many
//! *distinct* frame hashes appeared (a frozen screen is one hash), the hottest
//! PCs and how many consecutive frames the hottest PC held still, which SWI
//! numbers reached the dispatcher and whether anyone serviced them, and -- in
//! the test builds, where unwinding is available -- any core panic caught
//! mid-frame, as text.
//!
//! # Knobs (all environment variables, all optional)
//!
//! * `GBA_PROBE_FRAMES` / `GBA_PROBE_WARMUP` -- window sizes (defaults 180/60,
//!   i.e. the 240 frames the first version fixed).
//! * `GBA_PROBE_STUCK_FRAMES` -- how many consecutive frames of an unchanged
//!   hottest PC the verdict calls "stable" (default 30).
//! * `GBA_PROBE_INPUT` -- a button schedule, `"frame:BUTTON[,BUTTON];..."`,
//!   frames 1-based over the measured window (warm-up excluded). `120:A,Start`
//!   presses and holds A+Start from frame 120 on; `150:` releases everything.
//!   Button names use the `GbaButton::name()` spelling, case-insensitive.
//! * `GBA_PROBE_ONLY` -- probe only cartridges whose file name contains this
//!   substring.
//!
//! The probes are `#[ignore]`d because they print rather than assert: these
//! are numbers to read, and a measurement is not a gate. One **resident** test
//! pins the DMA wording -- the plan's §1.3 trap -- and is part of the gate.
//! Run the probes with
//!
//! ```text
//! set FCEUX11_GBA_PROBE_DIR=<folder holding the .gba files>
//! cargo test -p fceux11-rust --no-default-features --features gba --lib -- --ignored --nocapture cartprobe
//! ```

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::{Path, PathBuf};

    use gba_core::cpu::hardware::keypad::GbaButton;
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

    /// How many consecutive frames of an unchanged hottest PC the verdict
    /// calls "stable". `GBA_PROBE_STUCK_FRAMES` overrides it.
    const STUCK_FRAMES: usize = 30;

    /// How many frames after a scheduled press the stroke report watches for
    /// a picture change.
    const STROKE_HORIZON: usize = 30;

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
        /// The frame's most-sampled PC and how many samples it got. "PC
        /// unchanged for N frames" is read off consecutive frames' values.
        top_pc: Option<(u32, u32)>,
        /// The caught panic's text, if the core panicked mid-frame. Only ever
        /// set in builds that unwind (see the note on [`run_frame`]); a frame
        /// that panicked has no honest hash or block count, so those read 0.
        panic: Option<String>,
    }

    impl Frame {
        #[allow(clippy::too_many_arguments)]
        fn new(
            steps: u64,
            hit_limit: bool,
            dma_blocks: u64,
            seen_timing: [u8; 4],
            hash: u64,
            top_pc: Option<(u32, u32)>,
            panic: Option<String>,
        ) -> Self {
            Self { steps, hit_limit, dma_blocks, seen_timing, hash, top_pc, panic }
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

    /// The name of a DMA start timing, **per channel**.
    ///
    /// Channel-aware on purpose (v2.0.3 plan §1.3): timing 3 ("special") means
    /// a different peripheral on every channel, and only DMA3's special source
    /// is unserviced by this core. DMA1/DMA2's special source is the sound
    /// FIFO, which `run_fifo_dma` services from timer overflow; calling that
    /// "never fired" -- as the first, channel-blind version of this text did
    /// -- is exactly the misleading verdict the plan calls out.
    fn describe_timing(channel: usize, timing: u16) -> &'static str {
        match timing {
            0 => "immediate",
            1 => "VBlank",
            2 => "HBlank",
            3 => match channel {
                0 => "SPECIAL -- prohibited on DMA0 (GBATEK A.1)",
                1 | 2 => "SPECIAL(sound FIFO) -- serviced by this core via timer-driven run_fifo_dma",
                _ => "SPECIAL(video capture) -- never fired by this core",
            },
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
    ///
    /// The frame step is wrapped in `catch_unwind` so a core panic becomes a
    /// **reading** ("this cartridge dies mid-frame, here is the message")
    /// instead of a dead test binary. The catch only works where unwinding
    /// does: cargo forces unwind for test targets even under `--release`
    /// (verified 2026-11-09 with a minimal out-of-repo experiment), and every
    /// entry point into this module is a `cargo test` probe. The shipping
    /// release build keeps `panic = "abort"`, so its behaviour is unchanged.
    fn run_frame(
        gba: &mut Gba,
        audio_out: &mut AudioOut,
        samples: &mut [i32],
        hottest: &mut BTreeMap<u32, u32>,
    ) -> Frame {
        let mut steps: u64 = 0;
        let mut seen_timing = [0u8; 4];
        let mut pcs: BTreeMap<u32, u32> = BTreeMap::new();
        let blocks_before = gba.cpu.bus.dma_blocks_run;

        install_panic_probe();
        // Cleared per frame, so a reading is this frame's own.
        RECORDED_PANIC.with(|slot| *slot.borrow_mut() = None);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            while steps < CYCLE_LIMIT {
                if steps % SAMPLE_EVERY == 0 {
                    let pc = gba.cpu.registers.register_at(15);
                    *hottest.entry(pc).or_insert(0) += 1;
                    *pcs.entry(pc).or_insert(0) += 1;
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
        }));

        // The state after a caught panic is unknown, so the frame is left
        // unfinished on purpose: no audio advance, no render, hash and block
        // count meaningless, and the caller stops measuring this cartridge.
        if let Err(payload) = outcome {
            let text = RECORDED_PANIC
                .with(|slot| slot.borrow_mut().take())
                .unwrap_or_else(|| panic_text(&payload));
            let top_pc = pcs.into_iter().max_by_key(|&(_, count)| count);
            return Frame::new(
                steps,
                steps >= CYCLE_LIMIT,
                0,
                seen_timing,
                0,
                top_pc,
                Some(text),
            );
        }

        // Production order: the frame's audio is clocked forward first, then the
        // caller drains it.
        audio_out.advance_frame();
        let cap = samples.len() as u32;
        let mut written = 0u32;
        // SAFETY: `samples` is a live slice of `cap` writable i32s, which is
        // what `render` copies into, and `written` is a live local.
        let _ = unsafe { audio_out.render(samples.as_mut_ptr(), cap, &mut written) };

        let top_pc = pcs.into_iter().max_by_key(|&(_, count)| count);
        let dma_blocks = gba.cpu.bus.dma_blocks_run - blocks_before;
        Frame::new(
            steps,
            steps >= CYCLE_LIMIT,
            dma_blocks,
            seen_timing,
            frame_hash(gba),
            top_pc,
            None,
        )
    }

    /// A caught panic payload as text. Local to each catch site (this file and
    /// `frame.rs`): ten lines do not justify a new cross-module seam. Best
    /// effort only -- the hook ([`install_panic_probe`]) is the primary
    /// source, because on this toolchain a formatted panic's payload does not
    /// downcast to a string type.
    fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
        if let Some(text) = payload.downcast_ref::<&str>() {
            (*text).to_string()
        } else if let Some(text) = payload.downcast_ref::<String>() {
            text.clone()
        } else {
            "<non-string panic payload>".to_string()
        }
    }

    thread_local! {
        /// What the hook saw on *this* thread: `file:line: message`.
        static RECORDED_PANIC: std::cell::RefCell<Option<String>> =
            const { std::cell::RefCell::new(None) };
    }

    /// Install the hook that renders a caught panic as a readable reading.
    ///
    /// Same trick as `frame.rs`'s frame-panic probe and `fixtures/oam.rs`'s
    /// OAM probe: the hook is the text source (this toolchain's formatted
    /// panic payloads do not downcast), the record is thread-local, and the
    /// hook chains to whatever was installed before it.
    fn install_panic_probe() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let previous = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                let location = info
                    .location()
                    .map(|location| format!("{}:{}", location.file(), location.line()));
                let text = match info.payload_as_str() {
                    Some(text) => text.to_string(),
                    None => info.to_string(),
                };
                RECORDED_PANIC.with(|slot| {
                    *slot.borrow_mut() = Some(match location {
                        Some(location) => format!("{location}: {text}"),
                        None => text,
                    })
                });
                previous(info);
            }));
        });
    }

    /// `GBA_PROBE_<name>` as a usize, or the default. Malformed values panic:
    /// a probe that quietly ignores a typo'd knob reports the wrong window.
    fn env_usize(name: &str, default: usize) -> usize {
        match std::env::var(name) {
            Ok(value) => value
                .trim()
                .parse()
                .unwrap_or_else(|e| panic!("{name}={value:?} is not a number: {e}")),
            Err(_) => default,
        }
    }

    /// A scheduled press: the measured-frame index (1-based) it lands on, the
    /// full button mask to hold from that frame on, and a printable label.
    struct Stroke {
        frame: usize,
        mask: u16,
        label: String,
    }

    /// Parse `GBA_PROBE_INPUT="120:A,Start;150:"`.
    ///
    /// Malformed input panics the probe on purpose: a silently ignored
    /// schedule would produce a readout that reads as "the game ignores
    /// input" -- the exact wrong conclusion.
    fn parse_input(spec: &str) -> Vec<Stroke> {
        let all_names = GbaButton::ALL.map(|button| button.name()).join("/");
        let mut strokes = Vec::new();
        for entry in spec.split(';') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let (frame, names) = entry
                .split_once(':')
                .unwrap_or_else(|| panic!("GBA_PROBE_INPUT: expected `frame:BUTTONS`, got {entry:?}"));
            let frame: usize = frame
                .trim()
                .parse()
                .unwrap_or_else(|e| panic!("GBA_PROBE_INPUT: frame {frame:?} is not a number: {e}"));
            let mut mask = 0u16;
            let mut named = Vec::new();
            for name in names.split(',') {
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let button = GbaButton::ALL
                    .into_iter()
                    .find(|button| button.name().eq_ignore_ascii_case(name))
                    .unwrap_or_else(|| {
                        panic!("GBA_PROBE_INPUT: unknown button {name:?} (expected one of {all_names})")
                    });
                mask |= button as u16;
                named.push(button.name().to_string());
            }
            let label = if named.is_empty() {
                "release all".to_string()
            } else {
                named.join("+")
            };
            strokes.push(Stroke { frame, mask, label });
        }
        strokes.sort_by_key(|stroke| stroke.frame);
        strokes
    }

    /// Full-state button write, mirroring `frame.rs`'s `gba_set_buttons`: the
    /// core releases every button the mask does not name.
    fn apply_buttons(gba: &mut Gba, mask: u16) {
        for button in GbaButton::ALL {
            gba.cpu.bus.keypad.set_button(button, mask & (button as u16) != 0);
        }
    }

    /// Whether the picture moved after a scheduled press. The baseline is the
    /// hash of the frame the press landed in; the horizon is a judgement call
    /// and is stated in the line, not hidden.
    fn report_stroke(stroke: &Stroke, hashes: &[u64]) {
        if stroke.frame == 0 || stroke.frame > hashes.len() {
            println!(
                "  input frame {}: beyond the measured window ({} frames), never applied",
                stroke.frame,
                hashes.len()
            );
            return;
        }
        let base = hashes[stroke.frame - 1];
        let horizon = hashes.len().min(stroke.frame + STROKE_HORIZON);
        let after = &hashes[stroke.frame..horizon];
        let verdict = if after.is_empty() {
            "landed on the last frame; nothing after it to observe".to_string()
        } else if after.iter().any(|hash| *hash != base) {
            "picture CHANGED within the horizon".to_string()
        } else {
            "picture did NOT change within the horizon".to_string()
        };
        println!(
            "  input frame {}: {} -- {} (horizon {} frames)",
            stroke.frame, stroke.label, verdict, STROKE_HORIZON
        );
    }

    /// Print what the SWI dispatcher saw. `served` needs interpreting: the
    /// vendored core answers six numbers itself when we decline (plan §1.2.1),
    /// and after the first truly unserved number the CPU parks at BIOS 0x08,
    /// so nothing later in the run can even reach the dispatcher.
    fn report_swi_tallies() {
        let tallies = crate::gba::swi::take_swi_tallies();
        if tallies.is_empty() {
            println!("-- SWI dispatcher: no call reached it");
            return;
        }
        println!("-- SWI dispatcher: calls per number --");
        for (num, calls, served) in tallies {
            let status = if served > 0 {
                "served by our dispatch".to_string()
            } else if crate::gba::swi::vendor_core_serves(num) {
                "declined -- the vendored core services this number itself".to_string()
            } else {
                "NOT SERVED -- the CPU parks at BIOS 0x08 from the first such call on".to_string()
            };
            println!(
                "  {:#04x} {:<22} {:>6} calls  {}",
                num,
                crate::gba::swi::swi_name(num),
                calls,
                status
            );
        }
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

        // Knobs first, so the header states the window actually used.
        let warmup = env_usize("GBA_PROBE_WARMUP", WARMUP_FRAMES);
        let frames = env_usize("GBA_PROBE_FRAMES", MEASURE_FRAMES).max(1);
        let stuck_frames = env_usize("GBA_PROBE_STUCK_FRAMES", STUCK_FRAMES).max(1);
        let strokes = std::env::var("GBA_PROBE_INPUT")
            .map(|spec| parse_input(&spec))
            .unwrap_or_default();
        if !strokes.is_empty() {
            let listed: Vec<String> = strokes
                .iter()
                .map(|stroke| format!("frame {}: {}", stroke.frame, stroke.label))
                .collect();
            println!(
                "input schedule: {} (frames 1-based over the {} measured)",
                listed.join("; "),
                frames
            );
        }
        println!(
            "window: warm-up {} + measured {} frames (GBA_PROBE_WARMUP / GBA_PROBE_FRAMES)",
            warmup, frames
        );

        // Drained before the run and reported after it, so the tallies are
        // this cartridge's own: the recorder is thread-local and a previous
        // probe's numbers must not bleed in.
        let _ = crate::gba::swi::take_swi_tallies();

        let mut hottest: BTreeMap<u32, u32> = BTreeMap::new();
        for _ in 0..warmup {
            let frame = run_frame(&mut gba, &mut audio_out, &mut samples, &mut hottest);
            if let Some(text) = &frame.panic {
                println!("  PANIC during a warm-up frame: {text}");
                println!("  -- the cartridge is dead this early; readings end here");
                report_swi_tallies();
                return;
            }
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
        let mut steps_all = Vec::with_capacity(frames);
        let mut limit_hits = 0usize;
        // Per-frame hashes, in order: the stroke report reads windows out of
        // them, and the distinct count comes from a sorted copy.
        let mut hashes: Vec<u64> = Vec::with_capacity(frames);
        let mut dma_blocks_in_window = 0u64;
        // "PC unchanged for N consecutive frames": the hottest PC of each
        // frame, compared frame to frame. The streak that matters is the one
        // still standing at the end -- a machine that froze mid-window keeps
        // its streak; one that merely idled early and moved on does not.
        let mut prev_top: Option<u32> = None;
        let mut streak = 0usize;

        // Strokes land at the top of their frame, so the buttons are down for
        // the whole frame -- the same order the C++ side uses (input first,
        // then the frame step).
        for index in 0..frames {
            if let Some(stroke) = strokes.iter().find(|stroke| stroke.frame == index + 1) {
                apply_buttons(&mut gba, stroke.mask);
            }
            let frame = run_frame(&mut gba, &mut audio_out, &mut samples, &mut hottest);
            if let Some(text) = &frame.panic {
                println!("\n  PANIC during measured frame {}: {text}", index + 1);
                println!(
                    "  -- this cartridge is stopped here; the readings below cover the frames before it"
                );
                break;
            }
            steps_all.push(frame.steps);
            hashes.push(frame.hash);
            if frame.hit_limit {
                limit_hits += 1;
            }
            dma_blocks_in_window += frame.dma_blocks;
            for idx in 0..4 {
                seen_timing[idx] |= frame.seen_timing[idx];
            }
            match frame.top_pc {
                Some((pc, _)) if prev_top == Some(pc) => streak += 1,
                Some(_) => streak = 1,
                None => {}
            }
            prev_top = frame.top_pc.map(|(pc, _)| pc);
        }
        println!("audio underruns: {}", audio_out.underruns());

        for stroke in &strokes {
            report_stroke(stroke, &hashes);
        }

        if steps_all.is_empty() {
            println!("no frame completed -- the window produced nothing to judge");
            report_swi_tallies();
            return;
        }

        let min = steps_all.iter().copied().min().unwrap_or(0);
        let max = steps_all.iter().copied().max().unwrap_or(0);
        let first: Vec<String> = steps_all.iter().take(8).map(u64::to_string).collect();
        let tail: Vec<String> = steps_all.iter().rev().take(4).rev().map(u64::to_string).collect();
        let mut distinct = hashes.clone();
        distinct.sort_unstable();
        distinct.dedup();

        println!("steps/frame  first 8: [{}]", first.join(", "));
        println!("steps/frame  last 4 : [{}]", tail.join(", "));
        println!("steps/frame  min {} / max {} (budget {})", min, max, CYCLE_LIMIT);
        println!("frames that exhausted the budget: {} / {}", limit_hits, steps_all.len());
        println!(
            "distinct frame hashes: {} (1 = frozen picture)",
            distinct.len()
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
                    modes.push(format!("{} ({})", timing, describe_timing(idx, timing)));
                }
            }
            println!("  DMA{}: armed with timing {}", idx, modes.join(", "));
            let end = dma_control(&gba, idx);
            if end & 0x8000 != 0 {
                println!(
                    "        STILL ENABLED at end of run: control={:#06x} timing={} -- armed and unserviced",
                    end,
                    describe_timing(idx, timing_of(end))
                );
            }
        }

        println!("-- hottest PCs --");
        let mut ranked: Vec<(u32, u32)> = hottest.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        for (pc, count) in ranked.iter().take(8) {
            println!("  {:#010x}  {} samples", pc, count);
        }

        // The verdict, stated as what was measured rather than as a conclusion,
        // and judged per channel (v2.0.3 plan §1.3): only DMA3's special timing
        // is unserviced by this core; DMA1/DMA2's is the sound FIFO, which
        // `run_fifo_dma` services from timer overflow -- calling an armed FIFO
        // channel "never fired" was exactly the misleading verdict this probe
        // used to print.
        let wedged = limit_hits * 2 > steps_all.len();
        let frozen = distinct.len() <= 1;
        println!(
            "-- verdict: {} / picture {}",
            if wedged { "WEDGED (budget exhausted most frames)" } else { "running" },
            if frozen { "FROZEN" } else { "changing" }
        );
        for idx in 0..4 {
            if seen_timing[idx] & 0b1000 == 0 {
                continue;
            }
            match idx {
                3 => println!(
                    "  DMA3 special (video capture): ARMED BUT NEVER SERVICED -- whatever waits on it waits forever"
                ),
                1 | 2 => println!(
                    "  DMA{} special (sound FIFO): armed -- this core services that timing, so this alone is not a wedge",
                    idx
                ),
                _ => println!("  DMA0 special: armed with a timing GBATEK calls prohibited on DMA0"),
            }
        }
        if prev_top.is_some() && streak >= stuck_frames {
            println!(
                "  PC stable: the hottest PC {:#010x} held for the last {} frames{}",
                prev_top.unwrap(),
                streak,
                if frozen { ", picture FROZEN" } else { "" }
            );
        }
        report_swi_tallies();
    }

    #[test]
    #[ignore = "prints a per-cartridge report; a measurement is not a gate"]
    fn probe_cartridges_for_wedges() {
        let dir = std::env::var("FCEUX11_GBA_PROBE_DIR").unwrap_or_else(|_| {
            panic!("set FCEUX11_GBA_PROBE_DIR to the folder holding the .gba files")
        });
        let dir = PathBuf::from(dir);
        let only = std::env::var("GBA_PROBE_ONLY").ok();
        if let Some(sub) = &only {
            println!("filter: file names containing {sub:?}");
        }

        let mut roms: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension().map(|e| e.eq_ignore_ascii_case("gba")) != Some(true) {
                    return None;
                }
                if let Some(sub) = &only {
                    let matches = path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_lowercase())
                        .is_some_and(|name| name.contains(&sub.to_lowercase()));
                    if !matches {
                        return None;
                    }
                }
                Some(path)
            })
            .collect();
        roms.sort();

        println!("probing {} cartridge(s) from {}", roms.len(), dir.display());
        for rom in &roms {
            probe_one(rom);
        }
    }

    /// The wording lock for the v2.0.3 plan §1.3 trap: DMA1/DMA2's special
    /// timing is the sound FIFO and **is** serviced by this core; only DMA3's
    /// never fires. The first version of `describe_timing` was channel-blind
    /// and called a serviced timing "never fired by this core" -- exactly the
    /// misleading verdict the plan forbids. Resident (not ignored): the
    /// wording is the deliverable, and reverting to channel-blind text must
    /// turn this red.
    #[test]
    fn special_timing_wording_is_channel_aware() {
        for channel in 0..4 {
            assert_eq!(describe_timing(channel, 0), "immediate");
            assert_eq!(describe_timing(channel, 1), "VBlank");
            assert_eq!(describe_timing(channel, 2), "HBlank");
        }
        for fifo in [1usize, 2] {
            let text = describe_timing(fifo, 3);
            assert!(text.contains("FIFO"), "DMA{fifo} timing 3 is the sound FIFO: {text}");
            assert!(
                !text.contains("never fired"),
                "DMA{fifo} timing 3 IS serviced; the text must not say otherwise: {text}"
            );
        }
        let dma3 = describe_timing(3, 3);
        assert!(dma3.contains("never fired"), "DMA3 timing 3 is the unserviced one: {dma3}");
        assert!(dma3.contains("video capture"), "name the special source: {dma3}");
        let dma0 = describe_timing(0, 3);
        assert!(dma0.contains("prohibited"), "DMA0 timing 3 is prohibited: {dma0}");
    }
}