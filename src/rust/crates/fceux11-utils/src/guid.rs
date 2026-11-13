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
/// `guid` must point to a valid, writable `FceuGuid`, or be NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_guid_new(guid: *mut FceuGuid) {
    // hotfix1 P1-13 (H-13): tolerate NULL rather than dereferencing into
    // address 0.
    if guid.is_null() {
        return;
    }
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

/// Decode one ASCII hex digit. `None` for any byte that is not one.
fn hex_digit_value(b: u8) -> Option<u32> {
    match b {
        b'0'..=b'9' => Some((b - b'0') as u32),
        b'a'..=b'f' => Some((b - b'a' + 10) as u32),
        b'A'..=b'F' => Some((b - b'A' + 10) as u32),
        _ => None,
    }
}

/// Parse `bytes[start .. start + len]` as a big-endian hex number.
///
/// Byte based on purpose: slicing a `&str` by byte range panics when the range
/// splits a multi-byte UTF-8 char. The semantics match
/// `uN::from_str_radix(.., 16).unwrap_or(0)`: one bad digit makes the whole
/// group 0 (not just that digit), and a leading '+' is accepted as a sign.
fn parse_hex_group(bytes: &[u8], start: usize, len: usize) -> u32 {
    let group = match bytes.get(start..start + len) {
        Some(g) => g,
        None => return 0,
    };
    let digits = match group.first() {
        Some(b'+') => &group[1..],
        _ => group,
    };
    let mut value: u32 = 0;
    for &b in digits {
        match hex_digit_value(b) {
            Some(v) => value = (value << 4) | v,
            None => return 0,
        }
    }
    value
}

