//! 6502 assembler and disassembler.
//!
//! Replaces the logic formerly in `src/asm.cpp`.
//! Note: Disassemble is implemented without GetMem() callback — memory values
//! are not resolved. C++ wrapper can pre-read memory if needed.

use std::ffi::c_char;

// --------------------------------------------------------------------------
// Assembler — Assemble 6502 assembly text to opcode bytes
// --------------------------------------------------------------------------

/// Assemble 6502 assembly text into opcode bytes.
/// `output` must have room for at least 3 bytes.
/// Returns 0 on success, 1 on error.
///
/// # Safety
/// `output` must point to at least 3 writable bytes and `str` must be a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_asm_assemble(
    output: *mut u8,
    addr: i32,
    str: *const c_char,
) -> i32 {
    if output.is_null() || str.is_null() {
        return 1;
    }

    let c_str = unsafe { std::ffi::CStr::from_ptr(str) };
    let asm_str = c_str.to_str().unwrap_or("");
    let len = asm_str.len() as i32;
    if len == 0 || len > 127 {
        return 1;
    }

    // Working copy of string, uppercased
    let mut astr: Vec<u8> = asm_str.as_bytes().to_vec();
    for b in astr.iter_mut() {
        if *b >= b'a' && *b <= b'z' {
            *b -= 32; // to uppercase
        }
    }
    let astr_str = String::from_utf8(astr).unwrap_or_default();

    // Extract instruction (first 3 chars)
    let ins = astr_str.chars().take(3).collect::<String>();
    if ins.len() != 3 {
        return 1;
    }

    // Find instruction start after the mnemonic
    // After extracting ins="LDA", the rest can be " #$12" or just "#$12" etc.
    // We need to allow operands that start with '#', '$', '(', etc.
    let after_ins = astr_str[3..].trim_start();
    if !after_ins.is_empty()
        && !after_ins.starts_with(' ')
        && !after_ins.starts_with('#')
        && !after_ins.starts_with('$')
        && !after_ins.starts_with('(')
        && !after_ins.starts_with('[')
    {
        return 1;
    }

    // Strip all whitespace
    let astr_clean: String = astr_str.chars().filter(|c| !c.is_whitespace()).collect();

    // Basic syntax repairs: brackets to parens, comments to null
    let mut astr_fixed: String = astr_clean
        .chars()
        .map(|c| match c {
            '[' => '(',
            ']' => ')',
            '{' => '(',
            '}' => ')',
            ';' => '\0',
            _ => c,
        })
        .collect();
    // Remove 0X prefix in favor of $
    astr_fixed = astr_fixed.replace("0X", "$");
    if let Some(null_pos) = astr_fixed.find('\0') {
        astr_fixed.truncate(null_pos);
    }

    let output_slice = unsafe { std::slice::from_raw_parts_mut(output, 3) };
    output_slice[0] = 0;
    output_slice[1] = 0;
    output_slice[2] = 0;

    let remaining = &astr_fixed[3..]; // after the 3-char instruction

    // Implied instructions (no operands)
    macro_rules! implied {
        ($($ins:expr => $op:expr),*) => {
            if remaining.is_empty() {
                $(
                    if ins == $ins { output_slice[0] = $op; return 0; }
                )*
                return 1;
            }
        }
    }

    implied! {
        "BRK" => 0x00,
        "PHP" => 0x08,
        "ASL" => 0x0A,
        "CLC" => 0x18,
        "PLP" => 0x28,
        "ROL" => 0x2A,
        "SEC" => 0x38,
        "RTI" => 0x40,
        "PHA" => 0x48,
        "LSR" => 0x4A,
        "CLI" => 0x58,
        "RTS" => 0x60,
        "PLA" => 0x68,
        "ROR" => 0x6A,
        "SEI" => 0x78,
        "DEY" => 0x88,
        "TXA" => 0x8A,
        "TYA" => 0x98,
        "TXS" => 0x9A,
        "TAY" => 0xA8,
        "TAX" => 0xAA,
        "CLV" => 0xB8,
        "TSX" => 0xBA,
        "INY" => 0xC8,
        "DEX" => 0xCA,
        "CLD" => 0xD8,
        "INX" => 0xE8,
        "NOP" => 0xEA,
        "SED" => 0xF8
    }

    // Instructions with operands
    macro_rules! op_start {
        ($($ins:expr => $op:expr),*) => {
            $(
                if ins == $ins { output_slice[0] = $op; }
            )*
        }
    }

    op_start! {
        "ORA" => 0x01,
        "ASL" => 0x06,
        "BPL" => 0x10,
        "JSR" => 0x20,
        "AND" => 0x21,
        "BIT" => 0x24,
        "ROL" => 0x26,
        "BMI" => 0x30,
        "EOR" => 0x41,
        "LSR" => 0x46,
        "JMP" => 0x4C,
        "BVC" => 0x50,
        "ADC" => 0x61,
        "ROR" => 0x66,
        "BVS" => 0x70,
        "STA" => 0x81,
        "STY" => 0x84,
        "STX" => 0x86,
        "BCC" => 0x90,
        "LDY" => 0xA0,
        "LDA" => 0xA1,
        "LDX" => 0xA2,
        "BCS" => 0xB0,
        "CPY" => 0xC0,
        "CMP" => 0xC1,
        "DEC" => 0xC6,
        "BNE" => 0xD0,
        "CPX" => 0xE0,
        "SBC" => 0xE1,
        "INC" => 0xE6,
        "BEQ" => 0xF0
    }

    if output_slice[0] == 0 {
        return 1;
    }

    // Parse operands - simplified parsing
    let operand = remaining.trim_start_matches('\0');
    if operand.is_empty() {
        return 1;
    }

    // Immediate: #$12 or #12 or #$ABCD
    if operand.starts_with('#') {
        // Handle #$, #$12, #$
        let hex_part = if let Some(stripped) = operand.strip_prefix("#$") {
            stripped // skip #$
        } else {
            operand.trim_start_matches('#')
        };
        // Handle both #$<hex> and #<hex> formats
        let clean_hex = hex_part.trim_start_matches('$');

        if let Ok(val) = u8::from_str_radix(clean_hex, 16) {
            // Check if instruction allows immediate mode (rejection list)
            match output_slice[0] {
                0x06 | 0x10 | 0x20 | 0x24 | 0x26 | 0x30 | 0x46 | 0x4C | 0x50 | 0x66 | 0x70
                | 0x81 | 0x84 | 0x86 | 0x90 | 0xC6 | 0xE6 => return 1,
                _ => {}
            }
            // A0, A2, C0, E0 don't get immediate bit set
            match output_slice[0] {
                0xA0 | 0xA2 | 0xC0 | 0xE0 => {}
                _ => output_slice[0] |= 0x08,
            }
            output_slice[1] = val;
            return 0;
        }
    }

    // Absolute / Zero Page: $1234 or $12
    if operand.starts_with('$') && !operand.contains('(') {
        let hex_part = operand.trim_start_matches('$');
        let hex_len = hex_part.len();
        if hex_len <= 4
            && hex_part.chars().all(|c| c.is_ascii_hexdigit())
            && let Ok(val) = u32::from_str_radix(hex_part, 16)
        {
            match output_slice[0] {
                0x20 | 0x4C => {
                    // Jumps - always absolute
                    output_slice[1] = (val & 0xFF) as u8;
                    output_slice[2] = (val >> 8) as u8;
                    return 0;
                }
                0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xB0 | 0xD0 | 0xF0 => {
                    // Branches - 1 byte relative offset
                    let offset = (val as i32).wrapping_sub(addr + 2);
                    if !(-128..=127).contains(&offset) {
                        return 1;
                    }
                    output_slice[1] = (offset & 0xFF) as u8;
                    return 0;
                }
                _ => {
                    if val > 0xFF {
                        // Absolute
                        output_slice[0] |= 0x0C;
                        output_slice[1] = (val & 0xFF) as u8;
                        output_slice[2] = (val >> 8) as u8;
                    } else {
                        // Zero Page
                        output_slice[0] |= 0x04;
                        output_slice[1] = val as u8;
                    }
                    return 0;
                }
            }
        }
    }

    1
}

