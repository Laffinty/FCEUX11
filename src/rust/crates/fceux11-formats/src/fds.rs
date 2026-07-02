//! Famicom Disk System (FDS) — pure-logic helpers migrated in v0.2.26.
//!
//! Scope (per `docs/rust_refactor_plan_v0.2.12-v0.2.30.md` §6.3):
//! - Disk-image header recognition (`FDS\x1a` / `*NINTENDO-HVC*`) and
//!   per-side count computation.
//! - Save-state XOR helper used by `PreSave` / `PostSave` /
//!   `FDSStateRestore`.
//! - IRQ countdown and fire-decision logic (the C++ main loop still
//!   makes the actual `X6502_IRQBegin` call based on the returned flags).
//! - $4030 / $4032 register-value computation.
//! - $4025 write-side state machine for the block-transfer FSM.
//! - Block-size lookup and motor-on block-advance helper.
//!
//! Out of scope (kept in C++):
//! - All sound code (`FDSSound`, `RenderSound*`, `FDS_ESI`,
//!   `GameExpSound` callbacks).
//! - File I/O (`FDSLoad`, `FDSClose`, `FDSInit`).
//! - UI / movie / netplay hooks (`FCEU_FDSInsert`, `FCEU_FDSSelect`
//!   messaging).
//! - $4031 / $4024 disk-byte read/write (depend on
//!   `mapperFDS_diskaccess` FSM that the C++ side owns).
//! - Save-state `AddExState` registration — all C++ globals remain at
//!   their canonical addresses so save-state binary format is unchanged.

// ------------------------------------------------------------------
// C-compatible constants
// ------------------------------------------------------------------

/// One FDS disk side is exactly 65500 bytes of payload.
pub const FCEUX11_RUST_FDS_DISK_SIDE_SIZE: u32 = 65500;
/// FDS BIOS image size in bytes.
pub const FCEUX11_RUST_FDS_BIOS_SIZE: u32 = 8192;
/// FDS work RAM size, mapped at $6000–$DFFF.
pub const FCEUX11_RUST_FDS_RAM_SIZE: u32 = 32768;
/// FDS CHR RAM size, mapped at $0000–$1FFF.
pub const FCEUX11_RUST_FDS_CHR_RAM_SIZE: u32 = 8192;
/// Maximum number of disk sides supported per image.
pub const FCEUX11_RUST_FDS_MAX_SIDES: u8 = 8;
/// CPU cycles between motor-on/seek events and the disk-seek IRQ.
pub const FCEUX11_RUST_FDS_DISK_SEEK_CYCLES: i32 = 150;
/// Sentinel `InDisk` value meaning "no disk inserted".
pub const FCEUX11_RUST_FDS_NOT_INSERTED: u8 = 255;
/// IRQa bit 0: when set, IRQ rearms on each fire.
pub const FCEUX11_RUST_FDS_IRQ_REPEAT: u8 = 0x01;
/// IRQa bit 1: master IRQ enable.
pub const FCEUX11_RUST_FDS_IRQ_ENABLED: u8 = 0x02;
/// Block-FSM: initial / no-block state.
pub const FCEUX11_RUST_FDS_DSK_INIT: u8 = 0;
/// Block-FSM: disk volume label block (0x38 bytes).
pub const FCEUX11_RUST_FDS_DSK_VOLUME: u8 = 1;
/// Block-FSM: file-count block (0x02 bytes).
pub const FCEUX11_RUST_FDS_DSK_FILECNT: u8 = 2;
/// Block-FSM: file header block (0x10 bytes).
pub const FCEUX11_RUST_FDS_DSK_FILEHDR: u8 = 3;
/// Block-FSM: file data block (0x01 + file_size bytes).
pub const FCEUX11_RUST_FDS_DSK_FILEDATA: u8 = 4;
/// IRQ source bit for the FDS timer IRQ (mirrors C++ `FCEU_IQEXT`).
pub const FCEUX11_RUST_FDS_FCEU_IQEXT: u8 = 0x01;
/// IRQ source bit for the FDS disk-seek IRQ (mirrors C++ `FCEU_IQEXT2`).
pub const FCEUX11_RUST_FDS_FCEU_IQEXT2: u8 = 0x02;

// ------------------------------------------------------------------
// C-compatible result structs
// ------------------------------------------------------------------

/// Result of header recognition on a 16-byte prefix.
///
/// `kind`:
/// - `0` = unrecognized format (caller should return LOADER_INVALID_FORMAT)
/// - `1` = standard `FDS\x1a` header at offset 0 (16-byte header before
///   the first side's payload)
/// - `2` = raw image with `*NINTENDO-HVC*` marker at offset 1 (no header
///   to skip; sides are derived from file size)
#[repr(C)]
pub struct FceuFdsHeaderInfo {
    pub kind: u8,
    /// Bytes to skip before the first side's payload (16 for kind=1,
    /// 0 for kind=2, 0 for kind=0).
    pub header_size: u8,
    /// For kind=1, the side count advertised by header[4].
    /// For kind=0/2, set to 0.
    pub advertised_sides: u8,
}

/// Mirror of the C++ IRQ-related globals used by `FDSFix`.
/// `irq_count` and `disk_seek_irq` may go negative after subtraction
/// (the C++ comparison is `<= 0`).
#[repr(C)]
pub struct FceuFdsIrqState {
    pub irq_count: i32,
    pub irq_latch: i32,
    pub irq_a: u8,
    pub disk_seek_irq: i32,
    pub fds_regs_5: u8,
}

/// Decision returned by `fceux11_rust_fds_irq_tick`.
/// C++ acts on these by calling `X6502_IRQBegin(FCEU_IQEXT)` and
/// `X6502_IRQBegin(FCEU_IQEXT2)` respectively.
#[repr(C)]
pub struct FceuFdsIrqTickResult {
    pub timer_fire: bool,
    pub seek_fire: bool,
}

/// State delta to apply after a $4025 write.
/// `new_block`/`new_blocklen` are meaningful only when `motor_on_edge`
/// is true. If `transfer_reset` is set, C++ overrides the block to
/// `DSK_INIT` and zeros `blockstart`/`blocklen`/`diskaddr` itself.
#[repr(C)]
pub struct FceuFdsWrite4025Result {
    pub motor_on_edge: bool,
    pub transfer_reset: bool,
    pub motor_on: bool,
    pub new_block: u8,
    pub new_blocklen: u16,
}

// ------------------------------------------------------------------
// Pure helpers (no FFI overhead)
// ------------------------------------------------------------------

