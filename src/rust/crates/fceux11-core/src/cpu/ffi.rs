//! FFI surface for the Rust 6502 CPU.
//!
//! Phase 3 (revised) — step 2 of `docs/plans/cpu-rust-v2.md`. The
//! symbols here are what the C++ side calls when
//! `FCEUX11_RUST_CPU=ON` is set.
//!
//! ## Lifecycle
//!
//! 1. C++ side calls `fceux11_cpu_set_bus(read_fn, write_fn)` once at
//!    `fceu11::Initialize()`, after `g_bus.init()` is done.
//! 2. The existing `fceu11::Cpu` singleton continues to own its
//!    64-byte `X6502 layout_` blob — the FFI passes a pointer into
//!    it on every call.
//! 3. `Cpu::run(cycles)` calls `fceux11_cpu_run(&layout_, cycles)`,
//!    which copies the blob into a Rust-owned `CpuState`, drives the
//!    instruction loop, and copies the post-state back. The Rust
//!    `CpuState.nmi_fresh` flag persists across calls inside the
//!    FFI's static slot.
//!
//! ## Byte compatibility
//!
//! `X6502Layout` (Rust) and `X6502` (C++) are byte-compatible
//! (`#[repr(C, align(64))]` matches the C++ `alignas(64)`; field
//! offsets are pinned by `offset_of!` assertions in `state.rs`).
//! `fceux11_cpu_snapshot` / `fceux11_cpu_restore` are pure
//! `core::ptr::copy_nonoverlapping` over the 64-byte blob, so they are
//! automatically byte-equal to the C++ savestate path.
//!
//! ## Safety
//!
//! Every function takes a `*mut u8` / `*const u8` pointing at the C++
//! `X6502` blob. The caller (C++ side) must guarantee the pointer is
//! non-null, 8-byte-aligned (the struct's `alignas(64)` implies this),
//! and points at a live 64-byte `X6502` for the duration of the call.
//! Null pointers are tolerated by returning early, matching the
//! defensive behaviour of the legacy `X6502_*` free functions.
//!
//! Functions are not thread-safe — they rely on the
//! single-threaded-emulator invariant the C++ side already has.
//! `unsafe` on the `extern "C"` boundary documents that callers must
//! uphold the invariants; the FFI does no further synchronisation.

use crate::cpu::addressing::CpuState;
use crate::cpu::bus::CppBus;
use crate::cpu::execute::run;
use crate::cpu::state::{IrqSource, X6502Layout};

/// Size of the X6502 state blob. Mirrors `sizeof(X6502)` on the C++
/// side (pinned to 64 by `static_assert` in `src/cpu.cpp:21–23` and
/// `state.rs:81–94`).
pub const X6502_STATE_SIZE: usize = core::mem::size_of::<X6502Layout>();

// Compile-time assertion that the constant matches the layout size.
// Defensive: if anyone ever changes the layout to 128 bytes (e.g. to
// add debugger hooks), this constant must follow.
const _: () = {
    assert!(X6502_STATE_SIZE == 64, "X6502_LAYOUT_SIZE drifted from 64");
};

// ---------------------------------------------------------------------------
// Side-state slot. CpuState has fields beyond X6502Layout (`nmi_fresh`)
// that must persist across successive fceux11_cpu_run calls. The C++
// side has `g_e1_nmi_fresh` (a global bool); we mirror that pattern
// with a single static. See `cpu::addressing::CpuState` for the
// rationale on the `nmi_fresh` flag.
//
// Single-threaded by the same argument as `READ_FN` / `WRITE_FN` in
// `cpu/bus.rs`. We use `&raw mut` to satisfy the Rust 2024
// `static_mut_refs` deny lint — passing a raw pointer (rather than a
// `&mut` reference) avoids the aliasing-rule violations the lint is
// designed to catch, while still giving the same machine code.
// ---------------------------------------------------------------------------
static mut FFI_CPU_STATE: CpuState = CpuState {
    regs: X6502Layout::zeroed(),
    nmi_fresh: false,
    cycles_in_run: 0,
};

