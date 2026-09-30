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
    /// The element width is not one the format can express.
    ///
    /// The bitstream is consumed 32 bits at a time, so a width that does not
    /// divide 32 has no well-defined place to stop.
    BadElementWidth(u8),
    /// The node table does not contain the node or data byte the walk asked
    /// for -- a corrupt tree, or a truncation.
    BadTree,
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
            Self::BadElementWidth(bits) => write!(
                f,
                "element width {bits} does not divide 32, so the bitstream has no \
                 well-defined boundary"
            ),
            Self::BadTree => write!(f, "node table does not contain the requested node"),
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
    /// Read one word, little-endian, as the bus presents it at a 4-aligned
    /// address.
    fn read_word(&mut self, addr: usize) -> u32;
    /// Write one word, little-endian.
    fn write_word(&mut self, addr: usize, value: u32);
}

/// Somewhere to send whole words. Huffman is the only algorithm here that
/// works in 32-bit units -- it has no Wram/Vram split, because VRAM *is* word
/// addressed.
pub trait WordSink {
    /// Accept one 32-bit output word.
    fn push_word(&mut self, word: u32);
    /// Called once the stream is complete or has failed.
    fn finish(&mut self) {}
}

/// `HuffUnComp`'s memory: a byte cursor for the header and tree, a word
/// cursor for the bitstream, and a word cursor for the output.
///
/// The bitstream has to be read as words rather than bytes because the format
/// stores it in 32-bit units and the BIOS reads it that way -- reading it a
/// byte at a time and reassembling would be a second, silently different
/// interpretation of the same bytes.
pub struct HuffmanPort<'a, B> {
    bus: &'a mut B,
    /// Next compressed byte (header and tree only).
    src: u32,
    /// Next bitstream word.
    bitstream: u32,
    /// Next output word.
    dest: u32,
    written: u32,
}

impl<'a, B> HuffmanPort<'a, B> {
    /// Decompress from `src` into `dest` on `bus`.
    pub const fn new(bus: &'a mut B, src: u32, dest: u32) -> Self {
        Self {
            bus,
            src,
            bitstream: src,
            dest: dest & !3,
            written: 0,
        }
    }

    /// Move the compressed byte cursor and the bitstream cursor together: the
    /// bitstream starts immediately after the tree, which is the last thing the
    /// byte cursor reads.
    pub const fn source_cursor(&self) -> u32 {
        self.src
    }

    /// How many output bytes the stream produced.
    pub const fn written(&self) -> u32 {
        self.written
    }
}

impl<B: BusLike> ByteSource for HuffmanPort<'_, B> {
    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.bus.read_byte(self.src as usize);
        self.src += 1;
        Some(byte)
    }
}

/// What the Huffman walker needs from a machine: a byte cursor for the header
/// and the node table, a **word** cursor for the bitstream, and a word sink.
///
/// A trait rather than the concrete [`HuffmanPort`] so the walk can be driven
/// against plain arrays in tests -- and the reason it exists at all is that
/// the bitstream is a sequence of 32-bit units, not a byte stream. Reading it
/// a byte at a time and reassembling would be a second, silently different
/// reading of the same bytes.
pub trait HuffmanPortLike: ByteSource {
    /// Where the next compressed byte will come from -- after the header and
    /// the node table have been read, that is where the bitstream begins.
    fn source_cursor(&self) -> u32;
    /// The next bitstream word, little-endian as the bus presents it.
    fn next_bitstream_word(&mut self) -> u32;
    /// Where the bitstream begins, once the node table has been read.
    fn seek_bitstream(&mut self, addr: u32);
    /// Accept one 32-bit output word.
    fn push_word(&mut self, word: u32);
    /// Called once the stream is complete or has failed.
    fn finish(&mut self) {}
}

impl<B: BusLike> HuffmanPortLike for HuffmanPort<'_, B> {
    fn source_cursor(&self) -> u32 {
        self.src
    }

    fn next_bitstream_word(&mut self) -> u32 {
        let word = self.bus.read_word(self.bitstream as usize);
        self.bitstream += 4;
        word
    }

    fn seek_bitstream(&mut self, addr: u32) {
        self.bitstream = addr;
    }

    fn push_word(&mut self, word: u32) {
        self.bus.write_word(self.dest as usize, word);
        self.dest += 4;
        self.written += 4;
    }
}

