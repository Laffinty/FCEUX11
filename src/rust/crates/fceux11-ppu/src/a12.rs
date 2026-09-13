//! Filtered PPU A12 watcher (v2.1.3 batch 1; batch 2 moved the address
//! source into the rendering pipeline).
//!
//! MMC3-family mappers clock their IRQ counter on *filtered* PPU A12
//! rising edges. Batch 1 replaced the v2.1.2 per-scanline approximation
//! (one `notify_hblank` per visible scanline at dot 256) with a
//! hardware-shaped model:
//!
//! 1. Batch 1 reconstructed the offered address with a pure function of
//!    `(scanline, dot, $2000, secondary OAM)`. Batch 2 replaced that
//!    approximation with the real fetch state machine
//!    ([`crate::rendering::tick_dot`]), which reports the address each
//!    fetch actually places on the bus — including mid-scanline
//!    `$2000`/`$2005`/`$2006` changes that re-point the fetches.
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
//! Only bit 12 of the reported address is meaningful, so the pipeline's
//! reporting reduces to:
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
/// `tick` is a monotonic PPU-dot counter advanced once per
/// [`crate::frame::tick_dot`]. The rendering pipeline observes the
/// address for the dot it is processing *before* that tick advance, which
/// shortens every recorded timestamp by one dot — low-period *durations*
/// (what the filter measures) are unaffected.
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
pub(crate) fn sprite_pattern_addr(
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

    /// Standard CHR configuration ($2000 bit 3 set): 8x8 sprites take the
    /// $1000 table whatever the tile index, and an empty slot fetches the
    /// dummy tile — which is still `$1xxx`. One rise per slot, and the
    /// rise moves to $0000 when the slot is filled with a table-0 sprite.
    #[test]
    fn sprite_pattern_addr_8x8_uses_ctrl_bit3() {
        let oam = [0u8; 32];
        assert_eq!(sprite_pattern_addr(0, 0x08, 10, &oam, 0) & 0x1000, 0x1000);
        assert_eq!(sprite_pattern_addr(0, 0x00, 10, &oam, 0) & 0x1000, 0x0000);
    }

    /// 8x16: the table comes from the tile index bit 0, so two sprites on
    /// one scanline can pull A12 in opposite directions — the multiple
    /// edges per line blargg's `A12_clocking` case exercises.
    #[test]
    fn sprite_pattern_addr_8x16_uses_tile_bit0() {
        let mut oam = [0u8; 32];
        oam[0..4].copy_from_slice(&[50, 0x02, 0x00, 10]); // tile bit 0 = 0
        oam[4..8].copy_from_slice(&[50, 0x01, 0x00, 20]); // tile bit 0 = 1
        assert_eq!(sprite_pattern_addr(0, 0x20, 50, &oam, 2) & 0x1000, 0x0000);
        assert_eq!(sprite_pattern_addr(1, 0x20, 50, &oam, 2) & 0x1000, 0x1000);
        // Empty slots fetch the dummy $FF tile → always table 1.
        assert_eq!(sprite_pattern_addr(2, 0x20, 50, &oam, 2) & 0x1000, 0x1000);
    }

    #[test]
    fn sprite_pattern_addr_stays_within_14_bits() {
        let mut oam = [0u8; 32];
        for i in 0..8 {
            oam[i * 4..i * 4 + 4].copy_from_slice(&[10, 0xFF, 0xFF, 0xFF]);
        }
        for slot in 0..8 {
            assert!(sprite_pattern_addr(slot, 0x38, 50, &oam, 8) <= 0x3FFF);
            assert!(sprite_pattern_addr(slot, 0x20, 50, &oam, 3) <= 0x3FFF);
        }
    }
}
