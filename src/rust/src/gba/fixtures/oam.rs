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
//!   ⚠️ **S2-a 补记（v2.0.3 r7）**：`oam_bad_shape` 与 `oam_gfx_mode3` 已随
//!   S2-a 的修复摘掉 `#[ignore]` 转为**常驻绿锁**（vendor 把禁止值映射为
//!   `ObjMode::Disabled`，目标行为成立）；其余复现器的 `#[ignore]` 不变，
//!   守卫 G4 的复现器集合相应从 7 条减到 5 条。
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
    /// Resident since S2-a (v2.0.3 r7): the vendor now maps shape = 3 onto
    /// `ObjMode::Disabled`, so the target behaviour holds and this lock runs
    /// in the gate instead of waiting behind an `#[ignore]`.
    #[test]
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
    /// Resident since S2-a (v2.0.3 r7), like [`oam_bad_shape`].
    #[test]
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

// ---------------------------------------------------------------------------
// OBJ 瓦片窗口跨度夹具（OBJ 越界 panic 修复，2026-11-21）
//
// 个案 B（0004 Kurukuru Kururin / 0015 Fire Pro Wrestling A）钉死的 vendor
// 缺陷：OBJ 瓦片读取 4 处直索引 `video_ram[0x10000 + tile_data_offset]`，
// 精灵 tile 跨过 OBJ 窗口（0x8000 字节界）时数组越界 panic；release
// panic="abort" ⇒ 进程死亡（业主所见「卡死/白屏」）。真机行为 = 总线把
// 0x06018000-0x0601FFFF 镜像到 0x06010000-0x06017FFF（gbadoc VRAM 节一手
// 原文；GBATEK 快照 :439 只标 "Not used"，无读语义）⇒ 越窗读回绕。
//
// 四条腿各锁一处读取点（防「修一处漏三处」）：
//
//   | 腿 | 锁的读取点 | 精灵形态 |
//   |---|---|---|
//   | render 4bpp 1D | 渲染路径 4bpp | 0004 同款：1D 64×32 tile=1004 |
//   | render 8bpp 1D | 渲染路径 8bpp | 同基座 8bpp |
//   | winobj 4bpp 2D | OBJ 窗口掩码路径 4bpp | 0015 同款：2D 64×32 tile=992 |
//   | winobj 8bpp 1D | OBJ 窗口掩码路径 8bpp | 同基座 8bpp |
//
// 断言三重：① 渲染全程无 panic（缺陷本体）；② 越窗像素读的是**回绕后**
// 的数据（「越窗读零」类变异可红）；③ 回绕到受保护 tile 0 的段落保持透明
// （区分镜像与「全不透明」的假实现）。期望值全部按位段手工推导（纪律 5），
// 不调用被测公式。
#[cfg(test)]
mod window_span {
    use super::{
        BUDGET, Cartridge, Fixture, HALT, OBJ_PALETTE_ADDR, OBJ_PALETTE_WORD, OAM_BASE,
        SCREEN_WIDTH, VCOUNT_ADDR, catch_panic_reading, swi,
    };