fn lookup_block_size(block_type: u8, file_size: u16) -> i32 {
    match block_type {
        FCEUX11_RUST_FDS_DSK_VOLUME => 0x38,
        FCEUX11_RUST_FDS_DSK_FILECNT => 0x02,
        FCEUX11_RUST_FDS_DSK_FILEHDR => 0x10,
        FCEUX11_RUST_FDS_DSK_FILEDATA => 1i32 + file_size as i32,
        _ => 0,
    }
}

fn advance_block(current: u8) -> u8 {
    // C++ src/fds.cpp:661–664
    //   mapperFDS_block++;
    //   if (mapperFDS_block > DSK_FILEDATA) mapperFDS_block = DSK_FILEHDR;
    // Numerically: 0→1, 1→2, 2→3, 3→4, 4→3, (>=5)→clamped to FILEHDR.
    let next = current.saturating_add(1);
    if next > FCEUX11_RUST_FDS_DSK_FILEDATA {
        FCEUX11_RUST_FDS_DSK_FILEHDR
    } else {
        next
    }
}

// ------------------------------------------------------------------
// FFI: header validation
// ------------------------------------------------------------------

/// Recognize FDS disk-image format from a 16-byte prefix.
///
/// `buf` must be non-null and point to at least 16 readable bytes.
/// `len` is the number of readable bytes available; a length below 16
/// (or below 15 for raw images) causes the function to return
/// `kind = 0`.
///
/// Mirrors `src/fds.cpp:717–733`'s `memcmp` chain.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_validate_header(
    buf: *const u8,
    len: usize,
) -> FceuFdsHeaderInfo {
    let invalid = FceuFdsHeaderInfo {
        kind: 0,
        header_size: 0,
        advertised_sides: 0,
    };
    if buf.is_null() {
        return invalid;
    }
    // Need at least 15 bytes to check the raw marker at offset 1..15.
    if len < 15 {
        return invalid;
    }
    let header = unsafe { std::slice::from_raw_parts(buf, len.min(16)) };

    // Standard FDS header: "FDS\x1a" at offset 0
    if header.len() >= 5 && &header[0..4] == b"FDS\x1a" {
        return FceuFdsHeaderInfo {
            kind: 1,
            header_size: 16,
            advertised_sides: header[4],
        };
    }

    // Raw image: "*NINTENDO-HVC*" at offset 1..15
    if header.len() >= 15 && &header[1..15] == b"*NINTENDO-HVC*" {
        return FceuFdsHeaderInfo {
            kind: 2,
            header_size: 0,
            advertised_sides: 0,
        };
    }

    invalid
}

/// Clamp the side count to [1, FCEUX11_RUST_FDS_MAX_SIDES].
///
/// If `has_fds_header != 0`, the `advertised` value is used as the
/// starting point (the byte at header[4]).  Otherwise the side count
/// is derived from `file_size / FCEUX11_RUST_FDS_DISK_SIDE_SIZE`,
/// matching the C++ "raw image" code path (`src/fds.cpp:719–723`).
///
/// The strange `if (t < 65500) t = 65500;` line in the original is
/// preserved here: even a 0-byte raw image yields one side.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_compute_total_sides(
    file_size: usize,
    advertised: u8,
    has_fds_header: u8,
) -> u8 {
    let raw: u32 = if has_fds_header != 0 {
        advertised as u32
    } else {
        let t = file_size.max(FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize);
        (t / FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize) as u32
    };
    raw.clamp(1, FCEUX11_RUST_FDS_MAX_SIDES as u32) as u8
}

// ------------------------------------------------------------------
// FFI: save-state XOR
// ------------------------------------------------------------------

/// XOR the original disk image (`src`) into the current disk image
/// (`dst`) in place — `dst[i] ^= src[i]` for `i` in
/// `0..FCEUX11_RUST_FDS_DISK_SIDE_SIZE`.
///
/// Used in three places to keep save-state binary identical regardless
/// of the run-time disk-write state:
///   - `PreSave`  (src/fds.cpp:744)  — encode writes before serializing
///   - `PostSave` (src/fds.cpp:753)  — undo encoding after serializing
///   - `FDSStateRestore` (src/fds.cpp:118) — re-encode after load
///
/// Both pointers must be non-null and point to at least
/// `FCEUX11_RUST_FDS_DISK_SIDE_SIZE` bytes.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_xor_disk_data(dst: *mut u8, src: *const u8) {
    if dst.is_null() || src.is_null() {
        return;
    }
    let n = FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize;
    let d = unsafe { std::slice::from_raw_parts_mut(dst, n) };
    let s = unsafe { std::slice::from_raw_parts(src, n) };
    for i in 0..n {
        d[i] ^= s[i];
    }
}

// ------------------------------------------------------------------
// FFI: IRQ tick decision
// ------------------------------------------------------------------

/// Decrement IRQ counters and report which IRQs should fire.
///
/// 1:1 translation of `FDSFix` (src/fds.cpp:230–258).  Mutates the
/// caller's `state` in place so C++ can write the new values back to
/// its globals.  Returns `timer_fire`/`seek_fire` so C++ can issue
/// `X6502_IRQBegin(FCEU_IQEXT[2])` outside the FFI boundary.
///
/// Puff Puff Golf notes (preserved verbatim from C++):
/// > Game freezes while music playing ingame after inserting Disk Side B.
/// > IRQ is usually fired at scanline 169 and 183 for music to work.
/// > At some point after inserting disk B, an IRQ is fired at scanline
/// > 174 which will just freeze game while music plays.
/// > If you ignore triggering IRQ altogether, game plays but no music.
///
/// This is a historical behaviour of the original C++ — do NOT add a
/// suppression flag here.  The point of this migration is byte-for-byte
/// equivalence.
///
/// `delta_cycles` is the number of CPU cycles elapsed since the
/// previous tick; the C++ caller (`MapIRQHook`) always passes a
/// non-negative value, but we use `wrapping_sub` for defensiveness.
/// # Safety
/// The caller must ensure that all raw pointers are non-null, properly aligned, and
/// point to valid memory regions of the expected size for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_irq_tick(
    state: *mut FceuFdsIrqState,
    delta_cycles: i32,
) -> FceuFdsIrqTickResult {
    let mut result = FceuFdsIrqTickResult {
        timer_fire: false,
        seek_fire: false,
    };
    if state.is_null() {
        return result;
    }
    let s = unsafe { &mut *state };

    // Timer IRQ
    if s.irq_a & FCEUX11_RUST_FDS_IRQ_ENABLED != 0 {
        s.irq_count = s.irq_count.wrapping_sub(delta_cycles);
        if s.irq_count <= 0 {
            s.irq_count = s.irq_latch;
            result.timer_fire = true;
            if s.irq_a & FCEUX11_RUST_FDS_IRQ_REPEAT == 0 {
                s.irq_a &= !FCEUX11_RUST_FDS_IRQ_ENABLED;
            }
        }
    }

    // Disk-seek IRQ
    if s.disk_seek_irq > 0 {
        s.disk_seek_irq = s.disk_seek_irq.wrapping_sub(delta_cycles);
        if s.disk_seek_irq <= 0 && (s.fds_regs_5 & 0x80) != 0 {
            result.seek_fire = true;
        }
    }

    result
}

