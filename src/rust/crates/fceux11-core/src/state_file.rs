//! FCEUX savestate file format management.
//!
//! Replaces the file-level serialization (header, chunk orchestration,
//! compression) from `src/state.cpp`.  C++ retains SFORMAT field-level
//! serialization; Rust owns the file envelope.
//!
//! # File format
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
//! Old format (pre-FCSX) is supported for loading only:
//!   [0..3] "FCS" magic
//!   [3]    version byte (or 0xFF + u32 at offset 8)
//!   [4..8] totalsize (u32 LE)
//!   payload is uncompressed.

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

/// Maximum uncompressed size we are willing to accept (safety limit).
const MAX_UNCOMPRESSED_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

/// Size of the savestate file header.
const HEADER_SIZE: usize = 16;

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
}

/// Build a savestate file from chunks.
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
    // 1. Serialize uncompressed payload
    let mut payload = Vec::new();
    for chunk in chunks {
        payload.push(chunk.chunk_type);
        payload.extend_from_slice(&(chunk.data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&chunk.data);
    }

    let totalsize = payload.len() as u32;

    // 2. Compress if requested
    let (body, comprlen) = if compression_level == 0 {
        (payload, u32::MAX)
    } else {
        let level = match compression_level {
            -1 => Compression::default(),
            n if (1..=9).contains(&n) => Compression::new(n as u32),
            _ => Compression::default(),
        };
        let mut encoder = ZlibEncoder::new(Vec::new(), level);
        std::io::Write::write_all(&mut encoder, &payload)
            .map_err(|_| StateError::CompressionFailed)?;
        let compressed = encoder
            .finish()
            .map_err(|_| StateError::CompressionFailed)?;
        let comprlen_u32 = compressed.len() as u32;
        (compressed, comprlen_u32)
    };

    // 3. Assemble header + body
    let mut file = Vec::with_capacity(HEADER_SIZE + body.len());
    file.extend_from_slice(b"FCSX");
    file.extend_from_slice(&totalsize.to_le_bytes());
    file.extend_from_slice(&version.to_le_bytes());
    file.extend_from_slice(&comprlen.to_le_bytes());
    file.extend_from_slice(&body);

    Ok(file)
}

/// Parse a savestate file, returning `(version, chunks, totalsize)`.
///
/// Supports both FCSX and legacy "FCS" headers.
pub fn load_state_file(data: &[u8]) -> Result<(u32, Vec<StateChunk>, u32), StateError> {
    if data.len() < HEADER_SIZE {
        return Err(StateError::InvalidHeader);
    }

    let magic = &data[0..4];
    let totalsize = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

    // Detect format and extract uncompressed payload
    let version: u32;
    let payload: Vec<u8>;

    if magic == b"FCSX" {
        version = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let comprlen = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);

        if (totalsize as usize) > MAX_UNCOMPRESSED_SIZE {
            return Err(StateError::PayloadTooLarge);
        }

        if comprlen == u32::MAX {
            // Uncompressed
            if data.len() < HEADER_SIZE + totalsize as usize {
                return Err(StateError::InvalidHeader);
            }
            payload = data[HEADER_SIZE..HEADER_SIZE + totalsize as usize].to_vec();
        } else {
            // Compressed
            if data.len() < HEADER_SIZE + comprlen as usize {
                return Err(StateError::InvalidHeader);
            }
            let compressed = &data[HEADER_SIZE..HEADER_SIZE + comprlen as usize];
            let mut decoder = ZlibDecoder::new(compressed);
            let mut uncompressed = Vec::with_capacity(totalsize as usize);
            std::io::Read::read_to_end(&mut decoder, &mut uncompressed)
                .map_err(|_| StateError::DecompressionFailed)?;
            if uncompressed.len() != totalsize as usize {
                return Err(StateError::DecompressionFailed);
            }
            payload = uncompressed;
        }
    } else if &magic[0..3] == b"FCS" {
        // Legacy format
        version = if data[3] == 0xFF {
            u32::from_le_bytes([data[8], data[9], data[10], data[11]])
        } else {
            (data[3] as u32) * 100
        };
        if data.len() < HEADER_SIZE + totalsize as usize {
            return Err(StateError::InvalidHeader);
        }
        payload = data[HEADER_SIZE..HEADER_SIZE + totalsize as usize].to_vec();
    } else {
        return Err(StateError::InvalidHeader);
    };

    // Parse chunks from payload
    let mut chunks = Vec::new();
    let mut pos = 0usize;
    let payload_len = payload.len();

    while pos < payload_len {
        if pos + 5 > payload_len {
            return Err(StateError::InvalidChunk);
        }
        let chunk_type = payload[pos];
        let chunk_size = u32::from_le_bytes([
            payload[pos + 1],
            payload[pos + 2],
            payload[pos + 3],
            payload[pos + 4],
        ]) as usize;
        pos += 5;

        if pos + chunk_size > payload_len {
            return Err(StateError::InvalidChunk);
        }
        chunks.push(StateChunk {
            chunk_type,
            data: payload[pos..pos + chunk_size].to_vec(),
        });
        pos += chunk_size;
    }

    Ok((version, chunks, totalsize))
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
// FFI: Save
// ============================================================