/// C ABI: Parse a hex string into a GUID.
///
/// # Safety
/// `guid` must point to a valid, writable `FceuGuid`.
/// `str` must point to a null-terminated string in the format "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX".
///
/// v2.0.4 (V24): the parse never panics and every failure path leaves a
/// zeroed GUID behind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_guid_scan(guid: *mut FceuGuid, str: *const c_char) {
    if guid.is_null() || str.is_null() {
        return;
    }
    let guid = unsafe { &mut *guid };
    let cstr = unsafe { std::ffi::CStr::from_ptr(str) };
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => {
            // Not valid UTF-8: fail closed with a zeroed GUID.
            guid.data = [0u8; 16];
            return;
        }
    };

    // Parse "XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
    // The C++ code uses strtoul (big-endian parse) + FCEU_en32lsb (little-endian store),
    // which effectively byte-swaps each group. We replicate this by parsing big-endian
    // hex values and storing as little-endian bytes.
    //
    // v2.0.4 (V24): the parse works on raw bytes only. Slicing a `&str` by byte
    // range panics when the range splits a multi-byte UTF-8 char, and this
    // function is `extern "C"` (a panic cannot unwind out of it) => the process
    // would abort. No `&str` range slice is taken below.
    let clean: Vec<u8> = s.bytes().filter(|b| *b != b'-').collect();
    if clean.len() != 32 {
        guid.data = [0u8; 16];
        return;
    }
    // A GUID is an ASCII hex string: any non-ASCII byte (e.g. a multi-byte
    // UTF-8 char) means the input is not a GUID at all => fail closed with a
    // zeroed GUID instead of trying to interpret those bytes.
    if !clean.iter().all(|b| b.is_ascii()) {
        guid.data = [0u8; 16];
        return;
    }

    // Group 1: 8 hex digits -> u32 (big-endian parse) -> store as little-endian bytes
    let v0 = parse_hex_group(&clean, 0, 8);
    guid.data[0..4].copy_from_slice(&v0.to_le_bytes());

    // Group 2: 4 hex digits -> u16 (big-endian parse) -> store as little-endian bytes
    let v1 = parse_hex_group(&clean, 8, 4) as u16;
    guid.data[4..6].copy_from_slice(&v1.to_le_bytes());

    // Group 3: 4 hex digits -> u16 (big-endian parse) -> store as little-endian bytes
    let v2 = parse_hex_group(&clean, 12, 4) as u16;
    guid.data[6..8].copy_from_slice(&v2.to_le_bytes());

    // Group 4: 4 hex digits -> u16 (big-endian parse) -> store as little-endian bytes
    let v3 = parse_hex_group(&clean, 16, 4) as u16;
    guid.data[8..10].copy_from_slice(&v3.to_le_bytes());

    // Group 5: 12 hex digits -> 6 bytes (direct, no endian conversion needed)
    for i in 0..6 {
        guid.data[10 + i] = parse_hex_group(&clean, 20 + i * 2, 2) as u8;
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

    /// Scan `s` into `guid` through the C ABI entry point.
    ///
    /// `CString` is used so the NUL terminator is guaranteed; a plain `&str`
    /// literal is not required to be NUL terminated.
    fn scan_str(guid: &mut FceuGuid, s: &str) {
        let c = std::ffi::CString::new(s).unwrap();
        unsafe {
            fceux11_rust_guid_scan(guid, c.as_ptr());
        }
    }

    #[test]
    fn test_guid_scan_multibyte_does_not_panic() {
        // v2.0.4 (V24): the byte layout is picked so that the old implementation
        // sliced a `String` in the middle of the 3-byte UTF-8 char. After the
        // two '-' separators are dropped the group-2 cut lands on byte 12,
        // which is inside the char => "byte index 12 is not a char boundary"
        // => panic => abort (this function is `extern "C"`, it cannot unwind).
        //
        //   "12345678901"        11 ASCII bytes
        //   "\u{20AC}"            3 bytes (E2 82 AC)
        //   "0123456789abcdefgh" 18 ASCII bytes
        //   => 11 + 3 + 18 = 32 bytes once the separators are removed.
        let input = "12345678901-\u{20AC}-0123456789abcdefgh";
        assert_eq!(input.bytes().filter(|b| *b != b'-').count(), 32);

        // Prefilled with a non-zero pattern: the all-zero assertion below
        // proves the failure path really wrote the zeros.
        let mut guid = FceuGuid { data: [0xA5; 16] };
        scan_str(&mut guid, input);
        assert_eq!(guid.data, [0u8; 16]);
    }

    #[test]
    fn test_guid_scan_byte_layout_unchanged() {
        // The expected bytes are derived by hand from the "big-endian parse +
        // little-endian store" rule (the C++ strtoul + FCEU_en32lsb
        // semantics), not from the function under test:
        //
        //   group 1 "01234567"     -> 0x01234567 -> LE 67 45 23 01
        //   group 2 "89ab"         -> 0x89ab     -> LE ab 89
        //   group 3 "cdef"         -> 0xcdef     -> LE ef cd
        //   group 4 "0123"         -> 0x0123     -> LE 23 01
        //   group 5 "456789abcdef" -> 45 67 89 ab cd ef
        let input = "01234567-89ab-cdef-0123-456789abcdef";
        let expected: [u8; 16] =
            *b"\x67\x45\x23\x01\xab\x89\xef\xcd\x23\x01\x45\x67\x89\xab\xcd\xef";
        let mut guid = FceuGuid { data: [0xA5; 16] };
        scan_str(&mut guid, input);
        assert_eq!(guid.data, expected);

        // The C++ visible representation is the same string again.
        unsafe {
            let ptr = fceux11_rust_guid_to_string(&guid);
            let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
            assert_eq!(cstr.to_str().unwrap(), input);
        }
    }

    #[test]
    fn test_guid_scan_invalid_length_zeroes_guid() {
        // 33 hex digits after the separators are dropped => rejected.
        let mut guid = FceuGuid { data: [0xA5; 16] };
        scan_str(&mut guid, "01234567-89ab-cdef-0123-456789abcdef0");
        assert_eq!(guid.data, [0u8; 16]);
    }

    #[test]
    fn test_guid_scan_non_utf8_zeroes_guid() {
        // 0xFF is not valid UTF-8 => the string is rejected.
        let raw = b"01234567-89ab-cdef-0123-456789abcde\xff\0";
        let cstr = std::ffi::CStr::from_bytes_with_nul(raw).unwrap();
        let mut guid = FceuGuid { data: [0xA5; 16] };
        unsafe {
            fceux11_rust_guid_scan(&mut guid, cstr.as_ptr());
        }
        assert_eq!(guid.data, [0u8; 16]);
    }

    #[test]
    fn test_guid_scan_invalid_hex_digit_keeps_group_zero() {
        // The lax behaviour of the old implementation is kept as-is: one bad
        // digit only zeroes the group that holds it, the other groups still
        // parse. Tightening this is a separate work item.
        //
        //   group 5 = "456789abcdeZ" -> the last pair "eZ" is invalid -> 0x00,
        //   the five pairs before it still parse.
        let input = "01234567-89ab-cdef-0123-456789abcdeZ";
        let expected: [u8; 16] =
            *b"\x67\x45\x23\x01\xab\x89\xef\xcd\x23\x01\x45\x67\x89\xab\xcd\x00";
        let mut guid = FceuGuid { data: [0xA5; 16] };
        scan_str(&mut guid, input);
        assert_eq!(guid.data, expected);
    }
}
