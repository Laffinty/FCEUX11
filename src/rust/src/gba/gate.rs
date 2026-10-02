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

/// Where the program body starts: the `0xC0` slot, which on a real cartridge
/// holds a branch, and `0xC4`, which is where a branch from `0x00` would
/// land. Either is fine; what matters is that the boot reaches the cartridge
/// and the cartridge's own first instruction reaches the body.
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


/// A cartridge: a header, an entry branch, then `body`.
///
/// The stub BIOS branches to `08000000h` and stops there; the cartridge's own
/// first instruction decides where its code is. So this one has to be a real
/// cartridge: a `b` at offset 0, reached by the boot, landing on `body`. The
/// stub does **not** read a word out of the cartridge and branch to it -- an
/// earlier version of the stub believed `0xC0` held the entry address, and
/// these tests were built to match it, which is how a boot no real cartridge
/// survives went green. See r49.
fn cartridge(body: &[u32]) -> Vec<u8> {
    let mut rom = vec![0u8; 0x200];

    // A real cartridge opens with a branch to its code, and this one is
    // executed: the boot lands at ROM offset 0.
    let offset = CODE_BASE.wrapping_sub(0x0800_0000 + 8);
    rom[..4].copy_from_slice(&(0xEA00_0000 | ((offset >> 2) & 0x00FF_FFFF)).to_le_bytes());
    // Main unit code 0x00 marks a GBA cartridge.
    rom[0xB3] = 0x00;

    let end = CODE_OFFSET + body.len() * 4;
    assert!(end <= rom.len(), "the program does not fit in the image");
    for (i, word) in body.iter().enumerate() {
        let at = CODE_OFFSET + i * 4;
        rom[at..at + 4].copy_from_slice(&word.to_le_bytes());
    }
    rom
}

/// A program: set `r0`, call SWI `number`.
///
/// No trailing `B .`: [`run`] appends the `Halt` that stops the machine, and a
/// branch here would park the program before it ever got there. The first two
/// drafts of this helper had exactly that, and the tests using it failed while
/// the hand-written ones passed -- which is the kind of inconsistency worth
/// more than a green suite, since it pointed straight at the helper.
///
/// `r1` is left as the stub left it, so a test needing a second argument sets
/// it in the setup and reads the register back rather than expecting the
/// program to have stored anything.
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
    body
}

/// Run a program and report `r0`.
///
/// # How the program is stopped
///
/// A trailing `B .` does **not** work, and the reason is the ARM pipeline.
/// With three stages, a branch at `P` is fetched as `P`, `P+4` and `P+8`
/// before it takes effect, so the program counter walks
/// `P -> P+4 -> P+8 -> P -> ...` in a three-step cycle. It never repeats on
/// consecutive steps, so "the PC stopped moving" is not a usable signal --
/// which is what the first two drafts of this function tried, and why they
/// either exited at the wrong moment or never exited at all.
///
/// Instead the program ends with `Halt` (SWI `0x02`), which we claim: the
/// machine stops, and a halted CPU keeps its program counter still. That is
/// both a real use of one of our own SWIs and a signal with no pipeline
/// ambiguity.
use crate::gba::swi::Swi;

fn run(body: &[u32]) -> u32 {
    let mut halted = Vec::new();
    halted.extend_from_slice(body);
    halted.push(swi(Swi::Halt as u32));

    let mut gba = gba_core::gba::Gba::new(crate::gba::bios::stub(), &cartridge(&halted));
    crate::gba::install_swi_hook(&mut gba);

    for _ in 0..CYCLE_BUDGET {
        if gba.cpu.halted {
            return gba.cpu.registers.register_at(0);
        }
        gba.step();
    }
    panic!("the program never reached its `Halt`");
}

#[cfg(test)]
mod tests {
    use super::{cartridge, immediate_field};
    use crate::gba::swi::Swi;

