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

/// Parse a 24-bit LZ77 header word.
///
/// The low 24 bits are the compressed length. Bit 31 says whether the second
/// field counts bytes or 8-byte units. Bit 30 is reserved and ignored, which
/// is what the BIOS does.
pub fn parse_lz77_header(raw: u32) -> Lz77Header {
    let compressed_len = raw & 0x00FF_FFFF;
    let output_field = raw & 0x0FFF_FFFF;
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
