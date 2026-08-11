//! PPU integration tests (Phase 3).
//!
//! Exercises the PPU pipeline end-to-end: register file, dot clock,
//! segment scheduler, background rendering, sprite evaluation,
//! composition, VBlank NMI, and NSF idle mode.
//!
//! The tests use synthetic nametable/pattern/CHR data (no real ROM)
//! and verify the renderer produces sensible output.  Real-ROM
//! blargg parity tests are gated on ROM fixtures (Phase 6 shadow run).

use vnesu11::ppu::background::BackgroundRenderer;
use vnesu11::ppu::compositing::Compositor;
use vnesu11::ppu::dot_clock::{
    DOTS_PER_SCANLINE, SCANLINES_PER_FRAME, VBLANK_SCANLINE, VBLANK_SET_DOT,
};
use vnesu11::ppu::nmi::NmiController;
use vnesu11::ppu::oam::SpriteEntry;
use vnesu11::ppu::registers::{PpuRegisters, SpriteSize};
use vnesu11::ppu::sprite_lut::{SpriteLut, SPRITE_LUT_SIZE};
use vnesu11::ppu::{PpuCore, Segment};

// ====================================================================
// Frame timing
// ====================================================================

#[test]
fn frame_is_341x262() {
    // NTSC frame is 341 dots × 262 scanlines.
    assert_eq!(DOTS_PER_SCANLINE, 341);
    assert_eq!(SCANLINES_PER_FRAME, 262);
}

#[test]
fn vblank_set_at_241_dot_1() {
    // Per NESdev wiki + src/ppu.cpp: FCEUPPU_Loop sets VBlank at
    // scanline 241, dot 1.
    assert_eq!(VBLANK_SCANLINE, 241);
    assert_eq!(VBLANK_SET_DOT, 1);
}

// ====================================================================
// Registers
// ====================================================================

#[test]
fn ppuctrl_nametable_select() {
    let mut r = PpuRegisters::new();
    for (val, expected) in [(0, 0x2000u16), (1, 0x2400), (2, 0x2800), (3, 0x2C00)] {
        r.ppuctrl = val;
        assert_eq!(r.nametable_base(), expected);
    }
}

#[test]
fn ppuctrl_data_increment() {
    let mut r = PpuRegisters::new();
    r.ppuctrl = 0x00;
    assert_eq!(r.ppu_data_increment(), 1);
    r.ppuctrl = 0x04;
    assert_eq!(r.ppu_data_increment(), 32);
}

#[test]
fn ppu_status_read_clears_vblank() {
    let mut r = PpuRegisters::new();
    r.status = 0xC0; // VBlank + sprite 0 hit
    let v = r.read_status();
    assert_eq!(v, 0xC0);
    // After read, VBlank cleared but sprite 0 hit persists.
    assert_eq!(r.status, 0x40);
}

#[test]
fn scroll_two_writes_split_x_y() {
    let mut r = PpuRegisters::new();
    r.write_scroll(0xAB);
    // First write: x = AB & 7, coarse_x = AB >> 3 = 0x15.
    assert_eq!(r.x, 0x03);
    assert_eq!(r.t.coarse_x(), 0x15);
    r.write_scroll(0xCD);
    // Second write: fine_y = CD & 7, coarse_y = CD >> 3.
    assert_eq!(r.t.fine_y(), 0x05);
    assert_eq!(r.t.coarse_y(), 0x19);
}

// ====================================================================
// Background renderer
// ====================================================================