/// The decoded shape of a Huffman header word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HuffmanHeader {
    /// The type nibble: `0x20` for Huffman. It shares byte 0 with the
    /// element width, unlike LZ77 and RLE where the byte is a plain
    /// signature.
    pub signature: u8,
    /// Bits per data element. Only widths that divide 32 are legal, so in
    /// practice 1, 2, 4, 8, 16 or 32.
    pub element_bits: u8,
    /// Decompressed size in **bytes** -- not elements. This is not a
    /// transcription of a spec sentence: the BIOS decrements this count by 4
    /// for every 32-bit word it stores, so it is measured in the same unit
    /// the store loop moves in. For 4-bit elements the element count is
    /// twice this number (plan r21 ①).
    pub output_len: u32,
}

/// Parse a GBA Huffman header word.
///
/// Byte 0 is `0x20 | element_bits`, so the signature is the top nibble and
/// the element width is the bottom one.
pub fn parse_huffman_header(raw: u32) -> HuffmanHeader {
    HuffmanHeader {
        signature: (raw & 0xF0) as u8,
        element_bits: (raw & 0x0F) as u8,
        output_len: (raw >> 8) & 0x00FF_FFFF,
    }
}

/// The Huffman signature with a zero element width.
pub const HUFFMAN_TYPE: u8 = 0x20;

/// The length of the node table, from the single byte that follows the header.
///
/// `(byte << 1) + 1` -- always **odd**. That is not a quirk: the layout is
/// `[header 4][tree length 1][tree N][bitstream]` and the bitstream is read as
/// 32-bit words, so `source + 5 + N` has to be 4-byte aligned, which forces
/// `N` odd. The reading that gives an even `N` cannot produce an aligned
/// bitstream at all. See plan r22 ①; this is the fourth time a spec sentence
/// was overturned by an internal-consistency check rather than by a vote.
pub const fn tree_size_byte(byte: u8) -> u32 {
    ((byte as u32) << 1) + 1
}

/// One decoded symbol's worth of state, carried between bitstream words.
///
/// `filled` counts from the **low** end: the first element decoded lands in
/// the lowest bits and each subsequent one shifts up, and the word is stored
/// once 32 bits have accumulated (plan r21 ②).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OutputWord {
    filled: u8,
    value: u32,
}

impl OutputWord {
    const fn empty() -> Self {
        Self {
            filled: 0,
            value: 0,
        }
    }
}

/// Expand a `HuffUnComp` (0x12) block into `port`.
///
/// The tree is walked one bit at a time: `0` goes to child 0, `1` to child 1,
/// and a child the parent marked terminal holds the data rather than another
/// node. A terminal child's byte is the symbol itself, masked to the element
/// width -- the upper bits are meaningless for elements narrower than a byte.
pub fn huffman_decompress<P: HuffmanPortLike>(
    port: &mut P,
    element_bits: u8,
    output_len: u32,
) -> Result<u32, DecompressError> {
    if element_bits == 0 || element_bits > 32 || 32 % element_bits != 0 {
        return Err(DecompressError::BadElementWidth(element_bits));
    }
    if output_len == 0 {
        return Err(DecompressError::BadOutputLength);
    }

    // The node table. Its length is forced odd so the bitstream lands aligned;
    // see `tree_size_byte`.
    let tree_len = tree_size_byte(port.next_byte().ok_or(DecompressError::Truncated)?);
    let mut tree = Vec::with_capacity(tree_len as usize);
    for _ in 0..tree_len {
        tree.push(port.next_byte().ok_or(DecompressError::Truncated)?);
    }
    port.seek_bitstream(port.source_cursor());

    // Output is produced in whole 32-bit units, but the declared length need
    // not be a multiple of four -- so progress is counted in **symbols**, not
    // in words. Counting words and rounding the length down would silently
    // skip the whole walk for a 1-to-3 byte output.
    let symbols = (output_len * 8) / u32::from(element_bits);
    let mut emitted = 0u32;
    let mut word = OutputWord::empty();
    let mut node = 0usize;

    while emitted < symbols {
        let bitstream = port.next_bitstream_word();
        // The bitstream is padded to whole 32-bit units, so its last word
        // usually carries more bits than the declared output still needs.
        // The walk stops the moment the output is full: decoding the rest of
        // the word would store whole extra words past the end of the caller's
        // buffer, which in VRAM means overwriting whatever is next.
        'bits: for shift in 0..32u32 {
            let node_byte = *tree.get(node).ok_or(DecompressError::BadTree)?;
            // `(current AND NOT 1) + offset*2 + 2`, in the tree's own index
            // space. The tree starts one byte into a 4-aligned block, so its
            // first address is odd and `AND NOT 1` lands one index below the
            // root -- which is what the `node - 1` here reproduces. Signed,
            // because for the root that subtraction goes negative.
            let base = (((node as isize - 1) | 1) + isize::from(node_byte & 0x3F) * 2 + 2)
                .try_into()
                .map_err(|_| DecompressError::BadTree)?;

            let goes_right = bitstream & (1 << (31 - shift)) != 0;
            let child = if goes_right { base + 1 } else { base };
            let terminal = if goes_right {
                node_byte & 0x40 != 0
            } else {
                node_byte & 0x80 != 0
            };

            if terminal {
                // The child's byte is the data, not a node. Its upper bits
                // are meaningless when the element is narrower than a byte,
                // so the write masks them off.
                let data = *tree.get(child).ok_or(DecompressError::BadTree)?;
                let mask = ((1u32 << element_bits) - 1) as u8;
                word.value |= u32::from(data & mask) << word.filled;
                word.filled += element_bits;
                emitted += 1;
                if word.filled >= 32 {
                    port.push_word(word.value);
                    word = OutputWord::empty();
                }
                node = 0;
                if emitted == symbols {
                    break 'bits;
                }
            } else {
                node = child;
            }
        }
    }

    // Whatever is left in the accumulator is the zero-padded tail of a
    // sub-word output, and it is stored -- VRAM only ever holds halfwords and
    // words, so there is nowhere else for it to go.
    if word.filled > 0 {
        port.push_word(word.value);
    }
    port.finish();
    Ok(output_len)
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

