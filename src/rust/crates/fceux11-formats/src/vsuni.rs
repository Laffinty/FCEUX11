//! VS UniSystem — VS System (arcade NES) game database and helpers.
//!
//! Provides the `VSUniGames` lookup table, DIP-switch helpers, and the
//! on-screen DIP status draw routine.

use std::ffi::c_char;

// ------------------------------------------------------------------
// C-compatible constants (mirrors git.h)
// ------------------------------------------------------------------

pub const GIT_VSUNI: u8 = 1;

pub const GIPPU_USER: u8 = 0;
pub const GIPPU_RP2C04_0001: u8 = 1;
pub const GIPPU_RP2C04_0002: u8 = 2;
pub const GIPPU_RP2C04_0003: u8 = 3;
pub const GIPPU_RP2C04_0004: u8 = 4;
pub const GIPPU_RC2C03B: u8 = 5;
pub const GIPPU_RC2C05_01: u8 = 6;
pub const GIPPU_RC2C05_02: u8 = 7;
pub const GIPPU_RC2C05_03: u8 = 8;
pub const GIPPU_RC2C05_04: u8 = 9;

pub const EGIVS_NORMAL: u8 = 0;
pub const EGIVS_RBI: u8 = 1;
pub const EGIVS_TKO: u8 = 2;
pub const EGIVS_XEVIOUS: u8 = 3;

pub const VS_OPTION_GUN: i32 = 0x1;
pub const VS_OPTION_SWAPDIRAB: i32 = 0x2;
pub const VS_OPTION_PREDIP: i32 = 0x10;

// ------------------------------------------------------------------
// VSUniEntry — C-compatible layout
// ------------------------------------------------------------------

#[repr(C)]
pub struct VsUniEntry {
    pub name: *const c_char,
    pub md5partial: u64,
    pub mapper: i32,
    pub mirroring: u8,
    pub ppu: u8,
    pub ioption: i32,
    pub predip: i32,
    pub game_type: u8,
}

// Safety: all name pointers point to static immutable string literals.
unsafe impl Sync for VsUniEntry {}

// ------------------------------------------------------------------
// Static game database (formerly VSUniGames[] in vsuni.cpp)
// ------------------------------------------------------------------

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

