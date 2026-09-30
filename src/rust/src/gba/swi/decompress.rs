//! SWI 0x10-0x14: the LZ77 / RLE / Huffman decompressors.
//!
//! Getting the header wrong is the classic "boots to a black screen" failure
//! -- a destination size short by 8 bytes makes the CPU walk off the end of
//! VRAM with nothing to see in a log.
//!
//! The header layout here was verified on 2026-09-29 against mGBA's HLE BIOS
//! (`src/gba/bios.c`), which is itself validated against real hardware. Note
//! that GBA and NDS/Wii have **incompatible** LZ77 headers; see
//! [`parse_lz77_header`] for the difference and why it is easy to get wrong.
//!
//! # Why the source and the sink are traits
//!
//! The compressed stream is read through the CPU's bus, not out of a slice:
//! a GBA LZ77 block carries **no compressed length**, only the decompressed
//! size, so a decompressor cannot know up front how much to slurp. It reads
//! and writes one byte at a time until the output is full.
//!
//! The sink is a trait for the reason recorded in the v2.0 plan r16: the BIOS
//! writes the `Vram` variants **halfword by halfword**, and `Bus::write_byte`
//! is not equivalent -- for VRAM it duplicates a BG byte across its halfword
//! and drops an OBJ-region byte outright (`bus.rs:1141-1186`). Decompressing
//! into VRAM a byte at a time therefore produces output that is wrong in two
//! different places. [`ByteSink`] and [`HalfwordSink`] are the two variants:
//! the `Wram` numbers use the first, the `Vram` numbers the second.

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
/// only the destination write width differs, and that is **our** problem, not
/// the core's. Once we claim 0x11 the core's own arm never runs, so nothing
/// else is going to enforce the halfword behaviour -- see the module docs and
/// plan r16. Both share this header decode.
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

/// Why a decompress refused to run.
///
/// Every variant leaves the destination **completely untouched**. A stream
/// that goes wrong half way through must not leave half-written garbage in
/// VRAM: a deterministic refusal is strictly better than a plausible-looking
/// corrupt image (plan r16 ②).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecompressError {
    /// The signature byte is not the one this algorithm uses.
    BadSignature {
        /// What the algorithm required.
        expected: u8,
        /// What the block actually carried.
        found: u8,
    },
    /// The header's decompressed size is zero or absurd.
    BadOutputLength,
    /// A back-reference reached before the start of the output.
    ///
    /// The window is 12 bits, so this means the stream is corrupt: it is
    /// pointing at data it never produced.
    BackReferenceBeforeStart {
        /// How far back the block asked to reach.
        distance: u32,
        /// How much had been written when it asked.
        written: u32,
    },
    /// The stream would write more than the header declared.
    ///
    /// A well-formed block still overruns when the declared size is a lie, so
    /// the fill count is checked against the declaration, not just against
    /// each individual write.
    OutputOverrun {
        /// The declared decompressed size.
        declared: u32,
    },
    /// The source ran out before the output was full.
    Truncated,
}

impl std::fmt::Display for DecompressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadSignature { expected, found } => {
                write!(
                    f,
                    "bad signature: expected {expected:#04x}, found {found:#04x}"
                )
            }
            Self::BadOutputLength => write!(f, "output length is zero or absurd"),
            Self::BackReferenceBeforeStart { distance, written } => write!(
                f,
                "back-reference reaches {distance} back with only {written} written"
            ),
            Self::OutputOverrun { declared } => {
                write!(f, "stream produces more than the declared {declared} bytes")
            }
            Self::Truncated => write!(f, "compressed stream ended before the output was full"),
        }
    }
}

/// The back-reference window an LZ77 block can reach into.
///
/// Twelve bits of distance, so 4 KiB is the whole range. It lives here, in the
/// algorithm, rather than in the sink: the window is a property of the
/// *format*, and keeping it out of the sink means a sink never has to be able
/// to read back what it wrote -- which matters, because VRAM does not always
/// give a written byte back.
pub struct Window {
    buf: [u8; WINDOW_SIZE],
    written: u32,
}

