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
/// Returns a pointer to a `&'static CStr`, or null if not found.
///
/// hotfix1 P1-15 (H-15): the previous implementation kept the result in a
/// `thread_local!` cell, so thread A's returned pointer would silently
/// get overwritten the next time thread B called mapper_name(). Even on
/// a single thread, the buffer's lifetime was tied to the calling
/// thread's existence (panics inside the borrow_mut() guard would
/// poison the cell for the rest of the program).
///
/// We replace it with a once-initialized `Vec<(i32, &'static CStr)>`
/// keyed by mapper number. The CStr pointers have `'static` lifetime
/// and a single backing allocation built on first call, so they can be
/// read concurrently from any thread.
fn mapper_name_cache() -> &'static std::sync::RwLock<
    std::collections::HashMap<i32, &'static std::ffi::CStr>,
> {
    use std::collections::HashMap;
    use std::ffi::CStr;
    use std::sync::{OnceLock, RwLock};
    static CACHE: OnceLock<RwLock<HashMap<i32, &'static CStr>>> = OnceLock::new();
    CACHE.get_or_init(|| {
        // hotfix1 P1-15 follow-up: the prior version called
        // CStr::from_bytes_with_nul(s.as_bytes()) on a plain &str,
        // which does NOT include the trailing NUL byte and panicked
        // at first FFI entry with NotNulTerminated. Walk MAPPER_NAMES,
        // convert each entry to CString (which adds the NUL), then
        // leak the boxed CStr so the cache holds &'static CStr.
        // Bounded leak (finite mapper table, ~hundred entries).
        let mut m: HashMap<i32, &'static CStr> =
            HashMap::with_capacity(ines_data::MAPPER_NAMES.len());
        for (n, s) in ines_data::MAPPER_NAMES {
            let owned = std::ffi::CString::new(*s)
                .expect("static mapper name contained interior NUL");
            m.insert(*n, Box::leak(owned.into_boxed_c_str()));
        }
        RwLock::new(m)
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_ines_mapper_name(mapper_no: i32) -> *const c_char {
    let guard = match mapper_name_cache().read() {
        Ok(g) => g,
        // POISON: another thread panicked while holding the write lock;
        // fall through with the still-readable contents rather than abort.
        Err(poisoned) => poisoned.into_inner(),
    };
    match guard.get(&mapper_no) {
        Some(s) => s.as_ptr(),
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
// FFI: Apply ROM corrections to parse result
// ------------------------------------------------------------------

/// Master ROM info result for C++ compatibility.
#[repr(C)]
pub struct FceuMasterRomInfoResult {
    pub found: bool,
    pub bonus: i32,
    pub busc: i32,
}

/// Look up MasterRomInfo parameters by partial MD5.
/// Returns bonus/busc values, or -1 if not set.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_ines_lookup_master_info(
    partial_md5: u64,
    out: *mut FceuMasterRomInfoResult,
) -> bool {
    if out.is_null() {
        return false;
    }
    unsafe {
        (*out).found = false;
        (*out).bonus = -1;
        (*out).busc = -1;
    }

    if let Some(entry) = ines_data::MASTER_ROM_INFO
        .iter()
        .find(|e| e.md5lower == partial_md5)
    {
        unsafe {
            (*out).found = true;
        }
        // Parse params string (e.g. "bonus=0" or "busc=1")
        for part in entry.params.split(',') {
            if let Some((key, val)) = part.split_once('=') {
                if let Ok(v) = val.trim().parse::<i32>() {
                    unsafe {
                        match key.trim() {
                            "bonus" => (*out).bonus = v,
                            "busc" => (*out).busc = v,
                            _ => {}
                        }
                    }
                }
            }
        }
        return true;
    }

    false
}

/// Apply ROM corrections (mapper 118/24/26/99 special cases) to parse result.
/// Returns a tofix bitmask indicating what was changed.
///
/// # Safety
/// `parse` must point to a writable `FceuInesParseResult`.
/// `hinfo` must point to a valid `FceuInesHInfoResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_apply_corrections(
    parse: *mut FceuInesParseResult,
    hinfo: *const FceuInesHInfoResult,
    _partial_md5: u64,
    has_chr_rom: bool,
) -> i32 {
    if parse.is_null() || hinfo.is_null() {
        return 0;
    }

    let p = unsafe { &mut *parse };
    let h = unsafe { &*hinfo };
    let mut tofix: i32 = 0;

    // Apply clear_vrom
    if h.clear_vrom != 0 && has_chr_rom {
        p.vrom_size_8kb = 0;
        tofix |= 8;
    }

    // Apply mapper correction
    if h.mapper >= 0 && p.mapper_no != h.mapper as u32 {
        tofix |= 1;
        p.mapper_no = h.mapper as u32;
    }

    // Apply mirror correction
    if h.mirror >= 0 {
        if h.mirror == 8 {
            // Special: only change if currently four-screen
            if p.mirroring == 2 {
                tofix |= 2;
                p.mirroring = 0;
            }
        } else if p.mirroring != h.mirror {
            if p.mirroring != (h.mirror & !4) && (h.mirror & !4) <= 2 {
                tofix |= 2;
            }
            p.mirroring = h.mirror;
        }
    }

    // Apply force_battery
    if h.force_battery != 0 && !p.battery {
        tofix |= 4;
        p.battery = true;
    }

    // Hard-coded mapper-specific corrections
    // Mapper 118, 24, 26 with four-screen -> horizontal
    if (p.mapper_no == 118 || p.mapper_no == 24 || p.mapper_no == 26)
        && p.mirroring == 2
    {
        p.mirroring = 0;
        tofix |= 2;
    }

    // Mapper 99: four-screen implicitly set
    if p.mapper_no == 99 {
        p.mirroring = 2;
        tofix |= 2;
    }

    tofix
}

// ------------------------------------------------------------------
// FFI: Complete iNES header parsing
// ------------------------------------------------------------------

/// Complete iNES header parse result.
/// Contains all fields extracted from the 16-byte header, ready for
/// C++ to use without re-parsing.
#[repr(C)]
pub struct FceuInesParseResult {
    pub is_nes2: bool,
    pub mapper_no: u32,
    pub submapper: u8,
    pub mirroring: i32,          // 0=H, 1=V, 2=four-screen
    pub mirroring_as_2bits: i32,
    pub battery: bool,
    pub trainer: bool,
    pub rom_size_16kb: u32,      // PRG-ROM in 16KB units (rounded)
    pub vrom_size_8kb: u32,      // CHR-ROM in 8KB units
    pub rom_size_raw: u32,       // PRG-ROM raw (not rounded)
    pub wram_size: u32,          // NES 2.0 WRAM bytes
    pub battery_wram_size: u32,  // NES 2.0 battery WRAM bytes
    pub vram_size: u32,          // NES 2.0 VRAM bytes
    pub battery_vram_size: u32,  // NES 2.0 battery VRAM bytes
    pub vs_system: i32,          // 0=cart, 1=VS UniSystem, -1=unsupported
    pub vs_ppu: i32,             // VS PPU type (0-11)
    pub vs_type: i32,            // VS system type (0-3)
    pub tv_system: i32,          // 0=NTSC, 1=PAL
}

/// Parse a 16-byte iNES header into a structured result.
///
/// Performs: magic validation, garbage cleanup, NES 2.0 detection,
/// mapper number assembly, mirroring extraction, ROM/CHR size
/// computation, VS UniSystem detection, battery/trainer flags.
///
/// Returns `true` if the header is valid iNES, `false` otherwise.
///
/// # Safety
/// `header_bytes` must point to at least 16 readable bytes.
/// `out` must point to a writable `FceuInesParseResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_parse_header(
    header_bytes: *const u8,
    out: *mut FceuInesParseResult,
) -> bool {
    if header_bytes.is_null() || out.is_null() {
        return false;
    }

    // Copy header to local struct for safe access
    let mut hdr = FceuInesHeader {
        id: [0; 4],
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
    unsafe {
        std::ptr::copy_nonoverlapping(header_bytes, &mut hdr as *mut _ as *mut u8, 16);
    }

    // Validate magic
    if &hdr.id != b"NES\x1A" {
        return false;
    }

    // Clean garbage signatures
    hdr.cleanup();

    // NES 2.0 detection
    let is_nes2 = (hdr.rom_type2 & 0x0C) == 0x08;

    // Mapper number assembly
    let mut mapper_no: u32 = (hdr.rom_type >> 4) as u32;
    mapper_no |= (hdr.rom_type2 & 0xF0) as u32;
    if is_nes2 {
        mapper_no |= ((hdr.rom_type3 & 0x0F) as u32) << 8;
    }

    // Submapper
    let submapper = if is_nes2 { hdr.rom_type3 >> 4 } else { 0 };

    // NES 2.0 WRAM/VRAM sizes
    let (wram_size, battery_wram_size, vram_size, battery_vram_size) = if is_nes2 {
        let w = if hdr.ram_size & 0x0F != 0 { 64u32 << (hdr.ram_size & 0x0F) } else { 0 };
        let bw = if hdr.ram_size & 0xF0 != 0 { 64u32 << ((hdr.ram_size & 0xF0) >> 4) } else { 0 };
        let v = if hdr.vram_size & 0x0F != 0 { 64u32 << (hdr.vram_size & 0x0F) } else { 0 };
        let bv = if hdr.vram_size & 0xF0 != 0 { 64u32 << ((hdr.vram_size & 0xF0) >> 4) } else { 0 };
        (w, bw, v, bv)
    } else {
        (0, 0, 0, 0)
    };

    // Mirroring
    let mirroring = if hdr.rom_type & 8 != 0 { 2 } else { (hdr.rom_type & 1) as i32 };
    let mut mirroring_as_2bits = (hdr.rom_type & 1) as i32;
    if hdr.rom_type & 8 != 0 {
        mirroring_as_2bits |= 2;
    }

    // Battery and trainer
    let battery = (hdr.rom_type & 2) != 0;
    let trainer = (hdr.rom_type & 4) != 0;

    // ROM/CHR sizes
    let mut sizes = crate::cart::FceuRomSizes {
        rom_size_16kb: 0,
        vrom_size_8kb: 0,
        chrram_size: 0,
        rom_size_raw: 0,
    };
    unsafe {
        crate::cart::fceux11_rust_cart_compute_rom_sizes(
            &hdr as *const FceuInesHeader,
            is_nes2,
            &mut sizes as *mut crate::cart::FceuRomSizes,
        );
    }

    // VS UniSystem detection
    let (vs_system, vs_ppu, vs_type) = if !is_nes2 {
        if hdr.rom_type2 & 1 != 0 { (1, 0, 0) } else { (0, 0, 0) }
    } else {
        let game_type = if hdr.rom_type2 & 2 == 0 {
            hdr.rom_type2 & 3
        } else {
            hdr.vs_hardware & 0xF
        };
        match game_type {
            0 => (0, 0, 0),  // cart
            1 => {
                // VS UniSystem — extract PPU and type
                let ppu = if hdr.rom_type2 & 2 == 0 {
                    match hdr.vs_hardware & 0xF {
                        0x0 => 0,  // RC2C03B
                        0x2 => 2,  // RP2C04_0001
                        0x3 => 3,  // RP2C04_0002
                        0x4 => 4,  // RP2C04_0003
                        0x5 => 5,  // RP2C04_0004
                        0x6 => 0,  // RC2C03B
                        0x8 => 8,  // RC2C05_01
                        0x9 => 9,  // RC2C05_02
                        0xA => 10, // RC2C05_03
                        0xB => 11, // RC2C05_04
                        _ => -1,   // unsupported
                    }
                } else {
                    0
                };
                let vtype = match hdr.vs_hardware >> 4 {
                    0x0 => 0, // NORMAL
                    0x1 => 1, // RBI
                    0x2 => 2, // TKO
                    0x3 => 3, // XEVIOUS
                    _ => 0,
                };
                (1, ppu, vtype)
            }
            _ => (-1, 0, 0), // unsupported
        }
    };

    // TV system
    let tv_system = if is_nes2 && (hdr.tv_system & 3) == 1 { 1 } else { 0 };

    unsafe {
        *out = FceuInesParseResult {
            is_nes2,
            mapper_no,
            submapper,
            mirroring,
            mirroring_as_2bits,
            battery,
            trainer,
            rom_size_16kb: sizes.rom_size_16kb,
            vrom_size_8kb: sizes.vrom_size_8kb,
            rom_size_raw: sizes.rom_size_raw,
            wram_size,
            battery_wram_size,
            vram_size,
            battery_vram_size,
            vs_system,
            vs_ppu,
            vs_type,
            tv_system,
        };
    }

    true
}

// ------------------------------------------------------------------
// FFI: ROM/CHR layout computation
// ------------------------------------------------------------------

/// Computed ROM/CHR layout from an iNES header parse result.
///
/// Contains byte-level sizes and flags needed for memory allocation
/// and data loading.  Produced by `fceux11_rust_ines_compute_layout`.
#[repr(C)]
pub struct FceuInesLayout {
    /// PRG-ROM size in bytes (`rom_size_16kb << 14`).
    pub prg_size_bytes: u32,
    /// CHR-ROM size in bytes (`vrom_size_8kb << 13`).
    pub chr_size_bytes: u32,
    /// CHR-RAM size in bytes when no CHR-ROM is present.
    /// -1 means CHR-ROM is present (no CHR-RAM needed).
    /// 0 means size will be determined later by mapper init.
    pub chrram_size: i32,
    /// Whether the mapper number is a power of two (determines rounding).
    pub round_prg: bool,
    /// Number of 16 KiB PRG banks (after rounding if applicable).
    pub prg_banks_16kb: u32,
    /// Number of 8 KiB CHR banks.
    pub chr_banks_8kb: u32,
    /// Whether a 512-byte trainer is present.
    pub trainer_present: bool,
    /// PRG-ROM raw (not rounded) size in 16 KiB units.
    pub prg_banks_raw: u32,
}

/// Compute ROM/CHR layout from a parsed iNES header.
///
/// Takes the `FceuInesParseResult` from `fceux11_rust_ines_parse_header`
/// and computes byte-level sizes, CHR-RAM requirements, and rounding info.
///
/// Returns `true` on success.
///
/// # Safety
/// `parse` must point to a valid `FceuInesParseResult`.
/// `out` must point to a writable `FceuInesLayout`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_compute_layout(
    parse: *const FceuInesParseResult,
    out: *mut FceuInesLayout,
) -> bool {
    if parse.is_null() || out.is_null() {
        return false;
    }
    let p = unsafe { &*parse };
    let out = unsafe { &mut *out };

    let round = crate::ines::fceux11_rust_ines_not_power2(p.mapper_no as i32) == 0;

    out.prg_banks_16kb = p.rom_size_16kb;
    out.prg_banks_raw = p.rom_size_raw;
    out.chr_banks_8kb = p.vrom_size_8kb;
    out.round_prg = round;
    out.trainer_present = p.trainer;

    // PRG size in bytes: use rounded banks if mapper requires it, else raw
    let prg_units = if round { p.rom_size_16kb } else { p.rom_size_raw };
    out.prg_size_bytes = prg_units << 14;

    // CHR size in bytes
    out.chr_size_bytes = p.vrom_size_8kb << 13;

    // CHR-RAM: determined by mapper when no CHR-ROM present
    // The actual CHRRAM size is computed during mapper init (iNES_Init),
    // so we leave it at 0 here.  The caller should use
    // `fceux11_rust_cart_compute_chrram_size` if it needs the value early.
    out.chrram_size = if p.vrom_size_8kb > 0 { -1 } else { 0 };

    true
}

// ------------------------------------------------------------------
// FFI: ROM hash computation
// ------------------------------------------------------------------

/// Hash result for iNES ROM data.
#[repr(C)]
pub struct FceuInesHashResult {
    pub md5: [u8; 16],
    pub crc32: u32,
    pub partial_md5: u64,
}

/// Compute MD5 and CRC32 of PRG + CHR ROM data.
///
/// # Safety
/// `prg_data` must point to `prg_size` valid bytes.
/// `chr_data` may be null if `chr_size == 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_compute_hash(
    prg_data: *const u8,
    prg_size: u32,
    chr_data: *const u8,
    chr_size: u32,
    out: *mut FceuInesHashResult,
) -> bool {
    if prg_data.is_null() || out.is_null() {
        return false;
    }
    if prg_size == 0 {
        return false;
    }

    let prg = unsafe { std::slice::from_raw_parts(prg_data, prg_size as usize) };

    // Compute CRC32 of PRG+CHR (chained)
    let crc = if chr_size > 0 && !chr_data.is_null() {
        let chr = unsafe { std::slice::from_raw_parts(chr_data, chr_size as usize) };
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(prg);
        hasher.update(chr);
        hasher.finalize()
    } else {
        crc32fast::hash(prg)
    };

    // Compute MD5 of PRG+CHR
    let mut md5_ctx = md5::Context::new();
    md5_ctx.consume(prg);
    if chr_size > 0 && !chr_data.is_null() {
        let chr = unsafe { std::slice::from_raw_parts(chr_data, chr_size as usize) };
        md5_ctx.consume(chr);
    }
    let md5_digest = md5_ctx.compute();
    let md5_bytes: [u8; 16] = md5_digest.into();

    // Compute partial MD5 (first 8 bytes, big-endian reassembly)
    let mut partial_md5: u64 = 0;
    for x in 0..8 {
        partial_md5 |= (md5_bytes[7 - x] as u64) << (x * 8);
    }

    unsafe {
        *out = FceuInesHashResult {
            md5: md5_bytes,
            crc32: crc,
            partial_md5,
        };
    }

    true
}

// ------------------------------------------------------------------
// FFI: Complete iNES load
// ------------------------------------------------------------------

/// Result of a complete iNES file load.
///
/// Contains pointers into the original file buffer for PRG, CHR, and
/// trainer data, along with all parsed metadata.  The caller must keep
/// the file buffer alive for as long as it uses the data pointers.
#[repr(C)]
pub struct FceuInesCartResult {
    /// Pointer to PRG-ROM data within the file buffer.
    pub prg_data: *const u8,
    /// PRG-ROM size in bytes.
    pub prg_size: u32,
    /// Pointer to CHR-ROM data within the file buffer (null if no CHR-ROM).
    pub chr_data: *const u8,
    /// CHR-ROM size in bytes.
    pub chr_size: u32,
    /// Pointer to trainer data within the file buffer (null if no trainer).
    pub trainer_data: *const u8,
    /// Trainer size in bytes (0 if no trainer).
    pub trainer_size: u32,
    /// Mapper number.
    pub mapper_no: u32,
    /// NES 2.0 submapper (0 if not NES 2.0).
    pub submapper: u8,
    /// Mirroring mode (0=H, 1=V, 2=four-screen).
    pub mirror: i32,
    /// Mirroring as 2-bit value.
    pub mirror_as_2bits: i32,
    /// Battery-backed RAM present.
    pub battery: bool,
    /// NES 2.0 flag.
    pub is_nes2: bool,
    /// MD5 hash of PRG+CHR.
    pub md5: [u8; 16],
    /// CRC32 of PRG+CHR.
    pub crc32: u32,
    /// Partial MD5 (first 8 bytes, big-endian).
    pub partial_md5: u64,
    /// VS UniSystem type (0=cart, 1=VS, -1=unsupported).
    pub vs_system: i32,
    /// VS PPU type.
    pub vs_ppu: i32,
    /// VS system type.
    pub vs_type: i32,
    /// TV system (0=NTSC, 1=PAL).
    pub tv_system: i32,
    /// NES 2.0 WRAM size in bytes.
    pub wram_size: u32,
    /// NES 2.0 battery-backed WRAM size in bytes.
    pub battery_wram_size: u32,
    /// NES 2.0 VRAM size in bytes.
    pub vram_size: u32,
    /// NES 2.0 battery-backed VRAM size in bytes.
    pub battery_vram_size: u32,
    /// Raw PRG-ROM size in 16 KiB units (before rounding).
    pub rom_size_raw: u32,
    /// PRG-ROM size in 16 KiB units (after rounding).
    pub rom_size_16kb: u32,
    /// CHR-ROM size in 8 KiB units.
    pub vrom_size_8kb: u32,
}

/// Load an iNES ROM file from a buffer.
///
/// Parses the 16-byte header, extracts trainer/PRG/CHR data pointers,
/// and computes MD5/CRC32 hashes.  Data pointers point into the original
/// `file_data` buffer, so the caller must keep it alive.
///
/// Returns `true` on success.
///
/// # Safety
/// `file_data` must point to at least `file_len` readable bytes.
/// `out_cart` must point to a writable `FceuInesCartResult`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_ines_load(
    file_data: *const u8,
    file_len: usize,
    out_cart: *mut FceuInesCartResult,
) -> bool {
    if file_data.is_null() || out_cart.is_null() || file_len < 16 {
        return false;
    }

    // Parse header
    let mut parse_result = FceuInesParseResult {
        is_nes2: false,
        mapper_no: 0,
        submapper: 0,
        mirroring: 0,
        mirroring_as_2bits: 0,
        battery: false,
        trainer: false,
        rom_size_16kb: 0,
        vrom_size_8kb: 0,
        rom_size_raw: 0,
        wram_size: 0,
        battery_wram_size: 0,
        vram_size: 0,
        battery_vram_size: 0,
        vs_system: 0,
        vs_ppu: 0,
        vs_type: 0,
        tv_system: 0,
    };

    if unsafe { !fceux11_rust_ines_parse_header(file_data, &mut parse_result) } {
        return false;
    }

    // Compute layout
    let mut layout = FceuInesLayout {
        prg_size_bytes: 0,
        chr_size_bytes: 0,
        chrram_size: 0,
        round_prg: false,
        prg_banks_16kb: 0,
        chr_banks_8kb: 0,
        trainer_present: false,
        prg_banks_raw: 0,
    };

    if unsafe { !fceux11_rust_ines_compute_layout(&parse_result, &mut layout) } {
        return false;
    }

    // Calculate offsets
    let mut offset: usize = 16; // skip header

    // Trainer
    let trainer_data = if parse_result.trainer {
        if offset + 512 > file_len {
            return false;
        }
        let ptr = unsafe { file_data.add(offset) };
        offset += 512;
        ptr
    } else {
        std::ptr::null()
    };
    let trainer_size: u32 = if parse_result.trainer { 512 } else { 0 };

    // PRG-ROM
    let prg_size = layout.prg_size_bytes as usize;
    if offset + prg_size > file_len {
        return false;
    }
    let prg_data = unsafe { file_data.add(offset) };
    offset += prg_size;

    // CHR-ROM
    let chr_size = layout.chr_size_bytes as usize;
    let chr_data = if chr_size > 0 {
        if offset + chr_size > file_len {
            return false;
        }
        unsafe { file_data.add(offset) }
    } else {
        std::ptr::null()
    };

    // Compute hash
    let mut hash_result = FceuInesHashResult {
        md5: [0; 16],
        crc32: 0,
        partial_md5: 0,
    };
    unsafe {
        fceux11_rust_ines_compute_hash(
            prg_data,
            layout.prg_size_bytes,
            chr_data,
            layout.chr_size_bytes,
            &mut hash_result,
        );
    }

    // Fill output
    unsafe {
        *out_cart = FceuInesCartResult {
            prg_data,
            prg_size: layout.prg_size_bytes,
            chr_data,
            chr_size: layout.chr_size_bytes,
            trainer_data,
            trainer_size,
            mapper_no: parse_result.mapper_no,
            submapper: parse_result.submapper,
            mirror: parse_result.mirroring,
            mirror_as_2bits: parse_result.mirroring_as_2bits,
            battery: parse_result.battery,
            is_nes2: parse_result.is_nes2,
            md5: hash_result.md5,
            crc32: hash_result.crc32,
            partial_md5: hash_result.partial_md5,
            vs_system: parse_result.vs_system,
            vs_ppu: parse_result.vs_ppu,
            vs_type: parse_result.vs_type,
            tv_system: parse_result.tv_system,
            wram_size: parse_result.wram_size,
            battery_wram_size: parse_result.battery_wram_size,
            vram_size: parse_result.vram_size,
            battery_vram_size: parse_result.battery_vram_size,
            rom_size_raw: parse_result.rom_size_raw,
            rom_size_16kb: parse_result.rom_size_16kb,
            vrom_size_8kb: parse_result.vrom_size_8kb,
        };
    }

    true
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
