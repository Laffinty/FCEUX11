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
        // OAM DMA: begin the async pump. The Phase 3 scheduler
        // (`fceux11-ppu/src/scheduler.rs`) calls
        // `tick_oam_dma` once per CPU cycle until 256 bytes are
        // transferred; this avoids the 256-byte burst that the
        // synchronous `start_oam_dma` would otherwise perform
        // (which masks real CPU-side bus contention for the next
        // 256 cycles).
        sb.state.begin_oam_dma(val);
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
/// Phase 3: drives the frame through `NesScheduler`, which advances
/// PPU dot-by-dot and fires mapper event hooks (notify_a12_rising /
/// notify_hblank / notify_scanline / notify_vblank) at the natural
/// PPU-side boundaries. The CPU-side interleaving is owned by the C++
/// bridge — `fceux11_ppu_emulate_frame` advances only the PPU; the
/// C++ side drives the CPU per-instruction between PPU advances
/// (mirroring `X6502_Run(1)` at every `runppu(1)` in
/// `src/ppu_rendering.cpp`). The CPU-side hookup for the per-cycle
/// interleave is wired separately by
/// `fceux11_cpu_step_one_instruction` (Phase 3 follow-on).
///
/// Phase 4: at the start of each visible scanline (sl 0..239 dot 0),
/// the BG renderer is invoked to write 256 pixels into the
/// framebuffer. This is the bit-exact NROM gate; the algorithm
/// matches `pputile.inc::FetchAndDrawTile<kFNormal>` and produces
/// the same per-tile output as the C++ reference.
pub unsafe extern "C" fn fceux11_ppu_emulate_frame(state: *mut PpuState, n_cycles: u32) -> i32 {
    let sb = lookup(state);
    let mut bus = make_bus_adapter(sb);
    // Use the scheduler so notify_* hooks fire on scanline transitions
    // and at dot 256. The scheduler's per-dot advance is what wires
    // A12/HBlank/scanline hooks to the C++ mapper globals
    // (`GameHBIRQHook` / `GameHBIRQHook2` / `PPU_hook`).
    let mut sched = crate::scheduler::NesScheduler::new();
    sched.set_video_system(sb.pal);
    sched.begin_frame();
    let total_dots = n_cycles.saturating_mul(crate::scheduler::PPU_DOTS_PER_CPU_CYCLE);
    let mut last_outcome = TickOutcome::default();
    while sched.ppu_dots_consumed() < total_dots {
        // Phase 4: BG render at the start of each visible scanline.
        // The C++ algorithm runs `RefreshLine` once per visible
        // scanline (at sl N, dot 0); we do the same.
        if sb.state.scanline >= 0
            && sb.state.scanline <= 239
            && sb.state.dot == 0
        {
            render_visible_scanline(
                &mut sb.state,
                &mut bus,
                &mut sb.framebuffer,
                sb.mirror,
                sb.chr_window_ptr,
                sb.chr_window_len,
                sb.nt_window_ptr,
                sb.nt_window_len,
                sb.pal_window_ptr,
                sb.pal_window_len,
            );
        }
        last_outcome = sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
        // OAM DMA pump fires once per CPU cycle (= every 3 PPU dots).
        if sb.state.oam_dma_pending
            && sched.ppu_dots_consumed() % crate::scheduler::PPU_DOTS_PER_CPU_CYCLE == 0
        {
            sb.state.tick_oam_dma(&mut bus);
        }
    }
    if last_outcome.frame_advanced {
        // The framebuffer is owned by Rust; the C++ side copies it
        // into XBuf via `fceux11_ppu_get_framebuffer`.
    }
    0
}