#[cfg(test)]
mod huffman {
    use super::{
        huffman_decompress, parse_huffman_header, tree_size_byte, ByteSource, DecompressError,
        HuffmanPortLike, HUFFMAN_TYPE,
    };

    /// A port over two plain arrays.
    ///
    /// The output is collected as **words**, not bytes: this algorithm's write
    /// width is the whole difference between right and subtly wrong, so a
    /// byte-oriented fixture could not tell them apart.
    struct SliceHuff {
        src: Vec<u8>,
        at: usize,
        words: Vec<u32>,
    }

    impl SliceHuff {
        fn new(src: Vec<u8>) -> Self {
            Self {
                src,
                at: 0,
                words: Vec::new(),
            }
        }
    }

    impl ByteSource for SliceHuff {
        fn next_byte(&mut self) -> Option<u8> {
            let byte = *self.src.get(self.at)?;
            self.at += 1;
            Some(byte)
        }
    }

    impl HuffmanPortLike for SliceHuff {
        fn source_cursor(&self) -> u32 {
            self.at as u32
        }

        fn next_bitstream_word(&mut self) -> u32 {
            let mut word = 0u32;
            for i in 0..4 {
                word |= u32::from(*self.src.get(self.at + i).unwrap_or(&0)) << (i * 8);
            }
            self.at += 4;
            word
        }

        fn seek_bitstream(&mut self, addr: u32) {
            self.at = addr as usize;
        }

        fn push_word(&mut self, word: u32) {
            self.words.push(word);
        }
    }

    /// A node table for a two-symbol code where both children of the root are
    /// leaves.
    ///
    /// The root sits at index 0. `(0-1)|1` is `-1`, so `child0 = -1 + 0*2 + 2
    /// = 1` and `child1 = 2` -- indices 1 and 2, which is why the table is
    /// three bytes: one root plus one pair. Bits 7 and 6 of the root mark both
    /// children terminal.
    fn two_symbol_tree(a: u8, b: u8) -> Vec<u8> {
        vec![0b1100_0000, a, b]
    }

    /// A node table where the root's **left** child is a node and its right
    /// child is a leaf, so the walk has to descend and come back.
    ///
    /// Index by index: the root is at 0, and `(0-1)|1` is `-1`, so with a zero
    /// offset field the root's children land at `1 + 0*2 + 2 - 1` -- that is
    /// 1 and 2. Index 1 is the inner node; its own children, with
    /// `(1-1)|1 == 1`, land at `1 + 0*2 + 2` -- 3 and 4. That is five bytes:
    /// one root, then two pairs.
    fn nested_tree(root_right: u8, inner_left: u8, inner_right: u8) -> Vec<u8> {
        // root: right terminal (bit 6), left not terminal (bit 7 clear)
        vec![
            0b0100_0000,
            0b1100_0000,
            root_right,
            inner_left,
            inner_right,
        ]
    }

    /// The length byte that goes with a node table.
    fn length_byte(tree_len: usize) -> u8 {
        assert_eq!(tree_len % 2, 1, "a node table is always odd-length");
        ((tree_len as u32 - 1) / 2) as u8
    }

