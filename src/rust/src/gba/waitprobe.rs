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

use gba_core::bus::Bus;
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
/// `mov r1, #1024` + `SUBS`/`BNE` loop + `LDRH r0, [r4]` + an end marker.
///
/// Assembled form of `agswait_probe/divtest.asm`. The host arms TM0 and leaves
/// `r4 = 0x04000100`; it recognises the end by `r7`, which this program writes
/// exactly once and only when it is done.
const DIVTEST: [u32; 10] = [
    0xE3A04000, 0xE3844C01, 0xE3844000, 0xE3844301, 0xE3A01B01, 0xE2511001, 0x1AFFFFFD,
    0xE1D400B0, 0xE3A07C7F, 0xEAFFFFFE,
];

/// What the AGS `TIMER PRESCALER` test expects, by prescaler code.
///
/// One test covers all four: a fixed small offset is invisible at 4096 and fatal
/// at 4, which is the whole reason the `÷1` result being right did not make the
/// AGS row green.
const AGS_PRESCALER_EXPECTED: [u32; 4] = [4096, 64, 16, 4];
const DIVIDER_NAMES: [&str; 4] = ["÷1", "÷64", "÷256", "÷1024"];

/// Read the AGS `TIMER PRESCALER` case for each prescaler.
///
/// The timer is armed from the host (`0x0080_0000 | prescaler` to TM0CNT, which
/// sets divide-by-one plus enable in the high half and the prescaler code plus
/// a zero counter in the low half), the loop runs 1024 iterations of
/// `SUBS` + `BNE`, and the result comes back in `r0`.
#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_every_ags_prescaler_case() {
    const TM0: usize = 0x0400_0100;

    println!("AGS TIMER PRESCALER, one prescaler at a time (1024 iterations each)");
    for (prescaler, expected) in AGS_PRESCALER_EXPECTED.iter().enumerate() {
        let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
        crate::gba::install_swi_hook(&mut gba);

        for (index, word) in DIVTEST.iter().enumerate() {
            gba.cpu.bus.write_word(PROGRAM_BASE + index * 4, *word);
        }

        gba.cpu.bus.write_word(TM0, 0); // counter 0, stopped
        // The divider belongs in `TMxCNT_H` bits 0-1, so it has to travel in the
        // high half — `(j << 16) | 0x800000`, which is exactly what the AGS
        // routine writes. This used to be `| prescaler`, which put the code in
        // the *low* half: that is the counter/reload, so every case armed
        // `TMxCNT_H = 0x0080` (divide by one) and the only thing that varied was
        // where the counter started. The table it produced, {4147, 4148, 4149,
        // 4150}, was an artifact of that and never exercised three of the four
        // dividers — see plan r21.
        gba.cpu
            .bus
            .write_word(TM0, 0x0080_0000 | ((prescaler as u32) << 16));

        let mut done = false;
        for _ in 0..200_000 {
            if gba.cpu.registers.register_at(7) == 0x7F00 {
                done = true;
                break;
            }
            gba.step();
        }
        assert!(done, "{}: the loop never finished", DIVIDER_NAMES[prescaler]);

        let read = gba.cpu.registers.register_at(0);
        println!(
            "  {:<6} expected {:<5} measured {:<5} delta {:+}",
            DIVIDER_NAMES[prescaler],
            expected,
            read,
            read as i64 - *expected as i64
        );
    }
    println!("  (only the divide-by-one case is inside a tolerance that hides an");
    println!("   offset; the other three are single-digit numbers)");
}

