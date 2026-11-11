//! SWI 0x16 / 0x17 / 0x18: the three diff-unfilter BIOS calls.
//!
//! | SWI | GBATEK name | source unit | destination store |
//! |---|---|---|---|
//! | `0x16` | `Diff8bitUnFilterWrite8bit` ("Wram") | 8 bit | one byte at a time |
//! | `0x17` | `Diff8bitUnFilterWrite16bit` ("Vram") | 8 bit | one halfword at a time |
//! | `0x18` | `Diff16bitUnFilter` | 16 bit | one halfword at a time |
//!
//! # What the calls do
//!
//! They are **not** decompressors -- the destination is exactly as large as
//! the source. The source holds a *difference* stream and the destination holds
//! the running sum:
//!
//! ```text
//! unfiltered:   10  11  12  13  14  15  16  17  18  19
//! filtered:     10  +1  +1  +1  +1  +1  +1  +1  +1  +1
//! ```
//!
//! That example is GBATEK's own (`.zcode/gbatek_plain.txt:192826-192827`), and
//! it is the golden vector the locks assert against -- the expected values in
//! the tests are **not** recomputed with the same algorithm the code uses
//! (a test that recomputes the answer is a mirror of the implementation, and
//! the two can be wrong together).
//!
//! # The header
//!
//! `r0` points at a 32-bit header followed by the data units:
//!
//! ```text
//! Bit 0-3   Data size (must be 1 for Diff8bit, 2 for Diff16bit)
//! Bit 4-7   Type (must be 8 for DiffFiltered)
//! Bit 8-31  24bit size after decompression
//! ```
//!
//! (GBATEK `:192848-192853`.) The layout is the LZ77 one, so the size is read
//! as **bytes** -- the same reading [`super::decompress::parse_lz77_header`]
//! already uses, and the one mGBA's `_unFilter` implements (`remaining =
//! header >> 8`, decremented by the number of bytes stored at each step).
//!
//! # Evidence
//!
//! * GBATEK (`:192251-192253` for the three numbers, `:192804-192867` for the
//!   contract and the arithmetic) -- names, header layout, the delta rule.
//! * mGBA's `_unFilter` (`src/gba/bios.c:903-946`) -- read for **format only**
//!   (MPL-2.0 is incompatible with this project's licence, so none of its
//!   code is copied). It fixes the one thing GBATEK leaves implicit: which of
//!   the three numbers reads 8-bit units and which one stores halfwords
//!   (`_unFilter(gba, inwidth, outwidth)`, called at `bios.c:578` with
//!   `inwidth = 1` for 0x16/0x17 and `2` for 0x18, and `outwidth = 1` for
//!   0x16 and `2` for 0x17/0x18).
//!
//! # Deliberate differences from that reference
//!
//! * mGBA writes `r0`/`r1` back with the cursors it reached (`bios.c:944-945`).
//!   GBATEK says the call has **no return value** (`:192867`), and the SWI ABI
//!   leaves `r0`/`r1` undefined for such calls, so this implementation leaves
//!   them alone.
//! * mGBA ignores the type and data-size fields; we log a mismatch and carry
//!   on, because the unit width comes from the SWI number and not from the
//!   header. Refusing would break a game that leaves the field zero.
//! * mGBA writes one byte too many when a halfword-store call is given an odd
//!   size; we refuse an odd size instead -- a halfword store cannot express
//!   it, and a deterministic refusal beats a plausible-looking overrun.

/// The type nibble a diff-filtered block carries (GBATEK `:192851`).
pub const DIFF_TYPE: u8 = 8;

/// The data-size field value for 8-bit units (GBATEK `:192850`).
pub const DIFF_DATA_SIZE_8BIT: u8 = 1;

/// The data-size field value for 16-bit units (GBATEK `:192850`).
pub const DIFF_DATA_SIZE_16BIT: u8 = 2;