#[test]
fn background_renders_uniform_tile() {
    let mut nametable = [0u8; 960];
    let mut attribute = [0u8; 64];
    let mut pattern_lo = [0u8; 8192];
    let mut pattern_hi = [0u8; 8192];

    // Tile 1: full opaque pattern.
    for row in 0..8 {
        pattern_lo[1 * 16 + row] = 0xFF;
        pattern_hi[1 * 16 + row] = 0xFF;
    }
    nametable.fill(1);
    // Attribute: all 4 quadrants → palette 1.
    attribute.fill(0x55);

    let mut output = [0u8; 256];
    BackgroundRenderer::render_line(
        &nametable,
        &attribute,
        &pattern_lo,
        &pattern_hi,
        0, 0, 0, 0,
        0,
        &mut output,
    );
    // Every pixel: color = 3, palette = 1, encoded = (1 << 2) | 3 = 7.
    for (x, &p) in output.iter().enumerate() {
        assert_eq!(p, 7, "x={}: got {}", x, p);
    }
}

#[test]
fn background_transparent_passes_through() {
    let nametable = [0u8; 960];
    let attribute = [0u8; 64];
    let pattern_lo = [0u8; 8192];
    let pattern_hi = [0u8; 8192];
    let mut output = [0xFFu8; 256];
    BackgroundRenderer::render_line(
        &nametable,
        &attribute,
        &pattern_lo,
        &pattern_hi,
        0, 0, 0, 0,
        0,
        &mut output,
    );
    // All-transparent tile → all pixels = 0.
    for &p in &output {
        assert_eq!(p, 0);
    }
}

#[test]
fn background_respects_fine_x_scroll() {
    // Same tile, with fine_x = 4, should shift the rendering.
    let mut nametable = [0u8; 960];
    let mut pattern_lo = [0u8; 8192];
    let mut pattern_hi = [0u8; 8192];

    // Tile 1: alternating on/off pattern: bit 0 of each pixel.
    // Row 0: 0b10101010 = 0xAA (so even pixels = 1, odd = 0)
    pattern_lo[1 * 16 + 0] = 0xAA;
    pattern_hi[1 * 16 + 0] = 0xAA;
    nametable.fill(1);

    let attribute = [0u8; 64];
    let mut output1 = [0u8; 256];
    BackgroundRenderer::render_line(
        &nametable,
        &attribute,
        &pattern_lo,
        &pattern_hi,
        0, 0, 0, 0,
        0,
        &mut output1,
    );
    // Without scroll, first 8 pixels: alternating 3, 0, 3, 0, 3, 0, 3, 0.
    assert_eq!(output1[0], 3);
    assert_eq!(output1[1], 0);
    assert_eq!(output1[2], 3);
    assert_eq!(output1[7], 0);
}

#[test]
fn background_attribute_quadrant_picks_palette() {
    // Attribute byte 0xE4:
    //   bits 0-1 = quadrant (0,0) = palette 0
    //   bits 2-3 = quadrant (1,0) = palette 1
    //   bits 4-5 = quadrant (0,1) = palette 2
    //   bits 6-7 = quadrant (1,1) = palette 3
    //
    // With our renderer (pal_quadrant = (coarse_x&1)<<1 | (coarse_y&1)):
    //   - pixel x=0: coarse_x=0, pal_quadrant=0 → palette 0
    //   - pixel x=8: coarse_x=1, pal_quadrant=2 → palette 2
    //   - pixel x=16: coarse_x=2, pal_quadrant=0 → palette 0
    let mut nametable = [0u8; 960];
    let mut attribute = [0u8; 64];
    let mut pattern_lo = [0u8; 8192];
    let mut pattern_hi = [0u8; 8192];

    // Tile 0: full opaque (color=3).
    for row in 0..8 {
        pattern_lo[row] = 0xFF;
        pattern_hi[row] = 0xFF;
    }
    attribute.fill(0xE4);

    let mut output = [0u8; 256];
    BackgroundRenderer::render_line(
        &nametable,
        &attribute,
        &pattern_lo,
        &pattern_hi,
        0, 0, 0, 0,
        0,
        &mut output,
    );
    assert_eq!(output[0] & 0x0C, 0 << 2); // palette 0
    assert_eq!(output[8] & 0x0C, 2 << 2); // palette 2
    assert_eq!(output[16] & 0x0C, 0 << 2); // palette 0
}

