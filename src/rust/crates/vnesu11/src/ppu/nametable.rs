//! Nametable mirroring — function-pointer dispatch (cold path).
//!
//! Mirroring is set **once per frame** (or per mapper reset) by mapper code
//! via `VNesSoc::set_mirroring(...)`. The active function pointer is then
//! called on **every** PPU address decode, but only for the nametable
//! region — i.e. one call per visible pixel's tile fetch. Hot path uses
//! `inline(always)` on the `Mirroring` enum match in `ppu_read`.
//!
//! Why a function pointer (not `dyn`): the four mirror modes are fixed at
//! compile time. A function pointer gives us monomorphization at the call
//! site without vtable indirection. See `phase_2_bus_and_ram.md` §5.2.
//!
//! Reference: `MIRROR_*` enum in upstream FCEUX + NESdev wiki "Mirroring".
//!
//! # Bit layout of the nametable low-12-bit address
//!
//! ```text
//!   bit 11 (0x800) = top vs bottom row pair
//!   bit 10 (0x400) = left vs right column pair
//!        9..0       = within-nametable offset
//! ```
//!
//! ```text
//!              $2000 (A)   $2400 (B)
//!              $2800 (C)   $2C00 (D)
//! ```
//!
//! The canonical FCEUX mirror assignment (`src/ppu_class.cpp:121-138`):
//!
//! ```cpp
//! case 0:  // MI_H (horizontal)
//!     vnapage_[0] = vnapage_[1] = nt;          // A=B
//!     vnapage_[2] = vnapage_[3] = nt + 0x400;  // C=D
//! case 1:  // MI_V (vertical)
//!     vnapage_[0] = vnapage_[2] = nt;          // A=C
//!     vnapage_[1] = vnapage_[3] = nt + 0x400;  // B=D
//! ```
//!
//! So:
//! - Horizontal: A≡B and C≡D. Within 2 KiB VRAM, "low 1K" holds A/B,
//!   "high 1K" holds C/D. Mask off bit 10 (`0x400`).
//! - Vertical:   A≡C and B≡D. Within 2 KiB VRAM, "low 1K" holds A/C,
//!   "high 1K" holds B/D. Mask off bit 11 (`0x800`).

/// Nametable mirroring mode — fixed set of four canonical NES arrangements
/// plus a "single-screen low/high" pair (rarely used by real games).
///
/// Mirrors the `MIRROR_*` enum used by upstream FCEUX mappers.
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    #[default]
    Horizontal = 0,
    Vertical = 1,
    SingleScreenLow = 2,
    SingleScreenHigh = 3,
    FourScreen = 4,
}

impl Mirroring {
    /// # Safety
    /// `raw` must be one of the five valid values; out-of-range falls
    /// back to `Horizontal`.
    pub unsafe fn from_raw_unchecked(raw: u8) -> Self {
        match raw {
            1 => Self::Vertical,
            2 => Self::SingleScreenLow,
            3 => Self::SingleScreenHigh,
            4 => Self::FourScreen,
            _ => Self::Horizontal,
        }
    }

    /// Compile-time-known function pointer for this mirroring mode.
    /// Used to install the active mirror function on `VNesSoc`.
    #[inline(always)]
    pub const fn mirror_fn(self) -> MirrorFn {
        match self {
            Self::Horizontal => mirror_horizontal,
            Self::Vertical => mirror_vertical,
            Self::SingleScreenLow => mirror_single_low,
            Self::SingleScreenHigh => mirror_single_high,
            Self::FourScreen => mirror_four_screen,
        }
    }
}

/// Function signature for a mirroring routine: maps a 12-bit
/// `$2000-$2FFF` nametable address to a 0..=0x07FF offset into VRAM.
///
/// Cold-path, called at most once per PPU address decode. `#[inline]`
/// keeps it monomorphized at call sites.
pub type MirrorFn = fn(u16) -> u16;

/// Horizontal mirroring: A ($2000) ≡ B ($2400), C ($2800) ≡ D ($2C00).
///
/// Matches FCEUX `MI_H` (`src/ppu_class.cpp:121-129`):
/// ```text
///   vnapage_[0] = vnapage_[1] = nt;          // A and B  → low 1 KiB
///   vnapage_[2] = vnapage_[3] = nt + 0x400;  // C and D  → high 1 KiB
/// ```
///
/// So the page (low vs high 1 KiB) is selected by bit 11 of the
/// address: clear → low, set → high. The within-page offset is the low
/// 10 bits. Combined: `(addr & 0x03FF) | ((addr & 0x0800) >> 1)`.
#[inline]
pub fn mirror_horizontal(addr: u16) -> u16 {
    (addr & 0x03FF) | ((addr & 0x0800) >> 1)
}

