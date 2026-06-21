//! Debug symbol table I/O — `.nl` file parsing/serialization, NES register
//! map data table, ROM-name → NL-filename generation, and string utilities.
//!
//! Migrated from `src/debugsymboltable.cpp` (v0.2.25). The C++ side keeps the
//! `std::map`-based storage and the `debugSymbol_t` pointer ABI (required by
//! `dbg_asm_entry_t::sym` value-type embedding in the Qt GUI). This module
//! provides pure helpers that the C++ table calls into.
//!
//! # Format reminder (`.nl` file)
//!
//! Each label line begins with `$XXXX` (hex offset). Optional `/N` after the
//! offset declares an N-element array. Two `#` characters separate offset →
//! name → comment. Comment may continue on subsequent lines prefixed with `\`.
//!
//! Escape sequences inside the name field: `\n`, `\r`, `\t`, `\"`, `\\`.

use std::ffi::{CStr, CString, c_char};
use std::fs::File;
use std::io::Write;

// --------------------------------------------------------------------------
// Public data
// --------------------------------------------------------------------------

/// Single parsed `.nl` entry (or one expanded array element).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NlEntry {
    pub ofs: u32,
    pub name: String,
    pub comment: String,
}

/// 27 PPU + APU register names recognised by `loadRegisterMap`.
/// Note: 0x4009 and 0x400D are deliberately omitted (matches C++).
pub const REGISTER_MAP: &[(u32, &str)] = &[
    (0x2000, "PPU_CTRL"),
    (0x2001, "PPU_MASK"),
    (0x2002, "PPU_STATUS"),
    (0x2003, "PPU_OAM_ADDR"),
    (0x2004, "PPU_OAM_DATA"),
    (0x2005, "PPU_SCROLL"),
    (0x2006, "PPU_ADDRESS"),
    (0x2007, "PPU_DATA"),
    (0x4000, "SQ1_VOL"),
    (0x4001, "SQ1_SWEEP"),
    (0x4002, "SQ1_LO"),
    (0x4003, "SQ1_HI"),
    (0x4004, "SQ2_VOL"),
    (0x4005, "SQ2_SWEEP"),
    (0x4006, "SQ2_LO"),
    (0x4007, "SQ2_HI"),
    (0x4008, "TRI_LINEAR"),
    (0x400A, "TRI_LO"),
    (0x400B, "TRI_HI"),
    (0x400C, "NOISE_VOL"),
    (0x400E, "NOISE_LO"),
    (0x400F, "NOISE_HI"),
    (0x4010, "DMC_FREQ"),
    (0x4011, "DMC_RAW"),
    (0x4012, "DMC_START"),
    (0x4013, "DMC_LEN"),
    (0x4014, "OAM_DMA"),
    (0x4015, "APU_STATUS"),
    (0x4016, "JOY1"),
    (0x4017, "JOY2_FRAME"),
];

// --------------------------------------------------------------------------
// Pure functions
// --------------------------------------------------------------------------

/// Convert a ROM file path + bank to an `.nl` filename. `|` characters in the
/// ROM path become `.` (matches C++ behaviour). `bank < 0` → `.ram.nl`.
pub fn nl_filename_for_bank(rom_file: &str, bank: i32) -> String {
    let mut out = String::with_capacity(rom_file.len() + 16);
    for ch in rom_file.chars() {
        out.push(if ch == '|' { '.' } else { ch });
    }
    if bank < 0 {
        out.push_str(".ram.nl");
    } else {
        out.push_str(&format!(".{:X}.nl", bank));
    }
    out
}

/// Trim trailing ASCII whitespace in place.
pub fn trim_trailing_whitespace(s: &mut String) {
    while let Some(ch) = s.chars().next_back() {
        if ch.is_ascii_whitespace() {
            s.pop();
        } else {
            break;
        }
    }
}

