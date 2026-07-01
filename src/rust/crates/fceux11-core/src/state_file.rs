//! FCEUX savestate file format management.
//!
//! Replaces the file-level serialization (header, chunk orchestration,
//! compression) from `src/state.cpp`.  C++ retains SFORMAT field-level
//! serialization; Rust owns the file envelope.
//!
//! # Supported formats
//!
//! ## V2 format (v1.9+, default for saving)
//! ```text
//! Header (24 bytes):
//!   [0..8]   "FCEU11ST" magic
//!   [8..12]  format_version (u32 LE) = 2
//!   [12..16] chunk_count   (u32 LE)
//!   [16..20] total_uncompressed_size (u32 LE)
//!   [20..24] flags (u32 LE) — bit 0: compressed, bit 1: has_crc32
//!
//! Payload (compressed or raw):
//!   For each chunk:
//!     [type: u8][size: u32 LE][crc32: u32 LE][data: size bytes]
//! ```
//!
//! ## V1 format (FCSX, v1.0~v1.8, supported for load and legacy save)
//! ```text
//! Header (16 bytes):
//!   [0..4]   "FCSX" magic
//!   [4..8]   totalsize   (u32 LE) — uncompressed payload size
//!   [8..12]  version     (u32 LE) — FCEU_VERSION_NUMERIC
//!   [12..16] comprlen    (u32 LE) — compressed size, or ~0 for uncompressed
//!
//! Payload (totalsize bytes):
//!   [type: u8][size: u32 LE][data: size bytes]  (repeated)
//! ```
//!
//! ## Legacy format (pre-FCSX, load only)
//!   [0..3] "FCS" magic
//!   [3]    version byte (or 0xFF + u32 at offset 8)
//!   [4..8] totalsize (u32 LE)
//!   payload is uncompressed.

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

/// Maximum uncompressed size we are willing to accept (safety limit).
const MAX_UNCOMPRESSED_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

/// V1 (FCSX) header size.
const V1_HEADER_SIZE: usize = 16;

/// V2 (FCEU11ST) header size.
const V2_HEADER_SIZE: usize = 24;

/// V2 chunk header: type(1) + size(4) + crc32(4) = 9 bytes.
const V2_CHUNK_HEADER_SIZE: usize = 9;

/// V1 chunk header: type(1) + size(4) = 5 bytes.
const V1_CHUNK_HEADER_SIZE: usize = 5;

/// V2 flag: payload is zlib-compressed.
const V2_FLAG_COMPRESSED: u32 = 1 << 0;

/// V2 flag: each chunk carries a CRC32 checksum.
const V2_FLAG_HAS_CRC32: u32 = 1 << 1;

/// Savestate format version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum StateFormatVersion {
    /// V1 (FCSX) — v1.0 through v1.8.
    V1 = 1,
    /// V2 (FCEU11ST) — v1.9+, with per-chunk CRC32.
    V2 = 2,
}

// ============================================================
// Internal Rust API
// ============================================================

/// A single chunk extracted from / to be written into a savestate.
#[derive(Debug, Clone, PartialEq)]
pub struct StateChunk {
    pub chunk_type: u8,
    pub data: Vec<u8>,
}

/// Error type for savestate I/O.
#[derive(Debug, Clone, PartialEq)]
pub enum StateError {
    InvalidHeader,
    PayloadTooLarge,
    DecompressionFailed,
    CompressionFailed,
    InvalidChunk,
    CrcMismatch,
}

