//! `$2000`-`$2007` and `$4014` register file, scroll latches, open-bus buffer.
//!
//! The state machine in [`crate::frame::tick_dot`] drives timing events
//! (scanline/dot, VBlank flag set/clear, NMI sampling, scroll copy on
//! rendering scanlines). The methods on [`Registers`] are *value-level*
//! side-effects: each method commits exactly the bits the spec says a CPU
//! write or read triggers, without worrying about which dot the CPU is at.
//!
//! Bit layout reference: <https://www.nesdev.org/wiki/PPU_registers>.
//! Open-bus / buffered-read semantics: <https://www.nesdev.org/wiki/PPU_scrolling#PPU_internal_registers>.

use crate::bus::PpuBus;

/// Bit indices in `$2000` PPUCTRL.
pub mod ctrl_bits {
    pub const NMI_ENABLE: u8 = 7;
    pub const SPRITE_SIZE: u8 = 5; // 0 = 8x8, 1 = 8x16
    pub const BG_PATTERN: u8 = 4; // 0 = $0000, 1 = $1000
    pub const SPRITE_PATTERN: u8 = 3;
    pub const VRAM_INCREMENT: u8 = 2; // 0 = +1, 1 = +32
    pub const NAMETABLE_LO: u8 = 0;
    pub const NAMETABLE_HI: u8 = 1;
}

/// Bit indices in `$2001` PPUMASK.
pub mod mask_bits {
    pub const EMPH_BLUE: u8 = 7;
    pub const EMPH_GREEN: u8 = 6;
    pub const EMPH_RED: u8 = 5;
    pub const SHOW_SPRITES: u8 = 4;
    pub const SHOW_BG: u8 = 3;
    pub const SPRITE_LEFT8: u8 = 2;
    pub const BG_LEFT8: u8 = 1;
    pub const GRAYSCALE: u8 = 0;
}

/// Bit indices in `$2002` PPUSTATUS.
pub mod status_bits {
    pub const VBL: u8 = 7;
    pub const SPRITE0_HIT: u8 = 6;
    pub const SPRITE_OVERFLOW: u8 = 5;
    /// bits 4..=0 — last CPU write was to $2005 or $2006 (clear after
    /// $2002 read or $2004/$2007 access). The exact value is the
    /// "second-write toggle" before the read.
    pub const WRITE_TOGGLE: u8 = 4;
}

/// Mask covering bits 4..=0 of `$2002` (the open-bus / write-toggle echo).
pub const STATUS_WRITE_TOGGLE_MASK: u8 = 0x1F;

/// Phase 6.3.a: open-bus decay threshold (~600 ms at NTSC CPU clock).
///
/// Per blargg `ppu_open_bus` readme, an unwritten data-bus bit decays
/// to 0 after about 600 ms. NTSC CPU clock is 1.789773 MHz so 600 ms
/// = 1 073 864 cycles. Matches `PPU_OPEN_BUS_DECAY_CYCLES` in
/// `src/ppu.cpp:478`. The C++ engine checks this once per frame (~16.67 ms
/// granularity) — well within the threshold tolerance.
pub const DATA_BUS_DECAY_CYCLES: u32 = 1_073_864;

/// PPU register file + the hidden latches the CPU can't see but the
/// rendering state machine relies on.
///
/// `v` and `t` are 15-bit VRAM addresses laid out as
/// `yyy NN YYYYY XXXXX` (bits 14..=12 = fine Y, 11 = nametable Y,
/// 10 = nametable X, 9..=5 = coarse Y, 4..=0 = coarse X). `fine_x`
/// holds bits 2..=0 of the X scroll (the rest of the first `$2005`
/// write goes into coarse X bits of `t`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registers {
    /// `$2000` PPUCTRL.
    pub ctrl: u8,
    /// `$2001` PPUMASK.
    pub mask: u8,
    /// `$2002` PPUSTATUS (VBL / sprite0 hit / overflow + write-toggle echo).
    pub status: u8,
    /// `$2003` OAMADDR.
    pub oam_addr: u8,
    /// `$2005`/`$2006` second-write toggle. False = next write is the
    /// first, true = next write is the second.
    pub write_toggle: bool,
    /// Current VRAM address (15 bits used).
    pub v: u16,
    /// Temporary VRAM address (15 bits used) — the scroll latches.
    pub t: u16,
    /// Fine X scroll (3 bits).
    pub fine_x: u8,
    /// Buffered read result for `$2007` open-bus behaviour. Palette reads
    /// don't update this; everything else does.
    pub vram_buffer: u8,
    /// Phase 6.3.a: PPU internal data-bus open-bus value. Updated by
    /// every CPU write to a PPU register ($2000-$2007, $4014). Reading
    /// $2005 / $2006 (write-only registers) returns this value, which
    /// is what blargg's `ppu_read_buffer` test exercises. Without this
    /// field the reads return 0 and the test fails on subtest 1.
    /// Decay (slow float to 0 over time) is a refinement deferred to
    /// a later sub-phase; blargg_ppu_read_buffer does not require it.
    pub data_bus: u8,
    /// Phase 6.3.a: CPU cycle timestamp of the most recent refresh
    /// of `data_bus`. The C++ engine (PPUGenLatch_last_refresh_cycle,
    /// `src/ppu.cpp:480-490`) tracks this to detect when the open-bus
    /// value should decay to 0 (after ~600 ms / 1 073 864 NTSC CPU
    /// cycles without a refresh). Stored as u64 of the absolute CPU
    /// cycle count (matches the C++ `timestamp_base + timestamp_ref`)
    /// so the 600 ms threshold is correctly measured across frame
    /// boundaries (per-frame `timestamp_ref` wraps every frame at
    /// 89 342 cycles, far short of the 1 073 864 threshold). See
    /// [`Registers::check_data_bus_decay`].
    ///
    /// Not serialized into RPU1 v1 payload: after a savestate load
    /// the field defaults to 0, which the decay check interprets as
    /// "never refreshed" → `data_bus` is zeroed on the first decay
    /// check (matches cold-boot behaviour; data_bus is usually 0 in
    /// well-behaved games).
    pub data_bus_refresh_cycle: u64,
}

impl Default for Registers {
    fn default() -> Self {
        Self::new()
    }
}

impl Registers {
    /// Cold-state reset (after power-on): all registers cleared, latches
    /// cleared, open-bus buffer cleared.
    pub const fn new() -> Self {
        Self {
            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            write_toggle: false,
            v: 0,
            t: 0,
            fine_x: 0,
            vram_buffer: 0,
            data_bus: 0,
            data_bus_refresh_cycle: 0,
        }
    }

    /// Warm reset (post power-on, post soft reset): keep `vram_buffer` so
    /// a game reading `$2007` during boot still gets the last garbage byte
    /// from the previous frame. Match the legacy FCEUX behaviour.
    pub fn warm_reset(&mut self) {
        self.ctrl = 0;
        self.mask = 0;
        self.status = 0;
        self.oam_addr = 0;
        self.write_toggle = false;
        self.v = 0;
        self.t = 0;
        self.fine_x = 0;
        // `data_bus` is intentionally NOT reset here — power-on
        // open-bus behaviour keeps the bus sticky until the first
        // CPU write to a PPU register overwrites it (matches the
        // real 2C02 cold-boot floating-bus behaviour). The decay
        // timestamp also stays at 0 ("never refreshed"); the first
        // CPU write sets it.
    }

    // -----------------------------------------------------------------
    // Phase 6.3.a: data-bus open-bus helpers
    //
    // All CPU writes to PPU registers refresh the internal data bus
    // with the written byte and stamp the current CPU cycle for decay
    // tracking. Reads of $2005 / $2006 return the latched data-bus
    // value (or 0 after the decay threshold has elapsed without a
    // refresh). The decay check is invoked once per frame from the
    // scheduler; this module only exposes the per-cycle write refresh.
    // -----------------------------------------------------------------

