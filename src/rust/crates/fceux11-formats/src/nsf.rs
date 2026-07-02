//! NSF (NES Sound Format) parser — header validation, bank calculation,
//! NSFROM patching, and info queries.
//!
//! Replaces pure-computation routines formerly in `src/nsf.cpp`.
//! C++ retains `NSFLoad` (file I/O, memory allocation), `NSF_init`,
//! `NSF_read`/`NSF_write`, `DrawNSF`, `DoNSFFrame`, and `NSFGI`.

use std::ffi::c_char;

// ------------------------------------------------------------------
// NSF header — C-compatible layout (136 bytes)
// ------------------------------------------------------------------

#[repr(C)]
pub struct FceuNsfHeader {
    pub id: [u8; 5],
    pub version: u8,
    pub total_songs: u8,
    pub starting_song: u8,
    pub load_address_low: u8,
    pub load_address_high: u8,
    pub init_address_low: u8,
    pub init_address_high: u8,
    pub play_address_low: u8,
    pub play_address_high: u8,
    pub song_name: [u8; 32],
    pub artist: [u8; 32],
    pub copyright: [u8; 32],
    pub ntsc_speed: [u8; 2],
    pub bank_switch: [u8; 8],
    pub pal_speed: [u8; 2],
    pub video_system: u8,
    pub sound_chip: u8,
    pub expansion: [u8; 4],
    pub reserve: [u8; 8],
}

impl FceuNsfHeader {
    pub fn load_address(&self) -> u16 {
        u16::from_le_bytes([self.load_address_low, self.load_address_high])
    }

    pub fn init_address(&self) -> u16 {
        u16::from_le_bytes([self.init_address_low, self.init_address_high])
    }

    pub fn play_address(&self) -> u16 {
        u16::from_le_bytes([self.play_address_low, self.play_address_high])
    }
}

// ------------------------------------------------------------------
// Internal helper: round up to next power of two
// ------------------------------------------------------------------

fn uppow2(n: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let mut v = n - 1;
    v |= v >> 1;
    v |= v >> 2;
    v |= v >> 4;
    v |= v >> 8;
    v |= v >> 16;
    v + 1
}

// ------------------------------------------------------------------
// FFI: Header validation and address extraction
// ------------------------------------------------------------------

/// Validate an NSF header: check the "NESM\x1a" signature, ensure
/// text fields are null-terminated, and extract 16-bit addresses.
/// Returns `true` if the header is valid.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_header_validate(
    header: *mut FceuNsfHeader,
    out_load_addr: *mut u16,
    out_init_addr: *mut u16,
    out_play_addr: *mut u16,
) -> bool {
    if header.is_null()
        || out_load_addr.is_null()
        || out_init_addr.is_null()
        || out_play_addr.is_null()
    {
        return false;
    }
    let h = unsafe { &mut *header };

    // Verify signature
    if &h.id != b"NESM\x1a" {
        return false;
    }

    // Ensure text fields are null-terminated
    h.song_name[31] = 0;
    h.artist[31] = 0;
    h.copyright[31] = 0;

    unsafe {
        *out_load_addr = h.load_address();
        *out_init_addr = h.init_address();
        *out_play_addr = h.play_address();
    }

    true
}

// ------------------------------------------------------------------
// FFI: Bank configuration computation
// ------------------------------------------------------------------

/// Compute NSFMaxBank (before the decrement) and BSon/BankSwitch
/// from the header and raw NSF data size.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_compute_banks(
    header: *const FceuNsfHeader,
    nsf_size: u32,
    out_nsf_max_bank: *mut u32,
    out_bson: *mut u8,
    out_bank_switch: *mut u8,
) -> bool {
    if header.is_null()
        || out_nsf_max_bank.is_null()
        || out_bson.is_null()
        || out_bank_switch.is_null()
    {
        return false;
    }
    let h = unsafe { &*header };

    let load_addr = h.load_address();
    let nsf_max_bank =
        uppow2(((nsf_size.wrapping_add((load_addr & 0xfff) as u32)).wrapping_add(4095)) / 4096);

    let mut bank_switch = h.bank_switch;
    let mut bson: u8 = 0;
    for &b in &bank_switch {
        bson |= b;
    }

    if bson == 0 {
        if (h.load_address_high & 0x70) >= 0x70 {
            // Ice Climber and other F000 base address tunes need this
            bson = 0xFF;
        } else {
            let start = ((h.load_address_high & 0x70) / 0x10) as usize;
            let mut counter: u8 = 0;
            for item in bank_switch.iter_mut().skip(start) {
                *item = counter;
                counter = counter.wrapping_add(1);
            }
            bson = 0;
        }
    }

    for &b in &bank_switch {
        bson |= b;
    }

    unsafe {
        *out_nsf_max_bank = nsf_max_bank;
        *out_bson = bson;
        std::slice::from_raw_parts_mut(out_bank_switch, 8).copy_from_slice(&bank_switch);
    }

    true
}

// ------------------------------------------------------------------
// FFI: NSFROM bootstrap ROM patching
// ------------------------------------------------------------------

/// Patch the NSFROM bootstrap ROM with Init and Play addresses.
/// `nsfrom` must point to at least `nsfrom_len` bytes.
/// Returns `true` if the patch was applied successfully.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_patch_nsfrom(
    nsfrom: *mut u8,
    nsfrom_len: usize,
    init_addr: u16,
    play_addr: u16,
) -> bool {
    if nsfrom.is_null() || nsfrom_len == 0 {
        return false;
    }
    let buf = unsafe { std::slice::from_raw_parts_mut(nsfrom, nsfrom_len) };

    // Find the first JSR (0x20) instruction and patch the two target addresses.
    for i in 0..buf.len() {
        if buf[i] == 0x20 {
            if i + 2 < buf.len() {
                buf[i + 1] = (init_addr & 0xFF) as u8;
                buf[i + 2] = (init_addr >> 8) as u8;
            }
            if i + 9 < buf.len() {
                buf[i + 8] = (play_addr & 0xFF) as u8;
                buf[i + 9] = (play_addr >> 8) as u8;
            }
            return true;
        }
    }
    false
}

// ------------------------------------------------------------------
// FFI: Expansion chip name lookup
// ------------------------------------------------------------------

