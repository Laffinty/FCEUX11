//! Tile fetcher — templated fetch helpers.
//!
//! Background and sprite tile fetches share most of their logic; the
//! difference is in pattern table base, attribute handling, and pixel
//! ordering (sprites have horizontal/vertical flip).  We model these
//! as a `TileFlags` trait so the compiler instantiates a separate
//! function per (BG / sprite / flip-h / flip-v) combination, giving us
//! zero-overhead dispatch.
//!
//! Reference: `src/pputile_template.cpp` (template-based tile fetcher).
//!
//! Phase 3 ships the scaffolding + per-combination entry points.  The
//! actual pixel composition is delegated to `background.rs::BackgroundRenderer`
//! and `sprite.rs::SpriteState`.

use crate::ppu::registers::SpriteSize;

/// Tile fetcher flags. Each unique `TileFlags` impl produces a
/// monomorphized version of the generic functions below.
pub trait TileFlags {
    /// Sprite fetcher (true) or background fetcher (false).
    const IS_SPRITE: bool;
    /// Horizontal flip enabled.
    const FLIP_HORIZONTAL: bool;
    /// Vertical flip enabled.
    const FLIP_VERTICAL: bool;
    /// Sprite size (ignored for background).
    const SPRITE_SIZE: SpriteSize;
}

/// Background tile flags.
pub struct BackgroundTile;
impl TileFlags for BackgroundTile {
    const IS_SPRITE: bool = false;
    const FLIP_HORIZONTAL: bool = false;
    const FLIP_VERTICAL: bool = false;
    const SPRITE_SIZE: SpriteSize = SpriteSize::Size8x8;
}

/// 8x8 sprite, no flip.
pub struct SpriteNormalTile;
impl TileFlags for SpriteNormalTile {
    const IS_SPRITE: bool = true;
    const FLIP_HORIZONTAL: bool = false;
    const FLIP_VERTICAL: bool = false;
    const SPRITE_SIZE: SpriteSize = SpriteSize::Size8x8;
}

/// 8x8 sprite, horizontal flip.
pub struct SpriteFlipHTile;
impl TileFlags for SpriteFlipHTile {
    const IS_SPRITE: bool = true;
    const FLIP_HORIZONTAL: bool = true;
    const FLIP_VERTICAL: bool = false;
    const SPRITE_SIZE: SpriteSize = SpriteSize::Size8x8;
}

/// 8x8 sprite, vertical flip.
pub struct SpriteFlipVTile;
impl TileFlags for SpriteFlipVTile {
    const IS_SPRITE: bool = true;
    const FLIP_HORIZONTAL: bool = false;
    const FLIP_VERTICAL: bool = true;
    const SPRITE_SIZE: SpriteSize = SpriteSize::Size8x8;
}

/// 8x8 sprite, both flips.
pub struct SpriteFlipHVTile;
impl TileFlags for SpriteFlipHVTile {
    const IS_SPRITE: bool = true;
    const FLIP_HORIZONTAL: bool = true;
    const FLIP_VERTICAL: bool = true;
    const SPRITE_SIZE: SpriteSize = SpriteSize::Size8x8;
}

/// 8x16 sprite, no flip.
pub struct Sprite8x16Tile;
impl TileFlags for Sprite8x16Tile {
    const IS_SPRITE: bool = true;
    const FLIP_HORIZONTAL: bool = false;
    const FLIP_VERTICAL: bool = false;
    const SPRITE_SIZE: SpriteSize = SpriteSize::Size8x16;
}

/// Compute the row-in-tile index for a sprite, accounting for vertical
/// flip.
#[inline(always)]
pub fn sprite_row_in_tile<F: TileFlags>(
    scanline_in_sprite: u8,
    sprite_height: u8,
) -> u8 {
    let mut row = scanline_in_sprite & 0x07;
    if F::FLIP_VERTICAL {
        row = 7 - row;
    }
    let _ = sprite_height;
    row
}

/// Compute the bit index within a pattern byte for a sprite, accounting
/// for horizontal flip.
#[inline(always)]
pub fn sprite_bit_index<F: TileFlags>(within_tile_x: u8) -> u8 {
    if F::FLIP_HORIZONTAL {
        within_tile_x & 0x07
    } else {
        7 - (within_tile_x & 0x07)
    }
}

/// Fetch the pattern low/high bytes for a sprite tile.
#[inline(always)]
pub fn fetch_sprite_pattern<F: TileFlags>(
    pattern_lo: &[u8],
    pattern_hi: &[u8],
    tile_id: u8,
    row_in_tile: u8,
) -> (u8, u8) {
    let offset = (tile_id as usize) * 16 + (row_in_tile as usize);
    let lo = if offset < pattern_lo.len() {
        pattern_lo[offset]
    } else {
        0
    };
    let hi = if offset < pattern_hi.len() {
        pattern_hi[offset]
    } else {
        0
    };
    let _ = std::marker::PhantomData::<F>;
    (lo, hi)
}

/// 8x16 sprite tile + bank selection.
#[inline]
pub fn sprite_8x16_tile_bank(
    tile_id: u8,
    row_in_sprite: u8,
) -> (u8, u16) {
    let bank = (tile_id & 0x01) as u16 * 0x1000;
    let tile = if (row_in_sprite & 0x08) != 0 {
        (tile_id & 0xFE) + 1
    } else {
        tile_id & 0xFE
    };
    (tile, bank)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_row_no_flip() {
        assert_eq!(sprite_row_in_tile::<SpriteNormalTile>(0, 8), 0);
        assert_eq!(sprite_row_in_tile::<SpriteNormalTile>(7, 8), 7);
    }

    #[test]
    fn sprite_row_vflip() {
        assert_eq!(sprite_row_in_tile::<SpriteFlipVTile>(0, 8), 7);
        assert_eq!(sprite_row_in_tile::<SpriteFlipVTile>(7, 8), 0);
    }

    #[test]
    fn sprite_bit_no_flip() {
        // Without H-flip, bit 7 is leftmost pixel.
        assert_eq!(sprite_bit_index::<SpriteNormalTile>(0), 7);
        assert_eq!(sprite_bit_index::<SpriteNormalTile>(7), 0);
    }

    #[test]
    fn sprite_bit_hflip() {
        // With H-flip, bit 0 is leftmost.
        assert_eq!(sprite_bit_index::<SpriteFlipHTile>(0), 0);
        assert_eq!(sprite_bit_index::<SpriteFlipHTile>(7), 7);
    }

    #[test]
    fn sprite_8x16_tile_selection() {
        // Tile 0xAA = 1010_1010 → bit 0 = 0 → bank = $0000
        let (t, b) = sprite_8x16_tile_bank(0xAA, 0);
        assert_eq!(t, 0xAA);
        assert_eq!(b, 0x0000);
        let (t, b) = sprite_8x16_tile_bank(0xAA, 8);
        assert_eq!(t, 0xAB); // bottom half: +1
        assert_eq!(b, 0x0000);

        // Tile 0xAB = 1010_1011 → bit 0 = 1 → bank = $1000
        let (t, b) = sprite_8x16_tile_bank(0xAB, 0);
        assert_eq!(t, 0xAA); // (0xAB & 0xFE) = 0xAA
        assert_eq!(b, 0x1000);
        let (t, b) = sprite_8x16_tile_bank(0xAB, 8);
        assert_eq!(t, 0xAB); // bottom half: 0xAA + 1 = 0xAB
        assert_eq!(b, 0x1000);
    }
}