//! RPU1 internal savestate module + LegacyView external interface.
//!
//! Phase 5.2 of the v2.1 PPU refactor plan
//! (`docs/plans/v2.1_ppu_rust_refactor_plan.md`). This module is
//! **option (a)** per plan §5.0.6: the RPU1 (Rust-PPU-1) chunk is held
//! internally for in-process checkpoint/restore tests, while the
//! legacy 264-byte view remains the only thing the C++ savestate
//! format ever carries. Keeping RPU1 off the wire is what guarantees
//! `golden_savestate_test` continues to match the Phase 4 byte-level
//! baselines — the `golden_savestate_test` 8-MD5 set is locked to
//! `06e9c9f` per Phase 5.0 and the only way to keep them green while
//! the Rust PPU gains internal round-trip fidelity is to never write
//! RPU1 into the .fc0 file.
//!
//! ## On-the-wire policy
//!
//! - **RPU1**: internal only. `serialize_rpu1` / `deserialize_rpu1`
//!   are exercised by `tests/snapshot_roundtrip.rs`. They are NOT
//!   wired into the C++ bridge's `state.cpp` Save/LoadState hooks in
//!   this phase.
//! - **LegacyView**: matches the canonical 264-byte packing for the
//!   fields that map directly to Rust `PpuState`. Two trailing
//!   padding bytes round the size to 264. Fields that exist only in
//!   C++ PPU state (`XOffset`, `PPUGenLatch`, `kook`, `ppudead`,
//!   `PPUSPL`) are not represented here; the legacy view is the
//!   Rust-side mirror of the registers/OAM/scroll bits the CPU can
//!   read back, not the full C++ new-PPU rendering state. The
//!   `RPU1` chunk is where that richer state lives — option (a)
//!   keeps it inside Rust.
//!
//! ## RPU1 wire format (internal)
//!
//! ```text
//! +--------+--------+--------+--------+
//! |  'R'   |  'P'   |  'U'   |  '1'   |  magic (4 bytes)
//! +--------+--------+--------+--------+
//! |   version (u32 LE, currently 1)   |
//! +-----------------------------------+
//! |   ts_ref (u32 LE, CPU timestamp)  |
//! +-----------------------------------+
//! |   payload_size (u32 LE)           |
//! +-----------------------------------+
//! |   payload (Payload; see below)    |
//! |   ...                             |
//! +-----------------------------------+
//! ```
//!
//! Payload layout (little-endian):
//!
//! | Offset | Size | Field |
//! |--------|------|-------|
//! | 0 | 16 | `Registers` (ctrl/mask/status/oam_addr + write_toggle/v/t/fine_x/vram_buffer) |
//! | 16 | 256 | `oam` |
//! | 272 | 32 | `secondary_oam` |
//! | 304 | 1 | `secondary_oam_count` |
//! | 305 | 1 | `sprite_overflow` |
//! | 306 | 1 | `sprite0_hit` |
//! | 307 | 1 | pad (alignment) |
//! | 308 | 2 | `scanline` (i16) |
//! | 310 | 2 | `dot` (u16) |
//! | 312 | 8 | `frame` (u64) |
//! | 320 | 1 | `vbl_suppressed_this_frame` |
//! | 321 | 1 | `nmi_pending` |
//! | 322 | 1 | `odd_frame` |
//! | 323 | 1 | `last_was_2005_or_2006` |
//! | 324 | 1 | `oam_dma_pending` |
//! | 325 | 1 | `oam_dma_page` |
//! | 326 | 2 | `oam_dma_counter` (u16) |
//! | 328 | 4 | `bg_pshift[0]` (u16) + pad |
//! | 332 | 4 | `bg_pshift[1]` (u16) + pad |
//! | 336 | 1 | `bg_atlatch` |
//! | 337 | 1 | `bg_next_nt` |
//! | 338 | 1 | `bg_next_at` |
//! | 339 | 1 | `bg_next_pattern_lo` |
//! | 340 | 1 | `bg_next_pattern_hi` |
//! | 341 | 1 | `bg_primed` |
//! | 342 | 1 | `bg_active` |
//! | 343 | 1 | pad |
//! | 344 | 16 | `sprite_shift[0..7][0/1]` |
//! | 360 | 8 | `sprite_attr[0..7]` |
//! | 368 | 8 | `sprite_x[0..7]` |
//! | 376 | 1 | `sprite0_in_range` |
//! | 377 | 1 | `sprite_eval_done` |
//! | 378 | 2 | trailing pad (so payload size is a multiple of 4) |
//!
//! Payload size = 380 bytes. Total RPU1 size = 4 (magic) + 4 (version)
//! + 4 (ts_ref) + 4 (payload_size) + 380 (payload) = 396 bytes.
//!
//! ## Versioning
//!
//! `RPU1_VERSION = 1`. A future v2 may extend the payload by appending
//! fields; v1 decoders must reject anything that is not exactly
//! `RPU1_VERSION`. v1 encoders must set `payload_size` to
//! `RPU1_PAYLOAD_SIZE`.
//!
//! ## Why `ts_ref` is part of the chunk
//!
//! The CPU timestamp at the snapshot point is what makes per-cycle
//! savestates restorable to the exact mid-frame state the mapper saw.
//! Plan §6.7 requires that loading an RPU1 record restore both the PPU
//! state AND the C++ CPU timestamp. The C++ side stashes this via
//! `fceux11_ppu_restore(... out_ts_ref: *mut u32)` and pushes it back
//! into the C++ `timestamp_ref`. For option (a) (internal-only) the
//! field is round-tripped in tests; if/when the C++ side opts into
//! using RPU1 on disk, the value lands in the C++ CPU view.
//!
//! ## `cargo test` consumers
//!
//! `tests/snapshot_roundtrip.rs` exercises:
//! 1. `rpu1_roundtrip_preserves_every_field` — full payload roundtrip
//! 2. `legacy_view_roundtrip` — 264-byte view roundtrip
//! 3. `legacy_view_pack_size_matches_contract` — size is 264
//! 4. `rpu1_rejects_bad_magic_or_short_buffer` — input validation