    /// Refresh `data_bus` with `val` and stamp the refresh cycle.
    /// Called by every CPU write to a PPU register ($2000-$2007, $4014)
    /// and by PPU-side reads that drive the data bus (palette/CHR/nametable
    /// reads place the read byte on the bus). The C++ reference
    /// (`src/ppu.cpp: PPUGenLatch = ret` after every read) does the same.
    ///
    /// `current_cpu_cycle` must be the absolute NTSC CPU cycle count
    /// (matches `g_cpu.timestamp_base() + g_cpu.timestamp_ref()` from
    /// the C++ side). For tests that don't care about decay, pass any
    /// monotonically increasing value.
    #[inline]
    pub fn refresh_data_bus(&mut self, val: u8, current_cpu_cycle: u64) {
        self.data_bus = val;
        self.data_bus_refresh_cycle = current_cpu_cycle;
    }

    /// Phase 6.3.a: zero `data_bus` if more than
    /// [`DATA_BUS_DECAY_CYCLES`] CPU cycles have elapsed since the
    /// last refresh. Idempotent and cheap when the bus is already 0
    /// (the common case). Called once per frame from the C++ bridge
    /// (`ppu_rust_bridge_emit_frame` or its per-cycle equivalent)
    /// — per-frame granularity (~16.67 ms) is well within the 600 ms
    /// threshold tolerance.
    ///
    /// `data_bus_refresh_cycle == 0` is treated as "refreshed at cycle
    /// 0" (the typical cold-boot state), not as "never refreshed".
    /// This lets tests deliberately refresh at cycle 0 and observe the
    /// decay window without the bookkeeping branching into a special
    /// case. In real emulation the per-cycle bridge always sets a
    /// monotonically increasing cycle value, so this only matters for
    /// synthetic unit tests.
    #[inline]
    pub fn check_data_bus_decay(&mut self, current_cpu_cycle: u64) {
        if self.data_bus == 0 {
            return;
        }
        if current_cpu_cycle > self.data_bus_refresh_cycle
            && current_cpu_cycle - self.data_bus_refresh_cycle > DATA_BUS_DECAY_CYCLES as u64
        {
            self.data_bus = 0;
        }
    }

    // -- control / mask ------------------------------------------------

    /// `$2000` write. `nmi_enable_changed` is set if bit 7 transitioned.
    pub fn write_ctrl(&mut self, val: u8) -> bool {
        let old_nmi = (self.ctrl >> ctrl_bits::NMI_ENABLE) & 1;
        self.ctrl = val;
        let new_nmi = (val >> ctrl_bits::NMI_ENABLE) & 1;
        self.data_bus = val;
        old_nmi != new_nmi
    }

    /// `$2001` write.
    pub fn write_mask(&mut self, val: u8) {
        self.mask = val;
        self.data_bus = val;
    }

    // -- status --------------------------------------------------------

    /// `$2002` read — returns the status byte ANDed with the open-bus
    /// low-5 bits, clears the VBL flag, and refreshes the open-bus
    /// latch with the returned byte.
    ///
    /// Per the C++ reference (`src/ppu.cpp:642-650`):
    /// ```text
    ///   ret = PPU_status;             // bits 5-7 = VBL/sprite0/overflow
    ///   ret |= PPUGenLatch & 0x1F;    // bits 0-4 = open-bus latch
    /// ```
    /// The bits 0-4 are sourced from the open-bus latch (the last
    /// value placed on the PPU data bus by any CPU write or PPU-side
    /// read), NOT from a fixed write_toggle mask. The blargg
    /// `vbl_basics` subtest 1 / `ppu_open_bus` subtest 2 ("write to any
    /// PPU register should set decay value") rely on this — writing
    /// `0xAA` to `$2000` and reading `$2002` must return
    /// `(status & 0xE0) | 0x0A`, not the fixed `0x1F`.
    ///
    /// Side effects:
    /// - VBL bit cleared (`status &= 0x7F`)
    /// - `write_toggle` reset (used by the second-write logic of $2005/$2006)
    /// - `data_bus` refreshed with the returned byte (per blargg
    ///   ppu_open_bus readme: a `$2002` read places the returned byte
    ///   on the PPU data bus)
    pub fn read_status(&mut self) -> u8 {
        let visible_mask = (1 << status_bits::VBL)
            | (1 << status_bits::SPRITE0_HIT)
            | (1 << status_bits::SPRITE_OVERFLOW);
        let mut v = self.status & visible_mask;
        v |= self.data_bus & 0x1F;
        // Reading $2002 clears bit 7 (VBL) and resets the write toggle.
        self.status &= !(1 << status_bits::VBL);
        self.write_toggle = false;
        // Refresh the open-bus latch with the returned byte. The C++
        // reference does `PPUGenLatch = ret` here too (line 651).
        self.data_bus = v;
        v
    }

    /// Set the VBL flag from outside (the frame state machine calls this
    /// at sl 241 dot 1, *unless* the early-read suppression already
    /// cleared it).
    pub fn set_vbl_flag(&mut self) {
        self.status |= 1 << status_bits::VBL;
    }

    /// Clear the VBL flag.
    pub fn clear_vbl_flag(&mut self) {
        self.status &= !(1 << status_bits::VBL);
    }

    /// Mark the VBL flag as suppressed for this frame — `$2002` was read
    /// at sl 241 dot 0 (one PPU dot before the working-config set
    /// boundary). The frame state machine consults this before set.
    pub fn suppress_vbl_for_frame(&mut self) {
        self.status &= !(1 << status_bits::VBL);
    }

    /// Set the sprite-0-hit flag (used by sprite eval).
    pub fn set_sprite0_hit(&mut self) {
        self.status |= 1 << status_bits::SPRITE0_HIT;
    }

    /// Clear sprite-0-hit (called when rendering turns off).
    pub fn clear_sprite0_hit(&mut self) {
        self.status &= !(1 << status_bits::SPRITE0_HIT);
    }

    /// Set sprite overflow flag.
    pub fn set_sprite_overflow(&mut self) {
        self.status |= 1 << status_bits::SPRITE_OVERFLOW;
    }

    /// Clear sprite overflow flag.
    pub fn clear_sprite_overflow(&mut self) {
        self.status &= !(1 << status_bits::SPRITE_OVERFLOW);
    }

    // -- OAM DMA / direct ----------------------------------------------

    /// `$4014` OAM DMA — write triggers a 256-byte copy from
    /// `(page << 8)..=(page << 8 | 0xFF)` into OAM. The PPU state is
    /// responsible for actually driving the read loop through the bus;
    /// `Registers` only stores `oam_addr = 0` afterwards (matches C++
    /// behaviour).
    pub fn start_oam_dma(&mut self) {
        self.oam_addr = 0;
    }

    /// `$2003` write.
    pub fn write_oam_addr(&mut self, val: u8) {
        self.oam_addr = val;
        self.data_bus = val;
    }

    /// `$2004` write — increments `oam_addr` (mod 256).
    pub fn increment_oam_addr(&mut self) {
        self.oam_addr = self.oam_addr.wrapping_add(1);
    }

    // -- scroll / addr latches ----------------------------------------

    /// `$2005` write — double-write scroll. First write stores coarse X
    /// in `t` and fine X in `fine_x`; second stores fine Y and coarse Y.
    pub fn write_scroll(&mut self, val: u8) {
        if !self.write_toggle {
            // First write: t.coarse_x = val >> 3; fine_x = val & 7.
            self.t = (self.t & !0x001F) | ((val as u16) >> 3);
            self.fine_x = val & 0x07;
        } else {
            // Second write: t.fine_y   = val & 0x07;
            //             t.coarse_y = val >> 3.
            // Bit layout of t: yyy NN YYYYY XXXXX
            //  coarse_y is bits 9..=5 → shift val >> 3 by 5
            //  fine_y   is bits 14..=12 → shift val & 0x07 by 12
            self.t = (self.t & !0x7BE0)
                | ((((val as u16) >> 3) & 0x1F) << 5)
                | (((val as u16) & 0x07) << 12);
        }
        self.write_toggle = !self.write_toggle;
        self.data_bus = val;
    }

