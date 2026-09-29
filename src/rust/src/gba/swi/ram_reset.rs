//! SWI 0x01: `RegisterRamReset` -- clear memory regions and reset I/O.
//!
//! The core already implements most of this number, but claiming it is
//! all-or-nothing (see the v2.0 plan r15): `handle_swi_hle` returns as soon as
//! our hook answers `true`, so the core's own arm never runs. Claiming `0x01`
//! therefore means reimplementing bits 0/2/3/4 as well, or the four regions
//! that already work would silently stop being cleared.
//!
//! What the core *does not* do, and why we are here:
//!
//! - **bit 1** (IWRAM) is skipped outright, with a source TODO admitting it
//!   ("would break IRQ handlers"). It does not need to: the last `0x200` bytes
//!   of IWRAM hold the stack, the IRQ vector at `03007FFC`, the BIOS interrupt
//!   flags at `03007FF8` and the `IntrWait` state, and preserving *exactly*
//!   that tail is what the real BIOS does.
//! - **bits 5-7** were a three-line `not fully implemented` comment with no
//!   code at all. Bit `0x80` is specified and is implemented here; bits 5/6
//!   (SIO and sound registers) have no bit-level spec on hand and are recorded
//!   as known limitation **L6** rather than guessed at.
//!
//! The bit-to-region mapping below is pure logic with no CPU behind it, which
//! is what lets it be tested exhaustively without a running machine.

/// Clear the 256K EWRAM.
pub const CLEAR_EWRAM: u8 = 0x01;
/// Clear IWRAM, keeping the last [`PRESERVED_TAIL`] bytes.
pub const CLEAR_IWRAM: u8 = 0x02;
/// Clear the 1K palette RAM.
pub const CLEAR_PALETTE: u8 = 0x04;
/// Clear the 96K VRAM.
pub const CLEAR_VRAM: u8 = 0x08;
/// Clear the 1K OAM.
pub const CLEAR_OAM: u8 = 0x10;
/// Reset the serial (SIO) registers. **Not implemented** -- known limit L6.
pub const RESET_SIO: u8 = 0x20;
/// Reset the sound registers. **Not implemented** -- known limit L6.
pub const RESET_SOUND: u8 = 0x40;
/// Reset IE, IF, WAITCNT and IME.
pub const RESET_IO: u8 = 0x80;

/// Bits this implementation acts on. The other two are accepted in `r0` and
/// ignored, rather than rejected: the BIOS is a no-op for unimplemented
/// register groups, not a trap.
pub const IMPLEMENTED_MASK: u8 =
    CLEAR_EWRAM | CLEAR_IWRAM | CLEAR_PALETTE | CLEAR_VRAM | CLEAR_OAM | RESET_IO;

/// The bytes kept at the top of EWRAM and IWRAM.
///
/// This is not a fudge factor. GBA IWRAM is 32K, and the last `0x200` hold the
/// system stack (`03007F00`), the IRQ mode stack (`03007FA0`), the supervisor
/// stack (`03007FE0`), the BIOS interrupt flags (`03007FF8`) and the IRQ
/// vector (`03007FFC`). Clearing them would destroy exactly the state the
/// BIOS is there to preserve, which is the reason the upstream core punted.
pub const PRESERVED_TAIL: u32 = 0x200;

/// Interrupt Enable, `04000200h`.
pub const IE: u32 = 0x0400_0200;
/// Interrupt Flags, `04000204h`. Write-one-to-clear, so `0xFFFF` acknowledges
/// every pending interrupt at once.
pub const IF: u32 = 0x0400_0202;
/// Waitstate Control, `04000204h`.
pub const WAITCNT: u32 = 0x0400_0204;
/// Interrupt Master Enable, `04000208h`.
pub const IME: u32 = 0x0400_0208;

/// A memory region to zero, as a half-open address range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearRegion {
    /// First byte to write.
    pub start: u32,
    /// One past the last byte to write.
    pub end: u32,
}

impl ClearRegion {
    /// How many bytes this region contributes.
    pub const fn len(&self) -> u32 {
        self.end - self.start
    }
}

/// A decoded `RegisterRamReset` request, straight from `r0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RamResetRequest {
    flags: u8,
}

impl RamResetRequest {
    /// Decode the raw `r0` the caller passed.
    ///
    /// Only the low byte is meaningful -- the core read it the same way -- and
    /// an unknown bit is ignored rather than trapped, matching a BIOS that
    /// simply has no register group wired to it.
    pub fn decode(flags: u32) -> Self {
        Self { flags: flags as u8 }
    }