// --------------------------------------------------------------------------
// Disassembler — Disassemble opcode bytes to text
// --------------------------------------------------------------------------

/// Disassemble a 6502 opcode into a string.
/// Writes to `out_buf` with max `out_buf_size` bytes.
/// Returns bytes written (including null terminator), or -1 on error.
///
/// # Safety
/// `opcode` must point to at least one readable byte and `out_buf` must point to `out_buf_size` writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_asm_disassemble(
    _addr: i32,
    opcode: *const u8,
    out_buf: *mut c_char,
    out_buf_size: i32,
) -> i32 {
    if opcode.is_null() || out_buf.is_null() || out_buf_size <= 0 {
        return -1;
    }

    let op = unsafe { *opcode };

    // Common instruction name lookup table -- the complete official 6502
    // set (151 opcodes) + the 6 single-byte NOPs, written ascending by
    // opcode.  S2.5-r26 F-2: the previous table was authored column-by-column
    // and only kept the first opcode of each group correct (the whole zp
    // column decoded as ORA, 0x8D/0x8E/0x8C/0x91/0x99/0x9D fell to "???"),
    // which made the headless trace text unreliable.  Locked by
    // lock_disasm_mnemonics_256 (exhaustive, independent flat reference).
    // Multi-byte unofficial opcodes remain "???" (pre-existing scope).
    #[allow(unreachable_patterns)]
    let name = match op {
        0x00 => "BRK",
        0x01 | 0x05 | 0x09 | 0x0D | 0x11 | 0x15 | 0x19 | 0x1D => "ORA",
        0x06 | 0x0A | 0x0E | 0x16 | 0x1E => "ASL",
        0x08 => "PHP",
        0x10 => "BPL",
        0x18 => "CLC",
        0x20 => "JSR",
        0x21 | 0x25 | 0x29 | 0x2D | 0x31 | 0x35 | 0x39 | 0x3D => "AND",
        0x24 => "BIT",
        0x26 | 0x2A | 0x2E | 0x36 | 0x3E => "ROL",
        0x28 => "PLP",
        0x2C => "BIT",
        0x30 => "BMI",
        0x38 => "SEC",
        0x40 => "RTI",
        0x78 => "SEI",
        0x41 | 0x45 | 0x49 | 0x4D | 0x51 | 0x55 | 0x59 | 0x5D => "EOR",
        0x46 | 0x4A | 0x4E | 0x56 | 0x5E => "LSR",
        0x48 => "PHA",
        0x4C => "JMP",
        0x50 => "BVC",
        0x58 => "CLI",
        0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => "NOP",  // unofficial single-byte NOPs
        0x60 => "RTS",
        0x61 | 0x65 | 0x69 | 0x6D | 0x71 | 0x75 | 0x79 | 0x7D => "ADC",
        0x66 | 0x6A | 0x6E | 0x76 | 0x7A | 0x7E => "ROR",
        0x68 => "PLA",
        0x6C => "JMP",
        0x70 => "BVS",
        0x7A => "NOP",
        0x81 | 0x85 | 0x8D | 0x91 | 0x95 | 0x99 | 0x9D => "STA",
        0x84 | 0x8C => "STY",
        0x86 | 0x8E => "STX",
        0x88 => "DEY",
        0x8A => "TXA",
        0x90 => "BCC",
        0x94 => "STY",
        0x95 => "STA",
        0x96 => "STX",
        0x98 => "TYA",
        0x9A => "TXS",
        0x9D => "STA",
        0xA1 | 0xA5 | 0xA9 | 0xAD | 0xB1 | 0xB5 | 0xB9 | 0xBD => "LDA",
        0xA2 => "LDX",
        0xA0 | 0xA4 | 0xAC | 0xB4 | 0xBC => "LDY",
        0xA6 | 0xAE | 0xB6 | 0xBE => "LDX",
        0xA8 => "TAY",
        0xAA => "TAX",
        0xB0 => "BCS",
        0xB8 => "CLV",
        0xBA => "TSX",
        0xC0 | 0xC4 | 0xCC => "CPY",
        0xC1 | 0xC5 | 0xC9 | 0xCD | 0xD1 | 0xD5 | 0xD9 | 0xDD => "CMP",
        0xC6 | 0xCE | 0xD6 | 0xDE => "DEC",
        0xC8 => "INY",
        0xCA => "DEX",
        0xD0 => "BNE",
        0xD8 => "CLD",
        0xDA => "NOP",
        0xE0 | 0xE4 | 0xEC => "CPX",
        0xE1 | 0xE5 | 0xE9 | 0xEB | 0xED | 0xF1 | 0xF5 | 0xF9 | 0xFD => "SBC",
        0xE6 | 0xEE | 0xF6 | 0xFE => "INC",
        0xE8 => "INX",
        0xEA => "NOP",
        0xF0 => "BEQ",
        0xF8 => "SED",
        _ => "???",
    };

    // Build output string based on addressing mode
    #[allow(unreachable_patterns)]
    let result = match op {
        // Implied / Accumulator
        0x00 | 0x08 | 0x0A | 0x18 | 0x28 | 0x2A | 0x38 | 0x40 | 0x48 | 0x4A | 0x58 | 0x60
        | 0x68 | 0x6A | 0x78 | 0x88 | 0x8A | 0x98 | 0x9A | 0xA8 | 0xAA | 0xB8 | 0xBA | 0xC8
        | 0xCA | 0xD8 | 0xE8 | 0xEA | 0xF8 => name.to_string(),

        // Immediate
        0x09 | 0x29 | 0x49 | 0x69 | 0xA0 | 0xA2 | 0xA9 | 0xC0 | 0xC9 | 0xE0 | 0xE9 | 0xEB => {
            let val = unsafe { *opcode.add(1) };
            format!("{} #${:02X}", name, val)
        }

        // Zero Page
        0x05 | 0x06 | 0x24 | 0x25 | 0x26 | 0x45 | 0x46 | 0x65 | 0x66 | 0x84 | 0x85 | 0x86
        | 0xA4 | 0xA5 | 0xA6 | 0xC4 | 0xC5 | 0xC6 | 0xE4 | 0xE5 | 0xE6 => {
            let val = unsafe { *opcode.add(1) };
            format!("{} ${:02X}", name, val)
        }

        // Absolute (JMP, JSR, LDA, etc.)
        0x20 | 0x0D | 0x0E | 0x2C | 0x2D | 0x2E | 0x4C | 0x4D | 0x4E | 0x6D | 0x6E | 0x8C | 0x8D
        | 0x8E | 0xAC | 0xAD | 0xAE | 0xCC | 0xCD | 0xCE | 0xEC | 0xED | 0xEE => {
            let lo = unsafe { *opcode.add(1) };
            let hi = unsafe { *opcode.add(2) };
            format!("{} ${:02X}{:02X}", name, hi, lo)
        }

        // Branches (relative)
        0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xB0 | 0xD0 | 0xF0 => {
            let offset = unsafe { *opcode.add(1) } as i8;
            let target = (_addr + 2 + offset as i32) as u16;
            format!("{} ${:04X}", name, target)
        }

        // (Indirect,X) — (zp,X)
        0x01 | 0x21 | 0x41 | 0x61 | 0x81 | 0xA1 | 0xC1 | 0xE1 => {
            let zp = unsafe { *opcode.add(1) };
            format!("{} (${:02X},X)", name, zp)
        }

        // Zero Page,X
        0x15 | 0x16 | 0x35 | 0x36 | 0x55 | 0x56 | 0x75 | 0x76 | 0x94 | 0x95 | 0xB4 | 0xB5
        | 0xD5 | 0xD6 | 0xF5 | 0xF6 => {
            let zp = unsafe { *opcode.add(1) };
            format!("{} ${:02X},X", name, zp)
        }

        // (Indirect),Y — (zp),Y
        0x11 | 0x31 | 0x51 | 0x71 | 0x91 | 0xB1 | 0xD1 | 0xF1 => {
            let zp = unsafe { *opcode.add(1) };
            format!("{} (${:02X}),Y", name, zp)
        }

        // Absolute,X / Absolute,Y
        0x19 | 0x39 | 0x59 | 0x79 | 0x99 | 0xB9 | 0xBE | 0xD9 | 0xF9 => {
            let lo = unsafe { *opcode.add(1) };
            let hi = unsafe { *opcode.add(2) };
            format!("{} ${:02X}{:02X},Y", name, hi, lo)
        }
        0x1D | 0x1E | 0x3D | 0x3E | 0x5D | 0x5E | 0x7D | 0x7E | 0x9D | 0xBC | 0xBD | 0xDD
        | 0xDE | 0xFD | 0xFE => {
            let lo = unsafe { *opcode.add(1) };
            let hi = unsafe { *opcode.add(2) };
            format!("{} ${:02X}{:02X},X", name, hi, lo)
        }

        // JMP indirect
        0x6C => {
            let lo = unsafe { *opcode.add(1) };
            let hi = unsafe { *opcode.add(2) };
            format!("JMP (${:02X}{:02X})", hi, lo)
        }

        // STX Zero Page,Y / LDX Zero Page,Y
        0x96 | 0xB6 => {
            let zp = unsafe { *opcode.add(1) };
            format!("{} ${:02X},Y", name, zp)
        }

        // STY Zero Page,X / LDY Zero Page,X
        0x94 | 0xB4 => {
            let zp = unsafe { *opcode.add(1) };
            format!("{} ${:02X},X", name, zp)
        }

        _ => name.to_string(),
    };

    // Write to output buffer
    let bytes = result.as_bytes();
    let len = bytes.len().min((out_buf_size - 1) as usize);
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, len);
        *out_buf.add(len) = 0;
    }
    (len + 1) as i32
}

