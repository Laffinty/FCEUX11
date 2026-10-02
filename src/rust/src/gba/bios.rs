//! The stub BIOS: a 16 KB image we author, replacing Nintendo's.
//!
//! The v2.0 plan (section 5.2) drops the real BIOS entirely and splits the job
//! in two: this image owns the boot and exception vectors, and
//! [`crate::gba::swi`] owns the SWI semantics. Nothing here is derived from
//! third-party BIOS code -- the words below are written out by hand from the
//! documented ARM7TDMI reset behaviour.
//!
//! # Why the words live in Rust instead of a `.s` file
//!
//! Assembling them would drag in a cross-assembler the project does not have
//! and would not otherwise need. A `const fn` that writes the words into the
//! image keeps the build free of extra tooling, and the test module pins every
//! word, so a typo cannot reach the image silently.
//!
//! # What the image deliberately does not do
//!
//! * **Stack pointers.** The real BIOS sets `sp_svc`/`sp_irq`/`sp_usr` during
//!   boot. The vendored core already does that in `Arm7tdmi::new`
//!   (`arm7tdmi.rs:845`), so doing it again here would be dead code.
//! * **The `IntrWait` flag word at `03007FF8`.** GBATEK lists it as BIOS
//!   memory, but it is written by the *game's* interrupt handler, not by the
//!   BIOS: the BIOS handler only saves registers, calls the pointer at
//!   `03007FFC`, and restores. Adding a write here would double-count every
//!   interrupt a real game already reported.

/// Size of the BIOS aperture on a GBA.
pub const BIOS_SIZE: usize = 0x4000;

/// `WAITCNT` (`04000204h`): the cartridge timing the BIOS programs before
/// handing control to the game. 0x4317 is the value a manufactured cartridge
/// runs with -- WS0/ROM 3,1 clocks, SRAM 8, prefetch enabled.
const WAITCNT_ADDR: u32 = 0x0400_0204;
const WAITCNT_DEFAULT: u32 = 0x4317;

/// Where a cartridge's code begins.
///
/// **Not a header field.** A GBA cartridge has no "entry point" word: the
/// machine resets into the BIOS, the BIOS branches to `08000000h`, and the
/// cartridge's own `b` at offset 0 takes it wherever it likes. The word at
/// `080000C0h` is the *Nintendo reserved* slot, which holds a branch
/// instruction like any other code -- on a commercial cartridge it is
/// routinely `E3A0xxxx`, a `mov`.
///
/// The first draft of this file read `0x080000C0` and branched to it, on the
/// reasoning that a "0xC0 must hold the entry address" convention had been
/// observed somewhere. It had not: it had been assumed, then written into
/// four lock tests that built cartridges to match, and the suite went green
/// over a boot sequence no real cartridge can survive. See r49.
const CART_ENTRY: u32 = 0x0800_0000;

/// Pointer to the game's interrupt handler, at `03007FFC`.
const IRQ_HANDLER_PTR: u32 = 0x0300_7FFC;

/// Where the boot code lives. Everything from `0x20` up is ordinary BIOS ROM:
/// the vectors are the eight words at `0x00`-`0x1C`, and the reset stub at
/// `0x00` branches past them.
const BOOT_CODE: usize = 0x40;

// ---- instruction encodings -------------------------------------------------
//
// Written as named constants so the table below reads like the assembly it
// stands for, and so a reviewer can check one line at a time.