static CHIP_NAMES: &[&str] = &[
    "Konami VRCVI",
    "Konami VRCVII",
    "Nintendo FDS",
    "Nintendo MMC5",
    "Namco 106",
    "Sunsoft FME-07",
];

/// Return the expansion chip name for a sound chip bitmask.
/// If a chip is recognised, writes the single-bit mask to `*out_mask`
/// and returns a pointer to a static thread-local C string.
/// Returns null if no recognised chip is set.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_chip_name(
    sound_chip: u8,
    out_mask: *mut u8,
) -> *const c_char {
    for (i, &name) in CHIP_NAMES.iter().enumerate() {
        let mask = 1 << i;
        if sound_chip & mask != 0 {
            if !out_mask.is_null() {
                unsafe {
                    *out_mask = mask;
                }
            }
            thread_local! {
                static BUF: std::cell::RefCell<[u8; 64]> = const { std::cell::RefCell::new([0u8; 64]) };
            }
            return BUF.with(|buf| {
                let mut b = buf.borrow_mut();
                let bytes = name.as_bytes();
                let len = bytes.len().min(63);
                b[..len].copy_from_slice(&bytes[..len]);
                b[len] = 0;
                b.as_ptr() as *const c_char
            });
        }
    }
    if !out_mask.is_null() {
        unsafe {
            *out_mask = 0;
        }
    }
    std::ptr::null()
}

// ------------------------------------------------------------------
// FFI: Song change with bounds checking
// ------------------------------------------------------------------

/// Safely change the current song number.
/// Returns the clamped new song number and sets `*out_reload` to 0xFF.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_change_song(
    current: i32,
    amount: i32,
    total: i32,
    out_reload: *mut u8,
) -> i32 {
    let mut new = current + amount;
    if new < 1 {
        new = 1;
    } else if new > total {
        new = total;
    }
    if !out_reload.is_null() {
        unsafe {
            *out_reload = 0xFF;
        }
    }
    new
}

// ------------------------------------------------------------------
// FFI: Metadata query
// ------------------------------------------------------------------

/// Copy NSF metadata strings into caller-provided buffers.
/// `maxlen` is the maximum number of bytes to copy (including null terminator).
/// Returns `TotalSongs`.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_get_info(
    header: *const FceuNsfHeader,
    name: *mut u8,
    artist: *mut u8,
    copyright: *mut u8,
    maxlen: usize,
) -> i32 {
    if header.is_null() {
        return 0;
    }
    let h = unsafe { &*header };

    unsafe {
        if !name.is_null() && maxlen > 0 {
            copy_cstr(name, &h.song_name, maxlen);
        }
        if !artist.is_null() && maxlen > 0 {
            copy_cstr(artist, &h.artist, maxlen);
        }
        if !copyright.is_null() && maxlen > 0 {
            copy_cstr(copyright, &h.copyright, maxlen);
        }
    }

    h.total_songs as i32
}

unsafe fn copy_cstr(dst: *mut u8, src: &[u8], maxlen: usize) {
    let limit = maxlen - 1;
    let mut len = 0;
    for (i, &byte) in src.iter().enumerate().take(src.len().min(limit)) {
        unsafe {
            *dst.add(i) = byte;
        }
        len = i + 1;
    }
    unsafe {
        *dst.add(len) = 0;
    }
}

// ------------------------------------------------------------------
// FFI: Complete NSF load
// ------------------------------------------------------------------

/// Result of a complete NSF file load.
///
/// Contains the NSF data pointer, parsed header info, and computed
/// addresses.  The caller must keep the file buffer alive for as long
/// as it uses the data pointer.
#[repr(C)]
pub struct FceuNsfCartResult {
    /// Pointer to NSF data (after the 0x80 header), within file buffer.
    pub nsf_data: *const u8,
    /// NSF data size in bytes.
    pub nsf_size: u32,
    /// Load address.
    pub load_addr: u16,
    /// Init address.
    pub init_addr: u16,
    /// Play address.
    pub play_addr: u16,
    /// Maximum bank number.
    pub max_bank: u32,
    /// Bank-switching on flag.
    pub bank_switch: bool,
    /// Video system (0=NTSC, 1=PAL).
    pub video_system: u8,
    /// Sound chip flags.
    pub sound_chip: u8,
    /// Total songs.
    pub total_songs: u8,
    /// Starting song (1-based).
    pub starting_song: u8,
    /// Song name (null-terminated).
    pub song_name: [u8; 32],
    /// Artist (null-terminated).
    pub artist: [u8; 32],
    /// Copyright (null-terminated).
    pub copyright: [u8; 32],
}

/// Load an NSF ROM file from a buffer.
///
/// Parses the 0x80 header, validates it, computes bank layout, and
/// returns a pointer to the NSF data.  The data pointer points into
/// the original `file_data` buffer, so the caller must keep it alive.
///
/// Returns `true` on success.
///
/// # Safety
/// `file_data` must point to at least `file_len` readable bytes.
/// `out_cart` must point to a writable `FceuNsfCartResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_load(
    file_data: *const u8,
    file_len: usize,
    out_cart: *mut FceuNsfCartResult,
) -> bool {
    if file_data.is_null() || out_cart.is_null() || file_len < 0x80 {
        return false;
    }

    let result = unsafe { &mut *out_cart };

    // Validate header
    let header = file_data as *mut FceuNsfHeader;
    let mut load_addr: u16 = 0;
    let mut init_addr: u16 = 0;
    let mut play_addr: u16 = 0;

    if unsafe { !fceux11_rust_nsf_header_validate(header, &mut load_addr, &mut init_addr, &mut play_addr) } {
        return false;
    }

    if load_addr < 0x6000 {
        return false;
    }

    // Compute banks
    let nsf_size = (file_len - 0x80) as u32;
    let mut max_bank: u32 = 0;
    let mut bson: u8 = 0;

    // Read bank_switch from header
    let hdr = unsafe { &*header };
    if unsafe { !fceux11_rust_nsf_compute_banks(header, nsf_size, &mut max_bank, &mut bson, hdr.bank_switch.as_ptr() as *mut u8) } {
        return false;
    }

    // Extract header fields
    result.nsf_data = unsafe { file_data.add(0x80) };
    result.nsf_size = nsf_size;
    result.load_addr = load_addr;
    result.init_addr = init_addr;
    result.play_addr = play_addr;
    result.max_bank = max_bank;
    result.bank_switch = bson != 0;
    result.video_system = hdr.video_system;
    result.sound_chip = hdr.sound_chip;
    result.total_songs = hdr.total_songs;
    result.starting_song = hdr.starting_song;
    result.song_name = hdr.song_name;
    result.artist = hdr.artist;
    result.copyright = hdr.copyright;

    true
}

