//! Filtered PPU A12 watcher + per-dot fetch-address model (v2.1.3 batch 1).
//!
//! MMC3-family mappers clock their IRQ counter on *filtered* PPU A12
//! rising edges. Batch 1 replaces the v2.1.2 per-scanline approximation
//! (one `notify_hblank` per visible scanline at dot 256) with a
//! hardware-shaped model:
//!
//! 1. [`fetch_bus_address`] reconstructs the address the PPU presents on
//!    its bus at each dot of the fetch lines (pre-render + visible),
//!    following the nesdev PPU rendering cycle table. Reporting covers
//!    every fetch (NT / AT / pattern), not just the pattern fetches: the
//!    NT/AT addresses (`$2xxx`) are what pull A12 low and give the filter
//!    its low-period measurements.
//! 2. [`A12Watcher`] applies the cartridge-side A12 filter: a rising edge
//!    only counts after A12 has stayed low for
//!    [`A12_MIN_LOW_DOTS`] PPU dots. Hardware evidence (Furrtek's MMC3C
//!    silicon reverse: A12 must be low across 3 M2 falling edges ≈ 3 CPU
//!    cycles ≈ 9 dots) puts the threshold at 9; blargg's own suite pins
//!    the valid window:
//!    - the sprite fetch window's inter-group garbage NT fetches create
//!      4-dot low periods that must NOT clock the counter (blargg
//!      2.Details subtest 7: exactly 241 clocks per fully rendered frame
//!      in the standard CHR configuration = one per fetch line), and
//!    - blargg's `$2006`-toggle sequences leave A12 low for ~26 CPU
//!      cycles (~78 dots) and MUST clock the counter (tests 1/3).
//!
//! Only bit 12 of the reported address is meaningful, so the model is a
//! pure function of `(scanline, dot, $2000, secondary OAM)`:
//! - NT/AT fetches (BG + the sprite window's garbage fetches) → `$2xxx`
//!   region, A12 = 0;
//! - BG pattern fetches → A12 = `$2000` bit 4;
//! - sprite pattern fetches → A12 = `$2000` bit 3 (8x8), or the sprite's
//!   tile index bit 0 (8x16 — the per-sprite table selection is what lets
//!   one scanline produce several A12 oscillations); empty slots fetch
//!   dummy tile `$FF`, which selects `$1xxx` outright in 8x16 mode.
//!
//! When rendering is off (or during VBlank) the PPU bus carries the live
//! `v` register instead; those transitions are reported from the CPU
//! access path in `ffi.rs` (`$2006` second write, `$2007` read/write
//! increment, `$2001` rendering disable) — blargg test 3 verifies all of
//! them clock the counter whether or not rendering is on.

/// Minimum A12-low period (PPU dots) before a rising edge clocks the
/// counter. 9 dots = 3 M2 falling edges (Furrtek MMC3C silicon reverse,
/// see `docs/knowledge_base/cart/mapper_irq_mechanisms.md` §2.1/§4).
pub const A12_MIN_LOW_DOTS: u64 = 9;

/// Filtered A12 edge detector.
///
/// `tick` is a monotonic PPU-dot counter advanced once per `tick_dot`;
/// CPU-driven address pushes (which happen between dots) observe the
/// counter as-is, which is within one dot of the true transition point —
/// far below the 9-dot filter resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct A12Watcher {
    /// Last A12 level seen (`true` = high, address bit 12 set).
    pub level_high: bool,
    /// Tick at which the current low period began (`None` = A12 has not
    /// been observed low yet — boot state; a rising edge before any low
    /// observation never counts).
    pub low_since_tick: Option<u64>,
    /// Monotonic PPU dot counter.
    pub tick: u64,
}

impl Default for A12Watcher {
    fn default() -> Self {
        Self::new()
    }
}

impl A12Watcher {
    pub const fn new() -> Self {
        Self {
            level_high: false,
            low_since_tick: None,
            tick: 0,
        }
    }

