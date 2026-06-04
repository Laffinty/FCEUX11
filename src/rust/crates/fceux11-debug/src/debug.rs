//! Debug helpers — pure-computation extracts from `src/debug.cpp` plus the
//! `DebuggerState` ownership migration (v0.2.25).
//!
//! # Scope
//!
//! * **`evaluate_write` / `get_value`** — pure register lookups and opcode
//!   semantics; called from `evaluate()` in C++. No memory access here.
//! * **`log_cd_vectors` / `log_cd_data`** — Code/Data Logger bit twiddling.
//!   The `cdloggerdata` buffer is still C++ owned (allocated by
//!   `CodeDataLogger.cpp`); we operate on a caller-supplied slice. The three
//!   `volatile int` counters (`codecount`/`datacount`/`undefinedcount`) are
//!   also C++ globals, mutated via raw pointer.
//! * **`DebuggerState`** — six fields (step / stepout / runline /
//!   runline_end_time / badopbreak / jsrcount) now Rust-owned via lock-free
//!   atomics, exposed individually via FFI plus a bulk copy-out/copy-in pair.
//!
//! # Why atomics (not Mutex)?
//!
//! `DebugCycle()` reads up to four fields per 6502 instruction (CPU hot
//! path). Per-call `Mutex::lock` would add ~20-100 ns × millions of
//! instructions/s = visible regression. `AtomicBool::load(Relaxed)` compiles
//! to a single MOV on x86_64.

use std::ffi::c_char;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};

// --------------------------------------------------------------------------
// CPU P-register flag masks — mirror of `src/x6502.h:56-63`
// --------------------------------------------------------------------------

pub const N_FLAG: u8 = 0x80;
pub const V_FLAG: u8 = 0x40;
pub const U_FLAG: u8 = 0x20;
pub const B_FLAG: u8 = 0x10;
pub const D_FLAG: u8 = 0x08;
pub const I_FLAG: u8 = 0x04;
pub const Z_FLAG: u8 = 0x02;
pub const C_FLAG: u8 = 0x01;

// --------------------------------------------------------------------------
// Pure functions — evaluate_write & get_value
// --------------------------------------------------------------------------

/// Predict the byte that opcode `opcode` will write at `address`, given the
/// current register state. Mirrors `src/debug.cpp:376-400` exactly. Returns
/// `0` for non-writing opcodes (`opwrite[opcode] == 0`).
///
/// `mem_at_addr` is the current memory value at `address`, supplied by the
/// caller (Rust avoids the C++ `GetMem` callback for purity).
///
/// `opwrite_byte` is `opwrite[opcode]` from `src/x6502.cpp:635` — supplied
/// by the caller rather than duplicating the 256-entry table here.
pub fn evaluate_write(
    opwrite_byte: u8,
    address: u16,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    mem_at_addr: u8,
) -> u8 {
    match opwrite_byte {
        // 0 — no write (also covers the C++ `default` branch).
        0 => 0,
        // 1 — STA, PHA
        1 => a,
        // 2 — STX
        2 => x,
        // 3 — STY
        3 => y,
        // 4 — PHP
        4 => p,
        // 5 — ASL (SLO)
        5 => mem_at_addr.wrapping_shl(1),
        // 6 — LSR (SRE)
        6 => mem_at_addr >> 1,
        // 7 — ROL (RLA)
        7 => mem_at_addr.wrapping_shl(1) | (p & 1),
        // 8 — ROR (RRA) — note C++ comment says "ROL" but logic is ROR
        8 => (mem_at_addr >> 1) | ((p & 1) << 7),
        // 9 — INC (ISC)
        9 => mem_at_addr.wrapping_add(1),
        // 10 — DEC (DCP)
        10 => mem_at_addr.wrapping_sub(1),
        // 11 — SAX
        11 => a & x,
        // 12 — AHX — `_A & _X & (((address - _Y) >> 8) + 1)` truncated to u8
        12 => {
            let high = ((address.wrapping_sub(y as u16) >> 8) as u32).wrapping_add(1);
            (a as u32 & x as u32 & high) as u8
        }
        // 13 — SHY
        13 => {
            let high = ((address.wrapping_sub(x as u16) >> 8) as u32).wrapping_add(1);
            (y as u32 & high) as u8
        }
        // 14 — SHX
        14 => {
            let high = ((address.wrapping_sub(y as u16) >> 8) as u32).wrapping_add(1);
            (x as u32 & high) as u8
        }
        // 15 — TAS
        15 => {
            let high = ((address.wrapping_sub(y as u16) >> 8) as u32).wrapping_add(1);
            (s as u32 & high) as u8
        }
        // Any other value (defensive) — return 0.
        _ => 0,
    }
}

