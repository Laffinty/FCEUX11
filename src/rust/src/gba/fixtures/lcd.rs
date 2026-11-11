//! The LCD bitmap-mode fixtures — S3-e.
//!
//! | fixture | what it drives | belongs to |
//! |---|---|---|
//! | `mode5_page_flip_reads_the_selected_frame` | DISPCNT bit 4 selects between the two mode-5 frames | S3-e-2 (`s3de_design.md` §2.2) |
//! | `mode4_index0_is_transparent_under_a_lower_priority_obj` | mode 4 palette index 0 stays transparent | S3-e-1 防回归锁 (`s3de_design.md` §2.1) |
//!
//! # The two contracts
//!
//! **Mode 5 page flip (the defect S3-e fixes).** `render_mode5` had no
//! `frame_offset`: whichever bit 4 said, it read Frame 0. GBATEK `:1918` puts
//! the Display Frame Select at DISPCNT bit 4 "(for BG Modes 4,5 only)" and
//! `:3204-3206` splits mode 5's 40 KB bitmap into Frame 0 =
//! `06000000-06009FFF` and Frame 1 = `0600A000-06013FFF`. The fixture fills
//! the two frames with **row-distinct** patterns — row y of frame 0 is colour
//! `2000h+y`, row y of frame 1 is `4000h+y` — and runs the machine twice,
//! once per bit-4 value. The per-row distinction is what makes the mutations
//! observable: "no offset at all" reads frame 0's rows (wrong colour per
//! row), and "mode-4 base `0x9600`" — the page-size confusion — reads frame
//! 0's rows 75-77, which hold *other* rows' colours. With a per-frame
//! constant fill both mutations would pass (discipline 12: the assertion's
//! observability must live on the path the mutation touches).
//!
//! **Mode 4 index 0 (the lock that must never regress).** GBATEK `:3195-3198`:
//! "Color 0 (backdrop) is transparent, and OBJs may be displayed behind the
//! bitmap." The vendor already does this (`render_mode4` returns `None` for
//! palette index 0) — an older plan text said the opposite and was falsified
//! against the snapshot, so this fixture exists so nobody "fixes" it back.
//!
//! ⚠️ **The toothless assertion trap.** "index 0 renders as palette entry 0"
//! proves nothing: the backdrop colour *is* `bg_palette_ram[0]`, so the
//! transparent and the opaque readings have the same value. The fixture gets
//! its teeth from **priority**: BG2 runs at priority 0 and the sprite at
//! priority 1 — deliberately worse. On an index-0 column *under the sprite*,
//! transparency lets the sprite show (colour Z); opacity lets BG2 win (colour
//! X = the backdrop colour). Same column, two different readings, one
//! behaviour pin.
//!
//! # Contract shared with `oam.rs`
//!
//! * The machine runs to its `Halt` and then keeps stepping (`RENDER_STEPS`)
//!   so the LCD renders the compared scanlines; `VCOUNT` is asserted to have
//!   passed them, because a framebuffer from a machine that never got there
//!   proves nothing.
//! * The display setup is planted through the bus, not written by the
//!   program: it is not the input under test, and keeping it out of the
//!   program keeps every run's instruction count identical — two runs that
//!   stop at different points have different framebuffers, and the
//!   comparison between them would be meaningless.
//! * Expected colours are constants the fixtures assemble from named parts,
//!   never read back from the implementation.

use super::{mov_imm, orr_imm, strh, swi, Cartridge, Fixture, BUDGET, HALT};

// ---------------------------------------------------------------------------
// Addresses and register values
// ---------------------------------------------------------------------------

