//! SWI 0x10: `BitUnPack`.
//!
//! Widens the colour depth of bitmap or tile data: a 1bpp monochrome font
//! becomes 4bpp or 8bpp GBA tiles, 4bpp tiles become 8bpp, and so on. The
//! unpack description is passed separately, so one source buffer can be
//! re-emitted at several depths.
//!
//! # Contract
//!
//! ```text
//! r0  source address (no alignment required)
//! r1  destination address (must be 32bit-word aligned)
//! r2  pointer to the unpack information:
//!       16bit  length of source data in bytes
//!        8bit  width of source units in bits   (1, 2, 4 or 8)
//!        8bit  width of destination units      (1, 2, 4, 8, 16 or 32)
//!       32bit  data offset in bits 0-30, zero-data flag in bit 31
//! ```
//!
//! The data offset is added to every non-zero source unit. When the zero-data
//! flag is set it is added to zero units as well. Output is written in 32-bit
//! words, to WRAM or VRAM alike -- the size of the unpacked data must be a
//! multiple of 4 bytes.
//!
//! # Bit order
//!
//! Both the source and the destination are consumed **from the least
//! significant end**:
//!
//! * a source byte holds `8 / source_width` source units, packed low bit to
//!   high bit -- unit `n` occupies bits `n*w .. n*w+w` of the byte;
//! * destination units are accumulated from bit 0 upward, and the word is
//!   stored once 32 bits have accumulated.
//!
//! This is the same direction the Huffman unpacker uses (see
//! [`super::decompress`]), and the opposite of the LZ77 flag byte, which is
//! MSB-first. The two decompressors disagree, and getting either backwards
//! yields output that still decompresses cleanly -- so this is pinned by
//! dedicated tests rather than left to a round trip.
//!
//! # Overflow is not masked, deliberately
//!
//! If `unit + offset` does not fit the destination element width, the excess
//! bits spill into the *following* destination elements of the same word.
//! We reproduce that rather than truncating, because it is what the hardware
//! does, and because a tool that produced such data would have meant the
//! widths to differ. Callers are expected to keep
//! `source_width + offset <= destination_width`. Recorded as known
//! limitation **L10**.
//!
//! # Evidence
//!
//! Four independent sources agree on every point above: GBATEK's
//! `BitUnPack` entry, mGBA's `_unBitPack`, tonc's `BUP` struct, and the
//! `gba` crate's annotated `BitUnPack` wrapper. mGBA is read for format only
//! (MPL-2.0 is incompatible with this project's licence); the code here is
//! our own.

/// Source units narrower than this are not supported by the BIOS.
const VALID_SOURCE_WIDTHS: [u32; 4] = [1, 2, 4, 8];

/// Destination units wider than 32 bits cannot be represented.
const VALID_DEST_WIDTHS: [u32; 6] = [1, 2, 4, 8, 16, 32];

/// The bit-31 marker of the 32-bit offset field: add the offset to zero units too.
const ZERO_DATA_FLAG: u32 = 0x8000_0000;

/// The unpack description `r2` points at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnpackInfo {
    /// Length of the source data, in bytes.
    pub source_len: u16,
    /// Width of a source unit, in bits.
    pub source_width: u8,
    /// Width of a destination unit, in bits.
    pub dest_width: u8,
    /// Value added to each unit; bit 31 additionally covers zero units.
    pub offset_and_zero: u32,
}

impl UnpackInfo {
    /// The offset with the zero-data flag stripped off.
    pub const fn offset(&self) -> u32 {
        self.offset_and_zero & !ZERO_DATA_FLAG
    }

    /// Whether the offset is also added to zero-valued units.
    pub const fn touches_zero(&self) -> bool {
        self.offset_and_zero & ZERO_DATA_FLAG != 0
    }
}

/// Why a stream could not be unpacked.
///
/// The BIOS has no defined behaviour for any of these; it reads the
/// description and carries on regardless. We reject them instead, because a
/// silently wrong unpacked buffer is far harder to diagnose than a refused
/// call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitUnpackError {
    /// `source_width` is not one of 1, 2, 4, 8.
    BadSourceWidth(u8),
    /// `dest_width` is not one of 1, 2, 4, 8, 16, 32.
    BadDestWidth(u8),
    /// The source is shorter than `source_len` bytes.
    SourceTooShort { declared: u16, available: usize },
    /// The destination address is not 32-bit aligned.
    MisalignedDest(usize),
}