/// The largest block we accept.
///
/// The same bound the LZ77 decompressors use
/// ([`super::decompress::LZ77_MAX_OUTPUT`]): the size field is 24 bits wide,
/// so a corrupt header can ask for 16 MiB of writes. Refusing is better than
/// honouring it.
pub const MAX_BLOCK: u32 = 16 * 1024 * 1024;

/// A decoded diff-block header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffHeader {
    /// Bits 0-3: the unit size in bytes the block claims (1 or 2).
    pub data_size: u8,
    /// Bits 4-7: the block type (8 = diff filtered).
    pub block_type: u8,
    /// Bits 8-31: the size of the block, in **bytes**.
    pub size: u32,
}

/// Parse the 32-bit header word.
///
/// The field layout is GBATEK's (`:192848-192853`), and it is the same layout
/// the LZ77 blocks use -- which is why the size is a **byte** count and not a
/// unit count.
#[must_use]
pub fn parse_diff_header(raw: u32) -> DiffHeader {
    DiffHeader {
        data_size: (raw & 0x0F) as u8,
        block_type: ((raw >> 4) & 0x0F) as u8,
        size: (raw >> 8) & 0x00FF_FFFF,
    }
}

/// The width of one data unit in the source stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// One byte per unit -- the two `Diff8bit` calls.
    Byte,
    /// One halfword per unit -- the `Diff16bit` call.
    Halfword,
}

impl Unit {
    /// How many bytes one unit occupies.
    #[must_use]
    pub const fn bytes(self) -> u32 {
        match self {
            Self::Byte => 1,
            Self::Halfword => 2,
        }
    }
}

/// How the decoded bytes reach the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Write {
    /// One byte at a time -- the "Wram" call.
    Byte,
    /// One halfword at a time -- the "Vram" call and the 16-bit call.
    Halfword,
}

/// Why a block was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnfilterError {
    /// The header's size field is zero: there is nothing to decode.
    ZeroSize,
    /// The header's size field asks for more than [`MAX_BLOCK`] bytes.
    SizeTooLarge {
        /// The size the header asked for.
        size: u32,
    },
    /// The size is not a whole number of source units.
    NotAWholeNumberOfUnits {
        /// The size the header asked for.
        size: u32,
        /// How many bytes one unit occupies.
        unit_bytes: u32,
    },
    /// The size is not a whole number of halfwords, but the call stores
    /// halfwords.
    NotAWholeNumberOfHalfwords {
        /// The size the header asked for.
        size: u32,
    },
    /// The destination is not halfword aligned, but the call stores halfwords.
    MisalignedDestination {
        /// The destination address the game asked for.
        address: u32,
    },
    /// The destination buffer is smaller than the block needs.
    DestinationTooSmall {
        /// How many bytes the block needs.
        needed: usize,
        /// How many bytes the buffer has.
        have: usize,
    },
}

impl std::fmt::Display for UnfilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroSize => write!(f, "the block declares zero bytes"),
            Self::SizeTooLarge { size } => {
                write!(f, "the block declares {size} bytes, over the sane bound")
            }
            Self::NotAWholeNumberOfUnits { size, unit_bytes } => write!(
                f,
                "the block declares {size} bytes, which is not a whole number of \
                 {unit_bytes}-byte units"
            ),
            Self::NotAWholeNumberOfHalfwords { size } => write!(
                f,
                "the block declares {size} bytes, which is not a whole number of \
                 halfwords"
            ),
            Self::MisalignedDestination { address } => write!(
                f,
                "the destination 0x{address:08X} is not halfword aligned, and \
                 this call stores halfwords"
            ),
            Self::DestinationTooSmall { needed, have } => {
                write!(f, "the destination holds {have} bytes, the block needs {needed}")
            }
        }
    }
}

