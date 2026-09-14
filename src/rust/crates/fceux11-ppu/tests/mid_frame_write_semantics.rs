//! Regression guards for the v2.1.3 batch 2.1 revert (plan §15.5 P0)
//! and the §15.7 red lines.
//!
//! Every test here drives the **real FFI entry points**
//! (`fceux11_ppu_cpu_write` / `fceux11_ppu_cpu_read`) — not the
//! register helpers directly — because the §15.4-6 audit found that
//! the batch 2.1 delay unit tests hardcoded the queue parameters and
//! never exercised the actual arming site in `ffi.rs`, letting hotfix
//! 2 change `v_addr_delay` 3→1 with all tests green.
//!
//! States are created through `fceux11_ppu_create()` because every
//! `fceux11_ppu_*` entry point validates its handle against the
//! registry and aborts on unknown pointers.
//!
//! What is pinned (reverted, synchronous semantics — plan §15.5):
//! 1. a rendering-time `$2006` second write commits `v = t`
//!    immediately and never arms the delay queue;
//! 2. a rendering-time `$2007` write increments `v` on the write dot;
//! 3. a rendering-time `$2007` read does a real bus read + increment;
//! 4. writes to `$2000`/`$2005` at the dot-257 boundary never corrupt
//!    `t` (the reverted race fired on a 3-dot window where the
//!    nesdev-authoritative transient glitch window is 257-258 only —
//!    §15.3);
//! 5. the batch 2.1 queue state fields stay disarmed after arbitrary
//!    register write sequences (they are kept serialized for savestate
//!    layout stability but must never influence behavior).

use fceux11_ppu::ffi::{fceux11_ppu_cpu_read, fceux11_ppu_cpu_write, fceux11_ppu_create, fceux11_ppu_destroy};
use fceux11_ppu::frame;
use fceux11_ppu::rendering::{tick_dot as render_dot, RenderWindows};
use fceux11_ppu::{FlatBus, PpuState};

/// Guard handle: the ffi-owned state, destroyed on drop.
struct FfiState(*mut PpuState);

impl FfiState {
    fn new() -> Self {
        unsafe { FfiState(fceux11_ppu_create()) }
    }
    fn state(&mut self) -> &mut PpuState {
        unsafe { &mut *self.0 }
    }
}

impl Drop for FfiState {
    fn drop(&mut self) {
        unsafe { fceux11_ppu_destroy(self.0) }
    }
}

/// Advance one full PPU dot (render tick + frame tick), mirroring the
/// scheduler's per-dot pairing.
fn advance_one_dot(state: &mut PpuState, bus: &mut FlatBus, win: &RenderWindows<'_>, fb: &mut [u8; 256 * 256]) {
    render_dot(state, bus, win, fb);
    frame::tick_dot(state, bus);
}

fn null_windows() -> RenderWindows<'static> {
    RenderWindows {
        nt: &[0u8; 4096],
        chr: &[0u8; 8192],
        palette: &[0u8; 32],
        mirror: 1,
    }
}

/// Visible scanline, mid-line dot, rendering on ($2001 bits 3-4), NMI off.
fn visible_rendering_state() -> FfiState {
    let mut s = FfiState::new();
    let st = s.state();
    st.ppudead = 0;
    st.scanline = 50;
    st.dot = 100;
    unsafe {
        fceux11_ppu_cpu_write(s.0, 0x2001, 0x1E);
    }
    s
}

#[test]
fn ffi_addr_second_write_commits_v_immediately_and_never_arms_queue() {
    // Plan §15.5: the rendering-time $2006 second write is synchronous
    // again on the real ffi path (the reverted 3-dot/1-dot queue must
    // not come back without probe evidence, §15.7).
    let mut s = visible_rendering_state();
    unsafe {
        fceux11_ppu_cpu_write(s.0, 0x2006, 0x21);
    }
    assert_eq!(s.state().registers.v, 0, "first write must not touch v");
    unsafe {
        fceux11_ppu_cpu_write(s.0, 0x2006, 0xCA);
    }
    assert_eq!(
        s.state().registers.v,
        0x21CA & 0x7FFF,
        "v = t on the write dot"
    );
    assert!(s.state().v_addr_pending.is_none(), "queue must stay disarmed");
    assert_eq!(s.state().v_addr_delay, 0, "queue must stay disarmed");
}

