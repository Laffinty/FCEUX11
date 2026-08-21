//! C++ bus bridge for the FFI-driven 6502.
//!
//! When the Rust CPU is wired into the C++ executable (via
//! `FCEUX11_RUST_CPU=ON`), the Rust side has no idea what memory layout
//! or mapper is active — all memory access is mediated through
//! callbacks into the C++ `fceu11::Bus` instance. Those callbacks are
//! installed once at FFI init via [`fceux11_cpu_set_bus`]; the Rust
//! CPU then reads/writes the same memory the C++ CPU would (mappers,
//! PPU/APU open-bus, Lua hooks, etc.).
//!
//! ## API choice — non-nullable function pointers
//!
//! The C ABI cannot represent `Option<fn ...>` directly (cbindgen emits
//! those as opaque `struct Option_Foo`). Instead we use a sentinel
//! `noop_read` / `noop_write` pair: the C++ side passes the read/write
//! thunks at init time, and falls back to the no-op thunks if it has
//! no real handler to install. The `CppBus` always calls whatever is
//! installed, never branches on null.
//!
//! ## Thread safety
//!
//! Matches the C++ side: the emulator is single-threaded, the bus is a
//! global (`fceu11::g_bus`), and the bus callbacks are installed once.
//! No locking is performed — calling [`fceux11_cpu_run`] from a thread
//! other than the one that called [`fceux11_cpu_set_bus`] is undefined
//! behaviour, exactly as it is on the C++ side.

use crate::cpu::addressing::Bus;

/// C++ bus read callback signature. Installed by the C++ side at
/// `fceu11::Cpu::init()` time.
///
/// The C++ side passes a thunk that calls `fceu11::g_bus.read(addr)`
/// after the Lua / mapper hook chain (mirroring what `X6502_RunDebug`'s
/// `RdMem` inline function does).
pub type ReadFn = extern "C" fn(u16) -> u8;

/// C++ bus write callback signature. See [`ReadFn`].
pub type WriteFn = extern "C" fn(u16, u8);

/// Sentinel: return 0 on every read. Used as the initial callback
/// until the C++ side installs a real one.
extern "C" fn noop_read(_addr: u16) -> u8 {
    0
}

/// Sentinel: drop every write. Used as the initial callback until
/// the C++ side installs a real one.
extern "C" fn noop_write(_addr: u16, _val: u8) {}

// Phase 4 closeout: pointer to the C++ `X6502` blob for the current run,
// used to mirror the Rust DB latch into the blob before/after every bus
// access. The FFI runs the CPU on an internal `CpuState` copy and only
// writes the blob back at call end, so WITHOUT this mirror, mid-call C++
// bus handlers that read `g_cpu.native_layout().DB` (JPRead $4016,
// mapper open-bus, FDS sound, VSUni, cart open-bus) observe a stale DB
// and diverge from the C++ reference dispatch (nestest NMI handler).
// Null in pure-Rust tests; `sync_db_to_blob` is then a no-op.
static mut BLOB_PTR: *mut crate::cpu::state::X6502Layout = core::ptr::null_mut();

/// Set the C++ blob pointer for the duration of a run (FFI entry points).
pub(crate) unsafe fn set_blob_ptr(p: *mut crate::cpu::state::X6502Layout) {
    unsafe { BLOB_PTR = p; }
}

/// Mirror the Rust DB latch and cycle counter into the C++ blob.
/// Active only when the IRQ bridge is installed (the FFI/C++ path);
/// pure-Rust tests never install it, so the global `BLOB_PTR` cannot
/// leak between tests running in parallel.
pub(crate) fn sync_db_to_blob(db: u8, count: i32) {
    unsafe {
        if IRQ_BRIDGE_INSTALLED && !BLOB_PTR.is_null() {
            (*BLOB_PTR).db = db;
            (*BLOB_PTR).count = count;
        }
    }
}

