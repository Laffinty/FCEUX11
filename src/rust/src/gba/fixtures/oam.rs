//! The OAM fixtures — S1-a, defect domain **illegal sprite attributes**.
//!
//! | fixture | what it drives | belongs to |
//! |---|---|---|
//! | `oam_bad_shape` | `STRH` writes OAM sprite 0's attr0 with **shape = 3** | S2-a |
//! | `oam_gfx_mode3` | the same, but attr0 with **gfx mode = 3** | S2-a |
//!
//! # The defect (v2.0.3 plan §1.2.2, corrected in r2)
//!
//! These are the **only two reachable panic sites** in the vendor core:
//! `object_attributes.rs:265` (`.unwrap()` on a prohibited `ObjShape`) and
//! `object_attributes.rs:144` (`unreachable!()` in `GfxMode::try_from`). The
//! release profile is `panic = "abort"`, so a panic in an `extern "C"` frame
//! takes the whole process down — that is the crash the commercial cartridges
//! show.
//!
//! On real hardware `shape = 3` and `gfx mode = 3` are **prohibited values**
//! and the sprite is simply not drawn. (`obj mode = 2` is the real "disable"
//! and the vendor already handles it.) So the target behaviour is
//! **"the sprite is treated as disabled"**, not "the write is ignored".
//!
//! # Contract
//!
//! * The OAM write **must** be a halfword or a word store: `bus.rs:1231-1234`
//!   ignores `STRB` into OAM, so a byte store cannot produce an illegal value
//!   and the fixture would measure nothing.
//! * The assertion writes the **target** behaviour, never the panic.
//! * Every reproducer carries `#[ignore = "S2-a: 修复后转常驻锁测试"]`.
//! * A panic must be **caught and read**, never left to abort the test
//!   binary: the release profile's `panic = "abort"` makes `catch_unwind`
//!   useless, so the reproducer has to survive in the debug profile and say
//!   what it saw.
//!
//! # Where the panic happens, and how the fixture gets there
//!
//! The OAM is parsed **once per visible scanline**, not once per frame:
//!
//! ```text
//!   Gba::step()                      (gba.rs:103)
//!     -> Arm7tdmi::step()            (arm7tdmi.rs:738)
//!       -> Bus::step()               (bus.rs:1301)  -- one master cycle
//!         -> Lcd::step()             (lcd.rs:208)   -- one pixel per 4 cycles
//!           -> LayerObj::handle_enter_vdraw()   (layer_obj.rs:691)
//!             -> object_attributes::get_attributes()  (object_attributes.rs:458)
//!               -> ObjAttribute0::try_from()      (object_attributes.rs:256)
//!                 -> .unwrap() / unreachable!()    -- the two panic sites
//! ```
//!
//! `handle_enter_vdraw` runs at the **start of every scanline below 160**
//! (`lcd.rs:211-231`), so the OAM is re-read 160 times per frame. The first
//! of those reads happens a few cycles into the boot, long before the
//! program has written anything; the read that sees the illegal attribute is
//! the one at the start of the **next** scanline, about 1232 cycles
//! (308 pixels x 4 cycles) after the machine started.
//!
//! So the fixture does not have to *make* the LCD render anything in
//! particular: it writes the illegal attribute and then lets the machine run
//! until the LCD reaches the next scanline start. The panic happens on the
//! way there.
//!
//! # How the fixture knows the sprite was "drawn" or "not drawn"
//!
//! The framebuffer is the only observable of the *rendered* result that
//! survives the vendor crate's module boundaries: `object_attributes` is a
//! private module, so the parsed attributes cannot be read from outside
//! `gba-core`. `Lcd::buffer` is public, so the fixture compares the rendered
//! pixels instead.
//!
//! Two details make that comparison mean something:
//!
//! * **The target sprite has its own tile.** OAM starts zeroed, and a
//!   zeroed sprite is a *legal* 8x8 square at (0, 0) drawing tile 0 — so
//!   all 127 untouched OAM slots are sprites too, and they would paint over
//!   the target sprite's area. The target sprite uses **tile 1** and tile 0
//!   is left blank, so the untouched slots draw nothing (palette index 0 is
//!   transparent, `layer_obj.rs:455-459`) and only the target sprite can
//!   produce the sprite's colour.
//! * **A legal control sprite of the same shape must be drawn.** Otherwise
//!   "the prohibited sprite is invisible" would hold for the wrong reason —
//!   a fixture whose display setup does not work would pass.
//!
//! With that in place the fixture asserts three things:
//!
//! 1. the legal sprite reaches the framebuffer,
//! 2. an explicitly disabled sprite (GBATEK's `OBJ Disable` bit) does not,
//! 3. the prohibited sprite produces **exactly** the disabled sprite's
//!    frame — "treated as disabled", not "the write was ignored" (an ignored
//!    write would leave the old, legal attribute in place and the sprite
//!    would still be drawn).
//!
//! # The numbers, from the GBATEK snapshot
//!
//! `.zcode/gbatek_plain.txt`:
//!
//! * `:3323-3330` — OAM is at `07000000h`-`070003FFh`, 1 Kbyte.
//! * `:3344-3346` — OBJ0's entry is at `07000000h`; each OBJ entry is
//!   three 16-bit attributes.
//! * `:3356-3373` — **OBJ Attribute 0**: bits 0-7 Y, bit 8 rotation flag,
//!   bit 9 `OBJ Disable`, **bits 10-11 `OBJ Mode` (3 = Prohibited)**, bit 12
//!   mosaic, bit 13 colours, **bits 14-15 `OBJ Shape` (3 = Prohibited)**.
//! * `:3388-3410` — OBJ Attribute 1: bits 0-8 X, bit 12/13 flips, bits 14-15
//!   size.
//! * `:3418-3428` — OBJ Attribute 2: bits 0-9 tile, 10-11 priority, 12-15
//!   palette.
//!
//! The vendor core calls the bits 10-11 field `gfx_mode`
//! (`object_attributes.rs:121-147`) and the bits 8-9 field `obj_mode`
//! (`object_attributes.rs:90-119`); GBATEK calls the same fields `OBJ Mode`
//! and (bit 8 / bit 9) the rotation flag and `OBJ Disable`. The two
//! descriptions agree field by field, and `obj mode = 2` — the value the
//! plan calls "disabled" — is bit 9 set, GBATEK's `OBJ Disable`.
//!
//! # Recommendations for the common base
//!
//! Three things this file needs are general enough to belong in
//! `fixtures.rs`:
//!
//! * a way to reach the rendered framebuffer (the base's `Fixture` hides
//!   the core),
//! * a "drive the machine until the LCD has rendered N scanlines" helper,
//! * the panic probe that turns a panic into a reading.
//!
//! Until then this file builds on the base's `Fixture` and reaches the
//! framebuffer through it, the way `fixtures/swi.rs` reaches the core.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};