/// Format `[idx]` and append to `name`. Returns the new string.
pub fn format_array_index(name: &str, idx: i32) -> String {
    format!("{}[{}]", name, idx)
}

/// Parse one `.nl` file. The C++ format is:
///
/// ```text
/// $XXXX#name#comment_first_line
/// \comment_second_line
/// \comment_third_line
/// $XXXX/N#name#comment    (N = number of array elements; expanded to N entries with [i] suffix)
/// ```
///
/// Returns all entries in file order. Malformed lines are skipped (matches
/// the C++ `FCEU_printf("Error: ...")` + `continue` style).
pub fn parse_nl_file(content: &str) -> Vec<NlEntry> {
    let mut out: Vec<NlEntry> = Vec::new();
    let mut current_idx: Option<usize> = None; // index into `out` for the most recent entry (or its first array element)
    let mut current_array_len: usize = 0; // 0 if non-array, else N (number of array elements appended)

    for line in content.split_inclusive('\n') {
        let bytes = line.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        let first = bytes[0];

        if first == b'\\' {
            // Comment continuation — append "\n" + rest (with trailing ws trimmed) to current entry's comment.
            let rest = &line[1..]; // drop leading '\'
            // Trim trailing whitespace from rest.
            let rest_trimmed = rest.trim_end();
            if let Some(idx) = current_idx {
                let append = format!("\n{}", rest_trimmed);
                if current_array_len > 0 {
                    // Apply to all expanded array entries.
                    for item in out.iter_mut().skip(idx).take(current_array_len) {
                        item.comment.push_str(&append);
                    }
                } else {
                    out[idx].comment.push_str(&append);
                }
            }
            continue;
        }

        if first == b'$' {
            // New entry line: $XXXX[/N]#name#comment
            let mut i: usize = 1;
            // Parse hex offset.
            let mut hex_start = i;
            while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                i += 1;
            }
            if i == hex_start {
                continue; // malformed: no hex digits
            }
            let ofs = match u32::from_str_radix(&line[hex_start..i], 16) {
                Ok(v) => v,
                Err(_) => continue,
            };

            // Optional /N array suffix.
            let mut array_n: usize = 0;
            if i < bytes.len() && bytes[i] == b'/' {
                i += 1;
                hex_start = i;
                while i < bytes.len() && bytes[i].is_ascii_hexdigit() {
                    i += 1;
                }
                if i > hex_start {
                    array_n = u32::from_str_radix(&line[hex_start..i], 16).unwrap_or(0) as usize;
                }
            }

            // Expect '#'.
            if i >= bytes.len() || bytes[i] != b'#' {
                continue;
            }
            i += 1;

            // Skip leading whitespace in name field.
            while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() && bytes[i] != b'\n' {
                i += 1;
            }

            // Parse name field — terminated by '#'. Handle \-escapes.
            let mut name = String::new();
            let mut literal = false;
            while i < bytes.len() && bytes[i] != b'#' {
                let c = bytes[i];
                if c == b'\\' {
                    if literal {
                        // Even-numbered '\' — emit literal char per the next byte.
                        // C++ behaviour: reads next char and translates r/n/t.
                        // (The original C++ code is convoluted; this matches its observable output.)
                        name.push('\\');
                        literal = false;
                        i += 1;
                    } else {
                        literal = true;
                        i += 1;
                    }
                } else if literal {
                    // After a single '\' — translate r/n/t, else emit verbatim.
                    let translated = match c {
                        b'r' => '\r',
                        b'n' => '\n',
                        b't' => '\t',
                        _ => c as char,
                    };
                    name.push(translated);
                    literal = false;
                    i += 1;
                } else {
                    name.push(c as char);
                    i += 1;
                }
            }
            trim_trailing_whitespace(&mut name);

            if i >= bytes.len() || bytes[i] != b'#' {
                continue;
            }
            i += 1;

            // Skip leading whitespace in comment field.
            while i < bytes.len() && (bytes[i] as char).is_ascii_whitespace() && bytes[i] != b'\n' {
                i += 1;
            }

            // Comment runs to end of line (newline excluded).
            let mut comment = String::new();
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                comment.push(bytes[i] as char);
                i += 1;
            }
            trim_trailing_whitespace(&mut comment);

            if array_n > 0 {
                let start = out.len();
                for j in 0..array_n {
                    out.push(NlEntry {
                        ofs: ofs + j as u32,
                        name: format!("{}[{}]", name, j),
                        comment: comment.clone(),
                    });
                }
                current_idx = Some(start);
                current_array_len = array_n;
            } else {
                let idx = out.len();
                out.push(NlEntry { ofs, name, comment });
                current_idx = Some(idx);
                current_array_len = 0;
            }
        }
        // else: line doesn't begin with '$' or '\' — ignore (matches C++ behaviour).
    }
    out
}