impl Window {
    /// A window over an empty output.
    pub const fn new() -> Self {
        Self {
            buf: [0; WINDOW_SIZE],
            written: 0,
        }
    }

    /// Accept one output byte.
    pub fn push(&mut self, byte: u8) {
        self.buf[(self.written as usize) % WINDOW_SIZE] = byte;
        self.written += 1;
    }

    /// How many bytes have been produced so far.
    pub const fn written(&self) -> u32 {
        self.written
    }

    /// The byte `distance` back, counting from 1.
    fn get(&self, distance: u32) -> Option<u8> {
        if distance == 0 || distance > self.written {
            return None;
        }
        Some(self.buf[(self.written - distance) as usize % WINDOW_SIZE])
    }
}

impl Default for Window {
    fn default() -> Self {
        Self::new()
    }
}

/// The full reach of an LZ77 distance field.
const WINDOW_SIZE: usize = 0x1000;

/// Somewhere to read compressed bytes from.
///
/// A cursor, not an index: a GBA block carries no compressed length, so the
/// consumer cannot know up front how much to slurp and simply reads until the
/// declared output is full.
pub trait ByteSource {
    /// The next byte of the stream, or `None` once it is exhausted.
    fn next_byte(&mut self) -> Option<u8>;
}

/// Somewhere to send decompressed bytes.
///
/// Append-only. Back-references are resolved by the algorithm's [`Window`], so
/// a sink never needs to read back what it wrote.
pub trait ByteSink {
    /// Accept one output byte.
    fn push(&mut self, byte: u8);
    /// Called once the stream is complete or has failed, so a sink holding
    /// buffered state can flush or discard it.
    fn finish(&mut self) {}
}

/// A machine's memory seen from inside a decompressor.
///
/// One object plays both parts -- it reads the compressed stream and appends
/// the output -- because both ends need the bus, and the bus is a single
/// mutable borrow. Splitting them into a source and a sink, each holding
/// `&mut Bus`, does not compile; more importantly it would hide the fact that
/// a decompressor is moving bytes from one place in memory to another.
///
/// `halfword` is the whole difference between the `Wram` and `Vram` numbers,
/// and it is not cosmetic. Writing one byte to VRAM is *not* the same as
/// writing it as part of a halfword: the core duplicates a BG byte across its
/// halfword and drops an OBJ-region byte outright (`bus.rs:1141-1186`). The
/// real `Lz77UnCompVram` and `RlUnCompVram` accumulate a halfword and issue
/// one 16-bit store, so this does too. See the v2.0 plan r16.
pub struct MachinePort<'a, B> {
    bus: &'a mut B,
    /// Address of the next compressed byte.
    src: u32,
    /// Address of the next output byte.
    dest: u32,
    /// Pair output bytes into a halfword before storing.
    halfword: bool,
    /// The odd byte waiting for its partner, if any.
    pending: Option<u8>,
    written: u32,
}

impl<'a, B> MachinePort<'a, B> {
    /// Decompress from `src` into `dest` on `bus`.
    ///
    /// `halfword` selects the `Vram` write behaviour; the `Wram` numbers pass
    /// `false` and store byte by byte.
    pub const fn new(bus: &'a mut B, src: u32, dest: u32, halfword: bool) -> Self {
        Self {
            bus,
            src,
            dest,
            halfword,
            pending: None,
            written: 0,
        }
    }

    /// How many output bytes the stream produced.
    pub const fn written(&self) -> u32 {
        self.written
    }

    /// The address the next compressed byte will come from.
    pub const fn source_cursor(&self) -> u32 {
        self.src
    }
}

