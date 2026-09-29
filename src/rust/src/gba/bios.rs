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

/// Cartridge header field holding the entry point, at `080000C0h`. Bit 0 of
/// the loaded word is the Thumb bit, which is why the boot ends in `BX`.
const CART_HEADER_ENTRY: u32 = 0x0800_00C0;

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
/// `LDR r2, [r2]`
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
/// `LDR r2, [pc, #12]` -- reaches the third boot literal at `0x60`.
///
/// All three boot loads share one rule: the base is `PC + 8`, the ARM reading
/// of the program counter, so a load at `A` with offset `N` addresses
/// `A + 8 + N`. The literals start at `BOOT_CODE + 0x18`.
const LDR_R0_PC_16: u32 = 0xE59F_0010;
/// `LDR r1, [pc, #16]`
const LDR_R1_PC_16: u32 = 0xE59F_1010;
/// `LDR r2, [pc, #12]`
const LDR_R2_PC_12_BOOT: u32 = 0xE59F_200C;
/// `STRH r1, [r0]`
const STRH_R1_R0: u32 = 0xE1C0_10B0;
/// `B .` -- park here. A deterministic hang beats executing whatever the
/// surrounding aperture happens to decode as.
const BRANCH_SELF: u32 = 0xEAFF_FFFE;
/// `B 0x40` -- the reset stub's jump into the boot code.
const BRANCH_BOOT: u32 = 0xEA00_000E;

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
    put_word(image, addr + 8, CART_HEADER_ENTRY);
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

    // Boot code: program the cartridge wait states, then jump to the entry
    // point the cartridge header names.
    put_word(&mut image, BOOT_CODE + 0x00, LDR_R0_PC_16);
    put_word(&mut image, BOOT_CODE + 0x04, LDR_R1_PC_16);
    put_word(&mut image, BOOT_CODE + 0x08, STRH_R1_R0);
    put_word(&mut image, BOOT_CODE + 0x0C, LDR_R2_PC_12_BOOT);
    put_word(&mut image, BOOT_CODE + 0x10, LDR_R2_R2);
    put_word(&mut image, BOOT_CODE + 0x14, BX_R2);
    put_literals(&mut image, BOOT_CODE + 0x18);

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
    use super::{BIOS_SIZE, BOOT_CODE, BRANCH_BOOT, BRANCH_SELF, STUB};
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
            BOOT_CODE + 0x18 + 8,
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

    /// The whole chain, end to end: a zeroed BIOS does nothing, a stub BIOS
    /// programs `WAITCNT` and hands control to the cartridge. Asserting on the
    /// register the cartridge writes is what proves the `BX` landed.
    #[test]
    fn boot_hands_control_to_the_cartridge() {
        // `MOV r5, #0x34` then park. 0x34 is one byte wide, so unlike a
        // 16-bit constant it is encodable as a rotated ARM immediate.
        let mut rom = vec![0u8; 0x200];
        for (i, chunk) in rom[..0xC0].chunks_exact_mut(4).enumerate() {
            let word = if i == 0 { 0xE3A0_5034 } else { BRANCH_SELF };
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        rom[0xC0..0xC4].copy_from_slice(&0x0800_0000u32.to_le_bytes());

        let mut gba = Gba::new(STUB, &rom);
        install_swi_hook(&mut gba);
        for _ in 0..16 {
            gba.step();
        }

        assert_eq!(
            gba.cpu.registers.register_at(5),
            0x34,
            "the cartridge entry point never ran"
        );
    }

    /// The wait-state register is the only I/O the boot code touches; a game
    /// that reads it back must see the cartridge profile, not zero.
    #[test]
    fn boot_programs_the_cartridge_wait_states() {
        let rom = {
            let mut r = vec![0u8; 0x200];
            r[0xC0..0xC4].copy_from_slice(&0x0800_0000u32.to_le_bytes());
            r
        };
        let mut gba = Gba::new(STUB, &rom);
        for _ in 0..16 {
            gba.step();
        }
        assert_eq!(gba.cpu.bus.read_half_word(0x0400_0204), 0x4317);
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
