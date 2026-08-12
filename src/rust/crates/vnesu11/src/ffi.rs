//! FFI surface for vNESU11 — extern "C" stubs (Phase 0).
//!
//! These functions are exported with `#[unsafe(no_mangle)]` (Rust 2024
//! convention) so C++ can call them once `vnesu11.lib` is linked. Phase 0
//! ships them as no-ops or minimal stubs sufficient for `VNESU11_CORE=ON`
//! link + startup. Real logic arrives in Phase 1-6.
//!
//! See `02_architecture.md` §5 for the full surface spec.

use core::ffi::{c_int, c_void};
use crate::cpu::regs::CpuRegsLayout;
use crate::mapper::{MapperMetaVtable, ReadRangeHandler, WriteRangeHandler};
use crate::ram::RamInitOption;
use crate::soc::{VNesSoc, VNesSocOpaque};

// =========================================================================
// Lifecycle
// =========================================================================

/// Create a new SoC. Phase 0: returns a `Box::into_raw` of a default-constructed
/// `VNesSoc` wrapped in a heap-allocated `VNesSocOpaque`. Phase 1+ will wire
/// RAM init patterns + mapper attach.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_create() -> *mut VNesSocOpaque {
    let soc = Box::new(VNesSoc::default());
    let soc_raw: *mut VNesSoc = Box::into_raw(soc);
    let opaque = Box::new(VNesSocOpaque(soc_raw));
    Box::into_raw(opaque)
}