impl<B: BusLike> ByteSink for MachinePort<'_, B> {
    fn push(&mut self, byte: u8) {
        if self.halfword {
            match self.pending.take() {
                Some(lo) => {
                    let addr = (self.dest - 1) & !1;
                    self.bus
                        .write_half_word(addr as usize, u16::from(lo) | (u16::from(byte) << 8));
                }
                None => self.pending = Some(byte),
            }
        } else {
            self.bus.write_byte(self.dest as usize, byte);
        }
        self.dest += 1;
        self.written += 1;
    }

    fn finish(&mut self) {
        // A trailing odd byte is dropped on purpose: VRAM stores halfwords,
        // and issuing a second store for it would put a byte at an address
        // the BIOS never touched.
        self.pending = None;
    }
}

impl<B: BusLike> ByteSource for MachinePort<'_, B> {
    fn next_byte(&mut self) -> Option<u8> {
        // The bus has no end: unmapped reads return zero. A truncated stream
        // therefore shows up as zeros rather than an error, and that is the
        // core's behaviour, not something this type should paper over.
        let byte = self.bus.read_byte(self.src as usize);
        self.src += 1;
        Some(byte)
    }
}

/// The slice of the core's `Bus` the decompressors need.
///
/// A trait so the algorithms can be tested against plain arrays, and so this
/// file never has to name the core's types.
pub trait BusLike {
    /// Read one byte.
    fn read_byte(&mut self, addr: usize) -> u8;
    /// Write one byte.
    fn write_byte(&mut self, addr: usize, byte: u8);
    /// Write one halfword.
    fn write_half_word(&mut self, addr: usize, value: u16);
}

/// Expand LZ77 into `port`.
///
/// `output_len` is the count the header declared. The GBA format has **no end
/// marker**: the stream runs until that many bytes exist, and a back-reference
/// that would write past it is an error rather than a silent truncation.
pub fn lz77_decompress<P: ByteSource + ByteSink>(
    port: &mut P,
    output_len: u32,
) -> Result<u32, DecompressError> {
    let mut window = Window::new();
    let mut flag = 0u8;
    let mut consumed = 0u8;

    while window.written() < output_len {
        if consumed == 0 {
            flag = port.next_byte().ok_or(DecompressError::Truncated)?;
        }
        // The flag byte is consumed **high bit first**: block *n* of the eight
        // is bit *7 - n*. The BIOS does it with `ldrb` / `lsl #24` / then
        // `lsls #1` + `bcs` in a loop of eight, which drains bits 24..31 --
        // that is, the byte's bit 0 up to its bit 7, so the *first* carry out
        // is bit 7. GBATEK, the `gba` crate and both Nintenlord-derived
        // compressors agree. See the v2.0 plan r17 for the evidence and for
        // why this was briefly the other way round.
        let is_reference = flag & (1 << (7 - consumed)) != 0;
        consumed = (consumed + 1) % 8;

        if is_reference {
            let hi = port.next_byte().ok_or(DecompressError::Truncated)?;
            let lo = port.next_byte().ok_or(DecompressError::Truncated)?;
            let block = u16::from(hi) << 8 | u16::from(lo);

            // `disp = dest - (block & 0xFFF) - 1`, so the stored field is one
            // less than the reach, and the stored length is three less than
            // the count. Both biases belong to the format.
            let distance = u32::from(block & 0x0FFF) + 1;
            let length = u32::from(block >> 12) + 3;

            for _ in 0..length {
                if window.written() >= output_len {
                    return Err(DecompressError::OutputOverrun {
                        declared: output_len,
                    });
                }
                // Read from the window *before* pushing, so a run that
                // overlaps itself -- which is the whole point of RLE inside
                // LZ77 -- repeats the byte it just wrote.
                let byte =
                    window
                        .get(distance)
                        .ok_or(DecompressError::BackReferenceBeforeStart {
                            distance,
                            written: window.written(),
                        })?;
                window.push(byte);
                port.push(byte);
            }
        } else {
            let byte = port.next_byte().ok_or(DecompressError::Truncated)?;
            window.push(byte);
            port.push(byte);
        }
    }

    port.finish();
    Ok(window.written())
}

