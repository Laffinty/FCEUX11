//! SFORMAT chunk serialization/deserialization in Rust.
//!
//! Replaces the C++ `SubWrite` and `ReadStateChunk` functions from
//! `src/state.cpp`.  The binary format is identical:
//!
//! ```text
//! For each SFORMAT entry:
//!   [desc: 4 bytes][size: u32 LE][data: size bytes]
//! ```
//!
//! The C++ side passes SFORMAT table entries via FFI; Rust handles the
//! byte-level serialization and CRC32 checksums.

use crate::state_file::StateChunk;

/// FFI descriptor for a single SFORMAT entry (mirrors C++ `struct SFORMAT`).
///
/// Layout must match C++ exactly:
/// ```c
/// struct SFORMAT { void *v; uint32 s; const char *desc; };
/// ```
/// On x64: v(8) + s(4) + pad(4) + desc(8) = 24 bytes.
///
/// `v` points to the data in C++ memory. `s` encodes size plus flags
/// (RLSB = byte-swap, INDIRECT = pointer indirection). `desc` is a
/// 4-byte ASCII tag (pointer to static string in C++).
#[repr(C)]
pub struct FceuxSformatEntry {
    pub v: *const u8,
    pub s: u32,
    _pad: u32,
    pub desc: *const u8,
}

/// SFORMAT flag: multi-byte integer needs byte-order flip on big-endian.
/// (No-op on little-endian x86_64, but we preserve the flag for format
/// compatibility.)
const FCEUSTATE_RLSB: u32 = 0x80000000;

/// SFORMAT flag: `v` is a `void**` that must be dereferenced before reading.
const FCEUSTATE_INDIRECT: u32 = 0x40000000;

/// Mask to extract the raw size from `s`.
const FCEUSTATE_FLAGS: u32 = FCEUSTATE_RLSB | FCEUSTATE_INDIRECT;

// ============================================================
// Serialization (C++ → Rust → bytes)
// ============================================================

/// Serialize SFORMAT entries into a byte stream (the SFORMAT chunk payload).
///
/// # Safety
/// `entries` must point to a valid array of `count` SFORMAT entries.
/// Each entry's `v` pointer must be valid for reads of `entry.s & !FLAGS` bytes.
/// The array must be terminated by an entry with `v == null`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_sformat_serialize(
    entries: *const FceuxSformatEntry,
    count: usize,
    out_buf: *mut *mut u8,
    out_len: *mut usize,
) -> bool {
    if entries.is_null() || out_buf.is_null() || out_len.is_null() {
        return false;
    }

    let entries = unsafe { std::slice::from_raw_parts(entries, count) };
    let mut output = Vec::new();

    for entry in entries {
        if entry.v.is_null() {
            continue;
        }
        let raw_size = (entry.s & !FCEUSTATE_FLAGS) as usize;
        if raw_size == 0 {
            continue;
        }

        // Write desc (4 bytes from pointer) + size (4 bytes LE) + data
        let desc_bytes = unsafe { std::slice::from_raw_parts(entry.desc, 4) };
        output.extend_from_slice(desc_bytes);
        output.extend_from_slice(&(raw_size as u32).to_le_bytes());

        let data = unsafe { std::slice::from_raw_parts(entry.v, raw_size) };
        output.extend_from_slice(data);
    }

    output.shrink_to_fit();
    let ptr = output.as_mut_ptr();
    let len = output.len();
    std::mem::forget(output);

    unsafe {
        *out_buf = ptr;
        *out_len = len;
    }
    true
}

/// Free a buffer returned by `fceux11_rust_sformat_serialize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_sformat_buf_free(buf: *mut u8, len: usize) {
    if !buf.is_null() && len > 0 {
        unsafe {
            drop(Vec::from_raw_parts(buf, len, len));
        }
    }
}

// ============================================================
// Deserialization (bytes → C++ memory)
// ============================================================

/// Deserialize a SFORMAT byte stream into C++ memory regions.
///
/// For each entry in the stream (desc + size + data), finds the matching
/// entry in the SFORMAT table by `desc` and copies the data.
///
/// # Safety
/// `stream_data` must point to `stream_len` valid bytes of SFORMAT payload.
/// `entries` must point to a valid array of `count` SFORMAT entries.
/// Each entry's `v` pointer must be valid for writes of `entry.s & !FLAGS` bytes.
/// The array must be terminated by an entry with `v == null`.
// hotfix1 P1-5 (C-07): refuse any individual SFORMAT entry larger than
// 1 MiB. The savestate stream header declares the size, and the old
// code trusted it without bound — a 32-bit `size` field of 0xFFFFFFFF
// plus any pointer-deref target would copy ~4 GiB into the destination
// even though legitimate entries never exceed a few KB. We bail out
// before doing any memcpy.
const MAX_SFORMAT_ENTRY_SIZE: usize = 1 << 20;