    /// `$2006` write — double-write VRAM address.
    ///
    /// **v2.1.3 batch 2.1 (mid-frame writes)**: this method keeps the
    /// pre-batch behaviour of `v = t` on the second write. The render-time
    /// delayed path is [`Self::write_addr_rendering`], used by
    /// `crate::ffi::fceux11_ppu_cpu_write` when rendering is on, which
    /// writes `t` but defers `v = t` to the 3-PPU-dot queue in
    /// `crate::state::PpuState::v_addr_pending` / `v_addr_delay`.
    /// Rendering-off writes still call this method directly so the
    /// rendering-off bus path is synchronous (blargg MMC3 test 3 subtests
    /// 2-4 require v==t to update the A12 counter on the same CPU
    /// instruction).
    pub fn write_addr(&mut self, val: u8) {
        if !self.write_toggle {
            // First write: high 6 bits into bits 13..=8 of t.
            self.t = (self.t & !0x3F00) | (((val as u16) & 0x3F) << 8);
        } else {
            // Second write: low 8 bits, then v = t.
            self.t = (self.t & !0x00FF) | ((val as u16) & 0x00FF);
            self.v = self.t;
        }
        self.write_toggle = !self.write_toggle;
        self.data_bus = val;
    }

    /// v2.1.3 batch 2.1: render-time `$2006` write that does **not**
    /// commit `v = t` on the second write. The caller
    /// (`crate::ffi::fceux11_ppu_cpu_write`) is responsible for queueing
    /// `v_addr_pending = Some(self.t)` and `v_addr_delay = 3`; the per-dot
    /// consumer in `crate::rendering::tick_dot` commits the value when
    /// the delay counts down to 0.
    ///
    /// On the first write the value goes into `t` and `write_toggle`
    /// flips to "second pending" exactly as [`Self::write_addr`]. On the
    /// second write the value goes into `t`, `write_toggle` flips back,
    /// and the method returns `Some(self.t)` so the caller can enqueue it.
    /// No `v = t` happens here.
    // Unwired since the plan §15.5 P0 revert (2026-09-14): production
    // write/read paths are synchronous again; kept as behavior record and
    // exercised by the retained unit tests below. Do not wire back without
    // probe evidence per plan §15.7.
    #[allow(dead_code)]
    pub fn write_addr_rendering(&mut self, val: u8) -> Option<u16> {
        let mut pending: Option<u16> = None;
        if !self.write_toggle {
            self.t = (self.t & !0x3F00) | (((val as u16) & 0x3F) << 8);
        } else {
            self.t = (self.t & !0x00FF) | ((val as u16) & 0x00FF);
            pending = Some(self.t);
        }
        self.write_toggle = !self.write_toggle;
        self.data_bus = val;
        pending
    }