// ------------------------------------------------------------------
// FFI: block-FSM helpers
// ------------------------------------------------------------------

/// Look up `mapperFDS_blocklen` for a given block type.
/// Returns 0 for unknown block types (including `DSK_INIT`).
///
/// Mirrors the switch in `FDSWrite` case 0x4025 (src/fds.cpp:665–678).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_block_size(block_type: u8, file_size: u16) -> i32 {
    lookup_block_size(block_type, file_size)
}

/// Advance the block FSM on a motor-on edge.
/// 0→1, 1→2, 2→3, 3→4, 4→3, (>=5)→3 (FILEHDR clamp).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_block_advance_on_motor(current_block: u8) -> u8 {
    advance_block(current_block)
}

// ------------------------------------------------------------------
// FFI: register-read pure values
// ------------------------------------------------------------------

/// Compute the value returned by reading $4030.
/// Mirrors `FDSRead4030` (src/fds.cpp:261–265) — the C++ wrapper
/// retains the `fceuindbg`-guarded `X6502_IRQEnd` calls.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_read_4030_value(irq_low_ext: bool, irq_low_ext2: bool) -> u8 {
    let mut ret: u8 = 0;
    if irq_low_ext {
        ret |= 1;
    }
    if irq_low_ext2 {
        ret |= 2;
    }
    ret
}

/// Compute the value returned by reading $4032.
/// Mirrors `FDSRead4032` (src/fds.cpp:314–324).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_read_4032_value(
    in_disk: u8,
    fds_regs_5: u8,
    data_bus: u8,
) -> u8 {
    let mut ret = data_bus & !7u8;
    if in_disk == FCEUX11_RUST_FDS_NOT_INSERTED {
        ret |= 5;
    }
    if in_disk == FCEUX11_RUST_FDS_NOT_INSERTED || (fds_regs_5 & 1) == 0 || (fds_regs_5 & 2) != 0 {
        ret |= 2;
    }
    ret
}

// ------------------------------------------------------------------
// FFI: SelectDisk increment
// ------------------------------------------------------------------

/// Compute the next `SelectDisk` value after the user presses the
/// disk-swap key.  Mirrors `FCEU_FDSSelect` line 223.
/// Returns 0 if `total_sides == 0` (defensive — C++ already guards on
/// that before calling, but the FFI must not divide by zero).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_compute_select_disk_next(current: u8, total_sides: u8) -> u8 {
    if total_sides == 0 {
        return 0;
    }
    let next = (current.wrapping_add(1)) % total_sides;
    next & 3
}

// ------------------------------------------------------------------
// FFI: $4025 write decision
// ------------------------------------------------------------------

/// Compute the state changes for a write to $4025.
///
/// The result describes three independent conditions, each of which
/// the C++ side acts on in sequence:
///   - `motor_on_edge`: bit 6 transitioned 0→1.  C++ must reset
///     `mapperFDS_diskaccess` to 0, advance `mapperFDS_blockstart`
///     by the current `mapperFDS_diskaddr`, zero `mapperFDS_diskaddr`,
///     set `mapperFDS_block` to `new_block`, set `mapperFDS_blocklen`
///     to `new_blocklen`, and set `DiskSeekIRQ` to 150.
///   - `transfer_reset`: bit 1 is set.  C++ must zero
///     `mapperFDS_block`/`blockstart`/`blocklen`/`diskaddr` and set
///     `DiskSeekIRQ` to 150.  If both `motor_on_edge` and
///     `transfer_reset` fire, transfer-reset wins for the block fields.
///   - `motor_on`: bit 6 is set in the new value (regardless of edge).
///     C++ sets `DiskSeekIRQ = 150` (no-op if either of the above
///     already did so).
///
/// If `disk_inserted == 0`, all three flags are false (the C++ guards
/// the whole block on `mapperFDS_diskinsert`).
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_compute_write_4025(
    current_block: u8,
    current_filesize: u16,
    current_control: u8,
    value: u8,
    disk_inserted: u8,
) -> FceuFdsWrite4025Result {
    if disk_inserted == 0 {
        return FceuFdsWrite4025Result {
            motor_on_edge: false,
            transfer_reset: false,
            motor_on: false,
            new_block: current_block,
            new_blocklen: 0,
        };
    }

    let motor_on = (value & 0x40) != 0;
    let motor_on_edge = motor_on && (current_control & 0x40) == 0;
    let transfer_reset = (value & 0x02) != 0;

    let (new_block, new_blocklen) = if motor_on_edge {
        let advanced = advance_block(current_block);
        let blocklen = lookup_block_size(advanced, current_filesize).max(0) as u16;
        (advanced, blocklen)
    } else {
        (current_block, 0)
    };

    FceuFdsWrite4025Result {
        motor_on_edge,
        transfer_reset,
        motor_on,
        new_block,
        new_blocklen,
    }
}

// ------------------------------------------------------------------
// FFI: Disk I/O state and operations
// ------------------------------------------------------------------

/// Disk I/O state for FDS operations.
///
/// Contains the current position and block information for disk reads/writes.
/// The caller maintains ownership of the disk data arrays and passes pointers
/// to them as needed.
#[repr(C)]
pub struct FceuFdsDiskIoState {
    /// Current block type (DSK_INIT, DSK_VOLUME, DSK_FILECNT, DSK_FILEHDR, DSK_FILEDATA).
    pub block: u8,
    /// Start address of current block within disk data.
    pub block_start: u16,
    /// Length of current block.
    pub block_len: u16,
    /// Current address relative to block_start.
    pub disk_addr: u16,
    /// File size being read/written (for FILEHDR/FILEDATA blocks).
    pub file_size: u16,
    /// Control register value ($4025).
    pub control: u8,
    /// Whether disk is inserted.
    pub disk_inserted: u8,
    /// Whether disk was accessed (for IRQ timing).
    pub disk_access: u8,
}

