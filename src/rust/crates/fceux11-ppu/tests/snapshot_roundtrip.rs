//! Integration tests for the Phase 5.2 snapshot module.
//!
//! Plan §5.2 `cargo test -p fceux11-ppu snapshot_roundtrip` §门槛
//! requires exactly 4 tests pass. Option (a) (plan §5.0.6) keeps
//! RPU1 internal-only, so these tests cover the in-memory round
//! trips the Rust crate needs for future checkpoint/restore without
//! touching the .fc0 file format the C++ side writes.
//!
//! 1. `rpu1_roundtrip_preserves_every_field` — set every PpuState
//!    field to a recognisable non-default value, serialize, deserialize
//!    into a fresh PpuState, then assert every field matches.
//! 2. `legacy_view_roundtrip` — pack a LegacyView into 264 bytes,
//!    unpack into a fresh view, apply to a fresh PpuState, assert
//!    the CPU-observable fields (regs[4], OAM, write_toggle,
//!    vram_buffer) match the source.
//! 3. `legacy_view_pack_size_matches_contract` — sanity check the
//!    on-the-wire size is 264 (matches plan §6.7 "264-byte fixed
//!    ABI packing").
//! 4. `rpu1_rejects_bad_magic_or_short_buffer` — confirm deserialize
//!    rejects too-short buffers, non-RPU1 magic, wrong version, and
//!    wrong payload size; that it leaves the destination state
//!    untouched on every failure path.

use fceux11_ppu::registers::{Registers, ctrl_bits, mask_bits};
use fceux11_ppu::snapshot::{
    DeserializeOutcome, LegacyView, RPU1_MAGIC, RPU1_PAYLOAD_SIZE, RPU1_TOTAL_SIZE, RPU1_VERSION,
    SerializeOutcome, apply_legacy_view, deserialize_rpu1, export_legacy_view, pack_legacy_view,
    serialize_rpu1, unpack_legacy_view,
};
use fceux11_ppu::state::{OAM_SIZE, PpuState, SECONDARY_OAM_SIZE};

// ---------------------------------------------------------------------------
// Test 1 — full RPU1 round trip
// ---------------------------------------------------------------------------

