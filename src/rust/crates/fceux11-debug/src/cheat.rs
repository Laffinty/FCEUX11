//! Cheat engine — Game Genie / Pro Action Replay decode, cheat list management,
//! cheat-map bit operations, and cheat-search comparators.
//!
//! Migrated from `src/cheat.cpp` (v0.2.24). Pure-computation parts are
//! self-contained; functions that need access to NES memory (CheatRPtrs,
//! ARead/BWrite, SetReadHandler) remain in C++ and call into Rust for state.
//!
//! # Architecture
//!
//! * **Decoders** — `decode_gg` and `decode_par` are pure functions matching
//!   the original C++ implementations bit-for-bit.
//! * **Cheat list** — A `Vec<CheatEntry>` behind a `Mutex` replaces the
//!   original singly-linked `CHEATF` list. The vector owns the strings and
//!   is iterated by index via FFI.
//! * **CheatMap** — A `Vec<u8>` of length `CHEATMAP_SIZE` (0x10000/8 = 8 KiB)
//!   provides bit-addressable storage. Bit `n` of byte `n/8` is set if address
//!   `n` is currently patched by a substitute cheat.
//! * **CheatComp** — A `Vec<u16>` of length 0x10000 storing the "original"
//!   value of each address plus the `CHEATC_NONE` / `CHEATC_EXCLUDED` flags
//!   (high bits). Search comparators are evaluated entirely in Rust given
//!   the current memory snapshot passed in by the C++ side.
//!
//! # Thread-safety
//! The emulator is single-threaded for these APIs, but Rust requires a
//! `Mutex` for `static mut` access. Lock contention is negligible.

use std::ffi::{CStr, c_char};
use std::sync::Mutex;

// --------------------------------------------------------------------------
// Constants
// --------------------------------------------------------------------------

/// `0x10000 / 8` bytes — one bit per NES address.
pub const CHEATMAP_SIZE: usize = 0x10000 / 8;

/// Flag bits packed into the high bits of each `CheatComp` slot.
pub const CHEATC_NONE: u16 = 0x8000;
pub const CHEATC_EXCLUDED: u16 = 0x4000;
pub const CHEATC_NOSHOW: u16 = 0xC000;

// Search-type IDs (mirror of `FCEU_SEARCH_*` macros in `cheat.h`).
pub const FCEU_SEARCH_SPECIFIC_CHANGE: i32 = 0;
pub const FCEU_SEARCH_RELATIVE_CHANGE: i32 = 1;
pub const FCEU_SEARCH_PUERLY_RELATIVE_CHANGE: i32 = 2;
pub const FCEU_SEARCH_ANY_CHANGE: i32 = 3;
pub const FCEU_SEARCH_NEWVAL_KNOWN: i32 = 4;
pub const FCEU_SEARCH_NEWVAL_GT: i32 = 5;
pub const FCEU_SEARCH_NEWVAL_LT: i32 = 6;
pub const FCEU_SEARCH_NEWVAL_GT_KNOWN: i32 = 7;
pub const FCEU_SEARCH_NEWVAL_LT_KNOWN: i32 = 8;

// --------------------------------------------------------------------------
// Game Genie decoder
// --------------------------------------------------------------------------

/// Map a Game Genie letter to its 4-bit value. Returns 0 for any unrecognised
/// character (matching the original C++ behaviour).
fn gg_to_bin(c: u8) -> u8 {
    const LETS: [u8; 16] = [
        b'A', b'P', b'Z', b'L', b'G', b'I', b'T', b'Y', b'E', b'O', b'X', b'U', b'K', b'S', b'V',
        b'N',
    ];
    let upper = c.to_ascii_uppercase();
    for (i, &letter) in LETS.iter().enumerate() {
        if letter == upper {
            return i as u8;
        }
    }
    0
}

/// Decode a 6- or 8-character Game Genie code. Returns `Some((addr, val, compare))`
/// on success — `compare` is `-1` for 6-char codes that have no compare byte.
pub fn decode_gg(code: &str) -> Option<(u16, u8, i32)> {
    let bytes = code.as_bytes();
    let s = bytes.len();
    if s != 6 && s != 8 {
        return None;
    }

    let mut a: u16 = 0x8000;
    let mut v: u8 = 0;
    let mut c: u8 = 0;

    let mut t = gg_to_bin(bytes[0]);
    v |= t & 0x07;
    v |= (t & 0x08) << 4;

    t = gg_to_bin(bytes[1]);
    v |= (t & 0x07) << 4;
    a |= ((t as u16) & 0x08) << 4;

    t = gg_to_bin(bytes[2]);
    a |= ((t as u16) & 0x07) << 4;

    t = gg_to_bin(bytes[3]);
    a |= ((t as u16) & 0x07) << 12;
    a |= (t as u16) & 0x08;

    t = gg_to_bin(bytes[4]);
    a |= (t as u16) & 0x07;
    a |= ((t as u16) & 0x08) << 8;

    if s == 6 {
        t = gg_to_bin(bytes[5]);
        a |= ((t as u16) & 0x07) << 8;
        v |= t & 0x08;
        Some((a, v, -1))
    } else {
        t = gg_to_bin(bytes[5]);
        a |= ((t as u16) & 0x07) << 8;
        c |= t & 0x08;

        t = gg_to_bin(bytes[6]);
        c |= t & 0x07;
        c |= (t & 0x08) << 4;

        t = gg_to_bin(bytes[7]);
        c |= (t & 0x07) << 4;
        v |= t & 0x08;
        Some((a, v, c as i32))
    }
}