/// `DISPCNT` (`04000000h`).
const DISPCNT_ADDR: usize = 0x0400_0000;
/// `BG2CNT` (`0400000Ch`): bits 0-1 are BG2's priority.
const BG2CNT_ADDR: usize = 0x0400_000C;
/// OAM's base address (GBATEK `:3329`), as in `oam.rs`.
const OAM_BASE: u32 = 0x0700_0000;
/// OBJ0's attribute halfwords, 2 bytes apart (GBATEK `:3344-3346`).
const OBJ0_ATTR1_OFFSET: u32 = 2;
const OBJ0_ATTR2_OFFSET: u32 = 4;
/// `VCOUNT` (`04000006h`), as in `oam.rs`.
const VCOUNT_ADDR: usize = 0x0400_0006;
/// The screen width (`lcd.rs:83`) — repeated here for the same reason
/// `oam.rs` repeats it: the vendor's constant is private.
const SCREEN_WIDTH: usize = 240;

/// How many steps the fixture runs after the program halts, as in `oam.rs`:
/// a scanline is 1232 cycles and the machine spends one master cycle per
/// step while the CPU is halted, so this covers ~16 scanlines.
const RENDER_STEPS: usize = 20_000;

/// Mode 5's Frame 0 and Frame 1 base addresses in VRAM
/// (GBATEK `:3204-3206`).
const FRAME0: usize = 0x0600_0000;
const FRAME1: usize = 0x0600_A000;

/// DISPCNT for **mode 4** with BG2 and OBJ enabled, frame 0 selected:
/// mode bits 0-2 = 4, BG2 enable = bit 10, OBJ enable = bit 12, bit 4 clear,
/// forced blank (bit 7) clear, 2-dimensional OBJ mapping (bit 6 clear).
const MODE4_BG2_OBJ: u32 = 0x1404;

/// DISPCNT for **mode 5** with BG2 enabled. Frame select (bit 4) is 0 in the
/// frame-0 run and 1 in the frame-1 run — the only difference between them.
const MODE5_BG2: u32 = 0x0405;
const MODE5_BG2_FRAME1: u32 = 0x0415;

// The three colours of the mode-4 fixture. X is BG palette entry 0 -- which
// is also what the backdrop is built from (`lcd.rs:380-385`) -- Y is entry 1,
// and Z is OBJ palette entry 1, the colour the sprite's tile paints with.
/// BG palette entry 0 = the backdrop colour.
const COLOR_X: u16 = 0x7C00;
/// BG palette entry 1.
const COLOR_Y: u16 = 0x03E0;
/// OBJ palette entry 1: the colour the sprite's tile paints with.
const COLOR_Z: u16 = 0x001F;

/// The BG palette word at `05000000h`: entry 0 in the low half, entry 1 in
/// the high half (little-endian, 2 bytes per entry).
const BG_PALETTE_ADDR: usize = 0x0500_0000;
/// The OBJ palette word at `05000200h`: entry 0 in the low half, entry 1 in
/// the high half.
const OBJ_PALETTE_ADDR: usize = 0x0500_0200;

/// The sprite's OBJ tile number.
///
/// **512, not a small number**: in bitmap modes 3-5 the lower half of OBJ
/// VRAM holds the bitmap, and the vendor skips tile numbers below 512 there
/// (`layer_obj.rs:243-246`; GBATEK `:3156-3157` — bitmap-mode OBJ tiles live
/// only in the 16 KB at `06014000`). Tile 512 reads at
/// `video_ram[0x10000 + (512 << 5)] = video_ram[0x14000]` = VRAM
/// `06014000`, the first tile of the bitmap-mode OBJ area.
const SPRITE_TILE: u32 = 512;
/// Where tile 512's 32 bytes sit (see [`SPRITE_TILE`]).
const SPRITE_TILE_ADDR: usize = 0x0601_4000;
/// The tile's 32 bytes: every 4bpp nibble is colour index 1, so the whole
/// sprite paints `obj_palette_ram[2..4]` — OBJ palette entry 1, `COLOR_Z`.
const OBJ_TILE_DATA: [u32; 8] = [0x1111_1111; 8];

