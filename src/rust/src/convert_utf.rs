/// Rust implementation of Unicode UTF conversion routines.
/// Replaces src/utils/ConvertUTF.c with a memory-safe equivalent.
///
/// Phase 6 (v0.2.7): Unicode Conversion

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConversionResult {
    ConversionOK = 0,
    SourceExhausted = 1,
    TargetExhausted = 2,
    SourceIllegal = 3,
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConversionFlags {
    StrictConversion = 0,
    LenientConversion = 1,
}

const UNI_REPLACEMENT_CHAR: u32 = 0x0000FFFD;
const UNI_MAX_BMP: u32 = 0x0000FFFF;
const UNI_MAX_LEGAL_UTF32: u32 = 0x0010FFFF;
const UNI_SUR_HIGH_START: u32 = 0xD800;
const UNI_SUR_HIGH_END: u32 = 0xDBFF;
const UNI_SUR_LOW_START: u32 = 0xDC00;
const UNI_SUR_LOW_END: u32 = 0xDFFF;

/// Decode a single UTF-8 codepoint from the beginning of the slice.
/// Returns (codepoint, bytes_consumed) on success.
/// On failure, returns the appropriate ConversionResult.
fn decode_utf8_char(source: &[u8]) -> Result<(u32, usize), ConversionResult> {
    if source.is_empty() {
        return Err(ConversionResult::SourceExhausted);
    }

    let b0 = source[0];

    // 1-byte: U+0000..U+007F
    if b0 < 0x80 {
        return Ok((b0 as u32, 1));
    }

    // Invalid leading bytes: 0x80-0xBF (continuation) or 0xC0-0xC1 (overlong)
    if b0 < 0xC2 {
        return Err(ConversionResult::SourceIllegal);
    }

    // 2-byte: U+0080..U+07FF
    if b0 < 0xE0 {
        if source.len() < 2 {
            return Err(ConversionResult::SourceExhausted);
        }
        let b1 = source[1];
        if (b1 & 0xC0) != 0x80 {
            return Err(ConversionResult::SourceIllegal);
        }
        let ch = ((b0 & 0x1F) as u32) << 6 | (b1 & 0x3F) as u32;
        return Ok((ch, 2));
    }

    // 3-byte: U+0800..U+FFFF
    if b0 < 0xF0 {
        if source.len() < 3 {
            return Err(ConversionResult::SourceExhausted);
        }
        let b1 = source[1];
        let b2 = source[2];
        if (b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 {
            return Err(ConversionResult::SourceIllegal);
        }
        // Overlong check for E0xx
        if b0 == 0xE0 && b1 < 0xA0 {
            return Err(ConversionResult::SourceIllegal);
        }
        // Surrogate check for EDxx
        if b0 == 0xED && b1 > 0x9F {
            return Err(ConversionResult::SourceIllegal);
        }
        let ch = ((b0 & 0x0F) as u32) << 12
            | ((b1 & 0x3F) as u32) << 6
            | (b2 & 0x3F) as u32;
        return Ok((ch, 3));
    }

    // 4-byte: U+10000..U+10FFFF
    if b0 <= 0xF4 {
        if source.len() < 4 {
            return Err(ConversionResult::SourceExhausted);
        }
        let b1 = source[1];
        let b2 = source[2];
        let b3 = source[3];
        if (b1 & 0xC0) != 0x80 || (b2 & 0xC0) != 0x80 || (b3 & 0xC0) != 0x80 {
            return Err(ConversionResult::SourceIllegal);
        }
        // Overlong check for F0xx
        if b0 == 0xF0 && b1 < 0x90 {
            return Err(ConversionResult::SourceIllegal);
        }
        // Beyond U+10FFFF check for F4xx
        if b0 == 0xF4 && b1 > 0x8F {
            return Err(ConversionResult::SourceIllegal);
        }
        let ch = ((b0 & 0x07) as u32) << 18
            | ((b1 & 0x3F) as u32) << 12
            | ((b2 & 0x3F) as u32) << 6
            | (b3 & 0x3F) as u32;
        return Ok((ch, 4));
    }

    // 0xF5-0xFF: invalid
    Err(ConversionResult::SourceIllegal)
}

/// Compute the number of UTF-8 bytes required to encode a codepoint.
fn utf8_bytes_for_codepoint(ch: u32) -> usize {
    if ch < 0x80 {
        1
    } else if ch < 0x800 {
        2
    } else if ch < 0x10000 {
        3
    } else {
        4
    }
}

/// Encode a single codepoint into a UTF-8 buffer.
/// The buffer must be large enough (caller checks).
fn encode_utf8(ch: u32, buf: &mut [u8]) -> usize {
    let c = match char::from_u32(ch) {
        Some(c) => c,
        None => '\u{FFFD}',
    };
    c.encode_utf8(buf).len()
}

// ---------------------------------------------------------------------------
// C ABI wrappers
// ---------------------------------------------------------------------------

/// C ABI: Convert UTF-8 to UTF-16.
///
/// # Safety
/// All pointer parameters must be valid and non-null (except as noted).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_convert_utf8_to_utf16(
    source_start: *mut *const u8,
    source_end: *const u8,
    target_start: *mut *mut u16,
    target_end: *const u16,
    flags: ConversionFlags,
) -> ConversionResult {
    if source_start.is_null() || target_start.is_null() {
        return ConversionResult::SourceIllegal;
    }

    let mut source = unsafe { *source_start };
    let mut target = unsafe { *target_start };

    while (source as usize) < (source_end as usize) {
        let source_slice = unsafe {
            std::slice::from_raw_parts(source, source_end as usize - source as usize)
        };

        let (ch, consumed) = match decode_utf8_char(source_slice) {
            Ok(v) => v,
            Err(e) => {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return e;
            }
        };

        if ch <= UNI_MAX_BMP {
            if target as usize + std::mem::size_of::<u16>() > target_end as usize {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::TargetExhausted;
            }
            let write_val = if ch >= UNI_SUR_HIGH_START && ch <= UNI_SUR_LOW_END {
                if flags == ConversionFlags::StrictConversion {
                    unsafe {
                        *source_start = source;
                        *target_start = target;
                    }
                    return ConversionResult::SourceIllegal;
                }
                UNI_REPLACEMENT_CHAR as u16
            } else {
                ch as u16
            };
            unsafe {
                *target = write_val;
                target = target.add(1);
            }
        } else if ch > UNI_MAX_LEGAL_UTF32 {
            if flags == ConversionFlags::StrictConversion {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceIllegal;
            }
            if target as usize + std::mem::size_of::<u16>() > target_end as usize {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::TargetExhausted;
            }
            unsafe {
                *target = UNI_REPLACEMENT_CHAR as u16;
                target = target.add(1);
            }
        } else {
            // Surrogate pair needed
            if target as usize + 2 * std::mem::size_of::<u16>() > target_end as usize {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::TargetExhausted;
            }
            let ch_adj = ch - 0x10000;
            unsafe {
                *target = (0xD800 + (ch_adj >> 10)) as u16;
                target = target.add(1);
                *target = (0xDC00 + (ch_adj & 0x3FF)) as u16;
                target = target.add(1);
            }
        }

        source = unsafe { source.add(consumed) };
    }

    unsafe {
        *source_start = source;
        *target_start = target;
    }
    ConversionResult::ConversionOK
}

/// C ABI: Convert UTF-16 to UTF-8.
///
/// # Safety
/// All pointer parameters must be valid and non-null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_convert_utf16_to_utf8(
    source_start: *mut *const u16,
    source_end: *const u16,
    target_start: *mut *mut u8,
    target_end: *const u8,
    flags: ConversionFlags,
) -> ConversionResult {
    if source_start.is_null() || target_start.is_null() {
        return ConversionResult::SourceIllegal;
    }

    let mut source = unsafe { *source_start };
    let mut target = unsafe { *target_start };

    while (source as usize) < (source_end as usize) {
        let old_source = source;
        let mut ch = unsafe { *source } as u32;
        source = unsafe { source.add(1) };

        if ch >= UNI_SUR_HIGH_START && ch <= UNI_SUR_HIGH_END {
            if (source as usize) < (source_end as usize) {
                let ch2 = unsafe { *source } as u32;
                if ch2 >= UNI_SUR_LOW_START && ch2 <= UNI_SUR_LOW_END {
                    ch = ((ch - UNI_SUR_HIGH_START) << 10)
                        + (ch2 - UNI_SUR_LOW_START)
                        + 0x10000;
                    source = unsafe { source.add(1) };
                } else if flags == ConversionFlags::StrictConversion {
                    source = old_source;
                    unsafe {
                        *source_start = source;
                        *target_start = target;
                    }
                    return ConversionResult::SourceIllegal;
                }
            } else {
                source = old_source;
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceExhausted;
            }
        } else if flags == ConversionFlags::StrictConversion {
            if ch >= UNI_SUR_LOW_START && ch <= UNI_SUR_LOW_END {
                source = old_source;
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceIllegal;
            }
        }

        let bytes_to_write = utf8_bytes_for_codepoint(ch);

        if target as usize + bytes_to_write > target_end as usize {
            source = old_source;
            unsafe {
                *source_start = source;
                *target_start = target;
            }
            return ConversionResult::TargetExhausted;
        }

        let target_slice = unsafe {
            std::slice::from_raw_parts_mut(target, target_end as usize - target as usize)
        };
        let written = encode_utf8(ch, &mut target_slice[..bytes_to_write]);
        target = unsafe { target.add(written) };
    }

    unsafe {
        *source_start = source;
        *target_start = target;
    }
    ConversionResult::ConversionOK
}