use crate::registers::Registers;
use crate::state::PpuState;

/// Magic bytes at the head of an RPU1 chunk.
pub const RPU1_MAGIC: [u8; 4] = *b"RPU1";

/// Current RPU1 format version. Bump on any payload change.
pub const RPU1_VERSION: u32 = 1;

/// Total size of the RPU1 payload (everything after the 16-byte
/// header). 380 bytes — see the layout table in the module docs.
pub const RPU1_PAYLOAD_SIZE: u32 = 380;

/// Total size of the RPU1 chunk on the wire (header + payload).
/// Used by callers that want to size an `out` buffer without doing
/// the field math themselves.
pub const RPU1_TOTAL_SIZE: u32 = 16 + RPU1_PAYLOAD_SIZE;

/// On-the-wire byte budget for an RPU1 chunk. Plan §5.0.6(a) keeps
/// this off the wire; the constant exists for callers that need to
/// allocate a buffer for in-process checkpointing.
pub const RPU1_CAP: u32 = RPU1_TOTAL_SIZE;

/// Byte length of the legacy view that maps to fields the CPU can
/// observe (registers[4] | oam[256] | write_toggle | vram_buffer |
/// 2 bytes trailing pad). 264.
pub const LEGACY_VIEW_LEN: usize = 264;

// ---------------------------------------------------------------------------
// RPU1 serialization
// ---------------------------------------------------------------------------

/// Outcome of an RPU1 serialize call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializeOutcome {
    /// Wrote `bytes_written` bytes into `out`; the snapshot carries
    /// `ts_ref` for the C++ side to round-trip into the CPU timestamp.
    Ok { bytes_written: u32, ts_ref: u32 },
    /// `out` is too small to hold the full RPU1 chunk. Required size
    /// returned for the caller's convenience.
    BufferTooSmall { required: u32 },
}