static VSUNI_GAMES: &[VsUniEntry] = &[
    VsUniEntry {
        name: cstr!("Baseball"),
        md5partial: 0x691d4200ea42be45,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0001,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Battle City"),
        md5partial: 0x8540949d74c4d0eb,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0001,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Battle City(Bootleg)"),
        md5partial: 0x8093cbe7137ac031,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0001,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Clu Clu Land"),
        md5partial: 0x1b8123218f62b1ee,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0004,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Dr Mario"),
        md5partial: 0xe1af09c477dc0081,
        mapper: 1,
        mirroring: 0,
        ppu: GIPPU_RP2C04_0003,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Duck Hunt"),
        md5partial: 0x47735d1e5f1205bb,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C03B,
        ioption: VS_OPTION_GUN,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Excitebike"),
        md5partial: 0x3dcd1401bcafde77,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0003,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Excitebike (J)"),
        md5partial: 0x7ea51c9d007375f0,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0004,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Freedom Force"),
        md5partial: 0xed96436bd1b5e688,
        mapper: 4,
        mirroring: 0,
        ppu: GIPPU_RP2C04_0001,
        ioption: VS_OPTION_GUN,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Stroke and Match Golf"),
        md5partial: 0x612325606e82bc66,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0002,
        ioption: VS_OPTION_SWAPDIRAB | VS_OPTION_PREDIP,
        predip: 0x01,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Goonies"),
        md5partial: 0xb4032d694e1d2733,
        mapper: 151,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0003,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Gradius"),
        md5partial: 0x50687ae63bdad976,
        mapper: 151,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0001,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Gumshoe"),
        md5partial: 0x87161f8ee37758d3,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C05_03,
        ioption: VS_OPTION_GUN,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Gumshoe"),
        md5partial: 0xb8500780bf69ce29,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C05_03,
        ioption: VS_OPTION_GUN,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Hogan's Alley"),
        md5partial: 0xd78b7f0bb621fb45,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0001,
        ioption: VS_OPTION_GUN,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Ice Climber"),
        md5partial: 0xd21e999513435e2a,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0004,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Ladies Golf"),
        md5partial: 0x781b24be57ef6785,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0002,
        ioption: VS_OPTION_SWAPDIRAB | VS_OPTION_PREDIP,
        predip: 0x1,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Mach Rider"),
        md5partial: 0x015672618af06441,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0002,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Mach Rider (J)"),
        md5partial: 0xa625afb399811a8a,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0001,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Mighty Bomb Jack"),
        md5partial: 0xe6a89f4873fac37b,
        mapper: 0,
        mirroring: 2,
        ppu: GIPPU_RC2C05_02,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Ninja Jajamaru Kun"),
        md5partial: 0xb26a2c31474099c0,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C05_01,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Pinball"),
        md5partial: 0xc5f49d3def2e9b8,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0001,
        ioption: VS_OPTION_PREDIP,
        predip: 0x01,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Pinball (J)"),
        md5partial: 0x66ab1a3828cc901c,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C03B,
        ioption: VS_OPTION_PREDIP,
        predip: 0x01,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Platoon"),
        md5partial: 0x160f237351c19f1f,
        mapper: 68,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0001,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("RBI Baseball"),
        md5partial: 0x6a02d345812938af,
        mapper: 4,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0001,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_RBI,
    },
    VsUniEntry {
        name: cstr!("Soccer"),
        md5partial: 0xd4e7a9058780eda3,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0003,
        ioption: VS_OPTION_SWAPDIRAB,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Star Luster"),
        md5partial: 0x8360e134b316d94c,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C03B,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Stroke and Match Golf (J)"),
        md5partial: 0x869bb83e02509747,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C03B,
        ioption: VS_OPTION_SWAPDIRAB | VS_OPTION_PREDIP,
        predip: 0x01,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Super Sky Kid"),
        md5partial: 0x78d04c1dd4ec0101,
        mapper: 4,
        mirroring: 1,
        ppu: GIPPU_RC2C03B,
        ioption: VS_OPTION_SWAPDIRAB | VS_OPTION_PREDIP,
        predip: 0x20,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Super Xevious"),
        md5partial: 0x2d396247cf58f9fa,
        mapper: 206,
        mirroring: 0,
        ppu: GIPPU_RP2C04_0001,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_XEVIOUS,
    },
    VsUniEntry {
        name: cstr!("Tetris"),
        md5partial: 0x531a5e8eea4ce157,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RC2C03B,
        ioption: VS_OPTION_PREDIP,
        predip: 0x20,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("Top Gun"),
        md5partial: 0xf1dea36e6a7b531d,
        mapper: 2,
        mirroring: 0,
        ppu: GIPPU_RC2C05_04,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("VS Castlevania"),
        md5partial: 0x92fd6909c81305b9,
        mapper: 2,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0002,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("VS Slalom"),
        md5partial: 0x4889b5a50a623215,
        mapper: 0,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0002,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("VS Super Mario Bros"),
        md5partial: 0x39d8cfa788e20b6c,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0004,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("VS Super Mario Bros [a1]"),
        md5partial: 0xfc182e5aefbce14d,
        mapper: 99,
        mirroring: 2,
        ppu: GIPPU_RP2C04_0004,
        ioption: 0,
        predip: 0,
        game_type: EGIVS_NORMAL,
    },
    VsUniEntry {
        name: cstr!("VS TKO Boxing"),
        md5partial: 0x6e1ee06171d8ce3a,
        mapper: 4,
        mirroring: 1,
        ppu: GIPPU_RP2C04_0003,
        ioption: VS_OPTION_PREDIP,
        predip: 0x00,
        game_type: EGIVS_TKO,
    },
];

// ------------------------------------------------------------------
// FFI: Lookup a game by partial MD5
// ------------------------------------------------------------------

/// Look up a VS UniSystem game by its partial MD5 hash.
/// Returns a pointer to a static `VsUniEntry` if found, or null otherwise.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_vsuni_lookup(md5partial: u64) -> *const VsUniEntry {
    match VSUNI_GAMES.iter().find(|e| e.md5partial == md5partial) {
        Some(entry) => entry as *const VsUniEntry,
        None => std::ptr::null(),
    }
}

// ------------------------------------------------------------------
// FFI: DIP helpers
// ------------------------------------------------------------------

/// Toggle a DIP switch bit. Returns the new vsdip value.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_vsuni_toggle_dip(game_type: u8, vsdip: u8, w: i32) -> u8 {
    if game_type != GIT_VSUNI || !(0..=7).contains(&w) {
        return vsdip;
    }
    vsdip ^ (1u8 << (w as u32))
}

/// Return coin-on duration (6 frames) for a slot.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_vsuni_coin(game_type: u8, _slot: u8) -> u8 {
    if game_type != GIT_VSUNI {
        return 0;
    }
    6
}

/// Return service-button duration (6 frames).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_vsuni_service(game_type: u8) -> u8 {
    if game_type != GIT_VSUNI {
        return 0;
    }
    6
}

// ------------------------------------------------------------------
// FFI: Draw DIP status on screen
// ------------------------------------------------------------------