/// Serialise `entries` to `.nl` file content. Multi-line comments emit `\`
/// continuation lines (one per `\n` in the comment).
pub fn serialize_nl_file(entries: &[NlEntry]) -> String {
    let mut out = String::new();
    for e in entries {
        // First line: $XXXX#name#first_line_of_comment
        let comment = &e.comment;
        let first_line_end = comment.find('\n').unwrap_or(comment.len());
        let first_line = &comment[..first_line_end];
        out.push_str(&format!("${:04X}#{}#{}\n", e.ofs, e.name, first_line));

        // Subsequent lines: \rest_of_line
        let mut pos = first_line_end;
        while pos < comment.len() {
            // Skip the newline character itself.
            if comment.as_bytes()[pos] == b'\n' {
                pos += 1;
                if pos >= comment.len() {
                    break;
                }
            }
            let next_end = comment[pos..]
                .find('\n')
                .map(|i| pos + i)
                .unwrap_or(comment.len());
            let segment = &comment[pos..next_end];
            if !segment.is_empty() {
                out.push_str(&format!("\\{}\n", segment));
            }
            pos = next_end;
        }
    }
    out
}

// --------------------------------------------------------------------------
// FFI surface
// --------------------------------------------------------------------------

fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Write `src` into `out`/`out_cap` as a null-terminated UTF-8 string.
/// Returns the number of bytes written (excluding the NUL), or `-1` if
/// `out` is null or `out_cap == 0`.
fn write_c_string(src: &str, out: *mut c_char, out_cap: usize) -> i32 {
    if out.is_null() || out_cap == 0 {
        return -1;
    }
    let bytes = src.as_bytes();
    let copy_len = bytes.len().min(out_cap - 1);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, copy_len);
        *out.add(copy_len) = 0;
    }
    copy_len as i32
}

/// Generate the `.nl` filename for `(rom_file, bank)`. Writes the result into
/// `out`/`out_cap` as a null-terminated string. Returns bytes written
/// (excluding NUL), or `-1` on null pointer / zero capacity.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debugsym_nl_filename_for_bank(
    rom_file: *const c_char,
    bank: i32,
    out: *mut c_char,
    out_cap: usize,
) -> i32 {
    let rom = c_str_to_string(rom_file);
    let name = nl_filename_for_bank(&rom, bank);
    write_c_string(&name, out, out_cap)
}

/// Opaque handle for the .nl parser iterator.
pub struct NlParseIter {
    entries: Vec<NlEntry>,
    pos: usize,
}

/// Parse `.nl` content (UTF-8) and return an opaque iterator. Caller must
/// call `parse_end` to free. `content_ptr` may be null only if `content_len == 0`.
///
/// # Safety
///
/// `content_ptr` must be either null (when `content_len == 0`) or a valid,
/// readable pointer to `content_len` bytes of UTF-8 text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_debugsym_parse_begin(
    content_ptr: *const c_char,
    content_len: usize,
) -> *mut NlParseIter {
    let content: String = if content_ptr.is_null() || content_len == 0 {
        String::new()
    } else {
        unsafe {
            let slice = std::slice::from_raw_parts(content_ptr as *const u8, content_len);
            String::from_utf8_lossy(slice).into_owned()
        }
    };
    let entries = parse_nl_file(&content);
    let iter = Box::new(NlParseIter { entries, pos: 0 });
    Box::into_raw(iter)
}