    /// 64×32 = Horizontal + Size 3（GBATEK 尺寸表；0004/0015 的精灵形态）。
    const ATTR0_SHAPE_HORIZONTAL: u32 = 0x4000;
    const ATTR1_SIZE_64X32: u32 = 0xC000;
    /// attr0 bit 13 = 8bpp。
    const ATTR0_8BPP: u32 = 0x2000;
    /// gfx_mode = 2（OBJ Window，attr0 bits 10-11）：窗口精灵本体不渲染，
    /// 只把它的不透明像素标进 OBJ 窗口区（GBATEK 快照 :1051-1054）。
    const ATTR0_GFX_MODE_OBJ_WINDOW: u32 = 0x0800;
    /// DISPCNT 位段（GBATEK 快照 :708-727 钉死）：bit 6 = OBJ 1D 映射、
    /// bit 8 = BG0、bit 12 = OBJ、bit 15 = OBJ window display flag。
    const DISPCNT_1D: u32 = 0x40;
    const DISPCNT_BG0: u32 = 0x100;
    const DISPCNT_OBJ_WINDOW_FLAG: u32 = 0x8000;
    /// WINOUT（040004Ah，GBATEK 快照 :1038-1050）：bits 0-3 = 窗口外
    /// BG0-BG3、bits 8-11 = OBJ 窗口内 BG0-BG3。窗口外只开 BG0、OBJ 窗口
    /// 内全关 ⇒ 被窗口遮住的像素露 backdrop，掩码路径从帧缓冲可观测。
    /// ⚠️ 字面量必须是 8 位有效数字：`0x0400_04A` = 0x4004A（错），要写
    /// `0x0400_004A`——「分组看着对、少一位」这类错不报警（写进匹配黑洞
    /// 静默消失），2026-11-21 实测踩过一次。
    const WINOUT_ADDR: usize = 0x0400_004A;
    const WINOUT_OUTSIDE_BG0_ONLY: u32 = 0x0001;
    const BG0CNT_ADDR: usize = 0x0400_0008;
    /// BG0CNT = SBB 8（本仓 screen-base 惯例 ⇒ 0x06004000）、char base 0、
    /// size 0（32×32）、4bpp。
    const BG0CNT_SBB8: u32 = 0x0800;
    const BG0_MAP_ADDR: usize = 0x0600_4000;
    const BG0_TILE1_ADDR: usize = 0x0600_0020;
    const BG_PALETTE_ENTRY1_WORD: u32 = 0x7FFF_0000;
    /// 4bpp 皆可的颜色指数 1（index 0 透明），一字铺 8 像素。
    ///
    /// ⚠️ 仅用于 **4bpp** 瓦片：8bpp 瓦片的字节**就是**调色板指数——铺 0x11
    /// 会得到指数 17 而不是 1（未植入调色板 = 黑）。8bpp 腿用 [`OPAQUE_WORD_8BPP`]。
    const OPAQUE_WORD: u32 = 0x1111_1111;
    /// 8bpp 瓦片用：每字节 = 调色板指数 1。
    const OPAQUE_WORD_8BPP: u32 = 0x0101_0101;
    const COLOR_OPAQUE: u16 = 0x7FFF;
    const COLOR_BACKDROP: u16 = 0x0000;
    /// 精灵摆放：避开 OAM 清零留下的 127 个 (0,0) tile-0 精灵。
    const SPRITE_X: u32 = 40;
    const SPRITE_Y: u32 = 20;

    fn plant_words(fixture: &mut Fixture, start: usize, byte_len: usize, word: u32) {
        for i in 0..byte_len / 4 {
            fixture.write_word(start + i * 4, word);
        }
    }

    fn plant_oam(fixture: &mut Fixture, attr0: u32, attr1: u32, attr2: u32) {
        fixture.write_word(OAM_BASE as usize, (attr1 << 16) | attr0);
        fixture.write_word(OAM_BASE as usize + 4, attr2);
    }

    /// BG0 全屏白 + WINOUT「窗外 BG0、OBJ 窗内全关」：窗口掩码像素露 backdrop。
    ///
    /// ⚠️ WININ/WINOUT 是 0400048h 起的两个半字寄存器，而 `write_word` 会把
    /// 非对齐地址向下对齐——直接写 040004Ah 会落进 WININ。所以一字写覆盖
    /// 两者：低半字 = WININ（Win0 未启用，值无关），高半字 = WINOUT。
    fn plant_bg0_backdrop_split(fixture: &mut Fixture) {
        fixture.write_word(BG0CNT_ADDR, BG0CNT_SBB8);
        for i in 0..512 {
            fixture.write_word(BG0_MAP_ADDR + i * 4, 0x0001_0001);
        }
        plant_words(fixture, BG0_TILE1_ADDR, 32, OPAQUE_WORD);
        fixture.write_word(0x0500_0000, BG_PALETTE_ENTRY1_WORD);
        fixture.write_word(WINOUT_ADDR - 2, 0x0001_0001);
    }