/// Does the counter start moving on the enable edge, or is there a lag?
///
/// Plan r19 says an enable edge should take ~2 cycles before the counter moves,
/// and calibrates that constant by requiring the divide-by-one AGS case to come
/// out at exactly 4096. **That is a constant fitted to a single anchor** — the
/// same shape as the `-4` constant r12 retracted for being fitted. So this probe
/// measures the lag instead of fitting it: sample TM0 every cycle from the edge
/// and read the offset straight off the table.
///
/// Both columns are printed, so the answer needs no prediction at all. If
/// counting begins on the edge the counter tracks the clock and the lag column
/// is a constant; if it begins late, the lag column is that constant minus the
/// delay, and the constant is whatever the slope implies. Either way the number
/// is read off, not assumed.
///
/// The edge is at a *known* clock value: `write_word` charges `access_cycles`
/// before it stores, so the enabling byte is written on the last of the charged
/// cycles and `master_cycles` sampled straight after the call **is** the edge.
/// Nothing here is hand-encoded — the program is the assembled `divtest.asm`
/// block already in this file, and this case arms from the host because that is
/// the only shape in which the edge lands on a clock value I can name. (The bus
/// drains timers at instruction boundaries, so a store from a program and a
/// store from the host are the same event to this model.)
#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn observe_when_the_timer_starts_counting() {
    const TM0: usize = 0x0400_0100;
    const SAMPLES: usize = 24;

    let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
    crate::gba::install_swi_hook(&mut gba);

    for (index, word) in DIVTEST.iter().enumerate() {
        gba.cpu.bus.write_word(PROGRAM_BASE + index * 4, *word);
    }

    gba.cpu.bus.write_word(TM0, 0); // counter 0, stopped

    // Drain the host's own setup writes while the timer is still stopped. Left
    // in, they land in the same `timers.step` batch as the first real sample and
    // show up as a bogus lag on sample 0.
    for _ in 0..4 {
        gba.step();
    }

    gba.cpu.bus.write_word(TM0, 0x0080_0000); // enable + divide by one
    let edge = gba.cpu.bus.master_cycles();

    println!("TM0 sampled every cycle from the enable edge (divide by one)");
    println!("    sample   cycles since edge   TM0    lag");
    for sample in 0..SAMPLES {
        gba.step();
        let cycles = gba.cpu.bus.master_cycles() - edge;
        let counter = u64::from(gba.cpu.bus.read_half_word(TM0));
        println!(
            "    {sample:>6}   {cycles:>17}   {counter:>4}   {:>5}",
            cycles as i64 - counter as i64
        );
    }
    println!("  a constant lag column means counting starts immediately;");
    println!("  a lag that never reaches zero means the counter starts late");
}

/// `sub_8009294` — the AGS `TIMER CONNECT` routine — **as the machine code
/// actually present in the cartridge**, not re-encoded.
///
/// Read from `Research_only/gbatech/ags.gba` (sha1 `5c73fb40…`), ROM offset
/// `0x9294`, 26 words up to and including the PC-relative literal at `+0x64`.
/// It is copied verbatim because the routine's `ldr r4, [pc, #0x54]` reaches
/// that literal 0x5c bytes past the instruction, so every word has to keep its
/// relative position; re-encoding would put a different value on the bus for the
/// sake of a routine that already exists in a form we can read.
///
/// The three assertions are what make this trustworthy rather than a pile of
/// hex: they pin the parts the measurement turns on — TM3 enabled first while
/// its reload is still 0, TM0 enabled last, the two `mov r0, r0` between the
/// loop and the read, and the read being off TM3.
const AGS_CONNECT: [u32; 26] = [
    0xE82D4FF0, 0xE3A02B01, 0xE59F4054, 0xE3A0B000, 0xE584B000, 0xE584B004, 0xE584B008, 0xE584B00C,
    0xE3A05721, 0xE584500C, 0xE1855000, 0xE5845008, 0xE5845004, 0xE5845000, 0xE2522001, 0x1AFFFFFD,
    0xE1A00000, 0xE1A00000, 0xE1D400BC, 0xE584B000, 0xE584B004, 0xE584B008, 0xE584B00C, 0xE9BD4FF0,
    0xE12FFF1E, 0x04000100,
];

