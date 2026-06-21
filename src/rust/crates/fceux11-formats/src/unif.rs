//! UNIF ROM format — board-name-to-flags database and parsing helpers.
//!
//! Replaces the static `bmap[]` table formerly in `src/unif.cpp`.
//! C++ retains the name→mapper-init-function mapping; Rust owns the
//! name→hardware-flags mapping (CHR-RAM size, forced mirroring, etc.).

use std::ffi::{CStr, c_char};

// ------------------------------------------------------------------
// C-compatible constants
// ------------------------------------------------------------------

pub const BMCFLAG_FORCE4: i32 = 0x01;
pub const BMCFLAG_16KCHRR: i32 = 0x02;
pub const BMCFLAG_32KCHRR: i32 = 0x04;
pub const BMCFLAG_128KCHRR: i32 = 0x08;
pub const BMCFLAG_256KCHRR: i32 = 0x10;

// ------------------------------------------------------------------
// UnifBoardInfo — C-compatible layout
// ------------------------------------------------------------------

#[repr(C)]
pub struct UnifBoardInfo {
    pub name: *const c_char,
    pub flags: i32,
}

// Safety: all name pointers point to static immutable string literals.
unsafe impl Sync for UnifBoardInfo {}

// ------------------------------------------------------------------
// Helper macro for static C strings
// ------------------------------------------------------------------

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// ------------------------------------------------------------------
// Static board database (formerly bmap[] in src/unif.cpp)
// ------------------------------------------------------------------