#[test]
fn rpu1_roundtrip_preserves_every_field() {
    let mut src = PpuState::new();
    // Phase 6.4: ppudead is not part of the RPU1 payload (savestates
    // are always captured past the process's first frame; read_payload
    // pins it to 0), so the roundtrip source models a post-boot state.
    src.ppudead = 0;

    // Registers
    src.registers.ctrl = 0b1010_0101;
    src.registers.mask = 1 << mask_bits::SHOW_BG | 1 << mask_bits::SHOW_SPRITES;
    src.registers.status = 0b1100_0000; // VBL + sprite0 hit
    src.registers.oam_addr = 0x42;
    src.registers.write_toggle = true;
    src.registers.v = 0x2CAB;
    src.registers.t = 0x3DEF;
    src.registers.fine_x = 5;
    src.registers.vram_buffer = 0x77;

    // OAM — every byte unique so a buggy serializer shows up
    for i in 0..OAM_SIZE {
        src.oam[i] = (i as u8).wrapping_mul(13).wrapping_add(7);
    }
    for i in 0..SECONDARY_OAM_SIZE {
        src.secondary_oam[i] = (i as u8).wrapping_mul(11).wrapping_add(3);
    }
    src.secondary_oam_count = 5;
    src.sprite_overflow = true;
    src.sprite0_hit = true;

    // Frame counters
    src.scanline = 173;
    src.dot = 287;
    src.frame = 0x0123_4567_89AB_CDEF;
    src.vbl_suppressed_this_frame = true;
    src.nmi_pending = true;
    src.odd_frame = true;
    src.last_was_2005_or_2006 = true;

    // DMA pump mid-flight
    src.oam_dma_pending = true;
    src.oam_dma_page = 0x07;
    src.oam_dma_counter = 199;

    // BG render state
    src.bg_pshift = [0xABCD, 0x1234];
    src.bg_atlatch = 0x5A;
    src.bg_next_nt = 0x11;
    src.bg_next_at = 0x22;
    src.bg_next_pattern_lo = 0x33;
    src.bg_next_pattern_hi = 0x44;
    src.bg_primed = true;
    src.bg_active = true;

    // Sprite render state — every byte unique
    for i in 0..8 {
        src.sprite_shift[i] = [(i as u8).wrapping_mul(17), (i as u8).wrapping_mul(23)];
        src.sprite_attr[i] = (i as u8).wrapping_mul(29);
        src.sprite_x[i] = (i as u8).wrapping_mul(31).wrapping_add(1);
    }
    src.sprite0_in_range = true;
    src.sprite_eval_done = true;

    // Serialize into a buffer of the right size.
    let mut buf = vec![0u8; RPU1_TOTAL_SIZE as usize];
    let ts_ref: u32 = 0xDEAD_BEEF;
    let outcome = serialize_rpu1(&src, ts_ref, &mut buf);
    match outcome {
        SerializeOutcome::Ok {
            bytes_written,
            ts_ref: out_ts,
        } => {
            assert_eq!(bytes_written, RPU1_TOTAL_SIZE);
            assert_eq!(out_ts, ts_ref);
        }
        SerializeOutcome::BufferTooSmall { required } => {
            panic!("serialize_rpu1 returned BufferTooSmall (required={required})");
        }
    }

    // Header bytes
    assert_eq!(&buf[0..4], &RPU1_MAGIC);
    assert_eq!(
        u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
        RPU1_VERSION
    );
    assert_eq!(
        u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
        ts_ref
    );
    assert_eq!(
        u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
        RPU1_PAYLOAD_SIZE
    );

    // Deserialize into a fresh PpuState.
    let mut dst = PpuState::new();
    let outcome = deserialize_rpu1(&mut dst, &buf);
    assert_eq!(outcome, DeserializeOutcome::Ok { ts_ref });

    // Registers
    assert_eq!(dst.registers.ctrl, src.registers.ctrl);
    assert_eq!(dst.registers.mask, src.registers.mask);
    assert_eq!(dst.registers.status, src.registers.status);
    assert_eq!(dst.registers.oam_addr, src.registers.oam_addr);
    assert_eq!(dst.registers.write_toggle, src.registers.write_toggle);
    assert_eq!(dst.registers.v, src.registers.v);
    assert_eq!(dst.registers.t, src.registers.t);
    assert_eq!(dst.registers.fine_x, src.registers.fine_x);
    assert_eq!(dst.registers.vram_buffer, src.registers.vram_buffer);

    // OAM
    assert_eq!(dst.oam, src.oam);
    assert_eq!(dst.secondary_oam, src.secondary_oam);
    assert_eq!(dst.secondary_oam_count, src.secondary_oam_count);
    assert_eq!(dst.sprite_overflow, src.sprite_overflow);
    assert_eq!(dst.sprite0_hit, src.sprite0_hit);

    // Frame counters
    assert_eq!(dst.scanline, src.scanline);
    assert_eq!(dst.dot, src.dot);
    assert_eq!(dst.frame, src.frame);
    assert_eq!(dst.vbl_suppressed_this_frame, src.vbl_suppressed_this_frame);
    assert_eq!(dst.nmi_pending, src.nmi_pending);
    assert_eq!(dst.odd_frame, src.odd_frame);
    assert_eq!(dst.last_was_2005_or_2006, src.last_was_2005_or_2006);

    // DMA pump
    assert_eq!(dst.oam_dma_pending, src.oam_dma_pending);
    assert_eq!(dst.oam_dma_page, src.oam_dma_page);
    assert_eq!(dst.oam_dma_counter, src.oam_dma_counter);

    // BG render state
    assert_eq!(dst.bg_pshift, src.bg_pshift);
    assert_eq!(dst.bg_atlatch, src.bg_atlatch);
    assert_eq!(dst.bg_next_nt, src.bg_next_nt);
    assert_eq!(dst.bg_next_at, src.bg_next_at);
    assert_eq!(dst.bg_next_pattern_lo, src.bg_next_pattern_lo);
    assert_eq!(dst.bg_next_pattern_hi, src.bg_next_pattern_hi);
    assert_eq!(dst.bg_primed, src.bg_primed);
    assert_eq!(dst.bg_active, src.bg_active);

    // Sprite render state
    assert_eq!(dst.sprite_shift, src.sprite_shift);
    assert_eq!(dst.sprite_attr, src.sprite_attr);
    assert_eq!(dst.sprite_x, src.sprite_x);
    assert_eq!(dst.sprite0_in_range, src.sprite0_in_range);
    assert_eq!(dst.sprite_eval_done, src.sprite_eval_done);

    // Round-trip identity — full struct equality is the strongest
    // possible assertion and catches any field the payload forgets.
    assert_eq!(src, dst);
}