/// Decode one diff-filtered block.
///
/// `source` holds the data units -- the header is **not** part of it -- and
/// `dest` receives the running sums. The halfword unit reads little-endian,
/// the way the bus presents memory.
///
/// The accumulator wraps: 8-bit units wrap at 256, 16-bit units at 65536.
/// That is not an accident of the types but the hardware's behaviour -- the
/// BIOS adds into a 16-bit register and stores the low half.
pub fn unfilter_into(
    source: &[u8],
    dest: &mut [u8],
    unit: Unit,
) -> Result<usize, UnfilterError> {
    let unit_bytes = unit.bytes() as usize;
    if source.len() % unit_bytes != 0 {
        return Err(UnfilterError::NotAWholeNumberOfUnits {
            size: source.len() as u32,
            unit_bytes: unit_bytes as u32,
        });
    }
    let units = source.len() / unit_bytes;
    let needed = units * unit_bytes;
    if dest.len() < needed {
        return Err(UnfilterError::DestinationTooSmall {
            needed,
            have: dest.len(),
        });
    }

    match unit {
        Unit::Byte => {
            let mut accumulator: u8 = 0;
            for (slot, byte) in dest.iter_mut().zip(source.iter()) {
                accumulator = accumulator.wrapping_add(*byte);
                *slot = accumulator;
            }
        }
        Unit::Halfword => {
            let mut accumulator: u16 = 0;
            for index in 0..units {
                let raw = u16::from_le_bytes([source[index * 2], source[index * 2 + 1]]);
                accumulator = accumulator.wrapping_add(raw);
                let bytes = accumulator.to_le_bytes();
                dest[index * 2] = bytes[0];
                dest[index * 2 + 1] = bytes[1];
            }
        }
    }

    Ok(units)
}

#[cfg(test)]
mod tests {
    use super::{
        DIFF_DATA_SIZE_16BIT, DIFF_DATA_SIZE_8BIT, DIFF_TYPE, Unit, parse_diff_header, unfilter_into,
    };

