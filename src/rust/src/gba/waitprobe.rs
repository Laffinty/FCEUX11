//! Research-only diagnostic: measure the AGS Aging Cartridge v7.0 wait-state
//! window on this core, and see how far the number is from the cartridge's own
//! expectation.
//!
//! # Why the program is a flat block of words
//!
//! The cartridge runs `sub_800329C` from the stack, because
//! `Test_CallFromStack_ASM` copies the routine there before branching to it —
//! so every instruction fetch inside the measured window is an IWRAM access at
//! zero wait states. Reproducing that with a cartridge-resident program would
//! charge WS0 to the measurement's own instructions and change the number, and
//! reproducing it with a self-copying ROM program turned out to depend on more
//! moving parts than the measurement itself (see the probe notes in
//! `Research_only/gbatech/agswait_probe/`).
//!
//! So the host does the copying instead: the cartridge's first instruction is
//! a branch to IWRAM, and these words are written into IWRAM before the first
//! step. Nothing in the measured window is fetched from the Game Pak, which is
//! the property that makes the number comparable with the AGS table.
//!
//! # The words are not hand-encoded
//!
//! They are the output of `agswait_probe/program.asm`, assembled with
//! FASMARM 1.44, so the block cannot drift from the assembly it claims to be.
//!
//! # What it prints
//!
//! `r0`-`r3` after the program parks: the counter readings for wait-state
//! region 0 (base `0x08000000`), wait-field settings 0-3. The AGS table's first
//! row is `[0x28, 0x24, 0x20, 0x38]`; `0x28` is setting 0 with the most
//! wait states, so the largest reading is the one to compare first.
//!
//! `#[ignore]`d because it prints rather than asserts: a number to compare by
//! hand is a measurement, and a measurement is not a gate.
//!
//! # Status: the WAIT window is localised; the PRESCALER reading is not yet sound
//!
//! The read-back experiment (r4) settled the earlier ambiguity. `WAITCNT` on a
//! fresh machine is `0x0000`, and our stub BIOS *deliberately* programs it to
//! `0x4317` (`bios.rs`, `WAITCNT_DEFAULT` — "the value a manufactured cartridge
//! runs with"). The four read-backs are `0x4017, 0x4017, 0x401F, 0x401F`,
//! which is exactly `0x4317 & 0xF8FF | (setting << 2)` — the ROM's own mask
//! applied to that boot value. **So the program runs as disassembled and
//! `WAITCNT` is not defective.**
//!
//! The obvious suspect for the too-large value the probe read back — `LDRH`
//! leaving the upper half of its destination register — has since been
//! **ruled out** by a direct test in the core
//! (`arm_ldrh_zero_extends_into_the_whole_destination_register`).
//!
//! With the boot value accounted for, the per-read cost is `3 + n_wait` on both
//! sides and the entire difference is a constant: 15 cycles here against 12 on
//! the cartridge. The `N + S` pricing is therefore sound and the whole gap is
//! the window's instruction and I/O cost.
//!
//! **What the prescaler probe found (r6):** bracketing on the **loop counter**
//! rather than on PC, the measurement is clean — and it shows this core
//! spends **twice** the hardware's cycles on pure instruction execution. The
//! `SUBS` + `BNE` loop lives entirely in IWRAM and touches neither the Game Pak
//! nor any wait state, so `access_cycles` is not involved: 8.00 cycles per
//! iteration against the real 4.00, with `master_cycles` and the TM0 reading
//! agreeing in the same 2x direction. That is a separate defect line from the
//! WAIT window's 3-cycle constant, and it is the one that accounts for the
//! AGS `TIMER PRESCALER` failure.
//!
//! # Why the bracketing is on the counter
//!
//! Watching for the PC to reach the instruction *after* the loop fires two
//! steps into the first iteration: the fetch pointer runs ahead speculatively
//! past the `BNE` before the branch redirects, so the post-loop address is
//! visited on **every** pass. That produced a nonsense "12 steps per
//! instruction" ratio which was nearly reported as a 12x slowdown. The counter
//! has no such property: 1023 means the first decrement happened, 0 means the
//! loop finished, whatever the pipeline is doing. **A measurement that brackets
//! on a speculative observation needs the bracket moved before its numbers are
//! believed.**
//!
//! # Two dead ends worth not repeating
//!
//! * **A self-copying ROM program does not work here.** Copying the routine to
//!   IWRAM and branching to it (`iwram_exec.asm` proves `bx` into IWRAM works
//!   on its own) sent the CPU back into the copy loop every time. The host
//!   doing the copy, as here, sidesteps it.
//! * **A plain `b` cannot reach IWRAM from the cartridge.** The displacement is
//!   80 MB and `b` reaches +/-32 MB, so the offset silently does not fit its
//!   24-bit signed field and lands in OAM, where reads are ignored and the
//!   failure looks like a core that cannot execute out of IWRAM. Use
//!   `ldr pc, [pc, #-4]` plus a literal.