/// The sprite's position: OBJ0's attr0 Y and attr1 X.
///
/// The vendor places the sprite at the OAM coordinates directly (see the
/// note in `oam.rs`), so Y = 1 covers scanlines 1-8 and X = 20 covers
/// columns 20-27.
const SPRITE_Y: u32 = 1;
const SPRITE_X: u32 = 20;

// ---------------------------------------------------------------------------
// Framebuffer access, shared by both fixtures
// ---------------------------------------------------------------------------

/// One rendered row, as raw colour words — the same access `oam.rs` uses:
/// `Lcd::buffer` is public, and the base module's private `gba` field is
/// visible to its descendant modules.
fn frame_row(fixture: &Fixture, y: usize) -> Vec<u16> {
    let buffer = &fixture.gba.cpu.bus.lcd.buffer;
    (0..SCREEN_WIDTH).map(|x| buffer[y][x].0).collect()
}

/// Read `VCOUNT` through the bus.
fn vcount(fixture: &mut Fixture) -> u16 {
    fixture.read_half_word(VCOUNT_ADDR)
}

// ---------------------------------------------------------------------------
// Fixture A: mode 5 page flip (S3-e-2)
// ---------------------------------------------------------------------------

/// The mode-5 pattern colour for row `y` of one frame.
///
/// Frame 0 uses the `2000h` band, frame 1 the `4000h` band; every row gets
/// its own value so "read frame 0's row 75" cannot pass for "read frame 1's
/// row 0" (that is what the `0x9600` mutation does — see the module doc).
fn mode5_color(frame1: bool, y: usize) -> u16 {
    if frame1 {
        0x4000 + y as u16
    } else {
        0x2000 + y as u16
    }
}

/// Fill the whole 128-row mode-5 frame with the per-row pattern.
fn plant_mode5_frame(fixture: &mut Fixture, base: usize, frame1: bool) {
    // 128 rows x 160 pixels x 2 bytes = the whole 40 KB frame; a word write
    // covers two horizontally adjacent pixels.
    for y in 0..128usize {
        let color = mode5_color(frame1, y);
        let word = u32::from(color) | (u32::from(color) << 16);
        for word_index in 0..80usize {
            fixture.write_word(base + y * 320 + word_index * 4, word);
        }
    }
}

/// Build, plant and run one mode-5 pass. The only difference between the two
/// runs is the DISPCNT word planted here, so the runs are otherwise
/// instruction-for-instruction identical.
fn run_mode5(dispcnt: u32) -> Fixture {
    let mut cartridge = Cartridge::new();
    cartridge.code(&[swi(HALT)]);
    let mut fixture = Fixture::new(&cartridge.build());

    fixture.write_word(DISPCNT_ADDR, dispcnt);
    fixture.gba.cpu.bus.write_half_word(BG2CNT_ADDR, 0x0000); // BG2 priority 0
    plant_mode5_frame(&mut fixture, FRAME0, false);
    plant_mode5_frame(&mut fixture, FRAME1, true);

    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    fixture.run_steps(RENDER_STEPS);
    fixture
}

/// The mode-5 rows whose whole width the fixture compares.
const MODE5_ROWS: usize = 3;