    fn run_until_vcount(fixture: &mut Fixture, target: u16) {
        for _ in 0..300 {
            fixture.run_steps(1232);
            if fixture.read_half_word(VCOUNT_ADDR) >= target {
                return;
            }
        }
        panic!("the LCD never reached VCOUNT {target}");
    }

    fn frame_row_at(fixture: &Fixture, y: usize) -> Vec<u16> {
        let buffer = &fixture.gba.cpu.bus.lcd.buffer;
        (0..SCREEN_WIDTH).map(|x| buffer[y][x].0).collect()
    }

    fn expect_columns(row: &[u16], x0: usize, x1: usize, color: u16, what: &str) {
        for x in x0..=x1 {
            assert_eq!(
                row[x], color,
                "{what}: column {x} expected {color:#06X}, got {:#06X}",
                row[x]
            );
        }
    }

    /// 跑一个跨度用例：植入显示面与 OAM，跑到精灵最后一行之后；panic 一律
    /// 转成带读数的测试失败（缺陷本体 = panic，腿 1 就靠它转红）。
    fn run_span_case(
        dispcnt: u32,
        attr0: u32,
        attr1: u32,
        attr2: u32,
        plant: impl FnOnce(&mut Fixture),
    ) -> Fixture {
        let mut cartridge = Cartridge::new();
        cartridge.code(&[swi(HALT)]);
        let rom = cartridge.build();
        let mut fixture = Fixture::new(&rom);
        fixture.write_word(super::DISPCNT_ADDR, dispcnt);
        plant(&mut fixture);
        plant_oam(&mut fixture, attr0, attr1, attr2);
        catch_panic_reading(move || {
            let _ = fixture.run_to_halt(BUDGET);
            run_until_vcount(&mut fixture, SPRITE_Y as u16 + 32);
            fixture
        })
        .unwrap_or_else(|reading| {
            panic!(
                "the vendor core panicked during the window-span render\n  \
                 message: {}",
                reading.message
            )
        })
    }

    /// 【腿 1 · 渲染路径 4bpp】0004 同款：1D 64×32 tile=1004。
    ///
    /// 瓦片跨度 1004..=1035：r=0/r=1（tile 1004-1019）在窗内；r=2 的
    /// tile 1020-1023 在窗内、1024-1027 越窗 → 回绕到窗口 tile 0-3；
    /// r=3 的 tile 1028-1035 越窗 → 回绕到窗口 tile 4-11。窗口 tile 0
    /// 保留全零（既是回绕目标，也是 OAM 清零精灵的瓦片）⇒ r=2 的第 5 个
    /// d-tile 透明——这段把「镜像」与「越窗读零」区分开。
    #[test]
    fn obj_tile_span_1d_4bpp_reads_the_window_alias() {
        let fixture = run_span_case(
            super::DISPCNT_OBJ_ENABLE | DISPCNT_1D,
            ATTR0_SHAPE_HORIZONTAL | SPRITE_Y,
            ATTR1_SIZE_64X32 | SPRITE_X,
            1004,
            |fixture| {
                // 窗内：tile 1004-1023 = flat 0x17CE0-0x17FFF。
                plant_words(fixture, 0x0601_7CE0, 0x320, OPAQUE_WORD);
                // 越窗回绕目标：窗口 tile 1-15（0x20-0x1FF）；tile 0 保留全零。
                plant_words(fixture, 0x0601_0020, 0x1E0, OPAQUE_WORD);
                fixture.write_word(OBJ_PALETTE_ADDR, OBJ_PALETTE_WORD);
            },
        );
        expect_columns(&frame_row_at(&fixture, 25), 40, 103, COLOR_OPAQUE, "r=0");
        expect_columns(&frame_row_at(&fixture, 29), 40, 103, COLOR_OPAQUE, "r=1");
        let r2 = frame_row_at(&fixture, 38);
        expect_columns(&r2, 40, 71, COLOR_OPAQUE, "r=2 in-window d-tiles 0-3");
        expect_columns(&r2, 72, 79, COLOR_BACKDROP, "r=2 aliased onto tile 0");
        expect_columns(&r2, 80, 103, COLOR_OPAQUE, "r=2 wrapped past tile 0");
        expect_columns(&frame_row_at(&fixture, 47), 40, 103, COLOR_OPAQUE, "r=3");
    }