use super::{BUDGET, Cartridge, Fixture, HALT, mov_imm, orr_imm, strh, swi};

// ---------------------------------------------------------------------------
// Addresses and register values
// ---------------------------------------------------------------------------

/// OAM's base address (GBATEK `:3329`).
const OAM_BASE: u32 = 0x0700_0000;

/// OBJ0's attribute halfwords (GBATEK `:3344-3346`: OBJ0 is at `07000000h`
/// and each attribute is 16 bits, so attr0/attr1/attr2 are 2 bytes apart).
const OBJ0_ATTR1_OFFSET: u32 = 2;
const OBJ0_ATTR2_OFFSET: u32 = 4;

/// `DISPCNT` (`04000000h`): the OBJ layer's enable bit.
const DISPCNT_ADDR: usize = 0x0400_0000;
/// `DISPCNT` bit 12 — "OBJ enable" (`lcd.rs:251-253`).
///
/// Bit 7 (forced blank) stays clear, bit 6 (OBJ character VRAM mapping)
/// stays 0 (2-dimensional mapping) and bits 8-11 (the BG layers) stay
/// clear, so the sprite is the only thing that can paint.
const DISPCNT_OBJ_ENABLE: u32 = 0x1000;

/// The target sprite's tile number (GBATEK `:3426`: bits 0-9 of attr2).
///
/// Tile 0 is left blank so that the 127 untouched OAM slots — which are
/// legal 8x8 squares at (0, 0) drawing tile 0 — stay invisible.
const SPRITE_TILE: u32 = 1;