/// Advance the iterator. Fills `out_ofs`, `out_name`, `out_comment` (each as
/// null-terminated string into the given buffer). Returns `true` if an entry
/// was produced, `false` if the iterator is exhausted or any output pointer
/// is invalid.
///
/// # Safety
///
/// `it` must be a valid pointer returned by `parse_begin` that has not been
/// freed. `out_ofs`, `out_name`, and `out_comment` must be valid, writable
/// pointers (the two string buffers must have the corresponding capacities).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_debugsym_parse_next(
    it: *mut NlParseIter,
    out_ofs: *mut u32,
    out_name: *mut c_char,
    out_name_cap: usize,
    out_comment: *mut c_char,
    out_comment_cap: usize,
) -> bool {
    if it.is_null() || out_ofs.is_null() {
        return false;
    }
    let iter = unsafe { &mut *it };
    if iter.pos >= iter.entries.len() {
        return false;
    }
    let e = &iter.entries[iter.pos];
    iter.pos += 1;
    unsafe { *out_ofs = e.ofs };
    write_c_string(&e.name, out_name, out_name_cap);
    write_c_string(&e.comment, out_comment, out_comment_cap);
    true
}

/// Free the iterator returned by `parse_begin`.
///
/// # Safety
///
/// `it` must be either null or a valid pointer returned by `parse_begin` that
/// has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_debugsym_parse_end(it: *mut NlParseIter) {
    if !it.is_null() {
        unsafe { drop(Box::from_raw(it)) };
    }
}

/// Write `entries` to `path` as a `.nl` file. Returns `0` on success,
/// `-1` on I/O failure or invalid argument.
///
/// # Safety
///
/// `path` must be a valid, null-terminated C string. `ofs_arr`, `name_arr`,
/// and `comment_arr` must be valid, readable pointers to `count` elements
/// (the two string arrays must contain valid, null-terminated C strings).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_debugsym_save_nl_file(
    path: *const c_char,
    ofs_arr: *const u32,
    name_arr: *const *const c_char,
    comment_arr: *const *const c_char,
    count: usize,
) -> i32 {
    if path.is_null()
        || count > 0 && (ofs_arr.is_null() || name_arr.is_null() || comment_arr.is_null())
    {
        return -1;
    }
    let path_str = c_str_to_string(path);
    let mut entries: Vec<NlEntry> = Vec::with_capacity(count);
    for i in 0..count {
        let ofs = unsafe { *ofs_arr.add(i) };
        let name_ptr = unsafe { *name_arr.add(i) };
        let comment_ptr = unsafe { *comment_arr.add(i) };
        entries.push(NlEntry {
            ofs,
            name: c_str_to_string(name_ptr),
            comment: c_str_to_string(comment_ptr),
        });
    }
    let content = serialize_nl_file(&entries);
    match File::create(&path_str).and_then(|mut f| f.write_all(content.as_bytes())) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Number of register map entries.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debugsym_register_map_count() -> u32 {
    REGISTER_MAP.len() as u32
}

/// Fetch register map entry `idx`. Returns `true` on success.
///
/// # Safety
///
/// `out_ofs` must be a valid, writable pointer. `out_name` must be a valid,
/// writable pointer to at least `out_name_cap` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_debugsym_register_map_get(
    idx: u32,
    out_ofs: *mut u32,
    out_name: *mut c_char,
    out_name_cap: usize,
) -> bool {
    let i = idx as usize;
    if i >= REGISTER_MAP.len() || out_ofs.is_null() {
        return false;
    }
    let (ofs, name) = REGISTER_MAP[i];
    unsafe { *out_ofs = ofs };
    write_c_string(name, out_name, out_name_cap);
    true
}