// ---------------------------------------------------------------------------
// Test 2 — legacy view round trip (CPU-observable fields only)
// ---------------------------------------------------------------------------

#[test]
fn legacy_view_roundtrip() {
    let mut src = PpuState::new();

    // Distinct regs
    src.registers.ctrl = 1 << ctrl_bits::NMI_ENABLE; // 0x80
    src.registers.mask = 0b0001_1110;
    src.registers.status = 0b0100_0000; // sprite0 hit only
    src.registers.oam_addr = 0xAB;
    // Distinct OAM — byte 0x42 = 0xAA, byte 0xFF = 0xBB, etc.
    for i in 0..OAM_SIZE {
        src.oam[i] = ((i as u8) ^ 0x55).wrapping_add(1);
    }
    src.registers.write_toggle = true;
    src.registers.vram_buffer = 0xCD;

    // Pack to 264 bytes
    let mut view = LegacyView::default();
    export_legacy_view(&src, &mut view);
    let mut packed = vec![0u8; LegacyView::PACKED_LEN];
    assert!(pack_legacy_view(&view, &mut packed));

    // Unpack back into a fresh view
    let unpacked = unpack_legacy_view(&packed).expect("unpack should succeed on 264-byte buffer");

    // Pack bytes -> unpacked struct identity (catches layout bugs).
    assert_eq!(view, unpacked);

    // Apply to a fresh PpuState — must yield the source's CPU-observable
    // bits. Fields LegacyView doesn't model (scroll latches, secondary
    // OAM, rendering state, frame counters) start from `new()` defaults.
    let mut dst = PpuState::new();
    apply_legacy_view(&mut dst, &unpacked);

    assert_eq!(dst.registers.ctrl, src.registers.ctrl);
    assert_eq!(dst.registers.mask, src.registers.mask);
    assert_eq!(dst.registers.status, src.registers.status);
    assert_eq!(dst.registers.oam_addr, src.registers.oam_addr);
    assert_eq!(dst.oam, src.oam);
    assert_eq!(dst.registers.write_toggle, src.registers.write_toggle);
    assert_eq!(dst.registers.vram_buffer, src.registers.vram_buffer);

    // Scroll latches must remain at their PpuState::new() defaults —
    // LegacyView does NOT carry them and must NOT clobber them.
    assert_eq!(dst.registers.v, 0);
    assert_eq!(dst.registers.t, 0);
    assert_eq!(dst.registers.fine_x, 0);
}

// ---------------------------------------------------------------------------
// Test 3 — legacy view packed size is 264
// ---------------------------------------------------------------------------

#[test]
fn legacy_view_pack_size_matches_contract() {
    assert_eq!(LegacyView::PACKED_LEN, 264);
    assert_eq!(LegacyView::PACKED_LEN, 4 + 256 + 1 + 1 + 2);

    // Drive a non-default view through pack/unpack to confirm
    // size stability with non-zero data.
    let mut view = LegacyView::default();
    for i in 0..4 {
        view.regs[i] = (i as u8) + 1;
    }
    for i in 0..256 {
        view.oam[i] = (i as u8).wrapping_mul(2);
    }
    view.write_toggle = true;
    view.vram_buffer = 0xAB;
    view.padding = [0xDE, 0xAD];

    let mut buf = vec![0u8; 512];
    assert!(pack_legacy_view(&view, &mut buf[..LegacyView::PACKED_LEN]));
    assert_eq!(
        unpack_legacy_view(&buf[..LegacyView::PACKED_LEN]),
        Some(view.clone())
    );

    // A 263-byte buffer must be rejected
    let mut short = vec![0u8; 263];
    assert!(!pack_legacy_view(&view, &mut short));
    assert_eq!(unpack_legacy_view(&short), None);
}