/// Free the SoC. Phase 0: inverse of `vnesu11_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_destroy(soc: *mut VNesSocOpaque) {
    if soc.is_null() {
        return;
    }
    let opaque = Box::from_raw(soc);
    if !opaque.0.is_null() {
        drop(Box::from_raw(opaque.0));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_power_on(_soc: *mut VNesSocOpaque) {
    // Phase 0: stub. Phase 2 ships `vnesu11_set_ram_init` + a real
    // power-on path that consumes it.
}

/// Phase 2: configure the RAM-init option + seed. Must be called before
/// `vnesu11_power_on`. Equivalent to C++ setting `RAMInitOption` /
/// `RAMInitSeed` in `src/drivers/Qt/ConsoleEmuControl.cpp:475-501`.
///
/// `option`: 0=Checker (default), 1=AllOnes, 2=AllZeros, 3=Random.
/// `seed`: 32-bit seed for `splitmix64` (Random mode only).
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_ram_init(
    soc: *mut VNesSocOpaque,
    option: u32,
    seed: u32,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    // SAFETY: any 0..=3 is valid; out-of-range falls back to Checker.
    soc_ref.ram_init_option = unsafe { RamInitOption::from_raw_unchecked(option) };
    soc_ref.ram_init_seed = seed;
    0
}

/// Phase 2: power on with the previously-set `RamInitOption` + seed.
/// Mirrors `PowerNES` in `src/fceu.cpp:1000-1025`.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_power_on_with_init(soc: *mut VNesSocOpaque) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    let option = soc_ref.ram_init_option;
    let seed = soc_ref.ram_init_seed;
    soc_ref.power_on(option, seed);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_reset(soc: *mut VNesSocOpaque) {
    // Phase 2: minimal — re-init CPU, leave RAM alone.
    if let Some(s) = into_mut(soc) {
        let mut bus = crate::soc::VNesBusContext::new(s);
        s.cpu.reset(&mut bus);
        s.open_bus = 0;
        s.ppu_w = false;
        s.ppu_t = 0;
        s.ppu_v = 0;
        s.ppu_x = 0;
        s.ppu_read_buffer = 0;
    }
}

// =========================================================================
// Mapper handler registration
// =========================================================================

/// Register a per-range read handler. Phase 2+ will use this for actual reads;
/// Phase 0 stores the handler so future reads can resolve correctly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_read_handler(
    soc: *mut VNesSocOpaque,
    start: u16,
    end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16) -> u8,
    ctx: *mut c_void,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if soc_ref.mapper.read_count >= crate::mapper::MAX_RANGES {
        return -2;
    }
    let i = soc_ref.mapper.read_count;
    soc_ref.mapper.read_ranges[i] = ReadRangeHandler { start, end, fn_ptr, ctx };
    soc_ref.mapper.read_count += 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_write_handler(
    soc: *mut VNesSocOpaque,
    start: u16,
    end: u16,
    fn_ptr: unsafe extern "C" fn(*mut c_void, u16, u8),
    ctx: *mut c_void,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if soc_ref.mapper.write_count >= crate::mapper::MAX_RANGES {
        return -2;
    }
    let i = soc_ref.mapper.write_count;
    soc_ref.mapper.write_ranges[i] = WriteRangeHandler { start, end, fn_ptr, ctx };
    soc_ref.mapper.write_count += 1;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_clear_mapper_handlers(soc: *mut VNesSocOpaque) {
    if let Some(s) = into_mut(soc) {
        s.mapper.clear();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_attach_mapper_meta(
    soc: *mut VNesSocOpaque,
    mapper: *mut c_void,
    vtable: *const MapperMetaVtable,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if vtable.is_null() {
        return -2;
    }
    soc_ref.mapper_meta = Some(crate::soc::MapperMetaSlot {
        mapper_ctx: mapper,
        meta: *vtable,
    });
    0
}

// =========================================================================
// System type (Phase 6 — stub for now)
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_system_type(
    soc: *mut VNesSocOpaque,
    system_type: u32,
) -> c_int {
    // C++ `EGIT` encoding (src/git.h): 0=GIT_CART (iNES), 1=GIT_VSUNI
    // (VS), 2=GIT_FDS (FDS), 3=GIT_NSF (NSF). Out-of-range values are
    // rejected (-1) rather than silently treated as iNES.
    if system_type > 3 {
        return -1;
    }
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if system_type == 3 {
        soc_ref.ppu.idle = true;   // NSF — PPU idle stub
    } else {
        soc_ref.ppu.idle = false;  // iNES / FDS / VS — normal PPU
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_external_irq(
    _soc: *mut VNesSocOpaque,
    _source: u32,
    _on: bool,
) {
    // FDS disk IRQ — Phase 0 stub.
}

// =========================================================================
// CHR pages (Phase 6 — mapper adapter Bus::setchr1 forwarding)
// =========================================================================
/// Copy a 1 KiB CHR page from `src` into the SoC's `chr_pages[page_idx]`.
/// Called by the C++ adapter after `Bus::setchr1/4/8` updates the C++
/// PPU's `vpage_[idx]` pointer, so the Rust PPU's reads at
/// `$0000-$1FFF` see the same byte stream.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`. `src`
/// must point to at least 1024 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_chr_set_page(
    soc: *mut VNesSocOpaque,
    page_idx: u8,
    src: *const u8,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if src.is_null() {
        return -2;
    }
    if (page_idx as usize) >= soc_ref.chr_pages.len() {
        return -3;
    }
    core::ptr::copy_nonoverlapping(src, soc_ref.chr_pages[page_idx as usize].as_mut_ptr(), 1024);
    0
}

// =========================================================================
// Shadow-run state sync (Phase 6 §2.5)
// =========================================================================
//
// The C++ emulator is the primary; the Rust core is the shadow. To make
// a frame-level comparison meaningful, the C++ post-frame CPU + WRAM
// state is copied into the Rust SoC before each Rust frame runs (the
// mapper state is already shared via the per-range handler thunks).
// These two FFIs back the C++ `vnesu11_shadow_sync_from_cpp()` helper.

/// Copy the C++ 2 KiB WRAM (`::RAM`, $0000-$07FF mirrored) into the
/// SoC. Returns 0 on success.
///
/// # Safety
/// `src` must point to at least 2048 readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_wram(
    soc: *mut VNesSocOpaque,
    src: *const u8,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if src.is_null() {
        return -2;
    }
    core::ptr::copy_nonoverlapping(src, soc_ref.wram.as_mut_ptr(), 2048);
    // Keep the `ram_banks` snapshot in sync (savestate/snapshot path).
    soc_ref.ram_banks.wram.copy_from_slice(&soc_ref.wram);
    0
}

// =========================================================================
// APU state sync (Phase 6 P2 shadow-run)
//
// The shadow runner pushes the C++ side's APU state into Rust after each
// C++ frame so the per-cycle APU model (frame counter, master cycle
// counter, channel enables) starts from the same baseline Rust's CPU
// sees when it executes its own frame. Without this sync, the frame
// counter IRQ lands in a different instruction window in Rust vs C++
// (e.g. blargg cpu_dummy_reads diverges after frame 2 — Rust enters
// the IRQ handler at $E622 while C++ stays in the $2002 VBlank-wait
// loop). Channel-level sync (timer / length / envelope / sweep / DMC)
// is deferred to the broader state-sync pass (see phase_6_integration.md
// §9) — `vnesu11_apu_state_t` covers the timing-critical fields only.
// =========================================================================

/// Mirror of `ApuCore` state pushed from C++ to Rust each frame.
/// Layout MUST match the C++ `ApuStateMirror` in
/// `src/vnesu11_shadow.h`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ApuStateMirror {
    /// Master cycle counter (one tick per CPU cycle since power-on).
    pub cycles: u64,
    /// Frame counter period position (NTSC 29830, PAL 33254).
    pub fc_cycle_count: u64,
    /// Step counter (0..=3 for 4-step, 0..=4 for 5-step).
    pub fc_step: u8,
    /// 5-step mode flag ($4017 bit 7).
    pub fc_five_step: bool,
    /// IRQ inhibit flag ($4017 bit 6).
    pub fc_irq_inhibit: bool,
    /// PAL timing (mirrors C++ global `PAL` in FrameCounterTick).
    pub pal: bool,
    /// Pending $4017 mode bits (0x80 | 0x40) waiting to mature.
    pub fc_pending_mode: u8,
    /// Cycles until `fc_pending_mode` commits (3 or 4 after parity).
    pub fc_reset_in: u8,
    /// Quarter-frame flag latched from last tick.
    pub fc_quarter_frame: bool,
    /// Half-frame flag latched from last tick.
    pub fc_half_frame: bool,
    /// Frame IRQ pending (latched; consumed by IRQ check).
    pub frame_irq_pending: bool,
    /// DMC IRQ pending.
    pub dmc_irq_pending: bool,
    /// Bitmask of channels currently enabled ($4015).
    pub enabled_channels: u8,
}

/// Push the C++ side's APU state into Rust. Called by the shadow
/// runner after every C++ frame so Rust's per-cycle APU model
/// (frame counter + master cycle counter + channel enables) starts
/// the next frame from the same baseline.
///
/// Returns 0 on success, -1 on null SoC, -2 on null state pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_apu_poke_state(
    soc: *mut VNesSocOpaque,
    state: *const ApuStateMirror,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if state.is_null() {
        return -2;
    }
    let s = &*state;
    let apu = &mut soc_ref.apu;
    // Master cycle counter — drives $4017 parity and APU sub-cycle
    // bookkeeping. Resetting it to C++'s value makes the parity match.
    apu.cycles = s.cycles;
    // Frame counter — drives quarter/half/IRQ events. Resetting
    // cycle_count to C++'s value aligns Rust's IRQ firing with C++'s.
    apu.frame_counter.cycle_count = s.fc_cycle_count;
    apu.frame_counter.step = s.fc_step;
    apu.frame_counter.five_step = s.fc_five_step;
    apu.frame_counter.irq_inhibit = s.fc_irq_inhibit;
    apu.frame_counter.pal = s.pal;
    apu.frame_counter.pending_mode = s.fc_pending_mode;
    apu.frame_counter.reset_in = s.fc_reset_in;
    apu.frame_counter.quarter_frame = s.fc_quarter_frame;
    apu.frame_counter.half_frame = s.fc_half_frame;
    // IRQ pending flags — consumed by `apu.take_irq()` on next
    // instruction; syncing avoids Rust firing an IRQ C++ already
    // cleared or missing one C++ just fired.
    apu.frame_irq_pending = s.frame_irq_pending;
    apu.dmc_irq_pending = s.dmc_irq_pending;
    // Channel-enable mask ($4015) — channels gated off shouldn't
    // advance their timers / envelopes. Without this sync, Rust
    // ticks channels that C++ has already disabled.
    // (Note: the per-channel enable bits in pulse1/pulse2/etc. are
    // derived from `enabled_channels` at $4015-write time, so we
    // re-derive them here for consistency.)
    let _ = s.enabled_channels; // Phase 6 P2 follow-up: per-channel sync
    // If the C++ side's frame/DMC IRQ line is asserted at the frame
    // boundary, make sure Rust's CPU sees it on the next frame —
    // `poke_regs` synced `irq_low` from C++ X6502, but the APU slice
    // below is authoritative for the pending flags. Re-assert through
    // the IrqController so the next instruction's poll_interrupts
    // services it at the same time C++'s does.
    if apu.frame_irq_pending {
        soc_ref.irq.assert_irq(crate::apu::IRQ_FCOUNT);
    }
    if apu.dmc_irq_pending {
        soc_ref.irq.assert_irq(crate::apu::IRQ_DMC);
    }
    let agg = soc_ref.irq.aggregate_mask();
    if agg != 0 {
        soc_ref.cpu.irq_begin(agg);
    }
    0
}

/// Snapshot the Rust side's APU state into the mirror. Provided for
/// round-trip tests / savestate parity work. Returns 0 on success,
/// -1 on null SoC, -2 on null state pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_apu_peek_state(
    soc: *const VNesSocOpaque,
    out_state: *mut ApuStateMirror,
) -> c_int {
    let soc_ref = match into_const(soc) {
        Some(s) => s,
        None => return -1,
    };
    if out_state.is_null() {
        return -2;
    }
    let out = &mut *out_state;
    let apu = &soc_ref.apu;
    out.cycles = apu.cycles;
    out.fc_cycle_count = apu.frame_counter.cycle_count;
    out.fc_step = apu.frame_counter.step;
    out.fc_five_step = apu.frame_counter.five_step;
    out.fc_irq_inhibit = apu.frame_counter.irq_inhibit;
    out.pal = apu.frame_counter.pal;
    out.fc_pending_mode = apu.frame_counter.pending_mode;
    out.fc_reset_in = apu.frame_counter.reset_in;
    out.fc_quarter_frame = apu.frame_counter.quarter_frame;
    out.fc_half_frame = apu.frame_counter.half_frame;
    out.frame_irq_pending = apu.frame_irq_pending;
    out.dmc_irq_pending = apu.dmc_irq_pending;
    out.enabled_channels = 0;
    0
}