/// Initialize the 64-byte CPU state to all-zeros.
///
/// Mirrors `X6502_Init`'s `memset(0)` half. The ZN table is built at
/// crate-load time in Rust (`ZN_TABLE` is a `const`-built `[u8; 256]`)
/// and the table itself doesn't live in the state blob, so this is
/// all that's needed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_init(state: *mut u8) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = state as *mut X6502Layout;
        *layout = X6502Layout::zeroed();
        FFI_CPU_STATE.regs = X6502Layout::zeroed();
        FFI_CPU_STATE.nmi_fresh = false;
    }
}

/// Apply post-power-on state: S=0xFD, RESET bit set in `irq_low`.
///
/// Mirrors `X6502_Power`'s sequence:
/// ```c
/// _count=_tcount=_IRQlow=_PC=_A=_X=_Y=_P=_PI=_DB=_jammed=0;
/// _S=0xFD;
/// g_cpu.timestamp_ref()=g_cpu.sound_timestamp_ref()=0;
/// X6502_Reset();
/// StackAddrBackup = -1;
/// ```
/// Timestamps live on the C++ `Cpu` (timestamp_ref / sound_timestamp_ref
/// fields) and aren't part of the 64-byte blob, so we don't touch them
/// here. `StackAddrBackup` is a global in `x6502.cpp` that the C++ side
/// resets itself if it cares to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_power(state: *mut u8) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = &mut *(state as *mut X6502Layout);
        *layout = X6502Layout::zeroed();
        layout.s = 0xFD;
        layout.irq_low = IrqSource::RESET.bits();
        // Sync the side-state slot.
        FFI_CPU_STATE.regs = *layout;
        FFI_CPU_STATE.nmi_fresh = false;
    }
}

/// Apply warm reset.
///
/// Mirrors `X6502_Reset`: `_IRQlow = FCEU_IQRESET` (overwrite, not
/// OR — this clears any pending NMI/IRQ bits that may have been set
/// before the reset). The RESET bit gets consumed by the next
/// instruction boundary's dispatch loop.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_reset(state: *mut u8) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = &mut *(state as *mut X6502Layout);
        layout.irq_low = IrqSource::RESET.bits();
        FFI_CPU_STATE.regs = *layout;
    }
}

/// Set the NMI line.
///
/// Mirrors `TriggerNMI`: sets `FCEU_IQNMI` (0x080) and marks the
/// latch as fresh so the dispatch loop defers consumption to the
/// next instruction boundary (matches the C++ `g_e1_nmi_fresh`
/// semantics).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_trigger_nmi(state: *mut u8) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = &mut *(state as *mut X6502Layout);
        layout.irq_low |= IrqSource::NMI.bits();
        FFI_CPU_STATE.regs = *layout;
        FFI_CPU_STATE.nmi_fresh = true;
    }
}

/// Set the NMI2 (delayed-NMI) line. Mirrors `TriggerNMI2`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_trigger_nmi2(state: *mut u8) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = &mut *(state as *mut X6502Layout);
        layout.irq_low |= IrqSource::NMI2.bits();
        FFI_CPU_STATE.regs = *layout;
    }
}

/// Begin asserting an IRQ source (OR `src` into `irq_low`).
///
/// Mirrors `X6502_IRQBegin(int w)` — the C++ side passes a source
/// bitmask (FCEU_IQEXT=0x001, FCEU_IQDPCM=0x100, etc.).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_irq_begin(state: *mut u8, src: u32) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = &mut *(state as *mut X6502Layout);
        layout.irq_low |= src;
        FFI_CPU_STATE.regs = *layout;
    }
}

/// End an IRQ source (clear its bit in `irq_low`).
///
/// Mirrors `X6502_IRQEnd(int w)`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_irq_end(state: *mut u8, src: u32) {
    if state.is_null() {
        return;
    }
    unsafe {
        let layout = &mut *(state as *mut X6502Layout);
        layout.irq_low &= !src;
        FFI_CPU_STATE.regs = *layout;
    }
}

