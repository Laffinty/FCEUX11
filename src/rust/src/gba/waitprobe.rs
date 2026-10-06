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
//! **What is not settled:** the prescaler probe cannot produce a reading yet.
//! After 20 000 `gba.step()` calls its loop counter has only fallen from 1024
//! to 182, and the loop body is two instructions -- so roughly twelve `step()`
//! calls pass per iteration even though `Gba::step` is documented as "one CPU
//! instruction cycle". That 12:1 contradiction is unexplained: either `step()`
//! is not one instruction in this configuration, or this probe is miscounting.
//! **Do not read it as "the core runs 12x slow"** -- that would contradict
//! commercial ROMs running at the right frame rate, so at least one of the two
//! readings is wrong. Until "`step()` equals one instruction" is pinned down,
//! this probe cannot settle the accounting question.
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

    for _ in 0..20_000 {
        gba.step();
    }

    let measured = gba.cpu.registers.register_at(0);
    let loop_counter = gba.cpu.registers.register_at(1);
    let pc = gba.cpu.registers.program_counter();
    println!("AGS TIMER PRESCALER loop");
    println!("  PC        = {pc:#010x} (in program: {})", (PROGRAM_BASE..PROGRAM_BASE + 0x100).contains(&pc));
    println!("  loop counter r1 = {loop_counter} (0 means the loop finished)");
    println!("  iterations = {PRESCALER_ITERATIONS}");
    println!("  measured  = {measured}");
    println!("  AGS expects= {PRESCALER_EXPECTED}");
    println!(
        "  per iteration = {:.2} cycles (AGS: {:.2})",
        f64::from(measured) / f64::from(PRESCALER_ITERATIONS),
        f64::from(PRESCALER_EXPECTED) / f64::from(PRESCALER_ITERATIONS)
    );
    println!(
        "  delta = {:+}",
        measured as i64 - PRESCALER_EXPECTED as i64
    );
    println!("  master cycles = {}", gba.cpu.bus.master_cycles());
}