// =========================================================================
// PPU state sync (Phase 6 P2 shadow-run)
//
// The shadow runner pushes the C++ side's PPU state into Rust after
// each C++ frame so the two cores observe identical $2002 / $2007 /
// $2004 reads on the next frame. Without this, a ROM that branches on
// the PPU status register (e.g. blargg cpu_dummy_reads' `BIT $2002`
// VBlank-wait loop at $E48D) diverges after a few frames: the C++ and
// Rust PPUs start each frame from different internal state, so the CPU
// sees different values and takes different branches.
//
// Scope: registers + memory the CPU can read (status, read buffer,
// palette, VRAM, OAM). Internal render latches (PPUREGS daisy chain,
// line buffer, bg/sprite shifters) are NOT synced — they only affect
// the rendered frame, not CPU-observable state, and the CPU instruction
// stream stays identical once reads match (the v/t/x/w write sequence
// replays identically from the synced start).
//
// Layout MUST match the C++ `PpuStateMirror` in src/vnesu11_shadow.h.
// =========================================================================

/// Mirror of the PPU state the CPU can observe. Pushed from C++ to
/// Rust after each C++ frame.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PpuStateMirror {
    /// $2000 PPUCTRL (C++ PPU[0]).
    pub ppuctrl: u8,
    /// $2001 PPUMASK (C++ PPU[1]).
    pub ppumask: u8,
    /// $2002 PPUSTATUS (C++ PPU[2]) — VBlank / sprite0 / overflow.
    pub status: u8,
    /// $2003 OAMADDR (C++ PPU[3]).
    pub oam_addr: u8,
    /// $2007 read buffer (C++ PPUGenLatch).
    pub read_buffer: u8,
    /// CPU open-bus value (C++ `_DB` after last read).
    pub open_bus: u8,
    /// Palette RAM (C++ PALRAM[0x20]).
    pub palette: [u8; 32],
    /// Name-table RAM (C++ NTARAM[0x800], 4 nametables).
    pub vram: [u8; 2048],
    /// OAM (C++ g_ppu.oam()[256]).
    pub oam: [u8; 256],
}