/// Serialize the full `PpuState` (plus `ts_ref`) into a byte buffer
/// in RPU1 format.
///
/// `out` must be at least `RPU1_TOTAL_SIZE` bytes long; `cap` is the
/// number of bytes available starting at `out`. On success the
/// function writes the full RPU1 chunk and returns
/// `Ok { bytes_written, ts_ref }`; on insufficient space it returns
/// `BufferTooSmall { required }` without writing anything.
pub fn serialize_rpu1(state: &PpuState, ts_ref: u32, out: &mut [u8]) -> SerializeOutcome {
    let required = RPU1_TOTAL_SIZE;
    if out.len() < required as usize {
        return SerializeOutcome::BufferTooSmall { required };
    }
    // Header
    out[0..4].copy_from_slice(&RPU1_MAGIC);
    out[4..8].copy_from_slice(&RPU1_VERSION.to_le_bytes());
    out[8..12].copy_from_slice(&ts_ref.to_le_bytes());
    out[12..16].copy_from_slice(&RPU1_PAYLOAD_SIZE.to_le_bytes());
    // Payload
    write_payload(&mut out[16..], state);
    SerializeOutcome::Ok {
        bytes_written: RPU1_TOTAL_SIZE,
        ts_ref,
    }
}

/// Outcome of an RPU1 deserialize call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeserializeOutcome {
    /// Restored `PpuState`; `ts_ref` is the value stashed by the
    /// saver and is intended for the C++ CPU timestamp view.
    Ok { ts_ref: u32 },
    /// `buf` was shorter than `RPU1_TOTAL_SIZE`.
    TooShort,
    /// First 4 bytes are not the RPU1 magic.
    BadMagic,
    /// Version field was not `RPU1_VERSION`. v1 only accepts v1.
    BadVersion { found: u32 },
    /// The size header didn't match `RPU1_PAYLOAD_SIZE`. A v1 chunk
    /// must carry exactly that.
    BadPayloadSize { found: u32 },
}

/// Restore the `PpuState` from an RPU1 byte buffer. The buffer must
/// be exactly `RPU1_TOTAL_SIZE` bytes; any deviation (short, bad
/// magic, bad version, bad payload_size) leaves `state` untouched.
pub fn deserialize_rpu1(state: &mut PpuState, buf: &[u8]) -> DeserializeOutcome {
    if buf.len() < RPU1_TOTAL_SIZE as usize {
        return DeserializeOutcome::TooShort;
    }
    if buf[0..4] != RPU1_MAGIC {
        return DeserializeOutcome::BadMagic;
    }
    let version = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if version != RPU1_VERSION {
        return DeserializeOutcome::BadVersion { found: version };
    }
    let ts_ref = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let payload_size = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
    if payload_size != RPU1_PAYLOAD_SIZE {
        return DeserializeOutcome::BadPayloadSize {
            found: payload_size,
        };
    }
    read_payload(&buf[16..], state);
    DeserializeOutcome::Ok { ts_ref }
}

// ---------------------------------------------------------------------------
// Payload writer / reader
// ---------------------------------------------------------------------------