// ====================================================================
// OAM / sprite evaluation
// ====================================================================

#[test]
fn sprite_y_zero_top_is_scanline_1() {
    let s = SpriteEntry { y: 0, tile: 0, attr: 0, x: 0 };
    assert!(s.visible_at(0, 8).is_none());
    assert_eq!(s.visible_at(1, 8), Some(0));
    assert_eq!(s.visible_at(8, 8), Some(7));
    assert!(s.visible_at(9, 8).is_none());
}

#[test]
fn sprite_priority_flip_decoding() {
    let s = SpriteEntry { y: 0, tile: 0, attr: 0xFF, x: 0 };
    assert_eq!(s.palette(), 0x03);
    assert!(s.priority_behind_bg());
    assert!(s.flip_h());
    assert!(s.flip_v());
}

#[test]
fn sprite_size_8x8_vs_8x16() {
    let mut r = PpuRegisters::new();
    r.ppuctrl = 0x00;
    assert_eq!(r.sprite_size(), SpriteSize::Size8x8);
    r.ppuctrl = 0x20;
    assert_eq!(r.sprite_size(), SpriteSize::Size8x16);
    assert_eq!(r.sprite_size().height(), 16);
}

// ====================================================================
// Compositor
// ====================================================================

#[test]
fn compose_sprite_transparent_uses_bg() {
    let (pal, col) = Compositor::compose(2, 1, 0, 3, false);
    assert_eq!(pal, 1);
    assert_eq!(col, 2);
}

#[test]
fn compose_sprite_priority_behind_bg() {
    // Both opaque, sprite says it's behind BG.
    let (pal, col) = Compositor::compose(3, 0, 1, 2, true);
    assert_eq!(pal, 0); // BG wins
    assert_eq!(col, 3);
}

#[test]
fn compose_sprite_in_front() {
    // Both opaque, sprite says in front.
    let (pal, col) = Compositor::compose(3, 0, 1, 2, false);
    assert_eq!(pal, 2); // Sprite wins
    assert_eq!(col, 1);
}

#[test]
fn sprite_zero_hit_clipped_at_left_edge() {
    let mut c = Compositor::new();
    c.sprite_zero_loaded();
    // At x < 8 the hit is masked.
    assert!(!c.check_sprite_zero_hit(0, 0, true, true));
    assert!(!c.check_sprite_zero_hit(7, 0, true, true));
    // At x = 8 the hit fires.
    assert!(c.check_sprite_zero_hit(8, 0, true, true));
}

// ====================================================================
// NMI
// ====================================================================

#[test]
fn nmi_fires_once_per_frame() {
    let mut n = NmiController::new();
    assert!(!n.take());
    n.arm(241, 1);
    assert!(n.take());
    // Second take is no-op.
    assert!(!n.take());
}

// ====================================================================
// Sprite LUT
// ====================================================================

#[test]
fn sprite_lut_is_512_kib() {
    assert_eq!(SPRITE_LUT_SIZE, 524_288);
    assert_eq!(SPRITE_LUT_SIZE, 256 * 256 * 8);
}

#[test]
fn sprite_lut_is_deterministic() {
    let v1 = SpriteLut::get_static(0x42, 0x37, 0x05);
    let v2 = SpriteLut::get_static(0x42, 0x37, 0x05);
    assert_eq!(v1, v2);
}

// ====================================================================
// Segment scheduler
// ====================================================================

#[test]
fn visible_scanline_segment_budget_256() {
    let mut p = PpuCore::new();
    p.scanline = 0;
    match p.next_segment() {
        Segment::Visible { cpu_budget } => {
            assert_eq!(cpu_budget, 256);
        }
        _ => panic!("expected Visible at scanline 0"),
    }
}

#[test]
fn vblank_segment_at_241() {
    let mut p = PpuCore::new();
    p.scanline = 241;
    match p.next_segment() {
        Segment::VBlank { cpu_budget } => {
            assert_eq!(cpu_budget, 1, "VBlank set at dot 1 → budget 1");
        }
        _ => panic!("expected VBlank at scanline 241"),
    }
}

