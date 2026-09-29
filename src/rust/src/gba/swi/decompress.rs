//! SWI 0x10-0x14: the LZ77 / RLE / Huffman decompressors.
//!
//! Only the header handling lives here so far; the payloads are S1b work.
//! Getting the header wrong is the classic "boots to a black screen" failure
//! -- a destination size short by 8 bytes makes the CPU walk off the end of
//! VRAM with nothing to see in a log.
//!
//! The header layout here was verified on 2026-09-29 against mGBA's HLE BIOS
//! (`src/gba/bios.c`), which is itself validated against real hardware. Note
//! that GBA and NDS/Wii have **incompatible** LZ77 headers; see
//! [`parse_lz77_header`] for the difference and why it is easy to get wrong.

/// Signature byte of an LZ77 block.
pub const LZ77_SIGNATURE: u8 = 0x10;
/// Signature byte of a run-length block.
pub const RL_SIGNATURE: u8 = 0x30;
/// Signature nibble of a Huffman block; the low nibble of the same byte holds
/// the bits per element.
pub const HUFFMAN_SIGNATURE: u8 = 0x20;

/// Largest output we accept before treating the header as corrupt.
pub const LZ77_MAX_OUTPUT: u32 = 16 * 1024 * 1024;

/// A decoded compression header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressionHeader {
    /// The signature byte: `0x10` for LZ77, `0x30` for run-length.
    ///
    /// For Huffman the same byte carries the bits-per-element in its low
    /// nibble, on top of the `0x20` signature.
    pub signature: u8,
    /// Decompressed size in bytes (24 bits, no unit flag on this side).
    pub output_len: u32,
}

/// Parse a GBA compression header word.
///
/// Verified 2026-09-29 against mGBA's HLE BIOS (`src/gba/bios.c`,
/// `_unLz77` / `_unHuffman` / `_unRl`), which reads the decompressed size as
/// `(header & 0xFFFFFF00) >> 8` -- the top three bytes, little-endian -- and
/// treats byte 0 as the type signature, which it never uses.
///
/// # Two formats, and this is the GBA one
///
/// GBA and NDS/Wii both have an LZ77 routine with *incompatible* headers, and
/// mixing them up is easy because both are described in the same breath:
///
/// | | GBA (this function) | NDS / Wii |
/// |---|---|---|
/// | byte 0 | signature `0x10` | low byte of the compressed size |
/// | bytes 1-3 | decompressed size | (part of compressed size) |
/// | bit 31 | part of the size | `1` = size counts 8-byte units |
///
/// S0' read the size as `raw & 0x0FFF_FFF`, which folds the signature byte
/// into it; the S1b fix that followed read `(raw >> 24) & 0x7F`, which is the
/// NDS layout. Both are wrong for GBA data, and neither was caught by a test
/// built on a header whose low 24 bits were zero -- a value no real file has.
pub fn parse_lz77_header(raw: u32) -> CompressionHeader {
    CompressionHeader {
        signature: (raw & 0xFF) as u8,
        output_len: (raw >> 8) & 0x00FF_FFFF,
    }
}

/// `Lz77UnCompWram` (0x10) and `Lz77UnCompVram` (0x11) are one algorithm;
/// only the destination aperture differs, and that is enforced on the core
/// side. Both share this header decode.
pub fn lz77_header(raw: u32) -> CompressionHeader {
    parse_lz77_header(raw)
}

/// Reject a header that cannot be honoured.
///
/// A zero or absurd output length means a corrupt header, not a reason to
/// allocate; returning `Err` lets the caller trap loudly instead of
/// corrupting VRAM.
pub fn check_output_len(header: CompressionHeader) -> Result<(), &'static str> {
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
    use super::{
        LZ77_MAX_OUTPUT, LZ77_SIGNATURE, RL_SIGNATURE, check_output_len, lz77_header,
        parse_lz77_header,
    };

    /// Assemble a header from its parts: signature byte plus a 24-bit size.
    fn header(signature: u8, output_len: u32) -> u32 {
        assert!(output_len <= 0x00FF_FFFF, "the size field is 24 bits");
        (signature as u32 & 0xFF) | ((output_len & 0x00FF_FFFF) << 8)
    }

    /// The size is the top three bytes, and byte 0 is the signature. This is
    /// the reading mGBA's HLE BIOS uses, and it is what a real file looks like:
    /// `10 00 20 00` is an LZ77 block that decompresses to 0x2000 bytes.
    #[test]
    fn the_size_is_the_top_three_bytes() {
        let h = parse_lz77_header(header(LZ77_SIGNATURE, 0x2000));
        assert_eq!(h.signature, 0x10);
        assert_eq!(h.output_len, 0x2000);

        // A run-length block of 0x1234 bytes.
        let h = parse_lz77_header(header(RL_SIGNATURE, 0x1234));
        assert_eq!(h.signature, 0x30);
        assert_eq!(h.output_len, 0x1234);

        // Byte order really is little-endian on the size: swapping the halves
        // of the size field changes the answer, which is the transcription
        // mistake two earlier versions of this test made.
        let swapped = 0x10u32 | (0x0020u32 << 24) | (0x0010u32 << 8);
        assert_ne!(parse_lz77_header(swapped).output_len, 0x2000);
    }

    /// The NDS layout is a different format, not a synonym -- and reading one
    /// as the other does not fail loudly, it produces a size that is merely
    /// wrong. This is the hazard this file's history is about: an NDS-style
    /// word says 256 bytes, and the GBA reading of the same word says 10 MB,
    /// which `check_output_len` happily accepts.
    #[test]
    fn an_nds_style_word_reads_as_gba_is_silently_wrong() {
        // NDS: compressed size 0x10 in bits 0-23, 0x20 units in bits 24-30,
        // bit 31 set -> 0x20 * 8 = 0x100 bytes.
        let nds_word = 0x10u32 | (0x20 << 24) | (1 << 31);
        assert_eq!(0x20 * 8, 0x100, "the NDS side of the comparison");

        let as_gba = parse_lz77_header(nds_word);
        assert_eq!(as_gba.signature, 0x10, "byte 0 happens to look right");
        assert_eq!(
            as_gba.output_len, 0xA0_0000,
            "the GBA reading swallows the unit flag as a size bit"
        );
        // Wrong by a factor of 0xA0000, and still inside the sanity cap.
        assert!(check_output_len(as_gba).is_ok());
    }

    /// A corrupt header is rejected instead of being decompressed into
    /// whatever the output pointer happens to be.
    #[test]
    fn corrupt_headers_are_rejected() {
        assert!(check_output_len(lz77_header(header(0x10, 0x2000))).is_ok());
        // A header with no size at all is the "always zero" case.
        assert!(check_output_len(lz77_header(header(0x10, 0))).is_err());
        assert!(check_output_len(lz77_header(0)).is_err());
        // The largest expressible size is 0xFFFFFF, well under the cap.
        assert!(LZ77_MAX_OUTPUT >= 0x00FF_FFFF);
        assert!(check_output_len(lz77_header(header(0x10, 0x00FF_FFFF))).is_ok());
    }
}