fn write_payload(out: &mut [u8], state: &PpuState) {
    debug_assert_eq!(out.len(), RPU1_PAYLOAD_SIZE as usize);
    let mut off = 0usize;

    // Registers (16 bytes)
    out[off] = state.registers.ctrl;
    out[off + 1] = state.registers.mask;
    out[off + 2] = state.registers.status;
    out[off + 3] = state.registers.oam_addr;
    out[off + 4] = state.registers.write_toggle as u8;
    out[off + 5] = 0; // pad for alignment
    out[off + 6..off + 8].copy_from_slice(&state.registers.v.to_le_bytes());
    out[off + 8..off + 10].copy_from_slice(&state.registers.t.to_le_bytes());
    out[off + 10] = state.registers.fine_x;
    out[off + 11] = 0; // pad
    out[off + 12] = state.registers.vram_buffer;
    out[off + 13] = 0; // pad
    out[off + 14..off + 16].copy_from_slice(&[0, 0]); // pad to 16
    off += 16;

    // OAM (256 bytes)
    out[off..off + 256].copy_from_slice(&state.oam);
    off += 256;

    // Secondary OAM (32 bytes)
    out[off..off + 32].copy_from_slice(&state.secondary_oam);
    off += 32;

    // secondary_oam_count (1) + sprite_overflow (1) + sprite0_hit (1) + pad (1)
    out[off] = state.secondary_oam_count;
    out[off + 1] = state.sprite_overflow as u8;
    out[off + 2] = state.sprite0_hit as u8;
    out[off + 3] = 0;
    off += 4;

    // scanline (i16) + dot (u16)
    out[off..off + 2].copy_from_slice(&state.scanline.to_le_bytes());
    out[off + 2..off + 4].copy_from_slice(&state.dot.to_le_bytes());
    off += 4;

    // frame (u64)
    out[off..off + 8].copy_from_slice(&state.frame.to_le_bytes());
    off += 8;

    // Flags (1 each)
    out[off] = state.vbl_suppressed_this_frame as u8;
    out[off + 1] = state.nmi_pending as u8;
    out[off + 2] = state.odd_frame as u8;
    out[off + 3] = state.last_was_2005_or_2006 as u8;
    out[off + 4] = state.oam_dma_pending as u8;
    out[off + 5] = state.oam_dma_page;
    out[off + 6..off + 8].copy_from_slice(&state.oam_dma_counter.to_le_bytes());
    off += 8;

    // BG state
    out[off..off + 2].copy_from_slice(&state.bg_pshift[0].to_le_bytes());
    out[off + 2..off + 4].copy_from_slice(&[0, 0]);
    out[off + 4..off + 6].copy_from_slice(&state.bg_pshift[1].to_le_bytes());
    out[off + 6..off + 8].copy_from_slice(&[0, 0]);
    out[off + 8] = state.bg_atlatch;
    out[off + 9] = state.bg_next_nt;
    out[off + 10] = state.bg_next_at;
    out[off + 11] = state.bg_next_pattern_lo;
    out[off + 12] = state.bg_next_pattern_hi;
    out[off + 13] = state.bg_primed as u8;
    out[off + 14] = state.bg_active as u8;
    out[off + 15] = 0;
    off += 16;

    // Sprite shift / attr / x (16 + 8 + 8 = 32 bytes)
    for sprite in state.sprite_shift.iter() {
        out[off] = sprite[0];
        out[off + 1] = sprite[1];
        off += 2;
    }
    out[off..off + 8].copy_from_slice(&state.sprite_attr);
    off += 8;
    out[off..off + 8].copy_from_slice(&state.sprite_x);
    off += 8;

    // Final flags + trailing pad (4 bytes)
    out[off] = state.sprite0_in_range as u8;
    out[off + 1] = state.sprite_eval_done as u8;
    out[off + 2] = 0;
    out[off + 3] = 0;
    off += 4;

    debug_assert_eq!(off, RPU1_PAYLOAD_SIZE as usize);
}