/// Return the current value of a register or P-flag, indexed by ASCII char
/// constant (see `src/debug.cpp:108-128`).
pub fn get_value(
    reg_or_flag: i32,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    pc: u16,
    s: u8,
) -> i32 {
    match reg_or_flag as u8 as char {
        'A' => a as i32,
        'X' => x as i32,
        'Y' => y as i32,
        'N' => i32::from((p & N_FLAG) != 0),
        'V' => i32::from((p & V_FLAG) != 0),
        'U' => i32::from((p & U_FLAG) != 0),
        'B' => i32::from((p & B_FLAG) != 0),
        'D' => i32::from((p & D_FLAG) != 0),
        'I' => i32::from((p & I_FLAG) != 0),
        'Z' => i32::from((p & Z_FLAG) != 0),
        'C' => i32::from((p & C_FLAG) != 0),
        'P' => pc as i32,
        'S' => s as i32,
        _ => 0,
    }
}

// --------------------------------------------------------------------------
// CDLogger bit logic
// --------------------------------------------------------------------------

/// `LogCDVectors` — record the two-byte vector at `prg_addr` as data.
/// Mirrors `src/debug.cpp:495-512`. Operates on the caller-provided buffer
/// and counter pointers (C++ keeps ownership of both).
///
/// `cdloggerdata` is a `&mut [u8]` view of the allocation; `prg_addr` is
/// the result of C++'s `GetPRGAddress(which)`. If `prg_addr < 0` or the
/// addresses exceed the buffer length, no operation is performed (matches
/// the C++ early-return on `j == -1`).
pub fn log_cd_vectors(
    cdloggerdata: &mut [u8],
    prg_addr: i32,
    codecount: &mut i32,
    datacount: &mut i32,
    undefinedcount: &mut i32,
) {
    if prg_addr < 0 {
        return;
    }
    let j = prg_addr as usize;
    // Two consecutive bytes (vector low + high).
    for offset in 0..2 {
        let idx = j + offset;
        if idx >= cdloggerdata.len() {
            return;
        }
        let byte = cdloggerdata[idx];
        if byte & 0x02 == 0 {
            cdloggerdata[idx] = byte | 0x0E;
            *datacount += 1;
            if cdloggerdata[idx] & 1 == 0 {
                *undefinedcount -= 1;
            }
        }
    }
    // codecount unchanged — only datacount/undefinedcount are touched.
    // (The parameter is kept for symmetry with `log_cd_data`.)
    let _ = codecount;
}

/// Result of one `log_cd_data` call. The C++ side uses these flags to
/// decide whether to trigger `BreakHit` and to update its file-static
/// `indirectnext`.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct FceuLogCdDataResult {
    pub new_code_hit: bool,
    pub new_data_hit: bool,
    pub indirect_out: bool,
}