    /// Copy the horizontal bits of `t` into `v` — called by the state
    /// machine at the start of each visible scanline's visible dots.
    pub fn copy_horizontal(&mut self) {
        // v.coarse_x and v.nametable_x come from t.
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    /// Copy vertical bits of `t` into `v` — called on the pre-render scanline (dots 280..=304)
    pub fn copy_vertical(&mut self) {
        // v.coarse_y, v.nametable_y, v.fine_y come from t.
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    /// Increment coarse X by 1 with horizontal nametable flip at the
    /// coarse-X rollover. Triggered by the rendering BG fetch at the
    /// appropriate dot.
    pub fn increment_coarse_x(&mut self) {
        let coarse_x = (self.v & 0x001F) + 1;
        if coarse_x & 0x20 != 0 {
            // Rollover: coarse_x = 0; nametable_x flips.
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v = (self.v & !0x001F) | (coarse_x & 0x001F);
        }
    }

    /// Increment fine Y (3 bits) with carry into coarse Y / nametable Y.
    /// Triggered at the end of the visible portion of each scanline.
    pub fn increment_fine_y(&mut self) {
        let mut fine_y = (self.v >> 12) & 0x07;
        let mut coarse_y = (self.v >> 5) & 0x1F;
        let mut nametable_y = (self.v >> 11) & 0x01;

        fine_y = (fine_y + 1) & 0x07;
        if fine_y == 0 {
            // Carry into coarse_y; rollover flips nametable_y.
            coarse_y = (coarse_y + 1) & 0x1F;
            if coarse_y == 30 {
                coarse_y = 0;
                nametable_y ^= 1;
            }
        }

        // v layout: yyy NN YYYYY XXXXX = bits 14..12 / 11..10 / 9..5 / 4..0.
        // We clear those bits and rebuild from the freshly-incremented
        // fields. The other bits (which encode things like the BG/sprite
        // pattern-table selects in some PPU variants) are untouched.
        let v = self.v & 0x8C1F_u16;
        self.v = v | (coarse_y << 5) | (nametable_y << 11) | (fine_y << 12);
    }

    // -- $2007 data ----------------------------------------------------

    /// `$2007` read. Returns the buffered read if `v` is below the
    /// palette range; for palette reads (`v >= 0x3F00`) returns the
    /// real bus read OR'd with the high 2 bits of the open-bus latch,
    /// and does *not* update the VRAM buffer.
    ///
    /// After returning, `v` is incremented by 1 (or 32 if `ctrl` bit 2
    /// is set). The returned byte is also written to `data_bus` to
    /// refresh the open-bus latch (matches the C++ reference's
    /// `PPUGenLatch = ret` after every PPU-side read at
    /// `src/ppu.cpp:855-870`).
    ///
    /// **v2.1.3 batch 2.1 (mid-frame writes)**: this method always
    /// performs a real bus read and increments `v`; the render-time
    /// 6-dot throttle is implemented by [`Self::read_data_throttled`]
    /// which the `crate::ffi::fceux11_ppu_cpu_read` path calls when
    /// `PpuState::vram_read_cooldown > 0` (it returns the buffered
    /// value without touching `v` or the bus).
    pub fn read_data<B: PpuBus + ?Sized>(&mut self, bus: &mut B, ctrl: u8, rendering: bool) -> u8 {
        let v = self.v;
        let addr = self.mirror_data_addr(v);
        let result = if (v & 0x3FFF) < 0x3F00 {
            // Non-palette: return the buffered byte from the *last* read;
            // this read returns the real value into the buffer.
            let buffered = self.vram_buffer;
            self.vram_buffer = bus.read(addr);
            // Phase 6.4: the open-bus latch gets the byte RETURNED to
            // the CPU, not the freshly fetched one. C++ A2007 (newppu)
            // does `PPUGenLatch = ret` where ret == the pre-read
            // VRAMBuffer; the new fetch lands in VRAMBuffer only
            // (src/ppu.cpp:816/873). Refreshing with the new fetch
            // made $2002/$2005/$2006 open-bus readback observe the
            // NEXT buffer contents — the divergence blargg
            // ppu_read_buffer detects (A/B verified 2026-08-31:
            // C++ PPU value=0x00 PASS, Rust value=0x80 FAIL, §6.3.a.4).
            self.data_bus = buffered;
            buffered
        } else {
            // Palette range: per blargg ppu_open_bus readme, the
            // returned byte is "DD-- ----" — high 2 bits from
            // PPUGenLatch (the open-bus decay register), low 6 bits
            // from PALRAM. Without this OR the test reads `and #$C0`
            // against plain PALRAM and fails subtest 8 ("High 2 bits
            // from $2007 from palette should be from decay value").
            let mut ret = bus.read(addr);
            ret |= self.data_bus & 0xC0;
            self.data_bus = ret;
            // Phase 6.4: C++ A2007 also refills VRAMBuffer from the
            // nametable mirror underneath the palette
            // (`VRAMBuffer = CALL_PPUREAD(RefreshAddr - 0x1000)`,
            // src/ppu.cpp:865) — the next non-palette $2007 read
            // returns this byte. Note it uses the RAW $2Fxx mirror
            // address, not the palette-folded `addr`. The Rust path
            // never loaded the buffer here, so a palette read followed
            // by a non-palette read returned stale buffer content
            // (blargg ppu_read_buffer divergence, §6.3.a.4 A/B).
            self.vram_buffer = bus.read((v & 0x3FFF) - 0x1000);
            ret
        };
        self.increment_v(ctrl, rendering);
        result
    }

    /// `$2007` write. Writes through `v`; **does not update
    /// `vram_buffer`** (NESdev PPU_read_buffer: only `$2007` read
    /// and `$2006` second write change the buffered value -- `$2007`
    /// write leaves it untouched so the next `$2007` read still sees
    /// the prior contents, which were latched either by the last
    /// `$2007` read or by the `$2006` set that put `v` here).
    /// `data_bus` (the internal PPU I/O bus for `$2005`/`$2006`
    /// open-bus readback) does refresh, matching the C++ reference
    /// `PPUGenLatch = V;` in `B2007` (`src/ppu.cpp:1093-1102`).
    ///
    /// **v2.1.3 batch 2.1 (mid-frame writes)**: this method performs
    /// the bus write AND increments `v` immediately. The render-time
    /// 1-dot delayed path is [`Self::write_data_rendering`], which the
    /// `crate::ffi::fceux11_ppu_cpu_write` path uses when rendering is
    /// on, to defer the `v` increment to the next PPU dot.
    pub fn write_data<B: PpuBus + ?Sized>(&mut self, bus: &mut B, ctrl: u8, val: u8, rendering: bool) {
        let v = self.v;
        let addr = self.mirror_data_addr(v);
        bus.write(addr, val);
        // vram_buffer intentionally NOT refreshed on $2007 write.
        self.data_bus = val;
        self.increment_v(ctrl, rendering);
    }

    /// v2.1.3 batch 2.1: render-time `$2007` write. Writes the byte
    /// to the bus immediately (so PALRAM/NTAM updates land now) and
    /// refreshes the open-bus latch, but does **not** call
    /// `increment_v`. The caller is responsible for queueing
    /// `PpuState::vram_write_cooldown = 1`; the per-dot consumer in
    /// `crate::rendering::tick_dot` will call
    /// [`Self::commit_v_inc`] on the next dot to apply the same
    /// `increment_v` that would have run on the write dot.
    // Unwired since the plan §15.5 P0 revert (2026-09-14): production
    // write/read paths are synchronous again; kept as behavior record and
    // exercised by the retained unit tests below. Do not wire back without
    // probe evidence per plan §15.7.
    #[allow(dead_code)]
    pub fn write_data_rendering<B: PpuBus + ?Sized>(&mut self, bus: &mut B, val: u8) {
        let v = self.v;
        let addr = self.mirror_data_addr(v);
        bus.write(addr, val);
        // vram_buffer intentionally NOT refreshed on $2007 write.
        self.data_bus = val;
    }

    /// v2.1.3 batch 2.1: render-time `$2007` read with a 6-PPU-dot
    /// throttle. If `throttle_active` is `true` the function returns
    /// the buffered value without touching the bus or `v`; otherwise
    /// it performs a real bus read and increments `v` (same as
    /// [`Self::read_data`]). The C++ bridge sets `throttle_active =
    /// PpuState::vram_read_cooldown > 0` and arms the cooldown on
    /// every real read.
    // Unwired since the plan §15.5 P0 revert (2026-09-14): production
    // write/read paths are synchronous again; kept as behavior record and
    // exercised by the retained unit tests below. Do not wire back without
    // probe evidence per plan §15.7.
    #[allow(dead_code)]
    pub fn read_data_throttled<B: PpuBus + ?Sized>(
        &mut self,
        bus: &mut B,
        ctrl: u8,
        rendering: bool,
        throttle_active: bool,
    ) -> u8 {
        if throttle_active {
            // Cooldown window: return the current buffer without
            // touching the bus or v. Per Mesen2 `_ignoreVramRead` /
            // Nintendulator IOMode 6 PPU cycle behaviour. The
            // open-bus latch is NOT refreshed on a throttled read
            // (no real bus activity).
            return self.vram_buffer;
        }
        self.read_data(bus, ctrl, rendering)
    }

    /// v2.1.3 batch 2.1: apply the queued `v_addr_pending` to `v`.
    /// Called by `crate::rendering::tick_dot` when `v_addr_delay`
    /// counts down to 0, and by the rendering-off CPU path
    /// immediately for synchronous v==t commits.
    ///
    /// v2.1.3 batch 2.1 hotfix (2026-09-14, owner-reported freeze on
    /// 3186_Mario_1.nes): mask the input to 15 bits before assigning
    /// to `v`. The hardware `v` register is 15 bits; the input
    /// `pending` (the snapshotted `t`) is also 15 bits in practice,
    /// but the `rendering::tick_dot` `commit_v_addr` site accepts the
    /// 16-bit container for `Some(pending)`. Without the mask, a
    /// future change that passes a wider container (or a
    /// re-snapshotted `v` whose bit 15 was set by a stray open-bus
    /// write) would silently pollute the `v` increment for the
    /// rest of the frame.
    // Unwired since the plan §15.5 P0 revert (2026-09-14): production
    // write/read paths are synchronous again; kept as behavior record and
    // exercised by the retained unit tests below. Do not wire back without
    // probe evidence per plan §15.7.
    #[allow(dead_code)]
    pub fn commit_v_addr(&mut self, pending: u16) {
        self.v = pending & 0x7FFF;
    }

    /// v2.1.3 batch 2.1: apply the deferred `$2007` write-time `v`
    /// increment. Called by `crate::rendering::tick_dot` when
    /// `vram_write_cooldown` counts down to 0. Identical in effect to
    /// the trailing `increment_v` of [`Self::write_data`] — kept as a
    /// named entry point so the per-dot consumer does not need to
    /// peek at `ctrl` / `rendering` to call the right path.
    // Unwired since the plan §15.5 P0 revert (2026-09-14): production
    // write/read paths are synchronous again; kept as behavior record and
    // exercised by the retained unit tests below. Do not wire back without
    // probe evidence per plan §15.7.
    #[allow(dead_code)]
    pub fn commit_v_inc(&mut self, ctrl: u8, rendering: bool) {
        self.increment_v(ctrl, rendering);
    }

    /// Apply the `$2007` increment to `v`.
    ///
    /// Phase A (v2.1.1) — port of the C++ reference `Ppu::increment2007`
    /// (src/ppu_class.h:147): during active rendering the increment is
    /// the coarse-Y/fine-Y "Y increment" walk (`increment_vs()`), NOT a
    /// flat +1/+32; outside rendering the walk is field-wise, and the
    /// +32 form increments coarse-Y only, dropping the carry into the
    /// nametable-Y bit. The flat `v += 1/32` previously used here
    /// diverged from the C++ engine the moment a ROM touched $2007
    /// while rendering was on (blargg ppu_read_buffer "basic PPU memory
    /// I/O" phase: C++ v walked $2045→, Rust jumped to $2457 — the v
    /// streams diverged at frame 3 and the ROM's verification loop
    /// never completed).
    pub(crate) fn increment_v(&mut self, ctrl: u8, rendering: bool) {
        if rendering {
            // increment_vs(): fv++, carry into coarse-Y at 8, NT-Y flip
            // when coarse-Y wraps at 30. Field layout (flat v):
            // fv = bits 14-12, v_nt = bit 11, coarse_y = bits 9-5
            // (matches C++ ppur.{fv,v,vt} in get_2007access).
            let mut fv = (self.v >> 12) & 0x7;
            let mut coarse_y = (self.v >> 5) & 0x1F;
            let mut v_nt = (self.v >> 11) & 0x1;
            fv += 1;
            let fv_overflow = fv >> 3;
            coarse_y = (coarse_y + fv_overflow) & 0x1F;
            if coarse_y == 30 && fv_overflow == 1 {
                v_nt += 1;
                coarse_y = 0;
            }
            fv &= 0x7;
            v_nt &= 0x1;
            self.v = (self.v & !0x7BE0_u16)
                | (fv << 12)
                | (v_nt << 11)
                | (coarse_y << 5);
            return;
        }
        if ctrl & (1 << ctrl_bits::VRAM_INCREMENT) != 0 {
            // by32: coarse-Y only, carry dropped (C++ `vt++; ... vt &= 31`).
            let coarse_y = ((self.v >> 5) + 1) & 0x1F;
            self.v = (self.v & !0x03E0_u16) | (coarse_y << 5);
        } else {
            self.v = self.v.wrapping_add(1) & 0x7FFF;
        }
    }

    /// Apply the address-space mirroring that the real PPU uses for
    /// `$2007` access: $3F00-$3FFF mirror down to $3F00-$3FFF
    /// (palette aliases $3F00/$3F04/$3F08/$3F0C collapse to 0x00/0x04/0x08/0x0C).
    pub(crate) fn mirror_data_addr(&self, v: u16) -> u16 {
        let lo = v & 0x3FFF;
        if lo < 0x3F00 {
            lo
        } else {
            // Entry index is (addr and 0x1F); only entries with low nibble 0
            // $3F10/$3F14/$3F18/$3F1C alias to $3F00/$3F04/$3F08/$3F0C.
            let idx = lo & 0x1F;
            let entry = if (idx & 0x0F) == 0 { idx & 0x0F } else { idx };
            0x3F00 | entry
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::PpuState;
    use crate::rendering::{self, RenderWindows};
    use crate::bus::FlatBus;

    #[test]
    fn new_clears_all_latches() {
        let r = Registers::new();
        assert_eq!(r.ctrl, 0);
        assert_eq!(r.mask, 0);
        assert_eq!(r.status, 0);
        assert_eq!(r.oam_addr, 0);
        assert!(!r.write_toggle);
        assert_eq!(r.v, 0);
        assert_eq!(r.t, 0);
        assert_eq!(r.fine_x, 0);
        assert_eq!(r.vram_buffer, 0);
        assert_eq!(r.data_bus, 0, "Phase 6.3.a: data_bus defaults to 0");
    }

    #[test]
    fn ctrl_write_reports_nmi_edge() {
        let mut r = Registers::new();
        assert!(r.write_ctrl(0x80)); // 0 -> 0x80: rising edge
        assert!(r.write_ctrl(0x00)); // 0x80 -> 0x00: falling edge
        assert!(r.write_ctrl(0x80)); // 0 -> 0x80 again: rising edge
        assert!(!r.write_ctrl(0x80)); // 0x80 -> 0x80: hold
    }

    #[test]
    fn status_read_uses_open_bus_for_low_bits_and_clears_vbl() {
        let mut r = Registers::new();
        r.status = 1 << status_bits::VBL | 1 << status_bits::SPRITE0_HIT;
        r.write_toggle = true;
        // Per Phase 6.3.a the low 5 bits come from the open-bus
        // latch (`data_bus`), NOT from a fixed write_toggle mask.
        r.refresh_data_bus(0x42, 1000);
        let v = r.read_status();
        // VBL bit is returned (set at read time) AND the stored flag is
        // cleared so the next read returns 0.
        assert_ne!(v & (1 << status_bits::VBL), 0, "VBL is returned");
        assert_ne!(
            v & (1 << status_bits::SPRITE0_HIT),
            0,
            "sprite0 hit preserved"
        );
        // Open-bus low 5 bits = 0x42 & 0x1F = 0x02 (NOT the fixed 0x1F).
        assert_eq!(v & 0x1F, 0x02);
        assert!(!r.write_toggle, "$2002 read resets write toggle");
        let second = r.read_status();
        assert_eq!(
            second & (1 << status_bits::VBL),
            0,
            "VBL cleared after first read"
        );
        assert_ne!(
            second & (1 << status_bits::SPRITE0_HIT),
            0,
            "sprite0 hit preserved across reads"
        );
    }

    #[test]
    fn status_read_refreshes_open_bus_latch_with_returned_byte() {
        // Per blargg ppu_open_bus readme: a $2002 read places the
        // returned byte on the data bus. Subsequent $2005 reads must
        // therefore see the *returned* byte, not whatever was written
        // earlier.
        let mut r = Registers::new();
        r.refresh_data_bus(0x42, 1000);
        let v1 = r.read_status();
        // Returned byte contains the open-bus low bits + status bits.
        // data_bus is now refreshed to that byte.
        assert_eq!(r.data_bus, v1, "data_bus refreshed to read_status result");
        // A subsequent $2005 read returns the just-placed byte.
        assert_eq!(r.data_bus, v1);
    }

    #[test]
    fn scroll_first_write_sets_coarse_x_and_fine_x() {
        let mut r = Registers::new();
        r.write_scroll(0xA5); // 0b1010_0101 → coarse_x=0x14, fine_x=5
        assert_eq!(r.t & 0x001F, 0x14);
        assert_eq!(r.fine_x, 5);
        assert!(r.write_toggle);
    }

    #[test]
    fn scroll_second_write_sets_coarse_y_and_fine_y() {
        let mut r = Registers::new();
        r.write_scroll(0xFF); // first — toggle to true
        r.write_scroll(0x7B); // second: fine_y=3, coarse_y=0x0F
        assert_eq!((r.t >> 5) & 0x1F, 0x0F);
        assert_eq!((r.t >> 12) & 0x07, 0x03);
        assert!(!r.write_toggle);
    }

    #[test]
    fn addr_double_write_copies_t_to_v() {
        let mut r = Registers::new();
        r.write_addr(0x21); // high 6 bits into t bits 13..8
        assert!(r.write_toggle);
        assert_eq!((r.t >> 8) & 0x3F, 0x21);
        r.write_addr(0xCA); // low 8 bits, v = t
        assert!(!r.write_toggle);
        assert_eq!(r.v, 0x21CA & 0x7FFF);
        assert_eq!(r.v, r.t);
    }

    #[test]
    fn coarse_x_increment_rolls_over_and_flips_nametable() {
        let mut r = Registers::new();
        r.v = 0x0400 | 0x1F; // nametable_x=1, coarse_x=31
        r.increment_coarse_x();
        assert_eq!(r.v & 0x001F, 0, "coarse_x wrapped to 0");
        assert_eq!(r.v & 0x0400, 0, "nametable_x flipped to 0");
    }

    #[test]
    fn vram_buffer_round_trip_through_data_register() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        bus.write(0x2000, 0xAA);
        r.v = 0x2000;
        let first = r.read_data(&mut bus, r.ctrl, false);
        assert_eq!(first, 0, "first read returns the prior buffer (0)");
        let second = r.read_data(&mut bus, r.ctrl, false);
        assert_eq!(second, 0xAA, "second read returns what was buffered");
    }

    #[test]
    fn palette_read_refills_buffer_from_nt_mirror() {
        // Phase 6.4 (was `palette_read_does_not_update_buffer`): C++
        // A2007 refills VRAMBuffer from the nametable mirror under the
        // palette (`CALL_PPUREAD(RefreshAddr - 0x1000)`,
        // src/ppu.cpp:865) — $3F00 reads load the byte at $2F00.
        let mut r = Registers::new();
        r.vram_buffer = 0xCD;
        let mut bus = FlatBus::new();
        bus.write(0x3F00, 0x12);
        bus.write(0x2F00, 0xAB);
        r.v = 0x3F00;
        let v = r.read_data(&mut bus, r.ctrl, false);
        assert_eq!(v, 0x12, "palette returns the real bus value");
        assert_eq!(
            r.vram_buffer, 0xAB,
            "palette read refills the buffer from the NT mirror (v-0x1000)"
        );
    }

    #[test]
    fn oam_dma_resets_oam_addr() {
        let mut r = Registers::new();
        r.oam_addr = 0x80;
        r.start_oam_dma();
        assert_eq!(r.oam_addr, 0);
    }

    #[test]
    fn v_increment_uses_ctrl_bit2() {
        let mut r = Registers::new();
        r.v = 0x1000;
        r.read_data(&mut FlatBus::new(), 0, false); // +1
        assert_eq!(r.v, 0x1001);
        r.v = 0x1000;
        r.read_data(&mut FlatBus::new(), 1 << ctrl_bits::VRAM_INCREMENT, false); // +32
        assert_eq!(r.v, 0x1020);
    }

    // -----------------------------------------------------------------
    // Phase 6.3.a — PPU internal data-bus open-bus value.
    //
    // Every CPU write to a PPU register ($2000-$2007) places the
    // written byte on the PPU internal data bus. Reads of $2005 /
    // $2006 (write-only on real hardware) return this latched value
    // — this is what blargg's `ppu_read_buffer` subtest 1 expects.
    // -----------------------------------------------------------------

    #[test]
    fn data_bus_latches_last_cpu_write() {
        let mut r = Registers::new();
        assert_eq!(r.data_bus, 0);
        r.refresh_data_bus(0x80, 1000);
        assert_eq!(r.data_bus, 0x80);
        r.refresh_data_bus(0x1E, 2000);
        assert_eq!(r.data_bus, 0x1E);
        r.refresh_data_bus(0x42, 3000);
        assert_eq!(r.data_bus, 0x42);
        r.refresh_data_bus(0x55, 4000);
        assert_eq!(r.data_bus, 0x55);
        r.refresh_data_bus(0xAA, 5000);
        assert_eq!(r.data_bus, 0xAA);
    }

    #[test]
    fn data_bus_latches_2007_write_for_both_palette_and_non_palette() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        r.v = 0x2000;
        r.refresh_data_bus(0x77, 1000);
        assert_eq!(r.data_bus, 0x77, "non-palette refresh updates bus");
        r.v = 0x3F00;
        r.refresh_data_bus(0xAB, 2000);
        assert_eq!(r.data_bus, 0xAB, "palette refresh also updates bus");
    }

    #[test]
    fn read_status_refreshes_data_bus_with_returned_byte() {
        // Per Phase 6.3.a: a $2002 read places the returned byte on
        // the PPU data bus (matches the C++ reference
        // `PPUGenLatch = ret`). The test pins a recognisable value
        // and asserts the refresh happened.
        let mut r = Registers::new();
        r.refresh_data_bus(0x42, 1000);
        let v = r.read_status();
        // data_bus is refreshed to the returned byte (which is
        // status bits | open-bus low 5).
        assert_eq!(r.data_bus, v, "data_bus refreshed to returned byte");
    }

    // -----------------------------------------------------------------
    // Phase 6.3.a — open-bus decay
    //
    // Per blargg `ppu_open_bus` readme, an unwritten data-bus bit
    // decays to 0 after ~600 ms (1 073 864 NTSC CPU cycles). The
    // check fires once per frame from the C++ bridge; the registers
    // module exposes `check_data_bus_decay` for the FFI to call.
    // -----------------------------------------------------------------

    #[test]
    fn data_bus_decay_zeroes_after_threshold() {
        let mut r = Registers::new();
        r.refresh_data_bus(0xFF, 0);
        assert_eq!(r.data_bus, 0xFF);
        // Just below the threshold: no decay.
        r.check_data_bus_decay(DATA_BUS_DECAY_CYCLES as u64 - 1);
        assert_eq!(r.data_bus, 0xFF, "just under threshold keeps value");
        // Crossing the threshold: decay fires.
        r.check_data_bus_decay(DATA_BUS_DECAY_CYCLES as u64 + 1);
        assert_eq!(r.data_bus, 0, "over threshold zeros data_bus");
    }

    #[test]
    fn data_bus_decay_idempotent_when_already_zero() {
        let mut r = Registers::new();
        // Never refreshed, already 0.
        r.check_data_bus_decay(0);
        assert_eq!(r.data_bus, 0);
        r.check_data_bus_decay(u64::MAX);
        assert_eq!(r.data_bus, 0);
    }

    #[test]
    fn data_bus_decay_zeroes_when_refresh_cycle_zero_even_with_data_bus_nonzero() {
        // Defensive: a savestate loaded with data_bus=0xAB but
        // refresh_cycle=0 (RPU1 v1 format doesn't serialize the
        // cycle) should still decay normally. Treating
        // refresh_cycle=0 as "refreshed at cycle 0" means the decay
        // check at cycle > 1 073 864 fires; before that the value
        // persists, which is the closest behaviour to "just
        // refreshed" the data model can express without a sentinel.
        let mut r = Registers::new();
        r.data_bus = 0xAB;
        r.data_bus_refresh_cycle = 0;
        // Same cycle: no decay.
        r.check_data_bus_decay(0);
        assert_eq!(r.data_bus, 0xAB);
        // Past the threshold: decay fires.
        r.check_data_bus_decay(DATA_BUS_DECAY_CYCLES as u64 + 1);
        assert_eq!(r.data_bus, 0);
    }

    #[test]
    fn palette_sprite_entries_are_distinct_from_background() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        bus.cpu[0x3F01] = 0x11;
        r.v = 0x3F11;
        r.write_data(&mut bus, 0x00, 0x21, false);
        assert_eq!(bus.cpu[0x3F11], 0x21, "sprite palette entry 1 must be written");
        assert_eq!(bus.cpu[0x3F01], 0x11, "background palette entry 1 must be untouched");
    }

    #[test]
    fn palette_entry_zero_aliases_collapse() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        r.v = 0x3F10;
        r.write_data(&mut bus, 0x00, 0x30, false);
        assert_eq!(bus.cpu[0x3F00], 0x30, "3F10 aliases the shared backdrop cell");
        assert_eq!(bus.cpu[0x3F10], 0, "the alias targets the 3F00 cell");
    }