// The two callback slots. `static mut` is acceptable here because:
//   1. The C++ emulator is single-threaded (mirrors `fceu11::g_bus`).
//   2. We only mutate from `fceux11_cpu_set_bus`, called once at init.
//   3. We only read from `CppBus::read` / `CppBus::write`, called on
//      the same thread by `fceux11_cpu_run`.
// `Cell`/`Atomic*` aren't necessary because we don't share this state
// across threads; if multi-CPU emulation is ever added, switch to
// `AtomicPtr<fn(...)>` and add a relaxed-acquire load on the read path.
static mut READ_FN: ReadFn = noop_read;
static mut WRITE_FN: WriteFn = noop_write;

/// Install / replace the C++ bus callbacks used by the FFI-driven CPU.
///
/// The C++ side should call this once during `fceu11::Initialize()`,
/// after `g_bus.init()` runs. Pass [`noop_read_thunk`] /
/// [`noop_write_thunk`] (re-exported below) to disable either
/// direction; reads then return `0`, writes are dropped.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_set_bus(read_fn: ReadFn, write_fn: WriteFn) {
    unsafe {
        READ_FN = read_fn;
        WRITE_FN = write_fn;
    }
}

/// Re-export of the no-op read callback so the C++ side can pass it
/// without having to write a Rust trampoline.
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_cpu_noop_read_thunk(_addr: u16) -> u8 {
    noop_read(_addr)
}

/// Re-export of the no-op write callback. See
/// [`fceux11_cpu_noop_read_thunk`].
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_cpu_noop_write_thunk(_addr: u16, _val: u8) {
    noop_write(_addr, _val)
}

/// The Rust-side bus that calls the installed C++ callbacks.
///
/// Constructed inline on every `fceux11_cpu_run` call; cheap because
/// it's a zero-sized type.
pub struct CppBus;

// IRQ-lookup callbacks for the C++ `X6502::IRQlow` blob. The C++ side's
// `X6502_IRQBegin`/`X6502_IRQEnd` (called from mapper hooks and the
// APU frame-counter IRQ during the tick bridge) mutate this blob while
// a Rust `run_with_tick` call is in flight; the Rust state snapshot
// taken at call start is stale, so `Bus::sync_irq_*` re-reads /
// re-writes it around every dispatch boundary. Implemented as callback
// slots (like `READ_FN`/`WRITE_FN`) rather than direct extern symbols
// so the crate still links in pure-Rust test binaries.
type IrqGetFn = extern "C" fn() -> u32;
type IrqSetFn = extern "C" fn(u32);

extern "C" fn noop_irq_get() -> u32 {
    0
}
extern "C" fn noop_irq_set(_v: u32) {}

static mut IRQ_GET_FN: IrqGetFn = noop_irq_get;
static mut IRQ_SET_FN: IrqSetFn = noop_irq_set;
// Set when the C++ side installs the IRQ bridge. When unset (pure-Rust
// test binaries), `sync_irq_*` are no-ops so the Rust state's own IRQ
// bits (e.g. RESET from `fceux11_cpu_power`) are preserved.
static mut IRQ_BRIDGE_INSTALLED: bool = false;

// Phase 4 closeout: NMI-fresh deferral flag bridge. The C++ VBL NMI
// path (`TriggerNMI`) sets `g_e1_nmi_fresh` so the reference dispatch
// defers the NMI by one instruction boundary; that flag lives outside
// the `X6502::IRQlow` blob, so `sync_irq_*` must carry it separately.
// Without this the Rust CPU dispatches the VBL NMI one boundary early
// (nestest NMI test diverges: pushed return PC differs).
type FreshGetFn = extern "C" fn() -> bool;
type FreshSetFn = extern "C" fn(bool);
extern "C" fn noop_fresh_get() -> bool { false }
extern "C" fn noop_fresh_set(_v: bool) {}
static mut FRESH_GET_FN: FreshGetFn = noop_fresh_get;
static mut FRESH_SET_FN: FreshSetFn = noop_fresh_set;
static mut FRESH_BRIDGE_INSTALLED: bool = false;

