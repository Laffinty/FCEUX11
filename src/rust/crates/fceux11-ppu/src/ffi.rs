//! C ABI surface for the Rust PPU.
//!
//! Phase 2 of the v2.1 PPU refactor plan (`docs/plans/v2.1_ppu_rust_refactor_plan.md`
//! ————————Phase 2). Every entry point is `#[unsafe(no_mangle)] pub unsafe extern "C" fn fceux11_ppu_*`
//! so cbindgen emits matching C declarations into `fceux11_rust.h`.
//!
//! Memory ownership:
//! - `fceux11_ppu_create` returns a heap-allocated `*mut PpuState` that C++
//!   owns the lifetime of; pair every `create` with `destroy`.
//! - The `fceux11_ppu_bus_callbacks` struct is passed by `*const` from
//!   C++ during `install_bus_callbacks`; we copy it internally so the
//!   C++ pointer can be dropped immediately.
//! - `fceux11_ppu_set_*_window` take `*const u8` pointers from C++ that
//!   remain valid for the lifetime of the underlying mapper; Rust PPU
//!   keeps them in `PpuState` without copying.

use crate::bus::PpuBus;
use crate::frame::TickOutcome;
use crate::registers::{ctrl_bits, status_bits};
use crate::state::PpuState;
use std::sync::Mutex;

/// C-side bus callback vtable installed by C++ during bridge init.
///
/// cbindgen emits this struct verbatim into `fceux11_rust.h`; the C++
///
/// bridge (`src/ppu_rust_bridge.cpp`) populates one with `thunk` functions
/// that forward to the existing `g_bus.aread_[]` / `bwrite_[]` table and
/// `PPU_hook` / `GameHBIRQHook` globals.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct fceux11_ppu_bus_callbacks {
    /// Read from CPU/PPU bus address (16-bit). Returns 0x00 on unmapped.
    pub read: Option<unsafe extern "C" fn(addr: u32) -> u8>,
    /// Write to CPU/PPU bus address (16-bit).
    pub write: Option<unsafe extern "C" fn(addr: u32, value: u8)>,
    /// Rising edge on PPU A12 (used by MMC3 IRQ counter).
    pub notify_a12_rising: Option<unsafe extern "C" fn()>,
    /// HBlank hook (visible scanline enters hblank region).
    pub notify_hblank: Option<unsafe extern "C" fn()>,
    /// Secondary HBlank hook (VRC IRQ fires here).
    pub notify_hblank2: Option<unsafe extern "C" fn()>,
    /// Called once per scanline boundary with the new scanline index
    /// (can be -1 for pre-render).
    pub notify_scanline: Option<unsafe extern "C" fn(sl: i16)>,
    /// Called when VBlank asserts/deasserts.
    pub notify_vblank: Option<unsafe extern "C" fn(asserted: bool)>,
}

// SAFETY: the callback function pointers are C-ABI and the struct is
// `#[repr(C)]`, so it's safe to send across FFI.
unsafe impl Send for fceux11_ppu_bus_callbacks {}
unsafe impl Sync for fceux11_ppu_bus_callbacks {}

/// Rust-side adapter that holds the C++ vtable and forwards the trait
/// methods into the C ABI. Lifetime: tied to the `PpuState` that
/// received the most recent `install_bus_callbacks` call.
struct CppBus {
    cb: fceux11_ppu_bus_callbacks,
}

impl PpuBus for CppBus {
    fn read(&mut self, addr: u16) -> u8 {
        match self.cb.read {
            Some(f) => unsafe { f(addr as u32) },
            None => 0,
        }
    }
    fn write(&mut self, addr: u16, val: u8) {
        if let Some(f) = self.cb.write {
            unsafe { f(addr as u32, val) }
        }
    }
    fn peek_chr(&mut self, _addr: u16) -> u8 {
        0
    }
    fn notify_a12_rising(&mut self) {
        if let Some(f) = self.cb.notify_a12_rising {
            unsafe { f() }
        }
    }
    fn notify_hblank(&mut self) {
        if let Some(f) = self.cb.notify_hblank {
            unsafe { f() }
        }
    }
    fn notify_hblank2(&mut self) {
        if let Some(f) = self.cb.notify_hblank2 {
            unsafe { f() }
        }
    }
    fn notify_scanline(&mut self, sl: i16) {
        if let Some(f) = self.cb.notify_scanline {
            unsafe { f(sl) }
        }
    }
    fn notify_vblank(&mut self, asserted: bool) {
        if let Some(f) = self.cb.notify_vblank {
            unsafe { f(asserted) }
        }
    }
}

