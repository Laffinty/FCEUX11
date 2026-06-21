//! iNES ROM format parser — header structures and static correction databases.
//!
//! Replaces the static lookup tables and header-cleanup logic formerly in
//! `src/ines.cpp`. C++ retains `iNESLoad`, `iNES_Init`, and the `bmap[]`
//! function-pointer table; Rust owns all pure-data lookup and parsing logic.

use std::ffi::c_char;

mod ines_data;

// ------------------------------------------------------------------
// iNES header — C-compatible 16-byte layout
// ------------------------------------------------------------------

#[repr(C)]
pub struct FceuInesHeader {
    pub id: [u8; 4],
    pub rom_size: u8,
    pub vrom_size: u8,
    pub rom_type: u8,
    pub rom_type2: u8,
    pub rom_type3: u8,
    pub upper_rom_vrom_size: u8,
    pub ram_size: u8,
    pub vram_size: u8,
    pub tv_system: u8,
    pub vs_hardware: u8,
    pub misc_roms: u8,
    pub expansion: u8,
}

impl FceuInesHeader {
    /// Remove garbage signatures from the header (DiskDude, demiforce, Ni03).
    pub fn cleanup(&mut self) {
        let bytes = self.as_mut_bytes();
        // Offset 0x7 .. 0xF
        if bytes[0x7..0xF].starts_with(b"DiskDude") || bytes[0x7..0x10].starts_with(b"demiforce") {
            bytes[0x7..0x10].fill(0);
        }
        if bytes[0xA..0xE].starts_with(b"Ni03") {
            if bytes[0x7..0xA].starts_with(b"Dis") {
                bytes[0x7..0x10].fill(0);
            } else {
                bytes[0xA..0x10].fill(0);
            }
        }
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self as *mut _ as *mut u8, 16) }
    }
}

// ------------------------------------------------------------------
// FFI: Header cleanup
// ------------------------------------------------------------------

/// Clean garbage signatures out of an iNES header.
/// `header_bytes` must point to at least 16 writable bytes.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_header_cleanup(header_bytes: *mut u8) {
    if header_bytes.is_null() {
        return;
    }
    let hdr = unsafe { &mut *(header_bytes as *mut FceuInesHeader) };
    hdr.cleanup();
}

// ------------------------------------------------------------------
// FFI: Mapper name lookup
// ------------------------------------------------------------------

/// Return the human-readable name for a mapper number.
/// Returns a pointer to a static string, or null if not found.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_ines_mapper_name(mapper_no: i32) -> *const c_char {
    match ines_data::MAPPER_NAMES
        .iter()
        .find(|(n, _)| *n == mapper_no)
    {
        Some(entry) => {
            let name: &str = entry.1;
            // For simplicity we use a thread-local buffer.
            thread_local! {
                static BUF: std::cell::RefCell<[u8; 256]> = const { std::cell::RefCell::new([0u8; 256]) };
            }
            BUF.with(|buf| {
                let mut b = buf.borrow_mut();
                let bytes = name.as_bytes();
                let len = bytes.len().min(255);
                b[..len].copy_from_slice(&bytes[..len]);
                b[len] = 0;
                b.as_ptr() as *const c_char
            })
        }
        None => std::ptr::null(),
    }
}

// ------------------------------------------------------------------
// FFI: not_power2 mapper check
// ------------------------------------------------------------------

/// Returns 1 if the mapper is in the not_power2 list.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_ines_not_power2(mapper_no: i32) -> i32 {
    if ines_data::NOT_POWER2.contains(&mapper_no) {
        1
    } else {
        0
    }
}

// ------------------------------------------------------------------
// FFI: Controller selection by CRC32 (formerly SetInput)
// ------------------------------------------------------------------