#[test]
fn ffi_data_write_increments_v_on_the_write_dot() {
    let mut s = visible_rendering_state();
    s.state().registers.v = 0x2000; // coarse_y=0, fv=0; rendering-on walk → fv becomes 1
    unsafe {
        fceux11_ppu_cpu_write(s.0, 0x2007, 0x42);
    }
    assert_eq!(
        s.state().registers.v,
        0x3000,
        "v incremented on the write dot (no 1-dot deferral)"
    );
    assert_eq!(
        s.state().vram_write_cooldown,
        0,
        "write cooldown must stay disarmed"
    );
}

#[test]
fn ffi_data_read_increments_v_on_every_access() {
    // The reverted 6-dot throttle froze `v` (and skipped the bus access)
    // for reads inside the cooldown window. With synchronous semantics
    // restored, EVERY $2007 read must do a real access and advance v —
    // three rapid reads walk fv 0→3 (v: $2000 → $3000 → $4000 → $5000);
    // under the old throttle reads 2-3 would have left v at $3000.
    // (The NT byte itself is not asserted: with no bus callbacks
    // installed the adapter reads return 0.)
    let mut s = visible_rendering_state();
    s.state().registers.v = 0x2000; // fv=0, coarse_y=0
    unsafe {
        for expected in [0x3000u16, 0x4000, 0x5000] {
            let _ = fceux11_ppu_cpu_read(s.0, 0x2007);
            assert_eq!(
                s.state().registers.v, expected,
                "every read must increment v (no 6-dot throttle)"
            );
        }
    }
    assert_eq!(
        s.state().vram_read_cooldown,
        0,
        "read cooldown must stay disarmed"
    );
}

#[test]
fn dot_257_boundary_writes_never_corrupt_t() {
    // §15.3 counter-example, pinned: $2000/$2005 writes landing at dots
    // 255/256/257 of a visible scanline must leave `t` untouched. The
    // reverted batch 2.1 race corrupted t.nt on exactly these dots
    // (its window was [255,257]; the nesdev-authoritative transient
    // glitch window is 257-258 and never persisted into `t`).
    let win = null_windows();
    let mut bus = FlatBus::new();
    let mut fb = [0u8; 256 * 256];
    for write_dot in [255u32, 256, 257] {
        let mut s = visible_rendering_state();
        let st = s.state();
        st.scanline = 100;
        st.dot = (write_dot - 1).max(1) as u16;
        st.registers.t = 0x2005; // nt bit 10 set; must survive unchanged
        let t_before = st.registers.t & 0x0C00;
        unsafe {
            fceux11_ppu_cpu_write(s.0, 0x2005, 0x00);
            fceux11_ppu_cpu_write(s.0, 0x2000, 0x90);
        }
        // advance to (and past) dot 258 of this scanline
        while (s.state().dot as u32) <= 258 {
            advance_one_dot(s.state(), &mut bus, &win, &mut fb);
        }
        assert_eq!(
            s.state().registers.t & 0x0C00,
            t_before,
            "dot {write_dot}: t.nametable bits must never be rewritten by the boundary writes"
        );
        assert_eq!(
            s.state().last_scroll_write_dot, 0xFFFF,
            "race bookkeeping must stay disarmed"
        );
    }
}

#[test]
fn queue_fields_stay_disarmed_after_full_register_sweep() {
    let mut s = visible_rendering_state();
    unsafe {
        for &addr in &[0x2000u16, 0x2001, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007] {
            fceux11_ppu_cpu_write(s.0, addr, 0x55);
            fceux11_ppu_cpu_write(s.0, addr, 0xAA);
        }
        let _ = fceux11_ppu_cpu_read(s.0, 0x2007);
    }
    let st = s.state();
    assert!(st.v_addr_pending.is_none());
    assert_eq!(st.v_addr_delay, 0);
    assert_eq!(st.vram_read_cooldown, 0);
    assert_eq!(st.vram_write_cooldown, 0);
    assert_eq!(st.last_scroll_write_dot, 0xFFFF);
}
