//! v2.1.3 batch 2 synthetic rendering tests.
//!
//! These tests drive `rendering::tick_dot` — the per-dot background
//! pipeline — over a whole scanline and check the pixel output for a
//! known nametable/CHR/attribute layout. They pin the per-pixel colour,
//! attribute, palette, mirroring and grayscale paths in isolation from
//! any CPU/mapper activity.

use fceux11_ppu::bus::FlatBus;
use fceux11_ppu::registers::{ctrl_bits, mask_bits};
use fceux11_ppu::rendering::{RenderWindows, tick_dot as render_dot};
use fceux11_ppu::state::{DOTS_PER_SCANLINE, PpuState};
use fceux11_ppu::tick_dot as frame_dot;

/// Render scanline `sl` and return its 256 pixels.
///
/// The pipeline is a pipeline: the pixels of a line come from fetches
/// made during the previous line's preload window (dots 321-336), so a
/// priming pass runs first and the shift-register state it leaves behind
/// is what the render pass outputs.
fn render_line(
    state: &mut PpuState,
    bus: &mut FlatBus,
    win: &RenderWindows<'_>,
    sl: i16,
) -> [u8; 256] {
    state.ppudead = 0;
    state.scanline = sl;
    state.dot = 0;
    let (t0, fine0) = (state.registers.t, state.registers.fine_x);
    let mut scratch = [0u8; 256 * 256];
    for _ in 0..DOTS_PER_SCANLINE {
        render_dot(state, bus, win, &mut scratch);
        frame_dot(state, bus);
    }
    // Resume at dot 0 of the requested line with what the previous
    // line's preload window leaves behind: the first two tiles in the
    // shift registers and v carrying t's horizontal bits plus the two
    // preload coarse-X increments.
    state.registers.v = t0;
    state.registers.increment_coarse_x();
    state.registers.increment_coarse_x();
    state.registers.fine_x = fine0;
    state.scanline = sl;
    state.dot = 0;

    let mut fb = [0u8; 256 * 256];
    for _ in 0..DOTS_PER_SCANLINE {
        render_dot(state, bus, win, &mut fb);
        frame_dot(state, bus);
    }
    let mut row = [0u8; 256];
    let off = sl as usize * 256;
    row.copy_from_slice(&fb[off..off + 256]);
    row
}

#[test]
fn render_solid_color_pattern() {
    // 32 nametable tiles all pointing to CHR tile 0, a solid pattern
    // (both planes $FF) with attribute 0. Colour 3 → palette[3], and
    // PaletteAdjustPixel sets bit 7 when no emphasis bit is set.
    let mut state = PpuState::new();
    let mut chr = [0u8; 8192];
    for i in 0..16 {
        chr[i] = 0xFF;
    }
    let nt = [0u8; 4096];
    let mut palette = [0u8; 32];
    palette[3] = 0x16;

    state.registers.write_ctrl(0); // BG pattern $0000
    state.registers.write_mask(1 << mask_bits::SHOW_BG);
    state.registers.v = 0x2000;
    state.registers.t = 0x2000;

    let win = RenderWindows {
        nt: &nt,
        chr: &chr,
        palette: &palette,
        mirror: 0,
    };
    let mut bus = FlatBus::new();
    let row = render_line(&mut state, &mut bus, &win, 0);

    for x in 0..256 {
        assert_eq!(row[x], 0x96, "solid palette[3] pixel {x}");
    }
}

#[test]
fn render_attribute_is_uniform_across_each_8_pixel_string() {
    // Attribute byte $E4 = quadrants 0,1,2,3 across the 4x4 region. A
    // solid tile makes each 8-pixel string one solid colour: two tiles
    // per quadrant, and no half-tile attribute change (the FCEUX
    // `atlatch` quirk this pipeline replaced).
    let mut chr = [0u8; 8192];
    for i in 0..16 {
        chr[i] = 0xFF;
    }
    let mut nt = [0u8; 4096];
    nt[0x3C0] = 0xE4;
    nt[0x3C1] = 0xE4;
    // Palette entries are their own values, so colour `c` renders
    // (c | 0x80) and the quadrant shows up directly.
    let mut palette = [0u8; 32];
    for i in 0..16 {
        palette[i] = i as u8;
    }

    let mut state = PpuState::new();
    state.registers.write_ctrl(0);
    state.registers.write_mask(1 << mask_bits::SHOW_BG);
    state.registers.v = 0x2000;
    state.registers.t = 0x2000;

    let win = RenderWindows {
        nt: &nt,
        chr: &chr,
        palette: &palette,
        mirror: 0,
    };
    let mut bus = FlatBus::new();
    let row = render_line(&mut state, &mut bus, &win, 0);

    // Pixels 0..15 are the preloaded tiles (coarse X 0 and 1), then one
    // fetched tile per 8-pixel string starting at coarse X 2. Each
    // attribute byte covers four tiles, so the quadrants come in pairs
    // twice: 0, 0, 1, 1 across each byte.
    let expected = [0x83u8, 0x83, 0x87, 0x87, 0x83, 0x83, 0x87, 0x87];
    for tile in 0..8 {
        for px in 0..8 {
            assert_eq!(
                row[tile * 8 + px],
                expected[tile],
                "8-pixel string {tile} pixel {px}"
            );
        }
    }
}