    /// The rotated-immediate rule every test here depends on.
    ///
    /// Cross-checked against a brute-force enumeration in the next test rather
    /// than reasoned about, because the failure mode is silent: an
    /// unencodable value masked to 12 bits is still a *valid* immediate, just
    /// a different one, so the test would fail on a number nobody chose.
    #[test]
    fn the_immediate_rule_matches_the_hardware() {
        // One byte wide: rotation 0.
        assert_eq!(immediate_field(0), Some(0x000));
        assert_eq!(immediate_field(0x34), Some(0x034));
        assert_eq!(immediate_field(0x90), Some(0x090));

        // 0x4000 is 0x01 rotated right by 18, which is 2 * 9.
        assert_eq!(immediate_field(0x4000), Some(0x901));
        // 0x400 is 0x01 rotated right by 22 = 2 * 11. Note this is *not*
        // 0x04 with rotation 0 -- the field stores the constant after the
        // rotation, not before it.
        assert_eq!(immediate_field(0x400), Some(0xB01));
        // 0x8000 looks like an odd rotation but is not: 0x02 ROR 18 is it.
        assert_eq!(immediate_field(0x8000), Some(0x902));
        // 1000 is 0xFA rotated right by 26; two drafts of this file asserted
        // it was not encodable, and that was wrong.
        assert!(immediate_field(0x3E8).is_some(), "1000 does encode");

        // Two separate groups of bits: no rotation puts them in one byte.
        assert_eq!(immediate_field(0x1234), None, "0x1234 needs two bytes");
        assert_eq!(immediate_field(0x8034), None, "0x8034 is two groups");
    }

    /// Every field this module builds decodes back to its value.
    ///
    /// Checked by splitting the field into `rotate` and `constant` and
    /// reconstructing with `ROR`, exactly as the hardware does. A mirrored
    /// error in the builder would pass a test that only compared encodings --
    /// and that mirrored error is what this file got wrong twice.
    #[test]
    fn every_field_this_module_builds_decodes_back() {
        for value in [0u32, 1, 4, 0x34, 0x90, 128, 0x3E8, 0x400, 0x4000, 0xFF, 0x8000] {
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

    /// A cartridge is accepted and its body runs, before any SWI is involved.
    ///
    /// Separate from the round trips below so that "the harness works" and
    /// "the SWI is reached" are different assertions.
    #[test]
    fn a_synthetic_cartridge_hands_control_to_its_body() {
        // `Halt` on its own: the machine must reach it, which means the stub's
        // entry literal, the ROM reads and the ARM decode all line up.
        let mut gba = gba_core::gba::Gba::new(
            crate::gba::bios::stub(),
            &cartridge(&[super::swi(Swi::Halt as u32)]),
        );
        crate::gba::install_swi_hook(&mut gba);
        for _ in 0..100_000 {
            if gba.cpu.halted {
                return;
            }
            gba.step();
        }
        panic!("the cartridge never got control");
    }

    mod swi_round_trip {
        use super::super::{mov_r0_imm, program, run, swi};
        use crate::gba::swi::Swi;

        /// `Div` (SWI `0x06`): `128 / 4` is `32`, and `r1` is the remainder.
        ///
        /// `0x06` is **not** claimed by us -- the core's own arm handles it --
        /// so this also proves the hook declines a number and the core still
        /// runs. That path is exactly what would break if the hook started
        /// claiming things it should not.
        #[test]
        fn div_is_the_falling_through_case_and_still_correct() {
            // r1 is the divisor; `Div` takes the numerator in r0.
            let mut body = vec![mov_r0_imm(128)];
            body.push(0xE3A0_1000 | 0x004); // MOV r1, #4
            body.push(swi(Swi::Div as u32));
            assert_eq!(run(&body), 32, "SWI 0x06 should have divided 128 by 4");
        }

        /// `Sqrt` (SWI `0x08`): `0x90` is 144, and the contract is an integer
        /// result, so the answer is 12.
        #[test]
        fn sqrt_returns_the_integer_root() {
            let body = program(Some(0x90), Swi::Sqrt as u32);
            assert_eq!(run(&body), 12, "sqrt(144) is 12, and 0x08 is not ours");
        }

        /// `ArcTan` (SWI `0x09`): the contract pins `PI/2` to `0x4000`, so
        /// `atan(1.0)` is `0x2000`.
        ///
        /// This is the value the r28 rounding-order fix moved by exactly one
        /// unit, so it is the one most worth having cross-checked by a route
        /// that does not share the implementation's assumptions.
        #[test]
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
        fn arctan2_returns_the_exact_axis_constant() {
            let mut body = vec![mov_r0_imm(0)];
            body.push(0xE3A0_0000 | 0x4000 | 0x1000); // MOV r1, #0x4000
            body.push(swi(Swi::ArcTan2 as u32));
            assert_eq!(run(&body), 0x4000, "due +Y is exactly 0x4000");
        }

        /// `GetBiosChecksum` (SWI `0x0D`): the retail value, which is the
        /// deliberate trade recorded as L2.
        #[test]
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