/// Push the C++ side's PPU state into Rust. Called by the shadow
/// runner after every C++ frame. Returns 0 on success, -1 on null
/// SoC, -2 on null state pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_ppu_poke_state(
    soc: *mut VNesSocOpaque,
    state: *const PpuStateMirror,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if state.is_null() {
        return -2;
    }
    let s = &*state;
    // Registers (CPU-observable).
    soc_ref.ppu.regs.ppuctrl = s.ppuctrl;
    soc_ref.ppu.regs.ppumask = s.ppumask;
    soc_ref.ppu.regs.status = s.status;
    soc_ref.ppu.regs.oam_addr = s.oam_addr;
    soc_ref.ppu.regs.read_buffer = s.read_buffer;
    // Keep the SoC's public mirrors in sync (savestate/snapshot path).
    soc_ref.palette.copy_from_slice(&s.palette);
    soc_ref.vram.copy_from_slice(&s.vram);
    soc_ref.oam.copy_from_slice(&s.oam);
    soc_ref.ram_banks.palette.copy_from_slice(&s.palette);
    soc_ref.ram_banks.vram.copy_from_slice(&s.vram);
    soc_ref.ram_banks.oam.copy_from_slice(&s.oam);
    // Open bus value the CPU reads for unmapped/PPU reads.
    soc_ref.open_bus = s.open_bus;
    0
}