/// Vertical mirroring: A ($2000) ≡ C ($2800), B ($2400) ≡ D ($2C00).
///
/// Matches FCEUX `MI_V`:
/// ```text
///   vnapage_[0] = vnapage_[2] = nt;          // A and C  → low 1 KiB
///   vnapage_[1] = vnapage_[3] = nt + 0x400;  // B and D  → high 1 KiB
/// ```
///
/// Page selected by bit 10: clear → low, set → high. Combined:
/// `(addr & 0x03FF) | (addr & 0x0400)`.
#[inline]
pub fn mirror_vertical(addr: u16) -> u16 {
    (addr & 0x03FF) | (addr & 0x0400)
}

/// Single-screen "low" — all four nametables alias to the low 1 KiB.
#[inline]
pub fn mirror_single_low(addr: u16) -> u16 {
    addr & 0x03FF
}

/// Single-screen "high" — all four nametables alias to the high 1 KiB.
#[inline]
pub fn mirror_single_high(addr: u16) -> u16 {
    0x0400 | (addr & 0x03FF)
}

/// Four-screen — each nametable has its own VRAM page (mapper-supplied).
///
/// In vNESU11's internal-only model, fall back to VRAM modulo 4 KiB;
/// real four-screen VRAM is delivered via the mapper's CHR-RAM handler
/// in Phase 5. The mapping keeps within-page offsets intact.
#[inline]
pub fn mirror_four_screen(addr: u16) -> u16 {
    addr & 0x0FFF
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirroring math table — reference values per FCEUX upstream
    /// (`src/ppu_class.cpp:121-138`).
    #[test]
    fn horizontal_mirror_table() {
        // A and B share VRAM (low 1 KiB).
        assert_eq!(mirror_horizontal(0x2000), 0x000);
        assert_eq!(mirror_horizontal(0x2400), 0x000);
        // C and D share VRAM (high 1 KiB).
        assert_eq!(mirror_horizontal(0x2800), 0x400);
        assert_eq!(mirror_horizontal(0x2C00), 0x400);
        // Offsets within a nametable preserve.
        assert_eq!(mirror_horizontal(0x23FF), 0x3FF);
        assert_eq!(mirror_horizontal(0x27FF), 0x3FF);
        assert_eq!(mirror_horizontal(0x2BFF), 0x7FF);
    }

    #[test]
    fn vertical_mirror_table() {
        // A and C share VRAM (low 1 KiB).
        assert_eq!(mirror_vertical(0x2000), 0x000);
        assert_eq!(mirror_vertical(0x2800), 0x000);
        // B and D share VRAM (high 1 KiB).
        assert_eq!(mirror_vertical(0x2400), 0x400);
        assert_eq!(mirror_vertical(0x2C00), 0x400);
    }

    #[test]
    fn single_screen_low() {
        // All four nametables → A's VRAM page (low 1 KiB).
        for a in [0x2000u16, 0x2400, 0x2800, 0x2C00] {
            let v = mirror_single_low(a);
            assert_eq!(v & 0x03FF, a & 0x03FF);
            assert_eq!(v & 0x0400, 0);
        }
    }

    #[test]
    fn single_screen_high() {
        // All four nametables → B's VRAM page (high 1 KiB).
        for a in [0x2000u16, 0x2400, 0x2800, 0x2C00] {
            let v = mirror_single_high(a);
            assert_eq!(v & 0x0400, 0x0400);
            assert_eq!(v & 0x03FF, a & 0x03FF);
        }
    }

    #[test]
    fn four_screen_passes_through() {
        assert_eq!(mirror_four_screen(0x2000), 0x000);
        assert_eq!(mirror_four_screen(0x2400), 0x400);
        assert_eq!(mirror_four_screen(0x2800), 0x800);
        assert_eq!(mirror_four_screen(0x2C00), 0xC00);
    }

    #[test]
    fn mirror_fn_is_correct_per_variant() {
        // Function pointers compare via their `*const ()` representation
        // (avoids the deprecated "fn item as integer" lint).
        let h = Mirroring::Horizontal.mirror_fn() as *const () as usize;
        assert_eq!(h, mirror_horizontal as *const () as usize);
        let v = Mirroring::Vertical.mirror_fn() as *const () as usize;
        assert_eq!(v, mirror_vertical as *const () as usize);
        let sl = Mirroring::SingleScreenLow.mirror_fn() as *const () as usize;
        assert_eq!(sl, mirror_single_low as *const () as usize);
        let sh = Mirroring::SingleScreenHigh.mirror_fn() as *const () as usize;
        assert_eq!(sh, mirror_single_high as *const () as usize);
        let f4 = Mirroring::FourScreen.mirror_fn() as *const () as usize;
        assert_eq!(f4, mirror_four_screen as *const () as usize);
    }
}