/// Run the CPU for `cycles` CPU cycles. Returns the total CPU
/// cycles consumed during this run (NOT the number of instructions).
///
/// The C++ side uses this return value to advance `Cpu::timestamp_`
/// / `sound_timestamp_` — those fields live on the `fceu11::Cpu`
/// object outside the 64-byte X6502 layout that this FFI operates
/// on, so the Rust side can't update them directly.
///
/// Mirrors `X6502_RunDebug(fceu11::Cpu&, int32 cycles)`'s loop body.
/// Each instruction boundary first dispatches any pending IRQ/NMI/
/// RESET, then fetches + executes the next instruction.
///
/// ## Cycle accounting — read this before debugging
///
/// The C++ `X6502_RunDebug` does `_count += cycles_arg * 16` at start
/// and `_count -= CycTable[opcode] * 48` per instruction. The two
/// multipliers (16 vs 48) are different units: 16 represents
/// "1/16-CPU-cycle units per input cycle", 48 represents
/// "1/48-CPU-cycle units per consumed cycle". Net: each input
/// CPU-cycle budget translates into 1/3 of a CPU cycle consumed
/// per call — the C++ empirically works because FCEUPPU_Loop makes
/// multiple `X6502_Run` calls per frame and the cumulative work
/// matches the expected per-frame cycle count.
///
/// The Rust `cpu::run()` uses the inverse convention: `count` is a
/// *cumulative* counter incremented by `CycTable * 16` per
/// instruction, and the loop exits when `count >= target`. To
/// produce the same "1/3 of cycles_arg CPU cycles consumed"
/// behaviour we pass `cycles_arg / 3 * 16` (i.e., the budget the
/// C++ would consume, scaled to Rust's 1/16-CPU-cycle unit).
///
/// Math: Rust terminates when `sum(CycTable * 16) >= target`, i.e.
/// when `sum(CycTable) >= target / 16`. We want
/// `sum(CycTable) == cycles_arg / 3`, so `target == cycles_arg / 3 * 16`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_run(state: *mut u8, cycles: i32) -> i32 {
    if state.is_null() || cycles <= 0 {
        return 0;
    }
    unsafe {
        // Pull the 64-byte blob into the side-state slot. We do this
        // every call so the C++ side can mutate the blob between calls
        // (it doesn't, but the ABI permits it).
        FFI_CPU_STATE.regs = *(state as *const X6502Layout);
        let mut bus = CppBus;
        // Use a raw pointer to the mutable static (Rust 2024
        // `static_mut_refs` deny lint forbids `&mut STATIC`).
        let state_ptr = core::ptr::addr_of_mut!(FFI_CPU_STATE);
        // Phase 4 sub-step 5: with the per-instruction 3x multiplier
        // in `step()` (count += dot(cycles) * 3 = CycTable * 48), each
        // Rust instruction costs the same number of 1/16-dot units as
        // the C++ `ADDCYC(CycTable)` (which subtracts CycTable * 48).
        // The previous `(cycles / 3) * 16` integer-truncation made
        // Rust run ceil((cycles/3)/2) instructions vs C++'s
        // ceil(cycles/3), an off-by-one for every cycles_arg ∈
        // {6k+1, 6k+2} call.
        let scaled_cycles = cycles * 16;
        let cpu_cycles = run(&mut *state_ptr, &mut bus, scaled_cycles);
        // Write the post-state back. Mirrors the C++ `X6502_RunDebug`
        // semantics where `cpu.layout_` is mutated in-place.
        *(state as *mut X6502Layout) = (*state_ptr).regs;
        cpu_cycles
    }
}

/// Snapshot the 64-byte CPU state to `out` (savestate path).
///
/// Pure `copy_nonoverlapping` — byte-identical to the C++ blob, since
/// `X6502Layout` (Rust) and `X6502` (C++) are pinned to the same
/// layout by `state.rs`'s `offset_of!` asserts and the C++
/// `static_assert`s in `src/cpu.cpp:11–22`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_snapshot(state: *const u8, out: *mut u8) {
    if state.is_null() || out.is_null() {
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(state, out, X6502_STATE_SIZE);
    }
}