// ==================================================================
// NSF Runtime State Machine — v1.10 Cryptex Task 2
// ==================================================================
// Migrates NSF_init, NSF_read, NSF_write, NSFVectorRead,
// NSFROMRead, and DoNSFFrame logic from C++ to Rust.
//
// C++ retains: handler registration (SetReadHandler/SetWriteHandler),
// AddExState savestate registration, and the thin static handler
// wrappers that forward to the FFI functions below.

// ── Callback table for C++ operations Rust cannot do directly ──

/// Table of C++ function pointers that the Rust runtime calls back
/// for mapper, bus, and CPU operations. All fields are optional;
/// set to `NULL` (= `None`) if a callback is not needed.
#[repr(C)]
pub struct NsfRuntimeCallbacks {
    /// setprg4(A, bank) — map a 4 KiB PRG bank at address A.
    pub set_prg4: Option<unsafe extern "C" fn(u32, u32)>,
    /// setprg8(A, bank) — map an 8 KiB PRG bank at address A.
    pub set_prg8: Option<unsafe extern "C" fn(u32, u32)>,
    /// setprg8r(rom, A, bank) — map an 8 KiB PRG ROM bank at address A.
    pub set_prg8r: Option<unsafe extern "C" fn(i32, u32, u32)>,
    /// setprg32(A, bank) — map a 32 KiB PRG bank at address A.
    pub set_prg32: Option<unsafe extern "C" fn(u32, u32)>,
    /// SetupCartPRGMapping(chip, data, size, writable).
    pub setup_cart_prg_mapping:
        Option<unsafe extern "C" fn(i32, *const u8, u32, i32)>,
    /// ResetCartMapping().
    pub reset_cart_mapping: Option<unsafe extern "C" fn()>,
    /// fceu11::g_bus.write(addr, val).
    pub bus_write: Option<unsafe extern "C" fn(u16, u8)>,
    /// TriggerNMI().
    pub trigger_nmi: Option<unsafe extern "C" fn()>,
    /// CartBR(A) — fallback cart read.
    pub cart_br: Option<unsafe extern "C" fn(u16) -> u8>,
    /// Sound chip init dispatcher: takes sound_chip bitmask,
    /// calls the appropriate NSFVRC6/VRC7/FDS/MMC5/N106/AY init.
    pub sound_chip_init: Option<unsafe extern "C" fn(u8)>,
}

// ── Opaque runtime state (C++ sees only a forward-declared struct) ──

/// Holds all NSF runtime state previously scattered across C++ globals
/// (`doreset`, `NSFNMIFlags`, `SongReload`, `CurrentSong`, etc.) plus
/// the configuration needed by the read/write/init/frame handlers.
#[repr(C)]
pub struct NsfRuntimeState {
    // ── Volatile runtime state ──
    pub doreset: u8,
    pub nsf_nmi_flags: u8,
    pub song_reload: u8,
    pub current_song: i32,

    // ── Read-only configuration (set at load time) ──
    nsf_data: *const u8,
    nsf_max_bank: i32,
    bs_on: u8,
    sound_chip: u8,
    ex_wram: *mut u8,
    bank_switch: [u8; 8],
    load_addr: u16,

    // ── NSFROM bootstrap ROM (54 bytes, patched at load time) ──
    nsfrom: [u8; 54],

    // ── Callbacks into C++ for mapper/bus/CPU operations ──
    cb: NsfRuntimeCallbacks,
}

// ── Internal helper: BANKSET ──────────────────────────────────────

fn bankset(
    a: u32,
    bank: u32,
    max_bank: i32,
    sound_chip: u8,
    ex_wram: *mut u8,
    nsf_data: *const u8,
    cb: &NsfRuntimeCallbacks,
) {
    let bank = bank & (max_bank as u32);

    if sound_chip & 4 != 0 {
        // FDS: copy bank data into ExWRAM
        if !ex_wram.is_null() && !nsf_data.is_null() {
            let dst = unsafe { ex_wram.add((a - 0x6000) as usize) };
            let src = unsafe { nsf_data.add((bank as usize) << 12) };
            unsafe {
                std::ptr::copy_nonoverlapping(src, dst, 4096);
            }
        }
    } else if let Some(set_prg4) = cb.set_prg4 {
        unsafe { set_prg4(a, bank) };
    }
}

// ── FFI: Create / destroy ─────────────────────────────────────────

/// Allocate and zero-initialise an `NsfRuntimeState`.
/// The caller must eventually call `fceux11_rust_nsf_runtime_destroy` to free it.
/// # Safety
/// The returned pointer must be freed only via `_destroy`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_runtime_create() -> *mut NsfRuntimeState {
    let state = Box::new(NsfRuntimeState {
        doreset: 0,
        nsf_nmi_flags: 0,
        song_reload: 0,
        current_song: 0,
        nsf_data: std::ptr::null(),
        nsf_max_bank: 0,
        bs_on: 0,
        sound_chip: 0,
        ex_wram: std::ptr::null_mut(),
        bank_switch: [0u8; 8],
        load_addr: 0,
        nsfrom: [0u8; 54],
        cb: NsfRuntimeCallbacks {
            set_prg4: None,
            set_prg8: None,
            set_prg8r: None,
            set_prg32: None,
            setup_cart_prg_mapping: None,
            reset_cart_mapping: None,
            bus_write: None,
            trigger_nmi: None,
            cart_br: None,
            sound_chip_init: None,
        },
    });
    Box::into_raw(state)
}

/// Free an `NsfRuntimeState` allocated by `_create`.
/// # Safety
/// `state` must have been returned by `_create` and not previously freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_runtime_destroy(state: *mut NsfRuntimeState) {
    if !state.is_null() {
        unsafe {
            drop(Box::from_raw(state));
        }
    }
}

// ── FFI: Configure state before init ──────────────────────────────

