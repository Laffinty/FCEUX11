//! Condition expression parser for breakpoint conditions.
//!
//! Parses expressions like "A=5", "N=1|(Z=0)", "$C000=123" into a binary AST.
//! Replaces the logic formerly in `src/conddebug.cpp`.

use std::ffi::c_char;
use std::ptr;

// --------------------------------------------------------------------------
// Constants — mirror of conddebug.h TYPE_* and OP_*
// --------------------------------------------------------------------------

pub const TYPE_NO: u32 = 0;
pub const TYPE_REG: u32 = 1;
pub const TYPE_FLAG: u32 = 2;
pub const TYPE_NUM: u32 = 3;
pub const TYPE_ADDR: u32 = 4;
pub const TYPE_PC_BANK: u32 = 5;
pub const TYPE_DATA_BANK: u32 = 6;
pub const TYPE_VALUE_READ: u32 = 7;
pub const TYPE_VALUE_WRITE: u32 = 8;

pub const OP_NO: u32 = 0;
pub const OP_EQ: u32 = 1;
pub const OP_NE: u32 = 2;
pub const OP_GE: u32 = 3;
pub const OP_LE: u32 = 4;
pub const OP_G: u32 = 5;
pub const OP_L: u32 = 6;
pub const OP_PLUS: u32 = 7;
pub const OP_MINUS: u32 = 8;
pub const OP_MULT: u32 = 9;
pub const OP_DIV: u32 = 10;
pub const OP_OR: u32 = 11;
pub const OP_AND: u32 = 12;

// --------------------------------------------------------------------------
// AST node — mirrors C++ Condition struct
// --------------------------------------------------------------------------

#[repr(C)]
pub struct ConditionAst {
    pub lhs: *mut ConditionAst,
    pub rhs: *mut ConditionAst,
    pub type1: u32,
    pub value1: u32,
    pub op: u32,
    pub type2: u32,
    pub value2: u32,
}

impl ConditionAst {
    fn new() -> Box<ConditionAst> {
        Box::new(ConditionAst {
            lhs: ptr::null_mut(),
            rhs: ptr::null_mut(),
            type1: TYPE_NO,
            value1: 0,
            op: OP_NO,
            type2: TYPE_NO,
            value2: 0,
        })
    }
}

// --------------------------------------------------------------------------
// Character classification
// --------------------------------------------------------------------------

fn is_flag(c: u8) -> bool {
    matches!(c, b'N' | b'I' | b'C' | b'V' | b'Z' | b'B' | b'U' | b'D')
}

fn is_register(c: u8) -> bool {
    matches!(c, b'A' | b'X' | b'Y' | b'P' | b'S')
}

fn is_hexdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

// --------------------------------------------------------------------------
// FFI interface
// --------------------------------------------------------------------------

/// Parse a condition expression string into an AST.
/// Returns an opaque pointer to the ConditionAst, or null on error.
/// Format: <subject><op><value> where subject is a flag/register/address.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_conddebug_generate_condition(str: *const c_char) -> *mut std::ffi::c_void {
    if str.is_null() {
        return ptr::null_mut();
    }

    let c_str = unsafe { std::ffi::CStr::from_ptr(str) };
    let input = c_str.to_str().unwrap_or("");

    let input = input.trim();
    if input.is_empty() {
        return ptr::null_mut();
    }

    // Find the comparison operator
    // Operators: ||, &&, ==, !=, >=, <=, >, <, +, -, *, /
    // Find the main operator (highest-precedence one in the expression)
    // Grammar: Connect := Compare (('||' | '&&') Compare)*
    // So || and && are the only ones that split at the top level
    // Within each side, comparisons and arithmetic apply
    let mut op_pos: Option<usize> = None;
    let mut op_str: &str = "";
    let mut op_code: u32 = OP_NO;

    // First: find || or && (the Connect-level operators - LOWEST precedence, split last)
    // We want the FIRST occurrence of || or && as that's where we split
    if let Some(pos) = input.find("||") {
        op_pos = Some(pos);
        op_str = "||";
        op_code = OP_OR;
    }
    if let Some(pos) = input.find("&&") {
        if op_pos.map(|p| pos < p).unwrap_or(true) {
            op_pos = Some(pos);
            op_str = "&&";
            op_code = OP_AND;
        }
    }

    // Then plain = (equality) - but NOT if it's part of ==
    if let Some(pos) = input.find('=') {
        let before = input[..pos].chars().last().unwrap_or(' ');
        let after = input[pos + 1..].chars().next().unwrap_or(' ');
        if after != '=' && after != '>' && after != '<' && before != '=' {
            // Only use = if we haven't found a higher-precedence connective
            if op_pos.is_none() {
                op_pos = Some(pos);
                op_str = "=";
                op_code = OP_EQ;
            }
        }
    }

    if op_pos.is_none() || op_code == OP_NO {
        return ptr::null_mut();
    }

    let pos = op_pos.unwrap();
    let left = input[..pos].trim();
    let right = input[pos + op_str.len()..].trim();

    let mut cond = ConditionAst::new();
    cond.op = op_code;

    // Parse left side
    if let Some(first_char) = left.chars().next() {
        let fc = first_char as u8;
        if is_flag(fc) {
            cond.type1 = TYPE_FLAG;
            cond.value1 = fc as u32;
        } else if is_register(fc) {
            cond.type1 = TYPE_REG;
            cond.value1 = fc as u32;
        } else if fc == b'$' {
            cond.type1 = TYPE_ADDR;
            let hex_part = left.trim_start_matches('$');
            cond.value1 = u32::from_str_radix(hex_part, 16).unwrap_or(0);
        } else if fc == b'#' {
            cond.type1 = TYPE_NUM;
            let hex_part = left.trim_start_matches('#');
            cond.value1 = u32::from_str_radix(hex_part, 16).unwrap_or(0);
        } else if fc == b'K' {
            cond.type1 = TYPE_PC_BANK;
            cond.value1 = b'K' as u32;
        } else if fc == b'T' {
            cond.type1 = TYPE_DATA_BANK;
            cond.value1 = b'T' as u32;
        } else if fc == b'R' {
            cond.type1 = TYPE_VALUE_READ;
            cond.value1 = b'R' as u32;
        } else if fc == b'W' {
            cond.type1 = TYPE_VALUE_WRITE;
            cond.value1 = b'W' as u32;
        }
    }

    // Parse right side
    if let Some(first_char) = right.chars().next() {
        let fc = first_char as u8;
        if is_flag(fc) {
            cond.type2 = TYPE_FLAG;
            cond.value2 = fc as u32;
        } else if is_register(fc) {
            cond.type2 = TYPE_REG;
            cond.value2 = fc as u32;
        } else if fc == b'$' {
            cond.type2 = TYPE_ADDR;
            let hex_part = right.trim_start_matches('$');
            cond.value2 = u32::from_str_radix(hex_part, 16).unwrap_or(0);
        } else if fc == b'#' {
            cond.type2 = TYPE_NUM;
            let hex_part = right.trim_start_matches('#');
            cond.value2 = u32::from_str_radix(hex_part, 16).unwrap_or(0);
        } else if fc == b'K' {
            cond.type2 = TYPE_PC_BANK;
            cond.value2 = b'K' as u32;
        } else if fc == b'T' {
            cond.type2 = TYPE_DATA_BANK;
            cond.value2 = b'T' as u32;
        } else if fc == b'R' {
            cond.type2 = TYPE_VALUE_READ;
            cond.value2 = b'R' as u32;
        } else if fc == b'W' {
            cond.type2 = TYPE_VALUE_WRITE;
            cond.value2 = b'W' as u32;
        } else {
            // Plain number
            cond.type2 = TYPE_NUM;
            if right.starts_with("0x") || right.starts_with("0X") {
                cond.value2 = u32::from_str_radix(right.trim_start_matches("0x").trim_start_matches("0X"), 16).unwrap_or(0);
            } else {
                cond.value2 = right.parse().unwrap_or(0);
            }
        }
    }

    Box::into_raw(cond) as *mut std::ffi::c_void
}