#[test]
fn the_extracted_connect_routine_is_the_one_the_plan_describes() {
    // TM3 is enabled first, while its reload is still 0 — that ordering is the
    // whole reason TM3 can read 512 at all.
    assert_eq!(AGS_CONNECT[9], 0xE584_500C, "TM3 = 0x00840000");
    assert_eq!(AGS_CONNECT[7], 0xE584_B00C, "the zeroing pass reaches TM3 too");
    // TM0 is enabled last, so its own counting window is the full loop.
    assert_eq!(AGS_CONNECT[13], 0xE584_5000, "TM0 = 0x0084FFFE");
    // Two no-ops between the loop and the read, and the read is off TM3.
    assert_eq!(AGS_CONNECT[16], 0xE1A0_0000);
    assert_eq!(AGS_CONNECT[17], 0xE1A0_0000);
    assert_eq!(AGS_CONNECT[18], 0xE1D4_00BC, "ldrh r0, [r4, #0xc]");
    assert_eq!(AGS_CONNECT[25], 0x0400_0100, "the PC-relative literal");
}

/// Run the real routine and read what it reads.
///
/// The wrapper passes `65534` as the routine's first argument, and the stub BIOS
/// clobbers `r0` long before the routine starts, so it cannot simply be preset
/// — but `mov r2, #0x400` is the routine's second instruction, and a host-side
/// register write **costs no clock cycles**. Watching `r2` and writing `r0` the
/// moment it reads 0x400 therefore costs nothing measurable, and the `orr` that
/// needs it is still eight instructions away.
///
/// Exit is bracketed on the loop counter, never on PC: the fetch pointer runs
/// ahead speculatively past the `bne`. `r2 == 0` on its own could be true at step
/// zero, so the run is armed by first seeing the program write 0x400.
#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn run_the_real_ags_connect_routine() {
    let mut gba = Gba::new(crate::gba::bios::stub(), &cartridge_branches_to_iwram());
    crate::gba::install_swi_hook(&mut gba);

    for (index, word) in AGS_CONNECT.iter().enumerate() {
        gba.cpu.bus.write_word(PROGRAM_BASE + index * 4, *word);
    }

    let mut armed = false;
    let mut supplied_argument = false;
    let mut loop_done_at = 0u64;
    for step in 0..400_000u64 {
        let r2 = gba.cpu.registers.register_at(2);
        if !armed {
            if r2 == 0x400 {
                armed = true;
            }
        } else {
            if r2 == 0x400 && !supplied_argument {
                gba.cpu.registers.set_register_at(0, 65534);
                supplied_argument = true;
            }
            if r2 == 0 {
                loop_done_at = step;
                break;
            }
        }
        gba.step();
    }
    assert!(armed, "the routine never started");
    assert!(supplied_argument, "missed the window to supply the argument");

    // The falling-through `bne`, the two no-ops and the `ldrh` itself.
    let mut reading = 0xFFFF;
    for _ in 0..16 {
        gba.step();
        reading = gba.cpu.registers.register_at(0);
        if reading != 65534 {
            break;
        }
    }

    println!("AGS TIMER CONNECT, real machine code from ags.gba");
    println!("  loop finished at step {loop_done_at}");
    println!("  TM3 read by the routine = {}   (AGS expects 512)", reading);
    println!("  TIMER CONNECT: {}", if reading == 512 { "PASS" } else { "FAIL" });
}

/// `Test_CallFromStack_ASM` — the wrapper every AGS timing test goes through —
    /// **as the machine code in the cartridge**, not re-encoded. THUMB, 28
    /// halfwords from ROM offset `0xF150`, ending with the 32-bit literal that
    /// holds its own return address.
    ///
    /// Read from `ags.gba` (sha1 `5c73fb40…`), cross-checked against
    /// `asm/sub_800F150.s`. The assertion below is what makes it usable as an
    /// input: `blx sp` is the instruction that matters — it is the edge into the
    /// stack copy, and it also plants the return address in `lr` for the routine
    /// to come back through.
    const AGS_CALL_FROM_STACK: [u16; 28] = [
        0xB590, 0xB082, 0x466F, 0x603A, 0x607B, 0x1A0C, 0x466A, 0x1B12, 0x4695, 0x2100, 0x5843,
        0x5053, 0x3104, 0x42A1, 0xD1FA, 0x4805, 0x4686, 0x6838, 0x6879, 0x4768, 0x44A5, 0xB002,
        0xBC90, 0xBC02, 0x4708, 0x0000, 0xF179, 0x0800,
    ];

