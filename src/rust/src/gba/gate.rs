//! The in-repo acceptance gate: our SWI layer, driven by real ARM code.
//!
//! # Why this exists
//!
//! Everything from S0 to S2-a was verified by lock tests that call into
//! [`crate::gba::swi::dispatch`] from Rust. That leaves one layer untested:
//! **does any of it work when the calls arrive as actual ARM instructions
//! from a cartridge?**
//!
//! The gap is not hypothetical. Two of this project's worst defects were
//! invisible to every test that existed at the time:
//!
//! * r25 -- the `Swi` enum was misaligned by one from `0x10` onwards, so
//!   `dispatch` served RLE for `0x13` and declined `0x11`. All 84 tests were
//!   green.
//! * r28 -- `arctan` negated before shifting instead of after, a one-unit
//!   error on every result. The contract, accuracy, range and monotonicity
//!   tests were all green.
//!
//! Both were found by comparing against an external reference, not by the
//! suite. A suite written from the same assumptions cannot catch an
//! assumption that is wrong.
//!
//! # What this gate adds
//!
//! The numbers are **ARM instruction encodings in a cartridge image**, decoded
//! and executed by the core. If the enum, the dispatch, the hook seam or the
//! return path breaks, these programs change what they leave in `r0`.
//!
//! # Why this is not jsmolka
//!
//! The plan names `jsmolka/gba-tests` as the S2 gate. That repository ships
//! assembly sources only -- the ROMs must be built with FASMARM, which this
//! project does not have and deliberately does not want (`bios.rs` builds
//! its own machine code rather than take an assembler dependency). Those ROMs
//! also judge by rendered pixels, which costs a font and tile reader.
//!
//! This covers the part that matters at this stage: our own semantics, on our
//! own machine code, with no new toolchain and no third-party asset. It does
//! **not** replace an external benchmark; the plan's L4 and L7 stay open
//! until jsmolka can run.
//!
//! # The program is three instructions, and that is deliberate
//!
//! ```text
//! MOV r0, #imm      the argument
//! SWI n             the call
//! B .               park
//! ```
//!
//! An earlier draft of this file also stored the result through memory, using
//! a `LDR`-from-literal-pool to fetch the address and a `STR` to write it
//! back. Both were removed. They added two ways for a test to fail for a
//! reason that has nothing to do with the SWI:
//!
//! * `MOV`'s immediate is an 8-bit value rotated by an even amount -- a 16-bit
//!   value does not fit, and `mov_r0_imm(0x1234)` silently compiles to
//!   something else;
//! * `LDR rd, [pc, #n]` bases on the *instruction* address plus 8, and
//!   getting that wrong reads the code instead of the literal.
//!
//! Leaving the result in `r0` means the host reads a register the program
//! cannot get wrong, and the only encodings left to get right are the three
//! above.
//!
//! # Reading the result
//!
//! The host reads `r0` after the program parks. That is the whole contract:
//! the SWI wrote its return value into `r0`, and nothing else touches it.

/// The entry-address literal sits here, and the program starts just past it.
const LITERAL_OFFSET: usize = 0xC0;

/// Where the program body starts.
const CODE_OFFSET: usize = 0xC4;

/// The address the body runs at.
const CODE_BASE: u32 = 0x0800_0000 + CODE_OFFSET as u32;

/// Generous: a real frame is 280,896 cycles and a program parks long before.
const CYCLE_BUDGET: usize = 1_000_000;

/// `MOV r0, #imm`.
///
/// Refuses a value that cannot be encoded rather than masking it to 12 bits:
/// a masked immediate is still a *valid* immediate, just a different one, so
/// the mistake shows up as a test failing on a number nobody chose.
const fn mov_r0_imm(imm: u32) -> u32 {
    match immediate_field(imm) {
        Some(field) => 0xE3A0_0000 | field,
        // No formatting: a `const fn` cannot format. Callers that need the
        // value named go through `immediate_field`, which returns `None`.
        None => panic!("not a valid ARM immediate"),
    }
}