/// With bit 4 clear the LCD shows frame 0's pattern, with bit 4 set frame
/// 1's — per row; outside the 160-wide bitmap the backdrop shows. Each run
/// is the other's control: "always frame 0" fails the frame-1 assertions and
/// "always frame 1" fails the frame-0 ones, and the two runs' rows must
/// differ on top of that.
#[test]
fn mode5_page_flip_reads_the_selected_frame() {
    let mut frame0_run = run_mode5(MODE5_BG2);
    let mut frame1_run = run_mode5(MODE5_BG2_FRAME1);

    assert!(
        vcount(&mut frame0_run) >= MODE5_ROWS as u16,
        "the frame-0 run never rendered the compared rows"
    );
    assert!(
        vcount(&mut frame1_run) >= MODE5_ROWS as u16,
        "the frame-1 run never rendered the compared rows"
    );

    let frame0_rows: Vec<Vec<u16>> = (0..MODE5_ROWS)
        .map(|y| frame_row(&frame0_run, y))
        .collect();
    let frame1_rows: Vec<Vec<u16>> = (0..MODE5_ROWS)
        .map(|y| frame_row(&frame1_run, y))
        .collect();

    // Run A: bit 4 clear -- frame 0's per-row pattern, backdrop outside.
    for (y, row) in frame0_rows.iter().enumerate() {
        for x in 0..160 {
            assert_eq!(
                row[x],
                mode5_color(false, y),
                "bit 4 clear, row {y}, column {x}: the LCD must show frame 0"
            );
        }
        for x in 160..SCREEN_WIDTH {
            assert_eq!(
                row[x],
                0,
                "bit 4 clear, row {y}, column {x}: outside the mode-5 bitmap \
                 the backdrop (BG palette entry 0 = 0 here) shows"
            );
        }
    }

    // Run B: bit 4 set -- frame 1's per-row pattern, backdrop outside.
    for (y, row) in frame1_rows.iter().enumerate() {
        for x in 0..160 {
            assert_eq!(
                row[x],
                mode5_color(true, y),
                "bit 4 set, row {y}, column {x}: the LCD must show frame 1"
            );
        }
        for x in 160..SCREEN_WIDTH {
            assert_eq!(row[x], 0, "the backdrop shows outside the bitmap");
        }
    }

    assert_ne!(
        frame0_rows, frame1_rows,
        "the two runs rendered identical rows: the frame select changed \
         nothing, so the comparison above cannot have been about the page \
         flip"
    );
}

// ---------------------------------------------------------------------------
// Fixture B: mode 4 palette index 0 (S3-e-1 防回归锁)
// ---------------------------------------------------------------------------

/// The mode-4 index pattern for one row: alternating index 0 (even columns)
/// and index 1 (odd columns).
///
/// One byte per pixel (GBATEK `:3195-3197`), frame 0 selected by the
/// fixture's DISPCNT.
fn mode4_index(x: usize) -> u8 {
    (x & 1) as u8
}

/// Plant `rows` patterned rows of frame 0 starting at `y`.
///
/// The row is composed **four bytes per word write, at aligned addresses**:
/// the bus rounds a word write down to its word boundary
/// (`bus.rs` `write_word`), so a byte-per-word write would clobber the three
/// bytes after it and the pattern would land every fourth byte instead of
/// every byte.
fn plant_mode4_rows(fixture: &mut Fixture, y0: usize, rows: usize, pattern: fn(usize) -> u8) {
    for y in y0..y0 + rows {
        for word_index in 0..60usize {
            let mut word = 0u32;
            for lane in 0..4usize {
                word |= u32::from(pattern(word_index * 4 + lane)) << (lane * 8);
            }
            fixture.write_word(FRAME0 + y * 240 + word_index * 4, word);
        }
    }
}

/// OBJ0's three attributes: a legal 8x8 sprite at (`SPRITE_X`, `SPRITE_Y`)
/// with tile [`SPRITE_TILE`] at **priority 1** — deliberately worse than
/// BG2's priority 0, which is what makes an index-0 column under the sprite
/// distinguish "BG2 transparent" from "BG2 opaque".
fn sprite_program() -> Vec<u32> {
    let attr0 = SPRITE_Y; // bits 0-7: Y; square, normal, not disabled
    let attr1 = SPRITE_X; // bits 0-8: X; size 0 (8x8)
    let attr2 = SPRITE_TILE | (1 << 10); // bits 0-9 tile, bits 10-11 priority 1
    vec![
        mov_imm(3, OAM_BASE),
        mov_imm(2, attr0),
        strh(2, 3), // OBJ0 attr0
        mov_imm(2, attr1),
        orr_imm(0, 3, OBJ0_ATTR1_OFFSET),
        strh(2, 0), // OBJ0 attr1
        mov_imm(2, attr2),
        orr_imm(0, 3, OBJ0_ATTR2_OFFSET),
        strh(2, 0), // OBJ0 attr2
        swi(HALT),
    ]
}

