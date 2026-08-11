//! Savestate tag-driven serialization for memory banks.
//!
//! Mirrors the C++ `SFORMAT` tag convention used in `src/state.cpp`:
//!   `[4-byte tag][4-byte size][size bytes of data]`
//! See `docs/wip_2.0_plan/savestate_tags.md` for the full tag contract.
//!
//! # What we serialize (Phase 2 scope)
//!
//! - `"RAM\0"`  → 2 KiB WRAM
//! - `"NRAM"`   → 2 KiB VRAM (nametables)
//! - `"SPRAM"`  → 256 B OAM (sprite attribute memory)
//! - `"PALR"`   → 32 B palette
//!
//! The V2-chunked writer emits the same byte sequence as `state.cpp`'s
//! `SFCPU`/`FCEU_NEWPPU_STATEINFO`/`FCEUSND_STATEINFO` chunks, so the
//! same C++ `FCEUSS_SaveMS` reader can decode vNESU11 output without
//! modification.
//!
//! # Phase 6+ scope
//!
//! The CPU/PPU/APU savestate handlers (Phase 1/3/4) will use the same
//! chunk writer; this module provides the format primitives.

use crate::ram::{InternalRam, RamInitOption, RamRng};

/// Tag of a chunk — exactly 4 bytes, padded with NULs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tag(pub [u8; 4]);

impl Tag {
    pub const fn from_str(s: &str) -> Self {
        let bytes = s.as_bytes();
        let mut out = [0u8; 4];
        let mut i = 0;
        while i < 4 && i < bytes.len() {
            out[i] = bytes[i];
            i += 1;
        }
        Self(out)
    }
}

// The four tags we use in Phase 2:
pub const TAG_RAM: Tag = Tag::from_str("RAM");
pub const TAG_NRAM: Tag = Tag::from_str("NRAM");
pub const TAG_SPRAM: Tag = Tag::from_str("SPRAM");
pub const TAG_PALR: Tag = Tag::from_str("PALR");

/// Result of a save operation: total bytes written (tag + size + data).
#[derive(Debug, Default, Clone, Copy)]
pub struct SaveStats {
    pub bytes_written: usize,
    pub chunks: usize,
}

/// Chunked writer matching C++ `FCEUSS_SaveMS` V2 layout.
///
/// A `Writer` accumulates into an internal `Vec<u8>`. The caller can
/// then write that buffer to disk. This avoids the need for a C-ABI
/// callback for the basic Phase 2 use case; Phase 6 re-introduces the
/// callback-based FFI for streaming saves.
pub struct Writer {
    buf: Vec<u8>,
    stats: SaveStats,
}