/// Run the real wrapper around the real routine — the full path every AGS
    /// timing test actually takes.
    ///
    /// ⚠️ **This probe does not work yet; it asserts rather than reporting.** The
    /// routine on its own reads 512 (see `run_the_real_ags_connect_routine`) and
    /// the cartridge still reports `TIMER CONNECT` failing, so the wrapper is the
    /// one link that has been reasoned about but never actually executed here.
    /// Getting it to run means putting four arguments into a **Thumb** routine
    /// entered through an odd `ldr pc`, and that has not been made to work: the
    /// four `write_half_word` calls land (the readback proves it), the PC does
    /// reach the wrapper and later the routine, but the wrapper's `push` has not
    /// been observed to move `sp` at the point the arguments have to be supplied,
    /// so the window is missed.
    ///
    /// It is kept, and it fails loudly, because a probe that quietly prints
    /// "FAIL" for a run that never reached the routine would be indistinguishable
    /// from a hardware verdict — the exact confusion this project keeps paying
    /// for. Fixing it means running the wrapper from ROM as a real cartridge
    /// would, or instrumenting `sp` from inside rather than watching it from out.
    #[test]
    #[ignore = "prints a measurement to compare by hand; see the module docs"]
fn run_the_real_wrapper_around_the_real_connect_routine() {
    const WRAPPER: usize = 0x0300_0200;
    const ROUTINE: usize = 0x0300_0400;
    const ROUTINE_BYTES: usize = 0x68;

    let mut rom = vec![0u8; 0x200];
    rom[..4].copy_from_slice(&0xE51F_F004u32.to_le_bytes());
    // Odd address: entering Thumb state is what the wrapper is compiled as.
    rom[4..8].copy_from_slice(&((WRAPPER as u32) | 1).to_le_bytes());

    let mut gba = Gba::new(crate::gba::bios::stub(), &rom);
    crate::gba::install_swi_hook(&mut gba);

    for (index, half) in AGS_CALL_FROM_STACK.iter().enumerate() {
        gba.cpu.bus.write_half_word(WRAPPER + index * 2, *half);
    }
    // Read back what actually landed: a silent failure here looks exactly like a
    // CPU that cannot execute Thumb, which is the kind of false conclusion this
    // project has already paid for once (r4 ④'s `b`-to-IWRAM trap).
    let readback: Vec<u16> = (0..6)
        .map(|i| gba.cpu.bus.read_half_word(WRAPPER + i * 2))
        .collect();
    println!("  wrapper halfwords read back: {readback:04X?}");
    for (index, word) in AGS_CONNECT.iter().enumerate() {
        gba.cpu.bus.write_word(ROUTINE + index * 4, *word);
    }

    let boot_sp = gba.cpu.registers.register_at(13);
    let mut injected = false;
    let mut injected_at: Option<u64> = None;
    let mut seen_argument = false;
    let mut reading = u32::MAX;
    let mut step = 0;

    while step < 400_000 {
        step += 1;
        if !injected && gba.cpu.registers.register_at(13) != boot_sp {
            gba.cpu.registers.set_register_at(0, ROUTINE as u32);
            gba.cpu
                .registers
                .set_register_at(1, (ROUTINE + ROUTINE_BYTES) as u32);
            gba.cpu.registers.set_register_at(2, 65534);
            gba.cpu.registers.set_register_at(3, 0);
            injected = true;
            injected_at = Some(step);
        }
        gba.step();

        let r0 = gba.cpu.registers.register_at(0);
        // `ldr r0, [r7]` restores the first argument just before the branch, so
        // seeing it means the wrapper really took its path.
        if r0 == 65534 {
            seen_argument = true;
        } else if seen_argument && reading == u32::MAX {
            reading = r0;
            break;
        }
    }

    // Refuse to print a verdict this probe did not earn. Getting the four
    // arguments into a Thumb routine entered through an odd `ldr pc` has not
    // worked reliably yet, and a run that silently reports "FAIL" here would be
    // indistinguishable from a real hardware verdict — which is the one thing
    // this probe exists to avoid.
    assert!(
        injected && seen_argument && reading != u32::MAX,
        "the probe did not reach the routine: injected={injected} at {injected_at:?}, \
         argument seen={seen_argument}, reading={reading:#x}"
    );
    println!("AGS TIMER CONNECT through the real Test_CallFromStack_ASM");
    println!("  TM3 read by the routine = {reading}   (AGS expects 512)");
    println!(
        "  TIMER CONNECT: {}",
        if reading == 512 { "PASS" } else { "FAIL" }
    );
}