/// Snapshot the Rust side's PPU state into the mirror. Provided for
/// round-trip tests / savestate parity work. Returns 0 on success,
/// -1 on null SoC, -2 on null state pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_ppu_peek_state(
    soc: *const VNesSocOpaque,
    out_state: *mut PpuStateMirror,
) -> c_int {
    let soc_ref = match into_const(soc) {
        Some(s) => s,
        None => return -1,
    };
    if out_state.is_null() {
        return -2;
    }
    let out = &mut *out_state;
    out.ppuctrl = soc_ref.ppu.regs.ppuctrl;
    out.ppumask = soc_ref.ppu.regs.ppumask;
    out.status = soc_ref.ppu.regs.status;
    out.oam_addr = soc_ref.ppu.regs.oam_addr;
    out.read_buffer = soc_ref.ppu.regs.read_buffer;
    out.open_bus = soc_ref.open_bus;
    out.palette.copy_from_slice(&soc_ref.palette);
    out.vram.copy_from_slice(&soc_ref.vram);
    out.oam.copy_from_slice(&soc_ref.oam);
    0
}

// =========================================================================
// Emulation
// =========================================================================
//
// Phase 6: real implementation. Always runs a full emulation frame; the
// shadow runner passes skip=0 to keep both Rust and C++ outputs aligned.
// Per-CPU-cycle APU sampling produces ~29,780 stereo samples per frame;
// the shadow run decodes the buffer CRC (Phase 6 §2.5 — frame-level
// 3-tier diff), not sample-rate-corrected SNR (Phase 7 territory).

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_emulate_frame(
    soc: *mut VNesSocOpaque,
    _skip: c_int,
    xbuf: *mut u8,
    sbuf: *mut i16,
    sbuf_cap: usize,
    sbuf_written: *mut usize,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if xbuf.is_null() {
        return -2;
    }

    // 1. Drive one full frame (CPU + APU + PPU + DMA + IRQ routing).
    let result = soc_ref.run_frame();

    // 2. Copy the rendered frame buffer (61440 bytes = 256 × 240).
    //    Rust writes palette indices; the C++ side downstream converts
    //    them to NES colors (Phase 6: shadow run CRC operates on the
    //    raw palette-index bytes for a deterministic comparison).
    core::ptr::copy_nonoverlapping(
        soc_ref.frame_buffer.as_ptr(),
        xbuf,
        61440,
    );

    // 3. Drain the APU output buffer into the caller's sound slot.
    //    The Rust mixer ticks once per CPU cycle (Phase 5 stage 0
    //    simplification); Phase 6 shadow-run compares buffer CRCs,
    //    not sample-rate-corrected SNR (which lands in Phase 7).
    if !sbuf_written.is_null() {
        *sbuf_written = 0;
    }
    if !sbuf.is_null() && sbuf_cap > 0 {
        let samples = soc_ref.apu.drain_output();
        let n = samples.len().min(sbuf_cap);
        if n > 0 {
            core::ptr::copy_nonoverlapping(samples.as_ptr(), sbuf, n);
        }
        if !sbuf_written.is_null() {
            *sbuf_written = n;
        }
    }

    if result.completed {
        0
    } else {
        1
    }
}