impl Writer {
    pub fn new() -> Self {
        Self { buf: Vec::new(), stats: SaveStats::default() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { buf: Vec::with_capacity(cap), stats: SaveStats::default() }
    }

    /// Emit one V2 chunk: `tag (4) | size (4 LE u32) | data`.
    pub fn write_chunk(&mut self, tag: Tag, data: &[u8]) {
        self.buf.extend_from_slice(&tag.0);
        self.buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.buf.extend_from_slice(data);
        self.stats.bytes_written += 8 + data.len();
        self.stats.chunks += 1;
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn stats(&self) -> SaveStats {
        self.stats
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunked reader — walks a V2 byte stream and yields `(tag, data)`
/// pairs. Used to validate that a golden savestate round-trips through
/// the vNESU11 serializer byte-for-byte.
pub struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk<'a> {
    pub tag: Tag,
    pub data: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadError {
    Truncated,
    BadSize { pos: usize },
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Read the next chunk. Returns `None` at end-of-stream.
    pub fn next_chunk(&mut self) -> Result<Option<Chunk<'a>>, ReadError> {
        if self.pos >= self.bytes.len() {
            return Ok(None);
        }
        if self.bytes.len() - self.pos < 8 {
            return Err(ReadError::Truncated);
        }
        let tag = [
            self.bytes[self.pos],
            self.bytes[self.pos + 1],
            self.bytes[self.pos + 2],
            self.bytes[self.pos + 3],
        ];
        let size = u32::from_le_bytes([
            self.bytes[self.pos + 4],
            self.bytes[self.pos + 5],
            self.bytes[self.pos + 6],
            self.bytes[self.pos + 7],
        ]) as usize;
        self.pos += 8;
        if self.bytes.len() - self.pos < size {
            return Err(ReadError::BadSize { pos: self.pos });
        }
        let data = &self.bytes[self.pos..self.pos + size];
        self.pos += size;
        Ok(Some(Chunk { tag: Tag(tag), data }))
    }
}

// ---------------------------------------------------------------------------
// High-level save / load: InternalRam <-> byte stream
// ---------------------------------------------------------------------------

impl InternalRam {
    /// Append four chunks (RAM / NRAM / SPRAM / PALR) describing this
    /// RAM's state. The chunks are written in C++ `state.cpp` order.
    pub fn save_state(&self, w: &mut Writer) {
        w.write_chunk(TAG_RAM, &self.wram);
        w.write_chunk(TAG_NRAM, &self.vram);
        w.write_chunk(TAG_SPRAM, &self.oam);
        w.write_chunk(TAG_PALR, &self.palette);
    }

    /// Restore RAM from a reader. Unknown chunks are skipped; the chunks
    /// we expect are read in any order, matching the C++ behavior of
    /// looking up chunks by tag.
    pub fn load_state(&mut self, r: &mut Reader<'_>) -> Result<(), ReadError> {
        while let Some(chunk) = r.next_chunk()? {
            match chunk.tag {
                tag if tag == TAG_RAM => copy_slice(&mut self.wram, chunk.data),
                tag if tag == TAG_NRAM => copy_slice(&mut self.vram, chunk.data),
                tag if tag == TAG_SPRAM => copy_slice(&mut self.oam, chunk.data),
                tag if tag == TAG_PALR => copy_slice(&mut self.palette, chunk.data),
                _ => {
                    // Unknown chunk — skip (matches C++ tolerance).
                }
            }
        }
        Ok(())
    }
}

fn copy_slice(dst: &mut [u8], src: &[u8]) {
    let n = dst.len().min(src.len());
    dst[..n].copy_from_slice(&src[..n]);
}

// ---------------------------------------------------------------------------
// RamRng + RamInitOption save/load
// ---------------------------------------------------------------------------

impl RamRng {
    /// Save the PRNG state as 16 bytes (two u64 little-endian).
    pub fn save_state(&self, w: &mut Writer) {
        w.write_chunk(Tag(*b"PRNG"), &[
            self.s[0].to_le_bytes().as_slice(),
            self.s[1].to_le_bytes().as_slice(),
        ].concat());
    }

    /// Load the PRNG state from a PRNG-tagged chunk. If the stream has
    /// no PRNG chunk, the RNG is left untouched.
    pub fn load_state(&mut self, r: &mut Reader<'_>) -> Result<(), ReadError> {
        while let Some(chunk) = r.next_chunk()? {
            if chunk.tag == Tag(*b"PRNG") && chunk.data.len() == 16 {
                let mut lo = [0u8; 8];
                let mut hi = [0u8; 8];
                lo.copy_from_slice(&chunk.data[0..8]);
                hi.copy_from_slice(&chunk.data[8..16]);
                self.s = [u64::from_le_bytes(lo), u64::from_le_bytes(hi)];
            }
            // Unknown chunks ignored.
        }
        Ok(())
    }
}

impl RamInitOption {
    /// `RADO` tag — 1 byte.
    pub fn save_state(&self, w: &mut Writer) {
        w.write_chunk(Tag(*b"RADO"), &[*self as u8]);
    }

    pub fn load_state(&self, r: &mut Reader<'_>) -> Result<(), ReadError> {
        // Skip RADO chunk for now — Phase 2 doesn't need it for
        // shadow-run equivalence (it's a config knob, not state).
        while r.next_chunk()?.is_some() {}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_from_str_pads_with_zeros() {
        let t = Tag::from_str("PC");
        assert_eq!(t.0, [b'P', b'C', 0, 0]);
        let t = Tag::from_str("JAMM");
        assert_eq!(t.0, [b'J', b'A', b'M', b'M']);
    }

    #[test]
    fn writer_emits_v2_layout() {
        let mut w = Writer::new();
        w.write_chunk(TAG_RAM, &[0xAA, 0xBB, 0xCC]);
        let bytes = w.into_bytes();
        // tag + size (LE u32) + data
        assert_eq!(&bytes[0..4], b"RAM\0");
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 3);
        assert_eq!(&bytes[8..11], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(bytes.len(), 11);
    }

    #[test]
    fn writer_stats_match_bytes() {
        let mut w = Writer::new();
        w.write_chunk(TAG_RAM, &[1, 2, 3, 4]);
        w.write_chunk(TAG_NRAM, &[5, 6, 7]);
        let s = w.stats();
        assert_eq!(s.chunks, 2);
        // 8 header per chunk + 4 + 3 data = 8+8+4+3 = 23.
        assert_eq!(s.bytes_written, 23);
    }

    #[test]
    fn roundtrip_internal_ram_bytes() {
        let mut ram = InternalRam::new_zeroed();
        // Write a fingerprint into each bank.
        for (i, b) in ram.wram.iter_mut().enumerate() {
            *b = (i & 0xFF) as u8;
        }
        for (i, b) in ram.vram.iter_mut().enumerate() {
            *b = ((i * 7) & 0xFF) as u8;
        }
        for (i, b) in ram.oam.iter_mut().enumerate() {
            *b = ((i * 11) & 0xFF) as u8;
        }
        for (i, b) in ram.palette.iter_mut().enumerate() {
            *b = ((i * 13) & 0xFF) as u8;
        }

        // Save → load → compare.
        let mut w = Writer::with_capacity(8192);
        ram.save_state(&mut w);
        let bytes = w.into_bytes();

        let mut restored = InternalRam::new_zeroed();
        let mut r = Reader::new(&bytes);
        restored.load_state(&mut r).expect("round-trip");

        assert_eq!(restored.wram, ram.wram);
        assert_eq!(restored.vram, ram.vram);
        assert_eq!(restored.oam, ram.oam);
        assert_eq!(restored.palette, ram.palette);
    }

    #[test]
    fn reader_skips_unknown_chunks() {
        let mut w = Writer::new();
        w.write_chunk(Tag(*b"UNKN"), &[1, 2, 3]);
        w.write_chunk(TAG_RAM, &[0xDE, 0xAD]);
        w.write_chunk(Tag(*b"UNKN"), &[4, 5, 6, 7]);
        w.write_chunk(TAG_PALR, &[0xFF]);
        let bytes = w.into_bytes();

        let mut ram = InternalRam::new_zeroed();
        let mut r = Reader::new(&bytes);
        ram.load_state(&mut r).expect("unknown chunks skipped");

        // RAM is restored from the RAM tag, palette from PALR.
        assert_eq!(ram.wram[0], 0xDE);
        assert_eq!(ram.wram[1], 0xAD);
        assert_eq!(ram.palette[0], 0xFF);
    }

    #[test]
    fn reader_detects_truncated_chunk() {
        // Only 6 bytes — not enough for header.
        let bytes = [0u8, 1, 2, 3, 4, 5];
        let mut r = Reader::new(&bytes);
        let result = r.next_chunk();
        assert!(matches!(result, Err(ReadError::Truncated)));
    }

    #[test]
    fn reader_detects_bad_size() {
        // Header claims size = 100 but we only have 4 bytes after.
        let bytes = [b'R', b'A', b'M', 0, 100, 0, 0, 0, 1, 2, 3, 4];
        let mut r = Reader::new(&bytes);
        let result = r.next_chunk();
        assert!(matches!(result, Err(ReadError::BadSize { .. })));
    }

    #[test]
    fn rng_save_load_roundtrip() {
        let mut rng = RamRng::new();
        rng.seed(0xCAFE_BABE);
        // Consume a few samples so the state advances.
        let mut reference = [0u8; 16];
        for (i, b) in reference.iter_mut().enumerate() {
            *b = rng.next_u8();
            // Tie to i so we don't accidentally use `rng` after move.
            let _ = i;
        }

        // Re-seed and produce the same 16 bytes.
        let mut rng2 = RamRng::new();
        rng2.seed(0xCAFE_BABE);
        let mut reference2 = [0u8; 16];
        for b in reference2.iter_mut() {
            *b = rng2.next_u8();
        }
        assert_eq!(reference, reference2);

        // Save / load via Writer/Reader.
        let mut w = Writer::new();
        rng.save_state(&mut w);
        let bytes = w.into_bytes();
        let mut rng3 = RamRng::new();
        rng3.seed(0); // pre-load state — should be overwritten
        let mut r = Reader::new(&bytes);
        rng3.load_state(&mut r).expect("rng load");

        // After loading, both RNGs are at the post-seed state (after
        // 16 calls). Continue producing — should match.
        let mut seq_a = [0u8; 16];
        let mut seq_b = [0u8; 16];
        for (a, b) in seq_a.iter_mut().zip(seq_b.iter_mut()) {
            *a = rng.next_u8();
            *b = rng3.next_u8();
        }
        assert_eq!(seq_a, seq_b);
    }
}