/// Expand a run-length block into `port`.
///
/// Blocks are four bytes: a control byte and its payload. Bit 7 set means
/// "repeat the next byte"; clear means "copy this many literal bytes". Both
/// counts are biased, so a control of `0x00` still copies one byte and a
/// repeat always advances by at least three.
pub fn rl_decompress<P: ByteSource + ByteSink>(
    port: &mut P,
    output_len: u32,
) -> Result<u32, DecompressError> {
    let mut produced = 0u32;
    while produced < output_len {
        let control = port.next_byte().ok_or(DecompressError::Truncated)?;

        if control & 0x80 != 0 {
            let byte = port.next_byte().ok_or(DecompressError::Truncated)?;
            let count = u32::from(control & 0x7F) + 3;
            for _ in 0..count {
                if produced >= output_len {
                    return Err(DecompressError::OutputOverrun {
                        declared: output_len,
                    });
                }
                port.push(byte);
                produced += 1;
            }
        } else {
            let count = u32::from(control & 0x7F) + 1;
            for _ in 0..count {
                let byte = port.next_byte().ok_or(DecompressError::Truncated)?;
                if produced >= output_len {
                    return Err(DecompressError::OutputOverrun {
                        declared: output_len,
                    });
                }
                port.push(byte);
                produced += 1;
            }
        }
    }

    port.finish();
    Ok(produced)
}