#![cfg(test)]

use gba_core::gba::Gba;

/// IWRAM where the measurement block is placed.
const PROGRAM_BASE: usize = 0x0300_0100;

/// The AGS Aging Cartridge v7.0 expectations for wait-state region 0, from
/// `sub_80030E8` in the Normmatt/`ags_aging` disassembly.
const AGS_ROW_0: [u32; 4] = [0x28, 0x24, 0x20, 0x38];

/// The measurement block. Assembled form of `program.asm`.
const PROGRAM: [u32; 65] = [
    0xE3A0B000, 0xE38BB000, 0xE38BB000, 0xE38BB403, 0xE3A0C010, 0xE38CC000, 0xE38CC000,
    0xE38CC403, 0xE3A04004, 0xE3844C02, 0xE3844000, 0xE3844301, 0xE3A05000, 0xE3855C01,
    0xE3855000, 0xE3855301, 0xE3A09000, 0xE3899000, 0xE3899000, 0xE3899302, 0xE3A0A000,
    0xE1D460B0, 0xE3A070FF, 0xE3877B3E, 0xE0067007, 0xE3A08002, 0xE187781A, 0xE1C470B0,
    0xE1D460B0, 0xE78C610A, 0xE3A06000, 0xE5856000, 0xE3A06000, 0xE3866000, 0xE3866502,
    0xE3866000, 0xE5856000, 0xE5990000, 0xE5990000, 0xE5990000, 0xE5990000, 0xE1D500B0,
    0xE3A06000, 0xE5856000, 0xE78B010A, 0xE28AA001, 0xE35A0004, 0x1AFFFFE4, 0xE59B0000,
    0xE59B1004, 0xE59B2008, 0xE59B300C, 0xE59B4010, 0xE59B5014, 0xE59B6018, 0xE59B701C,
    0xE3A08000, 0xE3A09000, 0xE3A0A000, 0xE3A0B000, 0xE3A0C000, 0xE3A0D000, 0xE3A0E000,
    0xE3A0F000, 0xEAFFFFFE,
];

/// A cartridge that branches to [`PROGRAM_BASE`].
///
/// The stub BIOS branches to `08000000h` and lets the cartridge decide, so
/// nothing else in the image is ever executed.
///
/// Not a plain `b`: IWRAM is 80 MB away from `08000000h` and `b` reaches only
/// +/-32 MB, so the displacement does not fit in its 24-bit signed field. An
/// earlier version of this probe used one and landed at `07001000h` -- OAM,
/// where reads are ignored and execution derails. The failure looked exactly
/// like a core that cannot execute out of IWRAM, which `iwram_exec.asm` had
/// already shown it can.
fn cartridge_branches_to_iwram() -> Vec<u8> {
    let mut rom = vec![0u8; 0x200];
    // `ldr pc, [pc, #-4]` reads the literal immediately after it, because ARM
    // forms the PC-relative base as instruction address + 8.
    rom[..4].copy_from_slice(&0xE51F_F004u32.to_le_bytes());
    rom[4..8].copy_from_slice(&(PROGRAM_BASE as u32).to_le_bytes());
    rom
}

