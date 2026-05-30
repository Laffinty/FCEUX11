//! Cart cartridge management — ROM-size calculation, battery-save I/O.
//!
//! Replaces pure-computation and file-I/O routines formerly in
//! `src/ines.cpp` (ROM/CHR size calculation) and `src/cart.cpp`
//! (battery-backed save/load/clear).

use std::ffi::{c_char, CStr};
use std::fs::File;
use std::io::{Read, Write};

use crate::ines::FceuInesHeader;

// ------------------------------------------------------------------
// Internal helper: round up to next power of two (0 stays 0)
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
// C-compatible ROM-size result
// ------------------------------------------------------------------

#[repr(C)]
pub struct FceuRomSizes {
    /// Number of 16 KiB PRG banks (power of two; 256 for the special
    /// iNES-1 zero-size case).
    pub rom_size_16kb: u32,
    /// Number of 8 KiB CHR banks (power of two or iNES-2 computed).
    pub vrom_size_8kb: u32,
    /// CHR-RAM size in bytes.  -1 means "not applicable" (CHR-ROM
    /// present).  0 means "determined later".
    pub chrram_size: i32,
    /// Raw (not-rounded) PRG size in 16 KiB units, before .
    /// Needed for non-power-of-2 ROM loading and debug printing.
    pub rom_size_raw: u32,
}

// ------------------------------------------------------------------
// FFI: compute PRG / CHR ROM sizes from an iNES header
// ------------------------------------------------------------------

/// Compute `ROM_size` and `VROM_size` (in bank counts) from the iNES
/// header.  `mapper_no` is only used for the iNES-2 CHRRAM fallback.
/// Returns `true` on success.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cart_compute_rom_sizes(
    header: *const FceuInesHeader,
    is_nes2: bool,
    out: *mut FceuRomSizes,
) -> bool {
    if header.is_null() || out.is_null() {
        return false;
    }
    let h = unsafe { &*header };
    let out = unsafe { &mut *out };

    // ---- PRG ROM size ----
    let not_round_size = if !is_nes2 {
        h.rom_size as u32
    } else {
        let upper = h.upper_rom_vrom_size & 0x0F;
        if upper != 0x0F {
            (h.rom_size as u32) | ((upper as u32) << 8)
        } else {
            // exponent-multiplier notation (NES 2.0)
            let e = (h.rom_size >> 2) as u32;
            let m = ((h.rom_size & 0b11) as u32) * 2 + 1;
            ((1u32 << e).wrapping_mul(m)) >> 14
        }
    };

    out.rom_size_raw = not_round_size;
    out.rom_size_16kb = if !is_nes2 && h.rom_size == 0 {
        256
    } else {
        uppow2(not_round_size)
    };

    // ---- CHR ROM size ----
    out.vrom_size_8kb = if !is_nes2 {
        uppow2(h.vrom_size as u32)
    } else {
        let upper = (h.upper_rom_vrom_size & 0xF0) >> 4;
        if upper != 0x0F {
            uppow2((h.vrom_size as u32) | ((upper as u32) << 8))
        } else {
            // exponent-multiplier notation (NES 2.0)
            let e = (h.vrom_size >> 2) as u32;
            let m = ((h.vrom_size & 0b11) as u32) * 2 + 1;
            ((1u32 << e).wrapping_mul(m)) >> 13
        }
    };

    out.chrram_size = -1; // default: CHR-ROM present
    true
}

// ------------------------------------------------------------------
// FFI: compute default CHR-RAM size for a mapper
// ------------------------------------------------------------------

/// Return the default CHR-RAM size in bytes when no CHR-ROM is present.
/// For iNES-2 the caller should instead use `battery_vram_size +
/// vram_size`.  Returns -1 when CHR-ROM is present (`has_vrom` true).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cart_compute_chrram_size(
    mapper_no: i32,
    is_nes2: bool,
    vram_size: i32,
    battery_vram_size: i32,
    has_vrom: bool,
) -> i32 {
    if has_vrom {
        -1
    } else if !is_nes2 {
        match mapper_no {
            13 => 16 * 1024,
            6 | 29 | 30 | 45 | 96 => 32 * 1024,
            176 => 128 * 1024,
            _ => 8 * 1024,
        }
    } else {
        battery_vram_size + vram_size
    }
}

// ------------------------------------------------------------------
// Battery save / load / clear
// ------------------------------------------------------------------

#[repr(C)]
pub struct FceuSaveGameEntry {
    pub bufptr: *mut u8,
    pub buflen: u32,
}