    /// 【腿 2 · 渲染路径 8bpp】同基座 8bpp：s-tile 计数，跨窗位置不同。
    ///
    /// 1D 8bpp 的 d-tile k = s-tiles 1004+16r+2k/+2k+1。r=1 的 d-tile 2 =
    /// s-tiles 1024/1025 → 回绕到窗口 s-tiles 0/1：上半（s-tile 0 = 受保护
    /// 的 tile 0 区）透明、下半（s-tile 1 = 0x20-0x3F，已植入）不透明 ——
    /// 同一个 d-tile 上下半各一态，是镜像语义最锐的断言。
    #[test]
    fn obj_tile_span_1d_8bpp_reads_the_window_alias() {
        let fixture = run_span_case(
            super::DISPCNT_OBJ_ENABLE | DISPCNT_1D,
            ATTR0_SHAPE_HORIZONTAL | ATTR0_8BPP | SPRITE_Y,
            ATTR1_SIZE_64X32 | SPRITE_X,
            1004,
            |fixture| {
                plant_words(fixture, 0x0601_7CE0, 0x320, OPAQUE_WORD_8BPP);
                // 回绕目标：窗口 0x20-0x57F 植入（r=3 最远到 s-tile 1066 =
                // 0x8560+31 → 回绕 0x57F）；0x00-0x1F（tile 0）保留全零。
                plant_words(fixture, 0x0601_0020, 0x560, OPAQUE_WORD_8BPP);
                fixture.write_word(OBJ_PALETTE_ADDR, OBJ_PALETTE_WORD);
            },
        );
        expect_columns(&frame_row_at(&fixture, 25), 40, 103, COLOR_OPAQUE, "r=0");
        let r1a = frame_row_at(&fixture, 28);
        expect_columns(&r1a, 40, 55, COLOR_OPAQUE, "r=1 d-tiles 0-1 in-window");
        expect_columns(&r1a, 56, 63, COLOR_BACKDROP, "r=1 d-tile 2 top half");
        expect_columns(&r1a, 64, 103, COLOR_OPAQUE, "r=1 d-tiles 3-7 wrapped");
        expect_columns(&frame_row_at(&fixture, 32), 40, 103, COLOR_OPAQUE, "r=1 d-tile 2 bottom");
        expect_columns(&frame_row_at(&fixture, 47), 40, 103, COLOR_OPAQUE, "r=3");
    }