#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_the_ags_wait_state_window() {
    let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
    crate::gba::install_swi_hook(&mut gba);

    let before_boot = gba.cpu.bus.read_half_word(0x0400_0204);

    for (index, word) in PROGRAM.iter().enumerate() {
        gba.cpu
            .bus
            .write_word(PROGRAM_BASE + index * 4, *word);
    }

    // Read it again with the program written but not yet started: if this and
    // `before_boot` agree, nothing in the host setup touches WAITCNT, so
    // whatever value the program finds was put there by the boot chain.
    let after_write_before_step = gba.cpu.bus.read_half_word(0x0400_0204);

    // The block parks on a self-branch, so there is no termination point to
    // wait for: step a bounded number of instructions and read the registers.
    for _ in 0..4000 {
        gba.step();
    }

    let measured = [
        gba.cpu.registers.register_at(0),
        gba.cpu.registers.register_at(1),
        gba.cpu.registers.register_at(2),
        gba.cpu.registers.register_at(3),
    ];

    // The results area, read straight out of the bus rather than trusting the
    // program's own register reload: if the reload went wrong, this is the
    // value that actually got stored, and disagreeing with `measured` is
    // itself information.
    let stored: Vec<u32> = (0..4)
        .map(|i| gba.cpu.bus.read_word(0x0300_0000 + i * 4))
        .collect();

    let readback: Vec<u32> = (0..4)
        .map(|i| gba.cpu.bus.read_word(0x0300_0010 + i * 4))
        .collect();

    let pc = gba.cpu.registers.program_counter();
    println!("AGS wait-state region 0, settings 0-3");
    println!("  WAITCNT on a fresh machine          = {before_boot:#06x}");
    println!("  WAITCNT after host setup, pre-step  = {after_write_before_step:#06x}");
    println!("  PC        = {pc:#010x} (in program: {})", (PROGRAM_BASE..PROGRAM_BASE + 0x100).contains(&pc));
    println!("  measured  : {measured:08X?}");
    println!("  stored    : {stored:08X?}");
    println!("  WAITCNT read back after each write: {readback:08X?}");
    println!("  WAITCNT should be                : {:08X?}", [0u32, 4, 8, 12]);
    println!("  AGS expects: {AGS_ROW_0:08X?}");
    for (setting, (got, want)) in measured.iter().zip(AGS_ROW_0.iter()).enumerate() {
        println!(
            "  setting {setting}: measured {got:#04x}, expected {want:#04x}, delta {:+}",
            *got as i64 - *want as i64
        );
    }
    // The program never restores WAITCNT, so whatever it holds now is the
    // last case's value. If that is not `3 << 2`, the writes are not landing
    // where the timing model reads them, and every number above is suspect.
    println!("  WAITCNT now = {:#06x} (expected 0x{:#06x})", gba.cpu.bus.read_half_word(0x0400_0204), 3 << 2);
    println!("  master cycles = {}", gba.cpu.bus.master_cycles());
}

/// The AGS `TIMER PRESCALER` measurement loop, on its own.
///
/// The cartridge times 1024 iterations of `SUBS` + `BNE` and expects **4096** —
/// four cycles per iteration. Nothing in that loop touches the Game Pak or any
/// wait state: it lives in IWRAM, `SUBS` is a register operation and `BNE`
/// branches backwards. So it isolates what this core charges for plain
/// instruction execution, which is the question the three-cycle gap in the
/// WAIT window turns on: is that gap a per-access cost or a per-instruction one?
///
/// Assembled form of `prescaler.asm`.
const PRESCALER: [u32; 43] = [
    0xE3A04000, 0xE3844C01, 0xE3844000, 0xE3844301, 0xE3A06000, 0xE3A08000, 0xE3888C02,
    0xE3888000, 0xE3888301, 0xE5886000, 0xE3A08008, 0xE3888C02, 0xE3888000, 0xE3888301,
    0xE5886000, 0xE5846000, 0xE3A06000, 0xE3866000, 0xE3866502, 0xE3866000, 0xE5846000,
    0xE3A01B01, 0xE3A07000, 0xE2511001, 0x1AFFFFFD, 0xE1D400B0, 0xE3A06000, 0xE5846000,
    0xE3A01B01, 0xE3A02000, 0xE3A03000, 0xE3A05000, 0xE3A06000, 0xE3A07000, 0xE3A08000,
    0xE3A09000, 0xE3A0A000, 0xE3A0B000, 0xE3A0C000, 0xE3A0D000, 0xE3A0E000, 0xE3A0F000,
    0xEAFFFFFE,
];