/// Look up default input controllers by ROM CRC32.
/// Returns 1 if found, 0 otherwise. Results are written to out_* params.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_lookup_input_crc(
    crc32: u32,
    out_input1: *mut i32,
    out_input2: *mut i32,
    out_inputfc: *mut i32,
) -> i32 {
    if out_input1.is_null() || out_input2.is_null() || out_inputfc.is_null() {
        return 0;
    }
    match ines_data::INPSEL_CRC.iter().find(|e| e.crc32 == crc32) {
        Some(e) => {
            unsafe {
                *out_input1 = e.input1;
                *out_input2 = e.input2;
                *out_inputfc = e.inputfc;
            }
            1
        }
        None => 0,
    }
}

// ------------------------------------------------------------------
// FFI: Controller selection by NES 2.0 expansion byte
// ------------------------------------------------------------------

/// Look up default input controllers by NES 2.0 expansion device byte.
/// Returns 1 if found, 0 otherwise.
/// `out_eoptions_flag` is set to 32768 when expansion == 0x02 (Four-Score hack).
/// `out_vs_cswitch` is set to 1 when expansion == 0x05.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_lookup_input_nes20(
    expansion: u8,
    out_input1: *mut i32,
    out_input2: *mut i32,
    out_inputfc: *mut i32,
    out_eoptions_flag: *mut i32,
    out_vs_cswitch: *mut i32,
) -> i32 {
    if out_input1.is_null()
        || out_input2.is_null()
        || out_inputfc.is_null()
        || out_eoptions_flag.is_null()
        || out_vs_cswitch.is_null()
    {
        return 0;
    }
    unsafe {
        *out_eoptions_flag = if expansion == 0x02 { 32768 } else { 0 };
        *out_vs_cswitch = if expansion == 0x05 { 1 } else { 0 };
    }
    match ines_data::INPSEL_NES20
        .iter()
        .find(|e| e.expansion_id == expansion)
    {
        Some(e) => {
            unsafe {
                *out_input1 = e.input1;
                *out_input2 = e.input2;
                *out_inputfc = e.inputfc;
            }
            1
        }
        None => 0,
    }
}

// ------------------------------------------------------------------
// FFI: Bad ROM check
// ------------------------------------------------------------------

/// Check whether a ROM is known to be bad/corrupt/hacked.
/// Returns a pointer to a static description string, or null if OK.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_ines_check_bad(md5partial: u64) -> *const c_char {
    match ines_data::BAD_ROMS
        .iter()
        .find(|e| e.md5partial == md5partial)
    {
        Some(e) => {
            thread_local! {
                static BUF: std::cell::RefCell<[u8; 256]> = const { std::cell::RefCell::new([0u8; 256]) };
            }
            BUF.with(|buf| {
                let mut b = buf.borrow_mut();
                let bytes = e.name.as_bytes();
                let len = bytes.len().min(255);
                b[..len].copy_from_slice(&bytes[..len]);
                b[len] = 0;
                b.as_ptr() as *const c_char
            })
        }
        None => std::ptr::null(),
    }
}

// ------------------------------------------------------------------
// FFI: ROM hardware correction (formerly CheckHInfo)
// ------------------------------------------------------------------

#[repr(C)]
pub struct FceuInesHInfoResult {
    pub mapper: i32,      // -1 = no change, otherwise new mapper number
    pub mapper_mask: i32, // 0xFF or 0xFFF
    pub mirror: i32,      // -2 = no change, -1 = special handling, otherwise new mirror
    pub force_battery: i32,
    pub clear_vrom: i32,
}