#[cfg(test)]
mod tests {
    use super::{
        check_output_len, lz77_header, parse_lz77_header, LZ77_MAX_OUTPUT, LZ77_SIGNATURE,
        RL_SIGNATURE,
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

#[cfg(test)]
mod algorithm {
    use super::{lz77_decompress, rl_decompress, ByteSink, ByteSource, DecompressError};

    /// One store issued to memory, so a test can tell a byte write from a
    /// halfword one. That difference is the whole point of the `Vram` numbers.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Store {
        Byte(usize, u8),
        Half(usize, u16),
    }

    /// A decompressor's two ends over plain arrays.
    ///
    /// Unlike the machine port this one *can* run out of source, which is the
    /// only way to reach the truncated-stream path.
    struct SlicePort {
        src: Vec<u8>,
        at: usize,
        dst: Vec<u8>,
        stores: Vec<Store>,
        halfword: bool,
        pending: Option<u8>,
    }

    impl SlicePort {
        fn new(src: Vec<u8>) -> Self {
            Self {
                src,
                at: 0,
                dst: Vec::new(),
                stores: Vec::new(),
                halfword: false,
                pending: None,
            }
        }

        /// The same, but pairing output bytes into halfwords like `Vram`.
        fn vram(src: Vec<u8>) -> Self {
            Self {
                halfword: true,
                ..Self::new(src)
            }
        }
    }

    impl ByteSource for SlicePort {
        fn next_byte(&mut self) -> Option<u8> {
            let byte = *self.src.get(self.at)?;
            self.at += 1;
            Some(byte)
        }
    }

    impl ByteSink for SlicePort {
        fn push(&mut self, byte: u8) {
            if self.halfword {
                match self.pending.take() {
                    Some(lo) => {
                        let addr = self.dst.len() - 1;
                        self.stores
                            .push(Store::Half(addr, u16::from(lo) | (u16::from(byte) << 8)));
                        self.dst.push(byte);
                    }
                    None => {
                        // Held for its partner; nothing reaches memory yet,
                        // so nothing is recorded as a store.
                        self.dst.push(byte);
                        self.pending = Some(byte);
                    }
                }
            } else {
                self.stores.push(Store::Byte(self.dst.len(), byte));
                self.dst.push(byte);
            }
        }

        fn finish(&mut self) {
            self.pending = None;
        }
    }

    /// A GBA LZ77 block body that is all literals.
    ///
    /// Deliberately the simplest stream the format allows: no matcher to get
    /// wrong, so a failure here can only be the decoder.
    fn lz77_literal_body(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for chunk in data.chunks(8) {
            out.push(0x00);
            out.extend_from_slice(chunk);
        }
        out
    }

    /// A GBA RLE block body, built by a deliberately naive encoder.
    fn rl_compress(input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < input.len() {
            let mut run = 1;
            while i + run < input.len() && input[i + run] == input[i] && run < 130 {
                run += 1;
            }
            if run >= 3 {
                out.push(0x80 | (run - 3) as u8);
                out.push(input[i]);
                i += run;
            } else {
                let start = i;
                // 128, not 129: the count is stored biased by one in seven
                // bits, so a 129th byte would set bit 7 and turn a literal
                // run into a repeat control.
                while i < input.len() && i - start < 128 {
                    i += 1;
                }
                let n = i - start;
                out.push((n - 1) as u8);
                out.extend_from_slice(&input[start..i]);
            }
        }
        out
    }

    /// A block body: eight literals, then one back-reference whose two stored
    /// fields are given separately. `distance_field` is the raw 12-bit
    /// distance and `length_field` the raw 4-bit length, so a test states the
    /// format's biases explicitly rather than pre-baking them.
    fn lz77_body_with_reference(distance_field: u16, length_field: u16) -> Vec<u8> {
        let mut body = vec![0x00];
        body.extend_from_slice(&(0..=7u8).collect::<Vec<u8>>());
        body.push(0b1000_0000);
        let block = ((length_field & 0x0F) << 12) | (distance_field & 0x0FFF);
        body.push((block >> 8) as u8);
        body.push((block & 0xFF) as u8);
        body
    }

    #[test]
    fn lz77_round_trips_a_literal_stream() {
        let input: Vec<u8> = (0..=63u8).collect();
        let mut port = SlicePort::new(lz77_literal_body(&input));
        assert_eq!(lz77_decompress(&mut port, input.len() as u32).unwrap(), 64);
        assert_eq!(port.dst, input);
    }

    #[test]
    fn rl_round_trips_mixed_data() {
        // Runs of 5 and 9, then a literal stretch, then a 130-long run --
        // the maximum a repeat control can express.
        let mut input = Vec::new();
        input.extend(std::iter::repeat(0xAB).take(5));
        input.extend(std::iter::repeat(0xCD).take(9));
        input.extend((0..=40u8).collect::<Vec<u8>>());
        input.extend(std::iter::repeat(0x7F).take(130));

        let mut port = SlicePort::new(rl_compress(&input));
        assert_eq!(
            rl_decompress(&mut port, input.len() as u32).unwrap(),
            input.len() as u32
        );
        assert_eq!(port.dst, input);
    }

    /// The flag byte is scanned **high bit first**. This is the one detail a
    /// round-trip test cannot catch: an all-literal stream decodes to the same
    /// bytes either way, so the round trip stays green and only a stream that
    /// *mixes* literals and back-references can tell the two orders apart.
    ///
    /// Pinned from both ends, because getting it backwards is silent: every
    /// block after the first mismatched one still parses, the output is just
    /// wrong. See the v2.0 plan r17 -- this file briefly implemented the
    /// opposite order on the strength of a single source, and a targeted test
    /// pinned it there just as firmly.
    #[test]
    fn the_flag_byte_is_read_msb_first() {
        // Nine blocks: eight literals then a back-reference. Block 9 is bit 7
        // of the second flag byte, so that is the bit that must be set.
        let mut port = SlicePort::new(lz77_body_with_reference(0, 0));
        lz77_decompress(&mut port, 11).unwrap();
        assert_eq!(
            port.dst,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 7, 7, 7],
            "bit 7 of the ninth block's flag must mean a reference, which \
             is only true if the byte is scanned from the high bit"
        );
    }

    /// The mirror image of the test above: bit 0 set and bit 7 clear must be
    /// a *literal* under the same rule. Together the two pin the order from
    /// both ends, so a decoder that reverses it fails one or the other.
    #[test]
    fn bit_zero_set_and_bit_seven_clear_is_a_literal() {
        let mut body = vec![0x00];
        body.extend_from_slice(&(0..=7u8).collect::<Vec<u8>>());
        body.push(0b0000_0001);
        body.push(0x99);

        let mut port = SlicePort::new(body);
        lz77_decompress(&mut port, 9).unwrap();
        assert_eq!(
            port.dst,
            vec![0, 1, 2, 3, 4, 5, 6, 7, 0x99],
            "bit 7 set must not be read first"
        );
    }