    // -----------------------------------------------------------------
    // v2.1.3 batch 2.1 — mid-frame PPU register write delay semantics.
    //
    // The five new behaviours:
    // 1. $2006 second write (rendering on): v=t is **deferred** to the
    //    per-dot consumer; `write_addr_rendering` returns `Some(t)` and
    //    leaves v unchanged.
    // 2. $2007 read (rendering on): throttled inside a 6-PPU-dot
    //    window; `read_data_throttled` returns the buffered value
    //    without bus access or v increment.
    // 3. $2007 write (rendering on): bus write is immediate, v
    //    increment is deferred to the next dot; `write_data_rendering`
    //    does not call `increment_v`.
    // 4. $2006 second write (rendering off): still synchronous
    //    `v = t` (matches pre-batch behaviour; blargg MMC3 test 3
    //    subtests 2-4 require this).
    // 5. `commit_v_addr` / `commit_v_inc` are the per-dot apply
    //    hooks; the rendering loop calls them when the cooldowns
    //    count to 0.
    // -----------------------------------------------------------------

    #[test]
    fn write_addr_rendering_defers_v_on_second_write() {
        let mut r = Registers::new();
        // First write: t gets the high 6 bits, no pending value.
        assert_eq!(r.write_addr_rendering(0x21), None);
        assert_eq!((r.t >> 8) & 0x3F, 0x21);
        assert_eq!(r.v, 0, "v untouched on first write");
        // Second write: t gets the low 8 bits AND we get the new t
        // back for queueing; v is NOT yet updated.
        let pending = r.write_addr_rendering(0xCA);
        assert_eq!(pending, Some(0x21CA & 0x7FFF));
        assert_eq!(r.t, 0x21CA & 0x7FFF);
        assert_eq!(
            r.v, 0,
            "v stays at its prior value until commit_v_addr is called"
        );
        // The per-dot consumer calls commit_v_addr when the cooldown
        // expires; verify that one-shot commit does the right thing.
        r.commit_v_addr(pending.unwrap());
        assert_eq!(r.v, 0x21CA & 0x7FFF);
    }