/// `STMDB sp!, {r0-r3, r12, lr}`
const STMFD_SP_R0_R3_R12_LR: u32 = 0xE92D_500F;
/// `LDMIA sp!, {r0-r3, r12, lr}`
const LDMFD_SP_R0_R3_R12_LR: u32 = 0xE8BD_500F;
/// `LDR r2, [pc, #8]` -- the literal pool that follows the IRQ handler.
const LDR_R2_PC_8: u32 = 0xE59F_2008;
/// `LDR r2, [r2]` -- the IRQ vector's indirect load of the handler pointer.
const LDR_R2_R2: u32 = 0xE592_2000;
/// `CMP r2, #0`
const CMP_R2_ZERO: u32 = 0xE352_0000;
/// `BEQ +8` -- skip the call when the game never installed a handler.
const BEQ_POP: u32 = 0x0A00_0002;
/// `ADD lr, pc, #0` -- return address for the game's handler.
const ADD_LR_PC: u32 = 0xE28F_E000;
/// `BX r2` -- the indirect call, and the boot jump (honours the Thumb bit).
const BX_R2: u32 = 0xE12F_FF12;
/// `SUBS pc, lr, #4` -- return from an exception.
const SUBS_PC_LR_4: u32 = 0xE25E_F004;
/// `LDR r0, [pc, #12]` -- reaches the first boot literal at `0x54`.
///
/// All three boot loads share one rule: the base is `PC + 8`, the ARM reading
/// of the program counter, so a load at `A` with offset `N` addresses
/// `A + 8 + N`. The literals sit immediately after the boot code.
const LDR_R0_PC_12: u32 = 0xE59F_000C;
/// `LDR r1, [pc, #12]`
const LDR_R1_PC_12: u32 = 0xE59F_100C;
/// `STRH r1, [r0]`
const STRH_R1_R0: u32 = 0xE1C0_10B0;
/// `LDR r2, [pc, #8]` -- reaches the third boot literal at `0x5C`.
const LDR_R2_PC_8_BOOT: u32 = 0xE59F_2008;
/// `B .` -- park here. A deterministic hang beats executing whatever the
/// surrounding aperture happens to decode as.
const BRANCH_SELF: u32 = 0xEAFF_FFFE;
/// `B 0x40` -- the reset stub's jump into the boot code.
const BRANCH_BOOT: u32 = 0xEA00_000E;

/// Where the boot code's literal pool begins: five instructions, then three
/// words. Kept as a named constant because the tests and the S4 probe all
/// address it by arithmetic, and that arithmetic is exactly what went wrong
/// once already.
const BOOT_LITERALS: usize = BOOT_CODE + 0x14;

const fn put_word(image: &mut [u8; BIOS_SIZE], addr: usize, word: u32) {
    let bytes = word.to_le_bytes();
    let mut i = 0;
    while i < 4 {
        image[addr + i] = bytes[i];
        i += 1;
    }
}

const fn put_literals(image: &mut [u8; BIOS_SIZE], addr: usize) {
    put_word(image, addr, WAITCNT_ADDR);
    put_word(image, addr + 4, WAITCNT_DEFAULT);
    put_word(image, addr + 8, CART_ENTRY);
}

