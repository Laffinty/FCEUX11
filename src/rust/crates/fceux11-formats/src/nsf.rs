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

// ------------------------------------------------------------------
// Tests
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