/// Iterations the AGS prescaler test runs, and the cycles it expects.
const PRESCALER_ITERATIONS: u32 = 1024;
const PRESCALER_EXPECTED: u32 = 4096;

/// `SUBS r1, r1, #1` and the `LDRH r0, [r4]` that follows the loop, by word
/// index in [`PRESCALER`].
///
/// Kept for reference rather than used to bracket the measurement: the fetch
/// pointer visits the post-loop address on *every* pass, because it runs ahead
/// speculatively past the `BNE` before the branch redirects. Bracketing on the
/// counter is immune to that; bracketing on these addresses is not.
const PRESCALER_LOOP_WORD: usize = 23;
const PRESCALER_AFTER_LOOP_WORD: usize = 25;

/// Both constants exist for documentation; the measurement brackets on the
/// counter instead. Silence the unused lint without pretending they are used.
#[allow(dead_code)]
const _: () = {
    let _ = PRESCALER_LOOP_WORD;
    let _ = PRESCALER_AFTER_LOOP_WORD;
};

/// Where the H-BLANK measurement program puts its sample pairs.
///
/// Halfword `2k` is the `DISPSTAT` sample and halfword `2k+1` is the counter
/// reading for sample `k`, which is the layout `sub_8003D38` writes.
const HBLANK_SAMPLES: usize = 456;
const HBLANK_SAMPLE_BASE: usize = 0x0300_0000;
const HBLANK_SAMPLE_END: u32 = (HBLANK_SAMPLE_BASE + HBLANK_SAMPLES * 4) as u32;

/// A transcription of the AGS Aging Cartridge's H-BLANK measurement routine,
/// `sub_8003D38` in `src/sub_8003C88.arm.s`, made position independent so it can
/// be copied into IWRAM and branched to. Assembled form of
/// `agswait_probe/hblankmeas.asm`.
///
/// The program arms TM0 at divide-by-one, polls `DISPSTAT` bit 1 until it
/// changes, reads TM0 at that instant, re-arms, and stores the pair. Running it
/// here and printing the readings puts this core's numbers and the cartridge's
/// expected windows on the same ruler -- which is the one experiment both the
/// leftover `AGS` `TIMER` failures and the `H BLANK STATUS` failure both need.
const HBLANK_MEASURER: [u32; 39] = [
    0xE3A00000, 0xE3800000, 0xE3800000, 0xE3800403, 0xE3A01F72, 0xE3A04000, 0xE3844C01,
    0xE3844000, 0xE3844301, 0xE3A05000, 0xE5845000, 0xE3A06000, 0xE3866000, 0xE3866502,
    0xE3866000, 0xE5846000, 0xE3A07004, 0xE3877000, 0xE3877000, 0xE3877301, 0xE3A08002,
    0xE1A01101, 0xE0811000, 0xE1D720B0, 0xE0023008, 0xE1D790B0, 0xE009A008, 0xE153000A,
    0xE1A0300A, 0x0AFFFFFA, 0xE1D4B0B0, 0xE5845000, 0xE5846000, 0xE0C090B2, 0xE0C0B0B2,
    0xE1500001, 0x1AFFFFF3, 0xE5845000, 0xEAFFFFFE,
];

/// The windows the cartridge's own test demands (`sub_8003A1C`).
const AGS_HBLANK_WHEN_SET: (u16, u16) = (0x3DF, 0x3F1);
const AGS_HBLANK_WHEN_CLEAR: (u16, u16) = (0xD1, 0xE3);