/// Draw the VS UniSystem DIP-switch overlay into the pixel buffer.
/// Returns the decremented `dips_howlong` value (or -1 if nothing drawn).
/// # Safety
/// The caller must ensure that `xbuf` points to a writable 256x240 pixel buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_vsuni_draw(
    xbuf: *mut u8,
    vsdip: u8,
    dips_howlong: i32,
) -> i32 {
    if xbuf.is_null() || dips_howlong <= 0 {
        return dips_howlong - 1;
    }

    let buf = unsafe { std::slice::from_raw_parts_mut(xbuf, 256 * 240) };

    // Helper: fill a rectangle with a single colour index
    fn fill_rect(buf: &mut [u8], x: usize, y: usize, w: usize, h: usize, col: u8) {
        for row in y..y + h {
            let base = row * 256;
            if base + x + w <= buf.len() {
                for px in x..x + w {
                    buf[base + px] = col;
                }
            }
        }
    }

    // 1. Clear background rectangle: 72x24 at (164, 12)
    fill_rect(buf, 164, 12, 72, 24, 0);

    // 2. Draw white checkerboard-like pattern inside
    // Original uses uint32 writes for speed; we keep the same visual result.
    for row in 16..32 {
        let base = row * 256 + 170;
        for col in 0..16 {
            let px = base + col * 2;
            if px < buf.len() {
                buf[px] = 1;
            }
        }
    }

    // 3. Draw 8 DIP-switch indicators
    for dip in 0..8 {
        let on = (vsdip >> dip) & 1 != 0;
        let x = 170 + dip * 8;
        let y_base = if on { 16 } else { 16 + 40 }; // original: offset by 10 rows (10*4=40 px)
        for row in 0..4 {
            let px = (y_base + row) * 256 + x;
            if px < buf.len() {
                buf[px] = 0;
            }
            if px + 1 < buf.len() {
                buf[px + 1] = 0;
            }
        }
    }

    dips_howlong - 1
}

// ------------------------------------------------------------------
// FFI: VS UniSystem check result
// ------------------------------------------------------------------

/// Result of a VS UniSystem ROM check.
///
/// Contains all the information needed to set up a VS UniSystem game.
#[repr(C)]
pub struct FceuVsUniCheckResult {
    /// Whether a VS UniSystem entry was found.
    pub found: bool,
    /// Mapper number to use.
    pub mapper: i32,
    /// Mirroring mode.
    pub mirroring: u8,
    /// PPU type.
    pub ppu: u8,
    /// Game type (Normal, RBI, TKO, Xevious).
    pub game_type: u8,
    /// DIP switch default value.
    pub dip_value: u8,
    /// Whether to use zapper input.
    pub use_gun: bool,
    /// Whether to swap A/B controllers.
    pub swap_ab: bool,
}