    /// Bits for a run of one-bit tokens, packed **most significant bit first**
    /// into whole 32-bit words, little-endian on the bus. That is the only
    /// packing the format has.
    fn code_bits(tokens: &[bool]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for chunk in tokens.chunks(32) {
            let mut word = 0u32;
            for (i, bit) in chunk.iter().enumerate() {
                if *bit {
                    word |= 1 << (31 - i);
                }
            }
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes
    }

    /// Assemble a complete block: header, length byte, node table, bitstream.
    fn block(element_bits: u8, output_len: u32, tree: &[u8], tokens: &[bool]) -> Vec<u8> {
        let mut out = vec![HUFFMAN_TYPE | element_bits];
        out.push((output_len & 0xFF) as u8);
        out.push(((output_len >> 8) & 0xFF) as u8);
        out.push(((output_len >> 16) & 0xFF) as u8);
        out.push(length_byte(tree.len()));
        out.extend_from_slice(tree);
        out.extend_from_slice(&code_bits(tokens));
        out
    }

    /// Run the walker over a block and return the output words.
    ///
    /// Four bytes are skipped: the header, which the dispatch layer parses and
    /// passes in as arguments. The walk itself starts at the length byte, so
    /// the port is handed the block one byte short of the table.
    fn run(block: &[u8], element_bits: u8, output_len: u32) -> Result<Vec<u32>, DecompressError> {
        let mut port = SliceHuff::new(block.to_vec());
        for _ in 0..4 {
            port.next_byte();
        }
        huffman_decompress(&mut port, element_bits, output_len)?;
        Ok(port.words)
    }

    /// The header's size field is in **bytes**. For 8-bit elements that is
    /// indistinguishable from a symbol count, so the discriminating case is
    /// 4-bit elements: a block declaring 4 *bytes* holds 8 elements, which is
    /// one word.
    #[test]
    fn the_header_size_is_in_bytes_not_elements() {
        let tree = two_symbol_tree(0xAA, 0xBB);

        // 8-bit: 4 bytes = 4 symbols = one word.
        let b = block(8, 4, &tree, &[false; 4]);
        assert_eq!(run(&b, 8, 4).unwrap().len(), 1);

        // 8-bit: 8 bytes = two words.
        let b = block(8, 8, &tree, &[false; 8]);
        assert_eq!(run(&b, 8, 8).unwrap().len(), 2);

        // 4-bit: 4 bytes = 8 elements = one word. Reading the field as a
        // symbol count would decode only 4 elements and stop mid-word.
        let b = block(4, 4, &tree, &[false; 8]);
        assert_eq!(
            run(&b, 4, 4).unwrap().len(),
            1,
            "4 bytes of 4-bit data is one word"
        );
    }

    /// Elements fill the word from the **low** bit up: the first symbol
    /// decoded lands in the lowest byte. Reading the other way round yields
    /// the same symbols in a different order, so a round trip alone could not
    /// detect it -- this pins the order.
    #[test]
    fn elements_fill_the_word_from_the_low_bit_up() {
        let tree = two_symbol_tree(0x11, 0x22);
        let b = block(8, 4, &tree, &[false, true, false, true]);
        assert_eq!(
            run(&b, 8, 4).unwrap(),
            vec![0x22_11_22_11],
            "the first decoded element must sit in the lowest byte"
        );
    }

    /// The bitstream is consumed **most significant bit first**. Swapping the
    /// two symbols between the two orders is what makes this observable: an
    /// LSB-first decoder would produce the other sequence.
    #[test]
    fn the_bitstream_is_read_msb_first() {
        let tree = two_symbol_tree(0x11, 0x22);
        let b = block(8, 4, &tree, &[true, false, true, false]);
        assert_eq!(
            run(&b, 8, 4).unwrap(),
            vec![0x11_22_11_22],
            "the stream's first bit is bit 31 of the first word"
        );
    }

    /// Descending a level and coming back. A tree where the left child is a
    /// node catches an implementation that treats every child as data.
    #[test]
    fn the_walk_descends_through_non_terminal_children() {
        // The root's left child is a node, so the first token only *descends*
        // and produces nothing; the second reaches its left leaf 0x77. The
        // third returns to the root and takes its right leaf 0x88. So three
        // tokens, five bits, three symbols -- an implementation that treated
        // every child as data would produce a symbol for the first token and
        // come out one byte short.
        let tree = nested_tree(0x88, 0x77, 0x77);
        let b = block(8, 3, &tree, &[false, false, true]);
        let words = run(&b, 8, 3).unwrap();
        assert_eq!(
            words[0] & 0x00FF_FFFF,
            0x0077_88_77,
            "descend, then 0x77, then the root's right leaf 0x88"
        );
    }

    /// The node table length is `(byte << 1) + 1`, always **odd**.
    ///
    /// The alternative, `(byte+1)*2`, is always even -- and an even-length
    /// table puts the bitstream one byte past a 4-byte boundary, where the
    /// BIOS's 32-bit reads would rotate. That is the argument that settled it
    /// (plan r22).
    #[test]
    fn the_node_table_length_is_always_odd() {
        for byte in 0u8..=255 {
            assert_eq!(tree_size_byte(byte) % 2, 1, "byte {byte}");
        }
        assert_eq!(tree_size_byte(0), 1);
        assert_eq!(tree_size_byte(1), 3);
        assert_eq!(tree_size_byte(2), 5);
    }

    /// The consequence, and the argument that settled it. The bitstream sits
    /// at `4 + 1 + table_len` and has to be on a 4-byte boundary, so the table
    /// length must be `3 mod 4`.
    ///
    /// `(byte << 1) + 1` **can** be `3 mod 4`, for every odd byte. The
    /// alternative reading, `(byte+1)*2`, is always even and so is **never**
    /// `3 mod 4` -- it cannot leave the bitstream aligned for any value at
    /// all. That asymmetry, not a vote, is why one reading was rejected.
    #[test]
    fn only_the_odd_length_reading_can_leave_the_bitstream_aligned() {
        for byte in 0u8..=255 {
            let ours = 4 + 1 + tree_size_byte(byte);
            let alternative = 4 + 1 + (u32::from(byte) + 1) * 2;
            assert_ne!(
                alternative % 4,
                0,
                "byte {byte}: the even-length reading happened to align"
            );
            if byte % 2 == 1 {
                assert_eq!(ours % 4, 0, "byte {byte} must leave it aligned");
            }
        }
    }

    /// A width that does not divide 32 has no well-defined place for the
    /// bitstream to stop, so it is refused rather than guessed at.
    #[test]
    fn an_indivisible_element_width_is_refused() {
        for bad in [3u8, 5, 6, 7, 9, 17, 31] {
            let mut port = SliceHuff::new(vec![0; 256]);
            assert_eq!(
                huffman_decompress(&mut port, bad, 8),
                Err(DecompressError::BadElementWidth(bad))
            );
            assert!(port.words.is_empty(), "a refusal must write nothing");
        }
        for good in [1u8, 2, 4, 8, 16, 32] {
            let mut port = SliceHuff::new(vec![0; 256]);
            // All-zero input is not a valid tree, so these fail later -- the
            // point is only that the width itself is accepted.
            assert_ne!(
                huffman_decompress(&mut port, good, 8),
                Err(DecompressError::BadElementWidth(good)),
                "{good} divides 32 and must be accepted"
            );
        }
    }

    /// A zero declared size is refused without touching memory.
    #[test]
    fn a_zero_length_block_is_refused() {
        let mut port = SliceHuff::new(vec![0; 256]);
        assert_eq!(
            huffman_decompress(&mut port, 8, 0),
            Err(DecompressError::BadOutputLength)
        );
        assert!(port.words.is_empty());
    }

    /// The signature and the element width share byte 0, so the signature is
    /// the **top nibble**. Byte 0 of `0x30` is an RLE block, not Huffman.
    #[test]
    fn the_signature_is_the_top_nibble() {
        assert_eq!(parse_huffman_header(0x24).signature, HUFFMAN_TYPE);
        assert_eq!(parse_huffman_header(0x24).element_bits, 4);
        assert_eq!(parse_huffman_header(0x28).element_bits, 8);
        assert_ne!(
            parse_huffman_header(0x30).signature,
            HUFFMAN_TYPE,
            "0x30 is the RLE signature"
        );
        assert_eq!(parse_huffman_header(0x24).output_len, 0);
    }

    /// A node walk that leaves the table is a corrupt tree, and is reported
    /// rather than read out of bounds. The root's child offsets are set to
    /// point far past a three-byte table.
    #[test]
    fn a_child_offset_past_the_table_is_refused() {
        // header word, length byte, three-byte table, one bitstream word
        let mut bytes = vec![HUFFMAN_TYPE | 8, 4, 0, 0];
        bytes.push(length_byte(3));
        bytes.extend_from_slice(&[0b1100_0000 | 0x3F, 0xAA, 0xBB]);
        bytes.extend_from_slice(&code_bits(&[false; 4]));

        let mut port = SliceHuff::new(bytes);
        for _ in 0..4 {
            port.next_byte();
        }
        assert_eq!(
            huffman_decompress(&mut port, 8, 4),
            Err(DecompressError::BadTree)
        );
        assert!(port.words.is_empty());
    }
}