/// Run the AGS H-BLANK measurement on this core and print what it sees.
///
/// Bracketed on **r0**, the program's own writeback pointer, armed by the
/// pointer reaching the end of the buffer -- a register the program itself is
/// driving, so the window is architecturally exact. Not on PC: the fetch
/// pointer runs ahead speculatively, which is the third time that trap has cost
/// this project a measurement.
/// Measure the scanline periods by polling `DISPSTAT` from the host.
///
/// # Why this replaced the AGS-routine probe
///
/// The transcription of `sub_8003D38` measured 57536-60159 cycles and split
/// its samples 11 / 445, which is nowhere near this core's 960 / 272 geometry.
/// A measurement that disagrees with the thing it is measuring is a broken
/// ruler, so that version is not the one to iterate on.
///
/// This one needs no measurement program at all. Each turn of the loop is one
/// `gba.step()` (one instruction, hence one master cycle) plus one
/// `read_half_word` of an I/O register (`access_cycles` returns 1 for the
/// 32-bit-bus no-wait regions), so **the loop advances the master clock by
/// exactly 2 and that is known rather than fitted**. The CPU only has to be
/// doing something, because the LCD advances off the master clock and not off
/// instruction retires.
///
/// Resolution is therefore 2 cycles, which is far finer than either period, and
/// nothing has to be calibrated after the fact.
#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_the_scanline_periods_by_polling_dispstat() {
    let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
    crate::gba::install_swi_hook(&mut gba);

    // Park the CPU so it cannot interfere; the LCD runs off the master clock.
    gba.cpu
        .bus
        .write_word(PROGRAM_BASE, 0xEAFF_FFFE); // b .

    const DISPSTAT: usize = 0x0400_0004;
    let mut last = gba.cpu.bus.read_half_word(DISPSTAT) & 2;
    let mut previous_at = gba.cpu.bus.master_cycles();
    let mut active: Vec<u64> = Vec::new();
    let mut blank: Vec<u64> = Vec::new();

    for _ in 0..2_000_000 {
        gba.step();
        let now = gba.cpu.bus.master_cycles();
        let bit = gba.cpu.bus.read_half_word(DISPSTAT) & 2;
        if bit != last {
            // The interval that just ended: bit set means the active period
            // finished, matching the cartridge's own classification.
            let span = now - previous_at;
            if bit != 0 {
                active.push(span);
            } else {
                blank.push(span);
            }
            previous_at = now;
            last = bit;
            if active.len() >= 20 && blank.len() >= 20 {
                break;
            }
        }
    }

    let show = |name: &str, v: &[u64]| {
        let mut d = v.to_vec();
        d.sort_unstable();
        d.dedup();
        println!("  {name}: n={} distinct={d:?}", v.len());
    };
    println!("scanline periods, sampled from the host (2 cycles per sample)");
    show("entered H-BLANK (active period)", &active);
    show("left H-BLANK (h-blank period)", &blank);
    println!("  geometry predicts 960 active and 272 h-blank");
    println!("  AGS wants the *measured* windows, which include the cartridge's");
    println!("  own read-back cost: active [0x3df,0x3f1], h-blank [0xd1,0xe3]");
}

#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_the_ags_hblank_periods() {
    let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
    crate::gba::install_swi_hook(&mut gba);

    for (index, word) in HBLANK_MEASURER.iter().enumerate() {
        gba.cpu
            .bus
            .write_word(PROGRAM_BASE + index * 4, *word);
    }

    let mut armed = false;
    for _ in 0..4_000_000 {
        if gba.cpu.registers.register_at(0) >= HBLANK_SAMPLE_END {
            armed = true;
            break;
        }
        gba.step();
    }
    assert!(armed, "the measurement never filled its sample buffer");

    // Sample pairs: the DISPSTAT sample, then the counter reading.
    let mut when_set: Vec<u32> = Vec::new();
    let mut when_clear: Vec<u32> = Vec::new();
    for k in 0..HBLANK_SAMPLES {
        let at = HBLANK_SAMPLE_BASE + k * 4;
        let dispstat = gba.cpu.bus.read_half_word(at) as u32;
        let counter = gba.cpu.bus.read_half_word(at + 2) as u32;
        if dispstat & 2 != 0 {
            when_set.push(counter);
        } else {
            when_clear.push(counter);
        }
    }

    let summarise = |name: &str, values: &[u32], window: (u16, u16)| {
        let mut distinct = values.to_vec();
        distinct.sort_unstable();
        distinct.dedup();
        let inside = values
            .iter()
            .filter(|v| **v >= u32::from(window.0) && **v <= u32::from(window.1))
            .count();
        println!("  {name}: n={} distinct={distinct:?}", values.len());
        println!(
            "    AGS window [{:#05x}, {:#05x}] -> {inside}/{} inside",
            window.0, window.1, values.len()
        );
    };

    println!("AGS H-BLANK STATUS measurement, run on this core");
    summarise("entered H-BLANK (active period)", &when_set, AGS_HBLANK_WHEN_SET);
    summarise("left H-BLANK (h-blank period)", &when_clear, AGS_HBLANK_WHEN_CLEAR);
    println!("  our geometry: 240 visible + 68 h-blank dots at 4 cycles each");
    println!("   => 960 active, 272 h-blank, 228 lines, 280896 cycles per frame");
}

