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