/// `LogCDData` — record code/data hits for an executed instruction. Mirrors
/// `src/debug.cpp:517-587`. The C++ caller is responsible for the lookup of
/// PRG addresses via `GetPRGAddress` (which touches `Page[]`/`PRGptr[]`),
/// the `optype`/`opwrite` table reads, and the eventual `BreakHit` calls.
///
/// Arguments:
/// * `pc_prg_addr` — `GetPRGAddress(_PC)`; `-1` if unmapped.
/// * `a_prg_addr` — `GetPRGAddress(A)`; `-1` if unmapped.
/// * `optype_byte` — `optype[opcode[0]]` (`src/x6502.cpp:597`).
/// * `opwrite_byte` — `opwrite[opcode[0]]`.
/// * `indirect_in` — previous value of C++ `indirectnext` (file-static).
///
/// Returns the three flags the C++ caller acts on.
pub fn log_cd_data(
    cdloggerdata: &mut [u8],
    pc_prg_addr: i32,
    a_prg_addr: i32,
    pc: u16,
    a: u16,
    opcode0: u8,
    optype_byte: u8,
    opwrite_byte: u8,
    size: usize,
    indirect_in: bool,
    codecount: &mut i32,
    datacount: &mut i32,
    undefinedcount: &mut i32,
) -> FceuLogCdDataResult {
    let mut result = FceuLogCdDataResult::default();

    // 1. Mark instruction bytes as code (length = `size`).
    if pc_prg_addr >= 0 {
        let j = pc_prg_addr as usize;
        for i in 0..size {
            let idx = j + i;
            if idx >= cdloggerdata.len() {
                break;
            }
            if cdloggerdata[idx] & 1 != 0 {
                continue; // already logged
            }
            cdloggerdata[idx] |= 1;
            cdloggerdata[idx] |= (((pc as u32 + i as u32) >> 11) & 0x0C) as u8;
            // Bit 7 inverted high bit of PC's high byte. C++:
            //   |= ((_PC & 0x8000) >> 8) ^ 0x80
            cdloggerdata[idx] |= ((((pc as u32) & 0x8000) >> 8) ^ 0x80) as u8;
            if indirect_in {
                cdloggerdata[idx] |= 0x10;
            }
            *codecount += 1;
            if cdloggerdata[idx] & 2 == 0 {
                *undefinedcount -= 1;
            }
            result.new_code_hit = true;
        }
    }

    // 2. Determine outgoing `indirectnext` — true iff this is an indirect JMP (0x6C).
    result.indirect_out = opcode0 == 0x6C;

    // 3. Compute `memop` byte (0x20 only for optype 1 or 4).
    let memop: u8 = if optype_byte == 1 || optype_byte == 4 {
        0x20
    } else {
        0
    };

    // 4. Mark the data address (if mapped).
    if a_prg_addr >= 0 {
        let j = a_prg_addr as usize;
        if j < cdloggerdata.len() {
            if opwrite_byte == 0 {
                // Read — mark as data.
                if cdloggerdata[j] & 2 == 0 {
                    cdloggerdata[j] |= 2;
                    cdloggerdata[j] |= ((a >> 11) & 0x0C) as u8;
                    cdloggerdata[j] |= memop;
                    cdloggerdata[j] |= ((((a as u32) & 0x8000) >> 8) ^ 0x80) as u8;
                    *datacount += 1;
                    if cdloggerdata[j] & 1 == 0 {
                        *undefinedcount -= 1;
                    }
                    result.new_data_hit = true;
                }
            } else {
                // Write — clear the slot, adjusting counters.
                if cdloggerdata[j] & 1 != 0 {
                    *codecount -= 1;
                }
                if cdloggerdata[j] & 2 != 0 {
                    *datacount -= 1;
                }
                if cdloggerdata[j] & 3 != 0 {
                    *undefinedcount += 1;
                }
                cdloggerdata[j] = 0;
            }
        }
    }

    result
}

// --------------------------------------------------------------------------
// DebuggerState — atomics for zero-lock access from CPU hot path
// --------------------------------------------------------------------------