    /// GBATEK's own example (`:192826-192827`): the filtered stream and the
    /// unfiltered values it stands for.
    const FILTERED: [u8; 10] = [10, 1, 1, 1, 1, 1, 1, 1, 1, 1];
    const UNFILTERED: [u8; 10] = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19];

    /// The header word, assembled from its fields (GBATEK `:192848-192853`).
    fn header(data_size: u8, size: u32) -> u32 {
        (size << 8) | (u32::from(DIFF_TYPE) << 4) | u32::from(data_size)
    }

    /// The header's three fields land where GBATEK says they do.
    ///
    /// The expected values are assembled from the fields, not copied out of a
    /// decoded header -- otherwise the test would only prove that the decoder
    /// agrees with itself.
    #[test]
    fn the_header_fields_land_where_gbatek_says() {
        let raw = header(DIFF_DATA_SIZE_8BIT, 0x00AB_CD);
        let parsed = parse_diff_header(raw);

        assert_eq!(parsed.data_size, DIFF_DATA_SIZE_8BIT, "bits 0-3 are the data size");
        assert_eq!(parsed.block_type, DIFF_TYPE, "bits 4-7 are the block type");
        assert_eq!(parsed.size, 0x00AB_CD, "bits 8-31 are the size");
    }

    /// The size field is a **byte** count, not a unit count.
    ///
    /// It is the LZ77 header layout (GBATEK `:192850-192852`), and the LZ77
    /// blocks read it as bytes too -- `parse_lz77_header` in
    /// `super::decompress` takes `(raw >> 8) & 0x00FF_FFFF` as a byte count.
    /// A unit count would put the 24-bit size in a different place entirely,
    /// and the two readings differ by a factor of two for the 16-bit call.
    #[test]
    fn the_size_field_counts_bytes() {
        // 18 << 8 | (8 << 4) | 2: the size field holds 0x12 = 18 bytes.
        let parsed = parse_diff_header(0x0000_1282);
        assert_eq!(parsed.size, 18, "the size field is the byte count");
        assert_eq!(parsed.data_size, DIFF_DATA_SIZE_16BIT, "the data size field is 2");
        assert_eq!(parsed.block_type, DIFF_TYPE, "the type field is 8");
    }

    /// The size field is **24 bits** wide, not 16.
    ///
    /// A 64 KiB block is an ordinary size for a compressed asset, and its
    /// size has bit 16 set. A decoder that read only the low 16 bits would
    /// see **zero** for it and write nothing at all -- so the field's width
    /// needs its own assertion: the small values the other test uses fit in
    /// both readings and cannot tell them apart.
    #[test]
    fn the_size_field_is_twenty_four_bits_wide() {
        // 0x0001_0000 bytes = 64 KiB: bit 16 of the size field is set.
        let parsed = parse_diff_header(0x0100_0082);
        assert_eq!(
            parsed.size, 0x0001_0000,
            "a 64 KiB block must read back as 64 KiB, not as zero"
        );
    }

    /// GBATEK's example, decoded with 8-bit units.
    ///
    /// The expected bytes are GBATEK's published unfiltered row, not a
    /// recomputation of the same sum the code does.
    #[test]
    fn the_eight_bit_units_decode_to_gbateks_example() {
        let mut dest = [0u8; FILTERED.len()];
        let units = unfilter_into(&FILTERED, &mut dest, Unit::Byte).expect("a whole number of bytes");

        assert_eq!(units, FILTERED.len(), "one unit per source byte");
        assert_eq!(dest, UNFILTERED, "the destination must hold GBATEK's row");
    }

    /// The same example, decoded with 16-bit units.
    ///
    /// The source holds the same ten numbers as halfwords; the destination
    /// holds them back, little-endian.
    #[test]
    fn the_sixteen_bit_units_decode_to_gbateks_example() {
        let mut source = Vec::with_capacity(FILTERED.len() * 2);
        for value in FILTERED {
            source.extend_from_slice(&u16::from(value).to_le_bytes());
        }

        let mut dest = vec![0u8; source.len()];
        let units =
            unfilter_into(&source, &mut dest, Unit::Halfword).expect("a whole number of halfwords");

        assert_eq!(units, FILTERED.len(), "one unit per source halfword");
        for (index, expected) in UNFILTERED.iter().enumerate() {
            let at = index * 2;
            let got = u16::from_le_bytes([dest[at], dest[at + 1]]);
            assert_eq!(
                got,
                u16::from(*expected),
                "unit {index} must be GBATEK's unfiltered value"
            );
        }
    }

    /// The accumulator wraps: the hardware adds into a 16-bit register and
    /// stores the low half, so a stream that runs past 0xFF wraps to zero.
    ///
    /// GBATEK does not spell this out; it follows from the arithmetic it does
    /// spell out (`:192855-192858`) and from the store width.
    #[test]
    fn the_accumulator_wraps_at_the_unit_width() {
        let mut dest = [0u8; 3];
        unfilter_into(&[0xF0, 0x20, 0x01], &mut dest, Unit::Byte).expect("three bytes");

        assert_eq!(
            dest,
            [0xF0, 0x10, 0x11],
            "0xF0 + 0x20 must wrap to 0x10, and the next unit adds on top of it"
        );
    }

    /// A source that is not a whole number of units is refused, not rounded.
    #[test]
    fn a_ragged_source_is_refused() {
        let mut dest = [0u8; 4];
        let error = unfilter_into(&[1, 2, 3], &mut dest, Unit::Halfword)
            .expect_err("three bytes is not a whole number of halfwords");

        assert_eq!(
            error,
            super::UnfilterError::NotAWholeNumberOfUnits {
                size: 3,
                unit_bytes: 2
            },
            "a ragged source must be refused rather than rounded down"
        );
    }

    /// A destination that is too small is refused, not overrun.
    #[test]
    fn a_short_destination_is_refused() {
        let mut dest = [0u8; 2];
        let error = unfilter_into(&[1, 2, 3, 4], &mut dest, Unit::Byte)
            .expect_err("four bytes do not fit in two");

        assert_eq!(
            error,
            super::UnfilterError::DestinationTooSmall {
                needed: 4,
                have: 2
            },
            "the destination must not be overrun"
        );
    }
}