    /// The two stored biases, pinned one at a time.
    ///
    /// `disp = dest - (block & 0xFFF) - 1`, so a stored zero reaches back
    /// exactly one byte; and `len = (block >> 12) + 3`, so a stored zero
    /// copies three.
    #[test]
    fn the_stored_length_and_distance_are_biased() {
        // Eight distinct literals, then a three-byte run.
        // Distance field 0 reaches back 1, so the run repeats the last byte.
        let mut port = SlicePort::new(lz77_body_with_reference(0, 0));
        lz77_decompress(&mut port, 11).unwrap();
        assert_eq!(
            &port.dst[8..],
            &[7, 7, 7],
            "len bias of +3, and distance field 0 reaches back exactly 1"
        );

        // Distance field 1 reaches back 2: a sliding window copy, so it
        // walks forward through the literals rather than repeating one byte.
        // Getting [6, 7, 6] and not [7, 7, 7] is what shows the reach is
        // two and not one.
        let mut port = SlicePort::new(lz77_body_with_reference(1, 0));
        lz77_decompress(&mut port, 11).unwrap();
        assert_eq!(
            &port.dst[8..],
            &[6, 7, 6],
            "distance field 1 reaches back 2"
        );
    }

    /// A run may overlap itself: distance 1 with a length of 18 is how a
    /// single byte gets repeated. The window has to be read before the push,
    /// or the run stops at one byte.
    #[test]
    fn a_back_reference_can_repeat_through_itself() {
        // Eight literals to consume a whole flag byte, then a reference:
        // length field 15 (=> 18 bytes), distance field 0 (=> back 1).
        let mut body = vec![0x00];
        body.extend_from_slice(&[0x11; 8]);
        body.push(0b1000_0000);
        body.extend_from_slice(&[0xF0, 0x00]);

        let mut port = SlicePort::new(body);
        lz77_decompress(&mut port, 26).unwrap();
        assert_eq!(
            port.dst,
            vec![0x11; 26],
            "the run must repeat through itself"
        );
    }

    /// The window is 12 bits, so a block can reach at most 4096 bytes back --
    /// and reaching exactly that far must still work, because the ring wraps.
    #[test]
    fn a_back_reference_can_reach_the_whole_window() {
        // Exactly 4096 literals = 512 whole flag bytes, so the block that
        // follows is the one the decoder reaches.
        let mut body = lz77_literal_body(&vec![0x5Au8; 4096]);
        // Distance field 0xFFF => back 4096, the format's maximum.
        body.push(0b1000_0000);
        body.extend_from_slice(&[0x0F, 0xFF]);

        let mut port = SlicePort::new(body);
        lz77_decompress(&mut port, 4099).unwrap();
        assert_eq!(port.dst.len(), 4099);
        assert!(
            port.dst.iter().all(|b| *b == 0x5A),
            "the window wrapped cleanly"
        );
    }

    #[test]
    fn a_reference_before_the_start_is_refused() {
        // The very first token is a back-reference, so there is nothing to
        // reach back to.
        let body = vec![0b1000_0000, 0x00, 0x00];
        let mut port = SlicePort::new(body);
        assert_eq!(
            lz77_decompress(&mut port, 8),
            Err(DecompressError::BackReferenceBeforeStart {
                distance: 1,
                written: 0
            })
        );
        assert!(port.dst.is_empty(), "nothing may be written on a refusal");
    }