/// Save battery-backed RAM to `path`.  Each non-null entry is written
/// sequentially.  Returns `true` on success.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cart_battery_save(
    path: *const c_char,
    entries: *const FceuSaveGameEntry,
    count: usize,
) -> bool {
    let path = unsafe {
        if path.is_null() {
            return false;
        }
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    if entries.is_null() || count == 0 {
        return true;
    }
    let entries = unsafe { std::slice::from_raw_parts(entries, count) };

    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    for entry in entries {
        if !entry.bufptr.is_null() && entry.buflen > 0 {
            let buf = unsafe { std::slice::from_raw_parts(entry.bufptr, entry.buflen as usize) };
            if file.write_all(buf).is_err() {
                return false;
            }
        }
    }
    true
}

/// Load battery-backed RAM from `path`.  Each non-null entry is read
/// sequentially.  Returns `true` on success (short reads are treated
/// as failure for that entry but the function still returns true if
/// the file could be opened; this matches the original C++ behaviour).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cart_battery_load(
    path: *const c_char,
    entries: *mut FceuSaveGameEntry,
    count: usize,
) -> bool {
    let path = unsafe {
        if path.is_null() {
            return false;
        }
        match CStr::from_ptr(path).to_str() {
            Ok(s) => s,
            Err(_) => return false,
        }
    };

    if entries.is_null() || count == 0 {
        return true;
    }
    let entries = unsafe { std::slice::from_raw_parts_mut(entries, count) };

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    for entry in entries {
        if !entry.bufptr.is_null() && entry.buflen > 0 {
            let buf = unsafe { std::slice::from_raw_parts_mut(entry.bufptr, entry.buflen as usize) };
            // Original C++ warns on short read but continues.
            let _ = file.read_exact(buf);
        }
    }
    true
}