/// Result of a disk read operation ($4031).
#[repr(C)]
pub struct FceuFdsDiskReadResult {
    /// The byte read from disk.
    pub value: u8,
    /// Updated disk address after read.
    pub new_disk_addr: u16,
    /// Updated file size (if reading from FILEHDR block at specific offsets).
    pub new_file_size: u16,
    /// Whether a seek IRQ should be triggered.
    pub trigger_seek_irq: bool,
}

/// Result of a disk write operation ($4024).
#[repr(C)]
pub struct FceuFdsDiskWriteResult {
    /// Updated disk address after write.
    pub new_disk_addr: u16,
    /// Updated file size (if writing to FILEHDR block at specific offsets).
    pub new_file_size: u16,
    /// Whether disk was marked as written.
    pub disk_written: bool,
}

/// Read a byte from the FDS disk ($4031).
///
/// Performs the disk read operation based on the current block type and position.
/// Returns the read byte and updated state.
///
/// # Safety
/// `state` must point to a valid `FceuFdsDiskIoState`.
/// `disk_data` must point to at least 65500 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_disk_read(
    state: *const FceuFdsDiskIoState,
    disk_data: *const u8,
    out: *mut FceuFdsDiskReadResult,
) -> bool {
    if state.is_null() || disk_data.is_null() || out.is_null() {
        return false;
    }
    let s = unsafe { &*state };
    let result = unsafe { &mut *out };

    // Default: no seek IRQ, keep current state
    result.trigger_seek_irq = false;
    result.new_disk_addr = s.disk_addr;
    result.new_file_size = s.file_size;

    // Only read if disk is inserted and motor is on
    if s.disk_inserted == 0 || (s.control & 0x04) == 0 {
        result.value = 0xFF;
        return true;
    }

    // Check if we're within the block
    if s.disk_addr >= s.block_len {
        result.value = 0;
        return true;
    }

    // Read the byte from disk data
    let offset = (s.block_start as usize) + (s.disk_addr as usize);
    if offset >= 65500 {
        result.value = 0;
        return true;
    }

    let data = unsafe { std::slice::from_raw_parts(disk_data, 65500) };
    result.value = data[offset];

    // Update file size if reading from FILEHDR block
    if s.block == FCEUX11_RUST_FDS_DSK_FILEHDR {
        match s.disk_addr {
            13 => result.new_file_size = result.value as u16,
            14 => result.new_file_size = s.file_size | ((result.value as u16) << 8),
            _ => {}
        }
    }

    // Advance disk address
    result.new_disk_addr = s.disk_addr + 1;

    // Trigger seek IRQ
    result.trigger_seek_irq = true;

    true
}

/// Write a byte to the FDS disk ($4024).
///
/// Performs the disk write operation based on the current block type and position.
/// Returns updated state.
///
/// # Safety
/// `state` must point to a valid `FceuFdsDiskIoState`.
/// `disk_data` must point to at least 65500 writable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_disk_write(
    state: *const FceuFdsDiskIoState,
    disk_data: *mut u8,
    value: u8,
    out: *mut FceuFdsDiskWriteResult,
) -> bool {
    if state.is_null() || disk_data.is_null() || out.is_null() {
        return false;
    }
    let s = unsafe { &*state };
    let result = unsafe { &mut *out };

    // Default: keep current state, disk not written
    result.new_disk_addr = s.disk_addr;
    result.new_file_size = s.file_size;
    result.disk_written = false;

    // Only write if disk is inserted and motor is on
    if s.disk_inserted == 0 || (s.control & 0x04) == 0 {
        return true;
    }

    // Check if we're within the block
    if s.disk_addr >= s.block_len {
        return true;
    }

    // Write the byte to disk data
    let offset = (s.block_start as usize) + (s.disk_addr as usize);
    if offset >= 65500 {
        return true;
    }

    let data = unsafe { std::slice::from_raw_parts_mut(disk_data, 65500) };
    data[offset] = value;
    result.disk_written = true;

    // Update file size if writing to FILEHDR block
    if s.block == FCEUX11_RUST_FDS_DSK_FILEHDR {
        match s.disk_addr {
            13 => result.new_file_size = value as u16,
            14 => result.new_file_size = s.file_size | ((value as u16) << 8),
            _ => {}
        }
    }

    // Advance disk address
    result.new_disk_addr = s.disk_addr + 1;

    true
}

/// Switch to a different disk side.
///
/// Updates the InDisk value and resets the block FSM state.
/// Returns the new InDisk value.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_fds_switch_side(
    current_side: u8,
    total_sides: u8,
    new_side: u8,
) -> u8 {
    if total_sides == 0 {
        return current_side;
    }
    if new_side >= total_sides {
        return current_side;
    }
    new_side
}

// ==================================================================
// FDS Runtime State — v1.10 Cryptex Task 3
// ==================================================================
// Consolidates the FDS disk-block state machine, IRQ state, and
// disk-insertion state previously held by C++ static globals.
//
// C++ retains: FDSLoad / FDSClose file I/O, FDSSound DSP,
// AddExState savestate registration, and handler registration.

/// Flags returned by `fceux11_rust_fds_handle_write_4025` telling C++
/// which side effects to apply.
#[repr(C)]
pub struct FceuFdsWrite4025Action {
    /// New mirror mode: 0=horizontal, 1=vertical (call `setmirror`).
    pub mirror_mode: u8,
    /// `true` when `mirror_mode` changed (bit 3 of the new control value
    /// differs from the old one), so C++ can skip the call when false.
    pub mirror_changed: bool,
    /// `true` → C++ must call `X6502_IRQEnd(FCEU_IQEXT2)`.
    pub irq_end_ext2: bool,
    /// `true` → C++ must call `X6502_IRQBegin(FCEU_IQEXT)`.
    pub irq_begin_ext: bool,
    /// `true` → C++ must call `X6502_IRQEnd(FCEU_IQEXT)`.
    pub irq_end_ext: bool,
}

/// Opaque FDS runtime state — C++ holds an `FdsRuntimeState*` handle.
#[repr(C)]
pub struct FdsRuntimeState {
    // ── Disk block FSM (was `mapperFDS_*` C++ globals) ──
    pub control: u8,
    pub filesize: u16,
    pub block: u8,
    pub blockstart: u16,
    pub blocklen: u16,
    pub diskaddr: u16,
    pub diskaccess: u8,

    // ── IRQ state ──
    pub irq_count: i32,
    pub irq_latch: i32,
    pub irq_a: u8,
    pub disk_seek_irq: i32,