/// OBJ character data: the start of the OBJ tile area.
///
/// The vendor reads a 4bpp tile at `video_ram[0x10000 + (tile << 5) + ...]`
/// (`layer_obj.rs:441-445`), and `video_ram` index 0x10000 is VRAM address
/// `06010000h`.
const OBJ_TILE_AREA: usize = 0x0601_0000;
/// The target sprite's tile: tile 1, 32 bytes further in (a 4bpp tile is 32
/// bytes, `layer_obj.rs:438`).
const SPRITE_TILE_ADDR: usize = OBJ_TILE_AREA + (SPRITE_TILE as usize) * 32;
/// The tile's 32 bytes: every 4bpp pixel is colour index 1 (index 0 is
/// transparent, `layer_obj.rs:455-459`).
///
/// A 4bpp tile row is 4 bytes (`layer_obj.rs:441-442`), so the whole tile
/// has to be written for the sprite's lower rows to be opaque as well.
const OBJ_TILE_DATA: [u32; 8] = [0x1111_1111; 8];

/// The OBJ palette's first two entries (`05000200h`).
///
/// The vendor's OBJ palette entry `n` is read at `obj_palette_ram[2 * n]`
/// (`layer_obj.rs:157-164`), so a 4bpp pixel of colour index 1 (the tile
/// data's nibble value) reads `obj_palette_ram[2..4]` — the **second**
/// entry. `write_word` is word-aligned, so the word's bytes 2 and 3 land
/// exactly there.
const OBJ_PALETTE_ADDR: usize = 0x0500_0200;
/// The word written there: bytes 2-3 are entry 1's colour.
///
/// Little-endian, so byte 2 is the colour's low byte and byte 3 its high
/// byte, which is the order `read_color_from_obj_palette` combines them in.
const OBJ_PALETTE_WORD: u32 = 0x7FFF_0000;

/// `VCOUNT` (`04000006h`), readable through the bus (`bus.rs:875-876`).
const VCOUNT_ADDR: usize = 0x0400_0006;

/// The screen width (`lcd.rs:83`).
///
/// The vendor's own `LCD_WIDTH` is a private constant, so the fixture
/// repeats it; only the rows it compares are indexed by it.
const SCREEN_WIDTH: usize = 240;

/// The sprite's position: OBJ0's Y coordinate (attr0 bits 0-7) and X
/// coordinate (attr1 bits 0-8).
///
/// The vendor places the sprite at the OAM coordinates directly — it does
/// not apply the hardware's -16 (Y) and -8 (X) offsets — so Y = 1 puts the
/// sprite on scanlines 1 to 8.
const SPRITE_Y: u32 = 1;
const SPRITE_X: u32 = 20;

/// The colour a drawn sprite puts on the screen.
const SPRITE_COLOR: u16 = 0x7FFF;

/// How many scanlines the fixture compares.
///
/// The sprite covers scanlines `SPRITE_Y` to `SPRITE_Y + 8`, so nine rows
/// contain it in full.
const COMPARED_ROWS: usize = 9;

/// How many steps the fixture runs after the program halts.
///
/// A scanline is 308 pixels and a pixel is 4 master cycles
/// (`lcd.rs:11-27`), so a scanline is 1232 cycles and the machine spends
/// one master cycle per step while the CPU is halted
/// (`arm7tdmi.rs:750-758`). The OAM is re-read at every scanline start, so
/// the budget only has to cover the scanline the program was running in
/// plus the next one — but it is cheap to be generous, and a longer run
/// also renders more of the frame.
const RENDER_STEPS: usize = 20_000;

// ---------------------------------------------------------------------------
// The attribute values under test
// ---------------------------------------------------------------------------
//
// Every value is split into a "high" part that `MOV` can encode directly and
// a "low" part that is OR-ed in, so that all runs execute the *same number
// of instructions* and therefore reach the same point in the frame. A run
// that stopped one instruction earlier would have a different framebuffer,
// and the comparison between runs would be meaningless.

/// attr0 of the sprite with a **prohibited shape** (bits 14-15 = 3).
const SHAPE_PROHIBITED_HIGH: u32 = 0xC000;
const SHAPE_PROHIBITED_LOW: u32 = SPRITE_Y;

/// attr0 of the sprite with a **prohibited OBJ mode** — the vendor's
/// `gfx_mode`, bits 10-11 = 3.
const GFX_MODE_PROHIBITED_HIGH: u32 = 0x0C00;
const GFX_MODE_PROHIBITED_LOW: u32 = SPRITE_Y;