    /// Advance the dot clock by one. Called once per `tick_dot`.
    #[inline]
    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    /// Present a PPU bus address on A12. Returns `true` exactly when the
    /// presentation is a *filtered* rising edge (A12 0→1 after a low
    /// period of at least [`A12_MIN_LOW_DOTS`] dots). Repeated high
    /// presentations never re-clock (one clock per rising edge).
    pub fn observe(&mut self, addr: u16) -> bool {
        if addr & 0x1000 != 0 {
            if self.level_high {
                return false;
            }
            let edge = match self.low_since_tick {
                Some(t0) => self.tick - t0 >= A12_MIN_LOW_DOTS,
                None => false,
            };
            self.level_high = true;
            self.low_since_tick = None;
            edge
        } else {
            if self.level_high {
                self.level_high = false;
            }
            if self.low_since_tick.is_none() {
                self.low_since_tick = Some(self.tick);
            }
            false
        }
    }
}

/// Sprite pattern fetch address for one sprite slot of the current
/// scanline's fetch window (dots 257-320). Slot order and the dummy
/// `$FF` fetch for empty slots follow nesdev sprite evaluation /
/// `ppu_sprite_evaluation.md`.
fn sprite_pattern_addr(
    slot: usize,
    ctrl: u8,
    scanline: i16,
    secondary_oam: &[u8; 32],
    secondary_count: u8,
) -> u16 {
    const DUMMY_TILE: u16 = 0xFF;
    let large = ctrl & (1 << 5) != 0; // $2000 bit 5: 8x16 sprites
    let filled = (slot as u8) < secondary_count;
    let tile: u16 = if filled {
        secondary_oam[slot * 4 + 1] as u16
    } else {
        DUMMY_TILE
    };
    if large {
        // 8x16: the pattern table comes from tile index bit 0 (bit 3 of
        // $2000 is ignored); the second tile of the pair (tile|1) lives
        // at rows 8-15 of the same table.
        let table: u16 = if tile & 1 != 0 { 0x1000 } else { 0x0000 };
        let row: i32 = if filled {
            scanline as i32 - secondary_oam[slot * 4] as i32
        } else {
            0
        };
        let row = row.clamp(0, 15) as u16;
        let row_off = if row >= 8 { row + 8 } else { row };
        table | ((tile & 0xFE) << 4) | row_off
    } else {
        // 8x8: table from $2000 bit 3.
        let table: u16 = if ctrl & (1 << 3) != 0 { 0x1000 } else { 0x0000 };
        let row: i32 = if filled {
            scanline as i32 - secondary_oam[slot * 4] as i32
        } else {
            0
        };
        let row = row.clamp(0, 7) as u16;
        table | (tile << 4) | row
    }
}