    // ── Disk side state ──
    pub select_disk: u8,
    pub in_disk: u8,
    pub disk_written: u8,
    pub write_skip: u8,
    pub disk_ptr: i32,
    pub total_sides: i32,

    // ── FDS registers ──
    pub fds_regs: [u8; 6],
}

// ── FFI: Create / destroy ─────────────────────────────────────────

/// Allocate and zero-initialise an `FdsRuntimeState`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_runtime_create() -> *mut FdsRuntimeState {
    let state = Box::new(FdsRuntimeState {
        control: 0,
        filesize: 0,
        block: 0,
        blockstart: 0,
        blocklen: 0,
        diskaddr: 0,
        diskaccess: 0,
        irq_count: 0,
        irq_latch: 0,
        irq_a: 0,
        disk_seek_irq: 0,
        select_disk: 0,
        in_disk: 255,
        disk_written: 0,
        write_skip: 0,
        disk_ptr: 0,
        total_sides: 0,
        fds_regs: [0u8; 6],
    });
    Box::into_raw(state)
}

/// Free an `FdsRuntimeState`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_runtime_destroy(state: *mut FdsRuntimeState) {
    if !state.is_null() {
        unsafe { drop(Box::from_raw(state)); }
    }
}

/// Reset the FDS runtime state for a fresh disk load.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_runtime_reset(state: *mut FdsRuntimeState) {
    if state.is_null() { return; }
    let s = unsafe { &mut *state };
    s.control = 0;
    s.filesize = 0;
    s.block = 0;
    s.blockstart = 0;
    s.blocklen = 0;
    s.diskaddr = 0;
    s.diskaccess = 0;
    s.irq_count = 0;
    s.irq_latch = 0;
    s.irq_a = 0;
    s.disk_seek_irq = 0;
    s.select_disk = 0;
    s.in_disk = 255;
    s.disk_written = 0;
    s.write_skip = 0;
    s.disk_ptr = 0;
    s.fds_regs = [0u8; 6];
}

// ── FFI: Handle complete $4025 write ──────────────────────────────

/// Handle a write to FDS register $4025.
///
/// This replaces the entire C++ FDSWrite `case 0x4025:` block.
/// The caller (C++) must:
/// 1. Store `value` in `FDSRegs[A & 7]` AFTER this call
/// 2. If `action.irq_end_ext2`: call `X6502_IRQEnd(FCEU_IQEXT2)`
/// 3. If `action.mirror_changed`: call `setmirror(action.mirror_mode)`
///
/// # Safety
/// `state` and `action` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_handle_write_4025(
    state: *mut FdsRuntimeState,
    value: u8,
    action: *mut FceuFdsWrite4025Action,
) {
    if state.is_null() || action.is_null() {
        return;
    }
    let s = unsafe { &mut *state };
    let a = unsafe { &mut *action };

    a.irq_end_ext2 = true;
    a.mirror_mode = ((value >> 3) & 1) ^ 1;
    a.mirror_changed = (s.control & 0x08) != (value & 0x08);
    a.irq_begin_ext = false;
    a.irq_end_ext = false;

    let disk_inserted = s.in_disk != 255;

    let motor_on = (value & 0x40) != 0;
    let motor_on_edge = motor_on && (s.control & 0x40) == 0;
    let transfer_reset = (value & 0x02) != 0;

    if disk_inserted {
        if motor_on_edge {
            s.diskaccess = 0;
            s.disk_seek_irq = 150;

            s.blockstart = s.blockstart.wrapping_add(s.diskaddr);
            s.diskaddr = 0;

            let advanced = fceux11_rust_fds_block_advance_on_motor(s.block);
            s.block = advanced;
            s.blocklen = fceux11_rust_fds_block_size(advanced, s.filesize).max(0) as u16;
        }
        if transfer_reset {
            s.block = 0; // DSK_INIT
            s.blockstart = 0;
            s.blocklen = 0;
            s.diskaddr = 0;
            s.disk_seek_irq = 150;
        }
        if motor_on {
            s.disk_seek_irq = 150;
        }
    }

    s.control = value;
}

// ── FFI: Handle writes to $4020-$4024 ─────────────────────────────

/// Handle writes to FDS registers $4020–$4024.
///
/// Returns the new value for `FDSRegs[A & 7]` (usually `value`,
/// except for $4023 which reads back 0xFF).
///
/// # Safety
/// `state` must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_fds_handle_write_4020_4024(
    state: *mut FdsRuntimeState,
    addr: u16,
    value: u8,
    action: *mut FceuFdsWrite4025Action,
) -> u8 {
    if state.is_null() {
        return value;
    }
    let s = unsafe { &mut *state };

    // Default action: no side effects
    if !action.is_null() {
        let a = unsafe { &mut *action };
        a.irq_end_ext2 = false;
        a.mirror_changed = false;
        a.mirror_mode = 0;
        a.irq_begin_ext = false;
        a.irq_end_ext = false;
    }

    match addr {
        0x4020 => {
            s.irq_latch &= 0xFF00;
            s.irq_latch |= value as i32;
        }
        0x4021 => {
            s.irq_latch &= 0xFF;
            s.irq_latch |= (value as i32) << 8;
        }
        0x4022 => {
            if s.fds_regs[3] & 1 != 0 {
                s.irq_a = value & 0x03;
                if s.irq_a & 0x02 != 0 {
                    s.irq_count = s.irq_latch;
                } else {
                    if !action.is_null() {
                        let a = unsafe { &mut *action };
                        a.irq_end_ext = true;
                    }
                }
            }
        }
        0x4023 => {
            if value & 0x01 == 0 {
                s.irq_a &= !0x02;
                if !action.is_null() {
                    let a = unsafe { &mut *action };
                    a.irq_end_ext = true;
                    a.irq_end_ext2 = true;
                }
            }
        }
        0x4024 => {
            // Disk write: the C++ side handles the actual disk byte
            // write via fceux11_rust_fds_disk_write. Here we just
            // handle the seek IRQ side effect that the C++ side
            // expects to apply after disk_write returns.
            // (No Rust-side state changes needed — everything is in
            // fceux11_rust_fds_disk_write.)
        }
        _ => {}
    }

    value
}

