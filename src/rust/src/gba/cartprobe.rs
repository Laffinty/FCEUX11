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
    /// and a number nobody implements is no longer a wedge (v2.0.3 S2-b) --
    /// the stub BIOS's SWI vector returns the machine to the caller, so later
    /// calls can and do reach the dispatcher. The count of the unserved number
    /// is how often the game called into a missing implementation.
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
                "NOT SERVED -- returned without being serviced (S2-b fallback), \
                 game keeps running"
                    .to_string()
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

    // ---- S3-c forensics (v2.0.3 Stage 1) --------------------------------
    //
    // One probe that answers the four S3-c questions on a real cartridge:
    //
    // 1. *Who writes the IWRAM flag FireRed waits on* (plan §1.2.3). The plan
    //    quotes the flag at 0x030030FC (base 0x030030F0 + 0xC) but the same
    //    section quotes the polled instruction as `LDRH r1,[r2,#28]`, whose
    //    target is 0x0300310C -- the two disagree by 16 bytes, so the watch
    //    covers 0x030030F0..=0x0300310F and lets the machine say which one it
    //    is. Writes are observed by scanning the window every step; each
    //    change records the PC the machine had just after the writing step
    //    (the writer is the preceding instruction: PC-2 in Thumb, PC-4 in ARM).
    // 2. *GPIO read/write order* (plan §1.2.3's lead). Writes are observed
    //    exactly: `try_read_word(0x080000C4)` returns None if and only if
    //    `gpio_present` is true (internal_memory.rs:431 routes the overlapping
    //    word read to the slow path only when present), so the flip to true is
    //    the game's first GPIO write, dated to the step. Reads leave no state
    //    to poll, so they are observed through the CPU's own disassembler
    //    channel: the probe pops one entry per step and reads r0-r14; any
    //    register value inside the GPIO window is recorded with the formatted
    //    instruction -- this catches `ldr rX, =0x080000C4` pointer loads as
    //    well as the accesses themselves. Read-hits dated before the flip step
    //    are the "read before write" evidence; none is the refutation.
    // 3. *The save path on EEPROM carts* (Dragon Ball): `take_save_dirty()`
    //    per frame plus a `battery_data()` diff per frame say when a save
    //    command actually committed and where -- without ever reading the
    //    EEPROM window, which would consume protocol bits. CPU-level accesses
    //    to the EEPROM window are counted by the same disassembly sniff.
    // 4. *Long windows with DISPCNT/KEYS/IRQ readouts* for the Mario Kart and
    //    Minish Cap passes.
    //
    // Observers are individually switchable with GBA_PROBE_WATCH (comma
    // separated): `iwram` (the per-step flag-window scan -- 16 halfword reads
    // per step, affordable on short windows only), `gpio`, `save`, `irq`,
    // `disasm` (the sniff, bounded by GBA_PROBE_DISASM_STEPS instructions,
    // starting at 1-based global frame GBA_PROBE_DISASM_FROM_FRAME).
    // Everything else follows the cartprobe conventions: audio wired the way
    // the Qt layer does it, catch_unwind turns a core panic into a reading,
    // and the PC is sampled for attribution but never used as an exit
    // condition (discipline 11).

    use gba_core::cpu::DisasmEntry;

    /// The per-step IWRAM watch windows (base, halfword count):
    ///
    /// * `0x030030F0..=0x0300310F` -- FireRed's gMain: both candidate flag
    ///   addresses the plan quotes (0x030030FC and 0x0300310C) live here, plus
    ///   the three callback pointers at +0x04/+0x0C/+0x18.
    /// * `0x03000000..=0x0300000F` -- the IRQ-manager state area: FireRed
    ///   programs DISPSTAT as `(DISPSTAT & ~0x0018) | *(0x03000004)` (ROM
    ///   0x080009DC-0x080009F0), so the shadow word's writes are the game's
    ///   actual intent to enable the V-IRQ.
    /// * `0x03007FF8..=0x03007FFF` -- the BIOS IntrWait flags and IRQPTR.
    const WATCH_WINDOWS: [(usize, usize); 3] =
        [(0x0300_30F0, 16), (0x0300_0000, 8), (0x0300_7FF8, 4)];

    /// The GPIO register block inside the ROM window.
    const GPIO_BASE: usize = 0x0800_00C4;

    /// Sniff windows (half-open `[lo, hi)`).
    const GPIO_SNIFF: (u32, u32) = (0x0800_00C0, 0x0800_00D0);
    const EEPROM_SNIFF: (u32, u32) = (0x0D00_0000, 0x0E00_0000);
    const FLAG_SNIFF: (u32, u32) = (0x0300_3000, 0x0300_3200);

    /// Distinct-PC cap per sniff category; beyond it hits are only counted.
    const SNIFF_MAP_CAP: usize = 400;
    /// Ordered IWRAM change events kept verbatim; beyond it only the
    /// per-address aggregate keeps counting.
    const IWRAM_LOG_CAP: usize = 64;
    /// GPIO register-timeline cap.
    const GPIO_TIMELINE_CAP: usize = 64;

    /// Per-distinct-PC record of one disassembly-sniff category.
    #[derive(Default)]
    struct Sniff {
        total: u64,
        first_step: Option<u64>,
        /// pc -> (count, first step, formatted instruction, register snapshot)
        map: BTreeMap<u32, (u64, u64, String, String)>,
    }

    impl Sniff {
        /// Count one hit. When this is the first hit for `pc` (and the map has
        /// room) the returned bool tells the caller to produce the instruction
        /// text and register snapshot, which land in [`Sniff::attach`] --
        /// formatting per hit would dominate the run.
        fn hit(&mut self, pc: u32, step: u64) -> bool {
            self.total += 1;
            if self.first_step.is_none() {
                self.first_step = Some(step);
            }
            if let Some(entry) = self.map.get_mut(&pc) {
                entry.0 += 1;
                false
            } else if self.map.len() < SNIFF_MAP_CAP {
                self.map.insert(pc, (1, step, String::new(), String::new()));
                true
            } else {
                false
            }
        }

        fn attach(&mut self, pc: u32, text: String, regs: String) {
            if let Some(entry) = self.map.get_mut(&pc) {
                entry.2 = text;
                entry.3 = regs;
            }
        }
    }

    /// Everything the per-step observers saw. The PC recorded on memory-change
    /// events is `program_counter()` *after* the step that made the change:
    /// the writer is the instruction just before it (PC-2 in Thumb, PC-4 in
    /// ARM) -- attribution by adjacency, never an exit condition.
    struct Forensics {
        global_step: u64,
        watch_iwram: bool,
        watch_gpio: bool,
        watch_irq: bool,
        iwram_prev: Vec<u16>,
        /// addr -> (count, first step, first value, last value, last pc, last frame)
        iwram_agg: BTreeMap<u32, (u64, u64, u16, u16, u32, u32)>,
        /// (step, frame, addr, old, new, pc) in order, capped.
        iwram_log: Vec<(u64, u32, u32, u16, u16, u32)>,
        gpio_present: bool,
        gpio_flip_step: Option<u64>,
        gpio_last: Option<[u16; 3]>,
        gpio_timeline: Vec<(u64, [u16; 3])>,
        sniff_left: u64,
        gpio_sniff: Sniff,
        eeprom_sniff: Sniff,
        flag_sniff: Sniff,
        if_ever: u16,
        ie_last: u16,
        ie_changes: Vec<(u64, u16)>,
        if_first: [Option<u64>; 16],
        irq_vector_entries: u64,
        was_halted: bool,
        halt_episodes: u64,
        // DISPSTAT bit3 (V-IRQ enable) is the PPU-side gate for the VBlank IRQ:
        // lcd.rs:237 raises the request only when it is set. Its timeline is
        // the discriminator between "the game never enabled V-IRQs" and "the
        // vendor dropped them".
        dispstat_bit3_ever: bool,
        dispstat_bit3_first: Option<u64>,
        dispstat_bit3_last: bool,
        dispstat_bit3_turns: u64,
        dispstat_bit0_toggles: u64,
        dispstat_bit0_last: bool,
        vcount_min: u16,
        vcount_max: u16,
        irqptr_first: Option<(u64, u32)>,
    }

    impl Forensics {
        fn new(gba: &mut Gba, watch_iwram: bool, watch_gpio: bool, watch_irq: bool, sniff_budget: u64) -> Self {
            let mut iwram_prev = Vec::with_capacity(28);
            for (base, count) in WATCH_WINDOWS {
                for i in 0..count {
                    iwram_prev.push(
                        gba.cpu
                            .bus
                            .internal_memory
                            .try_read_half_word(base + i * 2)
                            .unwrap_or(0),
                    );
                }
            }
            Self {
                global_step: 0,
                watch_iwram,
                watch_gpio,
                watch_irq,
                iwram_prev,
                iwram_agg: BTreeMap::new(),
                iwram_log: Vec::new(),
                gpio_present: false,
                gpio_flip_step: None,
                gpio_last: None,
                gpio_timeline: Vec::new(),
                sniff_left: sniff_budget,
                gpio_sniff: Sniff::default(),
                eeprom_sniff: Sniff::default(),
                flag_sniff: Sniff::default(),
                if_ever: 0,
                ie_last: 0,
                ie_changes: Vec::new(),
                if_first: [None; 16],
                irq_vector_entries: 0,
                was_halted: false,
                halt_episodes: 0,
                dispstat_bit3_ever: false,
                dispstat_bit3_first: None,
                dispstat_bit3_last: false,
                dispstat_bit3_turns: 0,
                dispstat_bit0_toggles: 0,
                dispstat_bit0_last: false,
                vcount_min: u16::MAX,
                vcount_max: 0,
                irqptr_first: None,
            }
        }

        fn observe_step(&mut self, gba: &mut Gba, frame_idx: usize, sniffing: bool) {
            self.global_step += 1;
            let gs = self.global_step;

            if self.watch_iwram {
                let mut idx = 0usize;
                for (base, count) in WATCH_WINDOWS {
                    for widx in 0..count {
                        let addr = base + widx * 2;
                        let i = idx;
                        idx += 1;
                        if let Some(v) = gba.cpu.bus.internal_memory.try_read_half_word(addr) {
                            if v != self.iwram_prev[i] {
                                let old = self.iwram_prev[i];
                                let pc = gba.cpu.registers.program_counter() as u32;
                                let frame = frame_idx as u32;
                                let agg = self
                                    .iwram_agg
                                    .entry(addr as u32)
                                    .or_insert((0, gs, old, v, pc, frame));
                                agg.0 += 1;
                                agg.3 = v;
                                agg.4 = pc;
                                agg.5 = frame;
                                if self.iwram_log.len() < IWRAM_LOG_CAP {
                                    self.iwram_log.push((gs, frame, addr as u32, old, v, pc));
                                }
                                self.iwram_prev[i] = v;
                            }
                        }
                    }
                }
            }

            if self.watch_gpio {
                // The word read returns None exactly when `gpio_present` is
                // true, so the first None is the game's first GPIO write.
                let present = gba.cpu.bus.internal_memory.try_read_word(GPIO_BASE).is_none();
                if present && !self.gpio_present {
                    self.gpio_present = true;
                    self.gpio_flip_step = Some(gs);
                    let vals = [
                        raw16(gba, 0x0800_00C4),
                        raw16(gba, 0x0800_00C6),
                        raw16(gba, 0x0800_00C8),
                    ];
                    self.gpio_last = Some(vals);
                    self.gpio_timeline.push((gs, vals));
                } else if self.gpio_present && gs % 8 == 0 {
                    let vals = [
                        raw16(gba, 0x0800_00C4),
                        raw16(gba, 0x0800_00C6),
                        raw16(gba, 0x0800_00C8),
                    ];
                    if self.gpio_last != Some(vals) && self.gpio_timeline.len() < GPIO_TIMELINE_CAP
                    {
                        self.gpio_timeline.push((gs, vals));
                    }
                    self.gpio_last = Some(vals);
                }
            }

            if self.watch_irq {
                if gs % 4 == 0 {
                    let iff = raw16(gba, 0x0400_0202);
                    self.if_ever |= iff;
                    for bit in 0..16u32 {
                        if iff & (1 << bit) != 0 && self.if_first[bit as usize].is_none() {
                            self.if_first[bit as usize] = Some(gs);
                        }
                    }
                    let dispstat = raw16(gba, 0x0400_0004);
                    let bit3 = dispstat & 0x0008 != 0;
                    if bit3 {
                        if !self.dispstat_bit3_ever {
                            self.dispstat_bit3_ever = true;
                            self.dispstat_bit3_first = Some(gs);
                        }
                    } else if self.dispstat_bit3_last {
                        self.dispstat_bit3_turns += 1;
                    }
                    self.dispstat_bit3_last = bit3;
                    let bit0 = dispstat & 0x0001 != 0;
                    if bit0 != self.dispstat_bit0_last {
                        self.dispstat_bit0_toggles += 1;
                    }
                    self.dispstat_bit0_last = bit0;
                }
                if gs % 64 == 0 {
                    let ie = raw16(gba, 0x0400_0200);
                    if ie != self.ie_last && self.ie_changes.len() < 32 {
                        self.ie_changes.push((gs, ie));
                    }
                    self.ie_last = ie;
                    let vcount = raw16(gba, 0x0400_0006);
                    self.vcount_min = self.vcount_min.min(vcount);
                    self.vcount_max = self.vcount_max.max(vcount);
                    let irqptr = u32::from(raw16(gba, 0x0300_7FFC))
                        | (u32::from(raw16(gba, 0x0300_7FFE)) << 16);
                    if irqptr != 0 && self.irqptr_first.is_none() {
                        self.irqptr_first = Some((gs, irqptr));
                    }
                }
                let pc = gba.cpu.registers.program_counter() as u32;
                if pc < 0x40 {
                    self.irq_vector_entries += 1;
                }
                let halted = gba.cpu.halted;
                if halted && !self.was_halted {
                    self.halt_episodes += 1;
                }
                self.was_halted = halted;
            }

            if sniffing && self.sniff_left > 0 {
                // Pop first, process after: the consumer borrow of the
                // disassembly channel and the register reads cannot overlap.
                let mut popped: Vec<DisasmEntry> = Vec::new();
                if let Some(rx) = gba.disasm_rx.as_mut() {
                    while popped.len() < 4096 {
                        match rx.pop() {
                            Ok(entry) => popped.push(entry),
                            Err(_) => break,
                        }
                    }
                }
                for entry in popped {
                    if self.sniff_left == 0 {
                        break;
                    }
                    self.sniff_left -= 1;
                    let epc = match &entry {
                        DisasmEntry::Arm { pc, .. } | DisasmEntry::Thumb { pc, .. } => *pc,
                    };
                    let mut gpio_hit = None;
                    let mut eeprom_hit = None;
                    let mut flag_hit = None;
                    for i in 0..15usize {
                        let v = gba.cpu.registers.register_at(i) as u32;
                        if (GPIO_SNIFF.0..GPIO_SNIFF.1).contains(&v) {
                            gpio_hit.get_or_insert((i, v));
                        } else if (EEPROM_SNIFF.0..EEPROM_SNIFF.1).contains(&v) {
                            eeprom_hit.get_or_insert((i, v));
                        } else if (FLAG_SNIFF.0..FLAG_SNIFF.1).contains(&v) {
                            flag_hit.get_or_insert((i, v));
                        }
                    }
                    if let Some((reg, val)) = gpio_hit {
                        if self.gpio_sniff.hit(epc, gs) {
                            let text = entry.format();
                            let regs = reg_snapshot(gba, reg, val);
                            self.gpio_sniff.attach(epc, text, regs);
                        }
                    }
                    if let Some((reg, val)) = eeprom_hit {
                        if self.eeprom_sniff.hit(epc, gs) {
                            let text = entry.format();
                            let regs = reg_snapshot(gba, reg, val);
                            self.eeprom_sniff.attach(epc, text, regs);
                        }
                    }
                    if let Some((reg, val)) = flag_hit {
                        if self.flag_sniff.hit(epc, gs) {
                            let text = entry.format();
                            let regs = reg_snapshot(gba, reg, val);
                            self.flag_sniff.attach(epc, text, regs);
                        }
                    }
                }
            }
        }
    }

    /// Register snapshot for a sniff record: the hitting register plus the
    /// low registers that usually carry the rest of the context.
    fn reg_snapshot(gba: &Gba, hit_reg: usize, hit_val: u32) -> String {
        let mut parts: Vec<String> = (0..5)
            .map(|i| format!("r{i}=0x{:08X}", gba.cpu.registers.register_at(i) as u32))
            .collect();
        parts.push(format!("r{hit_reg}=0x{hit_val:08X}"));
        parts.join(" ")
    }

    /// Cycle-neutral halfword read: two `read_raw` byte reads.
    ///
    /// **Why not `bus.read_half_word`:** every `read_*` in the bus family
    /// except `read_raw` charges `access_cycles` into `cycles_count` and
    /// updates `last_used_address` -- and `cycles_count` is what the timers
    /// drain and the LCD catches up on (bus.rs:673, bus.rs:1302-1316). A probe
    /// that polls through the full path therefore **speeds the machine's clock
    /// up by its own sampling rate** and changes what it observes. This was
    /// found the hard way in the first S3-c run: polling IF every 4 steps made
    /// FireRed leave its frozen state (106 distinct hashes) where the untouched
    /// baseline stays frozen -- the observation was changing the observed.
    /// Everything this probe reads per step goes through here.
    fn raw16(gba: &Gba, address: usize) -> u16 {
        u16::from(gba.cpu.bus.read_raw(address))
            | (u16::from(gba.cpu.bus.read_raw(address + 1)) << 8)
    }

    /// FNV-1a over bytes; the battery-diff hash.
    fn fnv64(data: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for b in data {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// Every offset of the little-endian word `needle` in the ROM image.
    fn find_word_literal(rom: &[u8], needle: u32) -> Vec<usize> {
        let bytes = needle.to_le_bytes();
        let mut hits = Vec::new();
        if rom.len() >= 4 {
            for i in 0..=rom.len() - 4 {
                if rom[i..i + 4] == bytes[..] {
                    hits.push(i);
                }
            }
        }
        hits
    }

    /// Thumb `LDR rD, [PC, #imm8*4]` sites (01001 DDD iiiiiiii) whose literal
    /// address `((pc + 4) & !3) + imm*4` is `literal_offset`. Full-ROM scan:
    /// the honest way to find who names a pointer constant.
    fn thumb_ldr_literal_sites(rom: &[u8], literal_offset: usize) -> Vec<(usize, usize)> {
        let mut sites = Vec::new();
        if literal_offset + 4 > rom.len() {
            return sites;
        }
        let off = literal_offset as u32;
        let mut p = 0usize;
        while p + 2 <= rom.len() {
            let h = u16::from_le_bytes([rom[p], rom[p + 1]]);
            if (h & 0xF800) == 0x4800 {
                let imm = u32::from(h & 0xFF);
                let base = (p as u32 + 4) & !3;
                if base + imm * 4 == off {
                    sites.push((p, ((h >> 8) & 7) as usize));
                }
            }
            p += 2;
        }
        sites
    }

    /// ARM `LDR rD, [PC, #imm12]` sites (cond 001 1101 U=1 Rn=1111) whose
    /// literal address `((pc + 8) & !3) + imm12` is `literal_offset`.
    fn arm_ldr_literal_sites(rom: &[u8], literal_offset: usize) -> Vec<(usize, usize)> {
        let mut sites = Vec::new();
        if literal_offset + 4 > rom.len() {
            return sites;
        }
        let off = literal_offset as u32;
        let mut p = 0usize;
        while p + 4 <= rom.len() {
            let w = u32::from_le_bytes([rom[p], rom[p + 1], rom[p + 2], rom[p + 3]]);
            if (w & 0x0F7F_0000) == 0x051F_0000 {
                let imm = w & 0xFFF;
                let base = (p as u32 + 8) & !3;
                if base + imm == off {
                    sites.push((p, ((w >> 12) & 0xF) as usize));
                }
            }
            p += 4;
        }
        sites
    }

    /// One compact state line -- what the wait gates look like at the end.
    /// Reads through `raw16`: this runs after measurement, but the same
    /// cycle-neutrality rule applies everywhere.
    fn compact_state(gba: &Gba) -> String {
        let pc = gba.cpu.registers.program_counter();
        let halted = gba.cpu.halted;
        let dispcnt = raw16(gba, 0x0400_0000);
        let dispstat = raw16(gba, 0x0400_0004);
        let ie = raw16(gba, 0x0400_0200);
        let iff = raw16(gba, 0x0400_0202);
        let ime = raw16(gba, 0x0400_0208);
        let keys = raw16(gba, 0x0400_0130);
        let waitcnt = raw16(gba, 0x0400_0204);
        format!(
            "pc=0x{pc:08X} halted={halted} DISPCNT=0x{dispcnt:04X} DISPSTAT=0x{dispstat:04X} \
             IE=0x{ie:04X} IF=0x{iff:04X} IME=0x{ime:04X} KEYS=0x{keys:04X} WAITCNT=0x{waitcnt:04X}"
        )
    }

    /// The cartprobe frame loop with the forensics observers on every step.
    /// Mirrors `run_frame` otherwise: production audio order, catch_unwind so
    /// a core panic becomes a reading.
    fn run_frame_forensics(
        gba: &mut Gba,
        audio_out: &mut AudioOut,
        samples: &mut [i32],
        hottest: &mut BTreeMap<u32, u32>,
        fo: &mut Forensics,
        frame_idx: usize,
        sniffing: bool,
    ) -> Frame {
        let mut steps: u64 = 0;
        let mut pcs: BTreeMap<u32, u32> = BTreeMap::new();

        install_panic_probe();
        RECORDED_PANIC.with(|slot| *slot.borrow_mut() = None);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            while steps < CYCLE_LIMIT {
                if gba.step() {
                    break;
                }
                steps += 1;
                fo.observe_step(gba, frame_idx, sniffing);
                if steps % 256 == 0 {
                    let pc = gba.cpu.registers.program_counter() as u32;
                    *hottest.entry(pc).or_insert(0) += 1;
                    *pcs.entry(pc).or_insert(0) += 1;
                }
            }
        }));

        if let Err(payload) = outcome {
            let text = RECORDED_PANIC
                .with(|slot| slot.borrow_mut().take())
                .unwrap_or_else(|| panic_text(&payload));
            let top_pc = pcs.into_iter().max_by_key(|&(_, count)| count);
            return Frame::new(steps, steps >= CYCLE_LIMIT, 0, [0u8; 4], 0, top_pc, Some(text));
        }

        audio_out.advance_frame();
        let cap = samples.len() as u32;
        let mut written = 0u32;
        // SAFETY: `samples` is a live slice of `cap` writable i32s, which is
        // what `render` copies into, and `written` is a live local.
        let _ = unsafe { audio_out.render(samples.as_mut_ptr(), cap, &mut written) };

        let top_pc = pcs.into_iter().max_by_key(|&(_, count)| count);
        Frame::new(
            steps,
            steps >= CYCLE_LIMIT,
            0,
            [0u8; 4],
            frame_hash(gba),
            top_pc,
            None,
        )
    }

    /// Print an IF bit list with names and first-fire steps.
    fn print_if_bits(fo: &Forensics) {
        const NAMES: [&str; 16] = [
            "VBlank", "HBlank", "VCount", "Timer0", "Timer1", "Timer2", "Timer3", "SIO",
            "DMA0", "DMA1", "DMA2", "DMA3", "Keypad", "GamePak", "r14", "r15",
        ];
        if fo.if_ever == 0 {
            println!("  IF bits ever seen: none");
            return;
        }
        println!("  IF bits ever seen: 0x{:04X}", fo.if_ever);
        for bit in 0..16u32 {
            if fo.if_ever & (1 << bit) != 0 {
                let first = fo.if_first[bit as usize].map(|s| s.to_string()).unwrap_or("??".into());
                println!("    bit {bit:>2} {:<8} first set at step {first}", NAMES[bit as usize]);
            }
        }
    }

    /// Which single `.gba` the forensics probe should run. The frame.rs
    /// probes own their copy; this module never needed one until now -- same
    /// resolution order (`GBA_PROBE_ROM` first, then the Desktop), so the two
    /// stay consistent by convention.
    fn forensics_rom_path() -> Option<std::path::PathBuf> {
        if let Ok(path) = std::env::var("GBA_PROBE_ROM") {
            let path = std::path::PathBuf::from(path);
            return path.is_file().then_some(path);
        }
        let desktop = std::path::Path::new(&std::env::var("USERPROFILE").unwrap_or_default())
            .join("Desktop");
        std::fs::read_dir(&desktop)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("gba"))
            })
    }

    /// The S3-c forensics probe. `GBA_PROBE_ROM` selects the cartridge; see
    /// the module-level doc above for the observer knobs.
    #[test]
    #[ignore = "S3-c forensics: needs a real .gba via GBA_PROBE_ROM and only prints"]
    fn probe_s3c_forensics() {
        let Some(path) = forensics_rom_path() else {
            println!("no .gba found -- set GBA_PROBE_ROM to a path");
            return;
        };
        let rom = std::fs::read(&path).expect("read the cartridge");
        println!("\n============ S3-C FORENSICS: {} ============", path.display());
        println!("ROM: {} bytes", rom.len());

        let mut gba = Gba::new(bios::stub(), &rom);
        install_swi_hook(&mut gba);
        println!(
            "backup type: {:?} (core-detected)",
            gba.cpu.bus.internal_memory.backup_type()
        );

        // What a non-present GPIO reads back is the ROM image itself (GBATEK:
        // the 6-byte region should be zero-filled).
        let gpio_rom: Vec<u8> = rom.iter().skip(0xC4).take(6).copied().collect();
        println!("ROM image bytes at 0xC4-0xC9: {:02X?}", gpio_rom);

        // The production session loads a sibling .srm; do the same so the game
        // sees its own save.
        let srm = path.with_extension("srm");
        if let Ok(data) = std::fs::read(&srm) {
            let len = data.len();
            gba.cpu.bus.internal_memory.load_battery(&data);
            println!("battery: loaded {len} bytes from {}", srm.display());
        } else {
            println!("battery: no .srm beside the ROM (fresh buffer)");
        }

        // Audio, wired exactly as `Machine::new` and the Qt layer do it.
        let boot_rate = audio::configured_rate();
        let rx = gba.init_audio(boot_rate, audio::RING_SLOTS);
        let mut audio_out = AudioOut::new(boot_rate);
        audio_out.set_volume(audio::configured_volume());
        audio_out.attach(rx, boot_rate);
        let mut samples: Vec<i32> = vec![0; 2 * 5 * (DEVICE_RATE as usize / 60).max(1) + 64];

        let warmup = env_usize("GBA_PROBE_WARMUP", WARMUP_FRAMES);
        let frames = env_usize("GBA_PROBE_FRAMES", MEASURE_FRAMES).max(1);
        let report_every = env_usize("GBA_PROBE_REPORT_EVERY", 60).max(1);
        let watch = std::env::var("GBA_PROBE_WATCH")
            .unwrap_or_else(|_| "iwram,gpio,save,irq".to_string());
        let wants =
            |name: &str| watch.split(',').any(|s| s.trim().eq_ignore_ascii_case(name));
        let watch_iwram = wants("iwram");
        let watch_gpio = wants("gpio");
        let watch_save = wants("save");
        let watch_irq = wants("irq");
        let sniff_requested = wants("disasm");
        let sniff_budget = env_usize("GBA_PROBE_DISASM_STEPS", 20_000_000) as u64;
        // 1-based *global* frame (warm-up included) where sniffing starts.
        let sniff_from = env_usize("GBA_PROBE_DISASM_FROM_FRAME", 1).max(1);
        let strokes = std::env::var("GBA_PROBE_INPUT")
            .map(|spec| parse_input(&spec))
            .unwrap_or_default();
        if !strokes.is_empty() {
            let listed: Vec<String> = strokes
                .iter()
                .map(|stroke| format!("frame {}: {}", stroke.frame, stroke.label))
                .collect();
            println!(
                "input schedule: {} (frames 1-based over the {frames} measured)",
                listed.join("; ")
            );
        }
        println!(
            "window: warm-up {warmup} + measured {frames} frames; watch: iwram={watch_iwram} \
             gpio={watch_gpio} save={watch_save} irq={watch_irq} disasm={sniff_requested} \
             (budget {sniff_budget} instrs from global frame {sniff_from})"
        );

        let _ = crate::gba::swi::take_swi_tallies();
        let mut fo = Forensics::new(&mut gba, watch_iwram, watch_gpio, watch_irq, sniff_budget);
        if sniff_requested {
            gba.cpu.disasm_enabled = true;
        }

        let mut hottest: BTreeMap<u32, u32> = BTreeMap::new();
        for idx in 0..warmup {
            let sniffing = sniff_requested && (idx + 1) >= sniff_from;
            let frame = run_frame_forensics(
                &mut gba, &mut audio_out, &mut samples, &mut hottest, &mut fo, idx + 1, sniffing,
            );
            if let Some(text) = &frame.panic {
                println!("PANIC during warm-up frame {}: {text}", idx + 1);
                println!("-- the cartridge is dead this early; readings end here");
                println!("final: {}", compact_state(&mut gba));
                report_swi_tallies();
                return;
            }
        }

        // The Qt layer reconfigures on the session's first frame; do the same.
        audio_out.set_rate(DEVICE_RATE);
        let rx = gba.init_audio(DEVICE_RATE, audio::RING_SLOTS);
        audio_out.attach(rx, DEVICE_RATE);
        println!(
            "audio: boot rate {boot_rate} -> device rate {DEVICE_RATE}, ring {} slots",
            audio::RING_SLOTS
        );

        let mut hashes: Vec<u64> = Vec::with_capacity(frames);
        let mut steps_all: Vec<u64> = Vec::with_capacity(frames);
        let mut limit_hits = 0usize;
        let mut panics = 0usize;
        let mut dirty_frames: Vec<usize> = Vec::new();
        let mut battery_prev: Option<(u64, Vec<u8>)> = watch_save.then(|| {
            let data = gba.cpu.bus.internal_memory.battery_data().to_vec();
            (fnv64(&data), data)
        });
        // frame -> (byte offset / 8 = 64-bit block for EEPROM, old, new), capped
        let mut battery_changes: Vec<(usize, Vec<(usize, u8, u8)>)> = Vec::new();

        for index in 0..frames {
            let measured_frame = index + 1;
            let global_frame = warmup + measured_frame;
            if let Some(stroke) = strokes.iter().find(|stroke| stroke.frame == measured_frame) {
                apply_buttons(&mut gba, stroke.mask);
            }
            let sniffing =
                sniff_requested && global_frame >= sniff_from && fo.sniff_left > 0;
            let frame = run_frame_forensics(
                &mut gba,
                &mut audio_out,
                &mut samples,
                &mut hottest,
                &mut fo,
                global_frame,
                sniffing,
            );
            if fo.sniff_left == 0 {
                gba.cpu.disasm_enabled = false; // budget spent; stop the pushes
            }
            if let Some(text) = &frame.panic {
                panics += 1;
                println!("\nPANIC during measured frame {measured_frame}: {text}");
                println!("-- the cartridge is stopped here; readings below cover the frames before it");
                break;
            }
            steps_all.push(frame.steps);
            hashes.push(frame.hash);
            if frame.hit_limit {
                limit_hits += 1;
            }

            if watch_save {
                if gba.cpu.bus.internal_memory.take_save_dirty() && dirty_frames.len() < 512 {
                    dirty_frames.push(measured_frame);
                }
                if let Some((prev_hash, prev_data)) = &mut battery_prev {
                    let data = gba.cpu.bus.internal_memory.battery_data();
                    let h = fnv64(data);
                    if h != *prev_hash {
                        let mut diffs: Vec<(usize, u8, u8)> = Vec::new();
                        for (off, (o, n)) in prev_data.iter().zip(data.iter()).enumerate() {
                            if o != n && diffs.len() < 64 {
                                diffs.push((off / 8, *o, *n));
                            }
                        }
                        if battery_changes.len() < 128 {
                            battery_changes.push((measured_frame, diffs));
                        }
                        *prev_hash = h;
                        *prev_data = data.to_vec();
                    }
                }
            }

            if measured_frame % report_every == 0 {
                let distinct = hashes.iter().collect::<std::collections::HashSet<_>>().len();
                let dispcnt = raw16(&gba, 0x0400_0000);
                let keys = raw16(&gba, 0x0400_0130);
                println!(
                    "frame {measured_frame:>5}: steps {:>7} distinct-hashes {:>4} \
                     DISPCNT 0x{dispcnt:04X} KEYS 0x{keys:04X} dirty {} gpio {} halts {}",
                    frame.steps,
                    distinct,
                    dirty_frames.len(),
                    fo.gpio_present,
                    fo.halt_episodes
                );
            }
        }
        println!("audio underruns: {}", audio_out.underruns());
        for stroke in &strokes {
            report_stroke(stroke, &hashes);
        }

        // ---- the forensics report ----
        println!("\n-- IWRAM watch (gMain 0x030030F0-0x310F, IRQ shadow 0x03000000-0x0F, BIOS 0x03007FF8-FF) --");
        if !watch_iwram {
            println!("  (disabled by GBA_PROBE_WATCH)");
        } else if fo.iwram_agg.is_empty() {
            println!("  NO CHANGE in the whole window: nobody wrote 0x030030F0-0x0300310F");
        } else {
            let total: u64 = fo.iwram_agg.values().map(|a| a.0).sum();
            println!(
                "  {} changes across {} addresses (ordered log capped at {IWRAM_LOG_CAP}):",
                total,
                fo.iwram_agg.len()
            );
            println!(
                "    address     count   first@step  first->last   last writer pc (post-step)"
            );
            for (addr, (count, first, fv, lv, pc, _)) in &fo.iwram_agg {
                println!(
                    "    0x{addr:08X}  {count:>7}   {first:>10}  0x{fv:04X}->0x{lv:04X}   0x{pc:08X}"
                );
            }
            if !fo.iwram_log.is_empty() {
                println!("  first events in order:");
                for (step, frame, addr, old, new, pc) in &fo.iwram_log {
                    println!(
                        "    [step {step} frame {frame}] 0x{addr:08X} 0x{old:04X} -> 0x{new:04X}  (pc 0x{pc:08X})"
                    );
                }
            }
        }

        println!("\n-- GPIO --");
        if !watch_gpio {
            println!("  (disabled by GBA_PROBE_WATCH)");
        } else {
            println!(
                "  raw ROM halfwords: 0xC4=0x{:04X} 0xC6=0x{:04X} 0xC8=0x{:04X} \
                 (what a non-present GPIO reads)",
                u16::from_le_bytes([gpio_rom[0], gpio_rom[1]]),
                u16::from_le_bytes([gpio_rom[2], gpio_rom[3]]),
                u16::from_le_bytes([gpio_rom[4], gpio_rom[5]])
            );
            match fo.gpio_flip_step {
                Some(step) => println!(
                    "  gpio_present flipped TRUE at step {step} -- the game's first GPIO write \
                     (detection is exact to the step)"
                ),
                None => println!(
                    "  gpio_present NEVER flipped: the game never wrote 0xC4-0xC9 in this window"
                ),
            }
            for (step, vals) in &fo.gpio_timeline {
                println!(
                    "    step {step:>10}: C4=0x{:04X} C6=0x{:04X} C8=0x{:04X}",
                    vals[0], vals[1], vals[2]
                );
            }
        }

        println!("\n-- disassembly sniff --");
        if !sniff_requested {
            println!("  (disabled by GBA_PROBE_WATCH)");
        } else {
            println!(
                "  budget {} instructions; remaining {}; windows: GPIO {:?} EEPROM {:?} flag {:?}",
                sniff_budget, fo.sniff_left, GPIO_SNIFF, EEPROM_SNIFF, FLAG_SNIFF
            );
            for (name, sniff) in [
                ("GPIO window", &fo.gpio_sniff),
                ("EEPROM window", &fo.eeprom_sniff),
                ("flag region", &fo.flag_sniff),
            ] {
                println!(
                    "  {name}: {} hits, first at step {:?}, {} distinct pc(s)",
                    sniff.total,
                    sniff.first_step,
                    sniff.map.len()
                );
                let mut ranked: Vec<(&u32, &(u64, u64, String, String))> =
                    sniff.map.iter().collect();
                ranked.sort_by_key(|(_, entry)| std::cmp::Reverse(entry.0));
                for (pc, (count, first, text, regs)) in ranked.iter().take(24) {
                    println!("    0x{pc:08X} x{count:<8} first@{first}  {text}");
                    if !regs.is_empty() {
                        println!("        {regs}");
                    }
                }
            }
        }

        println!("\n-- IRQ --");
        if !watch_irq {
            println!("  (disabled by GBA_PROBE_WATCH)");
        } else {
            print_if_bits(&fo);
            if !fo.ie_changes.is_empty() {
                println!("  IE changes (first 32):");
                for (step, ie) in &fo.ie_changes {
                    println!("    step {step:>10}: IE=0x{ie:04X}");
                }
            }
            println!(
                "  steps with PC < 0x40 (IRQ vector entries): {}",
                fo.irq_vector_entries
            );
            println!(
                "  DISPSTAT bit3 (V-IRQ enable): ever set {} (first {:?}, now {}, turned off {} times)",
                fo.dispstat_bit3_ever, fo.dispstat_bit3_first, fo.dispstat_bit3_last, fo.dispstat_bit3_turns
            );
            println!(
                "  DISPSTAT bit0 (VBlank flag) toggles: {} (PPU frame edges the probe saw)",
                fo.dispstat_bit0_toggles
            );
            println!(
                "  VCOUNT range seen: {}..{}   IRQPTR first non-zero: {:?}",
                fo.vcount_min, fo.vcount_max, fo.irqptr_first
            );
            println!("  halt episodes: {}", fo.halt_episodes);
        }

        println!("\n-- save path --");
        if !watch_save {
            println!("  (disabled by GBA_PROBE_WATCH)");
        } else {
            if dirty_frames.is_empty() {
                println!("  save_dirty never set: no save command committed");
            } else {
                let tail: Vec<String> = dirty_frames
                    .iter()
                    .rev()
                    .take(16)
                    .rev()
                    .map(|f| f.to_string())
                    .collect();
                println!(
                    "  save_dirty set in {} frames (first 16 then last 16): {:?} ... [{}]",
                    dirty_frames.len(),
                    &dirty_frames[..dirty_frames.len().min(16)],
                    tail.join(", ")
                );
            }
            if battery_changes.is_empty() {
                println!("  battery bytes never changed");
            } else {
                println!("  battery changed in {} frames (first 16 shown):", battery_changes.len());
                for (frame, diffs) in battery_changes.iter().take(16) {
                    let blocks: Vec<String> = diffs
                        .iter()
                        .take(8)
                        .map(|(block, o, n)| format!("blk{block}:0x{o:02X}->0x{n:02X}"))
                        .collect();
                    println!("    frame {frame}: {} blocks shown: {}", diffs.len(), blocks.join(" "));
                }
            }
        }

        println!("\n-- run --");
        if !steps_all.is_empty() {
            let min = steps_all.iter().copied().min().unwrap_or(0);
            let max = steps_all.iter().copied().max().unwrap_or(0);
            let mut distinct = hashes.clone();
            distinct.sort_unstable();
            distinct.dedup();
            println!(
                "  frames measured {} (limit hits {limit_hits}), steps/frame min {min} max {max} (budget {CYCLE_LIMIT})",
                steps_all.len()
            );
            println!("  distinct frame hashes: {} (1 = frozen picture)", distinct.len());
        }
        println!("  panics: {panics}");
        println!("  hottest PCs:");
        let mut ranked: Vec<(u32, u32)> = hottest.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        for (pc, count) in ranked.iter().take(16) {
            println!("    0x{pc:08X}  {count} samples");
        }

        println!("\n-- static ROM scan (literal pools) --");
        for (label, needle) in [
            ("flag candidate 0x030030FC", 0x0300_30FCu32),
            ("flag candidate 0x0300310C", 0x0300_310C),
            ("flag base 0x030030F0", 0x0300_30F0),
            ("GPIO data 0x080000C4", 0x0800_00C4),
            ("GPIO control 0x080000C8", 0x0800_00C8),
            ("EEPROM window 0x0D000000", 0x0D00_0000),
        ] {
            let hits = find_word_literal(&rom, needle);
            if hits.is_empty() {
                println!("  {label}: no literal in ROM");
                continue;
            }
            let mut line = String::new();
            for off in hits.iter().take(8) {
                line.push_str(&format!(" 0x{:08X}", 0x0800_0000u32.wrapping_add(*off as u32)));
            }
            println!("  {label}: {} literal(s) at{line}", hits.len());
            for off in hits.iter().take(4) {
                for (site, reg) in thumb_ldr_literal_sites(&rom, *off).into_iter().take(6) {
                    println!(
                        "    thumb ldr r{reg},[pc,...] at 0x{:08X}",
                        0x0800_0000u32 + site as u32
                    );
                }
                for (site, reg) in arm_ldr_literal_sites(&rom, *off).into_iter().take(6) {
                    println!(
                        "    arm   ldr r{reg},[pc,...] at 0x{:08X}",
                        0x0800_0000u32 + site as u32
                    );
                }
            }
        }

        println!("\nfinal: {}", compact_state(&mut gba));
        report_swi_tallies();
    }
}