/// C ABI: Convert UTF-8 to UTF-32.
///
/// # Safety
/// All pointer parameters must be valid and non-null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_convert_utf8_to_utf32(
    source_start: *mut *const u8,
    source_end: *const u8,
    target_start: *mut *mut u32,
    target_end: *const u32,
    flags: ConversionFlags,
) -> ConversionResult {
    if source_start.is_null() || target_start.is_null() {
        return ConversionResult::SourceIllegal;
    }

    let mut source = unsafe { *source_start };
    let mut target = unsafe { *target_start };

    while (source as usize) < (source_end as usize) {
        let source_slice = unsafe {
            std::slice::from_raw_parts(source, source_end as usize - source as usize)
        };

        let (ch, consumed) = match decode_utf8_char(source_slice) {
            Ok(v) => v,
            Err(e) => {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return e;
            }
        };

        if target as usize + 1 > target_end as usize {
            unsafe {
                *source_start = source;
                *target_start = target;
            }
            return ConversionResult::TargetExhausted;
        }

        if ch <= UNI_MAX_LEGAL_UTF32 {
            if ch >= UNI_SUR_HIGH_START && ch <= UNI_SUR_LOW_END {
                if flags == ConversionFlags::StrictConversion {
                    unsafe {
                        *source_start = source;
                        *target_start = target;
                    }
                    return ConversionResult::SourceIllegal;
                }
                unsafe {
                    *target = UNI_REPLACEMENT_CHAR;
                    target = target.add(1);
                }
            } else {
                unsafe {
                    *target = ch;
                    target = target.add(1);
                }
            }
        } else {
            if flags == ConversionFlags::StrictConversion {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceIllegal;
            }
            unsafe {
                *target = UNI_REPLACEMENT_CHAR;
                target = target.add(1);
            }
        }

        source = unsafe { source.add(consumed) };
    }

    unsafe {
        *source_start = source;
        *target_start = target;
    }
    ConversionResult::ConversionOK
}