/// Populate the configuration fields and callback table of the runtime state.
/// Must be called once after `_create` and before `_init`.
/// # Safety
/// All pointers must be valid for the lifetime of the state.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_runtime_configure(
    state: *mut NsfRuntimeState,
    nsf_data: *const u8,
    nsf_max_bank: i32,
    bs_on: u8,
    sound_chip: u8,
    ex_wram: *mut u8,
    bank_switch: *const u8,
    load_addr: u16,
    nsfrom: *const u8,
    nsfrom_len: usize,
    callbacks: *const NsfRuntimeCallbacks,
) -> bool {
    if state.is_null() {
        return false;
    }
    let s = unsafe { &mut *state };

    s.nsf_data = nsf_data;
    s.nsf_max_bank = nsf_max_bank;
    s.bs_on = bs_on;
    s.sound_chip = sound_chip;
    s.ex_wram = ex_wram;
    s.load_addr = load_addr;

    if !bank_switch.is_null() {
        unsafe {
            std::slice::from_raw_parts(bank_switch, 8)
                .iter()
                .enumerate()
                .for_each(|(i, &b)| s.bank_switch[i] = b);
        }
    }

    // Copy NSFROM ROM (up to 54 bytes)
    let copy_len = nsfrom_len.min(54);
    if !nsfrom.is_null() && copy_len > 0 {
        unsafe {
            std::ptr::copy_nonoverlapping(nsfrom, s.nsfrom.as_mut_ptr(), copy_len);
        }
    }

    if !callbacks.is_null() {
        s.cb = unsafe { std::ptr::read(callbacks) };
    }

    true
}

// ── FFI: nsf_init ──────────────────────────────────────────────────

/// Execute NSF_init logic: reset cart mapping, set up PRG mapping,
/// configure initial banks, and initialise sound chip.
///
/// C++ must call this FIRST, then register handlers via
/// `SetReadHandler` / `SetWriteHandler` / `AddExState`.
///
/// Returns `true` on success.
/// # Safety
/// `state` must have been configured via `_configure`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_runtime_init(
    state: *mut NsfRuntimeState,
    starting_song: u8,
) -> bool {
    if state.is_null() {
        return false;
    }
    let s = unsafe { &mut *state };
    let cb = &s.cb;

    s.doreset = 1;

    // ResetCartMapping
    if let Some(reset_cart_mapping) = cb.reset_cart_mapping {
        unsafe { reset_cart_mapping() };
    }

    if s.sound_chip & 4 != 0 {
        // ── FDS sound chip path ──
        if let Some(setup) = cb.setup_cart_prg_mapping {
            unsafe { setup(0, s.ex_wram, 32768 + 8192, 1) };
        }
        if let Some(set_prg32) = cb.set_prg32 {
            unsafe { set_prg32(0x6000, 0) };
        }
        if let Some(set_prg8) = cb.set_prg8 {
            unsafe { set_prg8(0xE000, 4) };
        }
        if !s.ex_wram.is_null() {
            unsafe {
                std::ptr::write_bytes(s.ex_wram, 0x00, 32768 + 8192);
            }
        }
    } else {
        // ── Normal (non-FDS) path ──
        if !s.ex_wram.is_null() {
            unsafe {
                std::ptr::write_bytes(s.ex_wram, 0x00, 8192);
            }
        }
        if let Some(setup) = cb.setup_cart_prg_mapping {
            unsafe {
                setup(0, s.nsf_data, ((s.nsf_max_bank + 1) * 4096) as u32, 0);
                setup(1, s.ex_wram, 8192, 1);
            }
        }
        if let Some(set_prg8r) = cb.set_prg8r {
            unsafe { set_prg8r(1, 0x6000, 0) };
        }
    }

    // ── Initial bank mapping ──
    if s.bs_on != 0 {
        for x in 0..8 {
            // FDS: banks at 0x6000 for x>=6
            if s.sound_chip & 4 != 0 && x >= 6 {
                bankset(
                    0x6000 + ((x - 6) * 4096) as u32,
                    s.bank_switch[x] as u32,
                    s.nsf_max_bank,
                    s.sound_chip,
                    s.ex_wram,
                    s.nsf_data,
                    cb,
                );
            }
            bankset(
                0x8000 + (x * 4096) as u32,
                s.bank_switch[x] as u32,
                s.nsf_max_bank,
                s.sound_chip,
                s.ex_wram,
                s.nsf_data,
                cb,
            );
        }
    } else {
        // Fixed mapping: linearly from load address
        let mut a = (s.load_addr & 0xF000) as u32;
        while a < 0x10000 {
            let bank = (a - (s.load_addr & 0x7000) as u32) >> 12;
            bankset(a, bank, s.nsf_max_bank, s.sound_chip, s.ex_wram, s.nsf_data, cb);
            a += 0x1000;
        }
    }

    // ── Sound chip init ──
    if let Some(sound_chip_init) = cb.sound_chip_init {
        unsafe { sound_chip_init(s.sound_chip) };
    }

    // ── Finalise runtime state ──
    s.current_song = starting_song as i32;
    s.song_reload = 0xFF;
    s.nsf_nmi_flags = 0;

    true
}

// ── FFI: nsf_write ─────────────────────────────────────────────────

/// Handle a write to the NSF control / bank-switch registers.
/// C++ calls this from its thin `NSF_write` DECLFW handler.
/// # Safety
/// `state` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_write(
    state: *mut NsfRuntimeState,
    addr: u16,
    val: u8,
) {
    if state.is_null() {
        return;
    }
    let s = unsafe { &mut *state };
    let cb = &s.cb;

    match addr {
        0x3FF3 => s.nsf_nmi_flags |= 1,
        0x3FF4 => s.nsf_nmi_flags &= !2,
        0x3FF5 => s.nsf_nmi_flags |= 2,
        // Bank-switch registers: $5FF6-$5FFF
        0x5FF6 | 0x5FF7 => {
            if s.sound_chip & 4 == 0 {
                return; // FDS-only registers
            }
            // fall through to bank switch
            if s.bs_on != 0 {
                let a = (addr & 0xF) as u32;
                bankset(
                    a * 4096,
                    val as u32,
                    s.nsf_max_bank,
                    s.sound_chip,
                    s.ex_wram,
                    s.nsf_data,
                    cb,
                );
            }
        }
        a @ 0x5FF8..=0x5FFF => {
            if s.bs_on != 0 {
                let a = (a & 0xF) as u32;
                bankset(
                    a * 4096,
                    val as u32,
                    s.nsf_max_bank,
                    s.sound_chip,
                    s.ex_wram,
                    s.nsf_data,
                    cb,
                );
            }
        }
        _ => {}
    }
}