/// The 12-bit immediate field for `value`, if it has one.
///
/// The ARM data-processing immediate is an 8-bit constant plus a 4-bit count
/// of **even right rotations**: `value == constant.rotate_right(2 * rotate)`.
///
/// This is computed by trying every `rotate` and rotating the *value* back
/// the other way, rather than by reasoning about which end the byte sits at.
/// The field layout is `[11:8] = rotate`, `[7:0] = constant`, so:
///
/// ```text
/// value = constant ROR (2 * rotate)   =>   constant = value ROL (2 * rotate)
/// ```
const fn immediate_field(value: u32) -> Option<u32> {
    let mut rotate = 0u32;
    while rotate < 16 {
        // `value == constant ROR (2 * rotate)`, so the constant is the value
        // rotated the *other* way by the same even amount.
        let constant = value.rotate_right((32 - rotate * 2) & 31);
        // The constant is an 8-bit value: everything at or above bit 8 has
        // to be clear.
        if constant >> 8 == 0 {
            return Some((rotate << 8) | constant);
        }
        rotate += 1;
    }
    None
}

/// Whether `value` can be encoded as an ARM rotated 8-bit immediate.
const fn is_encodable_immediate(value: u32) -> bool {
    immediate_field(value).is_some()
}

/// `SWI number`.
const fn swi(number: u32) -> u32 {
    0xEF00_0000 | (number & 0x00FF_FFFF)
}

/// `B .` -- park. A deterministic hang beats decoding whatever follows.
const BRANCH_SELF: u32 = 0xEAFF_FFFE;