/// C ABI: Convert UTF-32 to UTF-8.
///
/// # Safety
/// All pointer parameters must be valid and non-null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_convert_utf32_to_utf8(
    source_start: *mut *const u32,
    source_end: *const u32,
    target_start: *mut *mut u8,
    target_end: *const u8,
    flags: ConversionFlags,
) -> ConversionResult {
    if source_start.is_null() || target_start.is_null() {
        return ConversionResult::SourceIllegal;
    }

    let mut source = unsafe { *source_start };
    let mut target = unsafe { *target_start };
    let mut result = ConversionResult::ConversionOK;

    while (source as usize) < (source_end as usize) {
        let mut ch = unsafe { *source };

        if flags == ConversionFlags::StrictConversion {
            if ch >= UNI_SUR_HIGH_START && ch <= UNI_SUR_LOW_END {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceIllegal;
            }
        }

        let bytes_to_write = if ch < 0x80 {
            1
        } else if ch < 0x800 {
            2
        } else if ch < 0x10000 {
            3
        } else if ch <= UNI_MAX_LEGAL_UTF32 {
            4
        } else {
            ch = UNI_REPLACEMENT_CHAR;
            result = ConversionResult::SourceIllegal;
            3
        };

        if target as usize + bytes_to_write > target_end as usize {
            unsafe {
                *source_start = source;
                *target_start = target;
            }
            return ConversionResult::TargetExhausted;
        }

        let target_slice = unsafe {
            std::slice::from_raw_parts_mut(target, target_end as usize - target as usize)
        };
        let written = encode_utf8(ch, &mut target_slice[..bytes_to_write]);
        target = unsafe { target.add(written) };
        source = unsafe { source.add(1) };
    }

    unsafe {
        *source_start = source;
        *target_start = target;
    }
    result
}