    /// 【腿 3 · OBJ 窗口掩码路径 4bpp】0015 同款：2D 64×32 tile=992。
    ///
    /// 2D：tile_offset = y*32 + x → r=1 的 tile 1024-1031、r=3 的
    /// 1088-1095 全部越窗，分别回绕到窗口 tile 0-7 / 32-39。窗口精灵不渲染
    /// 本体，但它的不透明像素把 OBJ 窗口区内的 BG0 遮掉（WINOUT 窗内全关
    /// ⇒ 露 backdrop）⇒ 掩码路径的执行与越窗读数都从帧缓冲可观测。
    #[test]
    fn obj_window_span_2d_4bpp_masks_through_the_alias() {
        let fixture = run_span_case(
            DISPCNT_BG0 | super::DISPCNT_OBJ_ENABLE | DISPCNT_OBJ_WINDOW_FLAG,
            ATTR0_GFX_MODE_OBJ_WINDOW | SPRITE_Y,
            ATTR1_SIZE_64X32 | SPRITE_X,
            992,
            |fixture| {
                plant_bg0_backdrop_split(fixture);
                // 窗内：tile 992-999 = flat 0x17C00-0x17DFF。
                plant_words(fixture, 0x0601_7C00, 0x200, OPAQUE_WORD);
                // 回绕（2D，tile_offset = y*32 + x）：r=1 → tile 1024-1031 =
                // 窗口 tile 0-7（0x000-0x0FF，tile 0 保留全零）；r=2 →
                // tile 1056-1063 = 窗口 tile 32-39（0x400-0x4FF）；r=3 →
                // tile 1088-1095 = 窗口 tile 64-71（0x800-0x8FF）。
                plant_words(fixture, 0x0601_0020, 0xE0, OPAQUE_WORD);
                plant_words(fixture, 0x0601_0400, 0x100, OPAQUE_WORD);
                plant_words(fixture, 0x0601_0800, 0x100, OPAQUE_WORD);
            },
        );
        // 窗口外：BG0 白 —— 正向对照（BG0 管线与 WINOUT 都在工作）。
        expect_columns(&frame_row_at(&fixture, 10), 0, 239, COLOR_OPAQUE, "outside sprite");
        expect_columns(&frame_row_at(&fixture, 25), 40, 103, COLOR_BACKDROP, "r=0");
        let r1 = frame_row_at(&fixture, 29);
        expect_columns(&r1, 40, 47, COLOR_OPAQUE, "r=1 d-tile 0 = aliased onto tile 0");
        expect_columns(&r1, 48, 103, COLOR_BACKDROP, "r=1 d-tiles 1-7");
        expect_columns(&frame_row_at(&fixture, 45), 40, 103, COLOR_BACKDROP, "r=3");
    }

    /// 【腿 4 · OBJ 窗口掩码路径 8bpp】同基座 8bpp，掩码路径的越窗读。
    #[test]
    fn obj_window_span_1d_8bpp_masks_through_the_alias() {
        let fixture = run_span_case(
            DISPCNT_BG0 | super::DISPCNT_OBJ_ENABLE | DISPCNT_OBJ_WINDOW_FLAG | DISPCNT_1D,
            ATTR0_GFX_MODE_OBJ_WINDOW | ATTR0_8BPP | SPRITE_Y,
            ATTR1_SIZE_64X32 | SPRITE_X,
            1004,
            |fixture| {
                plant_bg0_backdrop_split(fixture);
                plant_words(fixture, 0x0601_7CE0, 0x320, OPAQUE_WORD);
                plant_words(fixture, 0x0601_0020, 0x560, OPAQUE_WORD);
            },
        );
        expect_columns(&frame_row_at(&fixture, 10), 0, 239, COLOR_OPAQUE, "outside sprite");
        expect_columns(&frame_row_at(&fixture, 25), 40, 103, COLOR_BACKDROP, "r=0");
        let r1 = frame_row_at(&fixture, 28);
        expect_columns(&r1, 40, 55, COLOR_BACKDROP, "r=1 d-tiles 0-1 in-window");
        expect_columns(&r1, 56, 63, COLOR_OPAQUE, "r=1 d-tile 2 top half");
        expect_columns(&r1, 64, 103, COLOR_BACKDROP, "r=1 d-tiles 3-7 wrapped");
        expect_columns(&frame_row_at(&fixture, 32), 40, 103, COLOR_BACKDROP, "r=1 d-tile 2 bottom");
        expect_columns(&frame_row_at(&fixture, 47), 40, 103, COLOR_BACKDROP, "r=3");
    }
}