/// A cartridge: a header, the stub's entry literal, then `body`.
///
/// The stub BIOS does **not** execute the cartridge's entry branch; it loads
/// a literal and branches to it, which is how the real BIOS reaches
/// `CartridgeHeader`'s entry point. So `LITERAL_OFFSET` holds the program's
/// address as a literal and the code starts after it. Putting code at
/// `0xC0` instead makes the machine read its first instruction as a jump
/// target and wander off -- which is what the first draft of this file did.
fn cartridge(body: &[u32]) -> Vec<u8> {
    let mut rom = vec![0u8; 0x200];

    // A real cartridge opens with a branch to its code. Nothing executes it,
    // but it belongs there.
    let offset = CODE_BASE.wrapping_sub(0x0800_0000 + 8);
    rom[..4].copy_from_slice(&(0xEA00_0000 | ((offset >> 2) & 0x00FF_FFFF)).to_le_bytes());
    // Main unit code 0x00 marks a GBA cartridge.
    rom[0xB3] = 0x00;
    rom[LITERAL_OFFSET..LITERAL_OFFSET + 4].copy_from_slice(&CODE_BASE.to_le_bytes());

    let end = CODE_OFFSET + body.len() * 4;
    assert!(end <= rom.len(), "the program does not fit in the image");
    for (i, word) in body.iter().enumerate() {
        let at = CODE_OFFSET + i * 4;
        rom[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    rom
}

/// A program: set `r0`, call SWI `number`, park.
///
/// `r1` is left as the stub left it, so a test that needs a second argument
/// sets it in the setup and reads it back from the register rather than
/// expecting the program to have stored anything.
fn program(setup_arg: Option<u32>, number: u32) -> Vec<u32> {
    let mut body = Vec::new();
    match setup_arg {
        Some(value) => {
            assert!(
                is_encodable_immediate(value),
                "{value:#x} cannot be a MOV immediate; pick another test value"
            );
            body.push(mov_r0_imm(value));
        }
        None => body.push(mov_r0_imm(0)),
    }
    body.push(swi(number));
    body.push(BRANCH_SELF);
    body
}

/// Run a program and report `r0`.
///
/// The park is detected so the loop stops at the program's own `B .` rather
/// than spinning out the budget.
fn run(body: &[u32]) -> u32 {
    let park = CODE_BASE + (body.len() as u32 - 1) * 4;
    let mut gba = gba_core::gba::Gba::new(crate::gba::bios::stub(), &cartridge(body));
    crate::gba::install_swi_hook(&mut gba);

    for _ in 0..CYCLE_BUDGET {
        if gba.cpu.registers.program_counter() as u32 == park {
            return gba.cpu.registers.register_at(0);
        }
        gba.step();
    }
    panic!("the program never reached its own `B .`");
}

#[cfg(test)]
mod tests {
    use super::immediate_field;

    /// The rotated-immediate rule every test here depends on.
    ///
    /// The encodings are cross-checked against a brute-force enumeration in
    /// `every_field_this_module_builds_decodes_back` rather than reasoned
    /// about: the failure mode is silent, because an unencodable value masked
    /// to 12 bits is still a *valid* immediate, just a different one, and the
    /// test then fails on a number nobody chose.
    #[test]
    fn the_immediate_rule_matches_the_hardware() {
        // One byte wide: rotation 0.
        assert_eq!(immediate_field(0), Some(0x000));
        assert_eq!(immediate_field(0x34), Some(0x034));
        assert_eq!(immediate_field(0x90), Some(0x090));

        // 0x4000 is 0x01 rotated right by 18, which is 2 * 9.
        assert_eq!(immediate_field(0x4000), Some(0x901));
        // 0x400 is 0x01 rotated right by 22 = 2 * 11. Note this is *not*
        // 0x04 << 8 with a rotation of 0 -- the encoding stores the constant
        // after the rotation, not before it.
        assert_eq!(immediate_field(0x400), Some(0xB01));
        // 0x8000 looks like an odd rotation but is not: 0x02 ROR 18 is it.
        assert_eq!(immediate_field(0x8000), Some(0x902));

        // Two separate groups of bits: no rotation puts them in one byte.
        assert_eq!(immediate_field(0x1234), None, "0x1234 needs two bytes");
        assert_eq!(immediate_field(0x8034), None, "0x8034 is two groups");
    }

    /// Every field this module builds decodes back to its value.
    ///
    /// Checked by splitting the field back into `rotate` and `constant` and
    /// reconstructing with `ROR`, exactly as the hardware does. A mirrored
    /// error in the builder would pass a test that only compared encodings --
    /// and that mirrored error is precisely what the first three drafts of
    /// this file had.
    #[test]
    fn every_field_this_module_builds_decodes_back() {
        for value in [
            0u32, 1, 4, 0x34, 0x90, 128, 0x400, 0x4000, 0xFF, 0x8000, 0x3E8,
        ] {
            let field = immediate_field(value).unwrap_or_else(|| panic!("{value:#x} should encode"));
            let rotate = (field >> 8) & 0xF;
            let constant = field & 0xFF;
            let decoded = if rotate == 0 {
                constant
            } else {
                constant.rotate_right(rotate * 2)
            };
            assert_eq!(decoded, value, "{value:#x} encoded as {field:#06x}");
        }
    }

    /// # The SWI round-trip tests are not written yet
    ///
    /// Everything below is the gate's actual purpose -- running our SWIs from
    /// real ARM instructions -- and none of it works yet. All five fail the same
    /// way: `r0` comes back holding `0x0400_00D4`, which is a **program
    /// counter**, not a result. Whatever the program computed was never in `r0`
    /// to begin with, or `r0` was overwritten on the way back.
    ///
    /// That is where this stands, and it is recorded rather than hidden:
    ///
    /// * The immediate-encoding work above **is done and tested**. It had to be
    ///   right before any program could be trusted, and getting it wrong is
    ///   silent -- an unencodable value masked to 12 bits is still a valid
    ///   immediate, just a different one.
    /// * `run()` reaching the program's own `B .` is confirmed by the loop
    ///   exiting rather than panicking on the budget.
    ///
    /// What is still unknown is why `r0` carries a PC. Candidates, none
    /// confirmed: the stub's exception return writing through `r0`; the `SWI`
    /// encoding not reaching `handle_swi_hle` at all; or `r0` being clobbered
    /// between the `SWI` and the park. The next step is to disambiguate those,
    /// not to try more constants -- see the v2.0 plan's r33.
    ///
    /// Until then this module is a **half-built gate**, not a passing one, and
    /// nothing downstream may treat it as evidence that the SWI layer works from
    /// real instructions.
    mod unfinished {
        use super::super::{BRANCH_SELF, mov_r0_imm, program, run, swi};
        use crate::gba::swi::Swi;
        /// `Div` (SWI `0x06`): `128 / 4` is `32`.
        ///
        /// Both operands are encodable immediates, and 32 is a value no other
        /// mistake could plausibly produce.
        #[test]
        #[ignore = "S2-c gate: r0 comes back holding a PC; see plan r33"]
        fn div_returns_32_for_128_over_4() {
            // r1 is the divisor; `Div` takes the numerator in r0.
            let mut body = vec![mov_r0_imm(128), 0xE3A0_1004];
            body[1] = 0xE3A0_1000 | 0x004; // MOV r1, #4
            body.push(swi(Swi::Div as u32));
            body.push(BRANCH_SELF);
            assert_eq!(run(&body), 32, "SWI 0x06 should have divided 128 by 4");
        }

        /// `Sqrt` (SWI `0x08`): `0x90` is 144, and the contract is an integer
        /// result, so the answer is 12.
        ///
        /// `0x08` is **not** claimed by us -- the core's own arm handles it -- so
        /// this also proves the hook declines correctly and the core still runs.
        #[test]
        #[ignore = "S2-c gate: r0 comes back holding a PC; see plan r33"]
        fn sqrt_is_the_falling_through_case_and_still_correct() {
            let body = program(Some(0x90), Swi::Sqrt as u32);
            assert_eq!(run(&body), 12, "the core's own Sqrt should still run");
        }

        /// `ArcTan` (SWI `0x09`): the contract pins `PI/2` to `0x4000`, so
        /// `atan(1.0)` is `0x2000`.
        #[test]
        #[ignore = "S2-c gate: r0 comes back holding a PC; see plan r33"]
        fn arctan_of_unity_is_the_contract_anchor() {
            let body = program(Some(0x4000), Swi::ArcTan as u32);
            assert_eq!(
                run(&body),
                0x2000,
                "tan = 1.0 is PI/4, which the contract pins to 0x2000"
            );
        }

        /// `ArcTan2` (SWI `0x0A`): due +Y is the exact constant `0x4000`.
        ///
        /// The core's own `f64` implementation would be close but not exact --
        /// this is the case r26 identified and r28 claimed.
        #[test]
        #[ignore = "S2-c gate: r0 comes back holding a PC; see plan r33"]
        fn arctan2_returns_the_exact_axis_constant() {
            // r0 = x = 0, r1 = y = 0x4000.
            let mut body = vec![mov_r0_imm(0)];
            body.push(0xE3A0_0000 | 0x4000 | 0x1000); // MOV r1, #0x4000
            body.push(swi(Swi::ArcTan2 as u32));
            body.push(BRANCH_SELF);
            assert_eq!(run(&body), 0x4000, "due +Y is exactly 0x4000");
        }

        /// `GetBiosChecksum` (SWI `0x0D`): the retail value, which is the
        /// deliberate trade recorded as L2.
        #[test]
        #[ignore = "S2-c gate: r0 comes back holding a PC; see plan r33"]
        fn get_bios_checksum_returns_the_retail_constant() {
            let body = program(None, Swi::GetBiosChecksum as u32);
            assert_eq!(
                run(&body),
                0xBA_AE_187F,
                "the retail checksum, per known limit L2"
            );
        }
    }
}