static UNIF_BOARDS: &[UnifBoardInfo] = &[
    UnifBoardInfo {
        name: cstr!("11160"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("12-IN-1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("13in1JY110"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("190in1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("22211"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("3D-BLOCK"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("411120-C"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("42in1ResetSwitch"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("43272"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("603-5052"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("64in1NoRepeat"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("70in1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("70in1B"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("810544-C-A1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("8157"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("8237"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("8237A"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("830118C"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("A65AS"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("AC08"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("ANROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("AX5705"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("BB"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("BS-5"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("CC-21"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("CITYFIGHT"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("10-24-C-A1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("CNROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("CPROM"),
        flags: BMCFLAG_16KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("D1038"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("DANCE"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("DANCE2000"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("DREAMTECH01"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("EDU2000"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("EKROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("ELROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("ETROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("EWROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("FK23C"),
        flags: BMCFLAG_256KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("FK23CA"),
        flags: BMCFLAG_256KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("FS304"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("G-146"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("GK-192"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("GS-2004"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("GS-2013"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Ghostbusters63in1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("H2288"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("HKROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KOF97"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KONAMI-QTAI"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7010"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7012"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7013B"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7016"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7017"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7030"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7031"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7032"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7037"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("KS7057"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("LE05"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("LH10"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("LH32"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("LH53"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("MALISB"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("MARIO1-MALEE2"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("MHROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("N625092"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("NROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("NROM-128"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("NROM-256"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("NTBROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("NTD-03"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("NovelDiamond9999999in1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("OneBus"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("PEC-586"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("RET-CUFROM"),
        flags: BMCFLAG_32KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("RROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("RROM-128"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-002"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-0036"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-0037"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-009"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-016-1M"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-72007"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-72008"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SA-9602B"),
        flags: BMCFLAG_32KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("SA-NROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SAROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SBROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SC-127"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SCROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SEROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SGROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SHERO"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SKROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SL12"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SL1632"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SL1ROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SLROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SMB2J"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SNROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SOROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SSS-NROM-256"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SUNSOFT_UNROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Sachen-74LS374N"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Sachen-74LS374NA"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Sachen-8259A"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Sachen-8259B"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Sachen-8259C"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Sachen-8259D"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Super24in1SC03"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SuperHIK8in1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("Supervision16in1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("T-227-1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("T-230"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("T-262"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TBROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TC-U01-1.5M"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TEK90"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TEROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TF1201"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TFROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TGROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TKROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TKSROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TLROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TLSROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TQROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TR1ROM"),
        flags: BMCFLAG_FORCE4,
    },
    UnifBoardInfo {
        name: cstr!("TSROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("TVROM"),
        flags: BMCFLAG_FORCE4,
    },
    UnifBoardInfo {
        name: cstr!("Transformer"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("UNROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("UNROM-512-8"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("UNROM-512-16"),
        flags: BMCFLAG_16KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("UNROM-512-32"),
        flags: BMCFLAG_32KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("UOROM"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("VRC7"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("YOKO"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("SB-2000"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("COOLBOY"),
        flags: BMCFLAG_256KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("158B"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("DRAGONFIGHTER"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("EH8813A"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("HP898F"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("F-15"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("RT-01"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("81-01-31-C"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("8-IN-1"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("80013-B"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("HPxx"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("MINDKIDS"),
        flags: BMCFLAG_256KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("FNS"),
        flags: BMCFLAG_16KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("BS-400R"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("BS-4040R"),
        flags: 0,
    },
    UnifBoardInfo {
        name: cstr!("COOLGIRL"),
        flags: BMCFLAG_256KCHRR,
    },
    UnifBoardInfo {
        name: cstr!("JC-016-2"),
        flags: 0,
    },
];

// ------------------------------------------------------------------
// Internal helpers
// ------------------------------------------------------------------

fn lookup_board(name: &str) -> Option<&'static UnifBoardInfo> {
    UNIF_BOARDS.iter().find(|b| {
        // SAFETY: all entries use cstr! with a valid NUL-terminated literal.
        let bname = unsafe { CStr::from_ptr(b.name) };
        bname.to_str().unwrap_or("") == name
    })
}

/// Compute CHR-RAM size in KiB from board flags.
pub fn chrram_size_kb(flags: i32) -> u32 {
    if flags & BMCFLAG_16KCHRR != 0 {
        16
    } else if flags & BMCFLAG_32KCHRR != 0 {
        32
    } else if flags & BMCFLAG_128KCHRR != 0 {
        128
    } else if flags & BMCFLAG_256KCHRR != 0 {
        256
    } else {
        8
    }
}

// ------------------------------------------------------------------
// FFI
// ------------------------------------------------------------------

/// Look up a UNIF board by name.
/// Returns a pointer to a static `UnifBoardInfo` if found, or null otherwise.
/// # Safety
/// The caller must ensure that `name` is a valid, null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_unif_lookup_board(
    name: *const c_char,
) -> *const UnifBoardInfo {
    if name.is_null() {
        return std::ptr::null();
    }
    let name = unsafe { CStr::from_ptr(name) };
    let name_str = name.to_str().unwrap_or("");
    match lookup_board(name_str) {
        Some(b) => b as *const UnifBoardInfo,
        None => std::ptr::null(),
    }
}

/// Return the hardware flags for a given UNIF board name.
/// Returns -1 if the board is not found.
/// # Safety
/// The caller must ensure that `name` is a valid, null-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_unif_board_flags(name: *const c_char) -> i32 {
    if name.is_null() {
        return -1;
    }
    let name = unsafe { CStr::from_ptr(name) };
    let name_str = name.to_str().unwrap_or("");
    match lookup_board(name_str) {
        Some(b) => b.flags,
        None => -1,
    }
}

/// Compute CHR-RAM size in bytes from board flags.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_unif_chrram_size(flags: i32) -> u32 {
    chrram_size_kb(flags) << 10
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_lookup_found() {
        unsafe {
            let name = CString::new("FK23C").unwrap();
            let info = unsafe { fceux11_rust_unif_lookup_board(name.as_ptr()) };
            assert!(!info.is_null());
            unsafe {
                assert_eq!((*info).flags, BMCFLAG_256KCHRR);
            }
        }
    }

    #[test]
    fn test_lookup_not_found() {
        unsafe {
            let name = CString::new("NONEXISTENT").unwrap();
            let info = unsafe { fceux11_rust_unif_lookup_board(name.as_ptr()) };
            assert!(info.is_null());
        }
    }

    #[test]
    fn test_board_flags() {
        unsafe {
            let name = CString::new("CPROM").unwrap();
            assert_eq!(
                unsafe { fceux11_rust_unif_board_flags(name.as_ptr()) },
                BMCFLAG_16KCHRR
            );

            let name = CString::new("NROM").unwrap();
            assert_eq!(unsafe { fceux11_rust_unif_board_flags(name.as_ptr()) }, 0);

            let name = CString::new("TR1ROM").unwrap();
            assert_eq!(
                unsafe { fceux11_rust_unif_board_flags(name.as_ptr()) },
                BMCFLAG_FORCE4
            );
        }
    }

    #[test]
    fn test_board_flags_unknown() {
        unsafe {
            let name = CString::new("UNKNOWN").unwrap();
            assert_eq!(unsafe { fceux11_rust_unif_board_flags(name.as_ptr()) }, -1);
        }
    }

    #[test]
    fn test_chrram_size() {
        unsafe {
            assert_eq!(fceux11_rust_unif_chrram_size(0), 8 * 1024);
            assert_eq!(fceux11_rust_unif_chrram_size(BMCFLAG_16KCHRR), 16 * 1024);
            assert_eq!(fceux11_rust_unif_chrram_size(BMCFLAG_32KCHRR), 32 * 1024);
            assert_eq!(fceux11_rust_unif_chrram_size(BMCFLAG_128KCHRR), 128 * 1024);
            assert_eq!(fceux11_rust_unif_chrram_size(BMCFLAG_256KCHRR), 256 * 1024);
        }
    }

    #[test]
    fn test_chrram_size_combined_flags() {
        unsafe {
            // FORCE4 combined with 256KCHRR
            let flags = BMCFLAG_FORCE4 | BMCFLAG_256KCHRR;
            assert_eq!(fceux11_rust_unif_chrram_size(flags), 256 * 1024);
        }
    }

    #[test]
    fn test_null_safety() {
        unsafe {
            assert!(unsafe { fceux11_rust_unif_lookup_board(std::ptr::null()) }.is_null());
            assert_eq!(
                unsafe { fceux11_rust_unif_board_flags(std::ptr::null()) },
                -1
            );
        }
    }

    #[test]
    fn test_all_boards_have_valid_names() {
        unsafe {
            for board in UNIF_BOARDS.iter() {
                let name = unsafe { CStr::from_ptr(board.name) };
                let s = name.to_str().expect("board name must be valid UTF-8");
                assert!(!s.is_empty(), "board name must not be empty");
            }
        }
    }
}