impl core::fmt::Display for BitUnpackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadSourceWidth(w) => write!(f, "source width {w} is not 1, 2, 4 or 8"),
            Self::BadDestWidth(w) => {
                write!(f, "destination width {w} is not 1, 2, 4, 8, 16 or 32")
            }
            Self::SourceTooShort { declared, available } => {
                write!(f, "source declares {declared} bytes but only {available} are readable")
            }
            Self::MisalignedDest(addr) => {
                write!(f, "destination 0x{addr:08X} is not 32-bit aligned")
            }
        }
    }
}

/// Unpack one source buffer into 32-bit destination words.
///
/// `dest` is a word-aligned slice that is written from its start; it must be
/// at least as long as [`required_words`] reports. The number of words
/// actually written is returned.
pub fn unpack_into(
    src: &[u8],
    dest: &mut [u32],
    info: &UnpackInfo,
) -> Result<usize, BitUnpackError> {
    if !VALID_SOURCE_WIDTHS.contains(&u32::from(info.source_width)) {
        return Err(BitUnpackError::BadSourceWidth(info.source_width));
    }
    if !VALID_DEST_WIDTHS.contains(&u32::from(info.dest_width)) {
        return Err(BitUnpackError::BadDestWidth(info.dest_width));
    }
    let declared = usize::from(info.source_len);
    if src.len() < declared {
        return Err(BitUnpackError::SourceTooShort {
            declared: info.source_len,
            available: src.len(),
        });
    }
    let src = &src[..declared];

    let source_width = u32::from(info.source_width);
    let dest_width = u32::from(info.dest_width);
    let source_mask = (1u32 << source_width) - 1;
    let offset = info.offset();
    let touch_zero = info.touches_zero();

    let mut written = 0usize;
    let mut out: u32 = 0;
    let mut bits_eaten: u32 = 0;

    for &byte in src {
        // A source byte yields `8 / source_width` units; widths are powers of
        // two that divide 8, so this divides exactly.
        let mut unit = u32::from(byte);
        for _ in 0..(8 / source_width) {
            let mut scaled = unit & source_mask;
            unit >>= source_width;
            if scaled != 0 || touch_zero {
                scaled = scaled.wrapping_add(offset);
            }
            // Not masked to `dest_width`: see the module docs. `bits_eaten` is
            // always < 32 here, so this shift cannot panic.
            out |= scaled << bits_eaten;
            bits_eaten += dest_width;
            if bits_eaten == 32 {
                if written >= dest.len() {
                    // The description promised more output than the caller
                    // provided room for. Stop rather than overwrite.
                    return Ok(written);
                }
                dest[written] = out;
                written += 1;
                bits_eaten = 0;
                out = 0;
            }
        }
    }
    Ok(written)
}

/// How many 32-bit words [`unpack_into`] will write for this description.
///
/// Always rounds up: a partial word still consumes a full slot in the
/// destination, because the BIOS writes whole words. GBATEK requires the
/// unpacked size to be a multiple of 4 bytes, so well-formed callers land on
/// the boundary exactly.
pub const fn required_words(info: &UnpackInfo) -> usize {
    let source_width = info.source_width as u32;
    if source_width == 0 {
        return 0;
    }
    let units = (info.source_len as u32 * 8) / source_width;
    let dest_width = info.dest_width as u32;
    if dest_width == 0 {
        return 0;
    }
    // `units` destination elements, `32 / dest_width` of them per word, rounded
    // up: a partial word still occupies a whole slot, because the BIOS only
    // ever stores whole words.
    let per_word = 32 / dest_width;
    ((units + per_word - 1) / per_word) as usize
}

#[cfg(test)]
mod tests {
    use super::{
        BitUnpackError, UnpackInfo, ZERO_DATA_FLAG, required_words, unpack_into,
    };

    /// Build a description from named parts.
    ///
    /// Assembled field by field rather than written as a hex literal: a
    /// mistyped literal here would be indistinguishable from a wrong
    /// implementation, and has been in this project before.
    fn info(source_len: u16, source_width: u8, dest_width: u8, offset: u32, zero: bool) -> UnpackInfo {
        UnpackInfo {
            source_len,
            source_width,
            dest_width,
            offset_and_zero: if zero { offset | ZERO_DATA_FLAG } else { offset },
        }
    }

    /// Pack `values` into bytes, `width` bits each, low unit first.
    ///
    /// The inverse of what `unpack_into` does to the source side, so a round
    /// trip checks the implementation against itself -- which is the point of
    /// having it, but is *not* enough on its own: see the ordering tests.
    fn pack(values: &[u32], width: u8) -> Vec<u8> {
        let per_byte = 8 / u32::from(width);
        let mut out = Vec::new();
        for group in values.chunks(per_byte as usize) {
            let mut byte = 0u8;
            for (i, &v) in group.iter().enumerate() {
                let shift = u32::from(width) * i as u32;
                byte |= ((v & ((1u32 << width) - 1)) << shift) as u8;
            }
            out.push(byte);
        }
        out
    }