/// C ABI: Convert UTF-16 to UTF-32.
///
/// # Safety
/// All pointer parameters must be valid and non-null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_convert_utf16_to_utf32(
    source_start: *mut *const u16,
    source_end: *const u16,
    target_start: *mut *mut u32,
    target_end: *const u32,
    flags: ConversionFlags,
) -> ConversionResult {
    if source_start.is_null() || target_start.is_null() {
        return ConversionResult::SourceIllegal;
    }

    let mut source = unsafe { *source_start };
    let mut target = unsafe { *target_start };

    while (source as usize) < (source_end as usize) {
        let old_source = source;
        let mut ch = unsafe { *source } as u32;
        source = unsafe { source.add(1) };

        if ch >= UNI_SUR_HIGH_START && ch <= UNI_SUR_HIGH_END {
            if (source as usize) < (source_end as usize) {
                let ch2 = unsafe { *source } as u32;
                if ch2 >= UNI_SUR_LOW_START && ch2 <= UNI_SUR_LOW_END {
                    ch = ((ch - UNI_SUR_HIGH_START) << 10)
                        + (ch2 - UNI_SUR_LOW_START)
                        + 0x10000;
                    source = unsafe { source.add(1) };
                } else if flags == ConversionFlags::StrictConversion {
                    source = old_source;
                    unsafe {
                        *source_start = source;
                        *target_start = target;
                    }
                    return ConversionResult::SourceIllegal;
                }
            } else {
                source = old_source;
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceExhausted;
            }
        } else if flags == ConversionFlags::StrictConversion {
            if ch >= UNI_SUR_LOW_START && ch <= UNI_SUR_LOW_END {
                source = old_source;
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceIllegal;
            }
        }

        if target as usize >= target_end as usize {
            source = old_source;
            unsafe {
                *source_start = source;
                *target_start = target;
            }
            return ConversionResult::TargetExhausted;
        }

        unsafe {
            *target = ch;
            target = target.add(1);
        }
    }

    unsafe {
        *source_start = source;
        *target_start = target;
    }
    ConversionResult::ConversionOK
}