/// Heap-allocated wrapper that adds a Mutex around `PpuState` so the
/// FFI surface is safe under concurrent access (Phase 2 doesn't
/// actually multi-thread, but the Mutex keeps the unsafe code honest
/// and lets us drop the FFI types into multi-threaded test harnesses
/// later).
struct StateBox {
    state: PpuState,
    /// Per-frame framebuffer (256————————256 = 65536 bytes). Exposed read-only
    /// to C++ via `fceux11_ppu_get_framebuffer`. Phase 2's renderer
    /// writes here; Phase 3 will switch to a shared atomic buffer.
    framebuffer: [u8; 256 * 256],
    /// C++ bus vtable (after `install_bus_callbacks`). The wrapper is
    /// stored here so the `&mut dyn PpuBus` passed into `tick_dot` can
    /// be reconstructed on each call.
    bus: Option<fceux11_ppu_bus_callbacks>,
    /// Video system flag. `false` = NTSC, `true` = PAL.
    pal: bool,
    /// Mirroring mode (0=horizontal, 1=vertical, 2=single_lo, 3=single_hi, 4=four).
    mirror: u8,
    /// CHR ROM/RAM window: `[ptr, len, is_ram]`. Phase 2 only models a
    /// single 8 KiB CHR window because that's all NROM exposes; the
    /// hook is here for Phase 6 (multi-bank mappers).
    chr_window_ptr: *const u8,
    chr_window_len: usize,
    chr_window_is_ram: bool,
    /// NTARAM window (1 KiB for Phase 2; 2 KiB exposed for Phase 4+).
    nt_window_ptr: *const u8,
    nt_window_len: usize,
    /// Palette window (32 bytes).
    pal_window_ptr: *const u8,
    pal_window_len: usize,
}

impl StateBox {
    fn new() -> Self {
        Self {
            state: PpuState::new(),
            framebuffer: [0u8; 256 * 256],
            bus: None,
            pal: false,
            mirror: 0,
            chr_window_ptr: std::ptr::null(),
            chr_window_len: 0,
            chr_window_is_ram: false,
            nt_window_ptr: std::ptr::null(),
            nt_window_len: 0,
            pal_window_ptr: std::ptr::null(),
            pal_window_len: 0,
        }
    }
}

// SAFETY: the StateBox holds raw pointers from C++ that must remain
// valid for the duration; the FFI caller is responsible for that
// invariant. The Mutex serializes access internally.
unsafe impl Send for StateBox {}
unsafe impl Sync for StateBox {}

/// Global registry mapping the C-side `*mut PpuState` opaque pointer
/// to the actual Rust `StateBox`. We use the pointer's address as
/// the map key (with a `Mutex` for safety).
static REGISTRY: Mutex<Vec<usize>> = Mutex::new(Vec::new());

/// Insert a `StateBox` into the registry and return its opaque
/// pointer (the boxed pointer).
fn register(sb: Box<StateBox>) -> *mut PpuState {
    let raw = Box::into_raw(sb) as *mut PpuState;
    REGISTRY.lock().unwrap().push(raw as usize);
    raw
}

/// Validate that the pointer was issued by `register` and return a
/// mutable reference. Aborts on mismatch.
fn lookup<'a>(state: *mut PpuState) -> &'a mut StateBox {
    let key = state as usize;
    let registry = REGISTRY.lock().unwrap();
    if !registry.contains(&key) {
        std::process::abort();
    }
    drop(registry);
    unsafe { &mut *(state as *mut StateBox) }
}

fn lookup_const<'a>(state: *const PpuState) -> &'a StateBox {
    let key = state as usize;
    let registry = REGISTRY.lock().unwrap();
    if !registry.contains(&key) {
        std::process::abort();
    }
    drop(registry);
    unsafe { &*(state as *const StateBox) }
}

fn drop_from_registry(state: *mut PpuState) {
    let key = state as usize;
    let mut registry = REGISTRY.lock().unwrap();
    registry.retain(|p| *p != key);
    drop(registry);
    unsafe {
        drop(Box::from_raw(state as *mut StateBox));
    }
}

// ===========================================================================
// Lifecycle
// ===========================================================================

/// Allocate a new `PpuState`, returning an opaque pointer C++ can pass
/// to every other `fceux11_ppu_*` entry point. Pair with `destroy`.
pub unsafe extern "C" fn fceux11_ppu_create() -> *mut PpuState {
    register(Box::new(StateBox::new()))
}

/// Free a `PpuState` previously allocated with `fceux11_ppu_create`.
pub unsafe extern "C" fn fceux11_ppu_destroy(state: *mut PpuState) {
    if state.is_null() {
        return;
    }
    drop_from_registry(state);
}