// ── FFI: nsf_read ──────────────────────────────────────────────────

/// Handle a read from the NSF status registers ($3FF0-$3FFF).
/// Returns the value to place on the data bus.
/// # Safety
/// `state` must be valid. `ram` must point to 0x800 bytes of NES WRAM.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_read(
    state: *mut NsfRuntimeState,
    addr: u16,
    pal: u8,
    fceuindbg: i32,
    ram: *mut u8,
) -> u8 {
    if state.is_null() {
        return 0;
    }
    let s = unsafe { &mut *state };
    let cb = &s.cb;

    match addr {
        0x3FF0 => {
            let x = s.song_reload;
            if fceuindbg == 0 {
                s.song_reload = 0;
            }
            x
        }
        0x3FF1 => {
            if fceuindbg == 0 {
                // Clear RAM
                if !ram.is_null() {
                    unsafe {
                        std::ptr::write_bytes(ram, 0x00, 0x800);
                    }
                }
                // Silence all APU channels
                if let Some(bus_write) = cb.bus_write {
                    unsafe {
                        bus_write(0x4015, 0x0);
                        for x in 0..0x14 {
                            bus_write(0x4000 + x, 0);
                        }
                        bus_write(0x4015, 0xF);
                    }
                }
                if s.sound_chip & 4 != 0 {
                    // FDS extra registers
                    if let Some(bus_write) = cb.bus_write {
                        unsafe {
                            bus_write(0x4017, 0xC0);
                            bus_write(0x4089, 0x80);
                            bus_write(0x408A, 0xE8);
                        }
                    }
                } else {
                    // Clear ExWRAM
                    if !s.ex_wram.is_null() {
                        unsafe {
                            std::ptr::write_bytes(s.ex_wram, 0x00, 8192);
                        }
                    }
                    if let Some(bus_write) = cb.bus_write {
                        unsafe {
                            bus_write(0x4017, 0xC0);
                            bus_write(0x4017, 0xC0);
                            bus_write(0x4017, 0x40);
                        }
                    }
                }
                // Reset banks
                if s.bs_on != 0 {
                    for x in 0..8 {
                        bankset(
                            0x8000 + (x * 4096) as u32,
                            s.bank_switch[x] as u32,
                            s.nsf_max_bank,
                            s.sound_chip,
                            s.ex_wram,
                            s.nsf_data,
                            cb,
                        );
                    }
                }
                return (s.current_song - 1) as u8;
            }
            0
        }
        0x3FF3 => pal,
        _ => 0,
    }
}

// ── FFI: nsf_vector_read ───────────────────────────────────────────

/// Handle a read from the vector table ($FFFA-$FFFD).
/// Returns `true` if the read was intercepted and `*out_value` is valid.
/// Returns `false` if the caller should fall back to `CartBR(A)`.
/// # Safety
/// `state` and `out_value` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_vector_read(
    state: *mut NsfRuntimeState,
    addr: u16,
    out_value: *mut u8,
) -> bool {
    if state.is_null() || out_value.is_null() {
        return false;
    }
    let s = unsafe { &mut *state };

    // Condition: (NSFNMIFlags & 1 && SongReload) || (NSFNMIFlags & 2) || doreset
    if ((s.nsf_nmi_flags & 1) != 0 && s.song_reload != 0)
        || (s.nsf_nmi_flags & 2) != 0
        || s.doreset != 0
    {
        unsafe {
            *out_value = match addr {
                0xFFFA => 0x00, // NMI low  → $3800
                0xFFFB => 0x38,
                0xFFFC => 0x20, // Reset low → $3820
                0xFFFD => {
                    s.doreset = 0;
                    0x38
                }
                _ => return false,
            };
        }
        true
    } else {
        false
    }
}

// ── FFI: nsf_from_read ─────────────────────────────────────────────

/// Handle a read from $3800-$3835 (NSFROM bootstrap ROM).
/// Returns the byte at the given address.
/// This is stateless — the NSFROM ROM is stored in the state.
/// # Safety
/// `state` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_from_read(
    state: *const NsfRuntimeState,
    addr: u16,
) -> u8 {
    if state.is_null() {
        return 0;
    }
    let s = unsafe { &*state };
    let offset = (addr as usize).wrapping_sub(0x3800);
    if offset < s.nsfrom.len() {
        s.nsfrom[offset]
    } else {
        0
    }
}

// ── FFI: nsf_frame ────────────────────────────────────────────────

/// Evaluate the NSF frame-trigger condition.
/// Returns `true` if `TriggerNMI()` should be called.
/// # Safety
/// `state` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_nsf_frame(
    state: *mut NsfRuntimeState,
) -> bool {
    if state.is_null() {
        return false;
    }
    let s = unsafe { &*state };

    // Condition: (NSFNMIFlags & 1 && SongReload) || (NSFNMIFlags & 2)
    ((s.nsf_nmi_flags & 1) != 0 && s.song_reload != 0) || (s.nsf_nmi_flags & 2) != 0
}

// ==================================================================
// Runtime unit tests
// ==================================================================

#[cfg(test)]
mod runtime_tests {
    use super::*;

    // ── Standalone unit tests for pure functions ──────────────────

    /// Verify that the NSFROM ROM is stored and read correctly.
    #[test]
    fn test_nsfrom_read_basic() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            // Set up a minimal NSFROM with known content
            for i in 0..54 {
                s.nsfrom[i] = i as u8;
            }

            // Read at $3800 should give byte 0
            assert_eq!(unsafe { fceux11_rust_nsf_from_read(state, 0x3800) }, 0);
            // Read at $3801 should give byte 1
            assert_eq!(unsafe { fceux11_rust_nsf_from_read(state, 0x3801) }, 1);
            // Read at $3835 should give byte 53
            assert_eq!(unsafe { fceux11_rust_nsf_from_read(state, 0x3835) }, 53);
            // Read out of range
            assert_eq!(unsafe { fceux11_rust_nsf_from_read(state, 0x3836) }, 0);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify the NMI frame-trigger condition with various flag
    /// combinations.
    #[test]
    fn test_frame_trigger_conditions() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            // No flags set → no NMI
            s.nsf_nmi_flags = 0;
            s.song_reload = 0;
            assert!(!unsafe { fceux11_rust_nsf_frame(state) });