/// Serialize chunks into a savestate file buffer.
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
    let mut state_chunks = Vec::with_capacity(chunk_count);
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
// FFI: Load
// ============================================================

/// Deserialize a savestate file buffer into chunks.
///
/// On success, returns `true` and writes:
/// * `out_chunks` — pointer to an array of `FceuStateChunkOutput`
/// * `out_chunk_count` — number of chunks
/// * `out_version` — savestate version from header
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

    #[test]
    fn test_roundtrip_uncompressed() {
        unsafe {
            let chunks = make_test_chunks();
            let file = save_state_file(&chunks, 0x020400, 0).unwrap();

            assert!(file.len() >= 16);
            assert_eq!(&file[0..4], b"FCSX");

            let (version, loaded_chunks, totalsize) = load_state_file(&file).unwrap();
            assert_eq!(version, 0x020400);
            assert_eq!(loaded_chunks, chunks);
            assert_eq!(totalsize as usize, file.len() - 16);
        }
    }

    #[test]
    fn test_roundtrip_compressed() {
        unsafe {
            let chunks = make_test_chunks();
            let file_compressed = save_state_file(&chunks, 0x020400, 6).unwrap();
            let file_uncompressed = save_state_file(&chunks, 0x020400, 0).unwrap();

            // Compressed should be smaller (or equal) for this tiny payload
            // Actually tiny data may compress larger; just verify it parses.
            let (version, loaded_chunks, _) = load_state_file(&file_compressed).unwrap();
            assert_eq!(version, 0x020400);
            assert_eq!(loaded_chunks, chunks);

            let (_, loaded2, _) = load_state_file(&file_uncompressed).unwrap();
            assert_eq!(loaded2, chunks);
        }
    }

    #[test]
    fn test_old_format_load() {
        unsafe {
            // Legacy header: "FCS" + version byte + totalsize + padding
            let mut header = vec![b'F', b'C', b'S', 0x63]; // version = 99 * 100 = 9900
            // payload = chunk1(1+4+4) + chunk2(1+4+3) = 17 bytes
            header.extend_from_slice(&17u32.to_le_bytes());
            header.extend_from_slice(&[0u8; 8]); // padding

            // chunk type=1, size=4, data=[AA BB CC DD]
            header.push(1);
            header.extend_from_slice(&4u32.to_le_bytes());
            header.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);
            // chunk type=2, size=3, data=[11 22 33]
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
    }

    #[test]
    fn test_old_format_0xff_version() {
        unsafe {
            let mut header = vec![b'F', b'C', b'S', 0xFF];
            header.extend_from_slice(&5u32.to_le_bytes()); // totalsize
            header.extend_from_slice(&12345u32.to_le_bytes()); // version
            header.extend_from_slice(&[0u8; 4]);

            // 5 bytes payload
            header.push(1);
            header.extend_from_slice(&0u32.to_le_bytes());

            let (version, chunks, _) = load_state_file(&header).unwrap();
            assert_eq!(version, 12345);
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].chunk_type, 1);
            assert_eq!(chunks[0].data.len(), 0);
        }
    }

    #[test]
    fn test_invalid_header() {
        unsafe {
            assert!(load_state_file(b"BAD").is_err());
            assert!(load_state_file(b"FCSX\x00\x00\x00\x00").is_err()); // too short
        }
    }

    #[test]
    fn test_empty_chunks() {
        unsafe {
            let chunks: Vec<StateChunk> = vec![];
            let file = save_state_file(&chunks, 1, 0).unwrap();
            let (version, loaded, totalsize) = load_state_file(&file).unwrap();
            assert_eq!(version, 1);
            assert_eq!(totalsize, 0);
            assert!(loaded.is_empty());
        }
    }

    #[test]
    fn test_ffi_save_load_roundtrip() {
        unsafe {
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

            assert!(fceux11_rust_state_file_save(
                inputs.as_ptr(),
                inputs.len(),
                42,
                0,
                &mut out_buf,
            ));

            assert!(!out_buf.ptr.is_null());
            assert!(out_buf.len >= 16);

            let mut out_chunks: *mut FceuStateChunkOutput = std::ptr::null_mut();
            let mut out_chunk_count: usize = 0;
            let mut out_version: u32 = 0;
            let mut out_totalsize: u32 = 0;

            assert!(fceux11_rust_state_file_load(
                out_buf.ptr,
                out_buf.len,
                &mut out_chunks,
                &mut out_chunk_count,
                &mut out_version,
                &mut out_totalsize,
            ));

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

            fceux11_rust_state_file_chunks_free(out_chunks, out_chunk_count);
            fceux11_rust_state_file_buf_free(out_buf);
        }
    }
}