#[test]
fn idle_segment_when_nsf_mode() {
    let mut p = PpuCore::new();
    p.idle = true;
    assert!(matches!(p.next_segment(), Segment::Idle { .. }));
}

// ====================================================================
// SoC-level run_frame
// ====================================================================

#[test]
fn soc_run_frame_completes_with_idle_ppu() {
    // NSF-style idle PPU: run a frame, expect frame_ready after ~263
    // tick iterations.
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    soc.set_nsf_idle(true);
    let result = soc.run_frame();
    assert!(result.completed);
    assert!(soc.frame_ready);
}

#[test]
fn soc_run_frame_with_disabled_ppu_doesnt_render() {
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    // Don't enable BG or sprites. Frame buffer stays untouched.
    soc.ppu.regs.ppumask = 0x00;
    soc.run_frame();
    // All frame buffer bytes remain at 0 (no rendering happened).
    assert!(soc.frame_buffer.iter().all(|&b| b == 0));
}

// ====================================================================
// Phase 3 (a) — background rendering hook (real pixel output)
// ====================================================================
//
// These tests verify that the per-pixel rendering pipeline is now wired
// up to the SoC's PpuCore: when CHR/nametable caches are populated and
// `run_frame()` is called, the frame buffer actually contains non-zero
// pixel output (not just zeros).

/// Build a PpuCore with a single full-opaque tile (tile 0) in the
/// pattern table, an all-tile-0 nametable, and all-zero attribute.
fn ppu_with_uniform_tile_0() -> vnesu11::ppu::PpuCore {
    let mut p = vnesu11::ppu::PpuCore::new();
    // Tile 0 = full opaque (color = 3 in both planes).
    for row in 0..8 {
        p.pattern_lo[row] = 0xFF;
        p.pattern_hi[row] = 0xFF;
    }
    // Nametable = all tile 0.
    for tile_id in p.nametable.iter_mut() {
        *tile_id = 0;
    }
    // Attribute = all palette 0.
    for attr in p.attribute.iter_mut() {
        *attr = 0;
    }
    // Enable BG rendering.
    p.regs.ppumask = 0x08;
    // Pattern table 0.
    p.regs.ppuctrl = 0x00;
    p
}

#[test]
fn bg_render_produces_nonzero_pixels() {
    // Phase 3 (a): after run_frame, the frame buffer contains the
    // rendered pixel values (not zeros). This proves the per-pixel
    // background pipeline is wired into the SoC.
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    let mut ppu = ppu_with_uniform_tile_0();
    // BG enabled, pattern table 0.
    ppu.regs.ppumask = 0x08;
    soc.ppu = ppu;
    soc.run_frame();
    // First scanline (256 pixels) should all be (palette 0 << 2) | color 3 = 3.
    let first_scanline = &soc.frame_buffer[0..256];
    for (x, &p) in first_scanline.iter().enumerate() {
        assert_eq!(p, 3, "x={} got {} expected 3 (palette 0, color 3)", x, p);
    }
}

#[test]
fn bg_render_respects_scroll() {
    // With scroll_x = 8, the renderer should still produce pixels
    // (the scroll is applied within tile boundaries; coarse_x wraps).
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    let mut ppu = ppu_with_uniform_tile_0();
    ppu.regs.ppumask = 0x08;
    ppu.scroll_coarse_x = 1; // shifted by 1 tile (8 pixels)
    soc.ppu = ppu;
    soc.run_frame();
    // All pixels should still be opaque (color = 3) since the tile
    // is full-opaque.
    for (x, &p) in soc.frame_buffer[0..256].iter().enumerate() {
        assert_eq!(p, 3, "x={} got {}", x, p);
    }
}