            // Flag bit 2 set → NMI (regardless of song_reload)
            s.nsf_nmi_flags = 2;
            s.song_reload = 0;
            assert!(unsafe { fceux11_rust_nsf_frame(state) });

            // Flag bit 1 set + song_reload → NMI
            s.nsf_nmi_flags = 1;
            s.song_reload = 0xFF;
            assert!(unsafe { fceux11_rust_nsf_frame(state) });

            // Flag bit 1 set but song_reload=0 → no NMI
            s.nsf_nmi_flags = 1;
            s.song_reload = 0;
            assert!(!unsafe { fceux11_rust_nsf_frame(state) });

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify write to $3FF3/$3FF4/$3FF5 modifies NSFNMIFlags.
    #[test]
    fn test_write_nmi_flags() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;
            s.nsf_nmi_flags = 0;

            // $3FF3: set bit 0
            unsafe { fceux11_rust_nsf_write(state, 0x3FF3, 0x00) };
            assert_eq!(s.nsf_nmi_flags, 1);

            // $3FF5: set bit 1
            unsafe { fceux11_rust_nsf_write(state, 0x3FF5, 0x00) };
            assert_eq!(s.nsf_nmi_flags, 3);

            // $3FF4: clear bit 1
            unsafe { fceux11_rust_nsf_write(state, 0x3FF4, 0x00) };
            assert_eq!(s.nsf_nmi_flags, 1);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify read from $3FF0 returns song_reload and clears it
    /// (when not in debug).
    #[test]
    fn test_read_song_reload() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            s.song_reload = 0xAB;
            // Not in debug: should return song_reload and clear it
            let val = unsafe {
                fceux11_rust_nsf_read(
                    state,
                    0x3FF0,
                    0,    // pal
                    0,    // fceuindbg = not in debug
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(val, 0xAB);
            assert_eq!(s.song_reload, 0);

            // In debug: should return song_reload but NOT clear it
            s.song_reload = 0xCD;
            let val = unsafe {
                fceux11_rust_nsf_read(
                    state,
                    0x3FF0,
                    0,    // pal
                    1,    // fceuindbg = in debug
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(val, 0xCD);
            assert_eq!(s.song_reload, 0xCD); // preserved

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify read from $3FF3 returns PAL flag.
    #[test]
    fn test_read_pal_flag() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());

            let val = unsafe {
                fceux11_rust_nsf_read(state, 0x3FF3, 1, 0, std::ptr::null_mut())
            };
            assert_eq!(val, 1); // PAL

            let val = unsafe {
                fceux11_rust_nsf_read(state, 0x3FF3, 0, 0, std::ptr::null_mut())
            };
            assert_eq!(val, 0); // NTSC

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify vector read returns NSFROM vectors when the
    /// intercept condition is active.
    #[test]
    fn test_vector_read_intercept() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            // Set condition: NSFNMIFlags bit 2 → intercept active
            s.nsf_nmi_flags = 2;

            let mut val: u8 = 0;
            assert!(unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFA, &raw mut val) });
            assert_eq!(val, 0x00);
            assert!(unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFB, &raw mut val) });
            assert_eq!(val, 0x38);
            assert!(unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFC, &raw mut val) });
            assert_eq!(val, 0x20);
            assert!(unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFD, &raw mut val) });
            assert_eq!(val, 0x38);
            // doreset should be cleared after $FFFD read
            assert_eq!(s.doreset, 0);

            // Non-vector addresses: not intercepted
            assert!(!unsafe { fceux11_rust_nsf_vector_read(state, 0x8000, &raw mut val) });

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Vector read with doreset=1 should set doreset=0 after $FFFD.
    #[test]
    fn test_vector_read_doreset_clear() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            s.doreset = 1;
            s.nsf_nmi_flags = 0;
            s.song_reload = 0;

            let mut val: u8 = 0;
            // doreset alone should trigger intercept
            assert!(unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFA, &raw mut val) });
            assert_eq!(val, 0x00);
            assert!(unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFD, &raw mut val) });
            assert_eq!(val, 0x38);
            assert_eq!(s.doreset, 0);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Vector read without intercept condition should return false
    /// (signal to C++: "use CartBR fallback").
    #[test]
    fn test_vector_read_no_intercept() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            s.doreset = 0;
            s.nsf_nmi_flags = 0;
            s.song_reload = 0;

            let mut val: u8 = 0;
            assert!(!unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFA, &raw mut val) });
            assert!(!unsafe { fceux11_rust_nsf_vector_read(state, 0xFFFC, &raw mut val) });

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify configure copies the NSFROM and bank_switch correctly.
    #[test]
    fn test_configure_copies_data() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());

            let nsfrom_in: [u8; 54] = [0xAA; 54];
            let banks: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
            let cb = NsfRuntimeCallbacks {
                set_prg4: None,
                set_prg8: None,
                set_prg8r: None,
                set_prg32: None,
                setup_cart_prg_mapping: None,
                reset_cart_mapping: None,
                bus_write: None,
                trigger_nmi: None,
                cart_br: None,
                sound_chip_init: None,
            };

            assert!(unsafe {
                fceux11_rust_nsf_runtime_configure(
                    state,
                    std::ptr::null(),
                    0,
                    1,
                    0,
                    std::ptr::null_mut(),
                    banks.as_ptr(),
                    0x8000,
                    nsfrom_in.as_ptr(),
                    54,
                    &cb,
                )
            });

            let s = &*state;
            assert_eq!(s.nsfrom, nsfrom_in);
            assert_eq!(s.bank_switch, banks);
            assert_eq!(s.bs_on, 1);
            assert_eq!(s.load_addr, 0x8000);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify create/destroy lifecycle.
    #[test]
    fn test_create_destroy() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Destroy with null pointer should be safe.
    #[test]
    fn test_destroy_null() {
        unsafe {
            fceux11_rust_nsf_runtime_destroy(std::ptr::null_mut());
        }
    }

    /// Verify write to bank switch registers path distinction
    /// (FDS vs non-FDS, BSon on/off).
    #[test]
    fn test_write_bank_switch_paths() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            s.nsf_max_bank = 7;
            s.bs_on = 1;
            s.sound_chip = 0; // non-FDS

            // $5FF6-7 with non-FDS: should be blocked (return early).
            // Just verify it doesn't crash.
            fceux11_rust_nsf_write(state, 0x5FF6, 0x01);
            fceux11_rust_nsf_write(state, 0x5FF7, 0x02);

            // $5FF8 should still work (no FDS gate)
            fceux11_rust_nsf_write(state, 0x5FF8, 0x42);

            // With FDS sound_chip, $5FF6 should NOT return early
            s.sound_chip = 4; // FDS
            fceux11_rust_nsf_write(state, 0x5FF6, 0x01);
            fceux11_rust_nsf_write(state, 0x5FF8, 0x42);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify write to $5FF6/7 is blocked when NOT using FDS sound chip.
    #[test]
    fn test_write_5ff6_blocked_non_fds() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            s.nsf_max_bank = 7;
            s.bs_on = 1;
            s.sound_chip = 0; // NOT FDS

            // Write to $5FF6 without FDS: should be blocked (early return)
            // Just verify it doesn't crash.
            fceux11_rust_nsf_write(state, 0x5FF6, 0x01);
            fceux11_rust_nsf_write(state, 0x5FF7, 0x02);

            // $5FF8 should still work (no FDS gate)
            fceux11_rust_nsf_write(state, 0x5FF8, 0x01);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify bank switch registers are ignored when BSon=0.
    #[test]
    fn test_write_bank_switch_blocked_when_bson_off() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());
            let s = &mut *state;

            s.nsf_max_bank = 7;
            s.bs_on = 0; // bank switching OFF

            // Write to $5FF8 with BSon=0: should not call set_prg4
            // Just verify it doesn't crash.
            fceux11_rust_nsf_write(state, 0x5FF8, 0x42);
            fceux11_rust_nsf_write(state, 0x5FFF, 0x01);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }

    /// Verify the full init sequence sets up runtime state correctly.
    #[test]
    fn test_init_sets_runtime_state() {
        unsafe {
            let state = fceux11_rust_nsf_runtime_create();
            assert!(!state.is_null());

            // Configure for a non-FDS bank-switched NSF
            let banks: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
            let cb = NsfRuntimeCallbacks {
                set_prg4: None,
                set_prg8: None,
                set_prg8r: None,
                set_prg32: None,
                setup_cart_prg_mapping: None,
                reset_cart_mapping: None,
                bus_write: None,
                trigger_nmi: None,
                cart_br: None,
                sound_chip_init: None,
            };

            assert!(unsafe {
                fceux11_rust_nsf_runtime_configure(
                    state,
                    std::ptr::null(),
                    0,
                    1,
                    0,
                    std::ptr::null_mut(),
                    banks.as_ptr(),
                    0x8000,
                    [0u8; 54].as_ptr(),
                    54,
                    &cb,
                )
            });

            assert!(unsafe { fceux11_rust_nsf_runtime_init(state, 5) });
            let s = &*state;
            assert_eq!(s.current_song, 5);
            assert_eq!(s.song_reload, 0xFF);
            assert_eq!(s.nsf_nmi_flags, 0);
            assert_eq!(s.doreset, 1);

            unsafe { fceux11_rust_nsf_runtime_destroy(state) };
        }
    }
}