#[test]
fn the_extracted_wrapper_matches_the_source_it_claims_to_be() {
    assert_eq!(AGS_CALL_FROM_STACK[0], 0xB590, "push {{r4, r7, lr}}");
    assert_eq!(AGS_CALL_FROM_STACK[3], 0x603A, "str r2, [{{r7}}]");
    assert_eq!(AGS_CALL_FROM_STACK[4], 0x607B, "str r3, [{{r7, #4}}]");
    assert_eq!(AGS_CALL_FROM_STACK[19], 0x4768, "blx sp — the edge into the copy");
    // The 32-bit literal at the end is the wrapper's own return address.
    assert_eq!(
        u32::from(AGS_CALL_FROM_STACK[26]) | (u32::from(AGS_CALL_FROM_STACK[27]) << 16),
        0x0800_F179
    );
}

/// Research-only probe: what does this core charge for a DMA right now?
///
/// The AGS memory tests time `TimeDmaToAndFromMemory` against the cartridge's
/// own expectation, and P1 wants that number adjusted. This measures the raw
/// per-transfer cost for each region pair so the gap can be compared with the
/// model rather than guessed at.
///
/// Immediate (timing 0) DMA runs synchronously on the control write, so the
/// master-clock delta across that one write is the whole cost of the block.
///
/// `#[ignore]`d because it prints: a number to compare by hand is a
/// measurement, and a measurement is not a gate.
#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_what_a_dma_costs_in_each_region_pair() {
    const REGIONS: [(&str, u32); 6] = [
        ("EWRAM", 0x0200_0000),
        ("IWRAM", 0x0300_0000),
        ("PALETTE", 0x0500_0000),
        ("VRAM", 0x0600_0000),
        ("OAM", 0x0700_0000),
        ("ROM", 0x0800_0000),
    ];
    // The DMA count register holds N-1, so this is exactly 0x400 units.
    const UNITS: u32 = 0x400;
    const COUNT: u16 = UNITS as u16 - 1;

    println!("DMA cost on this core: master cycles for a 0x400-unit block");
    println!("    src -> dst");
    for (src_name, src) in REGIONS {
        for (dst_name, dst) in REGIONS {
            let mut bus = Bus::default();
            bus.write_word(0x0400_00B0, src); // DMA0 source
            bus.write_word(0x0400_00B4, dst); // DMA0 destination
            bus.write_half_word(0x0400_00B8, COUNT); // count is N-1
            // Control halfword is the one that starts an immediate transfer:
            // enable (15), timing 0 (immediate), 32-bit (10).
            let before = bus.master_cycles();
            bus.write_half_word(0x0400_00BA, (1 << 15) | (1 << 10));
            let after = bus.master_cycles();

            let per_unit = (after - before) as f64 / f64::from(UNITS);
            println!(
                "  {src_name:>7} -> {dst_name:<7} total {:>6}  per unit {per_unit:.3}",
                after - before
            );
        }
    }
    println!("  (mGBA's model, plan section 5: a fixed +3 per transfer start,");
    println!("   then each unit priced by its own source and destination waits)");
}