/// C ABI: Convert UTF-32 to UTF-16.
///
/// # Safety
/// All pointer parameters must be valid and non-null.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_convert_utf32_to_utf16(
    source_start: *mut *const u32,
    source_end: *const u32,
    target_start: *mut *mut u16,
    target_end: *const u16,
    flags: ConversionFlags,
) -> ConversionResult {
    if source_start.is_null() || target_start.is_null() {
        return ConversionResult::SourceIllegal;
    }

    let mut source = unsafe { *source_start };
    let mut target = unsafe { *target_start };

    while (source as usize) < (source_end as usize) {
        let ch = unsafe { *source };

        if ch <= UNI_MAX_BMP {
            if target as usize + std::mem::size_of::<u16>() > target_end as usize {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::TargetExhausted;
            }
            if ch >= UNI_SUR_HIGH_START && ch <= UNI_SUR_LOW_END {
                if flags == ConversionFlags::StrictConversion {
                    unsafe {
                        *source_start = source;
                        *target_start = target;
                    }
                    return ConversionResult::SourceIllegal;
                }
                unsafe {
                    *target = UNI_REPLACEMENT_CHAR as u16;
                    target = target.add(1);
                }
            } else {
                unsafe {
                    *target = ch as u16;
                    target = target.add(1);
                }
            }
        } else if ch > UNI_MAX_LEGAL_UTF32 {
            if flags == ConversionFlags::StrictConversion {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::SourceIllegal;
            }
            if target as usize >= target_end as usize {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::TargetExhausted;
            }
            unsafe {
                *target = UNI_REPLACEMENT_CHAR as u16;
                target = target.add(1);
            }
        } else {
            // Surrogate pair needed
            if target as usize + 1 >= target_end as usize {
                unsafe {
                    *source_start = source;
                    *target_start = target;
                }
                return ConversionResult::TargetExhausted;
            }
            let ch_adj = ch - 0x10000;
            unsafe {
                *target = (0xD800 + (ch_adj >> 10)) as u16;
                target = target.add(1);
                *target = (0xDC00 + (ch_adj & 0x3FF)) as u16;
                target = target.add(1);
            }
        }

        source = unsafe { source.add(1) };
    }

    unsafe {
        *source_start = source;
        *target_start = target;
    }
    ConversionResult::ConversionOK
}