const fn build() -> [u8; BIOS_SIZE] {
    let mut image = [0u8; BIOS_SIZE];

    // 0x00 reset vector: jump over the exception table into the boot code.
    put_word(&mut image, 0x00, BRANCH_BOOT);
    // 0x04-0x10 undefined / prefetch abort / data abort. Reaching any of them
    // means the emulated machine took an exception nothing handles, which is a
    // bug worth making loud and still rather than a silent wild jump.
    put_word(&mut image, 0x04, BRANCH_SELF);
    // 0x08 SWI vector: only reached for a number our dispatch declines. The
    // trace goes to stderr, so hanging here turns "unimplemented SWI" into an
    // obvious stall instead of an inexplicable one.
    put_word(&mut image, 0x08, BRANCH_SELF);
    put_word(&mut image, 0x0C, BRANCH_SELF);
    put_word(&mut image, 0x10, BRANCH_SELF);
    put_word(&mut image, 0x14, BRANCH_SELF);

    // 0x18 IRQ vector.
    put_word(&mut image, 0x18, STMFD_SP_R0_R3_R12_LR);
    put_word(&mut image, 0x1C, LDR_R2_PC_8);
    put_word(&mut image, 0x20, LDR_R2_R2);
    put_word(&mut image, 0x24, CMP_R2_ZERO);
    put_word(&mut image, 0x28, BEQ_POP);
    put_word(&mut image, 0x2C, IRQ_HANDLER_PTR); // literal pool
    put_word(&mut image, 0x30, ADD_LR_PC);
    put_word(&mut image, 0x34, BX_R2);
    put_word(&mut image, 0x38, LDMFD_SP_R0_R3_R12_LR);
    put_word(&mut image, 0x3C, SUBS_PC_LR_4);

    // Boot code: program the cartridge wait states, then branch to the
    // cartridge's first word. Five instructions and nothing else -- in
    // particular there is no dereference, because there is nothing at
    // 080000C0h to dereference.
    put_word(&mut image, BOOT_CODE + 0x00, LDR_R0_PC_12);
    put_word(&mut image, BOOT_CODE + 0x04, LDR_R1_PC_12);
    put_word(&mut image, BOOT_CODE + 0x08, STRH_R1_R0);
    put_word(&mut image, BOOT_CODE + 0x0C, LDR_R2_PC_8_BOOT);
    put_word(&mut image, BOOT_CODE + 0x10, BX_R2);
    put_literals(&mut image, BOOT_LITERALS);

    image
}

/// The stub BIOS image handed to [`gba_core::gba::Gba::new`].
pub const STUB: [u8; BIOS_SIZE] = build();

/// A fresh stub image, for callers that want their own copy.
#[must_use]
pub const fn stub() -> [u8; BIOS_SIZE] {
    STUB
}

#[cfg(test)]
mod tests {
    use super::{
        BIOS_SIZE, BOOT_CODE, BOOT_LITERALS, BRANCH_BOOT, BRANCH_SELF, BX_R2, CART_ENTRY,
        LDR_R0_PC_12, LDR_R1_PC_12, LDR_R2_PC_8_BOOT, STUB, STRH_R1_R0,
    };


    use crate::gba::install_swi_hook;
    use crate::gba::swi::Swi;
    use gba_core::gba::Gba;

    fn word(addr: usize) -> u32 {
        u32::from_le_bytes([STUB[addr], STUB[addr + 1], STUB[addr + 2], STUB[addr + 3]])
    }
    /// The image is exactly the BIOS aperture, and nothing outside the vectors
    /// and the two code blocks is populated. A stray write into the middle of
    /// the aperture would decode as instructions and run at some address we do
    /// not control.
    #[test]
    fn image_is_only_vectors_and_code() {
        assert_eq!(STUB.len(), BIOS_SIZE);
        let populated: Vec<usize> = (0..BIOS_SIZE / 4)
            .filter(|i| word(i * 4) != 0)
            .map(|i| i * 4)
            .collect();
        let last = *populated.last().expect("the image cannot be empty");
        assert_eq!(
            last,
            BOOT_LITERALS + 8,
            "unexpected word past the boot literals"
        );
        // Everything between the end of the IRQ vector and the boot code is
        // the middle of the aperture and must stay empty.
        for addr in (0x40..BOOT_CODE).step_by(4) {
            assert_eq!(word(addr), 0, "{addr:#06x} should be empty");
        }
    }

    #[test]
    fn reset_vector_branches_to_the_boot_code() {
        assert_eq!(word(0x00), BRANCH_BOOT);
    }

    #[test]
    fn unhandled_vectors_park_instead_of_wandering() {
        for addr in [0x04, 0x08, 0x0C, 0x10, 0x14] {
            assert_eq!(word(addr), BRANCH_SELF, "{addr:#06x}");
        }
    }