/// Build a V1 (FCSX) savestate file from chunks.
///
/// `compression_level`:
/// * `0`   → no compression
/// * `-1`  → default compression
/// * `1..=9` → zlib level
pub fn save_state_file(
    chunks: &[StateChunk],
    version: u32,
    compression_level: i32,
) -> Result<Vec<u8>, StateError> {
    // 1. Serialize uncompressed payload (V1 chunk format: type + size + data)
    let mut payload = Vec::new();
    for chunk in chunks {
        payload.push(chunk.chunk_type);
        payload.extend_from_slice(&(chunk.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&chunk.data);
    }

    let totalsize = payload.len() as u32;

    // 2. Compress if requested
    let (body, comprlen) = compress_payload(&payload, compression_level)?;

    // 3. Assemble V1 header + body
    let mut file = Vec::with_capacity(V1_HEADER_SIZE + body.len());
    file.extend_from_slice(b"FCSX");
    file.extend_from_slice(&totalsize.to_le_bytes());
    file.extend_from_slice(&version.to_le_bytes());
    file.extend_from_slice(&comprlen.to_le_bytes());
    file.extend_from_slice(&body);

    Ok(file)
}

/// Build a V2 (FCEU11ST) savestate file from chunks.
///
/// Each chunk carries a CRC32 checksum for integrity verification.
pub fn save_state_file_v2(
    chunks: &[StateChunk],
    compression_level: i32,
) -> Result<Vec<u8>, StateError> {
    // 1. Serialize uncompressed payload (V2 chunk format: type + size + crc32 + data)
    let mut payload = Vec::new();
    for chunk in chunks {
        let crc = crc32fast::hash(&chunk.data);
        payload.push(chunk.chunk_type);
        payload.extend_from_slice(&(chunk.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&crc.to_le_bytes());
        payload.extend_from_slice(&chunk.data);
    }

    let totalsize = payload.len() as u32;
    let chunk_count = chunks.len() as u32;

    // 2. Compress if requested
    let (body, comprlen) = compress_payload(&payload, compression_level)?;

    // 3. Build flags
    let mut flags: u32 = V2_FLAG_HAS_CRC32;
    if comprlen != u32::MAX {
        flags |= V2_FLAG_COMPRESSED;
    }

    // 4. Assemble V2 header + body
    let mut file = Vec::with_capacity(V2_HEADER_SIZE + body.len());
    file.extend_from_slice(b"FCEU11ST");
    file.extend_from_slice(&2u32.to_le_bytes()); // format_version
    file.extend_from_slice(&chunk_count.to_le_bytes());
    file.extend_from_slice(&totalsize.to_le_bytes());
    file.extend_from_slice(&flags.to_le_bytes());
    file.extend_from_slice(&body);

    Ok(file)
}

/// Parse a savestate file, returning `(version, chunks, totalsize)`.
///
/// Auto-detects V2 (FCEU11ST), V1 (FCSX), and legacy (FCS) formats.
/// For V2, `version` returned is the format version (2).
pub fn load_state_file(data: &[u8]) -> Result<(u32, Vec<StateChunk>, u32), StateError> {
    if data.len() < V1_HEADER_SIZE {
        return Err(StateError::InvalidHeader);
    }

    // Detect format by magic
    if data.len() >= V2_HEADER_SIZE && &data[0..8] == b"FCEU11ST" {
        load_v2(data)
    } else if &data[0..4] == b"FCSX" {
        load_v1(data)
    } else if &data[0..3] == b"FCS" {
        load_legacy(data)
    } else {
        Err(StateError::InvalidHeader)
    }
}

// ============================================================
// Format-specific loaders
// ============================================================

fn load_v1(data: &[u8]) -> Result<(u32, Vec<StateChunk>, u32), StateError> {
    let totalsize = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let version = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let comprlen = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

    if (totalsize as usize) > MAX_UNCOMPRESSED_SIZE {
        return Err(StateError::PayloadTooLarge);
    }

    let payload = if comprlen == u32::MAX {
        // Uncompressed
        if data.len() < V1_HEADER_SIZE + totalsize as usize {
            return Err(StateError::InvalidHeader);
        }
        data[V1_HEADER_SIZE..V1_HEADER_SIZE + totalsize as usize].to_vec()
    } else {
        // Compressed
        if data.len() < V1_HEADER_SIZE + comprlen as usize {
            return Err(StateError::InvalidHeader);
        }
        let compressed = &data[V1_HEADER_SIZE..V1_HEADER_SIZE + comprlen as usize];
        decompress_payload(compressed, totalsize as usize)?
    };

    let chunks = parse_v1_chunks(&payload)?;
    Ok((version, chunks, totalsize))
}

fn load_v2(data: &[u8]) -> Result<(u32, Vec<StateChunk>, u32), StateError> {
    let format_version = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
    let chunk_count = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;
    let totalsize = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
    let flags = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);

    if (totalsize as usize) > MAX_UNCOMPRESSED_SIZE {
        return Err(StateError::PayloadTooLarge);
    }

    let is_compressed = (flags & V2_FLAG_COMPRESSED) != 0;
    let has_crc32 = (flags & V2_FLAG_HAS_CRC32) != 0;

    let payload = if is_compressed {
        // Compressed: read remaining data and decompress
        if data.len() < V2_HEADER_SIZE {
            return Err(StateError::InvalidHeader);
        }
        let compressed = &data[V2_HEADER_SIZE..];
        decompress_payload(compressed, totalsize as usize)?
    } else {
        // Uncompressed
        if data.len() < V2_HEADER_SIZE + totalsize as usize {
            return Err(StateError::InvalidHeader);
        }
        data[V2_HEADER_SIZE..V2_HEADER_SIZE + totalsize as usize].to_vec()
    };

    let chunks = parse_v2_chunks(&payload, chunk_count, has_crc32)?;
    Ok((format_version, chunks, totalsize))
}

fn load_legacy(data: &[u8]) -> Result<(u32, Vec<StateChunk>, u32), StateError> {
    let totalsize = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let version = if data[3] == 0xFF {
        u32::from_le_bytes([data[8], data[9], data[10], data[11]])
    } else {
        (data[3] as u32) * 100
    };
    if data.len() < V1_HEADER_SIZE + totalsize as usize {
        return Err(StateError::InvalidHeader);
    }
    let payload = data[V1_HEADER_SIZE..V1_HEADER_SIZE + totalsize as usize].to_vec();
    let chunks = parse_v1_chunks(&payload)?;
    Ok((version, chunks, totalsize))
}

// ============================================================
// Chunk parsers
// ============================================================

fn parse_v1_chunks(payload: &[u8]) -> Result<Vec<StateChunk>, StateError> {
    let mut chunks = Vec::new();
    let mut pos = 0usize;
    let payload_len = payload.len();

    while pos < payload_len {
        if pos + V1_CHUNK_HEADER_SIZE > payload_len {
            return Err(StateError::InvalidChunk);
        }
        let chunk_type = payload[pos];
        let chunk_size = u32::from_le_bytes([
            payload[pos + 1],
            payload[pos + 2],
            payload[pos + 3],
            payload[pos + 4],
        ]) as usize;
        pos += V1_CHUNK_HEADER_SIZE;

        if pos + chunk_size > payload_len {
            return Err(StateError::InvalidChunk);
        }
        chunks.push(StateChunk {
            chunk_type,
            data: payload[pos..pos + chunk_size].to_vec(),
        });
        pos += chunk_size;
    }

    Ok(chunks)
}

fn parse_v2_chunks(
    payload: &[u8],
    expected_count: usize,
    verify_crc: bool,
) -> Result<Vec<StateChunk>, StateError> {
    let mut chunks = Vec::with_capacity(expected_count);
    let mut pos = 0usize;
    let payload_len = payload.len();

    while pos < payload_len {
        if pos + V2_CHUNK_HEADER_SIZE > payload_len {
            return Err(StateError::InvalidChunk);
        }
        let chunk_type = payload[pos];
        let chunk_size = u32::from_le_bytes([
            payload[pos + 1],
            payload[pos + 2],
            payload[pos + 3],
            payload[pos + 4],
        ]) as usize;
        let stored_crc = u32::from_le_bytes([
            payload[pos + 5],
            payload[pos + 6],
            payload[pos + 7],
            payload[pos + 8],
        ]);
        pos += V2_CHUNK_HEADER_SIZE;

        if pos + chunk_size > payload_len {
            return Err(StateError::InvalidChunk);
        }
        let chunk_data = &payload[pos..pos + chunk_size];

        if verify_crc {
            let computed_crc = crc32fast::hash(chunk_data);
            if computed_crc != stored_crc {
                return Err(StateError::CrcMismatch);
            }
        }

        chunks.push(StateChunk {
            chunk_type,
            data: chunk_data.to_vec(),
        });
        pos += chunk_size;
    }

    if chunks.len() != expected_count {
        return Err(StateError::InvalidChunk);
    }

    Ok(chunks)
}

// ============================================================
// Compression helpers
// ============================================================

fn compress_payload(payload: &[u8], compression_level: i32) -> Result<(Vec<u8>, u32), StateError> {
    if compression_level == 0 {
        return Ok((payload.to_vec(), u32::MAX));
    }

    let level = match compression_level {
        -1 => Compression::default(),
        n if (1..=9).contains(&n) => Compression::new(n as u32),
        _ => Compression::default(),
    };
    let mut encoder = ZlibEncoder::new(Vec::new(), level);
    std::io::Write::write_all(&mut encoder, payload)
        .map_err(|_| StateError::CompressionFailed)?;
    let compressed = encoder
        .finish()
        .map_err(|_| StateError::CompressionFailed)?;
    let comprlen = compressed.len() as u32;
    Ok((compressed, comprlen))
}

fn decompress_payload(compressed: &[u8], expected_size: usize) -> Result<Vec<u8>, StateError> {
    let mut decoder = ZlibDecoder::new(compressed);
    let mut uncompressed = Vec::with_capacity(expected_size);
    std::io::Read::read_to_end(&mut decoder, &mut uncompressed)
        .map_err(|_| StateError::DecompressionFailed)?;
    if uncompressed.len() != expected_size {
        return Err(StateError::DecompressionFailed);
    }
    Ok(uncompressed)
}

// ============================================================
// FFI types
// ============================================================

/// Input chunk descriptor for saving.
#[repr(C)]
pub struct FceuStateChunkInput {
    pub chunk_type: u8,
    pub data: *const u8,
    pub len: usize,
}

/// Output chunk descriptor for loading.
#[repr(C)]
pub struct FceuStateChunkOutput {
    pub chunk_type: u8,
    pub data: *mut u8,
    pub len: usize,
}

/// Buffer descriptor returned by save / used by free.
#[repr(C)]
pub struct FceuStateBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

// ============================================================
// FFI: Save (V1)
// ============================================================

/// Serialize chunks into a V1 (FCSX) savestate file buffer.
///
/// On success, returns `true` and writes a `FceuStateBuffer` to `out_buf`.
/// The caller must free `out_buf.ptr` via `fceux11_rust_state_file_buf_free`.
#[unsafe(no_mangle)]
/// # Safety
/// Callers must ensure all raw pointer arguments passed to `fceux11_rust_state_file_save` are valid.
pub unsafe extern "C" fn fceux11_rust_state_file_save(
    chunks: *const FceuStateChunkInput,
    chunk_count: usize,
    version: u32,
    compression_level: i32,
    out_buf: *mut FceuStateBuffer,
) -> bool {
    if chunks.is_null() || out_buf.is_null() {
        return false;
    }

    let inputs = unsafe { std::slice::from_raw_parts(chunks, chunk_count) };
    let state_chunks = ffi_inputs_to_chunks(inputs);

    match save_state_file(&state_chunks, version, compression_level) {
        Ok(mut vec) => {
            vec.shrink_to_fit();
            let buf = FceuStateBuffer {
                ptr: vec.as_mut_ptr(),
                len: vec.len(),
                cap: vec.capacity(),
            };
            std::mem::forget(vec);
            unsafe {
                *out_buf = buf;
            }
            true
        }
        Err(_) => false,
    }
}

// ============================================================
// FFI: Save V2
// ============================================================

/// Serialize chunks into a V2 (FCEU11ST) savestate file buffer.
///
/// On success, returns `true` and writes a `FceuStateBuffer` to `out_buf`.
/// The caller must free `out_buf.ptr` via `fceux11_rust_state_file_buf_free`.
#[unsafe(no_mangle)]
/// # Safety
/// Callers must ensure all raw pointer arguments passed to this function are valid.
pub unsafe extern "C" fn fceux11_rust_state_file_save_v2(
    chunks: *const FceuStateChunkInput,
    chunk_count: usize,
    compression_level: i32,
    out_buf: *mut FceuStateBuffer,
) -> bool {
    if chunks.is_null() || out_buf.is_null() {
        return false;
    }

    let inputs = unsafe { std::slice::from_raw_parts(chunks, chunk_count) };
    let state_chunks = ffi_inputs_to_chunks(inputs);

    match save_state_file_v2(&state_chunks, compression_level) {
        Ok(mut vec) => {
            vec.shrink_to_fit();
            let buf = FceuStateBuffer {
                ptr: vec.as_mut_ptr(),
                len: vec.len(),
                cap: vec.capacity(),
            };
            std::mem::forget(vec);
            unsafe {
                *out_buf = buf;
            }
            true
        }
        Err(_) => false,
    }
}

// ============================================================
// FFI: Load
// ============================================================

/// Deserialize a savestate file buffer into chunks.
///
/// Auto-detects V1 (FCSX) and V2 (FCEU11ST) formats.
///
/// On success, returns `true` and writes:
/// * `out_chunks` — pointer to an array of `FceuStateChunkOutput`
/// * `out_chunk_count` — number of chunks
/// * `out_version` — savestate version from header (format version for V2)
/// * `out_totalsize` — uncompressed payload size
///
/// The caller must free `out_chunks` via `fceux11_rust_state_file_chunks_free`.
#[unsafe(no_mangle)]
/// # Safety
/// Callers must ensure all raw pointer arguments passed to `fceux11_rust_state_file_load` are valid.
pub unsafe extern "C" fn fceux11_rust_state_file_load(
    file_data: *const u8,
    file_len: usize,
    out_chunks: *mut *mut FceuStateChunkOutput,
    out_chunk_count: *mut usize,
    out_version: *mut u32,
    out_totalsize: *mut u32,
) -> bool {
    if file_data.is_null() || out_chunks.is_null() || out_chunk_count.is_null() {
        return false;
    }

    let data = unsafe { std::slice::from_raw_parts(file_data, file_len) };
    match load_state_file(data) {
        Ok((version, chunks, totalsize)) => {
            let count = chunks.len();
            let mut c_chunks: Vec<FceuStateChunkOutput> = Vec::with_capacity(count);
            for chunk in chunks {
                let mut data = chunk.data;
                data.shrink_to_fit();
                c_chunks.push(FceuStateChunkOutput {
                    chunk_type: chunk.chunk_type,
                    data: data.as_mut_ptr(),
                    len: data.len(),
                });
                std::mem::forget(data);
            }

            c_chunks.shrink_to_fit();
            let ptr = c_chunks.as_mut_ptr();
            let _cap = c_chunks.capacity();
            std::mem::forget(c_chunks);

            unsafe {
                *out_chunks = ptr;
                *out_chunk_count = count;
                if !out_version.is_null() {
                    *out_version = version;
                }
                if !out_totalsize.is_null() {
                    *out_totalsize = totalsize;
                }
            }
            true
        }
        Err(_) => false,
    }
}

// ============================================================
// FFI: Free helpers
// ============================================================

/// Free a buffer previously returned by `fceux11_rust_state_file_save`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_state_file_buf_free(buf: FceuStateBuffer) {
    if !buf.ptr.is_null() {
        unsafe {
            drop(Vec::from_raw_parts(buf.ptr, buf.len, buf.cap));
        }
    }
}