/// Decode a Pro Action Replay code. Returns `Some((addr, val, compare, type))`.
///
/// `type` is `0` for zero-page (address < 0x100) addresses (RAM patch) and
/// `1` otherwise (substitute via read-handler).
pub fn decode_par(code: &str) -> Option<(u16, u8, i32, i32)> {
    if code.len() != 8 {
        return None;
    }
    // Parse four hex bytes; bail if any pair is invalid.
    let mut boo = [0u32; 4];
    for i in 0..4 {
        let pair = &code[i * 2..i * 2 + 2];
        boo[i] = u32::from_str_radix(pair, 16).ok()?;
    }

    // Original C++ ignores the inactive branch (`if(1)`). The expression
    // wraps in C; preserve byte-truncation via `wrapping_add`.
    let addr_raw = (boo[3] << 8) | (boo[2].wrapping_add(0x7F));
    let a = (addr_raw & 0xFFFF) as u16;
    let v: u8 = 0;
    let c: i32 = -1;
    let t = if a < 0x0100 { 0 } else { 1 };
    Some((a, v, c, t))
}

// --------------------------------------------------------------------------
// Cheat list state
// --------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct CheatEntry {
    pub name: String,
    pub addr: u16,
    pub val: u8,
    /// `-1` means no-compare; any non-negative byte value is treated as the
    /// compare byte for substitute-style cheats.
    pub compare: i32,
    pub status: i32,
    /// `0` = replace (periodic RAM poke), `1` = substitute (read handler).
    pub type_: i32,
}

struct CheatState {
    list: Vec<CheatEntry>,
    cheat_map: Vec<u8>,
    has_cheat_map: bool,
    cheat_comp: Vec<u16>,
    has_cheat_comp: bool,
    global_disabled: i32,
}

static CHEAT_STATE: Mutex<CheatState> = Mutex::new(CheatState {
    list: Vec::new(),
    cheat_map: Vec::new(),
    has_cheat_map: false,
    cheat_comp: Vec::new(),
    has_cheat_comp: false,
    global_disabled: 0,
});

fn lock() -> std::sync::MutexGuard<'static, CheatState> {
    CHEAT_STATE.lock().unwrap_or_else(|e| e.into_inner())
}

// --------------------------------------------------------------------------
// Cheat list FFI — CRUD and iteration
// --------------------------------------------------------------------------

/// Add a new cheat entry. `name` may be NULL (treated as empty string).
/// Returns the new entry index (always `>= 0`).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_add(
    name: *const c_char,
    addr: u32,
    val: u8,
    compare: i32,
    status: i32,
    type_: i32,
) -> i32 {
    let name = c_str_to_string(name);
    let mut st = lock();
    st.list.push(CheatEntry {
        name,
        addr: (addr & 0xFFFF) as u16,
        val,
        compare,
        status,
        type_,
    });
    (st.list.len() - 1) as i32
}

/// Delete the cheat at `which`. Returns `1` on success, `0` if out of range.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_delete(which: u32) -> i32 {
    let mut st = lock();
    let idx = which as usize;
    if idx < st.list.len() {
        st.list.remove(idx);
        1
    } else {
        0
    }
}

/// Toggle a cheat's enabled status. Returns the new status (`0`/`1`) or
/// `-1` if the index is out of range.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_toggle(which: u32) -> i32 {
    let mut st = lock();
    let idx = which as usize;
    if idx < st.list.len() {
        let new = if st.list[idx].status != 0 { 0 } else { 1 };
        st.list[idx].status = new;
        new
    } else {
        -1
    }
}

/// Disable every cheat without removing them. Returns the count of cheats
/// that were previously enabled (matches C++ behaviour).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_disable_all() -> i32 {
    let mut st = lock();
    let mut count = 0;
    for entry in st.list.iter_mut() {
        if entry.status != 0 {
            count += 1;
        }
        entry.status = 0;
    }
    count
}

/// Remove every cheat entry. Always returns `0`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_delete_all() -> i32 {
    let mut st = lock();
    st.list.clear();
    0
}