#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_the_ags_prescaler_loop() {
    let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
    crate::gba::install_swi_hook(&mut gba);

    for (index, word) in PRESCALER.iter().enumerate() {
        gba.cpu
            .bus
            .write_word(PROGRAM_BASE + index * 4, *word);
    }

    // Three observables, so that "the loop is slow" and "the boot ate the
    // steps" cannot be confused for one another again.
    //
    // Bracketing is done on the **loop counter**, not on PC. An earlier
    // version watched for the PC to reach the instruction after the loop and
    // fired two steps into the first iteration: the fetch pointer runs ahead
    // speculatively past the `BNE` before the branch redirects, so the address
    // after the loop is visited on every pass. The counter has no such
    // property -- 1023 means the first decrement happened and 0 means the loop
    // is finished, whatever the pipeline is doing.
    let mut entry: Option<(usize, u64)> = None;
    let mut exit: Option<(usize, u64)> = None;
    let mut boot_steps = None;

    for step in 0..200_000 {
        let pc = gba.cpu.registers.program_counter();
        if boot_steps.is_none() && pc == PROGRAM_BASE {
            boot_steps = Some(step);
        }
        let counter = gba.cpu.registers.register_at(1);
        if entry.is_none() && counter == PRESCALER_ITERATIONS - 1 {
            entry = Some((step, gba.cpu.bus.master_cycles()));
        }
        if entry.is_some() && exit.is_none() && counter == 0 {
            exit = Some((step, gba.cpu.bus.master_cycles()));
            // A few more instructions so the `LDRH` actually executes.
            for _ in 0..8 {
                gba.step();
            }
            break;
        }
        gba.step();
    }

    let measured = gba.cpu.registers.register_at(0);
    let loop_counter = gba.cpu.registers.register_at(1);
    let pc = gba.cpu.registers.program_counter();

    println!("AGS TIMER PRESCALER loop");
    println!("  PC        = {pc:#010x}");
    println!("  boot reached the program at step {boot_steps:?} (0-based; the stub BIOS before it)");
    println!("  loop counter r1 = {loop_counter} (0 means the loop finished)");
    match (entry, exit) {
        (Some((entry_step, entry_cycles)), Some((exit_step, exit_cycles))) => {
            println!("  loop entry  : step {entry_step}, master_cycles {entry_cycles}");
            println!("  loop exit   : step {exit_step}, master_cycles {exit_cycles}");
            println!(
                "  steps in loop       = {} (expected ~{})",
                exit_step - entry_step,
                PRESCALER_ITERATIONS as usize * 2 - 1
            );
            println!(
                "  steps per iteration = {:.2} (one step should be one instruction)",
                (exit_step - entry_step) as f64 / (PRESCALER_ITERATIONS - 1) as f64
            );
            println!(
                "  master_cycles delta = {} (expected ~{PRESCALER_EXPECTED})",
                exit_cycles - entry_cycles
            );
        }
        (entry, exit) => println!(
            "  loop not bracketed: entry={} exit={} (budget or path problem)",
            entry.is_some(),
            exit.is_some()
        ),
    }
    println!("  TM0 reading = {measured} (AGS expects {PRESCALER_EXPECTED})");
    println!(
        "  per iteration = {:.2} cycles (AGS: {:.2})",
        f64::from(measured) / f64::from(PRESCALER_ITERATIONS),
        f64::from(PRESCALER_EXPECTED) / f64::from(PRESCALER_ITERATIONS)
    );
    println!(
        "  delta vs AGS = {:+}",
        measured as i64 - PRESCALER_EXPECTED as i64
    );
}