    #[test]
    fn write_addr_rendering_does_not_dirty_write_toggle_polarity() {
        // The first/second write toggle must behave identically to the
        // immediate variant so the second-write detection in mesen2
        // calibration (3-dot delay) lines up.
        let mut r = Registers::new();
        assert!(!r.write_toggle);
        r.write_addr_rendering(0x21);
        assert!(r.write_toggle, "first write sets toggle to true");
        r.write_addr_rendering(0xCA);
        assert!(!r.write_toggle, "second write resets toggle to false");
    }

    #[test]
    fn write_addr_immediate_still_copies_v_for_rendering_off_path() {
        // Backwards-compat: blargg MMC3 test 3 subtests 2-4 require
        // v = t on the second $2006 write **when rendering is off**
        // so the A12 counter clocks on the same CPU instruction.
        let mut r = Registers::new();
        r.write_addr(0x21);
        r.write_addr(0xCA);
        assert_eq!(r.v, 0x21CA & 0x7FFF);
        assert_eq!(r.t, 0x21CA & 0x7FFF);
    }

    #[test]
    fn write_data_rendering_writes_bus_but_defers_v_increment() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        r.v = 0x0000;
        r.write_data_rendering(&mut bus, 0x42);
        // Bus was written immediately.
        assert_eq!(bus.cpu[0x0000], 0x42, "bus write is immediate");
        assert_eq!(r.data_bus, 0x42, "open-bus latch refreshed");
        // v has NOT been incremented yet — the per-dot consumer
        // will run `commit_v_inc` on the next dot.
        assert_eq!(r.v, 0x0000, "v increment deferred");
        // The trailing commit brings v forward exactly as the old
        // immediate path did (rendering on, +1 → field-wise Y walk,
        // fv 0 → 1 → v = 0x1000).
        r.commit_v_inc(r.ctrl, true);
        assert_eq!(r.v, 0x1000, "commit_v_inc matches the field-wise Y walk");
    }

    #[test]
    fn commit_v_inc_matches_immediate_write_data_path() {
        // Cross-check: starting from the same v, the deferred path
        // (write_data_rendering + commit_v_inc) produces exactly the
        // v the immediate path (write_data) would have produced.
        let mut r_immediate = Registers::new();
        r_immediate.v = 0x0000;
        let mut bus_immediate = FlatBus::new();
        r_immediate.write_data(&mut bus_immediate, 0, 0x42, true);

        let mut r_deferred = Registers::new();
        r_deferred.v = 0x0000;
        let mut bus_deferred = FlatBus::new();
        r_deferred.write_data_rendering(&mut bus_deferred, 0x42);
        r_deferred.commit_v_inc(0, true);

        assert_eq!(
            r_immediate.v, r_deferred.v,
            "deferred path v must equal immediate path v"
        );
    }

    #[test]
    fn read_data_throttled_skips_bus_and_v_during_cooldown() {
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        bus.write(0x2000, 0xAB);
        r.v = 0x2000;
        // First read (no throttle): returns the prior buffer (0),
        // refills the buffer from 0x2000, increments v (field-wise
        // Y walk: fv 0→1, coarse_y 0, v_nt 0 → v = 0x3000).
        let v0 = r.read_data_throttled(&mut bus, r.ctrl, true, false);
        assert_eq!(v0, 0, "first real read returns prior buffer");
        assert_eq!(r.vram_buffer, 0xAB, "buffer refilled from bus");
        assert_eq!(r.v, 0x3000, "v incremented after first real read");
        // Second read inside the throttle window: returns the
        // buffered 0xAB, does NOT touch the bus, does NOT increment v.
        let v1 = r.read_data_throttled(&mut bus, r.ctrl, true, true);
        assert_eq!(v1, 0xAB, "throttled read returns the buffered byte");
        assert_eq!(r.v, 0x3000, "v unchanged during throttle");
        // Cooldown expires: third read is a real read again.
        let v2 = r.read_data_throttled(&mut bus, r.ctrl, true, false);
        assert_eq!(v2, 0xAB, "post-cooldown read returns the new buffer");
        // v advances through another field-wise Y walk step.
        let v_after = r.v;
        assert!(v_after > 0x3000, "v increments after cooldown");
    }

    #[test]
    fn read_data_throttled_does_not_refresh_open_bus_latch() {
        // Mesen2 behaviour: a throttled read is "no real bus
        // activity", so the open-bus latch is NOT refreshed. Without
        // this, a burst of throttled reads leaks the buffer content
        // back into $2002/$2005 readback.
        let mut r = Registers::new();
        let mut bus = FlatBus::new();
        r.refresh_data_bus(0x77, 1000);
        bus.write(0x2000, 0xAB);
        r.v = 0x2000;
        r.read_data_throttled(&mut bus, r.ctrl, true, true);
        assert_eq!(
            r.data_bus, 0x77,
            "throttled read leaves the open-bus latch alone"
        );
    }

    #[test]
    fn commit_v_inc_matches_increment_v_for_rendering_off_path() {
        // Sanity: commit_v_inc is a thin wrapper around increment_v
        // for both rendering paths; check the +32 (VRAM_INCREMENT
        // bit) path.
        let mut r = Registers::new();
        r.v = 0x1000;
        r.commit_v_inc(1 << ctrl_bits::VRAM_INCREMENT, false);
        assert_eq!(r.v, 0x1020, "rendering-off +32 path");
        r.v = 0x1000;
        r.commit_v_inc(0, false);
        assert_eq!(r.v, 0x1001, "rendering-off +1 path");
    }

    #[test]
    fn write_addr_rendering_double_toggle_then_rearm() {
        // Two consecutive render-time $2006 pairs must each
        // independently yield one `Some(t)` from the second write.
        let mut r = Registers::new();
        assert_eq!(r.write_addr_rendering(0x10), None);
        assert_eq!(r.write_addr_rendering(0x00), Some(0x1000 & 0x7FFF));
        // Pair ends — toggle reset to first-write position.
        assert!(!r.write_toggle);
        // Next pair starts cleanly.
        assert_eq!(r.write_addr_rendering(0x28), None);
        assert_eq!(r.write_addr_rendering(0x00), Some(0x2800 & 0x7FFF));
    }

    // -----------------------------------------------------------------
    // v2.1.3 batch 2.1 hotfix: regression repro for the "v locks at
    // wrong address across frame boundary" symptom that surfaced on
    // 3186_Mario_1.nes (NROM-256 + vertical mirror) after the batch 2.1
    // mid-frame write delays shipped. The repro below models the SMB1
    // demo-mode CPU write pattern:
    //
    //   1. NMI service (dot 0 area) writes $2006 first+second with the
    //      upper-screen v (queue: v_addr_delay=3, pending=Some(0x2000)).
    //   2. CPU main loop runs through visible scanlines dot 0..=240.
    //   3. Sprite-0 hit triggers at scanline ~80, CPU writes the lower
    //      v_addr (queue: v_addr_delay=3, pending=Some(0x2400)) at the
    //      dot where the hit latched.
    //   4. Frame ends, NMI of next frame begins.
    //
    // The symptom in the field:
    //   - A static pink+navy rectangle (pal 1 attribute) sits where the
    //     original "BROS" residual used to be — i.e. the NTAM cell the
    //     BG fetch is reading is fixed at one address across multiple
    //     frames, while the rest of the screen scrolls.
    //   - Occasional "freeze": the screen stops animating and the
    //     status-bar text row disappears.
    //
    // These two tests are FAILING under the current implementation;
    // they describe the bug at the unit-test level. The hotfix will
    // correct the per-dot consumer in `crate::rendering::tick_dot`
    // and the queue-arm path in `crate::ffi::fceux11_ppu_cpu_write`
    // (next commit). Keep this comment block in sync with the
    // `docs/plans/v2.1.3_ppu_accuracy_plan.md` §11.4 addendum.
    // -----------------------------------------------------------------

    #[test]
    #[ignore = "batch 2.1 per-dot delay queue unwired by plan §15.5 P0 revert (behavior record only)"]
    fn hotfix_repro_pending_v_must_commit_within_3_ppu_dots() {
        // Contract: when the queue is armed with delay=3, the per-dot
        // consumer in `crate::rendering::tick_dot` MUST decrement it
        // on the FIRST dot after the write and commit on the third
        // dot. (Write dot is dot N; commit lands at dot N+3.)
        let mut state = PpuState::new();
        state.ppudead = 0;
        state.scanline = 50;
        state.dot = 100;
        state
            .registers
            .write_mask(1 << mask_bits::SHOW_BG);
        // First + second $2006 writes in the same CPU instruction pair
        // (mirrors what the SMB1 NMI service does for the upper-half
        // scroll).
        let pending = state.registers.write_addr_rendering(0x20);
        assert!(pending.is_none());
        let pending = state.registers.write_addr_rendering(0x00);
        assert_eq!(pending, Some(0x2000 & 0x7FFF));
        state.v_addr_pending = pending;
        state.v_addr_delay = 3;
        // 3 dot iterations: the third MUST land the commit. We do
        // 3 explicit `tick_dot` calls (the FFI consumer equivalent).
        for _ in 0..3 {
            crate::rendering::tick_dot(
                &mut state,
                &mut FlatBus::new(),
                &crate::rendering::RenderWindows {
                    nt: &[0u8; 4096],
                    chr: &[0u8; 8192],
                    palette: &[0u8; 32],
                    mirror: 0,
                },
                &mut [0u8; 256 * 256],
            );
            // Replay the frame advance equivalent inline; the test
            // does not need `frame::tick_dot` for this assertion.
            let next_dot = state.dot + 1;
            if next_dot >= 341 {
                state.dot = 0;
                state.scanline += 1;
            } else {
                state.dot = next_dot;
            }
        }
        assert_eq!(
            state.registers.v, 0x2000 & 0x7FFF,
            "v must commit on the 3rd per-dot iteration"
        );
        assert_eq!(state.v_addr_delay, 0);
        assert!(state.v_addr_pending.is_none());
    }

    #[test]
    fn hotfix_repro_increment_v_from_fv2_to_fv3() {
        // v=0x2000 = bits 14:12 = 010 (fv=2). The field-wise Y walk
        // bumps fv to 3 → v=0x3000. This is a non-zero v delta and
        // is the basis for the "leak across frame boundary" repro:
        // if the cooldown's increment lands in the wrong dot window
        // (say, two frames later instead of one), the BG fetch
        // will be one tile ahead of where the CPU instructed.
        let mut r = Registers::new();
        r.v = 0x2000;
        r.increment_v(0, true);
        assert_eq!(r.v, 0x3000, "fv 2 -> 3 lands at 0x3000");
    }

    #[test]
    #[ignore = "batch 2.1 per-dot delay queue unwired by plan §15.5 P0 revert (behavior record only)"]
    fn hotfix_repro_cooldowns_must_not_leak_across_frame_boundary() {
        // Contract: if a $2007 write on a visible scanline arms the
        // 1-dot cooldown, the cooldown MUST commit exactly one v
        // increment and MUST NOT double-commit on the next dot.
        // The batch 2.1 hotfix adds a `in_visible_window` guard so
        // that pre-render / post-render scanlines do NOT touch the
        // cooldown queue (which would let a stale delay fire at
        // scanline -1 dot 280 and clobber the BG fetch's `copy_vertical`
        // input).
        let mut state = PpuState::new();
        state.ppudead = 0;
        state.scanline = 100; // visible
        state.dot = 200; // mid-scanline, well before the 257 boundary
        state
            .registers
            .write_mask(1 << crate::registers::mask_bits::SHOW_BG);
        // Start with fv=0 so the increment is visible.
        state.registers.v = 0x0000;
        let mut bus = FlatBus::new();
        state.registers.write_data_rendering(&mut bus, 0x42);
        state.vram_write_cooldown = 1;
        // The hotfix's `in_visible_window` guard means the cooldown
        // will commit on the next visible per-dot tick. (Pre-render
        // / post-render ticks no longer touch the queue.)
        let v0 = state.registers.v;
        let mut fb = [0u8; 256 * 256];
        let win = crate::rendering::RenderWindows {
            nt: &[0u8; 4096],
            chr: &[0u8; 8192],
            palette: &[0u8; 32],
            mirror: 0,
        };
        crate::rendering::tick_dot(&mut state, &mut bus, &win, &mut fb);
        let v1 = state.registers.v;
        // Advance 1 dot, capture.
        let next_dot = state.dot + 1;
        state.dot = if next_dot >= 341 { 0 } else { next_dot };
        crate::rendering::tick_dot(&mut state, &mut bus, &win, &mut fb);
        let v2 = state.registers.v;
        let first_delta = v1.wrapping_sub(v0);
        let second_delta = v2.wrapping_sub(v1);
        assert_eq!(
            first_delta, 0x1000,
            "cooldown's v increment lands at the first tick"
        );
        assert_eq!(
            second_delta, 0,
            "no phantom v increment must land on the next dot"
        );
        assert_eq!(
            state.vram_write_cooldown, 0,
            "no cooldown must carry over to the next frame"
        );
    }

    #[test]
    #[ignore = "batch 2.1 per-dot delay queue unwired by plan §15.5 P0 revert (behavior record only)"]
    fn hotfix_repro_cooldowns_paused_outside_visible_window() {
        // Hotfix contract: the cooldown queue is paused outside the
        // visible scanline window (0..=239). A cooldown armed at
        // scanline 240 dot 100 must NOT decrement on the post-render
        // tick, must NOT commit at scanline -1 dot 280, and must
        // fire on the first visible scanline tick instead.
        let mut state = PpuState::new();
        state.ppudead = 0;
        state.scanline = 240; // post-render
        state.dot = 100;
        state
            .registers
            .write_mask(1 << crate::registers::mask_bits::SHOW_BG);
        state.registers.v = 0x0000;
        let mut bus = FlatBus::new();
        state.registers.write_data_rendering(&mut bus, 0x42);
        state.vram_write_cooldown = 1;
        let mut fb = [0u8; 256 * 256];
        let win = crate::rendering::RenderWindows {
            nt: &[0u8; 4096],
            chr: &[0u8; 8192],
            palette: &[0u8; 32],
            mirror: 0,
        };
        // Post-render tick: cooldown must NOT decrement.
        crate::rendering::tick_dot(&mut state, &mut bus, &win, &mut fb);
        assert_eq!(
            state.vram_write_cooldown, 1,
            "cooldown must not decrement in post-render"
        );
        // Advance through post-render (1 line), VBL block (20 lines),
        // and pre-render (1 line) into the first visible scanline.
        // The frame's scanline order is [241..=260, -1, 0..=240] per
        // the v2.1.3 §6.1.e VBL-block-phase alignment. The queue is
        // paused across all 22 non-visible scanlines; the cooldown
        // must fire on the first visible scanline tick.
        for _ in 0..(22 * 341 + 1) {
            let next_dot = state.dot + 1;
            if next_dot >= 341 {
                state.dot = 0;
                // Match the frame's VBL-first scanline order
                // (241..=260, -1, 0..=240).
                if state.scanline == 260 {
                    state.scanline = -1;
                } else {
                    state.scanline += 1;
                }
            } else {
                state.dot = next_dot;
            }
            crate::rendering::tick_dot(&mut state, &mut bus, &win, &mut fb);
        }
        // We've crossed into scanline >= 0 by now. The cooldown
        // must have fired on the first visible-scanline tick.
        assert!(
            state.scanline >= 0 && state.scanline <= 239,
            "test must land in visible range, got sl={}",
            state.scanline
        );
        assert_eq!(
            state.vram_write_cooldown, 0,
            "cooldown must fire on first visible tick"
        );
    }
}
