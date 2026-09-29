//! SWI 0x10-0x14: the LZ77 / RLE decompressors.
//!
//! Only the header handling lives here so far, which is the part that can be
//! checked against GBATEK without a decompressor: the payload is a bitstream
//! and is S1b work. Getting the header wrong is the classic "boots to a
//! black screen" failure -- a destination size short by 8 bytes makes the CPU
//! walk off the end of VRAM with nothing to see in a log.

/// Bit 31 of the LZ77 header: the output size field counts 8-byte units.
pub const LZ77_UNIT_FLAG: u32 = 1 << 31;

/// Largest LZ77 output we accept before treating the header as corrupt.
pub const LZ77_MAX_OUTPUT: u32 = 16 * 1024 * 1024;

/// A decoded LZ77 header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lz77Header {
    /// Payload size in bytes, following the header word.
    pub compressed_len: u32,
    /// Decompressed size in bytes.
    pub output_len: u32,
}

/// Parse an LZ77 header word.
///
/// The word packs two fields that must not overlap:
///
/// * bits 0-23: the compressed length, which is never zero in practice --
///   the compressed form is always longer than the 4-byte header;
/// * bits 24-30: the decompressed length, with bit 31 selecting the unit.
///
/// The pre-S1b version of this function read the decompressed field as
/// `raw & 0x0FFF_FFFF`, which folds bits 0-23 -- the *compressed* length --
/// into it. For any real header that returns the compressed length OR-ed
/// into the output length, so the value was always garbage. Nothing caught it
/// because the probe test used a header whose low 24 bits were zero, which is
/// exactly the case that cannot occur in a real file.
///
/// The internal argument is enough to reject the old reading: one 24-bit
/// field cannot be both lengths. The unit flag is bit 31, so the byte-count
/// form leaves the field at its raw value and the 8-byte-unit form scales it.
pub fn parse_lz77_header(raw: u32) -> Lz77Header {
    let compressed_len = raw & 0x00FF_FFFF;
    let output_field = (raw >> 24) & 0x7F;
    let output_len = if raw & LZ77_UNIT_FLAG != 0 {
        output_field.saturating_mul(8)
    } else {
        output_field
    };
    Lz77Header {
        compressed_len,
        output_len,
    }
}

/// `Lz77UnCompWram` (0x10) and `Lz77UnCompVram` (0x11) are one algorithm;
/// only the destination aperture differs, and that is enforced on the core
/// side. Both share this header decode.
pub fn lz77_header(raw: u32) -> Lz77Header {
    parse_lz77_header(raw)
}

/// Reject a header that cannot be honoured.
///
/// A zero or absurd output length means a corrupt header, not a reason to
/// allocate; returning `Err` lets the caller trap loudly instead of
/// corrupting VRAM.
pub fn check_output_len(header: Lz77Header) -> Result<(), &'static str> {
    if header.output_len == 0 {
        return Err("lz77: zero output length");
    }
    if header.output_len > LZ77_MAX_OUTPUT {
        return Err("lz77: output length exceeds sane bound");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{LZ77_MAX_OUTPUT, check_output_len, lz77_header, parse_lz77_header};

    /// Assemble a header from its two fields.
    ///
    /// Hand-written constants are how the previous test got this wrong: the
    /// size field starts at bit 24, so a "compressed length" with bit 24 set
    /// silently merges into it, and `0x20 << 24 | 1 << 31` is not the word one
    /// thinks it is. Composing the fields makes that class of slip impossible.
    fn header(compressed: u32, output_units: u32, eight_byte_units: bool) -> u32 {
        assert!(compressed <= 0x00FF_FFFF, "the compressed field is 24 bits");
        assert!(output_units <= 0x7F, "the output field is 7 bits");
        let size = (output_units & 0x7F) << 24;
        (compressed & 0x00FF_FFFF) | size | u32::from(eight_byte_units) << 31
    }

    /// The two length fields must not share bits. A header whose compressed
    /// length is non-zero -- which is every real header, since compressed
    /// data is always longer than the 4-byte header -- has to decode to an
    /// output length that contains nothing from the low 24 bits.
    #[test]
    fn the_two_length_fields_do_not_overlap() {
        let h = parse_lz77_header(header(0xA4, 0x20, false));
        assert_eq!(h.compressed_len, 0xA4);
        assert_eq!(h.output_len, 0x20);

        let h = parse_lz77_header(header(0xA4, 0x20, true));
        assert_eq!(h.compressed_len, 0xA4);
        assert_eq!(h.output_len, 0x20 * 8);

        // The pre-S1b reading returned `raw & 0x0FFF_FFFF`, which for this
        // header is the compressed length OR-ed with the size.
        assert_ne!(h.output_len, 0xA4);
        assert_ne!(h.output_len, 0xA4 | 0x20);
    }

    /// The whole 7-bit field is length, not just its low six bits: a 0x40
    /// output in 8-byte units is 512 bytes, and dropping the top bit would
    /// halve it.
    #[test]
    fn the_top_size_bit_is_part_of_the_length() {
        assert_eq!(parse_lz77_header(header(0x10, 0x40, true)).output_len, 0x40 * 8);
        assert_eq!(
            parse_lz77_header(header(0x10, 0x7F, true)).output_len,
            0x7F * 8
        );
    }

    /// A corrupt header is rejected instead of being decompressed into
    /// whatever the output pointer happens to be.
    #[test]
    fn corrupt_headers_are_rejected() {
        assert!(check_output_len(lz77_header(header(0xA4, 0x10, false))).is_ok());
        // A header with no size at all is the "always zero" case.
        assert!(check_output_len(lz77_header(0)).is_err(), "zero output");
        assert!(check_output_len(lz77_header(0x0000_00A4)).is_err());
        // The largest expressible output (0x7F units) stays under the cap.
        assert!(LZ77_MAX_OUTPUT >= 0x7F * 8);
        assert!(check_output_len(lz77_header(header(0x10, 0x7F, true))).is_ok());
    }

    /// A byte-unit header keeps its value across the whole field.
    #[test]
    fn a_byte_unit_header_keeps_its_value() {
        for len in [1u32, 8, 0x40, 0x7F] {
            let h = parse_lz77_header(header(0x12_3456, len, false));
            assert_eq!(h.output_len, len);
            assert_eq!(h.compressed_len, 0x12_3456);
        }
    }
}