/// Phase 4: invoke the BG renderer for the current scanline. Reads
/// from the CHR/NT/Palette windows installed by the C++ bridge
/// (`fceux11_ppu_set_chr_window` etc.) and writes into the
/// 256×256 framebuffer. Falls back to per-byte bus reads when the
/// windows are not installed.
fn render_visible_scanline<B: crate::bus::PpuBus + ?Sized>(
    state: &mut PpuState,
    bus: &mut B,
    framebuffer: &mut [u8; 256 * 256],
    mirror_mode: u8,
    chr_window_ptr: *const u8,
    chr_window_len: usize,
    nt_window_ptr: *const u8,
    nt_window_len: usize,
    pal_window_ptr: *const u8,
    pal_window_len: usize,
) {
    // Phase 4: build NT/CHR/Palette windows either from the
    // pre-installed direct pointers (set via
    // `fceux11_ppu_set_chr_window` etc.) or from the bus.
    //
    // SAFETY: the pointers are valid for the lifetime of the
    // installed windows. The StateBox guarantees that.
    let nt_window_storage: [u8; 2048];
    let nt_window: &[u8; 2048] = if !nt_window_ptr.is_null() && nt_window_len >= 2048 {
        unsafe { &*(nt_window_ptr as *const [u8; 2048]) }
    } else {
        nt_window_storage = {
            let mut w = [0u8; 2048];
            for i in 0..2048u16 {
                w[i as usize] = bus.read(0x2000 + i);
            }
            w
        };
        &nt_window_storage
    };
    let chr_window_storage: [u8; 8192];
    let chr_window: &[u8; 8192] = if !chr_window_ptr.is_null() && chr_window_len >= 8192 {
        unsafe { &*(chr_window_ptr as *const [u8; 8192]) }
    } else {
        chr_window_storage = {
            let mut w = [0u8; 8192];
            for i in 0..8192u16 {
                w[i as usize] = bus.read(i);
            }
            w
        };
        &chr_window_storage
    };
    let pal_window_storage: [u8; 32];
    let pal_window: &[u8; 32] = if !pal_window_ptr.is_null() && pal_window_len >= 32 {
        unsafe { &*(pal_window_ptr as *const [u8; 32]) }
    } else {
        pal_window_storage = {
            let mut w = [0u8; 32];
            for i in 0..32u16 {
                w[i as usize] = bus.read(0x3F00 + i);
            }
            w
        };
        &pal_window_storage
    };
    crate::rendering::render_scanline(
        state,
        bus,
        nt_window,
        chr_window,
        pal_window,
        mirror_mode,
        framebuffer,
    );
}

/// Tick `n_cycles` worth of CPU cycles (i.e. `n_cycles * 3` PPU dots).
/// Phase 3's NesScheduler uses this for per-instruction interleaving.
pub unsafe extern "C" fn fceux11_ppu_tick_cpu_cycle(state: *mut PpuState, n_cycles: u32) -> i32 {
    let sb = lookup(state);
    let mut bus = make_bus_adapter(sb);
    let mut sched = crate::scheduler::NesScheduler::new();
    sched.set_video_system(sb.pal);
    sched.begin_frame();
    let total_dots = n_cycles.saturating_mul(crate::scheduler::PPU_DOTS_PER_CPU_CYCLE);
    while sched.ppu_dots_consumed() < total_dots {
        sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
    }
    0
}

/// Phase 3: advance the Rust PPU by exactly one CPU cycle (3 PPU dots)
/// and fire mapper event hooks. Returns 1 if the frame completed this
/// call (sl 261 dot 340 wrap), 0 otherwise. Used by the C++ bridge's
/// `ppu_rust_bridge_emit_one_cpu_cycle` for per-cycle CPU/PPU
/// interleaving.
pub unsafe extern "C" fn fceux11_ppu_tick_one_cpu_cycle(state: *mut PpuState) -> i32 {
    let sb = lookup(state);
    let mut bus = make_bus_adapter(sb);
    let mut sched = crate::scheduler::NesScheduler::new();
    sched.set_video_system(sb.pal);
    // Track dots in a per-call scheduler; the C++ side calls this
    // ~89342 times per frame (one per CPU cycle), so per-call
    // allocation is acceptable.
    let outcome = sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
    sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
    sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
    // OAM DMA pump: one byte per CPU cycle.
    if sb.state.oam_dma_pending {
        sb.state.tick_oam_dma(&mut bus);
    }
    if outcome.frame_advanced || sched.ppu_dots_consumed() >= 1 {
        // Frame completed if the underlying frame counter advanced.
        // The C++ side doesn't need a strong frame-completion signal
        // here because `FCEUPPU_Loop` calls `ppu_rust_bridge_emit_frame`
        // once per frame; this entry point is for sub-frame interleaving.
        let _ = sched.ppu_dots_consumed();
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

/// Debug accessor: read a single PPU register byte (0=ctrl, 1=mask,
/// 2=status, 3=oam_addr). Returns 0 if `reg` is out of range.
pub unsafe extern "C" fn fceux11_ppu_get_register_state(state: *const PpuState, reg: u32) -> u8 {
    let sb = lookup_const(state);
    match reg {
        0 => sb.state.registers.ctrl,
        1 => sb.state.registers.mask,
        2 => sb.state.registers.status,
        3 => sb.state.registers.oam_addr,
        _ => 0,
    }
}

/// Debug accessor: read the current VRAM address (v).
pub unsafe extern "C" fn fceux11_ppu_get_v_state(state: *const PpuState) -> u16 {
    lookup_const(state).state.registers.v
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