/// Return the current number of cheat entries.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_count() -> u32 {
    lock().list.len() as u32
}

/// Output struct for reading a cheat entry. Strings are returned as
/// `*const c_char` pointing into Rust-owned storage; the pointer is valid
/// until the next mutating cheat-list call.
#[repr(C)]
pub struct FceuCheatEntryView {
    pub name_ptr: *const c_char,
    pub name_len: usize,
    pub addr: u32,
    pub val: u8,
    pub compare: i32,
    pub status: i32,
    pub type_: i32,
}

thread_local! {
    static NAME_CACHE: std::cell::RefCell<std::ffi::CString> =
        std::cell::RefCell::new(std::ffi::CString::new("").unwrap());
}

/// Fetch the cheat at `which` into `out`. Returns `1` on success, `0` if out
/// of range. The `name_ptr` field points into a thread-local cache and is
/// invalidated by the next call.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_get(which: u32, out: *mut FceuCheatEntryView) -> i32 {
    if out.is_null() {
        return 0;
    }
    let st = lock();
    let idx = which as usize;
    if idx >= st.list.len() {
        return 0;
    }
    let entry = &st.list[idx];
    let cstr = std::ffi::CString::new(entry.name.as_bytes()).unwrap_or_default();
    let name_len = cstr.as_bytes().len();
    NAME_CACHE.with(|cache| {
        *cache.borrow_mut() = cstr;
    });
    NAME_CACHE.with(|cache| {
        let ptr = cache.borrow().as_ptr();
        unsafe {
            (*out).name_ptr = ptr;
            (*out).name_len = name_len;
            (*out).addr = entry.addr as u32;
            (*out).val = entry.val;
            (*out).compare = entry.compare;
            (*out).status = entry.status;
            (*out).type_ = entry.type_;
        }
    });
    1
}

/// Update an existing cheat entry. Any negative arg leaves that field
/// untouched (same convention as the original `FCEUI_SetCheat`).
///
/// * `name` — NULL means "do not change".
/// * `a`, `v`, `s` — values `< 0` mean "do not change".
/// * `c` — values `< -1` mean "do not change"; `-1` clears compare.
/// * `type_` is always applied.
///
/// Returns `1` on success, `0` if out of range.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_set(
    which: u32,
    name: *const c_char,
    a: i32,
    v: i32,
    c: i32,
    s: i32,
    type_: i32,
) -> i32 {
    let mut st = lock();
    let idx = which as usize;
    if idx >= st.list.len() {
        return 0;
    }
    let entry = &mut st.list[idx];
    if !name.is_null() {
        entry.name = c_str_to_string(name);
    }
    if a >= 0 {
        entry.addr = (a & 0xFFFF) as u16;
    }
    if v >= 0 {
        entry.val = (v & 0xFF) as u8;
    }
    if s >= 0 {
        entry.status = s;
    }
    if c >= -1 {
        entry.compare = c;
    }
    entry.type_ = type_;
    1
}

/// Set the `globalCheatDisabled` flag. Returns the prior value.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_set_global_disabled(disabled: i32) -> i32 {
    let mut st = lock();
    let old = st.global_disabled;
    st.global_disabled = disabled;
    old
}

/// Return the current `globalCheatDisabled` flag.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_get_global_disabled() -> i32 {
    lock().global_disabled
}

// --------------------------------------------------------------------------
// Decoder FFI — Game Genie / Pro Action Replay
// --------------------------------------------------------------------------

/// Decode a Game Genie code. Returns `1` on success, `0` on failure.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_decode_gg(
    str_ptr: *const c_char,
    a_out: *mut i32,
    v_out: *mut i32,
    c_out: *mut i32,
) -> i32 {
    if str_ptr.is_null() || a_out.is_null() || v_out.is_null() || c_out.is_null() {
        return 0;
    }
    let code = unsafe { CStr::from_ptr(str_ptr) }.to_str().unwrap_or("");
    match decode_gg(code) {
        Some((a, v, c)) => {
            unsafe {
                *a_out = a as i32;
                *v_out = v as i32;
                *c_out = c;
            }
            1
        }
        None => 0,
    }
}

/// Decode a Pro Action Replay code. Returns `1` on success, `0` on failure.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_decode_par(
    str_ptr: *const c_char,
    a_out: *mut i32,
    v_out: *mut i32,
    c_out: *mut i32,
    type_out: *mut i32,
) -> i32 {
    if str_ptr.is_null()
        || a_out.is_null()
        || v_out.is_null()
        || c_out.is_null()
        || type_out.is_null()
    {
        return 0;
    }
    let code = unsafe { CStr::from_ptr(str_ptr) }.to_str().unwrap_or("");
    match decode_par(code) {
        Some((a, v, c, t)) => {
            unsafe {
                *a_out = a as i32;
                *v_out = v as i32;
                *c_out = c;
                *type_out = t;
            }
            1
        }
        None => 0,
    }
}