pub unsafe extern "C" fn fceux11_rust_sformat_deserialize(
    stream_data: *const u8,
    stream_len: usize,
    entries: *mut FceuxSformatEntry,
    count: usize,
    _version: u32,
) -> bool {
    if stream_data.is_null() || entries.is_null() {
        return false;
    }

    let stream = unsafe { std::slice::from_raw_parts(stream_data, stream_len) };
    let entries = unsafe { std::slice::from_raw_parts_mut(entries, count) };

    let mut pos = 0usize;
    while pos + 8 <= stream.len() {
        let desc = [stream[pos], stream[pos + 1], stream[pos + 2], stream[pos + 3]];
        let size = u32::from_le_bytes([
            stream[pos + 4],
            stream[pos + 5],
            stream[pos + 6],
            stream[pos + 7],
        ]) as usize;
        pos += 8;

        // hotfix1 P1-5 (C-07): cap individual entries. A 4-byte length
        // field with no upper bound check was enough for a malicious
        // savestate to scribble past the destination buffer.
        if size > MAX_SFORMAT_ENTRY_SIZE {
            return false;
        }
        if pos + size > stream.len() {
            return false; // truncated
        }

        let chunk_data = &stream[pos..pos + size];
        pos += size;

        // Find matching entry in SFORMAT table
        if let Some(entry) = find_entry_mut(entries, &desc, size) {
            let entry_size = (entry.s & !FCEUSTATE_FLAGS) as usize;
            let copy_size = std::cmp::min(size, entry_size);
            let dest = if (entry.s & FCEUSTATE_INDIRECT) != 0 {
                // Dereference pointer
                let ptr_ptr = entry.v as *const *mut u8;
                unsafe { *ptr_ptr }
            } else {
                entry.v as *mut u8
            };
            if !dest.is_null() {
                unsafe {
                    std::ptr::copy_nonoverlapping(chunk_data.as_ptr(), dest, copy_size);
                }
            }
        }
        // If no matching entry found, skip (unknown field in chunk)
    }

    true
}

fn find_entry_mut<'a>(
    entries: &'a mut [FceuxSformatEntry],
    desc: &[u8; 4],
    _size: usize,
) -> Option<&'a mut FceuxSformatEntry> {
    // Match by description only (size may differ between versions)
    for entry in entries.iter_mut() {
        if entry.v.is_null() || entry.desc.is_null() {
            continue;
        }
        let entry_desc = unsafe { std::slice::from_raw_parts(entry.desc, 4) };
        if entry_desc == desc {
            return Some(entry);
        }
    }
    None
}

// ============================================================
// CRC32 integrity for raw SFORMAT byte streams
// ============================================================