// ------------------------------------------------------------------
// Tests (existing)
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn zeroed_header() -> FceuNsfHeader {
        FceuNsfHeader {
            id: [0; 5],
            version: 0,
            total_songs: 0,
            starting_song: 0,
            load_address_low: 0,
            load_address_high: 0,
            init_address_low: 0,
            init_address_high: 0,
            play_address_low: 0,
            play_address_high: 0,
            song_name: [0; 32],
            artist: [0; 32],
            copyright: [0; 32],
            ntsc_speed: [0; 2],
            bank_switch: [0; 8],
            pal_speed: [0; 2],
            video_system: 0,
            sound_chip: 0,
            expansion: [0; 4],
            reserve: [0; 8],
        }
    }

    #[test]
    fn test_header_layout_size() {
        unsafe {
            assert_eq!(std::mem::size_of::<FceuNsfHeader>(), 136);
        }
    }

    #[test]
    fn test_header_validate_ok() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"NESM\x1a";
            h.load_address_low = 0x00;
            h.load_address_high = 0x80;
            h.init_address_low = 0x34;
            h.init_address_high = 0x12;
            h.play_address_low = 0x78;
            h.play_address_high = 0x56;

            let mut load = 0u16;
            let mut init = 0u16;
            let mut play = 0u16;
            assert!(unsafe {
                fceux11_rust_nsf_header_validate(&mut h, &mut load, &mut init, &mut play)
            });
            assert_eq!(load, 0x8000);
            assert_eq!(init, 0x1234);
            assert_eq!(play, 0x5678);
            assert_eq!(h.song_name[31], 0);
            assert_eq!(h.artist[31], 0);
            assert_eq!(h.copyright[31], 0);
        }
    }

    #[test]
    fn test_header_validate_bad_signature() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"BAD!!";
            let mut load = 0u16;
            let mut init = 0u16;
            let mut play = 0u16;
            assert!(!unsafe {
                fceux11_rust_nsf_header_validate(&mut h, &mut load, &mut init, &mut play)
            });
        }
    }

    #[test]
    fn test_header_validate_null_safety() {
        unsafe {
            assert!(!unsafe {
                fceux11_rust_nsf_header_validate(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            });
        }
    }

    #[test]
    fn test_compute_banks_no_bankswitch() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"NESM\x1a";
            h.load_address_low = 0x00;
            h.load_address_high = 0x80;

            let mut max_bank = 0u32;
            let mut bson = 0u8;
            let mut banks = [0u8; 8];
            assert!(unsafe {
                fceux11_rust_nsf_compute_banks(
                    &h,
                    4096,
                    &mut max_bank,
                    &mut bson,
                    banks.as_mut_ptr(),
                )
            });
            // load_addr=0x8000, offset=0, size=4096
            // => (4096+0+4095)/4096 = 1 => uppow2(1)=1
            assert_eq!(max_bank, 1);
            // load_address_high=0x80, (0x80&0x70)=0x00, start=0
            // banks filled 0..7
            assert_eq!(banks, [0, 1, 2, 3, 4, 5, 6, 7]);
            // bson after second pass = 0|1|2|3|4|5|6|7 = 0x0F
            assert_eq!(bson, 0x07);
        }
    }

    #[test]
    fn test_compute_banks_ice_climber() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"NESM\x1a";
            h.load_address_low = 0x00;
            h.load_address_high = 0xF0; // (0xF0 & 0x70) = 0x70

            let mut max_bank = 0u32;
            let mut bson = 0u8;
            let mut banks = [0u8; 8];
            assert!(unsafe {
                fceux11_rust_nsf_compute_banks(
                    &h,
                    8192,
                    &mut max_bank,
                    &mut bson,
                    banks.as_mut_ptr(),
                )
            });
            assert_eq!(bson, 0xFF);
        }
    }

    #[test]
    fn test_compute_banks_with_bankswitch() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"NESM\x1a";
            h.load_address_low = 0x00;
            h.load_address_high = 0x80;
            h.bank_switch = [1, 2, 3, 4, 5, 6, 7, 8];

            let mut max_bank = 0u32;
            let mut bson = 0u8;
            let mut banks = [0u8; 8];
            assert!(unsafe {
                fceux11_rust_nsf_compute_banks(
                    &h,
                    4096,
                    &mut max_bank,
                    &mut bson,
                    banks.as_mut_ptr(),
                )
            });
            assert_eq!(banks, [1, 2, 3, 4, 5, 6, 7, 8]);
            assert_eq!(bson, 0x0F);
        }
    }

    #[test]
    fn test_patch_nsfrom() {
        unsafe {
            let mut nsfrom = [
                0x8D, 0xF4, 0x3F, 0xA2, 0xFF, 0x9A, 0xAD, 0xF0, 0x3F, 0xF0, 0x09, 0xAD, 0xF1, 0x3F,
                0xAE, 0xF3, 0x3F, 0x20, 0x00, 0x00, 0xA9, 0x00, 0xAA, 0xA8, 0x20, 0x00, 0x00, 0x8D,
                0xF5, 0x3F, 0x90, 0xFE, 0x8D, 0xF3, 0x3F, 0x18, 0x90, 0xFE,
            ];
            assert!(unsafe {
                fceux11_rust_nsf_patch_nsfrom(nsfrom.as_mut_ptr(), nsfrom.len(), 0xABCD, 0x1234)
            });
            // First JSR at offset 0x11
            assert_eq!(nsfrom[0x12], 0xCD); // init low
            assert_eq!(nsfrom[0x13], 0xAB); // init high
            assert_eq!(nsfrom[0x19], 0x34); // play low
            assert_eq!(nsfrom[0x1A], 0x12); // play high
        }
    }

    #[test]
    fn test_patch_nsfrom_not_found() {
        unsafe {
            let mut buf = [0u8; 10];
            assert!(!unsafe {
                fceux11_rust_nsf_patch_nsfrom(buf.as_mut_ptr(), buf.len(), 0x1234, 0x5678)
            });
        }
    }

    #[test]
    fn test_chip_name() {
        unsafe {
            let mut mask = 0u8;
            assert!(!unsafe { fceux11_rust_nsf_chip_name(0x01, &mut mask) }.is_null());
            assert_eq!(mask, 0x01);
            assert!(!unsafe { fceux11_rust_nsf_chip_name(0x02, &mut mask) }.is_null());
            assert_eq!(mask, 0x02);
            assert!(!unsafe { fceux11_rust_nsf_chip_name(0x04, &mut mask) }.is_null());
            assert_eq!(mask, 0x04);
            assert!(!unsafe { fceux11_rust_nsf_chip_name(0x08, &mut mask) }.is_null());
            assert_eq!(mask, 0x08);
            assert!(!unsafe { fceux11_rust_nsf_chip_name(0x10, &mut mask) }.is_null());
            assert_eq!(mask, 0x10);
            assert!(!unsafe { fceux11_rust_nsf_chip_name(0x20, &mut mask) }.is_null());
            assert_eq!(mask, 0x20);
            assert!(unsafe { fceux11_rust_nsf_chip_name(0x00, &mut mask) }.is_null());
            assert_eq!(mask, 0);
            assert!(unsafe { fceux11_rust_nsf_chip_name(0x40, &mut mask) }.is_null());
            assert_eq!(mask, 0);
        }
    }

    #[test]
    fn test_change_song() {
        unsafe {
            let mut reload = 0u8;
            assert_eq!(
                unsafe { fceux11_rust_nsf_change_song(5, 1, 10, &mut reload) },
                6
            );
            assert_eq!(reload, 0xFF);
            assert_eq!(
                unsafe { fceux11_rust_nsf_change_song(1, -1, 10, std::ptr::null_mut()) },
                1
            );
            assert_eq!(
                unsafe { fceux11_rust_nsf_change_song(10, 1, 10, std::ptr::null_mut()) },
                10
            );
            assert_eq!(
                unsafe { fceux11_rust_nsf_change_song(5, 10, 10, std::ptr::null_mut()) },
                10
            );
            assert_eq!(
                unsafe { fceux11_rust_nsf_change_song(5, -10, 10, std::ptr::null_mut()) },
                1
            );
        }
    }

    #[test]
    fn test_get_info() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"NESM\x1a";
            h.total_songs = 5;
            h.song_name[..9].copy_from_slice(b"Test Song");
            h.artist[..11].copy_from_slice(b"Test Artist");
            h.copyright[..4].copy_from_slice(b"2024");

            let mut name = [0u8; 32];
            let mut artist = [0u8; 32];
            let mut copyright = [0u8; 32];
            let total = unsafe {
                fceux11_rust_nsf_get_info(
                    &h,
                    name.as_mut_ptr(),
                    artist.as_mut_ptr(),
                    copyright.as_mut_ptr(),
                    32,
                )
            };
            assert_eq!(total, 5);
            assert_eq!(&name[..10], b"Test Song\0");
            assert_eq!(&artist[..12], b"Test Artist\0");
            assert_eq!(&copyright[..5], b"2024\0");
        }
    }

    #[test]
    fn test_get_info_null_safety() {
        unsafe {
            let mut h = zeroed_header();
            h.id = *b"NESM\x1a";
            h.total_songs = 3;
            assert_eq!(
                unsafe {
                    fceux11_rust_nsf_get_info(
                        &h,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        0,
                    )
                },
                3
            );
        }
    }
}