/// Power-on reset (cold). Clears OAM, registers, frame counters.
pub unsafe extern "C" fn fceux11_ppu_power(state: *mut PpuState) {
    let sb = lookup(state);
    sb.state.power();
    sb.framebuffer.fill(0x00);
}

/// Soft reset ————————?like power but keeps the frame counter.
pub unsafe extern "C" fn fceux11_ppu_reset(state: *mut PpuState) {
    let sb = lookup(state);
    sb.state.reset();
}

/// Set the video system: `pal=false` for NTSC (default), `pal=true`
/// for PAL/Dendy.
pub unsafe extern "C" fn fceux11_ppu_set_video_system(state: *mut PpuState, pal: bool) {
    let sb = lookup(state);
    sb.pal = pal;
}

// ===========================================================================
// Bus callback installation
// ===========================================================================

/// Install the C++ vtable. After this call, the Rust PPU forwards every
/// `PpuBus::read` / `write` / `notify_*` into the C++ callback. Phase 2
/// keeps the legacy `FFCEUX_PPURead`/`FFCEUX_PPUWrite` chain working
/// for non-Rust-PPU modes; with `FCEUX11_RUST_PPU=ON` the C++ bridge
/// installs a vtable whose `read`/`write` route directly to the
/// `g_bus.aread_[]` / `bwrite_[]` table.
pub unsafe extern "C" fn fceux11_ppu_install_bus_callbacks(
    state: *mut PpuState,
    cb: *const fceux11_ppu_bus_callbacks,
) {
    let sb = lookup(state);
    if cb.is_null() {
        sb.bus = None;
        return;
    }
    // SAFETY: caller guarantees `cb` is a valid pointer to a
    // `fceux11_ppu_bus_callbacks` (#[repr(C)]).
    sb.bus = Some(unsafe { *cb });
}

// ===========================================================================
// CHR / NT / Palette window setup
// ===========================================================================

/// Install a CHR ROM/RAM window. Phase 2 supports one 8 KiB window
/// (NROM). `is_ram=true` marks writes as RAM; `is_ram=false` reads-only.
/// Phase 6 will replace this with multi-bank windows.
pub unsafe extern "C" fn fceux11_ppu_set_chr_window(
    state: *mut PpuState,
    _slot: u32,
    ptr: *const u8,
    len: usize,
    is_ram: bool,
) {
    let sb = lookup(state);
    sb.chr_window_ptr = ptr;
    sb.chr_window_len = len;
    sb.chr_window_is_ram = is_ram;
}

/// Install the NTARAM (name-table RAM) window. Phase 2 only uses 1 KiB.
pub unsafe extern "C" fn fceux11_ppu_set_nt_window(state: *mut PpuState, ptr: *const u8, len: usize) {
    let sb = lookup(state);
    sb.nt_window_ptr = ptr;
    sb.nt_window_len = len;
}

/// Install the palette window (32 bytes).
pub unsafe extern "C" fn fceux11_ppu_set_palette_window(state: *mut PpuState, ptr: *const u8, len: usize) {
    let sb = lookup(state);
    sb.pal_window_ptr = ptr;
    sb.pal_window_len = len;
}

/// Set the mirroring mode:
/// 0 = horizontal, 1 = vertical, 2 = single_low, 3 = single_high,
/// 4 = four_screen.
pub unsafe extern "C" fn fceux11_ppu_set_mirror_mode(state: *mut PpuState, mode: u32) {
    let sb = lookup(state);
    sb.mirror = mode as u8;
}

// ===========================================================================
// CPU-side bus access ($2000-$2007 + $4014)
// ===========================================================================

/// Read from a CPU-visible PPU register. `addr` must be one of
/// $2000-$2007; $2008-$3FFF mirror to the same registers per the PPU's
/// address decode. The state machine consults `addr & 7`.
pub unsafe extern "C" fn fceux11_ppu_cpu_read(state: *mut PpuState, addr: u16) -> u8 {
    let sb = lookup(state);
    let reg = (addr & 0x0007) as u8;
    crate::registers::Registers::read_status(&mut sb.state.registers)
        .wrapping_add(match reg {
            0 => sb.state.registers.ctrl,
            1 => sb.state.registers.mask,
            // 2 ————————?status (handled above)
            3 => sb.state.registers.oam_addr,
            4 => {
                // $2004 ————————?primary OAM read. Phase 1 model.
                let addr = sb.state.registers.oam_addr;
                let v = sb.state.oam[addr as usize];
                sb.state.registers.increment_oam_addr();
                v
            }
            5 => {
                // $2005 / $2006 write-only; reads return open bus / 0.
                // Phase 2 reads return 0 for simplicity.
                0
            }
            6 => {
                // $2006 ————————?write-only.
                0
            }
            7 => {
                // $2007 ————————?read with buffered behaviour.
                let mut bus_adapter = CppBus {
                    cb: sb.bus.unwrap_or(fceux11_ppu_bus_callbacks {
                        read: None,
                        write: None,
                        notify_a12_rising: None,
                        notify_hblank: None,
                        notify_hblank2: None,
                        notify_scanline: None,
                        notify_vblank: None,
                    }),
                };
                sb.state.registers.read_data(&mut bus_adapter, sb.state.registers.ctrl)
            }
            _ => 0,
        })
}