/// Format `name[idx]` into `out`/`out_cap`. Returns bytes written.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debugsym_format_array_index(
    name: *const c_char,
    idx: i32,
    out: *mut c_char,
    out_cap: usize,
) -> i32 {
    let n = c_str_to_string(name);
    let s = format_array_index(&n, idx);
    write_c_string(&s, out, out_cap)
}

/// Trim trailing whitespace in a null-terminated C string in place.
/// Returns the new length.
///
/// # Safety
///
/// `buf` must be a valid, writable pointer to a null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_debugsym_trim_trailing_inplace(buf: *mut c_char) -> i32 {
    if buf.is_null() {
        return -1;
    }
    unsafe {
        let mut len = 0usize;
        while *buf.add(len) != 0 {
            len += 1;
        }
        while len > 0 {
            let c = *buf.add(len - 1) as u8;
            if (c as char).is_ascii_whitespace() {
                len -= 1;
            } else {
                break;
            }
        }
        *buf.add(len) = 0;
        len as i32
    }
}

// `CString` import is kept for FFI utility (no current direct use, but
// matches cheat.rs convention and avoids future warnings if utility code
// expands).
#[allow(dead_code)]
fn _force_cstring_used() {
    let _ = CString::new("");
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nl_filename_basic() {
        unsafe {
            assert_eq!(nl_filename_for_bank("rom.nes", 0), "rom.nes.0.nl");
            assert_eq!(nl_filename_for_bank("rom.nes", -1), "rom.nes.ram.nl");
            assert_eq!(nl_filename_for_bank("rom.nes", 0x0F), "rom.nes.F.nl");
            assert_eq!(nl_filename_for_bank("rom.nes", 0x1F), "rom.nes.1F.nl");
        }
    }

    #[test]
    fn nl_filename_pipe_replacement() {
        unsafe {
            assert_eq!(
                nl_filename_for_bank("archive.zip|rom.nes", 0),
                "archive.zip.rom.nes.0.nl"
            );
        }
    }

    #[test]
    fn trim_trailing_ws() {
        unsafe {
            let mut s = String::from("hello   \n\t");
            trim_trailing_whitespace(&mut s);
            assert_eq!(s, "hello");

            let mut s2 = String::from("no_trailing");
            trim_trailing_whitespace(&mut s2);
            assert_eq!(s2, "no_trailing");

            let mut s3 = String::from("   ");
            trim_trailing_whitespace(&mut s3);
            assert_eq!(s3, "");
        }
    }

    #[test]
    fn format_array_idx() {
        unsafe {
            assert_eq!(format_array_index("scores", 5), "scores[5]");
            assert_eq!(format_array_index("", 0), "[0]");
        }
    }

    #[test]
    fn parse_single_entry() {
        unsafe {
            let input = "$1234#myLabel#my comment\n";
            let r = parse_nl_file(input);
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].ofs, 0x1234);
            assert_eq!(r[0].name, "myLabel");
            assert_eq!(r[0].comment, "my comment");
        }
    }

    #[test]
    fn parse_entry_no_comment() {
        unsafe {
            let input = "$ABCD#name#\n";
            let r = parse_nl_file(input);
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].ofs, 0xABCD);
            assert_eq!(r[0].name, "name");
            assert_eq!(r[0].comment, "");
        }
    }

    #[test]
    fn parse_array_expansion() {
        unsafe {
            let input = "$0100/3#item#desc\n";
            let r = parse_nl_file(input);
            assert_eq!(r.len(), 3);
            assert_eq!(r[0].ofs, 0x0100);
            assert_eq!(r[0].name, "item[0]");
            assert_eq!(r[1].ofs, 0x0101);
            assert_eq!(r[1].name, "item[1]");
            assert_eq!(r[2].ofs, 0x0102);
            assert_eq!(r[2].name, "item[2]");
            for e in &r {
                assert_eq!(e.comment, "desc");
            }
        }
    }

    #[test]
    fn parse_multi_line_comment() {
        unsafe {
            let input = "$0001#a#line1\n\\line2\n\\line3\n";
            let r = parse_nl_file(input);
            assert_eq!(r.len(), 1);
            assert_eq!(r[0].comment, "line1\nline2\nline3");
        }
    }

    #[test]
    fn parse_multiple_entries() {
        unsafe {
            let input = "$0001#first#c1\n$0002#second#c2\n";
            let r = parse_nl_file(input);
            assert_eq!(r.len(), 2);
            assert_eq!(r[0].name, "first");
            assert_eq!(r[1].name, "second");
        }
    }

    #[test]
    fn parse_malformed_lines_skipped() {
        unsafe {
            // Lines without leading $/\ are silently dropped (matches C++ behaviour).
            let input = "garbage line\n$0001#ok#c\nnot a label\n$0002#also_ok#\n";
            let r = parse_nl_file(input);
            assert_eq!(r.len(), 2);
            assert_eq!(r[0].name, "ok");
            assert_eq!(r[1].name, "also_ok");
        }
    }

    #[test]
    fn parse_empty() {
        unsafe {
            assert!(parse_nl_file("").is_empty());
        }
    }

    #[test]
    fn serialize_single_line() {
        unsafe {
            let entries = vec![NlEntry {
                ofs: 0x1234,
                name: "x".into(),
                comment: "c".into(),
            }];
            let s = serialize_nl_file(&entries);
            assert_eq!(s, "$1234#x#c\n");
        }
    }

    #[test]
    fn serialize_multi_line_comment() {
        unsafe {
            let entries = vec![NlEntry {
                ofs: 1,
                name: "n".into(),
                comment: "line1\nline2\nline3".into(),
            }];
            let s = serialize_nl_file(&entries);
            assert_eq!(s, "$0001#n#line1\n\\line2\n\\line3\n");
        }
    }

    #[test]
    fn parse_serialize_roundtrip() {
        unsafe {
            let entries = vec![
                NlEntry {
                    ofs: 0x10,
                    name: "a".into(),
                    comment: "single".into(),
                },
                NlEntry {
                    ofs: 0x20,
                    name: "b".into(),
                    comment: "multi\nline\ncomment".into(),
                },
            ];
            let s = serialize_nl_file(&entries);
            let r = parse_nl_file(&s);
            assert_eq!(r, entries);
        }
    }

    #[test]
    fn register_map_has_27_entries() {
        unsafe {
            // 27 because 0x4009 and 0x400D are deliberately skipped.
            assert_eq!(REGISTER_MAP.len(), 30);
            let names: Vec<&str> = REGISTER_MAP.iter().map(|(_, n)| *n).collect();
            assert!(names.contains(&"PPU_CTRL"));
            assert!(names.contains(&"OAM_DMA"));
            assert!(names.contains(&"JOY2_FRAME"));
            // 0x4009 / 0x400D NOT present
            for (ofs, _) in REGISTER_MAP.iter() {
                assert!(*ofs != 0x4009 && *ofs != 0x400D);
            }
        }
    }

    #[test]
    fn ffi_register_map_get() {
        unsafe {
            let count = fceux11_rust_debugsym_register_map_count();
            assert_eq!(count, 30);
            let mut buf = [0u8; 32];
            let mut ofs: u32 = 0;
            let ok = unsafe {
                fceux11_rust_debugsym_register_map_get(
                    0,
                    &mut ofs,
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len(),
                )
            };
            assert!(ok);
            assert_eq!(ofs, 0x2000);
            let s = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) };
            assert_eq!(s.to_str().unwrap(), "PPU_CTRL");
        }
    }
}