/// attr0 of a **legal** sprite: square, normal, at `SPRITE_Y`.
const LEGAL_HIGH: u32 = 0x0000;
const LEGAL_LOW: u32 = SPRITE_Y;

/// attr0 of an **explicitly disabled** sprite: GBATEK's `OBJ Disable`
/// (bit 9), which the vendor reads as `ObjMode::Disabled`
/// (`object_attributes.rs:90-119`, `layer_obj.rs:233-241`).
const DISABLED_HIGH: u32 = 0x0200;
const DISABLED_LOW: u32 = SPRITE_Y;

/// OBJ0's attr1: X coordinate, square 8x8 (size 0).
const OBJ0_ATTR1: u32 = SPRITE_X;

/// OBJ0's attr2: the sprite's tile, highest priority, palette bank 0.
const OBJ0_ATTR2: u32 = SPRITE_TILE;

// ---------------------------------------------------------------------------
// Panic -> reading
// ---------------------------------------------------------------------------

/// What a caught panic looked like.
#[derive(Debug)]
struct PanicReading {
    /// `file:line` of the panic, when the probe captured it.
    location: Option<String>,
    /// The panic payload, rendered as text.
    message: String,
}

impl PanicReading {
    /// The reading, as it is reported when the fixture fails.
    fn describe(&self, what: &str, attr0: u16) -> String {
        format!(
            "the vendor core panicked while the LCD parsed the OAM ({what})\n  \
             location: {}\n  \
             message:  {}\n  \
             input:    OAM 0x{OAM_BASE:08X}, OBJ0 attr0 = 0x{attr0:04X}\n  \
             expected: no panic; the sprite must be treated as disabled",
            self.location.as_deref().unwrap_or("<unknown>"),
            self.message,
        )
    }
}

thread_local! {
    /// Where the panic on *this* thread happened.
    ///
    /// Thread-local because the hook is process-wide but a panic is only
    /// interesting on the thread that raised it.
    static RECORDED_PANIC: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// Install a panic hook that records *where* a panic happened.
///
/// Installed **once** for the whole test binary, and it delegates to the
/// hook it replaced, so every other test keeps the output it had before. The
/// recording is thread-local, so it only ever adds information.
fn install_panic_probe() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let location = info
                .location()
                .map(|location| format!("{}:{}", location.file(), location.line()));
            RECORDED_PANIC.with(|slot| {
                *slot.borrow_mut() = location;
            });
            previous(info);
        }));
    });
}

/// Run `body`, turning a panic into a reading instead of letting it unwind
/// out of the test binary.
///
/// The release profile is `panic = "abort"`
/// (`src/rust/Cargo.toml:9-11`): under `--release` a panic takes the whole
/// process down and `catch_unwind` cannot help. The gate runs the **default
/// (debug) profile**, where the strategy is `unwind` and this works.
fn catch_panic_reading<T>(body: impl FnOnce() -> T) -> Result<T, PanicReading> {
    install_panic_probe();
    RECORDED_PANIC.with(|slot| *slot.borrow_mut() = None);
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => Ok(value),
        Err(payload) => Err(PanicReading {
            location: RECORDED_PANIC.with(|slot| slot.borrow_mut().take()),
            message: payload_message(payload.as_ref()),
        }),
    }
}

/// The panic payload as text.
fn payload_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

// ---------------------------------------------------------------------------
// The program
// ---------------------------------------------------------------------------

/// The program: write OBJ0's three attributes, then halt.
///
/// ```text
///     mov  r3, #OAM_BASE
///     mov  r2, #attr0_high
///     orr  r1, r2, #attr0_low   ; r1 = attr0_high | attr0_low
///     strh r1, [r3]             ; OBJ0 attr0  <- the defect's input
///     mov  r1, #OBJ0_ATTR1
///     orr  r0, r3, #2           ; r0 = OAM + 2
///     strh r1, [r0]             ; OBJ0 attr1 (X, size)
///     mov  r1, #OBJ0_ATTR2
///     orr  r0, r3, #4           ; r0 = OAM + 4
///     strh r1, [r0]             ; OBJ0 attr2 (tile, priority, palette)
///     swi  HALT
/// ```
///
/// The OAM stores are **halfword** stores on purpose: `bus.rs:1231-1234`
/// drops `STRB` into OAM, so a byte store could not produce an illegal
/// attribute at all.
///
/// Every `ORR` reads a register the destination does not share, so the
/// result does not depend on how the vendor sequences a read-modify-write
/// with `rd == rn`.
fn program(attr0_high: u32, attr0_low: u32) -> Vec<u32> {
    vec![
        mov_imm(3, OAM_BASE),
        mov_imm(2, attr0_high),
        orr_imm(1, 2, attr0_low),
        strh(1, 3),
        mov_imm(1, OBJ0_ATTR1),
        orr_imm(0, 3, OBJ0_ATTR1_OFFSET),
        strh(1, 0),
        mov_imm(1, OBJ0_ATTR2),
        orr_imm(0, 3, OBJ0_ATTR2_OFFSET),
        strh(1, 0),
        swi(HALT),
    ]
}