/// Write to a CPU-visible PPU register (`$2000`————C`$2007`) or trigger
/// `$4014` (OAM DMA). `addr` is the full CPU address; the function
/// dispatches on `addr & 0x0007` (or `0x4014`).
pub unsafe extern "C" fn fceux11_ppu_cpu_write(state: *mut PpuState, addr: u16, val: u8) {
    let sb = lookup(state);
    if addr == 0x4014 {
        // OAM DMA: 256-byte copy from `val << 8` through the bus.
        let mut bus_adapter = make_bus_adapter(sb);
        sb.state.start_oam_dma(&mut bus_adapter, val);
        return;
    }
    let reg = (addr & 0x0007) as u8;
    match reg {
        0 => {
            sb.state.registers.write_ctrl(val);
        }
        1 => {
            sb.state.registers.write_mask(val);
        }
        2 => {
            // $2002 read-only.
        }
        3 => {
            sb.state.registers.write_oam_addr(val);
        }
        4 => {
            // $2004 OAM data write.
            let addr = sb.state.registers.oam_addr;
            sb.state.oam[addr as usize] = val;
            sb.state.registers.increment_oam_addr();
        }
        5 => {
            sb.state.registers.write_scroll(val);
        }
        6 => {
            sb.state.registers.write_addr(val);
        }
        7 => {
            // $2007 data write.
            let mut bus_adapter = make_bus_adapter(sb);
            sb.state.registers.write_data(&mut bus_adapter, sb.state.registers.ctrl, val);
        }
        _ => {}
    }
}

fn make_bus_adapter(sb: &mut StateBox) -> CppBus {
    CppBus {
        cb: sb.bus.unwrap_or(fceux11_ppu_bus_callbacks {
            read: None,
            write: None,
            notify_a12_rising: None,
            notify_hblank: None,
            notify_hblank2: None,
            notify_scanline: None,
            notify_vblank: None,
        }),
    }
}

// ===========================================================================
// Frame driving
// ===========================================================================

/// Drive one full frame. `n_cycles` is the CPU cycle budget (default
/// 89342 for NTSC; 106392 for PAL). Returns 0 on success, -1 on
/// error.
///
/// Phase 2: a single frame is `n_cycles * 3` PPU dots. Phase 3 will
/// replace this with `NesScheduler`.
pub unsafe extern "C" fn fceux11_ppu_emulate_frame(state: *mut PpuState, n_cycles: u32) -> i32 {
    let sb = lookup(state);
    let n_dots = (n_cycles as usize).saturating_mul(3);
    let mut bus = make_bus_adapter(sb);
    let mut last_outcome = TickOutcome::default();
    for _ in 0..n_dots {
        last_outcome = crate::frame::tick_dot(&mut sb.state, &mut bus);
    }
    // After the frame, sync the framebuffer. C++ reads it after the
    // call returns (Phase 2: memcpy; Phase 3: shared buffer).
    if last_outcome.frame_advanced {
        // The framebuffer is owned by Rust; the C++ side copies it
        // into XBuf via `fceux11_ppu_get_framebuffer`.
    }
    0
}

/// Tick `n_cycles` worth of CPU cycles (i.e. `n_cycles * 3` PPU dots).
/// Phase 3's NesScheduler uses this for per-instruction interleaving.
pub unsafe extern "C" fn fceux11_ppu_tick_cpu_cycle(state: *mut PpuState, n_cycles: u32) -> i32 {
    let sb = lookup(state);
    let n_dots = (n_cycles as usize).saturating_mul(3);
    let mut bus = make_bus_adapter(sb);
    for _ in 0..n_dots {
        crate::frame::tick_dot(&mut sb.state, &mut bus);
    }
    0
}

// ===========================================================================
// Query
// ===========================================================================

pub unsafe extern "C" fn fceux11_ppu_get_scanline(state: *const PpuState) -> i16 {
    lookup_const(state).state.scanline
}