/// Free chunks previously returned by `fceux11_rust_state_file_load`.
#[unsafe(no_mangle)]
/// # Safety
/// Callers must ensure all raw pointer arguments passed to `fceux11_rust_state_file_chunks_free` are valid.
pub unsafe extern "C" fn fceux11_rust_state_file_chunks_free(
    chunks: *mut FceuStateChunkOutput,
    chunk_count: usize,
) {
    if chunks.is_null() || chunk_count == 0 {
        return;
    }
    unsafe {
        let slice = std::slice::from_raw_parts_mut(chunks, chunk_count);
        for chunk in slice {
            if !chunk.data.is_null() && chunk.len > 0 {
                drop(Vec::from_raw_parts(chunk.data, chunk.len, chunk.len));
            }
        }
        drop(Vec::from_raw_parts(chunks, chunk_count, chunk_count));
    }
}

// ============================================================
// Internal helpers
// ============================================================

fn ffi_inputs_to_chunks(inputs: &[FceuStateChunkInput]) -> Vec<StateChunk> {
    let mut state_chunks = Vec::with_capacity(inputs.len());
    for inp in inputs {
        let data = if inp.data.is_null() || inp.len == 0 {
            Vec::new()
        } else {
            unsafe { std::slice::from_raw_parts(inp.data, inp.len) }.to_vec()
        };
        state_chunks.push(StateChunk {
            chunk_type: inp.chunk_type,
            data,
        });
    }
    state_chunks
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_chunks() -> Vec<StateChunk> {
        vec![
            StateChunk {
                chunk_type: 1,
                data: vec![0xAA, 0xBB, 0xCC, 0xDD],
            },
            StateChunk {
                chunk_type: 2,
                data: vec![0x11, 0x22, 0x33],
            },
            StateChunk {
                chunk_type: 0x10,
                data: vec![0x01, 0x02],
            },
        ]
    }

    // ---- V1 tests (unchanged from original) ----

    #[test]
    fn test_v1_roundtrip_uncompressed() {
        let chunks = make_test_chunks();
        let file = save_state_file(&chunks, 0x020400, 0).unwrap();

        assert!(file.len() >= V1_HEADER_SIZE);
        assert_eq!(&file[0..4], b"FCSX");

        let (version, loaded_chunks, totalsize) = load_state_file(&file).unwrap();
        assert_eq!(version, 0x020400);
        assert_eq!(loaded_chunks, chunks);
        assert_eq!(totalsize as usize, file.len() - V1_HEADER_SIZE);
    }

    #[test]
    fn test_v1_roundtrip_compressed() {
        let chunks = make_test_chunks();
        let file_compressed = save_state_file(&chunks, 0x020400, 6).unwrap();
        let file_uncompressed = save_state_file(&chunks, 0x020400, 0).unwrap();

        let (version, loaded_chunks, _) = load_state_file(&file_compressed).unwrap();
        assert_eq!(version, 0x020400);
        assert_eq!(loaded_chunks, chunks);

        let (_, loaded2, _) = load_state_file(&file_uncompressed).unwrap();
        assert_eq!(loaded2, chunks);
    }

    #[test]
    fn test_old_format_load() {
        let mut header = vec![b'F', b'C', b'S', 0x63]; // version = 99 * 100 = 9900
        header.extend_from_slice(&17u32.to_le_bytes());
        header.extend_from_slice(&[0u8; 8]);

        header.push(1);
        header.extend_from_slice(&4u32.to_le_bytes());
        header.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
        header.push(2);
        header.extend_from_slice(&3u32.to_le_bytes());
        header.extend_from_slice(&[0x11, 0x22, 0x33]);

        let (version, chunks, totalsize) = load_state_file(&header).unwrap();
        assert_eq!(version, 9900);
        assert_eq!(totalsize, 17);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_type, 1);
        assert_eq!(chunks[0].data, vec![0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(chunks[1].chunk_type, 2);
        assert_eq!(chunks[1].data, vec![0x11, 0x22, 0x33]);
    }

    #[test]
    fn test_old_format_0xff_version() {
        let mut header = vec![b'F', b'C', b'S', 0xFF];
        header.extend_from_slice(&5u32.to_le_bytes());
        header.extend_from_slice(&12345u32.to_le_bytes());
        header.extend_from_slice(&[0u8; 4]);

        header.push(1);
        header.extend_from_slice(&0u32.to_le_bytes());

        let (version, chunks, _) = load_state_file(&header).unwrap();
        assert_eq!(version, 12345);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_type, 1);
        assert_eq!(chunks[0].data.len(), 0);
    }

    #[test]
    fn test_invalid_header() {
        assert!(load_state_file(b"BAD").is_err());
        assert!(load_state_file(b"FCSX\x00\x00\x00\x00").is_err());
    }

    #[test]
    fn test_v1_empty_chunks() {
        let chunks: Vec<StateChunk> = vec![];
        let file = save_state_file(&chunks, 1, 0).unwrap();
        let (version, loaded, totalsize) = load_state_file(&file).unwrap();
        assert_eq!(version, 1);
        assert_eq!(totalsize, 0);
        assert!(loaded.is_empty());
    }

    // ---- V2 tests ----

    #[test]
    fn test_v2_roundtrip_uncompressed() {
        let chunks = make_test_chunks();
        let file = save_state_file_v2(&chunks, 0).unwrap();

        assert!(file.len() >= V2_HEADER_SIZE);
        assert_eq!(&file[0..8], b"FCEU11ST");

        let (version, loaded_chunks, totalsize) = load_state_file(&file).unwrap();
        assert_eq!(version, 2);
        assert_eq!(loaded_chunks, chunks);
        assert!(totalsize > 0);
    }

    #[test]
    fn test_v2_roundtrip_compressed() {
        let chunks = make_test_chunks();
        let file = save_state_file_v2(&chunks, 6).unwrap();

        assert_eq!(&file[0..8], b"FCEU11ST");

        let (version, loaded_chunks, _) = load_state_file(&file).unwrap();
        assert_eq!(version, 2);
        assert_eq!(loaded_chunks, chunks);
    }

    #[test]
    fn test_v2_empty_chunks() {
        let chunks: Vec<StateChunk> = vec![];
        let file = save_state_file_v2(&chunks, 0).unwrap();
        let (version, loaded, _) = load_state_file(&file).unwrap();
        assert_eq!(version, 2);
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_v2_crc32_verification() {
        let chunks = make_test_chunks();
        let mut file = save_state_file_v2(&chunks, 0).unwrap();

        // Corrupt one byte in the payload (after V2 header)
        let corrupt_pos = V2_HEADER_SIZE + V2_CHUNK_HEADER_SIZE; // first data byte of first chunk
        file[corrupt_pos] ^= 0xFF;

        // Loading should fail with CRC mismatch
        let result = load_state_file(&file);
        assert_eq!(result.err(), Some(StateError::CrcMismatch));
    }

    #[test]
    fn test_v2_large_data() {
        // Test with larger data to exercise compression
        let chunks: Vec<StateChunk> = (0..10)
            .map(|i| StateChunk {
                chunk_type: i,
                data: vec![i as u8; 1024],
            })
            .collect();

        let file_uncompressed = save_state_file_v2(&chunks, 0).unwrap();
        let file_compressed = save_state_file_v2(&chunks, 6).unwrap();

        // Compressed should be smaller for repetitive data
        assert!(file_compressed.len() < file_uncompressed.len());

        // Both should roundtrip correctly
        let (_, loaded1, _) = load_state_file(&file_uncompressed).unwrap();
        assert_eq!(loaded1, chunks);

        let (_, loaded2, _) = load_state_file(&file_compressed).unwrap();
        assert_eq!(loaded2, chunks);
    }

    // ---- Cross-format tests ----

    #[test]
    fn test_v1_loads_v1_data() {
        let chunks = make_test_chunks();
        let file = save_state_file(&chunks, 100, 0).unwrap();
        let (version, loaded, _) = load_state_file(&file).unwrap();
        assert_eq!(version, 100);
        assert_eq!(loaded, chunks);
    }

    #[test]
    fn test_v2_loads_v2_data() {
        let chunks = make_test_chunks();
        let file = save_state_file_v2(&chunks, 0).unwrap();
        let (version, loaded, _) = load_state_file(&file).unwrap();
        assert_eq!(version, 2);
        assert_eq!(loaded, chunks);
    }

    // ---- FFI tests ----

    #[test]
    fn test_ffi_save_load_roundtrip() {
        let input_data1 = [0xAAu8, 0xBB, 0xCC];
        let input_data2 = [0x11u8, 0x22];
        let inputs = [
            FceuStateChunkInput {
                chunk_type: 1,
                data: input_data1.as_ptr(),
                len: input_data1.len(),
            },
            FceuStateChunkInput {
                chunk_type: 2,
                data: input_data2.as_ptr(),
                len: input_data2.len(),
            },
        ];

        let mut out_buf = FceuStateBuffer {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        };

        assert!(unsafe {
            fceux11_rust_state_file_save(
                inputs.as_ptr(),
                inputs.len(),
                42,
                0,
                &mut out_buf,
            )
        });

        assert!(!out_buf.ptr.is_null());
        assert!(out_buf.len >= V1_HEADER_SIZE);

        let mut out_chunks: *mut FceuStateChunkOutput = std::ptr::null_mut();
        let mut out_chunk_count: usize = 0;
        let mut out_version: u32 = 0;
        let mut out_totalsize: u32 = 0;

        assert!(unsafe {
            fceux11_rust_state_file_load(
                out_buf.ptr,
                out_buf.len,
                &mut out_chunks,
                &mut out_chunk_count,
                &mut out_version,
                &mut out_totalsize,
            )
        });

        assert_eq!(out_version, 42);
        assert_eq!(out_chunk_count, 2);
        assert!(!out_chunks.is_null());

        unsafe {
            let slice = std::slice::from_raw_parts(out_chunks, out_chunk_count);
            assert_eq!(slice[0].chunk_type, 1);
            assert_eq!(slice[0].len, 3);
            assert_eq!(
                std::slice::from_raw_parts(slice[0].data, slice[0].len),
                &[0xAA, 0xBB, 0xCC]
            );
            assert_eq!(slice[1].chunk_type, 2);
            assert_eq!(slice[1].len, 2);
            assert_eq!(
                std::slice::from_raw_parts(slice[1].data, slice[1].len),
                &[0x11, 0x22]
            );
        }

        unsafe {
            fceux11_rust_state_file_chunks_free(out_chunks, out_chunk_count);
            fceux11_rust_state_file_buf_free(out_buf);
        }
    }

    #[test]
    fn test_ffi_save_v2_roundtrip() {
        let input_data = [0xAAu8, 0xBB, 0xCC, 0xDD];
        let inputs = [FceuStateChunkInput {
            chunk_type: 5,
            data: input_data.as_ptr(),
            len: input_data.len(),
        }];

        let mut out_buf = FceuStateBuffer {
            ptr: std::ptr::null_mut(),
            len: 0,
            cap: 0,
        };

        assert!(unsafe {
            fceux11_rust_state_file_save_v2(
                inputs.as_ptr(),
                inputs.len(),
                0,
                &mut out_buf,
            )
        });

        assert!(!out_buf.ptr.is_null());

        let mut out_chunks: *mut FceuStateChunkOutput = std::ptr::null_mut();
        let mut out_chunk_count: usize = 0;
        let mut out_version: u32 = 0;

        assert!(unsafe {
            fceux11_rust_state_file_load(
                out_buf.ptr,
                out_buf.len,
                &mut out_chunks,
                &mut out_chunk_count,
                &mut out_version,
                std::ptr::null_mut(),
            )
        });

        assert_eq!(out_version, 2);
        assert_eq!(out_chunk_count, 1);

        unsafe {
            let slice = std::slice::from_raw_parts(out_chunks, out_chunk_count);
            assert_eq!(slice[0].chunk_type, 5);
            assert_eq!(
                std::slice::from_raw_parts(slice[0].data, slice[0].len),
                &[0xAA, 0xBB, 0xCC, 0xDD]
            );
            fceux11_rust_state_file_chunks_free(out_chunks, out_chunk_count);
            fceux11_rust_state_file_buf_free(out_buf);
        }
    }
}
