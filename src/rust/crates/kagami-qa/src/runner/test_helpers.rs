//! Shared harness utilities — the Rust equivalent of the C++
//! `tests/core/test_helpers.h` (golden loading, hashing, comparators,
//! path resolution) for the migrated ROM-oracle harnesses.
//!
//! Consolidates the helpers that the per-harness modules
//! (`rom_regression`, `mapper_byte_diff`, …) previously each carried
//! their own copy of:
//! - [`resolve_rom_path`] — resolve a relative `fixtures/...` path
//!   against the CTest `WORKING_DIRECTORY` (`tests/`);
//! - [`crc32`] / [`crc32_update`] — IEEE 802.3 CRC32 (byte-identical to
//!   the C++ `CalcCRC32` chain);
//! - golden-binary header constants + [`validate_golden_header`] — the
//!   `FMAP` header format used by `mapper_byte_diff`.
//!
//! The engine-lifecycle half of the C++ header (`core_init` / `load_rom`
//! / `emulate_n`) is provided by the Rust adapter layer
//! (`SutAdapter::load` / `step` + `Fceux11DirectAdapter::full_reset`),
//! so no separate Rust module is needed for it.

use std::path::{Path, PathBuf};

/// Size of a golden file header (magic 8 + version 4 + body_size 4).
pub const GOLDEN_HEADER_SIZE: usize = 16;
/// Golden file magic bytes ("FMAP" + 4 NUL bytes).
pub const GOLDEN_MAGIC: [u8; 8] = *b"FMAP\0\0\0\0";
/// Golden file format version.
pub const GOLDEN_VERSION: u32 = 1;

/// Resolve a relative ROM path against `workdir`, returning an absolute
/// path. Mirrors the C++ harness behaviour: tests run with
/// `WORKING_DIRECTORY = tests/`, so `fixtures/mapper_nrom.nes` resolves
/// to `tests/fixtures/mapper_nrom.nes`.
pub fn resolve_rom_path(workdir: &Path, rel: &str) -> PathBuf {
    let p = Path::new(rel);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        workdir.join(p)
    }
}

/// CRC32 over a byte slice using the IEEE 802.3 polynomial. Equivalent
/// to `CalcCRC32(0, buf, len)` in the C++ harness (`crc32fast` — the
/// same crate used by `fceux11-utils`).
pub fn crc32(buf: &[u8]) -> u32 {
    crc32_update(0, buf)
}

/// CRC32 with an explicit initial value (matches `CalcCRC32(crc, buf, len)`).
pub fn crc32_update(crc: u32, buf: &[u8]) -> u32 {
    let mut h = crc32fast::Hasher::new_with_initial(crc);
    h.update(buf);
    h.finalize()
}

/// Validate a golden binary header (`FMAP` magic + version + body_size).
/// Returns `Ok(body_size)` on success or a human-readable reason.
pub fn validate_golden_header(data: &[u8]) -> Result<u32, String> {
    if data.len() < GOLDEN_HEADER_SIZE {
        return Err(format!("golden file too short ({} bytes)", data.len()));
    }
    if data[0..8] != GOLDEN_MAGIC {
        return Err("golden magic mismatch".into());
    }
    let version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    if version != GOLDEN_VERSION {
        return Err(format!("golden version {} != {}", version, GOLDEN_VERSION));
    }
    let body_size = u32::from_le_bytes(data[12..16].try_into().unwrap());
    if data.len() != GOLDEN_HEADER_SIZE + body_size as usize {
        return Err(format!(
            "golden size {} != header + body_size {}",
            data.len(),
            body_size
        ));
    }
    Ok(body_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_relative_path_against_workdir() {
        let p = resolve_rom_path(Path::new("/tmp/work"), "fixtures/x.nes");
        assert_eq!(p, PathBuf::from("/tmp/work/fixtures/x.nes"));
    }

    #[test]
    fn resolve_absolute_path_passes_through() {
        let p = resolve_rom_path(Path::new("/tmp/work"), "/abs/x.nes");
        assert_eq!(p, PathBuf::from("/abs/x.nes"));
    }

    #[test]
    fn crc32_matches_known_value() {
        // Known CRC32 of "123456789" is 0xCBF43926 (IEEE 802.3).
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn crc32_chained_matches_single_call() {
        let buf = b"The quick brown fox";
        let whole = crc32(buf);
        let (a, b) = buf.split_at(10);
        assert_eq!(whole, crc32_update(crc32_update(0, a), b));
    }

    #[test]
    fn golden_header_valid() {
        let mut data = Vec::new();
        data.extend_from_slice(&GOLDEN_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&3u32.to_le_bytes());
        data.extend_from_slice(&[1, 2, 3]);
        assert_eq!(validate_golden_header(&data), Ok(3));
    }

    #[test]
    fn golden_header_bad_magic_rejected() {
        let mut data = vec![0u8; 20];
        data[0] = b'X';
        assert!(validate_golden_header(&data).is_err());
    }

    #[test]
    fn golden_header_size_mismatch_rejected() {
        let mut data = Vec::new();
        data.extend_from_slice(&GOLDEN_MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&5u32.to_le_bytes());
        assert!(validate_golden_header(&data).is_err());
    }
}