// ---------------------------------------------------------------------------
// Running it
// ---------------------------------------------------------------------------

/// What one fixture run observed.
struct Run {
    /// The rendered rows the fixture compares.
    rows: Vec<Vec<u16>>,
    /// `VCOUNT` when the run stopped.
    vcount: u16,
}

/// Build the machine, plant the display setup, run the program, and let the
/// LCD render.
fn run_case(attr0_high: u32, attr0_low: u32) -> Result<Run, PanicReading> {
    let mut cartridge = Cartridge::new();
    cartridge.code(&program(attr0_high, attr0_low));
    let rom = cartridge.build();

    let mut fixture = Fixture::new(&rom);

    // The display setup the sprite needs in order to be *able* to draw.
    //
    // It is planted through the bus rather than by the program, because it
    // is not the defect's input, and because keeping it out of the program
    // keeps every run's program the same length — a run that stopped one
    // instruction earlier would have a different framebuffer, and the
    // comparison between runs would be meaningless.
    fixture.write_word(DISPCNT_ADDR, DISPCNT_OBJ_ENABLE);
    for (index, word) in OBJ_TILE_DATA.iter().enumerate() {
        fixture.write_word(SPRITE_TILE_ADDR + index * 4, *word);
    }
    fixture.write_word(OBJ_PALETTE_ADDR, OBJ_PALETTE_WORD);

    let halted = catch_panic_reading(|| {
        let halted = fixture.run_to_halt(BUDGET);
        // The program is done; the machine keeps running (a halted core still
        // clocks the peripherals, `arm7tdmi.rs:741-758`), so the LCD reaches
        // the next scanline start and re-reads the OAM.
        fixture.run_steps(RENDER_STEPS);
        halted
    })?;

    assert!(halted, "the program never reached its Halt");

    let vcount = fixture.read_half_word(VCOUNT_ADDR);
    assert!(
        vcount >= COMPARED_ROWS as u16,
        "the LCD only reached VCOUNT {vcount}: the sprite's scanline was never rendered"
    );

    Ok(Run {
        rows: frame_rows(&fixture),
        vcount,
    })
}

/// The first `COMPARED_ROWS` rendered rows, as raw colour words.
///
/// `Lcd::buffer` is public (`lcd.rs:161`), but the core is hidden inside the
/// base's `Fixture`, so this reaches it the only way the crate allows: the
/// base module is an ancestor of this one, and its private field is visible
/// to its descendants.
fn frame_rows(fixture: &Fixture) -> Vec<Vec<u16>> {
    let buffer = &fixture.gba.cpu.bus.lcd.buffer;
    (0..COMPARED_ROWS)
        .map(|y| (0..SCREEN_WIDTH).map(|x| buffer[y][x].0).collect())
        .collect()
}