    /// A cartridge shaped the way a real one is: a branch at offset 0 into the
    /// code at `CODE_AT`, and `MOV r5, #MARK` there.
    ///
    /// The branch matters. A cartridge's first word is a real instruction the
    /// machine runs, so a test ROM that only fills the code region would be
    /// exercising a cartridge that does not exist.
    const CODE_AT: usize = 0xC0;
    const MARK: u32 = 0x34;
    const MOV_R5_MARK: u32 = 0xE3A0_5000 | MARK;

    /// `b` at ROM offset 0 to `CODE_AT`, encoded the way the core decodes it:
    /// the target is `PC + 8 + (offset << 2)`, and at offset 0 that is
    /// `8 + (offset << 2)`.
    fn branch_to(target: usize) -> u32 {
        let offset = ((target - 8) / 4) as u32;
        0xEA00_0000 | (offset & 0x00FF_FFFF)
    }

    /// A cartridge whose entry code parks after marking `r5`.
    ///
    /// `0xC0` is the *Nintendo reserved* slot on a real cartridge, so putting
    /// the branch target there is both what hardware does and what makes the
    /// word the old boot sequence used to read land on an instruction rather
    /// than an address.
    fn cartridge_entering_code() -> Vec<u8> {
        let mut rom = vec![0u8; 0x200];
        rom[..4].copy_from_slice(&branch_to(CODE_AT).to_le_bytes());
        rom[CODE_AT..CODE_AT + 4].copy_from_slice(&MOV_R5_MARK.to_le_bytes());
        rom
    }

    /// The whole chain, end to end: a zeroed BIOS does nothing, a stub BIOS
    /// programs `WAITCNT` and hands control to the cartridge. Asserting on the
    /// register the cartridge writes is what proves the `BX` landed.
    #[test]
    fn boot_hands_control_to_the_cartridge() {
        let mut gba = Gba::new(STUB, &cartridge_entering_code());
        install_swi_hook(&mut gba);
        for _ in 0..16 {
            gba.step();
        }

        assert_eq!(
            gba.cpu.registers.register_at(5),
            MARK,
            "the cartridge entry code never ran"
        );
    }

    /// The boot branches to `08000000h`, not to whatever word sits at
    /// `080000C0h`.
    ///
    /// The first version of this file read the reserved slot and branched to
    /// its contents. On the cartridge this was written against that was
    /// self-consistent; on a commercial one the reserved slot holds a `mov`,
    /// so the `BX` landed in the unmapped `0xExxxxxx` region and every real
    /// game was a black screen. The suite could not see it, because the
    /// cartridges in it were built to the same wrong convention.
    ///
    /// So the guard is about the *address*, asserted on its own: the boot's
    /// entry literal is `08000000h`, full stop. A cartridge that puts code at
    /// `0xC0` reaches it through its own branch, and that path is covered by
    /// `boot_hands_control_to_the_cartridge`.
    #[test]
    fn the_boot_branches_to_the_start_of_the_cartridge() {
        // The address is written out rather than naming `CART_ENTRY`. A guard
        // that compares the image against the constant it is guarding is a
        // guard that passes for any value the constant is given: changing
        // `CART_ENTRY` to `0x0800_00C0` turned the whole suite green, which is
        // the original bug, reproduced in the test that was written to catch
        // it.
        assert_eq!(
            word(BOOT_LITERALS + 8),
            0x0800_0000,
            "the boot's entry literal must be 08000000h, not a word read out of the cartridge"
        );
        assert_eq!(
            CART_ENTRY, 0x0800_0000,
            "CART_ENTRY is the constant the boot is built from; the image check above \
             is the one that holds when it drifts"
        );

        // The whole boot sequence, in order. Asserting the sequence rather
        // than one slot is what makes "the dereference came back" fail: it
        // would be a sixth instruction, and the `BX` would no longer be last.
        let expected = [
            LDR_R0_PC_12,
            LDR_R1_PC_12,
            STRH_R1_R0,
            LDR_R2_PC_8_BOOT,
            BX_R2,
        ];
        let actual: Vec<u32> = (0..expected.len()).map(|i| word(BOOT_CODE + i * 4)).collect();
        assert_eq!(
            actual, expected,
            "the boot sequence changed; five instructions ending in BX r2, and no dereference \
             between the load and the branch, is what a cartridge can actually survive"
        );
    }

