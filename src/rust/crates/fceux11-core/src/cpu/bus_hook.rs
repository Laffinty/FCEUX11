//! Batch 3c.2 (v2.1.4 plan §2, milestone 3c.2): per-bus-access PPU
//! catch-up hook.
//!
//! On hardware the 6502 performs one bus access per CPU cycle and the
//! PPU advances 3 dots per cycle — the two clocks are phase-locked, so
//! every instruction's memory accesses land at exact PPU dots. Under
//! the atomic-instruction model the whole instruction executes at one
//! grant dot, which smears CPU-PPU relative timing by ±4 dots (plan
//! §5 3c.2: the vbl_02 visibility-table smear).
//!
//! The hook closes the gap for the dominant access class: `rd`/`wr`
//! invoke it once per bus access with the access's 1-based index
//! within the current instruction phase; the host (root crate's
//! interleave loop) advances the PPU by 3 dots per access beyond the
//! first, placing access *k* at grant_dot + 3(k−1) dots. Known
//! residual: instructions with internal cycles between accesses (RMW
//! modify cycle, branch taken, page-cross dummy) land ≤3 dots early —
//! refined in a later increment.
//!
//! Statics mirror the `tick.rs` `TICK_FN` slot pattern (single
//! emulator instance, installed on the emulator thread).

/// Hook: `(addr, is_write, access_index)` — called BEFORE the bus
/// access so the host can advance the PPU first.
pub type BusAccessHook = unsafe extern "C" fn(addr: u16, is_write: bool, access_index: u8);

static mut BUS_ACCESS_HOOK: Option<BusAccessHook> = None;

/// Install (or clear with `None`) the per-bus-access hook.
pub fn set_bus_access_hook(hook: Option<BusAccessHook>) {
    // SAFETY: single emulator instance; the hook is installed on the
    // emulator thread before the interleave loop starts (same contract
    // as the tick.rs slots).
    unsafe {
        BUS_ACCESS_HOOK = hook;
    }
}

/// Invoke the hook if installed. Called by [`CpuState::rd`] /
/// [`CpuState::wr`] before the bus access.
#[inline]
pub fn on_bus_access(addr: u16, is_write: bool, access_index: u8) {
    // SAFETY: see `set_bus_access_hook`.
    unsafe {
        if let Some(f) = BUS_ACCESS_HOOK {
            f(addr, is_write, access_index);
        }
    }
}