/// C ABI: Check whether a byte sequence is a legal UTF-8 sequence.
///
/// # Safety
/// `source` and `source_end` must be valid pointers with `source <= source_end`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_is_legal_utf8_sequence(
    source: *const u8,
    source_end: *const u8,
) -> u8 {
    if source.is_null() || source_end.is_null() || source >= source_end {
        return 0;
    }
    let slice = unsafe {
        std::slice::from_raw_parts(source, source_end as usize - source as usize)
    };
    match decode_utf8_char(slice) {
        Ok((_, _)) => 1,
        Err(_) => 0,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_utf8_ascii() {
        assert_eq!(decode_utf8_char(b"A"), Ok((0x41, 1)));
    }

    #[test]
    fn test_decode_utf8_2byte() {
        assert_eq!(decode_utf8_char("é".as_bytes()), Ok((0xE9, 2)));
    }

    #[test]
    fn test_decode_utf8_3byte() {
        assert_eq!(decode_utf8_char("中".as_bytes()), Ok((0x4E2D, 3)));
    }

    #[test]
    fn test_decode_utf8_4byte() {
        assert_eq!(decode_utf8_char("𐍈".as_bytes()), Ok((0x10348, 4)));
    }

    #[test]
    fn test_decode_utf8_overlong() {
        // C0 80 is an overlong encoding of U+0000
        assert_eq!(decode_utf8_char(&[0xC0, 0x80]), Err(ConversionResult::SourceIllegal));
    }

    #[test]
    fn test_decode_utf8_surrogate() {
        // ED A0 80 is UTF-8 encoding of U+D800 (surrogate)
        assert_eq!(decode_utf8_char(&[0xED, 0xA0, 0x80]), Err(ConversionResult::SourceIllegal));
    }

    #[test]
    fn test_decode_utf8_beyond_max() {
        // F4 90 80 80 is beyond U+10FFFF
        assert_eq!(decode_utf8_char(&[0xF4, 0x90, 0x80, 0x80]), Err(ConversionResult::SourceIllegal));
    }

    #[test]
    fn test_decode_utf8_incomplete() {
        assert_eq!(decode_utf8_char(&[0xE4, 0xB8]), Err(ConversionResult::SourceExhausted));
    }

    #[test]
    fn test_utf8_to_utf16_basic() {
        let input: [u8; 3] = [0xE4, 0xB8, 0xAD]; // 中
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(input.len()) };
        let mut output: [u16; 2] = [0; 2];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(2) };

        let res = fceux11_rust_convert_utf8_to_utf16(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::ConversionOK);
        assert_eq!(source, source_end);
        assert_eq!(target, unsafe { output.as_mut_ptr().add(1) });
        assert_eq!(output[0], 0x4E2D);
    }

    #[test]
    fn test_utf8_to_utf16_surrogate_strict() {
        // ED A0 80 = U+D800 (surrogate) in UTF-8
        let input: [u8; 3] = [0xED, 0xA0, 0x80];
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(input.len()) };
        let mut output: [u16; 2] = [0; 2];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(2) };

        let res = fceux11_rust_convert_utf8_to_utf16(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::SourceIllegal);
        assert_eq!(source, input.as_ptr());
    }

    #[test]
    fn test_utf8_to_utf16_surrogate_lenient() {
        let input: [u8; 3] = [0xED, 0xA0, 0x80];
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(input.len()) };
        let mut output: [u16; 2] = [0; 2];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(2) };

        let res = fceux11_rust_convert_utf8_to_utf16(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::LenientConversion,
        );
        // ED A0 80 is an invalid UTF-8 sequence (surrogate), rejected even in lenient mode
        assert_eq!(res, ConversionResult::SourceIllegal);
        assert_eq!(source, input.as_ptr());
    }

    #[test]
    fn test_utf8_to_utf16_target_exhausted() {
        let input: [u8; 4] = [0xF0, 0x9F, 0x98, 0x80]; // 😀 (needs surrogate pair)
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(input.len()) };
        let mut output: [u16; 1] = [0];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(1) };

        let res = fceux11_rust_convert_utf8_to_utf16(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::TargetExhausted);
        assert_eq!(source, input.as_ptr());
    }

    #[test]
    fn test_utf16_to_utf8_basic() {
        let input: [u16; 1] = [0x4E2D]; // 中
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(1) };
        let mut output: [u8; 4] = [0; 4];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(4) };

        let res = fceux11_rust_convert_utf16_to_utf8(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::ConversionOK);
        assert_eq!(&output[..3], [0xE4, 0xB8, 0xAD]);
    }

    #[test]
    fn test_utf16_to_utf8_surrogate_pair() {
        let input: [u16; 2] = [0xD83D, 0xDE00]; // 😀
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(2) };
        let mut output: [u8; 4] = [0; 4];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(4) };

        let res = fceux11_rust_convert_utf16_to_utf8(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::ConversionOK);
        assert_eq!(&output[..4], [0xF0, 0x9F, 0x98, 0x80]);
    }

    #[test]
    fn test_utf16_to_utf32_unpaired_high_strict() {
        let input: [u16; 1] = [0xD800];
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(1) };
        let mut output: [u32; 1] = [0];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(1) };

        let res = fceux11_rust_convert_utf16_to_utf32(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        // Single high surrogate with no following code unit -> sourceExhausted (matches original C)
        assert_eq!(res, ConversionResult::SourceExhausted);
        assert_eq!(source, input.as_ptr());
    }

    #[test]
    fn test_utf32_to_utf16_basic() {
        let input: [u32; 1] = [0x4E2D];
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(1) };
        let mut output: [u16; 2] = [0; 2];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(2) };

        let res = fceux11_rust_convert_utf32_to_utf16(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::ConversionOK);
        assert_eq!(output[0], 0x4E2D);
    }

    #[test]
    fn test_utf32_to_utf16_non_bmp() {
        let input: [u32; 1] = [0x10348]; // 𐍈
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(1) };
        let mut output: [u16; 2] = [0; 2];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(2) };

        let res = fceux11_rust_convert_utf32_to_utf16(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::ConversionOK);
        assert_eq!(output[0], 0xD800);
        assert_eq!(output[1], 0xDF48);
    }

    #[test]
    fn test_utf32_to_utf8_beyond_legal() {
        let input: [u32; 1] = [0x110000];
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(1) };
        let mut output: [u8; 4] = [0; 4];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(4) };

        let res = fceux11_rust_convert_utf32_to_utf8(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::StrictConversion,
        );
        assert_eq!(res, ConversionResult::SourceIllegal);
        // Original C advances source even for illegal values > 0x10FFFF
        assert_eq!(source, unsafe { input.as_ptr().add(1) });
    }

    #[test]
    fn test_is_legal_utf8_sequence() {
        assert_eq!(fceux11_rust_is_legal_utf8_sequence(b"A".as_ptr(), unsafe { b"A".as_ptr().add(1) }), 1);
        let data = [0xED, 0xA0, 0x80];
        assert_eq!(fceux11_rust_is_legal_utf8_sequence(data.as_ptr(), unsafe { data.as_ptr().add(3) }), 0);
    }

    #[test]
    fn test_utf8_to_utf32_beyond_max() {
        // F4 90 80 80 = beyond U+10FFFF
        let input: [u8; 4] = [0xF4, 0x90, 0x80, 0x80];
        let mut source = input.as_ptr();
        let source_end = unsafe { source.add(4) };
        let mut output: [u32; 1] = [0];
        let mut target = output.as_mut_ptr();
        let target_end = unsafe { target.add(1) };

        let res = fceux11_rust_convert_utf8_to_utf32(
            &mut source,
            source_end,
            &mut target,
            target_end,
            ConversionFlags::LenientConversion,
        );
        // F4 90 80 80 is illegal UTF-8 (beyond U+10FFFF), rejected at decode stage
        assert_eq!(res, ConversionResult::SourceIllegal);
        assert_eq!(source, input.as_ptr());
    }

    #[test]
    fn test_roundtrip_utf8_utf16_utf8() {
        let original: &[u8] = "Hello 世界 🌍".as_bytes();
        let mut utf16_buf: [u16; 64] = [0; 64];
        let mut utf8_buf: [u8; 64] = [0; 64];

        // UTF-8 -> UTF-16
        let mut s1 = original.as_ptr();
        let s1_end = unsafe { s1.add(original.len()) };
        let mut t1 = utf16_buf.as_mut_ptr();
        let t1_end = unsafe { t1.add(64) };
        let res1 = fceux11_rust_convert_utf8_to_utf16(
            &mut s1, s1_end, &mut t1, t1_end, ConversionFlags::StrictConversion,
        );
        assert_eq!(res1, ConversionResult::ConversionOK);
        let utf16_len = unsafe { t1.offset_from(utf16_buf.as_mut_ptr()) } as usize;

        // UTF-16 -> UTF-8
        let mut s2 = utf16_buf.as_ptr();
        let s2_end = unsafe { s2.add(utf16_len) };
        let mut t2 = utf8_buf.as_mut_ptr();
        let t2_end = unsafe { t2.add(64) };
        let res2 = fceux11_rust_convert_utf16_to_utf8(
            &mut s2, s2_end, &mut t2, t2_end, ConversionFlags::StrictConversion,
        );
        assert_eq!(res2, ConversionResult::ConversionOK);
        let utf8_len = unsafe { t2.offset_from(utf8_buf.as_mut_ptr()) } as usize;

        assert_eq!(&utf8_buf[..utf8_len], original);
    }
}