/// Restore the 64-byte CPU state from `inp`. The inverse of
/// [`fceux11_cpu_snapshot`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_restore(state: *mut u8, inp: *const u8) {
    if state.is_null() || inp.is_null() {
        return;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(inp, state, X6502_STATE_SIZE);
        // Re-sync the side-state slot so a subsequent run() picks up the
        // restored regs and starts with a clean NMI-fresh flag.
        FFI_CPU_STATE.regs = *(state as *const X6502Layout);
        FFI_CPU_STATE.nmi_fresh = false;
    }
}

// ---------------------------------------------------------------------------
// Tests — exercise the FFI surface on a synthetic 64-byte buffer so we
// can build + run the crate without C++ linkage. These are pure-Rust
// unit tests (cargo test), not CTest.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::bus::{fceux11_cpu_set_bus, FlatBus};
    use crate::cpu::state::Flags;

    /// Mimic a 64-byte aligned buffer. The X6502Layout type is
    /// `#[repr(C, align(64))]`, so a Box<X6502Layout> gives us the
    /// alignment we need. We hold it in a Box and pass `&mut *box` as
    /// the `*mut u8` argument.
    fn make_state() -> Box<X6502Layout> {
        Box::new(X6502Layout::zeroed())
    }

    #[test]
    fn init_zeroes_state() {
        let mut buf = make_state();
        buf.p = 0xFF;
        unsafe {
            fceux11_cpu_init(&mut *buf as *mut X6502Layout as *mut u8);
        }
        assert_eq!(buf.p, 0);
        assert_eq!(buf.s, 0);
    }

    #[test]
    fn power_sets_reset_and_s() {
        let mut buf = make_state();
        unsafe {
            fceux11_cpu_power(&mut *buf as *mut X6502Layout as *mut u8);
        }
        assert_eq!(buf.s, 0xFD);
        assert_ne!(buf.irq_low & IrqSource::RESET.bits(), 0);
    }

    #[test]
    fn reset_only_sets_reset_bit() {
        let mut buf = make_state();
        buf.irq_low = IrqSource::NMI.bits(); // pre-existing NMI
        unsafe {
            fceux11_cpu_reset(&mut *buf as *mut X6502Layout as *mut u8);
        }
        // C++ X6502_Reset does an overwrite (`_IRQlow = FCEU_IQRESET`),
        // so the pre-existing NMI bit must be cleared and only the
        // RESET bit should remain.
        assert_eq!(buf.irq_low, IrqSource::RESET.bits());
    }

    #[test]
    fn trigger_nmi_sets_nmi_bit_and_fresh() {
        let mut buf = make_state();
        unsafe {
            fceux11_cpu_trigger_nmi(&mut *buf as *mut X6502Layout as *mut u8);
        }
        assert_ne!(buf.irq_low & IrqSource::NMI.bits(), 0);
        let state_ptr = core::ptr::addr_of_mut!(FFI_CPU_STATE);
        assert_eq!(
            unsafe { (*state_ptr).nmi_fresh },
            true,
            "trigger_nmi must set the nmi_fresh flag for next-boundary deferral"
        );
    }

    #[test]
    fn irq_begin_end_round_trip() {
        let mut buf = make_state();
        unsafe {
            fceux11_cpu_irq_begin(&mut *buf as *mut X6502Layout as *mut u8, 0x001);
        }
        assert_eq!(buf.irq_low & 0x001, 0x001);
        unsafe {
            fceux11_cpu_irq_end(&mut *buf as *mut X6502Layout as *mut u8, 0x001);
        }
        assert_eq!(buf.irq_low & 0x001, 0);
    }

    #[test]
    fn snapshot_restore_round_trip() {
        let mut src = make_state();
        src.p = 0xC0;
        src.a = 0x42;
        src.x = 0x99;
        src.pc = 0xABCD;
        let mut dst = make_state();
        unsafe {
            fceux11_cpu_snapshot(
                &*src as *const X6502Layout as *const u8,
                &mut *dst as *mut X6502Layout as *mut u8,
            );
        }
        assert_eq!(src.p, dst.p);
        assert_eq!(src.a, dst.a);
        assert_eq!(src.x, dst.x);
        assert_eq!(src.pc, dst.pc);

        // Restore into a third buffer that has its own state, then
        // verify it overwrote everything.
        let mut target = make_state();
        target.p = 0x00;
        target.a = 0x00;
        unsafe {
            fceux11_cpu_restore(
                &mut *target as *mut X6502Layout as *mut u8,
                &*dst as *const X6502Layout as *const u8,
            );
        }
        assert_eq!(target.p, 0xC0);
        assert_eq!(target.a, 0x42);
    }

    #[test]
    fn run_executes_nestest_reset_to_c000() {
        use crate::cpu::addressing::Bus;

        // The FFI read/write callbacks are `extern "C" fn` (no
        // closure captures), so we can't smuggle the FlatBus pointer
        // through the type system. Mirror the C++ side's pattern:
        // keep a single global `FlatBus` slot the trampoline reads
        // from, and copy our local bus into it before exercising.
        static mut BUS_PTR_FOR_TEST: FlatBus = FlatBus {
            mem: [0; 0x10000],
        };
        extern "C" fn read_trampoline(addr: u16) -> u8 {
            use crate::cpu::addressing::Bus;
            let bus_ptr = core::ptr::addr_of_mut!(BUS_PTR_FOR_TEST);
            unsafe { (*bus_ptr).read(addr) }
        }
        extern "C" fn write_trampoline(addr: u16, val: u8) {
            use crate::cpu::addressing::Bus;
            let bus_ptr = core::ptr::addr_of_mut!(BUS_PTR_FOR_TEST);
            unsafe { (*bus_ptr).write(addr, val) }
        }

        let mut bus = FlatBus::new();
        bus.mem[0xFFFC] = 0x00;
        bus.mem[0xFFFD] = 0xC0;
        // nestest first instruction: 4C F5 C5 (JMP $C5F5).
        bus.mem[0xC000] = 0x4C;
        bus.mem[0xC001] = 0xF5;
        bus.mem[0xC002] = 0xC5;

        unsafe {
            BUS_PTR_FOR_TEST = FlatBus { mem: bus.mem };
            fceux11_cpu_set_bus(read_trampoline, write_trampoline);
        }

        let mut state = make_state();
        unsafe {
            fceux11_cpu_power(&mut *state as *mut X6502Layout as *mut u8);
        }
        let consumed = unsafe {
            fceux11_cpu_run(
                &mut *state as *mut X6502Layout as *mut u8,
                // RESET handler (0 cycles — the dispatch doesn't
                // ADDCYC) + JMP abs (3 cycles) = 3 CPU cycles. The
                // FFI multiplies by 16 internally to match the C++
                // `_count += cycles * 16` unit convention.
                3,
            )
        };
        // The RESET dispatch eats 0 cycles; the only instruction
        // that runs is the JMP abs at $C000 (3 cycles).
        assert_eq!(consumed, 3, "RESET+JMP should consume 3 CPU cycles");
        assert_eq!(state.pc, 0xC5F5, "should land at $C5F5");
        assert_ne!(
            state.p & Flags::IRQ_DIS.bits(),
            0,
            "I flag should be set after RESET"
        );
    }

    #[test]
    fn null_state_is_tolerated() {
        // All entry points must early-return on null pointers rather
        // than crashing. This matches the C++ `X6502_*` functions'
        // defensive behaviour on bad inputs.
        unsafe {
            fceux11_cpu_init(core::ptr::null_mut());
            fceux11_cpu_power(core::ptr::null_mut());
            fceux11_cpu_reset(core::ptr::null_mut());
            fceux11_cpu_trigger_nmi(core::ptr::null_mut());
            fceux11_cpu_irq_begin(core::ptr::null_mut(), 0);
            fceux11_cpu_irq_end(core::ptr::null_mut(), 0);
            assert_eq!(fceux11_cpu_run(core::ptr::null_mut(), 100), 0);
            fceux11_cpu_snapshot(core::ptr::null(), core::ptr::null_mut());
            fceux11_cpu_restore(core::ptr::null_mut(), core::ptr::null());
        }
    }
}