/// Install the IRQ-low read/write callbacks (C++ side calls this once
/// at init with `kagami_bridge_get_cpu_irq_low` /
/// `kagami_bridge_set_cpu_irq_low`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_set_irq_bridge(get_fn: IrqGetFn, set_fn: IrqSetFn) {
    unsafe {
        IRQ_GET_FN = get_fn;
        IRQ_SET_FN = set_fn;
        IRQ_BRIDGE_INSTALLED = true;
    }
}

/// Install the NMI-fresh flag read/write callbacks (C++ side calls this
/// once at init with `kagami_bridge_get_cpu_nmi_fresh` /
/// `kagami_bridge_set_cpu_nmi_fresh`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_cpu_set_nmi_fresh_bridge(
    get_fn: FreshGetFn,
    set_fn: FreshSetFn,
) {
    unsafe {
        FRESH_GET_FN = get_fn;
        FRESH_SET_FN = set_fn;
        FRESH_BRIDGE_INSTALLED = true;
    }
}

impl Bus for CppBus {
    #[inline]
    fn read(&mut self, addr: u16) -> u8 {
        // SAFETY: single-threaded; READ_FN is installed once at init
        // and never mutated again.
        unsafe { READ_FN(addr) }
    }

    #[inline]
    fn write(&mut self, addr: u16, val: u8) {
        // SAFETY: single-threaded; see `read`.
        unsafe { WRITE_FN(addr, val) }
    }

    #[inline]
    fn sync_irq_from_host(&mut self, state: &mut crate::cpu::addressing::CpuState) {
        // The C++ `X6502::IRQlow` blob is the source of truth for the
        // IRQ lines: mapper hooks and the APU frame counter (fired via
        // the tick bridge) set bits on it mid-call, and the $4017 write
        // path clears bits on it. Overwrite the Rust state so both the
        // new assertions AND the clears are visible to the next
        // dispatch. No-op when the bridge is not installed (tests).
        if unsafe { IRQ_BRIDGE_INSTALLED } {
            state.regs.irq_low = unsafe { IRQ_GET_FN() };
        }
        self.fresh_sync_from_host(state);
    }

    #[inline]
    fn sync_irq_to_host(&mut self, state: &mut crate::cpu::addressing::CpuState) {
        // Push back bits consumed by Rust's dispatch (e.g. NMI) so the
        // C++ blob doesn't re-assert them on the next call.
        if unsafe { IRQ_BRIDGE_INSTALLED } {
            unsafe { IRQ_SET_FN(state.regs.irq_low) };
        }
        self.fresh_sync_to_host(state);
    }

    #[inline]
    fn fresh_sync_from_host(&mut self, state: &mut crate::cpu::addressing::CpuState) {
        if unsafe { FRESH_BRIDGE_INSTALLED } {
            state.nmi_fresh = unsafe { FRESH_GET_FN() };
        }
    }

    #[inline]
    fn fresh_sync_to_host(&mut self, state: &mut crate::cpu::addressing::CpuState) {
        if unsafe { FRESH_BRIDGE_INSTALLED } {
            unsafe { FRESH_SET_FN(state.nmi_fresh) };
        }
    }
}

/// Test-only variant that ignores the installed callbacks and uses an
/// in-memory flat 64 KiB bus. Available under `#[cfg(test)]` so the
/// pure-Rust unit tests don't need a C++ main.
#[cfg(test)]
pub struct FlatBus {
    pub mem: [u8; 0x10000],
}

#[cfg(test)]
impl FlatBus {
    pub fn new() -> Self {
        Self { mem: [0; 0x10000] }
    }
}

#[cfg(test)]
impl Default for FlatBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl Bus for FlatBus {
    fn read(&mut self, addr: u16) -> u8 {
        self.mem[addr as usize]
    }
    fn write(&mut self, addr: u16, val: u8) {
        self.mem[addr as usize] = val;
    }
}