/// The columns of `row` that carry `color`.
fn columns_of(row: &[u16], color: u16) -> Vec<usize> {
    row.iter()
        .enumerate()
        .filter(|(_, pixel)| **pixel == color)
        .map(|(x, _)| x)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        COMPARED_ROWS, DISABLED_HIGH, DISABLED_LOW, GFX_MODE_PROHIBITED_HIGH,
        GFX_MODE_PROHIBITED_LOW, LEGAL_HIGH, LEGAL_LOW, SHAPE_PROHIBITED_HIGH,
        SHAPE_PROHIBITED_LOW, SPRITE_COLOR, SPRITE_X, SPRITE_Y, columns_of, run_case,
    };

    /// OBJ0's attr0 with a **prohibited shape** (bits 14-15 = 3).
    ///
    /// GBATEK `:3373`: "14-15 OBJ Shape (0=Square,1=Horizontal,2=Vertical,
    /// 3=Prohibited)".
    ///
    /// The vendor's `ObjShape::try_from` returns `Err("Prohibited ObjShape")`
    /// for 3 (`object_attributes.rs:194-205`), and `ObjAttribute0::try_from`
    /// unwraps it (`object_attributes.rs:265`) — the first of the two
    /// reachable panic sites.
    #[test]
    #[ignore = "S2-a: 修复后转常驻锁测试"]
    fn oam_bad_shape() {
        assert_treated_as_disabled("shape = 3", SHAPE_PROHIBITED_HIGH, SHAPE_PROHIBITED_LOW);
    }

    /// OBJ0's attr0 with a **prohibited OBJ mode** — the vendor's `gfx_mode`,
    /// bits 10-11 = 3.
    ///
    /// GBATEK `:3370`: "10-11 OBJ Mode (0=Normal, 1=Semi-Transparent, 2=OBJ
    /// Window, 3=Prohibited)".
    ///
    /// The vendor's `GfxMode::try_from` has no arm for 3 and falls into
    /// `unreachable!()` (`object_attributes.rs:136-147`) — the second of the
    /// two reachable panic sites.
    #[test]
    #[ignore = "S2-a: 修复后转常驻锁测试"]
    fn oam_gfx_mode3() {
        assert_treated_as_disabled(
            "gfx mode = 3",
            GFX_MODE_PROHIBITED_HIGH,
            GFX_MODE_PROHIBITED_LOW,
        );
    }

    /// The body both reproducers share.
    ///
    /// The target behaviour is "the prohibited sprite is treated as
    /// disabled", which means **exactly** what an explicitly disabled sprite
    /// looks like — not "the write was ignored" (that would leave the old,
    /// legal attribute in place and the sprite would still be drawn).
    fn assert_treated_as_disabled(what: &str, prohibited_high: u32, prohibited_low: u32) {
        // Control 1: the same sprite, explicitly disabled (GBATEK's
        // `OBJ Disable` bit, which the vendor reads as `ObjMode::Disabled`).
        // This is what "treated as disabled" has to look like.
        let disabled = run_case(DISABLED_HIGH, DISABLED_LOW)
            .expect("the explicitly-disabled control run panicked");
        let disabled_columns = columns_of(&disabled.rows[SPRITE_Y as usize], SPRITE_COLOR);
        assert!(
            disabled_columns.is_empty(),
            "the disabled control sprite was drawn at columns {disabled_columns:?}: the \
             comparison against it would prove nothing"
        );

        // Control 2: the same sprite, legal. It has to be drawn, otherwise
        // "the prohibited sprite is invisible" would hold for the wrong
        // reason.
        let legal = run_case(LEGAL_HIGH, LEGAL_LOW).expect("the legal control run panicked");
        let legal_columns = columns_of(&legal.rows[SPRITE_Y as usize], SPRITE_COLOR);
        assert!(
            !legal_columns.is_empty(),
            "the legal control sprite never reached the framebuffer: the fixture's \
             display setup does not work, so 'the prohibited sprite is invisible' \
             would hold for the wrong reason"
        );
        assert!(
            legal_columns.contains(&(SPRITE_X as usize)),
            "the legal control sprite was drawn at columns {legal_columns:?}, not at \
             X = {SPRITE_X}: the vendor's X handling is not what the fixture assumes"
        );

        // The case under test.
        let attr0 = (prohibited_high | prohibited_low) as u16;
        let prohibited = run_case(prohibited_high, prohibited_low)
            .unwrap_or_else(|reading| panic!("{}", reading.describe(what, attr0)));

        assert!(
            prohibited.vcount >= COMPARED_ROWS as u16,
            "the frame did not step: VCOUNT only reached {}",
            prohibited.vcount
        );
        assert_eq!(
            prohibited.rows,
            disabled.rows,
            "a prohibited {what} must be treated like an explicitly disabled sprite: \
             the prohibited run drew the sprite at columns {:?}, the disabled run \
             at columns {:?}",
            columns_of(&prohibited.rows[SPRITE_Y as usize], SPRITE_COLOR),
            disabled_columns
        );
    }
}
