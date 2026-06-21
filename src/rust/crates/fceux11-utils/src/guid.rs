use std::ffi::c_char;

#[repr(C)]
pub struct FceuGuid {
    pub data: [u8; 16],
}

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

fn write_hex_u32(val: u32, out: &mut [c_char]) {
    for i in 0..8u8 {
        let nibble = ((val >> (i * 4)) & 0xF) as usize;
        out[7 - i as usize] = HEX_CHARS[nibble] as c_char;
    }
}

fn write_hex_u16(val: u16, out: &mut [c_char]) {
    for i in 0..4u8 {
        let nibble = ((val >> (i * 4)) & 0xF) as usize;
        out[3 - i as usize] = HEX_CHARS[nibble] as c_char;
    }
}

/// C ABI: Generate a new random GUID (UUID v4) into the provided buffer.
///
/// # Safety
/// `guid` must point to a valid, writable `FceuGuid`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_guid_new(guid: *mut FceuGuid) {
    let guid = unsafe { &mut *guid };
    let id = uuid::Uuid::new_v4();
    guid.data.copy_from_slice(id.as_bytes());
}

/// C ABI: Convert a GUID to a hex string representation.
///
/// # Safety
/// `guid` must point to a valid 16-byte GUID.
/// Returns a pointer to a thread-local static buffer. Caller should copy immediately.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_guid_to_string(guid: *const FceuGuid) -> *const c_char {
    if guid.is_null() {
        return std::ptr::null();
    }
    let guid = unsafe { &*guid };

    // Format: XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX (8-4-4-4-12)
    // Matches C++: sprintf(buf,"%08X-%04X-%04X-%04X-%02X%02X%02X%02X%02X%02X",
    //   FCEU_de32lsb(data), FCEU_de16lsb(data+4), FCEU_de16lsb(data+6),
    //   FCEU_de16lsb(data+8), data[10],...)
    thread_local! {
        static BUF: std::cell::RefCell<[c_char; 37]> = const {
            std::cell::RefCell::new([0; 37])
        };
    }

    BUF.with(|s| {
        let mut s = s.borrow_mut();

        // Part 1: bytes 0-3 as little-endian u32 -> 8 hex digits
        let v0 = u32::from_le_bytes([guid.data[0], guid.data[1], guid.data[2], guid.data[3]]);
        write_hex_u32(v0, &mut s[..8]);

        s[8] = b'-' as c_char;

        // Part 2: bytes 4-5 as little-endian u16 -> 4 hex digits
        let v1 = u16::from_le_bytes([guid.data[4], guid.data[5]]);
        write_hex_u16(v1, &mut s[9..13]);

        s[13] = b'-' as c_char;

        // Part 3: bytes 6-7 as little-endian u16 -> 4 hex digits
        let v2 = u16::from_le_bytes([guid.data[6], guid.data[7]]);
        write_hex_u16(v2, &mut s[14..18]);

        s[18] = b'-' as c_char;

        // Part 4: bytes 8-9 as little-endian u16 -> 4 hex digits
        let v3 = u16::from_le_bytes([guid.data[8], guid.data[9]]);
        write_hex_u16(v3, &mut s[19..23]);

        s[23] = b'-' as c_char;

        // Part 5: bytes 10-15 as-is, 2 hex digits each -> 12 hex digits
        for (i, &byte) in guid.data[10..16].iter().enumerate() {
            let hi = (byte >> 4) as usize;
            let lo = (byte & 0xF) as usize;
            s[24 + i * 2] = HEX_CHARS[hi] as c_char;
            s[25 + i * 2] = HEX_CHARS[lo] as c_char;
        }

        s[36] = 0;
        s.as_ptr()
    })
}

/// C ABI: Parse a hex string into a GUID.
///
/// # Safety
/// `guid` must point to a valid, writable `FceuGuid`.
/// `str` must point to a null-terminated string in the format "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX".
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_guid_scan(guid: *mut FceuGuid, str: *const c_char) {
    if guid.is_null() || str.is_null() {
        return;
    }
    let guid = unsafe { &mut *guid };
    let cstr = unsafe { std::ffi::CStr::from_ptr(str) };
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };

    // Parse "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
    // The C++ code uses strtoul (big-endian parse) + FCEU_en32lsb (little-endian store),
    // which effectively byte-swaps each group. We replicate this by parsing big-endian
    // hex values and storing as little-endian bytes.

    let clean: String = s.chars().filter(|c| *c != '-').collect();
    if clean.len() != 32 {
        return;
    }

    fn parse_hex_u32(s: &str, start: usize) -> u32 {
        let end = start + 8;
        u32::from_str_radix(&s[start..end], 16).unwrap_or(0)
    }

    fn parse_hex_u16(s: &str, start: usize) -> u16 {
        let end = start + 4;
        u16::from_str_radix(&s[start..end], 16).unwrap_or(0)
    }

    fn parse_hex_byte(s: &str, start: usize) -> u8 {
        u8::from_str_radix(&s[start..start + 2], 16).unwrap_or(0)
    }

    // Group 1: 8 hex digits -> u32 (big-endian parse) -> store as little-endian bytes
    let v0 = parse_hex_u32(&clean, 0);
    guid.data[0..4].copy_from_slice(&v0.to_le_bytes());

    // Group 2: 4 hex digits -> u16 (big-endian parse) -> store as little-endian bytes
    let v1 = parse_hex_u16(&clean, 8);
    guid.data[4..6].copy_from_slice(&v1.to_le_bytes());

    // Group 3: 4 hex digits -> u16 (big-endian parse) -> store as little-endian bytes
    let v2 = parse_hex_u16(&clean, 12);
    guid.data[6..8].copy_from_slice(&v2.to_le_bytes());

    // Group 4: 4 hex digits -> u16 (big-endian parse) -> store as little-endian bytes
    let v3 = parse_hex_u16(&clean, 16);
    guid.data[8..10].copy_from_slice(&v3.to_le_bytes());

    // Group 5: 12 hex digits -> 6 bytes (direct, no endian conversion needed)
    for i in 0..6 {
        guid.data[10 + i] = parse_hex_byte(&clean, 20 + i * 2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_roundtrip() {
        unsafe {
            let mut guid = FceuGuid { data: [0; 16] };
            fceux11_rust_guid_new(&mut guid);

            let ptr = fceux11_rust_guid_to_string(&guid);
            let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
            let s = cstr.to_str().unwrap();
            let bytes = s.as_bytes();

            // Verify format: 8-4-4-4-12
            assert_eq!(s.len(), 36);
            assert_eq!(bytes[8], b'-');
            assert_eq!(bytes[13], b'-');
            assert_eq!(bytes[18], b'-');
            assert_eq!(bytes[23], b'-');

            // Parse it back
            let mut guid2 = FceuGuid { data: [0; 16] };
            fceux11_rust_guid_scan(&mut guid2, s.as_ptr() as *const c_char);

            assert_eq!(guid.data, guid2.data);
        }
    }

    #[test]
    fn test_guid_scan_format() {
        unsafe {
            let mut guid = FceuGuid { data: [0; 16] };
            let test_str = "12345678-1234-1234-1234-123456789012";
            fceux11_rust_guid_scan(&mut guid, test_str.as_ptr() as *const c_char);

            // Verify it can be converted back
            let ptr = fceux11_rust_guid_to_string(&guid);
            let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
            let result = cstr.to_str().unwrap();

            assert_eq!(result, test_str);
        }
    }
}