fn read_payload(buf: &[u8], state: &mut PpuState) {
    debug_assert_eq!(buf.len(), RPU1_PAYLOAD_SIZE as usize);
    let mut off = 0usize;

    state.registers.ctrl = buf[off];
    state.registers.mask = buf[off + 1];
    state.registers.status = buf[off + 2];
    state.registers.oam_addr = buf[off + 3];
    state.registers.write_toggle = buf[off + 4] != 0;
    // off + 5 = pad
    state.registers.v = u16::from_le_bytes([buf[off + 6], buf[off + 7]]);
    state.registers.t = u16::from_le_bytes([buf[off + 8], buf[off + 9]]);
    state.registers.fine_x = buf[off + 10];
    // off + 11 = pad
    state.registers.vram_buffer = buf[off + 12];
    // off + 13..15 = pad
    off += 16;

    state.oam.copy_from_slice(&buf[off..off + 256]);
    off += 256;

    state.secondary_oam.copy_from_slice(&buf[off..off + 32]);
    off += 32;

    state.secondary_oam_count = buf[off];
    state.sprite_overflow = buf[off + 1] != 0;
    state.sprite0_hit = buf[off + 2] != 0;
    // off + 3 = pad
    off += 4;

    state.scanline = i16::from_le_bytes([buf[off], buf[off + 1]]);
    state.dot = u16::from_le_bytes([buf[off + 2], buf[off + 3]]);
    off += 4;

    state.frame = u64::from_le_bytes([
        buf[off],
        buf[off + 1],
        buf[off + 2],
        buf[off + 3],
        buf[off + 4],
        buf[off + 5],
        buf[off + 6],
        buf[off + 7],
    ]);
    off += 8;

    state.vbl_suppressed_this_frame = buf[off] != 0;
    state.nmi_pending = buf[off + 1] != 0;
    state.odd_frame = buf[off + 2] != 0;
    state.last_was_2005_or_2006 = buf[off + 3] != 0;
    state.oam_dma_pending = buf[off + 4] != 0;
    state.oam_dma_page = buf[off + 5];
    state.oam_dma_counter = u16::from_le_bytes([buf[off + 6], buf[off + 7]]);
    off += 8;

    state.bg_pshift[0] = u16::from_le_bytes([buf[off], buf[off + 1]]);
    // off + 2..4 = pad
    state.bg_pshift[1] = u16::from_le_bytes([buf[off + 4], buf[off + 5]]);
    // off + 6..8 = pad
    state.bg_atlatch = buf[off + 8];
    state.bg_next_nt = buf[off + 9];
    state.bg_next_at = buf[off + 10];
    state.bg_next_pattern_lo = buf[off + 11];
    state.bg_next_pattern_hi = buf[off + 12];
    state.bg_primed = buf[off + 13] != 0;
    state.bg_active = buf[off + 14] != 0;
    // off + 15 = pad
    off += 16;

    for sprite in state.sprite_shift.iter_mut() {
        sprite[0] = buf[off];
        sprite[1] = buf[off + 1];
        off += 2;
    }
    state.sprite_attr.copy_from_slice(&buf[off..off + 8]);
    off += 8;
    state.sprite_x.copy_from_slice(&buf[off..off + 8]);
    off += 8;

    state.sprite0_in_range = buf[off] != 0;
    state.sprite_eval_done = buf[off + 1] != 0;
    // off + 2..4 = trailing pad
    off += 4;

    debug_assert_eq!(off, RPU1_PAYLOAD_SIZE as usize);
}

// ---------------------------------------------------------------------------
// LegacyView — 264-byte packing matching the C++ FCEUPPU_STATEINFO fields
// the CPU can observe (registers, OAM, scroll bits). Not the full new-PPU
// rendering state — that's what RPU1 is for (kept internal under option (a)).
// ---------------------------------------------------------------------------

/// 264-byte CPU-observable view: `regs[4] | oam[256] | write_toggle |
/// vram_buffer | padding[2]`. Total 4+256+1+1+2 = 264.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyView {
    /// `$2000`-`$2003` mirrored (ctrl, mask, status, oam_addr).
    pub regs: [u8; 4],
    /// 256-byte primary OAM.
    pub oam: [u8; 256],
    /// `$2005`/`$2006` write toggle (the bit-4..=0 echo of `$2002`).
    pub write_toggle: bool,
    /// `$2007` open-bus / read buffer.
    pub vram_buffer: u8,
    /// Trailing pad so the packed size is 264 (matches plan §6.7).
    /// Reserved for fields the Rust side does not yet model
    /// (`XOffset`, `PPUGenLatch`); zero-filled.
    pub padding: [u8; 2],
}

impl Default for LegacyView {
    fn default() -> Self {
        Self {
            regs: [0; 4],
            oam: [0; 256],
            write_toggle: false,
            vram_buffer: 0,
            padding: [0; 2],
        }
    }
}

impl LegacyView {
    /// Byte length when packed (264).
    pub const PACKED_LEN: usize = LEGACY_VIEW_LEN;

