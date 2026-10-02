//! SWI 0x0D: `GetBiosChecksum`.
//!
//! The BIOS call sums the BIOS image as 32-bit words and hands the result
//! back in `r0`. GBATEK records the value as `BAAE187Fh` for the GBA and GBA
//! SP; the NDS/3DS BIOS running in GBA mode differs only in one byte
//! (`[3F0Ch]` is `01h` there) and so checksums to `BAAE1880h`.
//!
//! # What this returns, and why
//!
//! It returns **the retail GBA value**, not a checksum of the BIOS actually
//! mapped -- we run a stub of our own, and the sum of a 16 KB stub is a number
//! no game has ever seen. That is a deliberate trade, recorded as known
//! limitation **L2**:
//!
//! * returning the retail value puts games on their normal path, which is
//!   what "make the game work" means; and the plan's real-BIOS path
//!   (section 5.5) already validates a user-supplied BIOS against this same
//!   constant, so the two paths agree;
//! * the cost is that a program which branches on the checksum is told it is
//!   running retail hardware when it is not. We do not actually sum the stub
//!   because a checksum nobody recognises conveys nothing -- it is not a
//!   signal any game can act on.
//!
//! # Contract
//!
//! GBATEK: *"Parameters: None. Return: r0=Checksum."* Only `r0` is written.
//! Some implementations additionally stuff `r1 = 1` and `r3 = 0x4000`; those
//! are outside the documented contract, and writing registers the BIOS leaves
//! alone is how a caller ends up depending on garbage.

/// The GBA and GBA SP BIOS checksum.
pub const GBA_BIOS_CHECKSUM: u32 = 0xBA_AE_187F;

/// The NDS / 3DS BIOS checksum when running in GBA mode.
///
/// One byte differs from [`GBA_BIOS_CHECKSUM`] -- `[3F0Ch]` is `01h` there
/// instead of `00h` -- so the sum lands one higher. Not used: we always report
/// the GBA value, but it is recorded here so the difference is not mistaken for
/// a typo if a caller ever needs it.
pub const NDS_IN_GBA_MODE_CHECKSUM: u32 = 0xBA_AE_1880;

/// The value `GetBiosChecksum` leaves in `r0`.
pub const fn checksum() -> u32 {
    GBA_BIOS_CHECKSUM
}

#[cfg(test)]
mod tests {
    use super::{checksum, GBA_BIOS_CHECKSUM, NDS_IN_GBA_MODE_CHECKSUM};

    /// The two known BIOS sums differ by exactly one, and that is the whole
    /// of the difference: the NDS BIOS has one byte set where the GBA has it
    /// clear, at `3F0Ch`.
    #[test]
    fn the_two_known_checksums_differ_by_one() {
        assert_eq!(NDS_IN_GBA_MODE_CHECKSUM, GBA_BIOS_CHECKSUM + 1);
    }

    /// The GBA value is the one we report. Asserted as a literal on purpose:
    /// this constant is the entire content of the function, and a test that
    /// only compared it to `checksum()` would pass no matter what both were.
    #[test]
    fn the_reported_value_is_the_gba_bios_checksum() {
        assert_eq!(checksum(), 0xBA_AE_187F);
        assert_eq!(checksum(), GBA_BIOS_CHECKSUM);
    }
}