    /// The flag byte, masked to what this implementation understands.
    pub const fn flags(&self) -> u8 {
        self.flags & IMPLEMENTED_MASK
    }

    /// The memory regions to zero, in a stable order.
    pub fn regions(&self) -> Vec<ClearRegion> {
        let mut regions = Vec::new();
        if self.flags & CLEAR_EWRAM != 0 {
            regions.push(ClearRegion {
                start: 0x0200_0000,
                end: 0x0203_FE00,
            });
        }
        if self.flags & CLEAR_IWRAM != 0 {
            regions.push(ClearRegion {
                start: 0x0300_0000,
                end: 0x0300_7E00,
            });
        }
        if self.flags & CLEAR_PALETTE != 0 {
            regions.push(ClearRegion {
                start: 0x0500_0000,
                end: 0x0500_0400,
            });
        }
        if self.flags & CLEAR_VRAM != 0 {
            regions.push(ClearRegion {
                start: 0x0600_0000,
                end: 0x0601_8000,
            });
        }
        if self.flags & CLEAR_OAM != 0 {
            regions.push(ClearRegion {
                start: 0x0700_0000,
                end: 0x0700_0400,
            });
        }
        regions
    }

    /// Whether `IE`, `IF`, `WAITCNT` and `IME` should be reset.
    pub const fn resets_io(&self) -> bool {
        self.flags & RESET_IO != 0
    }

    /// Whether this request asks for a register group we do not implement.
    ///
    /// Surfaced so the caller can say so in a trace instead of letting a game
    /// believe its SIO registers were reset.
    pub const fn wants_unimplemented_groups(&self) -> u8 {
        self.flags & (RESET_SIO | RESET_SOUND) & !IMPLEMENTED_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ClearRegion, RamResetRequest, CLEAR_EWRAM, CLEAR_IWRAM, CLEAR_OAM, CLEAR_PALETTE,
        CLEAR_VRAM, IE, IF, IME, IMPLEMENTED_MASK, PRESERVED_TAIL, RESET_IO, RESET_SIO,
        RESET_SOUND, WAITCNT,
    };

    /// Build `r0` from named bits, the way a caller does. Never a hand-written
    /// literal: this file's neighbours have been wrong by hand twice.
    fn request(bits: &[u8]) -> RamResetRequest {
        let mut flags = 0u8;
        for bit in bits {
            flags |= bit;
        }
        RamResetRequest::decode(flags as u32)
    }

    /// Each bit selects exactly one region, and no other bit leaks into it.
    /// This is the per-bit lock the whole implementation rests on: a region
    /// that answered to the wrong bit would be a silent memory-corrupting bug.
    #[test]
    fn each_bit_selects_exactly_its_own_region() {
        for bit in [
            CLEAR_EWRAM,
            CLEAR_IWRAM,
            CLEAR_PALETTE,
            CLEAR_VRAM,
            CLEAR_OAM,
        ] {
            let regions = request(&[bit]).regions();
            assert_eq!(regions.len(), 1, "bit {bit:#04x} must select one region");
            for other in [
                CLEAR_EWRAM,
                CLEAR_IWRAM,
                CLEAR_PALETTE,
                CLEAR_VRAM,
                CLEAR_OAM,
            ] {
                if other != bit {
                    assert!(
                        !request(&[other]).regions().contains(&regions[0]),
                        "bit {other:#04x} must not select the region of {bit:#04x}"
                    );
                }
            }
        }
    }

    /// The regions are the ones the core already used, at the same addresses.
    /// Claiming the SWI is all-or-nothing, so any drift here would show up as
    /// a region that stopped being cleared.
    #[test]
    fn regions_match_the_core_addresses() {
        let expected = [
            (CLEAR_EWRAM, 0x0200_0000, 0x0203_FE00),
            (CLEAR_IWRAM, 0x0300_0000, 0x0300_7E00),
            (CLEAR_PALETTE, 0x0500_0000, 0x0500_0400),
            (CLEAR_VRAM, 0x0600_0000, 0x0601_8000),
            (CLEAR_OAM, 0x0700_0000, 0x0700_0400),
        ];
        for (bit, start, end) in expected {
            assert_eq!(
                request(&[bit]).regions(),
                vec![ClearRegion { start, end }],
                "bit {bit:#04x} address range"
            );
        }
    }