    /// Copy the CPU-observable fields from `state` into `self`.
    /// Fields that exist only in C++ PPU state (`XOffset`,
    /// `PPUGenLatch`, `kook`, `ppudead`, `PPUSPL`) are zero-filled.
    pub fn export_from(&mut self, state: &PpuState) {
        self.regs = [
            state.registers.ctrl,
            state.registers.mask,
            state.registers.status,
            state.registers.oam_addr,
        ];
        self.oam.copy_from_slice(&state.oam);
        self.write_toggle = state.registers.write_toggle;
        self.vram_buffer = state.registers.vram_buffer;
        self.padding = [0; 2];
    }

    /// Apply this view to `state`. Fields not represented in
    /// `LegacyView` (scroll latches, secondary OAM, rendering state,
    /// DMA pump, frame counters) are left untouched.
    pub fn apply_to(&self, state: &mut PpuState) {
        state.registers.ctrl = self.regs[0];
        state.registers.mask = self.regs[1];
        state.registers.status = self.regs[2];
        state.registers.oam_addr = self.regs[3];
        state.oam.copy_from_slice(&self.oam);
        state.registers.write_toggle = self.write_toggle;
        state.registers.vram_buffer = self.vram_buffer;
    }
}

/// Convenience: copy CPU-observable fields from `state` into `view`.
pub fn export_legacy_view(state: &PpuState, view: &mut LegacyView) {
    view.export_from(state);
}

/// Convenience: apply `view` to `state` (CPU-observable fields only).
pub fn apply_legacy_view(state: &mut PpuState, view: &LegacyView) {
    view.apply_to(state);
}

/// Pack `view` into a 264-byte little-endian buffer. Returns `false`
/// if `buf` is not exactly `LEGACY_VIEW_LEN` bytes.
pub fn pack_legacy_view(view: &LegacyView, buf: &mut [u8]) -> bool {
    if buf.len() != LEGACY_VIEW_LEN {
        return false;
    }
    buf[0..4].copy_from_slice(&view.regs);
    buf[4..260].copy_from_slice(&view.oam);
    buf[260] = view.write_toggle as u8;
    buf[261] = view.vram_buffer;
    buf[262] = view.padding[0];
    buf[263] = view.padding[1];
    true
}

/// Unpack a 264-byte buffer back into a `LegacyView`. Returns `None`
/// on size mismatch.
pub fn unpack_legacy_view(buf: &[u8]) -> Option<LegacyView> {
    if buf.len() != LEGACY_VIEW_LEN {
        return None;
    }
    let mut view = LegacyView::default();
    view.regs.copy_from_slice(&buf[0..4]);
    view.oam.copy_from_slice(&buf[4..260]);
    view.write_toggle = buf[260] != 0;
    view.vram_buffer = buf[261];
    view.padding = [buf[262], buf[263]];
    Some(view)
}

// ---------------------------------------------------------------------------
// In-module tests — minimum surface, the bulk of coverage lives in
// `tests/snapshot_roundtrip.rs`. Pulled in via `#[cfg(test)]` so the
// release build doesn't carry them.
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn _ensure_registers_symbol(_: &Registers) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn total_size_matches_header_plus_payload() {
        assert_eq!(RPU1_TOTAL_SIZE, 16 + RPU1_PAYLOAD_SIZE);
        assert_eq!(RPU1_CAP, RPU1_TOTAL_SIZE);
    }

    #[test]
    fn legacy_view_packed_len_is_264() {
        assert_eq!(LegacyView::PACKED_LEN, 264);
    }

    #[test]
    fn pack_legacy_view_rejects_wrong_size() {
        let view = LegacyView::default();
        let mut too_small = [0u8; 100];
        assert!(!pack_legacy_view(&view, &mut too_small));
        let mut too_big = [0u8; 300];
        assert!(!pack_legacy_view(&view, &mut too_big));
        let mut exact = [0u8; 264];
        assert!(pack_legacy_view(&view, &mut exact));
    }
}