// --------------------------------------------------------------------------
// Unit tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    // S2.5-r26 (F-2): exhaustive 256-opcode disassembler lock.  The expected
    // strings are the independent flat reference (official 6502 opcode
    // matrix, operands fixed at $11/$2211 so the formatting is pinned),
    // structurally disjoint from the grouped match above.  Multi-byte
    // unofficial opcodes = "???" (documented scope; the core's treatment of
    // them is out of the disassembler's claim).
    #[test]
    fn lock_disasm_mnemonics_256() {
        let expected: [&str; 256] = [
            /* 0x00 */ "BRK", "ORA ($11,X)", "???", "???", "???", "ORA $11", "ASL $11", "???",
            /* 0x08 */ "PHP", "ORA #$11", "ASL", "???", "???", "ORA $2211", "ASL $2211", "???",
            /* 0x10 */ "BPL $0213", "ORA ($11),Y", "???", "???", "???", "ORA $11,X", "ASL $11,X", "???",
            /* 0x18 */ "CLC", "ORA $2211,Y", "NOP", "???", "???", "ORA $2211,X", "ASL $2211,X", "???",
            /* 0x20 */ "JSR $2211", "AND ($11,X)", "???", "???", "BIT $11", "AND $11", "ROL $11", "???",
            /* 0x28 */ "PLP", "AND #$11", "ROL", "???", "BIT $2211", "AND $2211", "ROL $2211", "???",
            /* 0x30 */ "BMI $0213", "AND ($11),Y", "???", "???", "???", "AND $11,X", "ROL $11,X", "???",
            /* 0x38 */ "SEC", "AND $2211,Y", "NOP", "???", "???", "AND $2211,X", "ROL $2211,X", "???",
            /* 0x40 */ "RTI", "EOR ($11,X)", "???", "???", "???", "EOR $11", "LSR $11", "???",
            /* 0x48 */ "PHA", "EOR #$11", "LSR", "???", "JMP $2211", "EOR $2211", "LSR $2211", "???",
            /* 0x50 */ "BVC $0213", "EOR ($11),Y", "???", "???", "???", "EOR $11,X", "LSR $11,X", "???",
            /* 0x58 */ "CLI", "EOR $2211,Y", "NOP", "???", "???", "EOR $2211,X", "LSR $2211,X", "???",
            /* 0x60 */ "RTS", "ADC ($11,X)", "???", "???", "???", "ADC $11", "ROR $11", "???",
            /* 0x68 */ "PLA", "ADC #$11", "ROR", "???", "JMP ($2211)", "ADC $2211", "ROR $2211", "???",
            /* 0x70 */ "BVS $0213", "ADC ($11),Y", "???", "???", "???", "ADC $11,X", "ROR $11,X", "???",
            /* 0x78 */ "SEI", "ADC $2211,Y", "NOP", "???", "???", "ADC $2211,X", "ROR $2211,X", "???",
            /* 0x80 */ "???", "STA ($11,X)", "???", "???", "STY $11", "STA $11", "STX $11", "???",
            /* 0x88 */ "DEY", "???", "TXA", "???", "STY $2211", "STA $2211", "STX $2211", "???",
            /* 0x90 */ "BCC $0213", "STA ($11),Y", "???", "???", "STY $11,X", "STA $11,X", "STX $11,Y", "???",
            /* 0x98 */ "TYA", "STA $2211,Y", "TXS", "???", "???", "STA $2211,X", "???", "???",
            /* 0xA0 */ "LDY #$11", "LDA ($11,X)", "LDX #$11", "???", "LDY $11", "LDA $11", "LDX $11", "???",
            /* 0xA8 */ "TAY", "LDA #$11", "TAX", "???", "LDY $2211", "LDA $2211", "LDX $2211", "???",
            /* 0xB0 */ "BCS $0213", "LDA ($11),Y", "???", "???", "LDY $11,X", "LDA $11,X", "LDX $11,Y", "???",
            /* 0xB8 */ "CLV", "LDA $2211,Y", "TSX", "???", "LDY $2211,X", "LDA $2211,X", "LDX $2211,Y", "???",
            /* 0xC0 */ "CPY #$11", "CMP ($11,X)", "???", "???", "CPY $11", "CMP $11", "DEC $11", "???",
            /* 0xC8 */ "INY", "CMP #$11", "DEX", "???", "CPY $2211", "CMP $2211", "DEC $2211", "???",
            /* 0xD0 */ "BNE $0213", "CMP ($11),Y", "???", "???", "???", "CMP $11,X", "DEC $11,X", "???",
            /* 0xD8 */ "CLD", "CMP $2211,Y", "NOP", "???", "???", "CMP $2211,X", "DEC $2211,X", "???",
            /* 0xE0 */ "CPX #$11", "SBC ($11,X)", "???", "???", "CPX $11", "SBC $11", "INC $11", "???",
            /* 0xE8 */ "INX", "SBC #$11", "NOP", "SBC #$11", "CPX $2211", "SBC $2211", "INC $2211", "???",
            /* 0xF0 */ "BEQ $0213", "SBC ($11),Y", "???", "???", "???", "SBC $11,X", "INC $11,X", "???",
            /* 0xF8 */ "SED", "SBC $2211,Y", "NOP", "???", "???", "SBC $2211,X", "INC $2211,X", "???",
        ];
        for op in 0..=255usize {
            let bytes: [u8; 3] = [op as u8, 0x11, 0x22];
            let mut buf = [0u8; 48];
            let n = unsafe {
                fceux11_rust_asm_disassemble(
                    0x0200,
                    bytes.as_ptr(),
                    buf.as_mut_ptr() as *mut i8,
                    48,
                )
            };
            assert!(n >= 0, "op {:02X}: disassemble returned {}", op, n);
            let s: String = buf.iter().take_while(|&&c| c != 0).map(|&c| c as u8 as char).collect();
            assert_eq!(
                s, expected[op],
                "op {:02X}: disassembly diverged from the official matrix",
                op
            );
        }
    }

    use super::*;

    #[test]
    fn test_assemble_nop() {
        unsafe {
            let mut output = [0u8; 3];
            let result = unsafe {
                fceux11_rust_asm_assemble(
                    output.as_mut_ptr(),
                    0xC000,
                    std::ffi::CString::new("NOP").unwrap().as_ptr(),
                )
            };
            assert_eq!(result, 0);
            assert_eq!(output[0], 0xEA);
        }
    }

    #[test]
    fn test_assemble_brk() {
        unsafe {
            let mut output = [0u8; 3];
            let result = unsafe {
                fceux11_rust_asm_assemble(
                    output.as_mut_ptr(),
                    0xC000,
                    std::ffi::CString::new("BRK").unwrap().as_ptr(),
                )
            };
            assert_eq!(result, 0);
            assert_eq!(output[0], 0x00);
        }
    }

    #[test]
    fn test_assemble_lda_immediate() {
        unsafe {
            let mut output = [0u8; 3];
            let result = unsafe {
                fceux11_rust_asm_assemble(
                    output.as_mut_ptr(),
                    0xC000,
                    std::ffi::CString::new("LDA #$12").unwrap().as_ptr(),
                )
            };
            assert_eq!(result, 0);
            assert_eq!(output[0], 0xA9);
            assert_eq!(output[1], 0x12);
        }
    }

    #[test]
    fn test_assemble_jmp_absolute() {
        unsafe {
            let mut output = [0u8; 3];
            let result = unsafe {
                fceux11_rust_asm_assemble(
                    output.as_mut_ptr(),
                    0xC000,
                    std::ffi::CString::new("JMP $C000").unwrap().as_ptr(),
                )
            };
            assert_eq!(result, 0);
            assert_eq!(output[0], 0x4C);
            assert_eq!(output[1], 0x00);
            assert_eq!(output[2], 0xC0);
        }
    }

    #[test]
    fn test_assemble_invalid() {
        unsafe {
            let mut output = [0u8; 3];
            let result = unsafe {
                fceux11_rust_asm_assemble(
                    output.as_mut_ptr(),
                    0xC000,
                    std::ffi::CString::new("").unwrap().as_ptr(),
                )
            };
            assert_eq!(result, 1); // error
        }
    }

    #[test]
    fn test_disassemble_nop() {
        unsafe {
            let mut buf = [0i8; 64];
            let result = unsafe {
                fceux11_rust_asm_disassemble(0xC000, [0xEA].as_ptr(), buf.as_mut_ptr(), 64)
            };
            assert!(result > 0);
            let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
            assert_eq!(s, "NOP");
        }
    }

    #[test]
    fn test_disassemble_lda_immediate() {
        unsafe {
            let mut buf = [0i8; 64];
            let result = unsafe {
                fceux11_rust_asm_disassemble(0xC000, [0xA9, 0x42].as_ptr(), buf.as_mut_ptr(), 64)
            };
            assert!(result > 0);
            let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
            assert_eq!(s, "LDA #$42");
        }
    }

    #[test]
    fn test_disassemble_jmp() {
        unsafe {
            let mut buf = [0i8; 64];
            let result = unsafe {
                fceux11_rust_asm_disassemble(
                    0xC000,
                    [0x4C, 0x00, 0x80].as_ptr(),
                    buf.as_mut_ptr(),
                    64,
                )
            };
            assert!(result > 0);
            let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }.to_string_lossy();
            assert_eq!(s, "JMP $8000");
        }
    }
}