// =========================================================================
// Debugger / Lua / savestate peek/poke
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_peek(
    soc: *const VNesSocOpaque,
    addr: u16,
) -> u8 {
    let Some(s) = into_const(soc) else { return 0 };
    // `cpu_read` updates open_bus + advances the joypad shift register
    // on $4016/$4017 (real NES behavior). The FFI guarantees
    // single-threaded access, so the const handle is re-borrowed
    // mutably via raw pointers here (matches the `into_mut` pattern
    // used by the rest of the FFI surface).
    let raw = core::ptr::addr_of!(*s) as *mut VNesSoc;
    unsafe { (&mut *raw).cpu_read(addr) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_poke(
    soc: *mut VNesSocOpaque,
    addr: u16,
    val: u8,
) {
    let Some(s) = into_mut(soc) else { return };
    s.cpu_write(addr, val);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_peek_regs(
    soc: *const VNesSocOpaque,
    out: *mut CpuRegsLayout,
) {
    let (Some(s), false) = (into_const(soc), out.is_null()) else {
        if !out.is_null() {
            *out = CpuRegsLayout::default();
        }
        return;
    };
    let cpu = &s.cpu;
    *out = CpuRegsLayout {
        tcount: cpu.tcount,
        PC: cpu.pc(),
        A: cpu.a(),
        X: cpu.x(),
        Y: cpu.y(),
        S: cpu.s(),
        P: cpu.p(),
        moo_pi: cpu.moo_pi,
        jammed: u8::from(cpu.jammed()),
        count: cpu.count,
        irq_low: cpu.irq_pending,
        db: cpu.db,
        preexec: 0,
        cpu_hook: core::ptr::null_mut(),
        read_hook: core::ptr::null_mut(),
        write_hook: core::ptr::null_mut(),
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_cpu_poke_regs(
    soc: *mut VNesSocOpaque,
    regs: *const CpuRegsLayout,
) {
    if regs.is_null() {
        return;
    }
    let Some(s) = into_mut(soc) else { return };
    let r = &*regs;
    let cpu = &mut s.cpu;
    cpu.tcount = r.tcount;
    cpu.set_pc(r.PC);
    cpu.set_a(r.A);
    cpu.set_x(r.X);
    cpu.set_y(r.Y);
    cpu.set_s(r.S);
    cpu.set_p(r.P);
    cpu.moo_pi = r.moo_pi;
    cpu.jammed = r.jammed != 0;
    // Phase 6 P2 shadow fix (2026-08-12): C++ `X6502.count` is in the
    // ×16 internal unit (X6502_Run credits cycles*16, add_cycles debits
    // c*48), while Rust's count is in dots (budget is 256/85/341 dots,
    // each instruction debits tcount*3). The conversion is /16. Syncing
    // the raw value made the Rust budget residual differ by ~60 dots
    // (~20 cycles) per frame, which drove the frame 0-1 instruction
    // count delta (2-16) and the frame-counter phase drift (~7 cycles).
    cpu.count = r.count / 16;
    cpu.irq_pending = r.irq_low;
    cpu.db = r.db;
}

/// Shadow diagnostics: executed instruction counter (Phase 6 P2).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_instr_count(soc: *const VNesSocOpaque) -> u64 {
    let Some(s) = into_const(soc) else { return 0 };
    s.instr_count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_ppu_peek(
    soc: *const VNesSocOpaque,
    addr: u16,
) -> u8 {
    let Some(s) = into_const(soc) else { return 0 };
    // PPU-side read of CHR / nametable / palette (`ppu_read` is
    // `&self` — no side effects).
    s.ppu_read(addr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_joypad_set_button(
    soc: *mut VNesSocOpaque,
    pad: u8,
    btn: u32,
    pressed: bool,
) {
    if let Some(s) = into_mut(soc) {
        s.joypad.set_button(pad as usize, btn as u8, pressed);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_joypad_set_strobe(
    soc: *mut VNesSocOpaque,
    strobe: bool,
) {
    if let Some(s) = into_mut(soc) {
        let val = if strobe { 1u8 } else { 0u8 };
        s.joypad.write_strobe(val);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_set_lua_mem_hook(_active: bool) {}

// =========================================================================
// Savestate (Phase 0 stub — real impl in Phase 0 + Phase 6 per
// docs/wip_2.0_plan/savestate_tags.md)
// =========================================================================

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_save_cpu_state(
    _soc: *const VNesSocOpaque,
    _sink: *mut c_void,
    _write_fn: extern "C" fn(*mut c_void, *const u8, usize),
) -> c_int {
    -1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_load_cpu_state(
    _soc: *mut VNesSocOpaque,
    _source: *mut c_void,
    _read_fn: extern "C" fn(*mut c_void, *mut u8, usize) -> usize,
) -> c_int {
    -1
}

// =========================================================================
// Phase 2: save/load the four private RAM banks (WRAM/VRAM/OAM/Palette)
// =========================================================================
//
// The C++ side calls these through the FFI to integrate vNESU11 into the
// `FCEUSS_SaveMS` / `FCEUSS_LoadMS` flow. The V2-chunked byte stream
// matches the layout produced by `state.cpp`'s `SFCPU`/`FCEU_NEWPPU_STATEINFO`
// groups, so a vNESU11 savestate round-trips byte-for-byte with the C++
// reader.

/// Save the four RAM banks into a heap buffer. Caller takes ownership
/// of the returned `*mut u8` (free with `vnesu11_free_buffer`).
///
/// `out_len`: optional pointer to receive the byte count; pass null if
/// not needed.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`. The
/// returned `*mut u8` must be freed with `vnesu11_free_buffer`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_save_ram_state(
    soc: *const VNesSocOpaque,
    out_len: *mut usize,
) -> *mut u8 {
    let soc_ref = match into_const(soc) {
        Some(s) => s,
        None => return core::ptr::null_mut(),
    };
    let mut w = crate::snapshot::mem::Writer::with_capacity(8192);
    soc_ref.ram_banks.save_state(&mut w);
    let bytes = w.into_bytes();
    let n = bytes.len();
    let mut boxed = bytes.into_boxed_slice();
    let ptr = boxed.as_mut_ptr();
    core::mem::forget(boxed);
    if !out_len.is_null() {
        *out_len = n;
    }
    ptr
}

/// Load the four RAM banks from a V2 byte stream. Returns 0 on success,
/// negative on error.
///
/// # Safety
/// `soc` must be a valid pointer returned by `vnesu11_create`. `bytes`
/// must point to a buffer of at least `len` readable bytes (typically
/// a buffer previously returned by `vnesu11_save_ram_state`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_load_ram_state(
    soc: *mut VNesSocOpaque,
    bytes: *const u8,
    len: usize,
) -> c_int {
    let soc_ref = match into_mut(soc) {
        Some(s) => s,
        None => return -1,
    };
    if bytes.is_null() || len == 0 {
        return -2;
    }
    let slice = core::slice::from_raw_parts(bytes, len);
    let mut r = crate::snapshot::mem::Reader::new(slice);
    match soc_ref.ram_banks.load_state(&mut r) {
        Ok(()) => {
            soc_ref.sync_ram_banks_to_views();
            0
        }
        Err(_) => -3,
    }
}

/// Free a buffer returned by `vnesu11_save_ram_state`.
///
/// # Safety
/// `ptr` must either be null or a pointer returned by
/// `vnesu11_save_ram_state`; `len` must be the value written to
/// `out_len` at allocation time.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vnesu11_free_buffer(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Box::from_raw(core::ptr::slice_from_raw_parts_mut(ptr, len));
    }
}

// =========================================================================
// Helpers
// =========================================================================

/// Returns `Some(&mut soc)` if the pointer is valid (non-null + non-null inner).
unsafe fn into_mut(soc: *mut VNesSocOpaque) -> Option<&'static mut VNesSoc> {
    if soc.is_null() {
        return None;
    }
    let inner = (*soc).0;
    if inner.is_null() {
        return None;
    }
    Some(&mut *inner)
}

unsafe fn into_const(soc: *const VNesSocOpaque) -> Option<&'static VNesSoc> {
    if soc.is_null() {
        return None;
    }
    let inner = (*soc).0;
    if inner.is_null() {
        return None;
    }
    Some(&*inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_destroy_round_trip() {
        let soc = unsafe { vnesu11_create() };
        assert!(!soc.is_null());
        unsafe { vnesu11_destroy(soc); }
    }

    #[test]
    fn null_safety() {
        unsafe { vnesu11_destroy(core::ptr::null_mut()); }
        let r = unsafe { vnesu11_set_read_handler(
            core::ptr::null_mut(), 0, 0, dummy_read, core::ptr::null_mut()
        ) };
        assert_eq!(r, -1);
    }

    #[test]
    fn set_system_type_toggles_ppu_idle() {
        // C++ EGIT encoding: 0=GIT_CART, 1=GIT_VSUNI, 2=GIT_FDS,
        // 3=GIT_NSF. NSF drives the PPU through the idle stub.
        let soc = unsafe { vnesu11_create() };
        assert!(!soc.is_null());
        let rc = unsafe { vnesu11_set_system_type(soc, 3) };
        assert_eq!(rc, 0);
        assert!(unsafe { into_mut(soc) }.unwrap().ppu.idle, "NSF → PPU idle");
        let rc = unsafe { vnesu11_set_system_type(soc, 0) };
        assert_eq!(rc, 0);
        assert!(!unsafe { into_mut(soc) }.unwrap().ppu.idle, "iNES → normal PPU");
        // Null SoC → -1.
        let rc = unsafe { vnesu11_set_system_type(core::ptr::null_mut(), 3) };
        assert_eq!(rc, -1);
        unsafe { vnesu11_destroy(soc); }
    }

    /// Phase 6: `vnesu11_emulate_frame` runs one frame, copies the
    /// 256×240 frame buffer into `xbuf`, and drains APU samples into
    /// `sbuf`. Returns 0 on success.
    #[test]
    fn emulate_frame_runs_and_copies() {
        let soc = unsafe { vnesu11_create() };
        assert!(!soc.is_null());
        unsafe { vnesu11_power_on_with_init(soc) };
        let mut xbuf = [0u8; 61440];
        let mut sbuf = vec![0i16; 16384];
        let mut sbuf_written: usize = 0;
        let rc = unsafe {
            vnesu11_emulate_frame(
                soc,
                0,
                xbuf.as_mut_ptr(),
                sbuf.as_mut_ptr(),
                sbuf.len(),
                &mut sbuf_written as *mut usize,
            )
        };
        assert_eq!(rc, 0, "frame must complete");
        // The SoC frame buffer should now equal xbuf.
        let s = unsafe { into_const(soc) }.unwrap();
        assert_eq!(&xbuf[..], &s.frame_buffer[..]);
        // With no APU registers written the APU output is silent, so
        // the drained buffer is all-zero.
        assert!(sbuf_written <= sbuf.len());
        let written = sbuf_written;
        assert!(written > 0, "APU must tick during a frame");
        let all_zero = sbuf[..written].iter().all(|&v| v == 0);
        assert!(all_zero, "silent APU must output zero samples");
        unsafe { vnesu11_destroy(soc); }
    }

    /// Phase 6: writing a pulse channel register → APU produces
    /// non-zero samples in the drained buffer.
    #[test]
    fn emulate_frame_drains_nonzero_apu_output() {
        let soc = unsafe { vnesu11_create() };
        assert!(!soc.is_null());
        unsafe { vnesu11_power_on_with_init(soc) };
        unsafe { vnesu11_set_ram_init(soc, 0, 0) };
        // Enable channel 0 + program duty 0, vol 15, timer lo=0x10.
        let s = unsafe { into_mut(soc) }.unwrap();
        s.cpu_write(0x4015, 0x01);
        s.cpu_write(0x4000, 0x3F);
        s.cpu_write(0x4002, 0x10);
        s.cpu_write(0x4003, 0x08);
        // (no drop needed — `s` borrows from `soc` and isn't an owned
        // value here; we just exit the block scope.)
        let mut xbuf = [0u8; 61440];
        let mut sbuf = vec![0i16; 16384];
        let mut sbuf_written: usize = 0;
        unsafe {
            vnesu11_emulate_frame(
                soc,
                0,
                xbuf.as_mut_ptr(),
                sbuf.as_mut_ptr(),
                sbuf.len(),
                &mut sbuf_written as *mut usize,
            );
        }
        assert!(sbuf_written > 0);
        let any_nonzero = sbuf[..sbuf_written].iter().any(|&v| v != 0);
        assert!(any_nonzero, "pulse 1 must produce non-zero samples");
        unsafe { vnesu11_destroy(soc); }
    }

    /// Phase 6: null SoC returns -1.
    #[test]
    fn emulate_frame_null_soc_returns_minus_one() {
        let mut xbuf = [0u8; 61440];
        let mut sbuf = [0i16; 16];
        let mut sbuf_written: usize = 0;
        let rc = unsafe {
            vnesu11_emulate_frame(
                core::ptr::null_mut(),
                0,
                xbuf.as_mut_ptr(),
                sbuf.as_mut_ptr(),
                sbuf.len(),
                &mut sbuf_written as *mut usize,
            )
        };
        assert_eq!(rc, -1);
    }

    /// Phase 6: null xbuf returns -2 (must check xbuf up front to
    /// avoid copying to null).
    #[test]
    fn emulate_frame_null_xbuf_returns_minus_two() {
        let soc = unsafe { vnesu11_create() };
        assert!(!soc.is_null());
        let mut sbuf = [0i16; 16];
        let mut sbuf_written: usize = 0;
        let rc = unsafe {
            vnesu11_emulate_frame(
                soc,
                0,
                core::ptr::null_mut(),
                sbuf.as_mut_ptr(),
                sbuf.len(),
                &mut sbuf_written as *mut usize,
            )
        };
        assert_eq!(rc, -2);
        unsafe { vnesu11_destroy(soc); }
    }

    unsafe extern "C" fn dummy_read(_ctx: *mut core::ffi::c_void, _addr: u16) -> u8 { 0 }
}