/// PPU bus address presented at `(scanline, dot)` while rendering is
/// enabled on a fetch line (pre-render `-1` or visible `0..=239`).
///
/// `None` when the model has no fetch event at this dot (the address
/// presented is the previous fetch's — reporting it again would not
/// change A12). Only bit 12 of the returned address is significant; the
/// low bits are filled in to look like the hardware address for debug
/// value, not for accuracy.
///
/// Cycle table per nesdev PPU rendering:
/// - dot 0 (visible lines only): idle cycle showing the CHR address dot 5
///   will use — the BG pattern table region (Mesen2 does the same
///   `SetBusAddress` at scanline start; the pre-render line keeps `v` on
///   the bus until the first fetch at dot 1).
/// - dots 1-256: 32 tile groups of 8 dots — NT at 8g+1, AT at 8g+3,
///   pattern-low at 8g+6 (presented at 8g+5; the +1 is the M2-edge
///   sighting offset, see the match arm comment; pattern-high at 8g+8
///   shares bit 12 and is skipped).
/// - dots 257-320: 8 sprite groups — garbage NT at 257+8s, garbage
///   second fetch at 259+8s (both `$2xxx`), pattern-low at 262+8s.
/// - dots 321-336: the two next-line preload groups, same shape as
///   dots 1-256.
/// - dots 337 / 339: the two garbage NT fetches.
pub fn fetch_bus_address(
    scanline: i16,
    dot: u16,
    ctrl: u8,
    secondary_oam: &[u8; 32],
    secondary_count: u8,
) -> Option<u16> {
    // NT/AT fetches (and the sprite window's garbage fetches) always
    // address the nametable region: bit 12 clear, bit 13 set.
    const NT_REGION: u16 = 0x2000;
    let bg_table: u16 = if ctrl & (1 << 4) != 0 { 0x1000 } else { 0x0000 };

    if !(0..341).contains(&dot) {
        return None;
    }
    if dot == 0 {
        // Idle cycle: visible lines present the upcoming BG pattern
        // address (pre-render keeps `v` — handled by the CPU path).
        return if (0..=239).contains(&scanline) {
            Some(bg_table)
        } else {
            None
        };
    }
    // Sighting offset: the MMC3 samples A12 against M2 (CPU clock)
    // edges and its analog chain lands the effective counter clock
    // slightly after the PPU-domain presentation. blargg's suite pins
    // the offset to ~1 PPU dot (4-scanline_timing; see the plan's
    // calibration notes — the residual sub-dot gap is a per-cycle-CPU
    // (batch 3) concern).
    let shift: u16 = 1;
    if (1..=256).contains(&dot) {
        let phase = (dot - 1) & 0x7;
        return match phase {
            0 => Some(NT_REGION),     // nametable fetch
            2 => Some(NT_REGION | 0x03C0), // attribute fetch
            // Pattern low: presented at phase 4; the reported sighting is
            // shifted by `sighting_shift()` (M2-edge sampling + analog
            // chain delay, calibrated against blargg 4-scanline_timing).
            p if p == 4 + shift as u16 => Some(bg_table),
            _ => None,
        };
    }
    if (257..=320).contains(&dot) {
        let slot = ((dot - 257) >> 3) as usize;
        let phase = (dot - 257) & 0x7;
        return match phase {
            0 => Some(NT_REGION),        // garbage nametable fetch
            2 => Some(NT_REGION | 0x03C0), // garbage second fetch
            p if p == 4 + shift as u16 => Some(sprite_pattern_addr(
                slot,
                ctrl,
                scanline,
                secondary_oam,
                secondary_count,
            )),
            _ => None,
        };
    }
    if (321..=336).contains(&dot) {
        let phase = (dot - 321) & 0x7;
        return match phase {
            0 => Some(NT_REGION),
            2 => Some(NT_REGION | 0x03C0),
            p if p == 4 + shift as u16 => Some(bg_table),
            _ => None,
        };
    }
    if dot == 337 || dot == 339 {
        return Some(NT_REGION); // two garbage NT fetches
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watcher_ignores_rise_without_observed_low() {
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x1000), "no low period observed yet");
    }

    #[test]
    fn watcher_requires_min_low_dots() {
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x0000), "fall");
        // 8 dots of low: below the filter (inverted-config dot-5 case).
        for _ in 0..8 {
            w.advance_tick();
        }
        assert!(!w.observe(0x1000), "8-dot low must not clock");
        // A long low period passes.
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x0000));
        for _ in 0..A12_MIN_LOW_DOTS {
            w.advance_tick();
        }
        assert!(w.observe(0x1000), "9-dot low clocks the counter");
    }

    #[test]
    fn watcher_ignores_repeated_high() {
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x0000));
        for _ in 0..A12_MIN_LOW_DOTS {
            w.advance_tick();
        }
        assert!(w.observe(0x1000), "long enough low → edge");
        w.advance_tick();
        assert!(!w.observe(0x1FFF), "A12 staying high must not re-clock");
    }

    #[test]
    fn watcher_ignores_fall_to_fall() {
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x0000));
        w.advance_tick();
        assert!(!w.observe(0x0FFF), "1→0 transition never clocks");
    }

    #[test]
    fn fetch_model_standard_config_one_rise_per_line() {
        // Standard config: $2000 = $08 (sprites $1000, BG $0000), 8x8,
        // empty secondary OAM → all 8 slots fetch dummy tile $FF from
        // $1xxx. Exactly ONE filtered rising edge per fetch line, at the
        // first sprite pattern fetch (dot 261) — the blargg 2.Details
        // 241-clocks-per-frame anchor.
        let ctrl: u8 = 0x08;
        let oam = [0u8; 32];
        for sl in [-1, 0, 137, 239] {
            let mut rises = 0usize;
            let mut w = A12Watcher::new();
            // Prime with the previous line's last sprite fetch (high),
            // then fall at 321 — mirrors the steady-state frame.
            assert!(!w.observe(0x1000));
            for dot in 0..341u16 {
                w.advance_tick();
                if let Some(addr) = fetch_bus_address(sl, dot, ctrl, &oam, 0) {
                    if w.observe(addr) {
                        rises += 1;
                        assert_eq!(dot, 262, "single rise lands at dot 262");
                    }
                }
            }
            assert_eq!(rises, 1, "scanline {sl}: exactly one rise per fetch line");
        }
    }

    #[test]
    fn fetch_model_bg_at_1000_counts_at_preload_window() {
        // Inverted config: BG $1000 ($2000 = $10), sprites $0000. The
        // BG pattern fetches raise A12 with only 4-dot low periods
        // (rejected by the filter); the single count lands at the first
        // preload pattern fetch (dot 325) after the 68-dot sprite-window
        // low — the KB's "反配置计数点 dot 324 附近".
        let ctrl: u8 = 0x10;
        let oam = [0u8; 32];
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x0000)); // steady state: bus low
        let mut rises = Vec::new();
        for dot in 0..341u16 {
            w.advance_tick();
            if let Some(addr) = fetch_bus_address(-1, dot, ctrl, &oam, 0) {
                if w.observe(addr) {
                    rises.push(dot);
                }
            }
        }
        assert_eq!(rises, vec![326], "inverted config counts at dot 326 only");
    }

    #[test]
    fn fetch_model_8x16_per_slot_table_bit() {
        // 8x16 sprites ($2000 = $20): slots take the table from tile bit
        // 0. With slots [tile $02 (table 0), tile $01 (table 1), rest
        // empty (dummy $FF → table 1)] the sprite window is: slot 0
        // low, slots 1-7 high. The rises land at the first high slot's
        // pattern fetch (dot 269) and — because slot 0 pulled A12 low
        // for only 4 dots before it — nowhere else.
        let ctrl: u8 = 0x20;
        let mut oam = [0u8; 32];
        // Two in-range sprites for scanline 50: Y=50, tiles $02 / $01.
        oam[0..4].copy_from_slice(&[50, 0x02, 0x00, 10]);
        oam[4..8].copy_from_slice(&[50, 0x01, 0x00, 20]);
        let mut w = A12Watcher::new();
        assert!(!w.observe(0x0000)); // bus low entering the line
        let mut rises = Vec::new();
        for dot in 0..341u16 {
            w.advance_tick();
            if let Some(addr) = fetch_bus_address(50, dot, ctrl, &oam, 2) {
                if w.observe(addr) {
                    rises.push(dot);
                }
            }
        }
        assert_eq!(rises, vec![270], "first $1xxx sprite slot clocks at dot 270");
    }

    #[test]
    fn fetch_model_dots_without_fetch_are_none() {
        let oam = [0u8; 32];
        for dot in [0u16, 2, 5, 8, 258, 261, 330, 338, 340] {
            let addr = fetch_bus_address(0, dot, 0x08, &oam, 0);
            if dot == 0 {
                assert!(addr.is_some());
            } else {
                assert!(addr.is_none(), "dot {dot} has no fetch event");
            }
        }
    }

    #[test]
    fn fetch_model_all_reported_addresses_respect_14bit() {
        // The PPU address bus is 14 bits ($0000-$3FFF).
        let mut oam = [0u8; 32];
        for i in 0..8 {
            oam[i * 4..i * 4 + 4].copy_from_slice(&[10, 0xFF, 0xFF, 0xFF]);
        }
        for dot in 0..341u16 {
            if let Some(addr) = fetch_bus_address(50, dot, 0x38, &oam, 8) {
                assert!(addr <= 0x3FFF, "dot {dot} address {addr:#06x} over 14 bits");
            }
        }
    }
}