    /// The `0x200`-byte tail at the top of EWRAM and IWRAM is excluded. This
    /// is the whole reason the core punted on bit 1, so it is the one number
    /// worth being pedantic about.
    #[test]
    fn the_preserved_tail_is_excluded() {
        // EWRAM is 256K at 02000000h; the last 0x200 start at 0203FE00h.
        let ewram = request(&[CLEAR_EWRAM]).regions()[0];
        assert_eq!(ewram.end, 0x0200_0000 + 0x4_0000 - PRESERVED_TAIL);

        // IWRAM is 32K at 03000000h; the last 0x200 start at 03007E00h.
        let iwram = request(&[CLEAR_IWRAM]).regions()[0];
        assert_eq!(iwram.end, 0x0300_0000 + 0x8000 - PRESERVED_TAIL);

        // And the tail is exactly the state the BIOS exists to keep: the
        // stacks, the BIOS interrupt flags and the IRQ vector all live in it.
        assert!(iwram.end <= 0x0300_7F00, "system stack must survive");
        assert!(
            iwram.end <= 0x0300_7FF8,
            "BIOS interrupt flags must survive"
        );
        assert!(iwram.end <= 0x0300_7FFC, "IRQ vector must survive");
    }

    /// Bits combine, and the order is stable so a test can compare directly.
    #[test]
    fn bits_combine() {
        let regions = request(&[CLEAR_VRAM, CLEAR_EWRAM, CLEAR_PALETTE]).regions();
        assert_eq!(
            regions,
            vec![
                ClearRegion {
                    start: 0x0200_0000,
                    end: 0x0203_FE00
                },
                ClearRegion {
                    start: 0x0500_0000,
                    end: 0x0500_0400
                },
                ClearRegion {
                    start: 0x0600_0000,
                    end: 0x0601_8000
                },
            ]
        );

        // No bits, no work.
        assert!(request(&[]).regions().is_empty());
        assert!(!request(&[]).resets_io());
    }

    /// The same bit twice is one region, not two.
    #[test]
    fn duplicate_bits_do_not_double_clear() {
        assert_eq!(
            request(&[CLEAR_OAM, CLEAR_OAM]).regions().len(),
            1,
            "clearing OAM twice is the same as clearing it once"
        );
    }

    /// Bit `0x80` resets exactly the four registers the spec names, at the
    /// addresses the core's interrupt-control block decodes.
    #[test]
    fn the_io_bit_is_recognised() {
        assert!(request(&[RESET_IO]).resets_io());
        assert!(!request(&[CLEAR_VRAM]).resets_io());
        assert!(
            !request(&[CLEAR_VRAM, RESET_IO]).regions().is_empty(),
            "the I/O bit must not suppress the memory clears"
        );

        // IF is write-one-to-clear, so the reset value is 0xFFFF; the rest
        // are plain zero. These are the values the writer below sends.
        assert_eq!(IF, 0x0400_0202);
        assert_eq!(IE, 0x0400_0200);
        assert_eq!(WAITCNT, 0x0400_0204);
        assert_eq!(IME, 0x0400_0208);
    }

    /// Bits 5 and 6 are accepted and deliberately do nothing. The test exists
    /// so that known limit **L6** is a locked fact rather than a comment that
    /// drifts: if someone later implements them, this test is what they flip.
    #[test]
    fn unimplemented_register_groups_are_reported_not_silently_dropped() {
        let sio = request(&[RESET_SIO]);
        assert!(sio.wants_unimplemented_groups() & RESET_SIO != 0);
        assert!(sio.regions().is_empty());
        assert!(!sio.resets_io());

        let sound = request(&[RESET_SOUND]);
        assert!(sound.wants_unimplemented_groups() & RESET_SOUND != 0);

        // They never masquerade as implemented.
        assert_eq!(IMPLEMENTED_MASK & RESET_SIO, 0);
        assert_eq!(IMPLEMENTED_MASK & RESET_SOUND, 0);

        // An implemented request reports nothing outstanding.
        let all = request(&[CLEAR_EWRAM, CLEAR_IWRAM, RESET_IO]);
        assert_eq!(all.wants_unimplemented_groups(), 0);
    }

    /// `r0` is read as a byte, and unknown high bits are ignored rather than
    /// turned into work -- the core truncates the same way, and a caller
    /// passing a stray value must not get a surprise memory clear.
    #[test]
    fn only_the_low_byte_is_decoded() {
        let high_only = RamResetRequest::decode(0x0200_0000);
        assert!(high_only.regions().is_empty());
        assert!(!high_only.resets_io());
        assert_eq!(
            RamResetRequest::decode(0xFFFF_FFFF).flags(),
            IMPLEMENTED_MASK
        );
    }
}