/// Check ROM-correction databases and return recommended fixes.
/// C++ applies the fixes to its global state.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_check_hinfo(
    crc32: u32,
    partialmd5: u64,
    out: *mut FceuInesHInfoResult,
) -> i32 {
    if out.is_null() {
        return 0;
    }
    unsafe {
        (*out).mapper = -1;
        (*out).mapper_mask = 0xFF;
        (*out).mirror = -2;
        (*out).force_battery = 0;
        (*out).clear_vrom = 0;
    }

    // Look up sMasterRomInfo (only used by C++ to populate MasterRomInfoParams)
    // We don't return params here; C++ can query separately if needed.

    // Look up CHINF / ROM_CORRECTIONS
    if let Some(e) = ines_data::ROM_CORRECTIONS.iter().find(|e| e.crc32 == crc32) {
        if e.mapper >= 0 {
            let mask = if e.mapper & 0x1000 != 0 { 0xFFF } else { 0xFF };
            unsafe {
                (*out).mapper = e.mapper & mask;
                (*out).mapper_mask = mask;
            }
            if e.mapper & 0x800 != 0 {
                unsafe {
                    (*out).clear_vrom = 1;
                }
            }
        }
        if e.mirror >= 0 {
            unsafe {
                (*out).mirror = e.mirror;
            }
        }
    }

    // Look up savie whitelist -> force battery bit
    if ines_data::SAVIE_WHITELIST.contains(&partialmd5) {
        unsafe {
            (*out).force_battery = 1;
        }
    }

    // Hard-coded mapper-specific corrections from CheckHInfo
    // (Mapper 118, 24, 26 with four-screen mirroring -> horizontal)
    // These are applied by C++ after receiving the base result.

    1
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::ines_data::*;
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_header_cleanup_diskdude() {
        unsafe {
            let mut h = FceuInesHeader {
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
            // Write "DiskDude" at offset 7
            h.as_mut_bytes()[0x7..0xF].copy_from_slice(b"DiskDude");
            h.cleanup();
            assert_eq!(h.as_mut_bytes()[0x7..0x10], [0; 9]);
        }
    }

    #[test]
    fn test_header_cleanup_demiforce() {
        unsafe {
            let mut h = FceuInesHeader {
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
            h.as_mut_bytes()[0x7..0x10].copy_from_slice(b"demiforce");
            h.cleanup();
            assert_eq!(h.as_mut_bytes()[0x7..0x10], [0; 9]);
        }
    }

    #[test]
    fn test_header_cleanup_ni03() {
        unsafe {
            let mut h = FceuInesHeader {
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
            h.as_mut_bytes()[0xA..0xE].copy_from_slice(b"Ni03");
            h.cleanup();
            assert_eq!(h.as_mut_bytes()[0xA..0x10], [0; 6]);
        }
    }

    #[test]
    fn test_mapper_name_found() {
        unsafe {
            let name = fceux11_rust_ines_mapper_name(0);
            assert!(!name.is_null());
            let s = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
            assert_eq!(s, "NROM");
        }
    }

    #[test]
    fn test_mapper_name_mmc3() {
        unsafe {
            let name = fceux11_rust_ines_mapper_name(4);
            assert!(!name.is_null());
            let s = unsafe { CStr::from_ptr(name) }.to_str().unwrap();
            assert_eq!(s, "MMC3");
        }
    }

    #[test]
    fn test_mapper_name_not_found() {
        unsafe {
            let name = fceux11_rust_ines_mapper_name(9999);
            assert!(name.is_null());
        }
    }

    #[test]
    fn test_not_power2() {
        unsafe {
            assert_eq!(fceux11_rust_ines_not_power2(53), 1);
            assert_eq!(fceux11_rust_ines_not_power2(198), 1);
            assert_eq!(fceux11_rust_ines_not_power2(0), 0);
            assert_eq!(fceux11_rust_ines_not_power2(1), 0);
        }
    }

    #[test]
    fn test_lookup_input_crc_duck_hunt() {
        unsafe {
            let mut i1 = 0;
            let mut i2 = 0;
            let mut ifc = 0;
            let found = unsafe {
                fceux11_rust_ines_lookup_input_crc(0x24598791, &mut i1, &mut i2, &mut ifc)
            };
            assert_eq!(found, 1);
            assert_eq!(i1, SI_GAMEPAD);
            assert_eq!(i2, SI_ZAPPER);
            assert_eq!(ifc, SIFC_NONE);
        }
    }

    #[test]
    fn test_lookup_input_crc_not_found() {
        unsafe {
            let mut i1 = 0;
            let mut i2 = 0;
            let mut ifc = 0;
            let found = unsafe {
                fceux11_rust_ines_lookup_input_crc(0xDEADBEEF, &mut i1, &mut i2, &mut ifc)
            };
            assert_eq!(found, 0);
        }
    }

    #[test]
    fn test_lookup_input_nes20_standard() {
        unsafe {
            let mut i1 = 0;
            let mut i2 = 0;
            let mut ifc = 0;
            let mut eopt = 0;
            let mut vsc = 0;
            let found = unsafe {
                fceux11_rust_ines_lookup_input_nes20(
                    0x01, &mut i1, &mut i2, &mut ifc, &mut eopt, &mut vsc,
                )
            };
            assert_eq!(found, 1);
            assert_eq!(i1, SI_GAMEPAD);
            assert_eq!(i2, SI_GAMEPAD);
            assert_eq!(ifc, SIFC_UNSET);
            assert_eq!(eopt, 0);
            assert_eq!(vsc, 0);
        }
    }

    #[test]
    fn test_lookup_input_nes20_four_score() {
        unsafe {
            let mut i1 = 0;
            let mut i2 = 0;
            let mut ifc = 0;
            let mut eopt = 0;
            let mut vsc = 0;
            let found = unsafe {
                fceux11_rust_ines_lookup_input_nes20(
                    0x02, &mut i1, &mut i2, &mut ifc, &mut eopt, &mut vsc,
                )
            };
            assert_eq!(found, 1);
            assert_eq!(eopt, 32768);
        }
    }

    #[test]
    fn test_check_bad_found() {
        unsafe {
            let msg = fceux11_rust_ines_check_bad(0x1895afc6eef26c7d_u64);
            assert!(!msg.is_null());
            let s = unsafe { CStr::from_ptr(msg) }.to_str().unwrap();
            assert_eq!(s, "Super Mario Bros.");
        }
    }

    #[test]
    fn test_check_bad_not_found() {
        unsafe {
            let msg = fceux11_rust_ines_check_bad(0x0u64);
            assert!(msg.is_null());
        }
    }

    #[test]
    fn test_check_hinfo_force_battery() {
        unsafe {
            let mut res = FceuInesHInfoResult {
                mapper: -1,
                mapper_mask: 0,
                mirror: -2,
                force_battery: 0,
                clear_vrom: 0,
            };
            // partialmd5 for AD&D Heroes of the Lance (first in savie list)
            unsafe { fceux11_rust_ines_check_hinfo(0, 0xc04361e499748382_u64, &mut res) };
            assert_eq!(res.force_battery, 1);
        }
    }

    #[test]
    fn test_check_hinfo_mapper_correction() {
        unsafe {
            let mut res = FceuInesHInfoResult {
                mapper: -1,
                mapper_mask: 0,
                mirror: -2,
                force_battery: 0,
                clear_vrom: 0,
            };
            // Elevator Action -> mapper 0, mirror 0
            unsafe { fceux11_rust_ines_check_hinfo(0xfcdaca80, 0, &mut res) };
            assert_eq!(res.mapper, 0);
            assert_eq!(res.mirror, 0);
        }
    }

    #[test]
    fn test_all_databases_nonempty() {
        unsafe {
            use super::ines_data::*;
            assert!(!MAPPER_NAMES.is_empty());
            assert!(!INPSEL_CRC.is_empty());
            assert!(!INPSEL_NES20.is_empty());
            assert!(!BAD_ROMS.is_empty());
            assert!(!MASTER_ROM_INFO.is_empty());
            assert!(!SAVIE_WHITELIST.is_empty());
            assert!(!ROM_CORRECTIONS.is_empty());
            assert!(!NOT_POWER2.is_empty());
        }
    }
}