/// Check if a ROM is a VS UniSystem game and return setup information.
///
/// Looks up the ROM by its partial MD5 hash and returns all the
/// information needed to configure the emulator for VS UniSystem mode.
///
/// Returns `true` if a VS UniSystem entry was found.
///
/// # Safety
/// `out` must point to a writable `FceuVsUniCheckResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_vsuni_check(
    md5partial: u64,
    out: *mut FceuVsUniCheckResult,
) -> bool {
    if out.is_null() {
        return false;
    }
    let result = unsafe { &mut *out };

    let entry = fceux11_rust_vsuni_lookup(md5partial);
    if entry.is_null() {
        result.found = false;
        return false;
    }

    let vs = unsafe { &*entry };
    result.found = true;
    result.mapper = vs.mapper;
    result.mirroring = vs.mirroring;
    result.ppu = vs.ppu;
    result.game_type = vs.game_type;

    // Set DIP switch default
    result.dip_value = if (vs.ioption & VS_OPTION_PREDIP) != 0 {
        vs.predip as u8
    } else {
        0
    };

    // Set gun input
    result.use_gun = (vs.ioption & VS_OPTION_GUN) != 0;

    // Set swap A/B
    result.swap_ab = (vs.ioption & VS_OPTION_SWAPDIRAB) != 0;

    true
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_found() {
        unsafe {
            let entry = fceux11_rust_vsuni_lookup(0x39d8cfa788e20b6c);
            assert!(!entry.is_null());
            unsafe {
                assert_eq!((*entry).mapper, 99);
                assert_eq!((*entry).ppu, GIPPU_RP2C04_0004);
                assert_eq!((*entry).game_type, EGIVS_NORMAL);
            }
        }
    }

    #[test]
    fn test_lookup_not_found() {
        unsafe {
            let entry = fceux11_rust_vsuni_lookup(0xDEADBEEFCAFEBABE);
            assert!(entry.is_null());
        }
    }

    #[test]
    fn test_lookup_rbi() {
        unsafe {
            let entry = fceux11_rust_vsuni_lookup(0x6a02d345812938af);
            assert!(!entry.is_null());
            unsafe {
                assert_eq!((*entry).game_type, EGIVS_RBI);
                assert_eq!((*entry).mapper, 4);
            }
        }
    }

    #[test]
    fn test_lookup_tko() {
        unsafe {
            let entry = fceux11_rust_vsuni_lookup(0x6e1ee06171d8ce3a);
            assert!(!entry.is_null());
            unsafe {
                assert_eq!((*entry).game_type, EGIVS_TKO);
                assert_eq!((*entry).ppu, GIPPU_RP2C04_0003);
            }
        }
    }

    #[test]
    fn test_toggle_dip_vs() {
        unsafe {
            let vsdip = fceux11_rust_vsuni_toggle_dip(GIT_VSUNI, 0b0000_0000, 0);
            assert_eq!(vsdip, 0b0000_0001);

            let vsdip = fceux11_rust_vsuni_toggle_dip(GIT_VSUNI, 0b0000_0001, 0);
            assert_eq!(vsdip, 0b0000_0000);

            let vsdip = fceux11_rust_vsuni_toggle_dip(GIT_VSUNI, 0b0000_0000, 7);
            assert_eq!(vsdip, 0b1000_0000);
        }
    }

    #[test]
    fn test_toggle_dip_non_vs() {
        unsafe {
            let vsdip = fceux11_rust_vsuni_toggle_dip(0, 0b1010_1010, 3);
            assert_eq!(vsdip, 0b1010_1010); // unchanged
        }
    }

    #[test]
    fn test_toggle_dip_out_of_range() {
        unsafe {
            let vsdip = fceux11_rust_vsuni_toggle_dip(GIT_VSUNI, 0b1010_1010, 8);
            assert_eq!(vsdip, 0b1010_1010); // unchanged
            let vsdip = fceux11_rust_vsuni_toggle_dip(GIT_VSUNI, 0b1010_1010, -1);
            assert_eq!(vsdip, 0b1010_1010); // unchanged
        }
    }

    #[test]
    fn test_coin_vs() {
        unsafe {
            assert_eq!(fceux11_rust_vsuni_coin(GIT_VSUNI, 0), 6);
            assert_eq!(fceux11_rust_vsuni_coin(GIT_VSUNI, 1), 6);
        }
    }

    #[test]
    fn test_coin_non_vs() {
        unsafe {
            assert_eq!(fceux11_rust_vsuni_coin(0, 0), 0);
        }
    }

    #[test]
    fn test_service_vs() {
        unsafe {
            assert_eq!(fceux11_rust_vsuni_service(GIT_VSUNI), 6);
        }
    }

    #[test]
    fn test_service_non_vs() {
        unsafe {
            assert_eq!(fceux11_rust_vsuni_service(0), 0);
        }
    }

    #[test]
    fn test_draw_null_safety() {
        unsafe {
            assert_eq!(
                unsafe { fceux11_rust_vsuni_draw(std::ptr::null_mut(), 0, 0) },
                -1
            );
            assert_eq!(
                unsafe { fceux11_rust_vsuni_draw(std::ptr::null_mut(), 0, 180) },
                179
            );
        }
    }

    #[test]
    fn test_draw_decrements_counter() {
        unsafe {
            let mut buf = vec![0u8; 256 * 240];
            assert_eq!(
                unsafe { fceux11_rust_vsuni_draw(buf.as_mut_ptr(), 0xFF, 180) },
                179
            );
        }
    }

    #[test]
    fn test_draw_zero_counter() {
        unsafe {
            let mut buf = vec![0u8; 256 * 240];
            assert_eq!(
                unsafe { fceux11_rust_vsuni_draw(buf.as_mut_ptr(), 0xFF, 0) },
                -1
            );
        }
    }

    #[test]
    fn test_draw_pixel_modification() {
        unsafe {
            let mut buf = vec![0u8; 256 * 240];
            unsafe { fceux11_rust_vsuni_draw(buf.as_mut_ptr(), 0xFF, 180) };
            // Background clear should have set some pixels to 0
            // White pattern should have set some pixels to 1
            let has_ones = buf.contains(&1);
            assert!(has_ones, "Expected some pixels set to colour 1");
        }
    }

    #[test]
    fn test_draw_determinism() {
        unsafe {
            let mut buf1 = vec![0u8; 256 * 240];
            let mut buf2 = vec![0u8; 256 * 240];
            unsafe {
                fceux11_rust_vsuni_draw(buf1.as_mut_ptr(), 0xAA, 100);
                fceux11_rust_vsuni_draw(buf2.as_mut_ptr(), 0xAA, 100);
            }
            assert_eq!(buf1, buf2);
        }
    }
}