#[test]
fn render_rendering_off_fills_backdrop() {
    // With $2001 rendering bits clear the picture region shows EXT input
    // (palette index 0), still through PaletteAdjustPixel.
    let mut state = PpuState::new();
    let chr = [0u8; 8192];
    let nt = [0u8; 4096];
    let palette = [0u8; 32];

    state.registers.write_mask(0);
    state.registers.v = 0x2000;
    state.registers.t = 0x2000;

    let win = RenderWindows {
        nt: &nt,
        chr: &chr,
        palette: &palette,
        mirror: 0,
    };
    let mut bus = FlatBus::new();
    let row = render_line(&mut state, &mut bus, &win, 0);

    for x in 0..256 {
        assert_eq!(row[x], 0x80, "backdrop fill pixel {x}");
    }
}

#[test]
fn render_grayscale_masks_palette() {
    let mut state = PpuState::new();
    let mut chr = [0u8; 8192];
    for i in 0..16 {
        chr[i] = 0xFF;
    }
    let nt = [0u8; 4096];
    let mut palette = [0u8; 32];
    // Pattern $FF → colour 3; grayscale masks the palette entry with
    // $30, so 0x3F renders 0x30 → 0xB0 after PaletteAdjustPixel.
    palette[3] = 0x3F;

    state.registers.write_ctrl(0);
    state
        .registers
        .write_mask(1 << mask_bits::SHOW_BG | 1 << mask_bits::GRAYSCALE);
    state.registers.v = 0x2000;
    state.registers.t = 0x2000;

    let win = RenderWindows {
        nt: &nt,
        chr: &chr,
        palette: &palette,
        mirror: 0,
    };
    let mut bus = FlatBus::new();
    let row = render_line(&mut state, &mut bus, &win, 0);

    for x in 0..256 {
        assert_eq!(row[x], 0xB0, "grayscale masked pixel {x}");
    }
}

#[test]
fn render_mirroring_horizontal() {
    // NT page 2 ($2800) mirrors to page 0 in horizontal mode.
    let mut state = PpuState::new();
    let mut chr = [0u8; 8192];
    for i in 0..16 {
        chr[i] = 0xFF;
    }
    let nt = [0u8; 4096];
    let mut palette = [0u8; 32];
    palette[3] = 0x10;

    state.registers.write_ctrl(0 << ctrl_bits::BG_PATTERN);
    state.registers.write_mask(1 << mask_bits::SHOW_BG);
    state.registers.v = 0x2800;
    state.registers.t = 0x2800;

    let win = RenderWindows {
        nt: &nt,
        chr: &chr,
        palette: &palette,
        mirror: 0,
    };
    let mut bus = FlatBus::new();
    let row = render_line(&mut state, &mut bus, &win, 0);

    for x in 0..256 {
        assert_eq!(row[x], 0x90, "page 2 mirrors page 0 pixel {x}");
    }
}

#[test]
fn render_sprite_over_opaque_background() {
    // Sprite 0 at x=10 with a fully opaque pattern row: the sprite
    // palette region ($3F10+) wins in front of the BG, and the
    // sprite-0 hit latches on the first overlapping opaque BG pixel.
    let mut state = PpuState::new();
    let mut chr = [0u8; 8192];
    for i in 0..16 {
        chr[i] = 0xFF; // BG tile 0 opaque
    }
    chr[0x01 * 16] = 0xFF; // sprite tile 1 opaque
    let nt = [0u8; 4096];
    let mut palette = [0u8; 32];
    palette[3] = 0x16;
    palette[0x11] = 0x27;

    state.registers.write_ctrl(0);
    state
        .registers
        .write_mask((1 << mask_bits::SHOW_BG) | (1 << mask_bits::SHOW_SPRITES));
    state.registers.v = 0x2000;
    state.registers.t = 0x2000;
    state.oam[0..4].copy_from_slice(&[0, 0x01, 0x00, 10]);

    let win = RenderWindows {
        nt: &nt,
        chr: &chr,
        palette: &palette,
        mirror: 0,
    };
    let mut bus = FlatBus::new();
    let row = render_line(&mut state, &mut bus, &win, 0);

    let sprite_px = 0x27 | 0x80;
    for x in 10..18 {
        assert_eq!(row[x], sprite_px, "sprite pixel {x}");
    }
    assert_eq!(row[9], 0x96, "background left of the sprite");
    assert_ne!(
        state.registers.status & (1 << fceux11_ppu::status_bits::SPRITE0_HIT),
        0,
        "sprite 0 over an opaque BG pixel must latch the hit"
    );
}