    /// The wait-state register is the only I/O the boot code touches; a game
    /// that reads it back must see the cartridge profile, not zero.
    #[test]
    fn boot_programs_the_cartridge_wait_states() {
        let mut gba = Gba::new(STUB, &cartridge_entering_code());
        for _ in 0..16 {
            gba.step();
        }
        assert_eq!(gba.cpu.bus.read_half_word(0x0400_0204), 0x4317);
    }

    /// The IRQ vector is the one piece of the stub that a test cannot reach by
    /// booting: something has to be *pending* for the vector to be entered.
    /// This drives the whole chain -- boot, exception, the vector's push, the
    /// indirect call through `03007FFC`, the game's `bx lr`, the pop, and the
    /// return -- and is the path every game with an interrupt handler depends
    /// on. Nothing above tests it, and S1b's jsmolka gate would hit it
    /// immediately.
    #[test]
    fn irq_vector_calls_the_handler_installed_at_03007ffc() {
        const HANDLER: u32 = 0x0800_0100;
        // The cartridge's entry code parks; the handler lives elsewhere.
        let mut rom = vec![0u8; 0x200];
        rom[..4].copy_from_slice(&branch_to(CODE_AT).to_le_bytes());
        for word in rom[8..0xC0].chunks_exact_mut(4) {
            word.copy_from_slice(&BRANCH_SELF.to_le_bytes());
        }
        rom[CODE_AT..CODE_AT + 4].copy_from_slice(&BRANCH_SELF.to_le_bytes());
        // The handler: mark that it ran, then return to the BIOS stub.
        rom[0x100..0x104].copy_from_slice(&0xE3A0_5034u32.to_le_bytes()); // mov r5,#0x34
        rom[0x104..0x108].copy_from_slice(&0xE12F_FF1Eu32.to_le_bytes()); // bx lr

        let mut gba = Gba::new(STUB, &rom);
        install_swi_hook(&mut gba);
        gba.cpu.bus.write_word(0x0300_7FFC, HANDLER);

        for _ in 0..16 {
            gba.step();
        }
        assert_eq!(
            gba.cpu.registers.register_at(5),
            0,
            "the cartridge should be running before the interrupt"
        );

        // Press a key, with the keypad interrupt armed, then let it fire.
        gba.cpu.bus.keypad.key_interrupt_control = (1 << 14) | 0x0001;
        gba.cpu.bus.keypad.key_input &= !0x0001;
        gba.cpu.bus.write_half_word(0x0400_0200, 1 << 12); // IE  = keypad
        gba.cpu.bus.write_half_word(0x0400_0208, 0x0001); // IME = enable
        for _ in 0..16 {
            gba.step();
        }

        assert_eq!(
            gba.cpu.registers.register_at(5),
            0x34,
            "the handler installed at 03007FFC was never called"
        );
        let pc = gba.cpu.registers.program_counter();
        assert!(
            (0x0800_0000..0x0800_0200).contains(&pc),
            "the BIOS stub did not return to the interrupted code: PC={pc:#010x}"
        );
    }

    /// The SWI vector is only reached when `dispatch` declines a number, and
    /// `Halt` is the first one S1a will claim. This test fixes the expectation
    /// that the stub does *not* implement it: claiming is our job, the vector
    /// is the fallback that parks.
    #[test]
    fn the_stub_does_not_implement_swi() {
        // The number exists in the dispatch table...
        assert!(Swi::from_raw(Swi::Halt as u32).is_some());
        // ...but the image has no code at the SWI vector to service it.
        assert_eq!(word(0x08), BRANCH_SELF);
    }
}