/// Compute CRC32 checksum of a raw SFORMAT byte stream.
///
/// The SFORMAT binary format is:
/// ```text
/// For each entry: [desc: 4 bytes][size: u32 LE][data: size bytes]
/// ```
///
/// This function validates the stream structure and returns a CRC32
/// checksum for integrity verification. Returns 0 for empty streams.
///
/// # Safety
/// `data` must point to `len` valid bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_sformat_crc32(
    data: *const u8,
    len: usize,
) -> u32 {
    if data.is_null() || len == 0 {
        return 0;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    crc32fast::hash(bytes)
}

/// Validate SFORMAT byte stream structure.
///
/// Walks the stream checking that each entry's declared size fits
/// within the remaining bytes. Returns the number of valid entries
/// found, or -1 if the stream is truncated/corrupt.
///
/// # Safety
/// `data` must point to `len` valid bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_sformat_validate(
    data: *const u8,
    len: usize,
) -> i32 {
    if len == 0 {
        return 0; // empty stream is valid
    }
    if data.is_null() {
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
    let mut pos = 0usize;
    let mut count = 0i32;
    while pos + 8 <= bytes.len() {
        let size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;
        if pos + size > bytes.len() {
            return -1; // truncated
        }
        pos += size;
        count += 1;
    }
    if pos != bytes.len() {
        return -1; // trailing bytes
    }
    count
}

// ============================================================
// High-level API: serialize SFORMAT table → StateChunk
// ============================================================

/// Serialize an SFORMAT table into a `StateChunk` with the given chunk type.
///
/// This is the Rust equivalent of the C++ `addSformatChunk` lambda in
/// `FCEUSS_SaveMS`. It reads from C++ memory via the SFORMAT entries and
/// produces a `StateChunk` ready for V2 file assembly.
///
/// # Safety
/// Same as `fceux11_rust_sformat_serialize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_sformat_to_chunk(
    entries: *const FceuxSformatEntry,
    count: usize,
    chunk_type: u8,
    out_chunk: *mut StateChunk,
) -> bool {
    if entries.is_null() || out_chunk.is_null() {
        return false;
    }

    let mut buf_ptr: *mut u8 = std::ptr::null_mut();
    let mut buf_len: usize = 0;

    if !unsafe { fceux11_rust_sformat_serialize(entries, count, &mut buf_ptr, &mut buf_len) } {
        return false;
    }

    if buf_len == 0 || buf_ptr.is_null() {
        return false;
    }

    let data = unsafe { Vec::from_raw_parts(buf_ptr, buf_len, buf_len) };
    let chunk = StateChunk {
        chunk_type,
        data,
    };

    unsafe {
        std::ptr::write(out_chunk, chunk);
    }
    true
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a FceuxSformatEntry with desc as a static pointer.
    fn make_entry<'a>(
        v: *const u8,
        s: u32,
        desc: &'static [u8; 4],
    ) -> FceuxSformatEntry {
        FceuxSformatEntry {
            v,
            s,
            _pad: 0,
            desc: desc.as_ptr(),
        }
    }

    #[test]
    fn test_serialize_single_entry() {
        let data = [0xAAu8, 0xBB, 0xCC, 0xDD];
        static DESC: [u8; 4] = [b'T', b'E', b'S', b'T'];
        let entry = make_entry(data.as_ptr(), 4, &DESC);

        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        assert!(unsafe {
            fceux11_rust_sformat_serialize(&entry, 1, &mut out_buf, &mut out_len)
        });

        assert!(!out_buf.is_null());
        assert_eq!(out_len, 4 + 4 + 4); // desc + size + data

        let result = unsafe { std::slice::from_raw_parts(out_buf, out_len) };
        assert_eq!(&result[0..4], b"TEST");
        assert_eq!(&result[4..8], &4u32.to_le_bytes());
        assert_eq!(&result[8..12], &[0xAA, 0xBB, 0xCC, 0xDD]);

        unsafe { fceux11_rust_sformat_buf_free(out_buf, out_len) };
    }

    #[test]
    fn test_serialize_multiple_entries() {
        let data1 = [0x01u8, 0x02];
        let data2 = [0x03u8, 0x04, 0x05];
        static DESC1: [u8; 4] = [b'A', b'B', b'C', b'D'];
        static DESC2: [u8; 4] = [b'E', b'F', b'G', b'H'];
        let entries = [
            make_entry(data1.as_ptr(), 2, &DESC1),
            make_entry(data2.as_ptr(), 3, &DESC2),
        ];

        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        assert!(unsafe {
            fceux11_rust_sformat_serialize(entries.as_ptr(), 2, &mut out_buf, &mut out_len)
        });

        // Entry 1: 4+4+2 = 10, Entry 2: 4+4+3 = 11, Total = 21
        assert_eq!(out_len, 21);

        let result = unsafe { std::slice::from_raw_parts(out_buf, out_len) };
        assert_eq!(&result[0..4], b"ABCD");
        assert_eq!(&result[4..8], &2u32.to_le_bytes());
        assert_eq!(&result[8..10], &[0x01, 0x02]);
        assert_eq!(&result[10..14], b"EFGH");
        assert_eq!(&result[14..18], &3u32.to_le_bytes());
        assert_eq!(&result[18..21], &[0x03, 0x04, 0x05]);

        unsafe { fceux11_rust_sformat_buf_free(out_buf, out_len) };
    }

    #[test]
    fn test_serialize_with_rlsb_flag() {
        let data = [0x12u8, 0x34, 0x56, 0x78];
        static DESC: [u8; 4] = [b'R', b'L', b'S', b'B'];
        let entry = make_entry(data.as_ptr(), 4 | FCEUSTATE_RLSB, &DESC);

        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        assert!(unsafe {
            fceux11_rust_sformat_serialize(&entry, 1, &mut out_buf, &mut out_len)
        });

        let result = unsafe { std::slice::from_raw_parts(out_buf, out_len) };
        assert_eq!(&result[4..8], &4u32.to_le_bytes());

        unsafe { fceux11_rust_sformat_buf_free(out_buf, out_len) };
    }

    #[test]
    fn test_serialize_null_entry_skipped() {
        static DESC: [u8; 4] = [b'N', b'U', b'L', b'L'];
        let entry = make_entry(std::ptr::null(), 4, &DESC);

        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;

        assert!(unsafe {
            fceux11_rust_sformat_serialize(&entry, 1, &mut out_buf, &mut out_len)
        });

        assert_eq!(out_len, 0);
    }

    #[test]
    fn test_roundtrip_serialize_deserialize() {
        let original = [0xDEu8, 0xAD, 0xBE, 0xEF];
        static DESC: [u8; 4] = [b'T', b'E', b'S', b'T'];

        // Serialize
        let entry_out = make_entry(original.as_ptr(), 4, &DESC);
        let mut out_buf: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;
        assert!(unsafe {
            fceux11_rust_sformat_serialize(&entry_out, 1, &mut out_buf, &mut out_len)
        });

        // Deserialize into target buffer
        let mut target = [0u8; 4];
        let entry_in = make_entry(target.as_mut_ptr(), 4, &DESC);

        let stream = unsafe { std::slice::from_raw_parts(out_buf, out_len) };
        assert!(unsafe {
            fceux11_rust_sformat_deserialize(
                stream.as_ptr(),
                stream.len(),
                &entry_in as *const FceuxSformatEntry as *mut FceuxSformatEntry,
                1,
                0,
            )
        });

        assert_eq!(target, original);
        unsafe { fceux11_rust_sformat_buf_free(out_buf, out_len) };
    }

    #[test]
    fn test_deserialize_unknown_desc_skipped() {
        let mut stream = Vec::new();
        stream.extend_from_slice(b"XXXX");
        stream.extend_from_slice(&2u32.to_le_bytes());
        stream.extend_from_slice(&[0xAA, 0xBB]);

        let mut target = [0u8; 2];
        static DESC: [u8; 4] = [b'T', b'E', b'S', b'T'];
        let entry = make_entry(target.as_mut_ptr(), 2, &DESC);

        assert!(unsafe {
            fceux11_rust_sformat_deserialize(
                stream.as_ptr(),
                stream.len(),
                &entry as *const FceuxSformatEntry as *mut FceuxSformatEntry,
                1,
                0,
            )
        });

        assert_eq!(target, [0u8; 2]);
    }

    #[test]
    fn test_crc32_deterministic() {
        let data = [0xAAu8, 0xBB, 0xCC, 0xDD];
        let crc1 = unsafe { fceux11_rust_sformat_crc32(data.as_ptr(), data.len()) };
        let crc2 = unsafe { fceux11_rust_sformat_crc32(data.as_ptr(), data.len()) };
        assert_eq!(crc1, crc2);
        assert_ne!(crc1, 0);
    }

    #[test]
    fn test_crc32_empty() {
        let crc = unsafe { fceux11_rust_sformat_crc32(std::ptr::null(), 0) };
        assert_eq!(crc, 0);
    }

    #[test]
    fn test_crc32_different_data() {
        let data1 = [0x01u8, 0x02, 0x03];
        let data2 = [0x04u8, 0x05, 0x06];
        let crc1 = unsafe { fceux11_rust_sformat_crc32(data1.as_ptr(), data1.len()) };
        let crc2 = unsafe { fceux11_rust_sformat_crc32(data2.as_ptr(), data2.len()) };
        assert_ne!(crc1, crc2);
    }

    #[test]
    fn test_validate_valid_stream() {
        // Build a valid SFORMAT stream: desc(4) + size(4) + data(4) = 12 bytes
        let mut stream = Vec::new();
        stream.extend_from_slice(b"TEST");
        stream.extend_from_slice(&4u32.to_le_bytes());
        stream.extend_from_slice(&[0xAA, 0xBB, 0xCC, 0xDD]);

        let count = unsafe { fceux11_rust_sformat_validate(stream.as_ptr(), stream.len()) };
        assert_eq!(count, 1);
    }

    #[test]
    fn test_validate_multiple_entries() {
        let mut stream = Vec::new();
        // Entry 1
        stream.extend_from_slice(b"AAAA");
        stream.extend_from_slice(&2u32.to_le_bytes());
        stream.extend_from_slice(&[0x01, 0x02]);
        // Entry 2
        stream.extend_from_slice(b"BBBB");
        stream.extend_from_slice(&3u32.to_le_bytes());
        stream.extend_from_slice(&[0x03, 0x04, 0x05]);

        let count = unsafe { fceux11_rust_sformat_validate(stream.as_ptr(), stream.len()) };
        assert_eq!(count, 2);
    }

    #[test]
    fn test_validate_truncated() {
        let mut stream = Vec::new();
        stream.extend_from_slice(b"TEST");
        stream.extend_from_slice(&8u32.to_le_bytes()); // claims 8 bytes
        stream.extend_from_slice(&[0xAA, 0xBB]); // only 2 bytes

        let count = unsafe { fceux11_rust_sformat_validate(stream.as_ptr(), stream.len()) };
        assert_eq!(count, -1);
    }

    #[test]
    fn test_validate_empty() {
        let count = unsafe { fceux11_rust_sformat_validate(std::ptr::null(), 0) };
        assert_eq!(count, 0);
    }
}