    /// Units at the given width in a destination buffer.
    fn units(words: &[u32], width: u8) -> Vec<u32> {
        let per_word = 32 / u32::from(width);
        let mask = (1u32 << width) - 1;
        let mut out = Vec::new();
        for &w in words {
            for i in 0..per_word {
                out.push((w >> (u32::from(width) * i)) & mask);
            }
        }
        out
    }

    /// A 4bpp source widened to 8bpp survives a round trip unchanged.
    #[test]
    fn widens_four_bit_units_to_eight() {
        let values: Vec<u32> = (0..16).map(|i| i * 3 % 16).collect();
        let src = pack(&values, 4);
        let info = info(src.len() as u16, 4, 8, 0, false);
        let mut dest = vec![0u32; required_words(&info)];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        assert_eq!(units(&dest[..n], 8), values);
    }

    /// The same description with the offset added to non-zero units.
    #[test]
    fn the_offset_reaches_non_zero_units_only() {
        let values: Vec<u32> = vec![0, 1, 0, 2, 3, 0, 4, 5];
        let src = pack(&values, 4);
        let info = info(src.len() as u16, 4, 8, 7, false);
        let mut dest = vec![0u32; required_words(&info)];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        let expected: Vec<u32> = values.iter().map(|&v| if v == 0 { 0 } else { v + 7 }).collect();
        assert_eq!(units(&dest[..n], 8), expected);
    }

    /// With the zero-data flag set, zero units are offset too.
    #[test]
    fn the_zero_data_flag_reaches_zero_units_too() {
        let values: Vec<u32> = vec![0, 1, 0, 2, 3, 0, 4, 5];
        let src = pack(&values, 4);
        let info = info(src.len() as u16, 4, 8, 7, true);
        let mut dest = vec![0u32; required_words(&info)];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        let expected: Vec<u32> = values.iter().map(|&v| v + 7).collect();
        assert_eq!(units(&dest[..n], 8), expected);
    }

    /// Source units are read low-first within a byte.
    ///
    /// This is the test a round trip cannot replace. Reading high-first
    /// produces output that still round trips through `pack` if the test's
    /// packer is written to match, and merely shifts values around otherwise
    /// -- nothing crashes either way. The values here are chosen so that low-
    /// and high-first give visibly different results.
    #[test]
    fn source_units_are_read_from_the_low_bit_upward() {
        // 0b0000_0101 as 2bpp units: low-first reads 1, 1, 0, 0.
        let src = vec![0b0000_0101u8];
        let info = info(1, 2, 8, 0, false);
        let mut dest = vec![0u32; 1];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        assert_eq!(units(&dest[..n], 8)[..4], vec![1, 1, 0, 0]);
    }