// ------------------------------------------------------------------
// Tests (existing)
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ----- xor_disk_data ------------------------------------------------

    #[test]
    fn xor_disk_data_null_safe() {
        unsafe {
            // Must not crash on null inputs.
            unsafe { fceux11_rust_fds_xor_disk_data(std::ptr::null_mut(), std::ptr::null()) };
            let mut buf = vec![0u8; FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize];
            unsafe { fceux11_rust_fds_xor_disk_data(buf.as_mut_ptr(), std::ptr::null()) };
            unsafe { fceux11_rust_fds_xor_disk_data(std::ptr::null_mut(), buf.as_ptr()) };
        }
    }

    #[test]
    fn xor_disk_data_self_inverse() {
        unsafe {
            // dst ^= src twice → original dst.
            let mut dst = vec![0xA5u8; FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize];
            let src = vec![0x5Au8; FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize];
            let original = dst.clone();
            unsafe { fceux11_rust_fds_xor_disk_data(dst.as_mut_ptr(), src.as_ptr()) };
            unsafe { fceux11_rust_fds_xor_disk_data(dst.as_mut_ptr(), src.as_ptr()) };
            assert_eq!(dst, original);
        }
    }

    #[test]
    fn xor_disk_data_zero_identity() {
        unsafe {
            let mut dst = vec![0xFFu8; FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize];
            let src = vec![0u8; FCEUX11_RUST_FDS_DISK_SIDE_SIZE as usize];
            let original = dst.clone();
            unsafe { fceux11_rust_fds_xor_disk_data(dst.as_mut_ptr(), src.as_ptr()) };
            assert_eq!(dst, original); // XOR with zero is identity
        }
    }

    // ----- validate_header ----------------------------------------------

    #[test]
    fn validate_header_fds_magic() {
        unsafe {
            let mut hdr = [0u8; 16];
            hdr[0..4].copy_from_slice(b"FDS\x1a");
            hdr[4] = 4;
            let info = unsafe { fceux11_rust_fds_validate_header(hdr.as_ptr(), 16) };
            assert_eq!(info.kind, 1);
            assert_eq!(info.header_size, 16);
            assert_eq!(info.advertised_sides, 4);
        }
    }

    #[test]
    fn validate_header_raw_magic() {
        unsafe {
            let mut hdr = [0u8; 16];
            hdr[1..15].copy_from_slice(b"*NINTENDO-HVC*");
            let info = unsafe { fceux11_rust_fds_validate_header(hdr.as_ptr(), 16) };
            assert_eq!(info.kind, 2);
            assert_eq!(info.header_size, 0);
            assert_eq!(info.advertised_sides, 0);
        }
    }

    #[test]
    fn validate_header_no_magic() {
        unsafe {
            let hdr = [0u8; 16];
            let info = unsafe { fceux11_rust_fds_validate_header(hdr.as_ptr(), 16) };
            assert_eq!(info.kind, 0);
        }
    }

    #[test]
    fn validate_header_too_short() {
        unsafe {
            let hdr = [0u8; 4];
            let info = unsafe { fceux11_rust_fds_validate_header(hdr.as_ptr(), 4) };
            assert_eq!(info.kind, 0);
        }
    }

    #[test]
    fn validate_header_null_safe() {
        unsafe {
            let info = unsafe { fceux11_rust_fds_validate_header(std::ptr::null(), 16) };
            assert_eq!(info.kind, 0);
        }
    }

    // ----- compute_total_sides ------------------------------------------

    #[test]
    fn compute_total_sides_fds_header_within_range() {
        unsafe {
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 4, 1), 4);
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 1, 1), 1);
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 8, 1), 8);
        }
    }

    #[test]
    fn compute_total_sides_fds_header_clamps() {
        unsafe {
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 0, 1), 1);
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 9, 1), 8);
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 255, 1), 8);
        }
    }

    #[test]
    fn compute_total_sides_raw_small_file() {
        unsafe {
            // Even 0-byte file gets 1 side (matches C++ `if t < 65500: t = 65500`).
            assert_eq!(fceux11_rust_fds_compute_total_sides(0, 0, 0), 1);
            assert_eq!(fceux11_rust_fds_compute_total_sides(1000, 0, 0), 1);
            assert_eq!(fceux11_rust_fds_compute_total_sides(65499, 0, 0), 1);
        }
    }

    #[test]
    fn compute_total_sides_raw_exact_multiples() {
        unsafe {
            assert_eq!(fceux11_rust_fds_compute_total_sides(65500, 0, 0), 1);
            assert_eq!(fceux11_rust_fds_compute_total_sides(131000, 0, 0), 2);
            assert_eq!(fceux11_rust_fds_compute_total_sides(65500 * 4, 0, 0), 4);
        }
    }

    #[test]
    fn compute_total_sides_raw_clamps_to_8() {
        unsafe {
            assert_eq!(fceux11_rust_fds_compute_total_sides(65500 * 8, 0, 0), 8);
            assert_eq!(fceux11_rust_fds_compute_total_sides(65500 * 99, 0, 0), 8);
        }
    }

    // ----- block_size ---------------------------------------------------

    #[test]
    fn block_size_known_types() {
        unsafe {
            assert_eq!(
                fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_VOLUME, 0),
                0x38
            );
            assert_eq!(
                fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_FILECNT, 0),
                0x02
            );
            assert_eq!(
                fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_FILEHDR, 0),
                0x10
            );
        }
    }

    #[test]
    fn block_size_filedata_uses_filesize() {
        unsafe {
            assert_eq!(
                fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_FILEDATA, 0),
                0x01
            );
            assert_eq!(
                fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_FILEDATA, 100),
                0x65
            );
            assert_eq!(
                fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_FILEDATA, 0xFFFF),
                0x10000
            );
        }
    }

    #[test]
    fn block_size_dsk_init_is_zero() {
        unsafe {
            assert_eq!(fceux11_rust_fds_block_size(FCEUX11_RUST_FDS_DSK_INIT, 0), 0);
        }
    }

    #[test]
    fn block_size_unknown_type_is_zero() {
        unsafe {
            assert_eq!(fceux11_rust_fds_block_size(99, 0), 0);
        }
    }

    // ----- block_advance_on_motor ---------------------------------------

    #[test]
    fn block_advance_walks_normal_sequence() {
        unsafe {
            assert_eq!(
                fceux11_rust_fds_block_advance_on_motor(FCEUX11_RUST_FDS_DSK_INIT),
                FCEUX11_RUST_FDS_DSK_VOLUME
            );
            assert_eq!(
                fceux11_rust_fds_block_advance_on_motor(FCEUX11_RUST_FDS_DSK_VOLUME),
                FCEUX11_RUST_FDS_DSK_FILECNT
            );
            assert_eq!(
                fceux11_rust_fds_block_advance_on_motor(FCEUX11_RUST_FDS_DSK_FILECNT),
                FCEUX11_RUST_FDS_DSK_FILEHDR
            );
            assert_eq!(
                fceux11_rust_fds_block_advance_on_motor(FCEUX11_RUST_FDS_DSK_FILEHDR),
                FCEUX11_RUST_FDS_DSK_FILEDATA
            );
        }
    }

    #[test]
    fn block_advance_wraps_filedata_to_filehdr() {
        unsafe {
            assert_eq!(
                fceux11_rust_fds_block_advance_on_motor(FCEUX11_RUST_FDS_DSK_FILEDATA),
                FCEUX11_RUST_FDS_DSK_FILEHDR
            );
        }
    }

    #[test]
    fn block_advance_clamps_out_of_range() {
        unsafe {
            assert_eq!(
                fceux11_rust_fds_block_advance_on_motor(99),
                FCEUX11_RUST_FDS_DSK_FILEHDR
            );
        }
    }

    // ----- read_4030_value ----------------------------------------------

    #[test]
    fn read_4030_combinations() {
        unsafe {
            assert_eq!(fceux11_rust_fds_read_4030_value(false, false), 0);
            assert_eq!(fceux11_rust_fds_read_4030_value(true, false), 1);
            assert_eq!(fceux11_rust_fds_read_4030_value(false, true), 2);
            assert_eq!(fceux11_rust_fds_read_4030_value(true, true), 3);
        }
    }

    // ----- read_4032_value ----------------------------------------------

    #[test]
    fn read_4032_ejected() {
        unsafe {
            // InDisk == 255 forces both '5' and '2' bits.
            let ret = fceux11_rust_fds_read_4032_value(255, 0, 0xFF);
            assert_eq!(ret, !7 | 5 | 2);
        }
    }

    #[test]
    fn read_4032_inserted_motor_off() {
        unsafe {
            // FDSRegs[5] & 1 == 0 → set bit 2
            let ret = fceux11_rust_fds_read_4032_value(0, 0, 0);
            assert_eq!(ret, 2);
        }
    }

    #[test]
    fn read_4032_inserted_motor_on_bit2_clear() {
        unsafe {
            // FDSRegs[5] = 0x01 → bit 0 set (motor), bit 1 clear → bit 2 NOT set
            let ret = fceux11_rust_fds_read_4032_value(0, 0x01, 0);
            assert_eq!(ret, 0);
        }
    }

    #[test]
    fn read_4032_inserted_motor_on_bit2_set() {
        unsafe {
            // FDSRegs[5] = 0x03 → both bits set → bit 2 of return set
            let ret = fceux11_rust_fds_read_4032_value(0, 0x03, 0);
            assert_eq!(ret, 2);
        }
    }

    #[test]
    fn read_4032_data_bus_preserved() {
        unsafe {
            // Bits ≥3 of data_bus should pass through.
            let ret = fceux11_rust_fds_read_4032_value(0, 0x01, 0xF8);
            assert_eq!(ret & 0xF8, 0xF8);
        }
    }

    // ----- compute_select_disk_next -------------------------------------

    #[test]
    fn select_disk_next_cycles() {
        unsafe {
            assert_eq!(fceux11_rust_fds_compute_select_disk_next(0, 2), 1);
            assert_eq!(fceux11_rust_fds_compute_select_disk_next(1, 2), 0);
        }
    }

    #[test]
    fn select_disk_next_total_4_with_mask() {
        unsafe {
            // total=4 cycles through 0..3 (& 3 is a no-op on values <4)
            assert_eq!(fceux11_rust_fds_compute_select_disk_next(3, 4), 0);
        }
    }

    #[test]
    fn select_disk_next_total_8_mask_clips() {
        unsafe {
            // total=8, current=3 → (4 % 8) & 3 = 0
            assert_eq!(fceux11_rust_fds_compute_select_disk_next(3, 8), 0);
            // current=6 → (7 % 8) & 3 = 3
            assert_eq!(fceux11_rust_fds_compute_select_disk_next(6, 8), 3);
        }
    }

    #[test]
    fn select_disk_next_zero_total_safe() {
        unsafe {
            assert_eq!(fceux11_rust_fds_compute_select_disk_next(7, 0), 0);
        }
    }

    // ----- irq_tick -----------------------------------------------------

    #[test]
    fn irq_tick_null_safe() {
        unsafe {
            let r = unsafe { fceux11_rust_fds_irq_tick(std::ptr::null_mut(), 10) };
            assert!(!r.timer_fire);
            assert!(!r.seek_fire);
        }
    }

    #[test]
    fn irq_tick_disabled_is_noop_for_timer() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 100,
                irq_latch: 200,
                irq_a: 0, // disabled
                disk_seek_irq: 0,
                fds_regs_5: 0,
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(!r.timer_fire);
            assert_eq!(s.irq_count, 100); // unchanged
            assert_eq!(s.irq_a, 0);
        }
    }

    #[test]
    fn irq_tick_enabled_decrements_only() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 100,
                irq_latch: 200,
                irq_a: FCEUX11_RUST_FDS_IRQ_ENABLED,
                disk_seek_irq: 0,
                fds_regs_5: 0,
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(!r.timer_fire);
            assert_eq!(s.irq_count, 90);
            assert_eq!(s.irq_a, FCEUX11_RUST_FDS_IRQ_ENABLED); // still enabled
        }
    }

    #[test]
    fn irq_tick_enabled_no_repeat_fires_once_and_disables() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 5,
                irq_latch: 200,
                irq_a: FCEUX11_RUST_FDS_IRQ_ENABLED, // no IRQ_REPEAT
                disk_seek_irq: 0,
                fds_regs_5: 0,
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(r.timer_fire);
            assert_eq!(s.irq_count, 200); // reloaded from latch
            assert_eq!(s.irq_a, 0); // ENABLED cleared
        }
    }

    #[test]
    fn irq_tick_enabled_repeat_keeps_firing() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 5,
                irq_latch: 200,
                irq_a: FCEUX11_RUST_FDS_IRQ_ENABLED | FCEUX11_RUST_FDS_IRQ_REPEAT,
                disk_seek_irq: 0,
                fds_regs_5: 0,
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(r.timer_fire);
            assert_eq!(s.irq_count, 200);
            // Both ENABLED and REPEAT still set
            assert_eq!(
                s.irq_a,
                FCEUX11_RUST_FDS_IRQ_ENABLED | FCEUX11_RUST_FDS_IRQ_REPEAT
            );
        }
    }

    #[test]
    fn irq_tick_seek_fires_when_motor_bit_set() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 0,
                irq_latch: 0,
                irq_a: 0,
                disk_seek_irq: 5,
                fds_regs_5: 0x80, // motor running, seek IRQ enabled
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(r.seek_fire);
            assert_eq!(s.disk_seek_irq, -5);
        }
    }

    #[test]
    fn irq_tick_seek_no_fire_when_motor_bit_clear() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 0,
                irq_latch: 0,
                irq_a: 0,
                disk_seek_irq: 5,
                fds_regs_5: 0x00, // motor bit clear
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(!r.seek_fire);
            assert_eq!(s.disk_seek_irq, -5);
        }
    }

    #[test]
    fn irq_tick_seek_not_yet_zero() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 0,
                irq_latch: 0,
                irq_a: 0,
                disk_seek_irq: 100,
                fds_regs_5: 0x80,
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 10) };
            assert!(!r.seek_fire);
            assert_eq!(s.disk_seek_irq, 90);
        }
    }

    #[test]
    fn irq_tick_zero_cycles_is_noop_for_decrement() {
        unsafe {
            let mut s = FceuFdsIrqState {
                irq_count: 50,
                irq_latch: 200,
                irq_a: FCEUX11_RUST_FDS_IRQ_ENABLED,
                disk_seek_irq: 50,
                fds_regs_5: 0x80,
            };
            let r = unsafe { fceux11_rust_fds_irq_tick(&mut s, 0) };
            assert!(!r.timer_fire);
            assert!(!r.seek_fire);
            assert_eq!(s.irq_count, 50);
            assert_eq!(s.disk_seek_irq, 50);
        }
    }

    // ----- compute_write_4025 -------------------------------------------

    #[test]
    fn write_4025_disk_not_inserted_noop() {
        unsafe {
            let r = fceux11_rust_fds_compute_write_4025(
                FCEUX11_RUST_FDS_DSK_INIT,
                0,
                0,
                0x42, // motor on edge would normally fire
                0,    // disk_inserted = false
            );
            assert!(!r.motor_on_edge);
            assert!(!r.transfer_reset);
            assert!(!r.motor_on);
        }
    }

    #[test]
    fn write_4025_motor_on_edge_advances_block() {
        unsafe {
            // Current control bit 6 = 0, value bit 6 = 1 → edge
            let r = fceux11_rust_fds_compute_write_4025(
                FCEUX11_RUST_FDS_DSK_INIT,
                0,
                0x00, // current_control bit 6 = 0
                0x40, // value bit 6 = 1
                1,    // disk_inserted
            );
            assert!(r.motor_on_edge);
            assert!(!r.transfer_reset);
            assert!(r.motor_on);
            assert_eq!(r.new_block, FCEUX11_RUST_FDS_DSK_VOLUME);
            assert_eq!(r.new_blocklen, 0x38);
        }
    }

    #[test]
    fn write_4025_motor_on_no_edge() {
        unsafe {
            // Both old and new have bit 6 set → no edge
            let r = fceux11_rust_fds_compute_write_4025(
                FCEUX11_RUST_FDS_DSK_VOLUME,
                0,
                0x40, // current_control bit 6 = 1
                0x40, // value bit 6 = 1
                1,
            );
            assert!(!r.motor_on_edge);
            assert!(!r.transfer_reset);
            assert!(r.motor_on); // motor is on but no edge
            assert_eq!(r.new_block, FCEUX11_RUST_FDS_DSK_VOLUME); // unchanged
        }
    }

    #[test]
    fn write_4025_transfer_reset() {
        unsafe {
            let r = fceux11_rust_fds_compute_write_4025(
                FCEUX11_RUST_FDS_DSK_FILEHDR,
                100,
                0x00,
                0x02, // bit 1 = transfer reset
                1,
            );
            assert!(!r.motor_on_edge);
            assert!(r.transfer_reset);
            assert!(!r.motor_on);
        }
    }

    #[test]
    fn write_4025_filedata_blocklen_uses_filesize() {
        unsafe {
            // FILEHDR → FILEDATA on motor edge with filesize
            let r = fceux11_rust_fds_compute_write_4025(
                FCEUX11_RUST_FDS_DSK_FILEHDR,
                0x100, // file size = 256
                0x00,
                0x40,
                1,
            );
            assert!(r.motor_on_edge);
            assert_eq!(r.new_block, FCEUX11_RUST_FDS_DSK_FILEDATA);
            assert_eq!(r.new_blocklen, 0x101);
        }
    }

    #[test]
    fn write_4025_motor_off_clears_motor_on() {
        unsafe {
            let r = fceux11_rust_fds_compute_write_4025(
                FCEUX11_RUST_FDS_DSK_VOLUME,
                0,
                0x40,
                0x00, // bit 6 cleared
                1,
            );
            assert!(!r.motor_on_edge);
            assert!(!r.motor_on);
        }
    }

    // ----- constants smoke check ----------------------------------------

    #[test]
    fn constants_match_spec() {
        unsafe {
            assert_eq!(FCEUX11_RUST_FDS_DISK_SIDE_SIZE, 65500);
            assert_eq!(FCEUX11_RUST_FDS_BIOS_SIZE, 8192);
            assert_eq!(FCEUX11_RUST_FDS_RAM_SIZE, 32768);
            assert_eq!(FCEUX11_RUST_FDS_CHR_RAM_SIZE, 8192);
            assert_eq!(FCEUX11_RUST_FDS_MAX_SIDES, 8);
            assert_eq!(FCEUX11_RUST_FDS_DISK_SEEK_CYCLES, 150);
            assert_eq!(FCEUX11_RUST_FDS_NOT_INSERTED, 255);
            assert_eq!(FCEUX11_RUST_FDS_IRQ_REPEAT, 0x01);
            assert_eq!(FCEUX11_RUST_FDS_IRQ_ENABLED, 0x02);
            assert_eq!(FCEUX11_RUST_FDS_DSK_INIT, 0);
            assert_eq!(FCEUX11_RUST_FDS_DSK_VOLUME, 1);
            assert_eq!(FCEUX11_RUST_FDS_DSK_FILECNT, 2);
            assert_eq!(FCEUX11_RUST_FDS_DSK_FILEHDR, 3);
            assert_eq!(FCEUX11_RUST_FDS_DSK_FILEDATA, 4);
            assert_eq!(FCEUX11_RUST_FDS_FCEU_IQEXT, 0x01);
            assert_eq!(FCEUX11_RUST_FDS_FCEU_IQEXT2, 0x02);
        }
    }
}
