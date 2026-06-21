use crc32fast::Hasher;
use std::slice;

/// C ABI: Compute CRC32 over a byte buffer with an optional initial CRC value.
///
/// # Safety
/// `buf` must point to at least `len` valid bytes, or be NULL when `len` is 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_crc32(crc: u32, buf: *const u8, len: u32) -> u32 {
    if buf.is_null() || len == 0 {
        return crc;
    }
    let data = unsafe { slice::from_raw_parts(buf, len as usize) };
    let mut hasher = Hasher::new_with_initial(crc);
    hasher.update(data);
    hasher.finalize()
}