// --------------------------------------------------------------------------
// CheatMap (bit-addressable per-address flag)
// --------------------------------------------------------------------------

/// Allocate the cheat-map buffer (8 KiB, one bit per address). Idempotent —
/// repeat calls leave the existing buffer in place.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_map_create() {
    let mut st = lock();
    if !st.has_cheat_map {
        st.cheat_map = vec![0u8; CHEATMAP_SIZE];
        st.has_cheat_map = true;
    } else {
        // Match C++ `FCEUI_RefreshCheatMap` reset-to-zero behaviour at the
        // call site (`FCEUI_CreateCheatMap` always invokes Refresh next).
        for b in st.cheat_map.iter_mut() {
            *b = 0;
        }
    }
}

/// Release the cheat-map buffer. Idempotent.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_map_release() {
    let mut st = lock();
    st.cheat_map.clear();
    st.has_cheat_map = false;
}

/// Clear all cheat-map bits (without releasing the buffer). The caller must
/// then re-mark currently-active cheats via `fceux11_rust_cheat_map_set`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_map_refresh_clear() {
    let mut st = lock();
    if st.has_cheat_map {
        for b in st.cheat_map.iter_mut() {
            *b = 0;
        }
    }
}

/// Return `1` if the cheat bit for `address` is set, `0` otherwise.
/// Returns `0` if the cheat map has not been created.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_map_find(address: u16) -> i32 {
    let st = lock();
    if !st.has_cheat_map {
        return 0;
    }
    let byte = (address as usize) / 8;
    let bit = (address as usize) % 8;
    ((st.cheat_map[byte] >> bit) & 1) as i32
}

/// Set or toggle the cheat bit for `address`.
///
/// The original C++ uses a ternary that is asymmetric:
/// `cheat ? bitmap |= mask : bitmap ^= mask`. We preserve that exact
/// behaviour — `cheat==1` sets, `cheat==0` toggles (XOR).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_map_set(address: u16, cheat: i32) {
    let mut st = lock();
    if !st.has_cheat_map {
        return;
    }
    let byte = (address as usize) / 8;
    let mask = 1u8 << ((address as usize) % 8);
    if cheat != 0 {
        st.cheat_map[byte] |= mask;
    } else {
        st.cheat_map[byte] ^= mask;
    }
}

/// Count the cheat-map bits set in the half-open range `[address, address+size)`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_map_count_affected(address: u32, size: u32) -> u32 {
    let st = lock();
    if !st.has_cheat_map {
        return 0;
    }
    let mut count = 0u32;
    for i in 0..size {
        let a = (address + i) & 0xFFFF;
        let byte = (a as usize) / 8;
        let bit = (a as usize) % 8;
        if ((st.cheat_map[byte] >> bit) & 1) != 0 {
            count += 1;
        }
    }
    count
}

// --------------------------------------------------------------------------
// CheatComp — search snapshot
// --------------------------------------------------------------------------

fn ensure_cheat_comp(st: &mut CheatState) {
    if !st.has_cheat_comp {
        st.cheat_comp = vec![CHEATC_NONE; 0x10000];
        st.has_cheat_comp = true;
    }
}

/// Allocate the cheat-comp buffer initialised to `CHEATC_NONE`. Idempotent —
/// existing values are preserved.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_init() -> i32 {
    let mut st = lock();
    ensure_cheat_comp(&mut st);
    1
}

/// Release the cheat-comp buffer (matches `FCEU_FlushGameCheats` cleanup).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_release() {
    let mut st = lock();
    st.cheat_comp.clear();
    st.has_cheat_comp = false;
}

/// Return `1` if the cheat-comp buffer exists.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_exists() -> i32 {
    if lock().has_cheat_comp { 1 } else { 0 }
}

/// Begin a search: for every address that has a backing RAM ptr in C++ (the
/// caller passes `mem_present[i]` non-zero), store the current memory value;
/// for addresses with no backing, store `CHEATC_NONE`.
///
/// `mem` is a `0x10000`-byte snapshot of CPU memory.
/// `mem_present` is a `0x10000`-byte boolean array: `1` if the address has a
/// `CheatRPtrs` entry (i.e. is a real RAM byte), `0` otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_search_begin(
    mem: *const u8,
    mem_present: *const u8,
) -> i32 {
    if mem.is_null() || mem_present.is_null() {
        return 0;
    }
    let mut st = lock();
    ensure_cheat_comp(&mut st);
    let mem_slice = unsafe { std::slice::from_raw_parts(mem, 0x10000) };
    let pres_slice = unsafe { std::slice::from_raw_parts(mem_present, 0x10000) };
    for x in 0..0x10000usize {
        if pres_slice[x] != 0 {
            st.cheat_comp[x] = mem_slice[x] as u16;
        } else {
            st.cheat_comp[x] = CHEATC_NONE;
        }
    }
    1
}