/// Zero-fill all save-game buffers.  Does **not** invoke C++ reset
/// callbacks; the caller must handle those separately.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_cart_battery_clear(
    entries: *const FceuSaveGameEntry,
    count: usize,
) {
    if entries.is_null() || count == 0 {
        return;
    }
    let entries = unsafe { std::slice::from_raw_parts(entries, count) };

    for entry in entries {
        if !entry.bufptr.is_null() && entry.buflen > 0 {
            let buf = unsafe { std::slice::from_raw_parts_mut(entry.bufptr, entry.buflen as usize) };
            buf.fill(0);
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_uppow2_internal() {
        assert_eq!(uppow2(0), 0);
        assert_eq!(uppow2(1), 1);
        assert_eq!(uppow2(2), 2);
        assert_eq!(uppow2(3), 4);
        assert_eq!(uppow2(15), 16);
        assert_eq!(uppow2(16), 16);
        assert_eq!(uppow2(17), 32);
        assert_eq!(uppow2(0x8000_0000), 0x8000_0000);
    }

    #[test]
    fn test_rom_sizes_ines1_basic() {
        let mut out = FceuRomSizes {
            rom_size_16kb: 0,
            vrom_size_8kb: 0,
            chrram_size: 0,
            rom_size_raw: 0,
        };
        let header = FceuInesHeader {
            id: [0x4E, 0x45, 0x53, 0x1A],
            rom_size: 2,
            vrom_size: 1,
            rom_type: 0,
            rom_type2: 0,
            rom_type3: 0,
            upper_rom_vrom_size: 0,
            ram_size: 0,
            vram_size: 0,
            tv_system: 0,
            vs_hardware: 0,
            misc_roms: 0,
            expansion: 0,
        };
        assert!(fceux11_rust_cart_compute_rom_sizes(&header, false, &mut out));
        assert_eq!(out.rom_size_16kb, 2); // uppow2(2) = 2
        assert_eq!(out.vrom_size_8kb, 1); // uppow2(1) = 1
        assert_eq!(out.chrram_size, -1);
    }

    #[test]
    fn test_rom_sizes_ines1_zero_prg() {
        let mut out = FceuRomSizes {
            rom_size_16kb: 0,
            vrom_size_8kb: 0,
            chrram_size: 0,
            rom_size_raw: 0,
        };
        let header = FceuInesHeader {
            id: [0x4E, 0x45, 0x53, 0x1A],
            rom_size: 0,
            vrom_size: 0,
            rom_type: 0,
            rom_type2: 0,
            rom_type3: 0,
            upper_rom_vrom_size: 0,
            ram_size: 0,
            vram_size: 0,
            tv_system: 0,
            vs_hardware: 0,
            misc_roms: 0,
            expansion: 0,
        };
        assert!(fceux11_rust_cart_compute_rom_sizes(&header, false, &mut out));
        assert_eq!(out.rom_size_16kb, 256); // special case for iNES1
        assert_eq!(out.vrom_size_8kb, 0);
    }

    #[test]
    fn test_rom_sizes_ines2_simple() {
        let mut out = FceuRomSizes {
            rom_size_16kb: 0,
            vrom_size_8kb: 0,
            chrram_size: 0,
            rom_size_raw: 0,
        };
        let header = FceuInesHeader {
            id: [0x4E, 0x45, 0x53, 0x1A],
            rom_size: 1,
            vrom_size: 1,
            rom_type: 0x08, // NES2.0 marker
            rom_type2: 0x08,
            rom_type3: 0,
            upper_rom_vrom_size: 0x12, // PRG upper=1, CHR upper=2
            ram_size: 0,
            vram_size: 0,
            tv_system: 0,
            vs_hardware: 0,
            misc_roms: 0,
            expansion: 0,
        };
        assert!(fceux11_rust_cart_compute_rom_sizes(&header, true, &mut out));
        // upper_rom_vrom_size = 0x12: PRG upper nibble = 2, CHR upper nibble = 1
        assert_eq!(out.rom_size_16kb, uppow2(1 | (2 << 8))); // 513 -> 1024
        assert_eq!(out.vrom_size_8kb, uppow2(1 | (1 << 8))); // 257 -> 512
    }

    #[test]
    fn test_rom_sizes_ines2_exponent_multiplier() {
        let mut out = FceuRomSizes {
            rom_size_16kb: 0,
            vrom_size_8kb: 0,
            chrram_size: 0,
            rom_size_raw: 0,
        };
        // PRG: exponent=3, multiplier=3  => (1<<3)*3 = 24 >> 14 = 0
        // CHR: exponent=3, multiplier=3  => (1<<3)*3 = 24 >> 13 = 0
        let header = FceuInesHeader {
            id: [0x4E, 0x45, 0x53, 0x1A],
            rom_size: (3 << 2) | 1, // e=3, m=3 (binary 0b1111... wait, rom_size = (e<<2)|(m-1)/2 ?)
            // Actually: rom_size[7:2] = exponent, rom_size[1:0] = (multiplier-1)/2
            // For e=3, m=3: rom_size = (3<<2) | ((3-1)/2) = 12 | 1 = 13 = 0x0D
            vrom_size: 0x0D,
            rom_type: 0x08,
            rom_type2: 0x08,
            rom_type3: 0,
            upper_rom_vrom_size: 0xFF, // both upper nybbles = 0xF => exponent-multiplier
            ram_size: 0,
            vram_size: 0,
            tv_system: 0,
            vs_hardware: 0,
            misc_roms: 0,
            expansion: 0,
        };
        assert!(fceux11_rust_cart_compute_rom_sizes(&header, true, &mut out));
        assert_eq!(out.rom_size_16kb, ((1u32 << 3) * 3) >> 14);
        assert_eq!(out.vrom_size_8kb, ((1u32 << 3) * 3) >> 13);
    }

    #[test]
    fn test_chrram_size_mapper_defaults() {
        assert_eq!(fceux11_rust_cart_compute_chrram_size(0, false, 0, 0, false), 8 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(13, false, 0, 0, false), 16 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(6, false, 0, 0, false), 32 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(29, false, 0, 0, false), 32 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(30, false, 0, 0, false), 32 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(45, false, 0, 0, false), 32 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(96, false, 0, 0, false), 32 * 1024);
        assert_eq!(fceux11_rust_cart_compute_chrram_size(176, false, 0, 0, false), 128 * 1024);
    }

    #[test]
    fn test_chrram_size_with_vrom() {
        assert_eq!(fceux11_rust_cart_compute_chrram_size(0, false, 0, 0, true), -1);
    }

    #[test]
    fn test_chrram_size_nes2() {
        assert_eq!(fceux11_rust_cart_compute_chrram_size(0, true, 4096, 8192, false), 12288);
    }

    #[test]
    fn test_battery_save_load_roundtrip() {
        let tmp = std::env::temp_dir().join("fceux11_test_battery.sav");
        let _ = std::fs::remove_file(&tmp);

        let mut buf1 = [1u8, 2, 3, 4];
        let mut buf2 = [5u8, 6, 7];
        let entries_save = [
            FceuSaveGameEntry { bufptr: buf1.as_mut_ptr(), buflen: buf1.len() as u32 },
            FceuSaveGameEntry { bufptr: buf2.as_mut_ptr(), buflen: buf2.len() as u32 },
        ];

        let path = CString::new(tmp.to_str().unwrap()).unwrap();
        assert!(fceux11_rust_cart_battery_save(path.as_ptr(), entries_save.as_ptr(), entries_save.len()));

        let mut buf1r = [0u8; 4];
        let mut buf2r = [0u8; 3];
        let mut entries_load = [
            FceuSaveGameEntry { bufptr: buf1r.as_mut_ptr(), buflen: buf1r.len() as u32 },
            FceuSaveGameEntry { bufptr: buf2r.as_mut_ptr(), buflen: buf2r.len() as u32 },
        ];

        assert!(fceux11_rust_cart_battery_load(path.as_ptr(), entries_load.as_mut_ptr(), entries_load.len()));
        assert_eq!(buf1r, [1, 2, 3, 4]);
        assert_eq!(buf2r, [5, 6, 7]);

        // Clear
        fceux11_rust_cart_battery_clear(entries_load.as_ptr(), entries_load.len());
        assert_eq!(buf1r, [0, 0, 0, 0]);
        assert_eq!(buf2r, [0, 0, 0]);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_battery_null_safety() {
        assert!(fceux11_rust_cart_battery_save(std::ptr::null(), std::ptr::null(), 0) == false);
        assert!(fceux11_rust_cart_battery_load(std::ptr::null(), std::ptr::null_mut(), 0) == false);
        // clear with null / zero count should not crash
        fceux11_rust_cart_battery_clear(std::ptr::null(), 0);
    }
}