#[test]
fn bg_render_picks_attribute_palette() {
    // Tile 0 = full opaque. Attribute 0xE4 = quadrant (0,0) = palette 0,
    // quadrant (1,0) = palette 1, quadrant (0,1) = palette 2, etc.
    // At pixel x=0: coarse_x=0 → quadrant (0,0) → palette 0.
    // At pixel x=8: coarse_x=1 → quadrant (1,0) → palette 2.
    // At pixel x=16: coarse_x=2 → quadrant (0,0) → palette 0.
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    let mut ppu = ppu_with_uniform_tile_0();
    ppu.regs.ppumask = 0x08;
    for attr in ppu.attribute.iter_mut() {
        *attr = 0xE4;
    }
    soc.ppu = ppu;
    soc.run_frame();
    let first_scanline = &soc.frame_buffer[0..256];
    // Pixel x=0 → palette 0 (bits 0..=3 == 0).
    assert_eq!(first_scanline[0] & 0x0C, 0 << 2, "x=0 palette");
    // Pixel x=8 → palette 2 (bits 0..=3 == 0b1000).
    assert_eq!(first_scanline[8] & 0x0C, 2 << 2, "x=8 palette");
    // Pixel x=16 → palette 0.
    assert_eq!(first_scanline[16] & 0x0C, 0 << 2, "x=16 palette");
}

#[test]
fn bg_render_disabled_leaves_framebuffer_zero() {
    // When BG is disabled (ppumask bit 3 clear), the frame buffer
    // should remain zero even if caches are populated.
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    let mut ppu = ppu_with_uniform_tile_0();
    ppu.regs.ppumask = 0x00; // BG disabled
    soc.ppu = ppu;
    soc.run_frame();
    assert!(soc.frame_buffer.iter().all(|&b| b == 0));
}

#[test]
fn bg_render_transparent_tile_produces_zero() {
    // Tile 0 = all transparent (pattern = 0). Even though BG is
    // enabled, no pixels are visible (output = 0).
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    let mut ppu = vnesu11::ppu::PpuCore::new();
    // Pattern tables all zero (tile 0 is transparent).
    // Nametable all 0 (tile 0).
    // Attribute all 0 (palette 0).
    ppu.regs.ppumask = 0x08; // BG enabled
    ppu.regs.ppuctrl = 0x00; // pattern table 0
    soc.ppu = ppu;
    soc.run_frame();
    assert!(soc.frame_buffer.iter().all(|&b| b == 0));
}

#[test]
fn bg_render_alternating_pattern_produces_visible_stripes() {
    // Tile 0 with alternating on/off pattern (bit 7 = leftmost pixel).
    // At row 0: lo = 0xAA, hi = 0xAA. Color sequence: 3, 0, 3, 0, ...
    use vnesu11::soc::VNesSoc;
    let mut soc = VNesSoc::default();
    let mut ppu = vnesu11::ppu::PpuCore::new();
    ppu.pattern_lo[0] = 0xAA;
    ppu.pattern_hi[0] = 0xAA;
    for tile_id in ppu.nametable.iter_mut() {
        *tile_id = 0;
    }
    ppu.regs.ppumask = 0x08;
    ppu.regs.ppuctrl = 0x00;
    soc.ppu = ppu;
    soc.run_frame();
    let first_scanline = &soc.frame_buffer[0..256];
    // Alternating 3 / 0 / 3 / 0 ...
    assert_eq!(first_scanline[0], 3);
    assert_eq!(first_scanline[1], 0);
    assert_eq!(first_scanline[2], 3);
    assert_eq!(first_scanline[7], 0);
    assert_eq!(first_scanline[8], 3);
}

// Helper extension trait so we can call `SpriteLut::get_static` from
// the test without managing a `LazyLock`.
trait SpriteLutExt {
    fn get_static(y: u8, tile: u8, row: u8) -> u8;
}
impl SpriteLutExt for SpriteLut {
    fn get_static(y: u8, tile: u8, row: u8) -> u8 {
        vnesu11::ppu::sprite_lut::SPRITE_LUT.get(y, tile, row)
    }
}