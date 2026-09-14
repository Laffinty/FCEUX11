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
use crate::video_system::VideoSystem;
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
    /// CPU-address-space read (`g_bus.read` dispatch), used by the OAM
    /// DMA source fetch. Distinct from `read` (which routes PPU space
    /// through `FFCEUX_PPURead`) — the two spaces overlap in
    /// `$0000-$3FFF` and must not be confused.
    pub cpu_read: Option<unsafe extern "C" fn(addr: u32) -> u8>,
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
    /// v2.1.3 batch 1: window-dirty poll, fired by the per-dot loops
    /// every 8 dots (once per BG/sprite fetch group). C++ mapper bank
    /// switches set a shared atomic dirty flag; this callback re-copies
    /// the CHR/NT window pages whose base pointers moved, so a bank
    /// switch becomes visible to the renderer at fetch-group
    /// granularity instead of scanline granularity.
    pub refresh_windows: Option<unsafe extern "C" fn()>,
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
    fn read_cpu(&mut self, addr: u16) -> u8 {
        match self.cb.cpu_read {
            Some(f) => unsafe { f(addr as u32) },
            None => 0,
        }
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
    fn refresh_windows(&mut self) {
        if let Some(f) = self.cb.refresh_windows {
            unsafe { f() }
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
    /// Region timings (Step B.5-2a). The C ABI still takes the legacy
    /// `pal: bool` (NTSC/PAL); Dendy gains an explicit spelling in
    /// Step B.5-2b.
    video_system: VideoSystem,
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
    /// Phase 5.1: persistent per-dot scheduler. Lives across FFI calls
    /// so `notify_scanline` / `notify_vblank` fire only on true
    /// transitions between the ~89342 per-dot calls the C++ bridge
    /// makes per frame; a fresh scheduler per call would re-fire
    /// `notify_scanline` on every call (its `last_scanline` resets).
    sched: crate::scheduler::NesScheduler,
    /// Phase 6.3.a: absolute NTSC CPU cycle count, set by the C++
    /// bridge via `fceux11_ppu_set_current_cpu_cycle` once per CPU
    /// cycle (in the per-cycle interleave path). Used by the
    /// `fceux11_ppu_cpu_write` and `fceux11_ppu_check_data_bus_decay`
    /// entry points to refresh / age the open-bus latch with the
    /// correct timestamp without taking an extra parameter on the
    /// hot-path FFI surface.
    current_cpu_cycle: u64,
    /// Phase 6.3.b (scaffolding): most recent DMC DMA stall
    /// request, in CPU cycles. Set by `fceux11_ppu_dmc_dma_arbitration`
    /// when the C++ APU's `DMCDMA()` decides to fetch a DMC sample
    /// byte. The per-dot interleave loop currently does not consume
    /// this — once the Rust scheduler gains an
    /// `fceux11_cpu_advance_cycles(cpu_state, stall_cycles)` API,
    /// this field drives the "skip CPU exec, advance timestamp"
    /// branch. See `docs/history/v2.1_phase6_batch_compat.md` §6.3.b.
    dmc_dma_pending_stall: u8,
}

impl StateBox {
    fn new() -> Self {
        Self {
            state: PpuState::new(),
            framebuffer: [0u8; 256 * 256],
            bus: None,
            video_system: VideoSystem::Ntsc,
            mirror: 0,
            chr_window_ptr: std::ptr::null(),
            chr_window_len: 0,
            chr_window_is_ram: false,
            nt_window_ptr: std::ptr::null(),
            nt_window_len: 0,
            pal_window_ptr: std::ptr::null(),
            pal_window_len: 0,
            // begin_frame() arms the scanline sentinel so the very
            // first tick reports the power-on pre-render line (-1).
            sched: {
                let mut s = crate::scheduler::NesScheduler::new();
                s.begin_frame();
                s
            },
            current_cpu_cycle: 0,
            dmc_dma_pending_stall: 0,
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

/// Registry-check-free `lookup` for the Phase 5.3 per-dot hot path.
/// SAFETY (call sites): `state` must be a handle previously validated by
/// [`lookup`] (every entry point validates once at its front door; the
/// per-dot loop validates on frame entry).
fn lookup_unchecked<'a>(state: *mut PpuState) -> &'a mut StateBox {
    unsafe { &mut *(state as *mut StateBox) }
}

// ---------------------------------------------------------------------
// v2.1.3 batch 2.1: mid-frame write probe.
//
// When the environment variable `FCEUX11_MID_FRAME_WRITE_PROBE=1` is
// set, every CPU write to a rendering-time PPU register ($2000 /
// $2005 / $2006 / $2007) is logged to stderr with its
// `(scanline, dot, register, value, delayed_until_dot)` tuple. The
// probe is opt-in (zero overhead in production), and the output is
// read by `tools/run_batch21_regression.ps1 -Probe` to investigate
// per-ROM regressions that the synthetic tests don't cover. See
// `docs/plans/v2.1.3_ppu_accuracy_plan.md` §4 batch 2.1 gate 7.
// ---------------------------------------------------------------------
#[inline]
fn mid_frame_write_probe(
    sb: &StateBox,
    reg: &str,
    val: u8,
    delayed_until_dot: Option<u16>,
) {
    if !mid_frame_write_probe_enabled() {
        return;
    }
    let sl = sb.state.scanline;
    let dot = sb.state.dot;
    let cpu_cycle = sb.current_cpu_cycle;
    let pending = match (sb.state.v_addr_pending, sb.state.v_addr_delay) {
        (Some(v), d) if d > 0 => format!("v_pending=0x{:04X} v_delay={}", v & 0x7FFF, d),
        _ => String::new(),
    };
    let delay_str = match delayed_until_dot {
        Some(target) => format!(" →+{} dots", target.saturating_sub(dot)),
        None => String::new(),
    };
    eprintln!(
        "[mfw-probe] cpu={} sl={} dot={} reg={} val=0x{:02X}{}{} {}",
        cpu_cycle, sl, dot, reg, val, delay_str, if pending.is_empty() { "" } else { " " }, pending
    );
}

#[inline]
fn mid_frame_write_probe_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("FCEUX11_MID_FRAME_WRITE_PROBE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes"))
            .unwrap_or(false)
    })
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
/// for PAL. The two-state spelling cannot express Dendy - use
/// [`fceux11_ppu_set_video_system_ex`] for that (Step B.5-2b).
pub unsafe extern "C" fn fceux11_ppu_set_video_system(state: *mut PpuState, pal: bool) {
    let sb = lookup(state);
    let vs = VideoSystem::from_pal_flag(pal);
    sb.video_system = vs;
    sb.state.video_system = vs;
}

/// Write one byte of primary OAM (Step D / M3). The HexEditor and the PPU
/// viewer edit OAM directly; under the Rust PPU the C++ `SPRAM` global is a
/// tombstone, so those editors must route the write here.
pub unsafe extern "C" fn fceux11_ppu_set_oam_byte(state: *mut PpuState, addr: u32, value: u8) {
    let sb = lookup(state);
    sb.state.oam[(addr & 0xFF) as usize] = value;
}

/// CPU `count` budget units per PPU dot for the current region
/// (Step B.5-2c): 16 for the NTSC/Dendy 3.0 ratio, 15 for PAL's 3.2.
/// The unit is 1/48 of a CPU cycle (C++ `X6502._count` scale).
pub unsafe extern "C" fn fceux11_ppu_cpu_ticks_per_dot(state: *const PpuState) -> u32 {
    lookup_const(state).state.video_system.timings().cpu_ticks_per_dot()
}

/// Region selector with an explicit Dendy spelling (Step B.5-2b):
/// 0 = NTSC, 1 = PAL, 2 = Dendy. Out-of-range codes are ignored.
pub unsafe extern "C" fn fceux11_ppu_set_video_system_ex(state: *mut PpuState, system: u32) {
    let sb = lookup(state);
    if let Some(vs) = VideoSystem::from_code(system) {
        sb.video_system = vs;
        sb.state.video_system = vs;
    }
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
pub unsafe extern "C" fn fceux11_ppu_set_nt_window(
    state: *mut PpuState,
    ptr: *const u8,
    len: usize,
) {
    let sb = lookup(state);
    sb.nt_window_ptr = ptr;
    sb.nt_window_len = len;
}

/// Install the palette window (32 bytes).
pub unsafe extern "C" fn fceux11_ppu_set_palette_window(
    state: *mut PpuState,
    ptr: *const u8,
    len: usize,
) {
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

    // Plan §0.8 step 1D.1: $2002 suppression window. Delegates to
    // `PpuState::apply_a2002_suppression` so the logic lives on
    // the state (and is unit-testable without an unsafe StateBox).
    // Per NESdev PPU frame timing:
    //   (sl 240, dot 340) 1 dot before VBL set
    //     -> reads VBL=0 and never sets VBL/NMI for this frame
    //   (sl 241, dot 0/1) same dot or 1 dot later than the VBL set
    //     -> reads VBL=1, clears, suppresses NMI (the read pulls
    //        /NMI back up before the CPU samples it)
    if reg == 2 {
        sb.state.apply_a2002_suppression();
    }

    // Phase 6.4: register-read parity with the C++ handlers. The old
    // Phase 1 model called `read_status()` unconditionally and ADDED
    // its result to every register read: any $2000/$2001/$2003/$2005/
    // $2006/$2007 read also cleared the VBL flag, reset the write
    // toggle, overwrote the open-bus latch, and polluted the return
    // value with (status & 0xE0) | (latch & 0x1F). The gates only ever
    // exercised $2002 (reg==2 → `_ => 0` arm) so it survived. C++
    // reference semantics (src/ppu.cpp):
    //   A2002 → status | latch&0x1F, latch ← ret            (line 643)
    //   A2004 → OAM byte, latch ← ret, oam_addr NOT bumped  (line 771+)
    //   A200x ($2000-$2006 it doesn't handle) → PPUGenLatch (line 795)
    //   A2007 → buffered read; latch ← returned value       (line 875)
    let ret = match reg {
        2 => crate::registers::Registers::read_status(&mut sb.state.registers),
        4 => {
            // $2004 — primary OAM read (Phase 1 model: direct OAM
            // byte; the C++ rendering-on path returns the sprite-eval
            // byte spr_read.ret instead — a §6.2.4 follow-up). Reads
            // do NOT increment oam_addr, and the byte drives the
            // open-bus latch (blargg ppu_open_bus test 11).
            let a = sb.state.registers.oam_addr;
            let v = sb.state.oam[a as usize];
            sb.state.registers.data_bus = v;
            v
        }
        7 => {
            // $2007 — read with buffered behaviour.
            let v_before = sb.state.registers.v;
            let mut bus_adapter = CppBus {
                cb: sb.bus.unwrap_or(fceux11_ppu_bus_callbacks {
                    read: None,
                    write: None,
                    cpu_read: None,
                    notify_a12_rising: None,
                    notify_hblank: None,
                    notify_hblank2: None,
                    notify_scanline: None,
                    notify_vblank: None,
                    refresh_windows: None,
                }),
            };
            // v2.1.3 batch 2.1: rendering-time reads go through the
            // 6-PPU-dot throttle. `vram_read_cooldown` is set by the
            // per-dot consumer (`crate::rendering::tick_dot`) and
            // armed by every real read. Reads inside the cooldown
            // window return the buffered byte without touching the
            // bus or `v`. Rendering-off reads are unthrottled
            // (matches the pre-batch-2.1 behaviour and the C++ new
            // PPU's `_ignoreVramRead` gating on `PPU[1] & 0x18`).
            let throttle = rendering_2007(sb)
                && sb.state.vram_read_cooldown > 0;
            let ret = sb.state.registers.read_data_throttled(
                &mut bus_adapter,
                sb.state.registers.ctrl,
                rendering_2007(sb),
                throttle,
            );
            if !throttle {
                if rendering_2007(sb) {
                    // Arm the cooldown for the next 6 PPU dots.
                    sb.state.vram_read_cooldown = 6;
                }
                // v2.1.3 batch 1: the post-read v increment pushes
                // the new address onto the PPU bus when the PPU
                // isn't rendering — blargg MMC3 test 3 subtest 5
                // counts that 0→1 A12 jump. With rendering on, the
                // cooldown window starts now and `v` only changes
                // after the cooldown expires (handled implicitly by
                // the immediate read; the bus-watch call below
                // compares the post-increment v against the
                // pre-read v).
                if sb.state.registers.v != v_before {
                    a12_report_v(sb, &mut bus_adapter);
                }
            }
            return ret;
        }
        // $2000/$2001/$2003/$2005/$2006 — write-only or open bus; the
        // C++ A200x fallback returns PPUGenLatch for every register it
        // doesn't handle specially (src/ppu.cpp:795-800).
        _ => sb.state.registers.data_bus,
    };
    ret
}

/// v2.1.3 batch 1: true when the PPU bus carries `v` rather than fetch
/// addresses, i.e. CPU-driven `v` changes must be reported to the A12
/// watcher (blargg MMC3 test 3: `$2006` second write, `$2007` read and
/// `$2007` write all clock the counter whether or not rendering is on).
/// During rendering on a fetch line the bus stays fetch-driven and the
/// CPU write does not reach the cartridge address pins as a `v` push.
fn a12_reports_v(sb: &StateBox) -> bool {
    sb.state.scanline >= 240 || !sb.state.rendering_enabled()
}

/// Present a CPU-driven address to the watcher and forward a filtered
/// rising edge to the C++ mapper hook. `v` is the post-change value
/// already stored in `state.registers.v` by the caller.
fn a12_report_v(sb: &mut StateBox, adapter: &mut CppBus) {
    if a12_reports_v(sb) {
        let v = sb.state.registers.v & 0x3FFF;
        if sb.state.a12.observe(v) {
            adapter.notify_a12_rising();
        }
    }
}

/// Phase 5.1: take-and-clear the PPU NMI latch. Returns 1 when the
/// frame state machine asserted NMI since the last call (sl 241 dot 1
/// with NMI enabled and no VBL suppression); the C++ bridge forwards
/// this to the canonical `TriggerNMI()` so the CPU samples the NMI
/// line with the same one-instruction deferral as the C++ engines.
/// Exported through the root crate's cbindgen wrapper
/// (`src/rust/src/lib.rs`) — keep this one `no_mangle`-free so the
/// symbol is not doubly defined under LTO.
pub unsafe extern "C" fn fceux11_ppu_take_nmi_pending(state: *mut PpuState) -> i32 {
    let sb = lookup(state);
    let pending = sb.state.nmi_pending;
    sb.state.nmi_pending = false;
    pending as i32
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
        //
        // Phase 6.3.a: $4014 writes also drive the PPU internal
        // data-bus open-bus value (per nesdev wiki, any CPU write
        // to $4014 places the page byte on the I/O bus; reads of
        // $2005/$2006 then return this byte). Without this update
        // a DMA write followed by a $2005 read would leak the
        // previous write's value.
        sb.state.registers.refresh_data_bus(val, sb.current_cpu_cycle);
        sb.state.begin_oam_dma(val);
        return;
    }
    let reg = (addr & 0x0007) as u8;
    match reg {
        0 => {
            // NOTE (Phase 5.3): the "enable NMI while VBL flag is set"
            // edge belongs to the C++ bridge (B2000 parity — deferred
            // TriggerNMI2 with the C++ CPU's nmi_fresh semantics), NOT
            // here. See ppu_rust_bridge_cpu_write.
            sb.state.registers.write_ctrl(val);
            // v2.1.3 batch 2.1: record this write's dot for the
            // dot 257 open-bus race (rendering-time $2000 writes
            // within 0-2 dots of dot 257 cause t.nametable bits to
            // mix with the open-bus latch).
            if sb.state.rendering_enabled()
                && sb.state.scanline >= 0
                && sb.state.scanline < 240
            {
                sb.state.last_scroll_write_dot = sb.state.dot;
            }
        }
        1 => {
            // v2.1.3 batch 1: disabling rendering mid-frame drops the
            // bus from fetch addresses back to `v` (Mesen2
            // UpdateState → SetBusAddress(v)); report the level change
            // so a later `$2006`-driven rise still measures against a
            // fresh low period.
            let was_rendering = sb.state.rendering_enabled();
            sb.state.registers.write_mask(val);
            if was_rendering && !sb.state.rendering_enabled() && sb.state.scanline < 240 {
                let v = sb.state.registers.v & 0x3FFF;
                let mut bus_adapter = make_bus_adapter(sb);
                if sb.state.a12.observe(v) {
                    bus_adapter.notify_a12_rising();
                }
            }
            // v2.1.3 batch 2.1: rendering off clears the throttle so
            // a later $2007 read (e.g. reading palette entries) is
            // not artificially held back by a stale window from the
            // last visible-line read.
            if !sb.state.rendering_enabled() {
                sb.state.vram_read_cooldown = 0;
            }
        }
        2 => {
            // $2002 read-only.
        }
        3 => {
            sb.state.registers.write_oam_addr(val);
        }
        4 => {
            // $2004 OAM data write. Also updates the PPU internal
            // data bus (Phase 6.3.a) so subsequent $2005/$2006 reads
            // return the value just written to OAM.
            sb.state.registers.refresh_data_bus(val, sb.current_cpu_cycle);
            let addr = sb.state.registers.oam_addr;
            sb.state.oam[addr as usize] = val;
            sb.state.registers.increment_oam_addr();
        }
        5 => {
            // v2.1.3 batch 2.1: $2005 always commits to t
            // immediately (its `fine_x` is observed by the BG fetch
            // pipeline on the very next dot). What we need to
            // record is "did a $2005 write happen near dot 257?", to
            // drive the open-bus race in `crate::rendering::tick_dot`.
            sb.state.registers.write_scroll(val);
            if sb.state.rendering_enabled()
                && sb.state.scanline >= 0
                && sb.state.scanline < 240
            {
                sb.state.last_scroll_write_dot = sb.state.dot;
            }
        }
        6 => {
            // v2.1.3 batch 2.1: $2006 second write commits `v = t`
            // after a 3-PPU-dot delay when rendering is on (Mesen2
            // VisualNES calibration, KB
            // `ppu/ppu_mid_frame_writes.md` §1.1). Rendering-off
            // writes are still synchronous so blargg MMC3 test 3
            // subtests 2-4 can clock the A12 counter on the same
            // CPU instruction.
            let v_before = sb.state.registers.v;
            let mut bus_adapter = make_bus_adapter(sb);
            if rendering_2007(sb) {
                if let Some(pending) = sb
                    .state
                    .registers
                    .write_addr_rendering(val)
                {
                    // Second write: queue the v commit and arm the
                    // 1-PPU-dot countdown (Lidnariq PPU_glitches
                    // wiki: "on the next pixel"). The per-dot
                    // consumer in `crate::rendering::tick_dot` will
                    // commit when the counter reaches 0. The KB §1.1
                    // value of 3 PPU dots is the Mesen2 VisualNES
                    // calibration, which folds several 2C02 pipeline
                    // effects (early-write open-bus, the dot 257/258
                    // t-coarse-X shoot-through) into a single
                    // 3-dot budget; the bare-hardware 2C02 commits
                    // on the next pixel.
                    sb.state.v_addr_pending = Some(pending);
                    sb.state.v_addr_delay = 1;
                    mid_frame_write_probe(
                        sb,
                        "$2006",
                        val,
                        Some(sb.state.dot.wrapping_add(3)),
                    );
                    // A12 reporting for the rendering-off-style v
                    // push does NOT fire here — the address bus
                    // remains fetch-driven through the delay. The
                    // delayed commit reports A12 in the per-dot
                    // consumer when the v commit lands.
                }
            } else {
                sb.state.registers.write_addr(val);
                if sb.state.registers.v != v_before {
                    a12_report_v(sb, &mut bus_adapter);
                }
            }
        }
        7 => {
            // v2.1.3 batch 2.1: rendering-time $2007 write: bus write
            // is immediate, v increment is delayed 1 PPU dot. With
            // rendering off, the original immediate path is used so
            // blargg MMC3 test 3 subtest 6 sees the A12 jump on the
            // same CPU instruction.
            let v_before = sb.state.registers.v;
            let mut bus_adapter = make_bus_adapter(sb);
            if rendering_2007(sb) {
                sb.state
                    .registers
                    .write_data_rendering(&mut bus_adapter, val);
                sb.state.vram_write_cooldown = 1;
                mid_frame_write_probe(
                    sb,
                    "$2007",
                    val,
                    Some(sb.state.dot.wrapping_add(1)),
                );
            } else {
                sb.state
                    .registers
                    .write_data(&mut bus_adapter, sb.state.registers.ctrl, val, false);
                if sb.state.registers.v != v_before {
                    a12_report_v(sb, &mut bus_adapter);
                }
            }
        }
        _ => {}
    }
}


/// Phase A (v2.1.1): the `$2007` rendering-time increment condition,
/// ported from the C++ `$2007` handlers' `ppur.increment2007(...)`
/// first argument (`src/ppu.cpp` A2007/B2007): scanline inside the
/// visible+post-render window and PPU rendering enabled (mask bits 3-4,
/// either BG or sprites — the C++ `PPUON` macro is `PPU[1] & 0x18`).
fn rendering_2007(sb: &StateBox) -> bool {
    let sl = sb.state.scanline;
    sl >= 0 && sl < 241 && (sb.state.registers.mask & 0x18) != 0
}

fn make_bus_adapter(sb: &mut StateBox) -> CppBus {
    CppBus {
        cb: sb.bus.unwrap_or(fceux11_ppu_bus_callbacks {
            read: None,
            write: None,
            cpu_read: None,
            notify_a12_rising: None,
            notify_hblank: None,
            notify_hblank2: None,
            notify_scanline: None,
            notify_vblank: None,
            refresh_windows: None,
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
/// v2.1.3 batch 2: the background renderer runs once per dot
/// ([`render_dot`]) instead of once per scanline, so mid-scanline
/// register writes and mapper bank switches are visible to the fetches
/// that follow them.
pub unsafe extern "C" fn fceux11_ppu_emulate_frame(state: *mut PpuState, n_cycles: u32) -> i32 {
    let sb = lookup(state);
    let mut bus = make_bus_adapter(sb);
    // Use the scheduler so notify_* hooks fire on scanline transitions.
    // The scheduler's per-dot advance is what wires the HBlank/scanline
    // hooks to the C++ mapper globals (`GameHBIRQHook` / `GameHBIRQHook2`
    // / `PPU_hook`); the A12 edges come from the fetch pipeline.
    let mut sched = crate::scheduler::NesScheduler::new();
    sched.set_video_system(sb.video_system);
    sched.begin_frame();
    let total_dots = n_cycles.saturating_mul(crate::scheduler::PPU_DOTS_PER_CPU_CYCLE);
    let mut last_outcome = TickOutcome::default();
    while sched.ppu_dots_consumed() < total_dots {
        poll_refresh_windows(sb, &mut bus);
        render_dot(sb, &mut bus);
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

/// v2.1.3 batch 2: advance the per-dot background pipeline for the
/// current `(scanline, dot)`.
///
/// Called once per PPU dot *before* the scheduler's `tick_one_ppu_dot`,
/// so the fetches this dot performs see the `v` / `fine_x` / mapper bank
/// state as of this dot (`frame::tick_dot` owns the scroll updates, and
/// runs after this for the same dot).
///
/// The windows are installed by the C++ bridge at power time
/// (`fceux11_ppu_set_chr_window` / `_nt_window` / `_palette_window`).
/// Without them there is nothing authoritative to render from — the
/// standalone C++ smoke test drives the FFI with a null-callback bus —
/// so the pipeline is skipped and the framebuffer keeps its contents.
fn render_dot(sb: &mut StateBox, bus: &mut CppBus) {
    // Cheapest rejection first: the pipeline does nothing on the
    // post-render and VBlank lines (21 of 262), and this runs once per
    // dot (~89k calls/frame).
    let sl = sb.state.scanline;
    if !(sl == -1 || (0..=239).contains(&sl)) {
        return;
    }
    if sb.nt_window_ptr.is_null()
        || sb.nt_window_len < 4096
        || sb.chr_window_ptr.is_null()
        || sb.chr_window_len < 8192
        || sb.pal_window_ptr.is_null()
        || sb.pal_window_len < 32
    {
        return;
    }
    let mirror = sb.mirror;
    let (nt_ptr, chr_ptr, pal_ptr) = (sb.nt_window_ptr, sb.chr_window_ptr, sb.pal_window_ptr);
    // SAFETY: the pointers are installed by `fceux11_ppu_set_*_window`
    // (power time and savestate load) and stay valid for the lifetime of
    // the mapper; the length checks above guarantee the fixed-size reads.
    let win = crate::rendering::RenderWindows {
        nt: unsafe { &*(nt_ptr as *const [u8; 4096]) },
        chr: unsafe { &*(chr_ptr as *const [u8; 8192]) },
        palette: unsafe { &*(pal_ptr as *const [u8; 32]) },
        mirror,
    };
    crate::rendering::tick_dot(&mut sb.state, bus, &win, &mut sb.framebuffer);
}

/// v2.1.3 batch 1: window-dirty poll — fired once per fetch group
/// (every 8 dots) so a mapper bank switch made mid-scanline re-copies
/// the moved CHR/NT window pages before the next fetch group reads
/// them, instead of waiting for the next scanline boundary.
#[inline]
fn poll_refresh_windows(sb: &StateBox, bus: &mut CppBus) {
    if sb.state.dot & 0x7 == 0 {
        bus.refresh_windows();
    }
}

/// Tick `n_cycles` worth of CPU cycles (i.e. `n_cycles * 3` PPU dots).
/// Phase 3's NesScheduler uses this for per-instruction interleaving.
pub unsafe extern "C" fn fceux11_ppu_tick_cpu_cycle(state: *mut PpuState, n_cycles: u32) -> i32 {
    let sb = lookup(state);
    let mut bus = make_bus_adapter(sb);
    let mut sched = crate::scheduler::NesScheduler::new();
    sched.set_video_system(sb.video_system);
    sched.begin_frame();
    let total_dots = n_cycles.saturating_mul(crate::scheduler::PPU_DOTS_PER_CPU_CYCLE);
    while sched.ppu_dots_consumed() < total_dots {
        poll_refresh_windows(sb, &mut bus);
        render_dot(sb, &mut bus);
        sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
    }
    0
}

/// Phase 5.1: advance the Rust PPU by exactly `n_dots` PPU dots —
/// running the background pipeline, firing mapper event hooks
/// (notify_scanline / notify_hblank / notify_hblank2 /
/// notify_a12_rising / notify_vblank) and pumping the OAM DMA — with
/// the same per-dot ordering as `fceux11_ppu_emulate_frame`.
///
/// The scheduler is PERSISTENT (stored in the `StateBox`): the C++
/// per-cycle loop calls this ~29781 times per frame, and a fresh
/// scheduler per call would re-fire `notify_scanline` on every call.
///
/// Returns 1 if the frame wrapped during this call, 0 otherwise.
pub unsafe extern "C" fn fceux11_ppu_tick_dots(state: *mut PpuState, n_dots: u32) -> i32 {
    let sb = lookup(state);
    let mut bus = make_bus_adapter(sb);
    // Take the persistent scheduler out so `sb` and the scheduler can
    // be borrowed mutably side by side; put it back before returning.
    let mut sched = std::mem::take(&mut sb.sched);
    sched.set_video_system(sb.video_system);
    let mut frame_advanced = false;
    for _ in 0..n_dots {
        poll_refresh_windows(sb, &mut bus);
        // Fetch, shift and emit this dot's pixel BEFORE the dot's
        // state-machine events, so the fetch reads the v/fine_x the
        // previous dot left behind and `frame::tick_dot`'s scroll
        // updates land after the addresses they precede.
        render_dot(sb, &mut bus);
        let outcome = sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
        // Phase 5.1: latch PPU NMI assertions so the C++ side can
        // forward them to the canonical `TriggerNMI()` path.
        if outcome.nmi_asserted {
            sb.state.nmi_pending = true;
        }
        if outcome.frame_advanced {
            // Next dot starts a new frame: reset the per-frame dot
            // counter (keeps the OAM DMA pump aligned to %3) without
            // re-arming the scanline sentinel — the transition into
            // the pre-render line already fired inside the wrap tick.
            sched.reset_frame_counters();
            frame_advanced = true;
        }
        // OAM DMA pump fires once per CPU cycle (= every 3 PPU dots).
        if sb.state.oam_dma_pending
            && sched.ppu_dots_consumed() % crate::scheduler::PPU_DOTS_PER_CPU_CYCLE == 0
        {
            sb.state.tick_oam_dma(&mut bus);
        }
    }
    sb.sched = sched;
    frame_advanced as i32
}

/// Phase 5.3: lock-free variants of the hot per-dot entries. The
/// per-frame interleave loop (root crate, `fceux11_run_frame_interleaved`)
/// calls these ONCE PER DOT in-process; taking the REGISTRY mutex on every
/// dot was the dominant frame-time cost (bench_tolerance_test +378% on the
/// reference machine). They are byte-for-byte the validated
/// [`fceux11_ppu_tick_dots(1)`] / [`fceux11_ppu_take_nmi_pending`] bodies
/// with `lookup` swapped for the registry-check-free `lookup_unchecked` —
/// deliberately NOT fused into a single whole-frame function, because the
/// fused variant changed codegen enough (LTO + `&mut` noalias across the
/// C++ hook re-entrancy) to diverge mapper_mmc1 frame 0.
pub unsafe extern "C" fn fceux11_ppu_tick_dots_direct(state: *mut PpuState, n_dots: u32) -> i32 {
    let sb = lookup_unchecked(state);
    let mut bus = make_bus_adapter(sb);
    let mut sched = std::mem::take(&mut sb.sched);
    sched.set_video_system(sb.video_system);
    let mut frame_advanced = false;
    for _ in 0..n_dots {
        poll_refresh_windows(sb, &mut bus);
        render_dot(sb, &mut bus);
        let outcome = sched.tick_one_ppu_dot(&mut sb.state, &mut bus);
        if outcome.nmi_asserted {
            sb.state.nmi_pending = true;
        }
        if outcome.frame_advanced {
            sched.reset_frame_counters();
            frame_advanced = true;
        }
        if sb.state.oam_dma_pending
            && sched.ppu_dots_consumed() % crate::scheduler::PPU_DOTS_PER_CPU_CYCLE == 0
        {
            sb.state.tick_oam_dma(&mut bus);
        }
    }
    sb.sched = sched;
    frame_advanced as i32
}

/// See [`fceux11_ppu_tick_dots_direct`].
pub unsafe extern "C" fn fceux11_ppu_take_nmi_direct(state: *mut PpuState) -> i32 {
    let sb = lookup_unchecked(state);
    let pending = sb.state.nmi_pending;
    sb.state.nmi_pending = false;
    pending as i32
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

/// Phase 6.3.a: stash the absolute NTSC CPU cycle count the C++
/// bridge is currently on. The C++ side calls this once per CPU
/// cycle (in the per-cycle interleave path of `FCEUPPU_Loop`)
/// before any CPU write to a PPU register that frame can possibly
/// observe. The Rust side then uses this value to stamp the
/// `data_bus_refresh_cycle` timestamp on every write so the open-bus
/// decay check has a reliable baseline. Per-cycle refresh matches
/// the C++ reference's per-write `PPUGenLatch_last_refresh_cycle =
/// now` semantic — the bridge can't drop a cycle since the writes
/// within a cycle all observe the same value.
pub unsafe extern "C" fn fceux11_ppu_set_current_cpu_cycle(
    state: *mut PpuState,
    current_cpu_cycle: u64,
) {
    let sb = lookup(state);
    sb.current_cpu_cycle = current_cpu_cycle;
}

/// Phase 6.3.b (placeholder): notify the Rust scheduler that a DMC
/// DMA fetch has just started and will stall the CPU for
/// `stall_cycles` CPU cycles (1-4 per `X6502_DMR` call; the C++ APU's
/// `DMCDMA` performs 4 such reads per fetch — see
/// `src/sound.cpp:660-686`).
///
/// **Status: scaffolding only.** The C++ APU's DMC arbitration hook
/// is not yet wired (see `docs/history/v2.1_phase6_batch_compat.md`
/// §6.3.b for the full design). When implemented, the per-dot
/// interleave loop will skip `cpu.run(1)` for `stall_cycles` cycles
/// after this notification, advancing only the Rust CPU timestamp
/// (`fceux11_cpu_advance_cycles`) so the C++ APU's DMC timing math
/// (`g_cpu.timestamp_ref` consumed by `X6502_DMR` via `ADDCYC`)
/// stays consistent.
///
/// Until then, this FFI just records the most recent stall request
/// in `StateBox` for instrumentation; the per-dot loop ignores it.
pub unsafe extern "C" fn fceux11_ppu_dmc_dma_arbitration(
    state: *mut PpuState,
    stall_cycles: u8,
) {
    let sb = lookup(state);
    sb.dmc_dma_pending_stall = stall_cycles;
}

/// Phase 6.3.c.1: take-and-clear the pending DMC DMA stall request.
/// Called by the per-dot interleave loop (`fceux11_run_frame_interleaved`)
/// once per dot; if the return value is non-zero, the loop calls
/// `fceux11_cpu_advance_cycles(cpu_state, -returned_value)` to consume
/// the stall from the Rust CPU's `count` budget before the next
/// `fceux11_cpu_run_with_tick(cpu_state, 1)` call. This mirrors the
/// C++ `g_cpu.timestamp_ref()` advance that the DMC's
/// `X6502_DMR → ADDCYC(1)` chain already performs, so the two sides
/// stay in sync.
pub unsafe extern "C" fn fceux11_ppu_take_dmc_dma_stall(state: *mut PpuState) -> u8 {
    let sb = lookup(state);
    let stall = sb.dmc_dma_pending_stall;
    sb.dmc_dma_pending_stall = 0;
    stall
}

/// Phase 6.3.a: refresh the PPU internal data-bus open-bus value and
/// stamp the current CPU cycle. Called by the C++ bridge after every
/// CPU write to a PPU register (and after every PPU-side read that
/// drives the bus, like `bridge_bus_read` for palette/CHR/nametable
/// accesses). The C++ side has the authoritative CPU timestamp.
pub unsafe extern "C" fn fceux11_ppu_refresh_data_bus(
    state: *mut PpuState,
    val: u8,
    current_cpu_cycle: u64,
) {
    let sb = lookup(state);
    sb.state.registers.refresh_data_bus(val, current_cpu_cycle);
}

/// Phase 6.3.a: zero `data_bus` if more than ~600 ms (1 073 864 NTSC
/// CPU cycles) have elapsed since the last refresh. Called once per
/// frame from the C++ bridge (`ppu_rust_bridge_emit_frame` and the
/// per-cycle interleave path) — per-frame granularity is well within
/// the decay threshold tolerance. The decay timestamp lives in
/// `Registers::data_bus_refresh_cycle`; the C++ side passes its
/// current absolute CPU cycle count
/// (`g_cpu.timestamp_base() + g_cpu.timestamp_ref()`).
pub unsafe extern "C" fn fceux11_ppu_check_data_bus_decay(
    state: *mut PpuState,
    current_cpu_cycle: u64,
) {
    let sb = lookup(state);
    sb.state.registers.check_data_bus_decay(current_cpu_cycle);
}

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

// ==============================================================================
// Step B.1 (D1-A): savestate bridge state block.
//
// The byte block is the canonical source-of-truth payload that the C++
// side turns into the chunk-3 / chunk-31 staging variables. It is HOST
// LITTLE-ENDIAN by design (matches the existing "2 | FCEUSTATE_RLSB"
// SFORMAT semantics on MSVC/x86-64; the RLSB swap is a no-op there, see
// src/state.cpp:172-188).
//
// Return value: 0 on success, -1 on bad arguments (null pointer or
// mismatched length). Length must equal STATE_BLOCK_SIZE.
// ==============================================================================

pub const STATE_BLOCK_SIZE: usize = 272;

/// Export the live PPU runtime state into "out".
///
/// Reads registers.{ctrl, mask, status, oam_addr} + the full primary
/// OAM + scroll latches + open-bus buffer + scanline/dot and packs
/// them into a fixed 272-byte host-little-endian blob.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_state_block_export(
    state: *mut PpuState,
    out: *mut u8,
    len: u32,
) -> i32 {
    if state.is_null() || out.is_null() || (len as usize) != STATE_BLOCK_SIZE {
        return -1;
    }
    let sb = lookup(state);
    let buf = unsafe { std::slice::from_raw_parts_mut(out, STATE_BLOCK_SIZE) };

    // 0..4: regs[4]
    buf[0] = sb.state.registers.ctrl;
    buf[1] = sb.state.registers.mask;
    buf[2] = sb.state.registers.status;
    buf[3] = sb.state.registers.oam_addr;

    // 4..260: oam[256]
    buf[4..4 + 0x100].copy_from_slice(&sb.state.oam);

    // 260..264: four single-byte fields
    buf[260] = sb.state.registers.write_toggle as u8;
    buf[261] = sb.state.registers.vram_buffer;
    buf[262] = sb.state.registers.fine_x;
    buf[263] = sb.state.registers.data_bus;

    // 264..272: four LE-encoded integers (v, t, scanline, dot)
    buf[264..266].copy_from_slice(&sb.state.registers.v.to_le_bytes());
    buf[266..268].copy_from_slice(&sb.state.registers.t.to_le_bytes());
    buf[268..270].copy_from_slice(&sb.state.scanline.to_le_bytes());
    buf[270..272].copy_from_slice(&sb.state.dot.to_le_bytes());

    0
}

/// Apply a previously-exported state block back into the live PPU.
///
/// Counterpart of fceux11_ppu_state_block_export. After this returns
/// the C++ side MUST refresh the CHR/NT/palette windows and push the
/// mirror mode (see ppu_bridge_state.cpp for the load contract).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_ppu_state_block_apply(
    state: *mut PpuState,
    buf: *const u8,
    len: u32,
) -> i32 {
    if state.is_null() || buf.is_null() || (len as usize) != STATE_BLOCK_SIZE {
        return -1;
    }
    let sb = lookup(state);
    let src = unsafe { std::slice::from_raw_parts(buf, STATE_BLOCK_SIZE) };

    // 0..4
    sb.state.registers.ctrl = src[0];
    sb.state.registers.mask = src[1];
    sb.state.registers.status = src[2];
    sb.state.registers.oam_addr = src[3];

    // 4..260
    sb.state.oam.copy_from_slice(&src[4..4 + 0x100]);

    // 260..264
    sb.state.registers.write_toggle = src[260] != 0;
    sb.state.registers.vram_buffer = src[261];
    sb.state.registers.fine_x = src[262];
    sb.state.registers.data_bus = src[263];

    // 264..272
    sb.state.registers.v = u16::from_le_bytes([src[264], src[265]]);
    sb.state.registers.t = u16::from_le_bytes([src[266], src[267]]);
    sb.state.scanline = i16::from_le_bytes([src[268], src[269]]);
    sb.state.dot = u16::from_le_bytes([src[270], src[271]]);

    0
}
// ===========================================================================
// Internal helpers ————————?suppress unused warnings.
// ===========================================================================

#[allow(dead_code)]
fn _suppress_unused_for_phase1_symbols() {
    let _ = ctrl_bits::NMI_ENABLE;
    let _ = status_bits::VBL;
}