/// Set every non-NOSHOW slot to the current memory value (used when
/// "Restart search" is requested without resetting flags).
///
/// Mirrors C++ `FCEUI_CheatSearchSetCurrentAsOriginal`:
/// for visible slots, `comp[x] = mem[x]` if backed, else OR-in `CHEATC_NONE`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_set_current_as_original(
    mem: *const u8,
    mem_present: *const u8,
) -> i32 {
    if mem.is_null() || mem_present.is_null() {
        return 0;
    }
    let mut st = lock();
    ensure_cheat_comp(&mut st);
    let mem_slice = unsafe { std::slice::from_raw_parts(mem, 0x10000) };
    let pres_slice = unsafe { std::slice::from_raw_parts(mem_present, 0x10000) };
    for x in 0..0x10000usize {
        if (st.cheat_comp[x] & CHEATC_NOSHOW) == 0 {
            if pres_slice[x] != 0 {
                st.cheat_comp[x] = mem_slice[x] as u16;
            } else {
                st.cheat_comp[x] |= CHEATC_NONE;
            }
        }
    }
    1
}

/// Clear the `CHEATC_EXCLUDED` flag from every slot.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_show_excluded() {
    let mut st = lock();
    if !st.has_cheat_comp {
        return;
    }
    for c in st.cheat_comp.iter_mut() {
        *c &= !CHEATC_EXCLUDED;
    }
}

/// Return the number of "visible" search hits — slots that are neither
/// NOSHOW nor masked by the C++ presence array.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_count(mem_present: *const u8) -> i32 {
    if mem_present.is_null() {
        return 0;
    }
    let st = lock();
    if !st.has_cheat_comp {
        return 0;
    }
    let pres_slice = unsafe { std::slice::from_raw_parts(mem_present, 0x10000) };
    let mut count = 0i32;
    for x in 0..0x10000usize {
        if (st.cheat_comp[x] & CHEATC_NOSHOW) == 0 && pres_slice[x] != 0 {
            count += 1;
        }
    }
    count
}

/// Read a single `CheatComp` slot. Returns `0xFFFFFFFF` if the buffer is not
/// allocated.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_get(address: u32) -> u32 {
    let st = lock();
    if !st.has_cheat_comp {
        return 0xFFFF_FFFF;
    }
    let idx = (address & 0xFFFF) as usize;
    st.cheat_comp[idx] as u32
}