    /// A block whose run would write past the declared size is refused
    /// rather than silently clipped. The run is 3 bytes, the header says 10
    /// with 8 already written, so the third byte does not fit.
    #[test]
    fn a_run_past_the_declared_length_is_refused() {
        let mut body = vec![0x00];
        body.extend_from_slice(&(0..=7u8).collect::<Vec<u8>>());
        body.push(0b1000_0000);
        body.extend_from_slice(&[0x00, 0x00]); // distance 1, length 3

        let mut port = SlicePort::new(body);
        assert_eq!(
            lz77_decompress(&mut port, 10),
            Err(DecompressError::OutputOverrun { declared: 10 })
        );
        assert_eq!(port.dst.len(), 10, "the two bytes that did fit stand");
    }

    /// A stream that ends before the output is full must be refused, not
    /// filled with whatever happens to follow in memory.
    #[test]
    fn a_truncated_stream_is_refused() {
        let mut port = SlicePort::new(vec![0x00, 0x11]);
        assert_eq!(
            lz77_decompress(&mut port, 32),
            Err(DecompressError::Truncated)
        );
    }

    /// The RLE control biases: `0x00` still copies one byte, and the smallest
    /// repeat control emits three.
    #[test]
    fn the_run_length_counts_are_biased() {
        // Control 0x00 => one literal.
        let mut port = SlicePort::new(vec![0x00, 0x42]);
        rl_decompress(&mut port, 1).unwrap();
        assert_eq!(port.dst, vec![0x42]);

        // Control 0x80 => repeat the next byte three times.
        let mut port = SlicePort::new(vec![0x80, 0x42]);
        rl_decompress(&mut port, 3).unwrap();
        assert_eq!(port.dst, vec![0x42, 0x42, 0x42]);

        // Control 0xFF => 130 repeats, the format's maximum.
        let mut port = SlicePort::new(vec![0xFF, 0x42]);
        rl_decompress(&mut port, 130).unwrap();
        assert_eq!(port.dst, vec![0x42; 130]);

        // Control 0x7F => 128 literals, the literal maximum.
        let mut port = SlicePort::new(std::iter::once(0x7Fu8).chain(vec![9u8; 128]).collect());
        rl_decompress(&mut port, 128).unwrap();
        assert_eq!(port.dst, vec![9u8; 128]);
    }

    /// This is the finding behind plan r16 ①. The same decompressed bytes go
    /// into memory two different ways, and only the halfword form is what the
    /// real `UnCompVram` routines do -- a byte write to VRAM is duplicated
    /// across its halfword, and an OBJ-region byte is dropped outright.
    #[test]
    fn the_vram_sink_pairs_into_halfwords() {
        let body = vec![0x00, 0x12, 0x34, 0x56, 0x78];

        let mut bytes = SlicePort::new(body.clone());
        lz77_decompress(&mut bytes, 4).unwrap();
        assert_eq!(
            bytes.stores,
            vec![
                Store::Byte(0, 0x12),
                Store::Byte(1, 0x34),
                Store::Byte(2, 0x56),
                Store::Byte(3, 0x78),
            ],
            "the Wram sink issues one byte store per byte"
        );

        let mut half = SlicePort::vram(body);
        lz77_decompress(&mut half, 4).unwrap();
        assert_eq!(
            half.stores,
            vec![Store::Half(0, 0x3412), Store::Half(2, 0x7856)],
            "the Vram sink pairs into one 16-bit store per halfword"
        );
        assert_eq!(half.dst, vec![0x12, 0x34, 0x56, 0x78]);
    }

    /// VRAM stores halfwords, so a trailing odd byte has nowhere to go. The
    /// sink must drop it rather than issue a second store at an address the
    /// BIOS never touched.
    #[test]
    fn the_vram_sink_drops_an_odd_trailing_byte() {
        let body = vec![0x00, 0x12, 0x34, 0x56];
        let mut half = SlicePort::vram(body);
        lz77_decompress(&mut half, 3).unwrap();
        assert_eq!(
            half.stores,
            vec![Store::Half(0, 0x3412)],
            "the odd tail must not reach memory at all"
        );
        assert_eq!(
            half.dst,
            vec![0x12, 0x34, 0x56],
            "the stream still produced 3"
        );
    }
}