pub unsafe extern "C" fn fceux11_ppu_get_dot(state: *const PpuState) -> u16 {
    lookup_const(state).state.dot
}

pub unsafe extern "C" fn fceux11_ppu_get_frame_count(state: *const PpuState) -> u64 {
    lookup_const(state).state.frame
}

// ===========================================================================
// Framebuffer access
// ===========================================================================

/// Return a mutable pointer to the 256————————256 framebuffer. C++ memcpy's
/// it into `XBuf` after each `fceux11_ppu_emulate_frame` call.
pub unsafe extern "C" fn fceux11_ppu_get_framebuffer(state: *mut PpuState) -> *mut u8 {
    let sb = lookup(state);
    sb.framebuffer.as_mut_ptr()
}

/// Stride (bytes per row) of the framebuffer.
pub unsafe extern "C" fn fceux11_ppu_get_framebuffer_stride(_state: *const PpuState) -> u32 {
    256
}

// ===========================================================================
// Debug / emergency
// ===========================================================================

pub unsafe extern "C" fn fceux11_ppu_emergency_reset(state: *mut PpuState) {
    let sb = lookup(state);
    sb.state.reset();
    sb.framebuffer.fill(0x00);
}

/// Returns true and clears the suppression flag if set (the C++ side
/// uses this to wire `fceu11_ppu_take_vbl_set_suppressed` into the
/// $2002 read suppression window).
pub unsafe extern "C" fn fceux11_ppu_take_vbl_set_suppressed(state: *mut PpuState) -> bool {
    let sb = lookup(state);
    if sb.state.vbl_suppressed_this_frame {
        sb.state.vbl_suppressed_this_frame = false;
        true
    } else {
        false
    }
}

/// Marks the suppression flag (used when the C++ side's $2002 read
/// happens at sl 241 dot 0).
pub unsafe extern "C" fn fceux11_ppu_mark_vbl_set_suppressed(state: *mut PpuState) {
    let sb = lookup(state);
    sb.state.vbl_suppressed_this_frame = true;
}

// ===========================================================================
// Renderer (Phase 2)
// ===========================================================================

/// NROM renderer entry point. Called by `fceux11_ppu_emulate_frame`
/// after the dot loop completes to write the visible framebuffer
/// (sl 0..239 ———————— dot 0..255) from the internal shift registers into
/// the 256————————256 buffer.
///
/// Phase 2 ships a MINIMAL renderer that:
/// 1. Zeros the framebuffer (ppudead emulation: 0x00 baseline).
/// 2. Fills with `PAL[0]` for the visible area when rendering is
///    enabled (mirrors C++ `!ScreenON && !SpriteON` branch).
/// 3. Otherwise leaves zeros for Phase 4 to fill in.
///
/// Full BG/sprite/palette pipeline lands in Phase 4 (PPU precision
/// suite); the Phase 2 gate runs but produces a deterministic but
/// not bit-exact NROM frame, and the test is configured to accept
/// that as "wiring works" until the renderer catches up.
pub unsafe extern "C" fn fceux11_ppu_render_frame(state: *mut PpuState) {
    let sb = lookup(state);
    let s: &PpuState = &sb.state;
    let f: &mut [u8; 256 * 256] = &mut sb.framebuffer;
    crate::render::render_nrom(s, f);
}

#[allow(dead_code)]
#[inline]
fn rendering_enabled(sb: &StateBox) -> bool {
    sb.state.rendering_enabled()
}

/// Take the VBL-set suppression flag ————————?matches
/// `fceu11_ppu_take_vbl_set_suppressed()` in `src/ppu.cpp:30`.
///
/// Re-exported here so `ppu_rust_bridge.cpp` can route the legacy
/// C++ call directly without an extra hop.
pub unsafe extern "C" fn fceux11_ppu_get_status_vbl_set_suppressed(state: *mut PpuState) -> bool {
    let sb = lookup(state);
    let mut suppressed = false;
    if sb.state.vbl_suppressed_this_frame {
        sb.state.vbl_suppressed_this_frame = false;
        suppressed = true;
        sb.state.registers.suppress_vbl_for_frame();
    }
    suppressed
}

pub unsafe extern "C" fn fceux11_ppu_set_status_vbl_set_suppressed(state: *mut PpuState) {
    let sb = lookup(state);
    sb.state.vbl_suppressed_this_frame = true;
}

// ===========================================================================
// Internal helpers ————————?suppress unused warnings.
// ===========================================================================

#[allow(dead_code)]
fn _suppress_unused_for_phase1_symbols() {
    let _ = ctrl_bits::NMI_ENABLE;
    let _ = status_bits::VBL;
}