/// The AGS cartridge image, loaded at run time.
///
/// Deliberately not `include_bytes!`: that would embed a 4 MB commercial ROM in
/// the test binary, and the image is gitignored research material that only this
/// `#[ignore]`d probe needs.
fn load_ags_rom() -> Vec<u8> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../Research_only/gbatech/ags.gba"
    );
    std::fs::read(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"))
}

/// Read the AGS `PREFETCH BUFFER` case on this core, by running the cartridge's
/// own routine.
///
/// # Status (r34): working
///
/// The head below is the reason the first three attempts never reached the
/// routine — it was written as the **Thumb** halfwords `0x4801`/`0x4700` while
/// the cartridge is entered in ARM mode, so the machine consumed one
/// conditional ARM instruction and derailed. With the ARM encodings it runs,
/// and on the fixed core it reads **24 / 51**, the cartridge's own expected
/// values, for the `0x4014` / `0x0014` WAITCNT cases. The root-crate gate
/// equivalent is `the_ags_prefetch_window_reads_24_buffered_and_51_unbuffered`
/// in `gba-core`, which replays the same routine from embedded bytes.
///
/// The routine is not re-created here. `ags.gba` already contains it at
/// `0x0800326C`, in the GamePak, where its own fetches cost real wait states —
/// which is the whole point of the test. Only the cartridge head is replaced,
/// so the BIOS hands control to that address instead of to the AGS's own entry.
///
/// Its shape (literal pool read from the ROM, not hand-computed): enable TM0
/// divide-by-one, do eight dummy reads of the timer register, then read the
/// counter — so the reading is how many cycles those eight I/O reads took.
///
/// The result comes back in `r0`, and the routine leaves the timer stopped
/// behind it. `r0` is watched rather than sampled at the end because the CPU
/// carries on into the rest of the cartridge afterwards and would overwrite it.
/// Watching a register costs no cycles; reading TM0 through the bus would cost
/// one per sample and perturb the very thing being measured (r13's lesson).
///
/// The per-step `(pc, delta, cumulative)` trace is the decomposition record:
/// it is what localised the last three cycles to the enable step's
/// speculative fetch when the buffered model was being chosen (r34).
#[test]
#[ignore = "prints a measurement to compare by hand; see the module docs"]
fn measure_the_ags_prefetch_case_under_both_waitcnt_settings() {
    const ROUTINE: u32 = 0x0800_3011; // Thumb entry of sub_8003010
    const TM0CNT: usize = 0x0400_0100;

    // Run the cartridge's own test rather than re-creating its two cases.
    // `sub_8003010` programs WAITCNT itself between the two measurements, and
    // asking the host to write WAITCNT before the first step does not work: the
    // stub BIOS overwrites it during boot, which showed up as both cases
    // measuring identically.
    let mut rom = load_ags_rom();
    // ARM head that interworks: `ldr r0, [pc, #0]` / `bx r0` / literal holding
    // the odd routine address. A plain `ldr pc, [pc, #-4]` cannot be used -- this
    // core's LDR PC does not switch processor state, so a Thumb entry point
    // loaded that way is executed as ARM and the run derails immediately
    // (observed: final PC 0x0000000C).
    //
    // Why the previous three attempts never reached the routine: the head was
    // written as the **Thumb** halfwords 0x4801/0x4700 while execution enters
    // the cartridge in ARM mode, so the machine consumed 0x47004801 as one
    // conditional ARM instruction and derailed. Both words below are the ARM
    // encodings, and each was verified by disassembling this exact image
    // (`armdis.py head_check.bin 0 0xC arm`): `ldr r0, [pc]` / `bx r0`, with
    // the literal landing at 0x08000008 where `[pc]` reads it.
    rom[0..4].copy_from_slice(&0xE59F_0000u32.to_le_bytes()); // ldr r0, [pc, #0]
    rom[4..8].copy_from_slice(&0xE12F_FF10u32.to_le_bytes()); // bx r0
    rom[8..12].copy_from_slice(&ROUTINE.to_le_bytes());
    let mut gba = Gba::new(crate::gba::bios::stub(), &rom);
    crate::gba::install_swi_hook(&mut gba);

    // `sub_8003010` calls `sub_800326C` twice, and that helper keeps TM0CNT in
    // r4 across its body and restores it on the way out. Watching r4 brackets
    // each measurement; r0 changes once per measurement, carrying the reading.
    // Both are register reads, so neither costs a cycle.
    let mut inside = false;
    let mut r0_at_entry = 0;
    let mut entries = 0u32;
    let mut r4_seen: Vec<u32> = Vec::new();
    let mut readings: Vec<u32> = Vec::new();
    // Research: per-step master-cycle deltas across each measurement window,
    // so the charging can be decomposed instead of guessed at. The deltas are
    // sampled from the host after every `gba.step()` and cost no clock cycles.
    let mut trace: Vec<(u32, u64, u64)> = Vec::new(); // (pc, delta, cum)
    let mut traces: Vec<Vec<(u32, u64, u64)>> = Vec::new();
    for _ in 0..400_000 {
        let before = gba.cpu.bus.master_cycles();
        gba.step();
        let after = gba.cpu.bus.master_cycles();
        let r4 = gba.cpu.registers.register_at(4);
        let r0 = gba.cpu.registers.register_at(0);
        if inside {
            let pc = gba.cpu.registers.program_counter() as u32;
            let last_cum = trace.last().map(|t| t.2).unwrap_or(0);
            trace.push((pc, after - before, last_cum + (after - before)));
        }
        if r4_seen.len() < 6 && !r4_seen.contains(&r4) {
            r4_seen.push(r4);
        }
        if r4 == TM0CNT as u32 {
            if !inside {
                // Only on entry: refreshing this every step would make the
                // comparison at exit trivially equal.
                r0_at_entry = r0;
                entries += 1;
                trace.clear();
            }
            inside = true;
        } else if inside {
            inside = false;
            traces.push(std::mem::take(&mut trace));
            if r0 != r0_at_entry {
                readings.push(r0);
            }
            if readings.len() == 2 {
                break;
            }
        }
    }
    for (index, one) in traces.iter().enumerate() {
        println!("  window trace {} (pc, per-step delta, cumulative):", index);
        for (pc, delta, cum) in one {
            println!("    pc={pc:08X} delta={delta:>2} cum={cum}");
        }
    }

    let expected = [0x18u32, 0x33];
    // Refuse to print readings this probe did not earn. Three attempts at
    // entering the Thumb routine from the cartridge head all failed to reach it
    // (`r4` never became TM0CNT, final PC landed in low memory), and an earlier
    // version of this probe duly printed a plausible-looking 41 that was just
    // whatever `r0` happened to hold. A measurement that cannot be shown to
    // have run is worse than no measurement, because it gets quoted.
    assert_eq!(
        entries, 2,
        "the cartridge routine was not reached twice (entries={entries}, \
         r4 seen={r4_seen:?}, final PC={:#010x})",
        gba.cpu.registers.program_counter()
    );
    assert_eq!(readings.len(), 2, "only {readings:?} came back");

    for (index, want) in expected.iter().enumerate() {
        let got = readings[index];
        println!(
            "  case {index}  expected {want:<3} measured {got:<5} delta {:+}  {}",
            got as i64 - *want as i64,
            if got == *want { "PASS" } else { "FAIL" }
        );
    }
}

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