// ---------------------------------------------------------------------------
// Test 4 — input validation: bad magic / version / payload size / short buf
// ---------------------------------------------------------------------------

#[test]
fn rpu1_rejects_bad_magic_or_short_buffer() {
    // --- Case A: buffer too short -----------------------------------------
    let mut state = PpuState::new();
    state.registers.ctrl = 0xFF; // sentinel: if validation is broken,
    // this will get overwritten to 0.
    let original = state.clone();

    let short_buf = vec![0u8; 32];
    assert_eq!(
        deserialize_rpu1(&mut state, &short_buf),
        DeserializeOutcome::TooShort
    );
    assert_eq!(state, original, "TooShort must not touch the destination");

    // --- Case B: bad magic -----------------------------------------------
    let mut buf = vec![0u8; RPU1_TOTAL_SIZE as usize];
    buf[0..4].copy_from_slice(b"NOPE");
    buf[4..8].copy_from_slice(&RPU1_VERSION.to_le_bytes());
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    buf[12..16].copy_from_slice(&RPU1_PAYLOAD_SIZE.to_le_bytes());

    let mut state = original.clone();
    assert_eq!(
        deserialize_rpu1(&mut state, &buf),
        DeserializeOutcome::BadMagic
    );
    assert_eq!(state, original, "BadMagic must not touch the destination");

    // --- Case C: wrong version -------------------------------------------
    let mut buf = vec![0u8; RPU1_TOTAL_SIZE as usize];
    buf[0..4].copy_from_slice(&RPU1_MAGIC);
    buf[4..8].copy_from_slice(&99u32.to_le_bytes()); // not 1
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    buf[12..16].copy_from_slice(&RPU1_PAYLOAD_SIZE.to_le_bytes());

    let mut state = original.clone();
    assert_eq!(
        deserialize_rpu1(&mut state, &buf),
        DeserializeOutcome::BadVersion { found: 99 }
    );
    assert_eq!(state, original, "BadVersion must not touch the destination");

    // --- Case D: wrong payload_size --------------------------------------
    let mut buf = vec![0u8; RPU1_TOTAL_SIZE as usize];
    buf[0..4].copy_from_slice(&RPU1_MAGIC);
    buf[4..8].copy_from_slice(&RPU1_VERSION.to_le_bytes());
    buf[8..12].copy_from_slice(&0u32.to_le_bytes());
    buf[12..16].copy_from_slice(&(RPU1_PAYLOAD_SIZE + 1).to_le_bytes());

    let mut state = original.clone();
    assert_eq!(
        deserialize_rpu1(&mut state, &buf),
        DeserializeOutcome::BadPayloadSize {
            found: RPU1_PAYLOAD_SIZE + 1
        }
    );
    assert_eq!(
        state, original,
        "BadPayloadSize must not touch the destination"
    );

    // --- Case E: empty buffer --------------------------------------------
    let mut state = original.clone();
    assert_eq!(
        deserialize_rpu1(&mut state, &[]),
        DeserializeOutcome::TooShort
    );
    assert_eq!(state, original);

    // --- Case F: a good buffer after a bad one still works ---------------
    let mut src = PpuState::new();
    src.registers.ctrl = 0xA5;
    src.registers.oam_addr = 0x42;
    src.scanline = 100;
    let mut good = vec![0u8; RPU1_TOTAL_SIZE as usize];
    serialize_rpu1(&src, 0x1234_5678, &mut good);

    let mut state = original.clone();
    assert_eq!(
        deserialize_rpu1(&mut state, &good),
        DeserializeOutcome::Ok {
            ts_ref: 0x1234_5678
        }
    );
    assert_eq!(state.registers.ctrl, 0xA5);
    assert_eq!(state.registers.oam_addr, 0x42);
    assert_eq!(state.scanline, 100);
}

// ---------------------------------------------------------------------------
// Bonus — `Registers` import sanity. The crate re-exports the register
// types so external test crates don't need to depend on private modules;
// this test just confirms the import paths we use above resolve.
// ---------------------------------------------------------------------------

#[test]
fn registers_type_is_reachable() {
    let _: Registers = Registers::new();
}