static DBG_STEP: AtomicBool = AtomicBool::new(false);
static DBG_STEPOUT: AtomicBool = AtomicBool::new(false);
static DBG_RUNLINE: AtomicBool = AtomicBool::new(false);
static DBG_RUNLINE_END_TIME: AtomicU64 = AtomicU64::new(0);
static DBG_BADOPBREAK: AtomicBool = AtomicBool::new(false);
static DBG_JSRCOUNT: AtomicI32 = AtomicI32::new(0);

const RELAXED: Ordering = Ordering::Relaxed;

/// POD bulk-transfer view of all six DebuggerState fields. Layout-compatible
/// with the C++ `FceuDebuggerStateView` struct.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FceuDebuggerStateView {
    pub step: bool,
    pub stepout: bool,
    pub runline: bool,
    pub runline_end_time: u64,
    pub badopbreak: bool,
    pub jsrcount: i32,
}

// --- Per-field accessors ---------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_get_step() -> bool {
    DBG_STEP.load(RELAXED)
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_set_step(v: bool) {
    DBG_STEP.store(v, RELAXED);
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_get_stepout() -> bool {
    DBG_STEPOUT.load(RELAXED)
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_set_stepout(v: bool) {
    DBG_STEPOUT.store(v, RELAXED);
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_get_runline() -> bool {
    DBG_RUNLINE.load(RELAXED)
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_set_runline(v: bool) {
    DBG_RUNLINE.store(v, RELAXED);
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_get_runline_end_time() -> u64 {
    DBG_RUNLINE_END_TIME.load(RELAXED)
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_set_runline_end_time(v: u64) {
    DBG_RUNLINE_END_TIME.store(v, RELAXED);
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_get_badopbreak() -> bool {
    DBG_BADOPBREAK.load(RELAXED)
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_set_badopbreak(v: bool) {
    DBG_BADOPBREAK.store(v, RELAXED);
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_get_jsrcount() -> i32 {
    DBG_JSRCOUNT.load(RELAXED)
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_set_jsrcount(v: i32) {
    DBG_JSRCOUNT.store(v, RELAXED);
}

/// Pre-increment `jsrcount`. Returns the new value.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_jsrcount_inc() -> i32 {
    // `fetch_add` returns the previous value; add 1 to get the new value.
    DBG_JSRCOUNT.fetch_add(1, RELAXED) + 1
}

/// Pre-decrement `jsrcount`. Returns the new value.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_jsrcount_dec() -> i32 {
    DBG_JSRCOUNT.fetch_sub(1, RELAXED) - 1
}

/// Reset to defaults — equivalent to C++ `DebuggerState::reset()`:
/// step=false, stepout=false, jsrcount=0. Note: the C++ reset does NOT
/// touch runline / runline_end_time / badopbreak (matches C++).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_reset() {
    DBG_STEP.store(false, RELAXED);
    DBG_STEPOUT.store(false, RELAXED);
    DBG_JSRCOUNT.store(0, RELAXED);
}

/// Snapshot all six fields into `out`. Useful for the GUI Step-Out block in
/// `ConsoleDebugger.cpp:3029-3055` which previously held a `DebuggerState&`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_copy_out(out: *mut FceuDebuggerStateView) {
    if out.is_null() {
        return;
    }
    unsafe {
        (*out).step = DBG_STEP.load(RELAXED);
        (*out).stepout = DBG_STEPOUT.load(RELAXED);
        (*out).runline = DBG_RUNLINE.load(RELAXED);
        (*out).runline_end_time = DBG_RUNLINE_END_TIME.load(RELAXED);
        (*out).badopbreak = DBG_BADOPBREAK.load(RELAXED);
        (*out).jsrcount = DBG_JSRCOUNT.load(RELAXED);
    }
}

/// Bulk-write all six fields.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_dbgstate_copy_in(in_: *const FceuDebuggerStateView) {
    if in_.is_null() {
        return;
    }
    let v = unsafe { *in_ };
    DBG_STEP.store(v.step, RELAXED);
    DBG_STEPOUT.store(v.stepout, RELAXED);
    DBG_RUNLINE.store(v.runline, RELAXED);
    DBG_RUNLINE_END_TIME.store(v.runline_end_time, RELAXED);
    DBG_BADOPBREAK.store(v.badopbreak, RELAXED);
    DBG_JSRCOUNT.store(v.jsrcount, RELAXED);
}

// --------------------------------------------------------------------------
// Pure-computation FFI wrappers
// --------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_evaluate_write(
    opwrite_byte: u8,
    address: u16,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    s: u8,
    mem_at_addr: u8,
) -> u8 {
    evaluate_write(opwrite_byte, address, a, x, y, p, s, mem_at_addr)
}

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_get_value(
    reg_or_flag: i32,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    pc: u16,
    s: u8,
) -> i32 {
    get_value(reg_or_flag, a, x, y, p, pc, s)
}

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_log_cd_vectors(
    cdloggerdata: *mut u8,
    cdloggerdata_size: usize,
    prg_addr: i32,
    codecount: *mut i32,
    datacount: *mut i32,
    undefinedcount: *mut i32,
) {
    if cdloggerdata.is_null()
        || cdloggerdata_size == 0
        || codecount.is_null()
        || datacount.is_null()
        || undefinedcount.is_null()
    {
        return;
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(cdloggerdata, cdloggerdata_size) };
    let cc = unsafe { &mut *codecount };
    let dc = unsafe { &mut *datacount };
    let uc = unsafe { &mut *undefinedcount };
    log_cd_vectors(buf, prg_addr, cc, dc, uc);
}

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_debug_log_cd_data(
    cdloggerdata: *mut u8,
    cdloggerdata_size: usize,
    pc_prg_addr: i32,
    a_prg_addr: i32,
    pc: u16,
    a: u16,
    opcode0: u8,
    optype_byte: u8,
    opwrite_byte: u8,
    size: usize,
    indirect_in: bool,
    codecount: *mut i32,
    datacount: *mut i32,
    undefinedcount: *mut i32,
) -> FceuLogCdDataResult {
    if cdloggerdata.is_null()
        || cdloggerdata_size == 0
        || codecount.is_null()
        || datacount.is_null()
        || undefinedcount.is_null()
    {
        return FceuLogCdDataResult::default();
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(cdloggerdata, cdloggerdata_size) };
    let cc = unsafe { &mut *codecount };
    let dc = unsafe { &mut *datacount };
    let uc = unsafe { &mut *undefinedcount };
    log_cd_data(
        buf,
        pc_prg_addr,
        a_prg_addr,
        pc,
        a,
        opcode0,
        optype_byte,
        opwrite_byte,
        size,
        indirect_in,
        cc,
        dc,
        uc,
    )
}

// Keep `c_char` import alive for symmetry with sibling modules.
#[allow(dead_code)]
fn _force_c_char_used(_p: *const c_char) {}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serialise DebuggerState tests since they share global atomics.
    static DBG_TEST_LOCK: Mutex<()> = Mutex::new(());

    // ---- evaluate_write -------------------------------------------------

    #[test]
    fn ew_no_write_opcodes_return_zero() {
        assert_eq!(evaluate_write(0, 0, 1, 2, 3, 4, 5, 6), 0);
    }

    #[test]
    fn ew_sta_returns_a() {
        assert_eq!(evaluate_write(1, 0, 0x42, 0, 0, 0, 0, 0), 0x42);
    }

    #[test]
    fn ew_stx_returns_x() {
        assert_eq!(evaluate_write(2, 0, 0, 0x55, 0, 0, 0, 0), 0x55);
    }

    #[test]
    fn ew_sty_returns_y() {
        assert_eq!(evaluate_write(3, 0, 0, 0, 0x66, 0, 0, 0), 0x66);
    }

    #[test]
    fn ew_php_returns_p() {
        assert_eq!(evaluate_write(4, 0, 0, 0, 0, 0xAB, 0, 0), 0xAB);
    }

    #[test]
    fn ew_asl_shifts_mem_left() {
        // ASL of 0x81 = 0x02 (high bit lost in u8 shift)
        assert_eq!(evaluate_write(5, 0, 0, 0, 0, 0, 0, 0x81), 0x02);
    }

    #[test]
    fn ew_lsr_shifts_mem_right() {
        assert_eq!(evaluate_write(6, 0, 0, 0, 0, 0, 0, 0x80), 0x40);
    }

    #[test]
    fn ew_rol_includes_carry() {
        // mem = 0x40, C set → (0x40 << 1) | 1 = 0x81
        assert_eq!(evaluate_write(7, 0, 0, 0, 0, C_FLAG, 0, 0x40), 0x81);
    }

    #[test]
    fn ew_ror_includes_carry() {
        // mem = 0x02, C set → (0x02 >> 1) | (1 << 7) = 0x81
        assert_eq!(evaluate_write(8, 0, 0, 0, 0, C_FLAG, 0, 0x02), 0x81);
    }

    #[test]
    fn ew_inc() {
        assert_eq!(evaluate_write(9, 0, 0, 0, 0, 0, 0, 0xFF), 0x00); // wrap
        assert_eq!(evaluate_write(9, 0, 0, 0, 0, 0, 0, 0x10), 0x11);
    }

    #[test]
    fn ew_dec() {
        assert_eq!(evaluate_write(10, 0, 0, 0, 0, 0, 0, 0x00), 0xFF); // wrap
    }

    #[test]
    fn ew_sax_and() {
        assert_eq!(evaluate_write(11, 0, 0xF0, 0x0F, 0, 0, 0, 0), 0x00);
        assert_eq!(evaluate_write(11, 0, 0xFF, 0x0F, 0, 0, 0, 0), 0x0F);
    }

    // ---- get_value ------------------------------------------------------

    #[test]
    fn gv_registers() {
        assert_eq!(get_value('A' as i32, 0x10, 0, 0, 0, 0, 0), 0x10);
        assert_eq!(get_value('X' as i32, 0, 0x20, 0, 0, 0, 0), 0x20);
        assert_eq!(get_value('Y' as i32, 0, 0, 0x30, 0, 0, 0), 0x30);
        assert_eq!(get_value('P' as i32, 0, 0, 0, 0, 0x1234, 0), 0x1234);
        assert_eq!(get_value('S' as i32, 0, 0, 0, 0, 0, 0xFD), 0xFD);
    }

    #[test]
    fn gv_flags() {
        let p = N_FLAG | Z_FLAG | C_FLAG;
        assert_eq!(get_value('N' as i32, 0, 0, 0, p, 0, 0), 1);
        assert_eq!(get_value('V' as i32, 0, 0, 0, p, 0, 0), 0);
        assert_eq!(get_value('U' as i32, 0, 0, 0, p, 0, 0), 0);
        assert_eq!(get_value('B' as i32, 0, 0, 0, p, 0, 0), 0);
        assert_eq!(get_value('D' as i32, 0, 0, 0, p, 0, 0), 0);
        assert_eq!(get_value('I' as i32, 0, 0, 0, p, 0, 0), 0);
        assert_eq!(get_value('Z' as i32, 0, 0, 0, p, 0, 0), 1);
        assert_eq!(get_value('C' as i32, 0, 0, 0, p, 0, 0), 1);
    }

    #[test]
    fn gv_unknown_returns_zero() {
        assert_eq!(get_value('?' as i32, 1, 2, 3, 4, 5, 6), 0);
    }

    // ---- log_cd_vectors -------------------------------------------------

    #[test]
    fn lcv_two_bytes_marked() {
        let mut buf = vec![0u8; 16];
        let mut cc = 0;
        let mut dc = 0;
        let mut uc = 2; // pretend both bytes were undefined
        log_cd_vectors(&mut buf, 4, &mut cc, &mut dc, &mut uc);
        assert_eq!(buf[4] & 0x0E, 0x0E);
        assert_eq!(buf[5] & 0x0E, 0x0E);
        assert_eq!(dc, 2);
        assert_eq!(uc, 0); // both bytes were undefined → now defined
    }

    #[test]
    fn lcv_already_data_skipped() {
        let mut buf = vec![0u8; 16];
        buf[4] = 0x02; // already data
        let mut cc = 0;
        let mut dc = 0;
        let mut uc = 1;
        log_cd_vectors(&mut buf, 4, &mut cc, &mut dc, &mut uc);
        // buf[4] not modified (already had data bit set); buf[5] gets marked
        assert_eq!(buf[4], 0x02);
        assert_eq!(buf[5] & 0x0E, 0x0E);
        assert_eq!(dc, 1);
    }

    #[test]
    fn lcv_negative_addr_no_op() {
        let mut buf = vec![0u8; 16];
        let (mut cc, mut dc, mut uc) = (0, 0, 0);
        log_cd_vectors(&mut buf, -1, &mut cc, &mut dc, &mut uc);
        assert_eq!(buf, vec![0u8; 16]);
        assert_eq!(dc, 0);
    }

    // ---- log_cd_data ----------------------------------------------------

    #[test]
    fn lcd_marks_new_code() {
        let mut buf = vec![0u8; 256];
        let mut cc = 0;
        let mut dc = 0;
        let mut uc = 3; // pretend 3 bytes were undefined
        let r = log_cd_data(
            &mut buf, 0x10, -1, 0x8000, 0, 0xEA, 0, 0, 3, false, &mut cc, &mut dc, &mut uc,
        );
        assert!(r.new_code_hit);
        assert!(!r.new_data_hit);
        assert!(!r.indirect_out);
        // All 3 bytes marked as code
        for i in 0x10..0x13 {
            assert!(buf[i] & 1 != 0);
        }
        assert_eq!(cc, 3);
        assert_eq!(uc, 0);
    }

    #[test]
    fn lcd_already_code_skipped() {
        let mut buf = vec![0u8; 256];
        buf[0x10] = 0x01; // already code
        let (mut cc, mut dc, mut uc) = (0, 0, 0);
        let r = log_cd_data(
            &mut buf, 0x10, -1, 0x8000, 0, 0xEA, 0, 0, 1, false, &mut cc, &mut dc, &mut uc,
        );
        assert!(!r.new_code_hit);
        assert_eq!(cc, 0);
    }

    #[test]
    fn lcd_indirect_jmp_returns_indirect_out() {
        let mut buf = vec![0u8; 256];
        let (mut cc, mut dc, mut uc) = (0, 0, 0);
        let r = log_cd_data(
            &mut buf, -1, -1, 0x8000, 0, 0x6C, 0, 0, 3, false, &mut cc, &mut dc, &mut uc,
        );
        assert!(r.indirect_out);
    }

    #[test]
    fn lcd_marks_data_for_read_opcode() {
        let mut buf = vec![0u8; 256];
        let (mut cc, mut dc, mut uc) = (0, 0, 0);
        // Read instruction (opwrite=0) targeting addr 0x50, mapped at j=0x50
        let r = log_cd_data(
            &mut buf, -1, 0x50, 0x8000, 0x50, 0xAD, 0, 0, 3, false, &mut cc, &mut dc, &mut uc,
        );
        assert!(r.new_data_hit);
        assert!(buf[0x50] & 2 != 0);
    }

    #[test]
    fn lcd_write_clears_slot() {
        let mut buf = vec![0u8; 256];
        buf[0x50] = 0x03; // both code and data set
        let mut cc = 1;
        let mut dc = 1;
        let mut uc = 0;
        // Write opcode (opwrite=1) at 0x50, no PC mapping
        let r = log_cd_data(
            &mut buf, -1, 0x50, 0x8000, 0x50, 0x8D, 0, 1, 3, false, &mut cc, &mut dc, &mut uc,
        );
        assert_eq!(buf[0x50], 0);
        assert_eq!(cc, 0);
        assert_eq!(dc, 0);
        assert_eq!(uc, 1);
        // new_data_hit is false on the write branch.
        assert!(!r.new_data_hit);
    }

    // ---- DebuggerState atomics & FFI ------------------------------------

    #[test]
    fn dbg_state_reset() {
        let _g = DBG_TEST_LOCK.lock().unwrap();
        fceux11_rust_debug_dbgstate_set_step(true);
        fceux11_rust_debug_dbgstate_set_stepout(true);
        fceux11_rust_debug_dbgstate_set_jsrcount(99);
        fceux11_rust_debug_dbgstate_reset();
        assert!(!fceux11_rust_debug_dbgstate_get_step());
        assert!(!fceux11_rust_debug_dbgstate_get_stepout());
        assert_eq!(fceux11_rust_debug_dbgstate_get_jsrcount(), 0);
    }

    #[test]
    fn dbg_state_jsr_inc_dec() {
        let _g = DBG_TEST_LOCK.lock().unwrap();
        fceux11_rust_debug_dbgstate_set_jsrcount(0);
        assert_eq!(fceux11_rust_debug_dbgstate_jsrcount_inc(), 1);
        assert_eq!(fceux11_rust_debug_dbgstate_jsrcount_inc(), 2);
        assert_eq!(fceux11_rust_debug_dbgstate_jsrcount_dec(), 1);
        assert_eq!(fceux11_rust_debug_dbgstate_get_jsrcount(), 1);
        // Reset for other tests
        fceux11_rust_debug_dbgstate_set_jsrcount(0);
    }

    #[test]
    fn dbg_state_copy_out_in_roundtrip() {
        let _g = DBG_TEST_LOCK.lock().unwrap();
        fceux11_rust_debug_dbgstate_set_step(true);
        fceux11_rust_debug_dbgstate_set_stepout(false);
        fceux11_rust_debug_dbgstate_set_runline(true);
        fceux11_rust_debug_dbgstate_set_runline_end_time(0xDEADBEEF_CAFEBABE);
        fceux11_rust_debug_dbgstate_set_badopbreak(true);
        fceux11_rust_debug_dbgstate_set_jsrcount(42);

        let mut view = FceuDebuggerStateView::default();
        fceux11_rust_debug_dbgstate_copy_out(&mut view);
        assert!(view.step);
        assert!(!view.stepout);
        assert!(view.runline);
        assert_eq!(view.runline_end_time, 0xDEADBEEF_CAFEBABE);
        assert!(view.badopbreak);
        assert_eq!(view.jsrcount, 42);

        // Modify view and write back
        view.step = false;
        view.jsrcount = -7;
        fceux11_rust_debug_dbgstate_copy_in(&view);
        assert!(!fceux11_rust_debug_dbgstate_get_step());
        assert_eq!(fceux11_rust_debug_dbgstate_get_jsrcount(), -7);

        // Restore defaults
        fceux11_rust_debug_dbgstate_reset();
        fceux11_rust_debug_dbgstate_set_runline(false);
        fceux11_rust_debug_dbgstate_set_runline_end_time(0);
        fceux11_rust_debug_dbgstate_set_badopbreak(false);
    }

    #[test]
    fn ffi_evaluate_write_matches_logic() {
        assert_eq!(
            fceux11_rust_debug_evaluate_write(1, 0, 0xAA, 0, 0, 0, 0, 0),
            0xAA
        );
    }

    #[test]
    fn ffi_get_value_matches_logic() {
        assert_eq!(fceux11_rust_debug_get_value('A' as i32, 0xAA, 0, 0, 0, 0, 0), 0xAA);
    }

    #[test]
    fn ffi_log_cd_data_null_safe() {
        // Null buffer / null counters → no-op default result.
        let r = fceux11_rust_debug_log_cd_data(
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            false,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert!(!r.new_code_hit && !r.new_data_hit && !r.indirect_out);
    }
}