/// Plant the mode-4 display setup and run the sprite program.
fn run_mode4() -> Fixture {
    let mut cartridge = Cartridge::new();
    cartridge.code(&sprite_program());
    let mut fixture = Fixture::new(&cartridge.build());

    fixture.write_word(DISPCNT_ADDR, MODE4_BG2_OBJ);
    fixture.gba.cpu.bus.write_half_word(BG2CNT_ADDR, 0x0000); // BG2 priority 0
    fixture.write_word(BG_PALETTE_ADDR, u32::from(COLOR_Y) << 16 | u32::from(COLOR_X));
    fixture.write_word(
        OBJ_PALETTE_ADDR,
        u32::from(COLOR_Z) << 16, // entry 1 = Z; entry 0 stays 0 (transparent)
    );
    for (index, word) in OBJ_TILE_DATA.iter().enumerate() {
        fixture.write_word(SPRITE_TILE_ADDR + index * 4, *word);
    }
    // Row 0 all index 1 (the display-alive control above the sprite), row 1
    // alternating, row 2 all index 1 again.
    plant_mode4_rows(&mut fixture, 0, 1, |_| 1);
    plant_mode4_rows(&mut fixture, 1, 1, mode4_index);
    plant_mode4_rows(&mut fixture, 2, 1, |_| 1);

    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    fixture.run_steps(RENDER_STEPS);
    fixture
}

/// Mode 4's palette index 0 is transparent (GBATEK `:3195-3198`), so a
/// lower-priority OBJ behind the bitmap shows through an index-0 column.
///
/// The four assertions interlock:
/// * row 0, column 0 (index 1, no OBJ): `COLOR_Y` — the bitmap path works;
/// * row 1, column 3 (index 1, outside the sprite): `COLOR_Y` — ditto on the
///   compared row;
/// * row 1, column 2 (index 0, outside the sprite): `COLOR_X` — the backdrop
///   path works (this leg alone is toothless: the opaque reading has the
///   same value);
/// * row 1, column 22 (index 0, under the priority-1 sprite): `COLOR_Z` —
///   **the tooth**. Transparency lets the sprite show; an index 0 rendered
///   opaque would let priority-0 BG2 win with `COLOR_X` instead.
#[test]
fn mode4_index0_is_transparent_under_a_lower_priority_obj() {
    let mut fixture = run_mode4();

    assert!(
        vcount(&mut fixture) >= 3,
        "the LCD never rendered the compared rows"
    );

    let row0 = frame_row(&fixture, 0);
    let row1 = frame_row(&fixture, 1);

    assert_eq!(
        row0[0], COLOR_Y,
        "row 0 is all index 1 with no sprite on it: the bitmap path must work"
    );
    assert_eq!(
        row1[3], COLOR_Y,
        "index 1 outside the sprite must show BG palette entry 1"
    );
    assert_eq!(
        row1[2], COLOR_X,
        "index 0 outside the sprite shows the backdrop colour -- which is \
         palette entry 0; this leg alone proves nothing, the next one has \
         the teeth"
    );
    assert_eq!(
        row1[usize::try_from(SPRITE_X).unwrap() + 2],
        COLOR_Z,
        "index 0 under the priority-1 sprite must be transparent and let the \
         sprite show; BG2 rendering it opaque would paint COLOR_X here instead"
    );
    // And the interlock's other half: an index-1 column under the sprite is
    // BG2 priority 0 beating the OBJ's priority 1.
    assert_eq!(
        row1[usize::try_from(SPRITE_X).unwrap() + 3],
        COLOR_Y,
        "index 1 under the sprite: BG2's priority 0 wins over the OBJ's 1"
    );
}