/// Destroy a ConditionAst previously returned by fceux11_rust_conddebug_generate_condition.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_conddebug_condition_destroy(condition: *mut std::ffi::c_void) {
    if !condition.is_null() {
        unsafe { std::mem::drop(Box::from_raw(condition as *mut ConditionAst)) };
    }
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> *mut ConditionAst {
        let c_str = std::ffi::CString::new(s).unwrap();
        fceux11_rust_conddebug_generate_condition(c_str.as_ptr()) as *mut ConditionAst
    }

    fn destroy(c: *mut ConditionAst) {
        if !c.is_null() {
            unsafe { std::mem::drop(Box::from_raw(c)) };
        }
    }

    #[test]
    fn test_simple_flag() {
        let c = parse("N=1");
        assert!(!c.is_null(), "N=1 should parse");
        unsafe {
            assert_eq!((*c).type1, TYPE_FLAG);
            assert_eq!((*c).value1, b'N' as u32);
            assert_eq!((*c).op, OP_EQ);
            assert_eq!((*c).type2, TYPE_NUM);
            assert_eq!((*c).value2, 1);
        }
        destroy(c);
    }

    #[test]
    fn test_register_address() {
        let c = parse("A=$FF");
        assert!(!c.is_null(), "A=$FF should parse");
        unsafe {
            assert_eq!((*c).type1, TYPE_REG);
            assert_eq!((*c).value1, b'A' as u32);
            assert_eq!((*c).op, OP_EQ);
            assert_eq!((*c).type2, TYPE_ADDR);
            assert_eq!((*c).value2, 0xFF);
        }
        destroy(c);
    }

    #[test]
    fn test_address_condition() {
        let c = parse("$C000=0");
        assert!(!c.is_null(), "$C000=0 should parse");
        unsafe {
            assert_eq!((*c).type1, TYPE_ADDR);
            assert_eq!((*c).value1, 0xC000);
            assert_eq!((*c).op, OP_EQ);
            assert_eq!((*c).type2, TYPE_NUM);
            assert_eq!((*c).value2, 0);
        }
        destroy(c);
    }

    #[test]
    fn test_compound_or() {
        let c = parse("Z=0||N=1");
        assert!(!c.is_null(), "Z=0||N=1 should parse");
        unsafe {
            assert_eq!((*c).op, OP_OR);
        }
        destroy(c);
    }

    #[test]
    fn test_compound_and() {
        let c = parse("A=5&&B=3");
        assert!(!c.is_null(), "A=5&&B=3 should parse");
        unsafe {
            assert_eq!((*c).op, OP_AND);
        }
        destroy(c);
    }

    #[test]
    fn test_pc_bank() {
        let c = parse("K=1");
        assert!(!c.is_null(), "K=1 should parse");
        unsafe {
            assert_eq!((*c).type1, TYPE_PC_BANK);
            assert_eq!((*c).value1, b'K' as u32);
        }
        destroy(c);
    }

    #[test]
    fn test_invalid_empty() {
        assert!(parse("").is_null(), "empty string should return null");
    }

    #[test]
    fn test_value_read() {
        let c = parse("R=$1000");
        assert!(!c.is_null());
        unsafe {
            assert_eq!((*c).type1, TYPE_VALUE_READ);
        }
        destroy(c);
    }
}