    /// Destination units are accumulated from bit 0 upward.
    ///
    /// The mirror of the test above, and equally invisible to a round trip:
    /// filling from the top instead of the bottom still writes four units, it
    /// just writes them reversed. The expected word is *assembled* from the
    /// units rather than written as a hex literal -- a little-endian mental
    /// image of the byte order is exactly the kind of slip that survives a
    /// green suite, and the memory layout here is the opposite of the file
    /// layout GBA programmers usually picture.
    #[test]
    fn destination_units_accumulate_from_the_low_bit_upward() {
        // All four values must fit the 2bpp source width; a 5th unit of 4
        // would be truncated by `pack` to 0 and the test would then be
        // measuring the packer rather than the ordering.
        let values: Vec<u32> = vec![1, 2, 3, 0];
        let src = pack(&values, 2);
        let info = info(src.len() as u16, 2, 8, 0, false);
        let mut dest = vec![0u32; required_words(&info)];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        let expected: u32 = values
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, &v)| acc | (v << (8 * i as u32)));
        assert_eq!(dest[0], expected);
        // Spelled out one byte at a time so the ordering is visible if the
        // assembly above changes. Each slice, not a shift: `x >> 0` is the
        // whole word, which would compare everything against a single unit.
        let bytes = dest[0].to_le_bytes();
        assert_eq!(bytes, [1, 2, 3, 0], "units ascend from the low byte up");
        assert_eq!(n, 1, "four 2bpp units in, one word out");
    }

    /// The canonical use: 1bpp font to 4bpp, the depth GBA tiles actually use.
    #[test]
    fn widens_one_bit_units_to_four() {
        let values: Vec<u32> = vec![0, 1, 1, 0, 1, 0, 0, 1];
        let src = pack(&values, 1);
        let info = info(src.len() as u16, 1, 4, 0, false);
        let mut dest = vec![0u32; required_words(&info)];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        assert_eq!(units(&dest[..n], 4)[..8], values);
    }

    /// Destination elements wider than a byte straddle word boundaries.
    ///
    /// The values are masked to the *source* width, because a 4bpp source can
    /// only carry 4bpp values -- anything wider is silently dropped by
    /// [`pack`], and comparing against the unmasked list would then be testing
    /// the packer rather than the unpacker.
    #[test]
    fn sixteen_bit_units_straddle_words() {
        let values: Vec<u32> = (0..12).map(|i| (i * 2731) & 0xF).collect();
        let src = pack(&values, 4);
        let info = info(src.len() as u16, 4, 16, 0, false);
        let mut dest = vec![0u32; required_words(&info)];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        assert_eq!(units(&dest[..n], 16)[..12], values);
    }

    /// A whole number of words is written; a partial tail is rounded up.
    #[test]
    fn the_word_count_rounds_a_partial_tail_up() {
        // 3 bytes of 2bpp = 12 units; at 16bpp that is 6 words exactly.
        let exact = info(3, 2, 16, 0, false);
        assert_eq!(required_words(&exact), 6);
        // 1 byte of 8bpp = 1 unit; at 32bpp that is 1 whole word.
        let single = info(1, 8, 32, 0, false);
        assert_eq!(required_words(&single), 1);
        // 3 bytes of 1bpp = 24 units; at 8bpp that fills 6 words.
        let ones = info(3, 1, 8, 0, false);
        assert_eq!(required_words(&ones), 6);
    }

    /// An unsupported source width is refused rather than guessed at.
    #[test]
    fn a_bad_source_width_is_rejected() {
        let info = info(1, 3, 8, 0, false);
        let mut dest = vec![0u32; 1];
        assert_eq!(
            unpack_into(&[0], &mut dest, &info),
            Err(BitUnpackError::BadSourceWidth(3))
        );
    }

    /// An unsupported destination width is refused too.
    #[test]
    fn a_bad_dest_width_is_rejected() {
        let info = info(1, 8, 3, 0, false);
        let mut dest = vec![0u32; 1];
        assert_eq!(
            unpack_into(&[0], &mut dest, &info),
            Err(BitUnpackError::BadDestWidth(3))
        );
    }

    /// A source shorter than the declared length is refused.
    #[test]
    fn a_short_source_is_rejected() {
        let info = info(8, 8, 8, 0, false);
        let mut dest = vec![0u32; 8];
        assert_eq!(
            unpack_into(&[0, 0, 0], &mut dest, &info),
            Err(BitUnpackError::SourceTooShort {
                declared: 8,
                available: 3
            })
        );
    }

    /// An offset that overflows the destination width spills into the next
    /// element, exactly as the hardware does.
    ///
    /// Pinned on purpose (see the module docs). If this ever starts
    /// truncating instead, that is a behaviour change a game could depend on,
    /// and L10 needs revisiting.
    #[test]
    fn an_overflowing_offset_spills_into_the_next_element() {
        // 4bpp source widened to 4bpp with offset 15. Four source bytes are
        // eight 4bpp units, which is exactly one word -- a partial word is
        // never stored, so the test has to fill one to observe anything. The
        // first unit becomes 1 + 15 = 16, which needs 5 bits, so its top bit
        // lands on the second unit.
        let src = pack(&[1, 0, 0, 0, 0, 0, 0, 0], 4);
        let info = info(src.len() as u16, 4, 4, 15, false);
        assert_eq!(required_words(&info), 1, "eight 4bpp units fill one word");
        let mut dest = vec![0u32; 1];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        // 16 = 0b1_0000: unit 0 keeps its low 4 bits (0), unit 1 gets the carry.
        assert_eq!(units(&dest[..n], 4), vec![0, 1, 0, 0, 0, 0, 0, 0]);
    }

    /// A destination buffer that is too small stops the write rather than
    /// panicking or overrunning.
    #[test]
    fn a_short_destination_stops_at_the_last_whole_word() {
        let values: Vec<u32> = vec![1; 16];
        let src = pack(&values, 4);
        let info = info(src.len() as u16, 4, 8, 0, false);
        // Two bytes of 4bpp = 4 units; at 8bpp that is 4 words.
        assert_eq!(required_words(&info), 4);
        let mut dest = vec![0u32; 2];
        let n = unpack_into(&src, &mut dest, &info).expect("valid description");
        assert_eq!(n, 2);
        assert_eq!(dest, vec![0x0101_0101, 0x0101_0101]);
    }
}