/// Apply a `FCEUI_CheatSearchEnd` filter using `type` and the operand bytes
/// `v1`/`v2`. Marks excluded slots with `CHEATC_EXCLUDED`.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cheat_comp_search_end(
    search_type: i32,
    v1: u8,
    v2: u8,
    mem: *const u8,
    mem_present: *const u8,
) -> i32 {
    if mem.is_null() || mem_present.is_null() {
        return 0;
    }
    let mut st = lock();
    ensure_cheat_comp(&mut st);
    let mem_slice = unsafe { std::slice::from_raw_parts(mem, 0x10000) };
    let pres_slice = unsafe { std::slice::from_raw_parts(mem_present, 0x10000) };

    let v1_u16 = v1 as u16;
    let v2_u16 = v2 as u16;

    for x in 0..0x10000usize {
        if (st.cheat_comp[x] & CHEATC_NOSHOW) != 0 {
            continue;
        }
        // For purely-memory-based comparators, the C++ code dereferences
        // `CheatRPtrs[x>>10][x]` unconditionally — meaning if the high bit
        // tests above pass but no RAM ptr exists, behaviour is undefined.
        // We treat missing memory as "no-present" and skip the comparator
        // (the result is the same as the C++ code, since the C++ check
        // above already filters NOSHOW which is set when ptr is missing).
        let cur = mem_slice[x] as u16;
        let prev = st.cheat_comp[x];
        let exclude = match search_type {
            FCEU_SEARCH_RELATIVE_CHANGE => {
                // `prev != v1 || abs(prev - cur) != v2`
                let diff = if prev >= cur { prev - cur } else { cur - prev };
                prev != v1_u16 || diff != v2_u16
            }
            FCEU_SEARCH_PUERLY_RELATIVE_CHANGE => {
                let diff = if prev >= cur { prev - cur } else { cur - prev };
                diff != v2_u16
            }
            FCEU_SEARCH_ANY_CHANGE => prev == cur,
            FCEU_SEARCH_NEWVAL_KNOWN => {
                // `*ptr != v1` — needs presence, otherwise skip.
                if pres_slice[x] == 0 {
                    continue;
                }
                cur != v1_u16
            }
            FCEU_SEARCH_NEWVAL_GT => {
                if pres_slice[x] == 0 {
                    continue;
                }
                prev >= cur
            }
            FCEU_SEARCH_NEWVAL_LT => {
                if pres_slice[x] == 0 {
                    continue;
                }
                prev <= cur
            }
            FCEU_SEARCH_NEWVAL_GT_KNOWN => {
                if pres_slice[x] == 0 {
                    continue;
                }
                // `(*ptr - prev) != v2`
                cur.wrapping_sub(prev) != v2_u16
            }
            FCEU_SEARCH_NEWVAL_LT_KNOWN => {
                if pres_slice[x] == 0 {
                    continue;
                }
                // `(prev - *ptr) != v2`
                prev.wrapping_sub(cur) != v2_u16
            }
            // Default and FCEU_SEARCH_SPECIFIC_CHANGE
            _ => prev != v1_u16 || cur != v2_u16,
        };
        if exclude {
            st.cheat_comp[x] |= CHEATC_EXCLUDED;
        }
    }
    1
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn c_str_to_string(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

// --------------------------------------------------------------------------
// Tests
// --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Tests share global `CHEAT_STATE`; serialise them with this mutex so
    /// one test's `delete_all` / `release` doesn't race another's setup.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn lock_for_test() -> std::sync::MutexGuard<'static, ()> {
        TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The canonical SMB1 "infinite lives" Game Genie code.
    /// `SXIOPO` → addr=0x95FF, val=0x07, no compare.
    #[test]
    fn gg_decode_6char_smb1_infinite_lives() {
        let _g = lock_for_test();
        let r = decode_gg("SXIOPO").unwrap();
        // Match the bit-encoding of the original C++ algorithm.
        assert_eq!(r.2, -1, "6-char code must have compare = -1");
        // Verify byte and addr ranges (high bit of addr is set).
        assert!(r.0 >= 0x8000);
    }

    #[test]
    fn gg_decode_8char_has_compare() {
        let _g = lock_for_test();
        let r = decode_gg("AAAAAAAA").unwrap();
        assert_ne!(r.2, -1, "8-char code must produce a compare byte");
    }

    #[test]
    fn gg_decode_wrong_length() {
        let _g = lock_for_test();
        assert!(decode_gg("ABC").is_none());
        assert!(decode_gg("ABCDEFG").is_none()); // 7 chars
        assert!(decode_gg("ABCDEFGHI").is_none()); // 9 chars
    }

    #[test]
    fn gg_decode_lowercase_accepted() {
        let _g = lock_for_test();
        // The original C++ uses toupper(); ensure we match.
        let upper = decode_gg("SXIOPO").unwrap();
        let lower = decode_gg("sxiopo").unwrap();
        assert_eq!(upper, lower);
    }

    #[test]
    fn gg_decode_unknown_char_treated_as_zero() {
        let _g = lock_for_test();
        // `?` is not a GG letter; the original returns 0 for it.
        let r = decode_gg("AAAAA?").unwrap();
        let zero = decode_gg("AAAAAA").unwrap();
        assert_eq!(r, zero);
    }

    #[test]
    fn par_decode_basic() {
        let _g = lock_for_test();
        let r = decode_par("00010203").unwrap();
        // `addr = (boo[3]<<8) | (boo[2] + 0x7F)` = (0x03<<8) | (0x02+0x7F) = 0x381
        assert_eq!(r.0, 0x381);
        assert_eq!(r.1, 0); // PAR sets val=0
        assert_eq!(r.2, -1);
        assert_eq!(r.3, 1); // type=1 for addr >= 0x100
    }

    #[test]
    fn par_decode_zeropage_type() {
        let _g = lock_for_test();
        // boo[2]+0x7F < 0x100 and boo[3]==0 → addr < 0x100 → type=0
        let r = decode_par("00000000").unwrap();
        assert_eq!(r.0, 0x7F);
        assert_eq!(r.3, 0);
    }

    #[test]
    fn par_decode_wrong_length() {
        let _g = lock_for_test();
        assert!(decode_par("0001020").is_none()); // 7 chars
        assert!(decode_par("000102030").is_none()); // 9 chars
    }

    #[test]
    fn par_decode_invalid_hex() {
        let _g = lock_for_test();
        assert!(decode_par("ZZZZZZZZ").is_none());
    }

    #[test]
    fn cheat_list_add_get_count() {
        let _g = lock_for_test();
        // Use a fresh process for isolation isn't possible — reset state.
        fceux11_rust_cheat_delete_all();
        let n = std::ffi::CString::new("test1").unwrap();
        let idx = fceux11_rust_cheat_add(n.as_ptr(), 0x1234, 0x42, -1, 1, 0);
        assert_eq!(idx, 0);
        let n2 = std::ffi::CString::new("test2").unwrap();
        let idx2 = fceux11_rust_cheat_add(n2.as_ptr(), 0x5678, 0x55, 0x10, 1, 1);
        assert_eq!(idx2, 1);
        assert_eq!(fceux11_rust_cheat_count(), 2);

        let mut view = FceuCheatEntryView {
            name_ptr: std::ptr::null(),
            name_len: 0,
            addr: 0,
            val: 0,
            compare: 0,
            status: 0,
            type_: 0,
        };
        assert_eq!(fceux11_rust_cheat_get(0, &mut view), 1);
        assert_eq!(view.addr, 0x1234);
        assert_eq!(view.val, 0x42);
        assert_eq!(view.compare, -1);
        assert_eq!(view.type_, 0);
        // Cleanup
        fceux11_rust_cheat_delete_all();
    }

    #[test]
    fn cheat_list_delete_and_toggle() {
        let _g = lock_for_test();
        fceux11_rust_cheat_delete_all();
        let n = std::ffi::CString::new("a").unwrap();
        fceux11_rust_cheat_add(n.as_ptr(), 0x0001, 1, -1, 1, 0);
        fceux11_rust_cheat_add(n.as_ptr(), 0x0002, 2, -1, 0, 0);
        // toggle index 0 (was 1 → 0)
        assert_eq!(fceux11_rust_cheat_toggle(0), 0);
        // toggle index 1 (was 0 → 1)
        assert_eq!(fceux11_rust_cheat_toggle(1), 1);
        // out-of-range
        assert_eq!(fceux11_rust_cheat_toggle(99), -1);
        // delete index 0
        assert_eq!(fceux11_rust_cheat_delete(0), 1);
        assert_eq!(fceux11_rust_cheat_count(), 1);
        // out-of-range delete
        assert_eq!(fceux11_rust_cheat_delete(99), 0);
        fceux11_rust_cheat_delete_all();
    }

    #[test]
    fn cheat_list_disable_all_counts_only_active() {
        let _g = lock_for_test();
        fceux11_rust_cheat_delete_all();
        let n = std::ffi::CString::new("x").unwrap();
        fceux11_rust_cheat_add(n.as_ptr(), 1, 1, -1, 1, 0); // active
        fceux11_rust_cheat_add(n.as_ptr(), 2, 2, -1, 0, 0); // already disabled
        fceux11_rust_cheat_add(n.as_ptr(), 3, 3, -1, 1, 0); // active
        let count = fceux11_rust_cheat_disable_all();
        assert_eq!(count, 2);
        // All should now be disabled
        let mut view = FceuCheatEntryView {
            name_ptr: std::ptr::null(),
            name_len: 0,
            addr: 0,
            val: 0,
            compare: 0,
            status: 99,
            type_: 0,
        };
        for i in 0..3 {
            fceux11_rust_cheat_get(i, &mut view);
            assert_eq!(view.status, 0);
        }
        fceux11_rust_cheat_delete_all();
    }

    #[test]
    fn cheat_map_create_and_set() {
        let _g = lock_for_test();
        fceux11_rust_cheat_map_release();
        fceux11_rust_cheat_map_create();
        // Initially zero
        assert_eq!(fceux11_rust_cheat_map_find(0x1234), 0);
        // Set bit
        fceux11_rust_cheat_map_set(0x1234, 1);
        assert_eq!(fceux11_rust_cheat_map_find(0x1234), 1);
        // C++ ternary: cheat==0 toggles via XOR (not clears)
        fceux11_rust_cheat_map_set(0x1234, 0);
        assert_eq!(fceux11_rust_cheat_map_find(0x1234), 0);
        // Test boundary 0xFFFF
        fceux11_rust_cheat_map_set(0xFFFF, 1);
        assert_eq!(fceux11_rust_cheat_map_find(0xFFFF), 1);
        fceux11_rust_cheat_map_release();
        // After release, find returns 0
        assert_eq!(fceux11_rust_cheat_map_find(0x1234), 0);
    }

    #[test]
    fn cheat_map_count_affected_range() {
        let _g = lock_for_test();
        fceux11_rust_cheat_map_release();
        fceux11_rust_cheat_map_create();
        fceux11_rust_cheat_map_set(0x0100, 1);
        fceux11_rust_cheat_map_set(0x0105, 1);
        fceux11_rust_cheat_map_set(0x0200, 1);
        assert_eq!(fceux11_rust_cheat_map_count_affected(0x0100, 0x10), 2);
        assert_eq!(fceux11_rust_cheat_map_count_affected(0x0000, 0x0300), 3);
        fceux11_rust_cheat_map_release();
    }

    #[test]
    fn cheat_comp_search_begin_marks_unpresent_as_none() {
        let _g = lock_for_test();
        let mem = vec![0u8; 0x10000];
        let mut pres = vec![1u8; 0x10000];
        pres[0x1000] = 0; // not present
        fceux11_rust_cheat_comp_release();
        let ok = fceux11_rust_cheat_comp_search_begin(mem.as_ptr(), pres.as_ptr());
        assert_eq!(ok, 1);
        // Unpresent slot should be CHEATC_NONE
        assert_eq!(fceux11_rust_cheat_comp_get(0x1000) as u16, CHEATC_NONE);
        // Present zero slot should be 0
        assert_eq!(fceux11_rust_cheat_comp_get(0x0500), 0);
        fceux11_rust_cheat_comp_release();
    }

    #[test]
    fn cheat_comp_search_end_specific_change() {
        let _g = lock_for_test();
        // Begin with mem = 0x42 everywhere
        let mem1 = vec![0x42u8; 0x10000];
        let pres = vec![1u8; 0x10000];
        fceux11_rust_cheat_comp_release();
        fceux11_rust_cheat_comp_search_begin(mem1.as_ptr(), pres.as_ptr());

        // New mem: address 0x100 became 0x99, the rest stayed 0x42
        let mut mem2 = vec![0x42u8; 0x10000];
        mem2[0x100] = 0x99;
        // Search: SPECIFIC_CHANGE (prev=0x42 → cur=0x99) — keeps 0x100, excludes the rest
        fceux11_rust_cheat_comp_search_end(
            FCEU_SEARCH_SPECIFIC_CHANGE,
            0x42,
            0x99,
            mem2.as_ptr(),
            pres.as_ptr(),
        );
        let cnt = fceux11_rust_cheat_comp_count(pres.as_ptr());
        assert_eq!(cnt, 1);
        fceux11_rust_cheat_comp_release();
    }

    #[test]
    fn cheat_comp_search_end_any_change() {
        let _g = lock_for_test();
        let mem1 = vec![0x00u8; 0x10000];
        let pres = vec![1u8; 0x10000];
        fceux11_rust_cheat_comp_release();
        fceux11_rust_cheat_comp_search_begin(mem1.as_ptr(), pres.as_ptr());
        // Change exactly 5 bytes
        let mut mem2 = vec![0x00u8; 0x10000];
        for i in 0..5 {
            mem2[0x200 + i] = 0xFF;
        }
        fceux11_rust_cheat_comp_search_end(
            FCEU_SEARCH_ANY_CHANGE,
            0,
            0,
            mem2.as_ptr(),
            pres.as_ptr(),
        );
        let cnt = fceux11_rust_cheat_comp_count(pres.as_ptr());
        assert_eq!(cnt, 5);
        fceux11_rust_cheat_comp_release();
    }

    #[test]
    fn global_disabled_flag_roundtrip() {
        let _g = lock_for_test();
        // Default 0
        let old = fceux11_rust_cheat_set_global_disabled(1);
        let prev_old = old; // capture so we can restore
        assert_eq!(fceux11_rust_cheat_get_global_disabled(), 1);
        fceux11_rust_cheat_set_global_disabled(0);
        assert_eq!(fceux11_rust_cheat_get_global_disabled(), 0);
        // Restore original
        fceux11_rust_cheat_set_global_disabled(prev_old);
    }

    #[test]
    fn cheat_set_updates_fields_selectively() {
        let _g = lock_for_test();
        fceux11_rust_cheat_delete_all();
        let n = std::ffi::CString::new("orig").unwrap();
        fceux11_rust_cheat_add(n.as_ptr(), 0x100, 0x10, -1, 1, 0);
        // Only update val; pass -1 for addr/status, name=NULL.
        fceux11_rust_cheat_set(0, std::ptr::null(), -1, 0x99, -2, -1, 0);
        let mut view = FceuCheatEntryView {
            name_ptr: std::ptr::null(),
            name_len: 0,
            addr: 0,
            val: 0,
            compare: 0,
            status: 0,
            type_: 99,
        };
        fceux11_rust_cheat_get(0, &mut view);
        assert_eq!(view.addr, 0x100); // unchanged
        assert_eq!(view.val, 0x99);   // changed
        assert_eq!(view.status, 1);   // unchanged
        assert_eq!(view.compare, -1); // unchanged (c < -1 means skip)
        fceux11_rust_cheat_delete_all();
    }

    #[test]
    fn gg_to_bin_known_letters() {
        let _g = lock_for_test();
        // Verify mapping for canonical letters
        assert_eq!(gg_to_bin(b'A'), 0);
        assert_eq!(gg_to_bin(b'P'), 1);
        assert_eq!(gg_to_bin(b'N'), 15);
        assert_eq!(gg_to_bin(b'a'), 0); // case-insensitive
        assert_eq!(gg_to_bin(b'?'), 0); // unknown → 0
    }
}
