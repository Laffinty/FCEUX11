//! The ROM-window fixtures — S1-a, defect domain **cartridge window**.
//!
//! | fixture | red/green | what it drives | belongs to |
//! |---|---|---|---|
//! | `rom_window_mirror` | red | ROM is `0x200` bytes; read `0x0800_0040` and `0x0800_0240` | S3-d |
//! | `gpio_read_before_write` | **green lock** | ROM carries `EEPROM_V124`; read GPIO `0xC4-0xC9` before and after writing it | S3-c (degraded) |
//!
//! # The defect (v2.0.3 plan §1.2.5)
//!
//! `read_rom` answers an address past the end of the image with **open-bus
//! garbage** — `((address >> 1) & 0xFFFF)` — instead of mirroring the
//! cartridge. A real cartridge of at most 32 MB mirrors itself in
//! `0x08000000-0x09FFFFFF`, so any access beyond its own capacity must read
//! the same bytes again.
//!
//! The GPIO behaviour is **not** a defect: GBATEK does not define what a game
//! reads from `0x080000C4` before it has written the GPIO registers, so the
//! current behaviour ("the ROM byte first, the GPIO value after a write") is
//! **kept as-is** and locked. That is why this fixture is a green lock.
//!
//! # Contract
//!
//! * `rom_window_mirror` asserts the **target** behaviour (red today);
//!   `gpio_read_before_write` asserts the **current** behaviour (green today
//!   and must stay green).
//! * The two reads must be compared against an **absolute golden value**,
//!   not against each other — "the two addresses agree" is a relative
//!   comparison, and a relative comparison cannot see both addresses being
//!   wrong in the same way.
//! * The expected value must be **computed from the bytes the fixture put in
//!   the image**, never written as a literal.
//!
//! # Evidence — the GBATEK snapshot (`.zcode/gbatek_plain.txt`)
//!
//! | what | where | quote |
//! |---|---|---|
//! | the 32 MB GamePak window | `:1352` | `08000000-09FFFFFF   Game Pak ROM/FlashROM (max 32MB) - Wait State 0` |
//! | where the GPIO port lives | `:9218` | `The I/O registers are mapped to a 6-byte region in the ROM-area at 80000C4h` |
//! | the data port | `:9227`, `:9234-9235` | `80000C4h - I/O Port Data (selectable W or R/W)` / `bit0-3  Data Bits 0..3 (0=Low, 1=High)` |
//! | the direction port | `:9243`, `:9250` | `80000C6h - I/O Port Direction (for above Data Port)` / `bit0-3  Direction for Data Port Bits 0..3 (0=In, 1=Out)` |
//! | the control port | `:9259`, `:9266` | `80000C8h - I/O Port Control (selectable W or R/W)` / `bit0    Register 80000C4h..80000C8h Control (0=Write-Only, 1=Read/Write)` |
//! | reads before the game writes | `:9275-9276` | `In write-only mode, reads return 00h (or possible other data, if the rom contains non-zero data at that location).` |
//! | ROM-bus writes are 16/32 bit only | `:9222-9223` | `Observe that ROM-bus writes are limited to 16bit/32bit access (STRB opcodes are ignored; that, only in DS mode?).` |
//! | the save-chip ID string | `:8509` | `EEPROM_Vnnn    EEPROM 512 bytes or 8 Kbytes (4Kbit or 64Kbit)` |
//! | the ID string's placement | `:8501-8502`, `:8532-8533` | `The ID string must be located at a word-aligned memory location, the string length should be a multiple of 4 bytes (padded with zero's).` / `Ideally, for faster detection, the ID should be put into the first some bytes of the ROM-image (ie. somewhere right after the ROM header).` |
//! | the GamePak bus is halfword-addressed, 16 bits wide | `:196486`, `:196503-196504` | `6-21   AD0-15  I/O  lower 16bit Address    and/or  16bit ROM-data (see below)` / `When accessing game pak ROM, a 24bit address is output through AD0-AD15 and A16-A23, then 16bit of data are transferred through AD0-AD15.` |
//!
//! **What the snapshot does *not* say**: it never defines what a game reads
//! from `80000C4h` **before** it has written the GPIO registers — the closest
//! sentence (`:9275-9276`) only covers write-only mode. ⇒ per discipline 4
//! the green lock may only lock the **current** behaviour and must not
//! invent a different expectation.
//!
//! The open-bus **formula** is not in the snapshot either (it is quoted by the
//! vendor from the GBATEK-only page, `internal_memory.rs:388-399`); the
//! snapshot corroborates the bus mechanics it comes from (`:196486`,
//! `:196503-196504`) and that the GBA ROM area does have "open bus values"
//! (`:48678-48679`). ⇒ the fixture copies the formula only as a
//! **precondition** that gives it teeth, never as a claim about hardware.
//!
//! # Where the fixture's own program lives
//!
//! `CODE_OFFSET` is `0xC4` — the same offset `gate.rs` uses — so **the
//! program body sits inside the six bytes the GPIO port occupies**
//! (`0xC4..=0xC9`). That is harmless for the green lock: the "ROM's own
//! bytes" the first read must return are simply the first instruction of the
//! fixture's own program, and the fixture derives the expected value from
//! the image it built instead of from a literal.

use super::{
    BUDGET, Cartridge, Fixture, HALT, ROM_BASE, ROM_LEN, ldr, ldrh, mov_imm, orr_imm, strh, swi,
};

// ---------------------------------------------------------------------------
// The ROM window fixture (S3-d)
// ---------------------------------------------------------------------------

/// Where the golden bytes sit inside the cartridge image.
///
/// Below `CODE_OFFSET` (`0xC4`): the builder refuses data that reaches into
/// the code region.
const GOLDEN_OFFSET: usize = 0x40;

/// The four bytes the fixture plants at [`GOLDEN_OFFSET`].
///
/// `GOLD` reads as itself in a hex dump, so a wrong answer is obvious in a
/// failure message.
const GOLDEN_BYTES: [u8; 4] = *b"GOLD";

/// The offset of the **mirrored** address: one whole image further on.
///
/// The image is [`ROM_LEN`] bytes, so `0x0800_0240` is `0x0800_0040` plus one
/// whole cartridge: past the end of the cartridge, but still inside the 32 MB
/// GamePak window (`08000000-09FFFFFF`, GBATEK `:1352`) — which is where a
/// real cartridge mirrors itself.
const MIRROR_OFFSET: usize = ROM_LEN + GOLDEN_OFFSET;

/// The word the cartridge must answer with at **both** addresses.
///
/// Assembled from the bytes the fixture planted, little-endian — the GBA is
/// little-endian, so the byte at the lowest address is the least significant
/// one. Never a literal: the expected value has to follow the bytes.
fn golden_word() -> u32 {
    u32::from_le_bytes(GOLDEN_BYTES)
}

/// The word the vendor's open-bus fallback answers for a **word** read at
/// `offset` inside the ROM window.
///
/// The GamePak bus is halfword-addressed and 16 bits wide (GBATEK `:196486`,
/// `:196503-196504`); the vendor's fallback puts the low 16 bits of the
/// halfword address on the data lines and lets the CPU pick the byte its
/// address's LSB selects (`internal_memory.rs:397-403`).
///
/// **This is not a claim about real hardware** — the snapshot does not state
/// the formula. It is the **precondition** that gives the fixture teeth: if
/// the value the defect produces were the golden one, the fixture could not
/// tell the two apart. The copy is deliberately independent of the vendor's
/// code, so the fixture keeps working once the defect is fixed.
fn open_bus_word(offset: usize) -> u32 {
    let byte_at = |address: usize| -> u8 {
        let halfword_address = ((address >> 1) & 0xFFFF) as u16;
        if address & 1 == 0 {
            (halfword_address & 0xFF) as u8
        } else {
            (halfword_address >> 8) as u8
        }
    };
    u32::from_le_bytes([
        byte_at(offset),
        byte_at(offset + 1),
        byte_at(offset + 2),
        byte_at(offset + 3),
    ])
}

/// A cartridge of at most 32 MB mirrors itself in the GamePak window, so the
/// two addresses must answer with the **same absolute golden word**.
///
/// The image is `ROM_LEN` (`0x200`) bytes. `0x0800_0040` is inside it;
/// `0x0800_0240` is one whole image further on, so it is past the end of the
/// cartridge but still inside the 32 MB window. The vendor answers the second
/// address with open-bus garbage instead (`read_rom`,
/// `internal_memory.rs:377-404`) ⇒ this fixture is **red** today, and becomes
/// a permanent lock once S3-d is fixed.
#[test]
#[ignore = "S3-d: 修复后转常驻锁测试"]
fn rom_window_mirror() {
    let mut cartridge = Cartridge::new();
    cartridge.bytes(GOLDEN_OFFSET, &GOLDEN_BYTES);

    let body = [
        // r0 = the GamePak window's base.
        mov_imm(0, ROM_BASE),
        // r1 = the address inside the cartridge, r2 = the mirrored one.
        orr_imm(1, 0, GOLDEN_OFFSET as u32),
        orr_imm(2, 0, MIRROR_OFFSET as u32),
        // r3 = the word read inside the cartridge, r4 = the mirrored read.
        ldr(3, 1),
        ldr(4, 2),
        swi(HALT),
    ];
    cartridge.code(&body);
    let rom = cartridge.build();

    let mut fixture = Fixture::new(&rom);
    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );

    let golden = golden_word();
    assert_ne!(
        open_bus_word(MIRROR_OFFSET),
        golden,
        "the open-bus value must differ from the golden one, or this fixture \
         could not see the defect at all"
    );

    assert_eq!(
        fixture.reg(3),
        golden,
        "the read inside the cartridge must answer with the bytes the image holds"
    );
    assert_eq!(
        fixture.reg(4),
        golden,
        "a {ROM_LEN} byte cartridge mirrors itself in the 32 MB GamePak window: \
         the read at 0x{:08X} must answer with the same bytes as the read at \
         0x{:08X}",
        ROM_BASE + MIRROR_OFFSET as u32,
        ROM_BASE + GOLDEN_OFFSET as u32
    );
}

// ---------------------------------------------------------------------------
// The GPIO fixture (S3-c, degraded to a green lock)
// ---------------------------------------------------------------------------

/// The save-chip marker the cartridge advertises.
///
/// GBATEK's `ID Strings` section (`:8497-8513`) lists `EEPROM_Vnnn` for
/// "EEPROM 512 bytes or 8 Kbytes" and asks for a **word-aligned** location
/// whose length is a multiple of four bytes, zero-padded (`:8501-8502`);
/// `:8532-8533` suggests the first bytes of the image. `0x40` is
/// word-aligned and inside the header the builder reserves, so the marker
/// goes there.
const EEPROM_ID: &[u8] = b"EEPROM_V124\0";

/// Where the marker sits in the image.
const EEPROM_ID_OFFSET: usize = 0x40;

/// The offset of the GPIO data port (`80000C4h`, GBATEK `:9227`).
const GPIO_DATA_OFFSET: u32 = 0xC4;

/// The offset of the GPIO direction port (`80000C6h`, GBATEK `:9243`).
const GPIO_DIRECTION_OFFSET: u32 = 0xC6;

/// The offset of the GPIO control port (`80000C8h`, GBATEK `:9259`).
const GPIO_CONTROL_OFFSET: u32 = 0xC8;

/// What the program writes to the direction port: every pin an output.
///
/// Bit 1 matters: the vendor only mixes the RTC's SIO line into the data
/// port while the direction says that pin is an **input**
/// (`internal_memory.rs:352-357`), so marking the pins as outputs keeps the
/// data port's read deterministic.
const GPIO_DIRECTION_VALUE: u16 = 0b0000_1111;

/// What the program writes to the control port: bit 0 = read/write.
///
/// GBATEK `:9266`: `bit0    Register 80000C4h..80000C8h Control (0=Write-Only,
/// 1=Read/Write)`.
const GPIO_CONTROL_VALUE: u16 = 0b0000_0001;

/// What the GPIO data port answers once the game has written the registers.
///
/// Assembled from the port's fields rather than written as a number: GBATEK
/// gives the data port four bits (`80000C4h - I/O Port Data`, `bit0-3  Data
/// Bits 0..3 (0=Low, 1=High)`, `:9227-9235`), the program never writes that
/// port, and the direction write marks the pins as outputs, so the vendor
/// does not mix the RTC's SIO line into it (`internal_memory.rs:352-357`).
///
/// The width is the register's own (16 bits), widened to the register file's
/// width so that the comparison is between two values of the same width.
fn gpio_data_register_value() -> u32 {
    let data_bits = 0b0000_1111u16;
    let written_by_the_game = 0b0000u16;
    let rtc_sio_line = 0b0000u16;
    u32::from((written_by_the_game & data_bits) | rtc_sio_line)
}

/// The GPIO region's behaviour before the game writes it — a **green lock**.
///
/// **This locks the current behaviour and nothing else.** GBATEK does not
/// define what a game reads from `80000C4h` before it has written the GPIO
/// registers, so asserting any other expectation would be inventing a
/// specification (discipline 4).
///
/// Today the vendor answers the **first** read with the ROM's own bytes — the
/// `gpio_present` flag starts `false` (`internal_memory.rs:222`) and only a
/// write into `0xC4..=0xC9` sets it (`:688-689`) — and answers the **second**
/// read with the GPIO registers, because by then the game has written the
/// direction and control ports.
#[test]
fn gpio_read_before_write() {
    let mut cartridge = Cartridge::new();
    cartridge.ascii(EEPROM_ID_OFFSET, EEPROM_ID);

    let body = [
        // r0 = the GamePak window's base; r1..r3 = the three GPIO ports.
        mov_imm(0, ROM_BASE),
        orr_imm(1, 0, GPIO_DATA_OFFSET),
        orr_imm(2, 0, GPIO_DIRECTION_OFFSET),
        orr_imm(3, 0, GPIO_CONTROL_OFFSET),
        // r4 = the read **before** the game writes anything.
        ldrh(4, 1),
        // The game writes the direction and control ports. Halfword writes,
        // because GBATEK `:9222-9223` says ROM-bus writes are limited to
        // 16bit/32bit access.
        mov_imm(5, GPIO_DIRECTION_VALUE as u32),
        strh(5, 2),
        mov_imm(6, GPIO_CONTROL_VALUE as u32),
        strh(6, 3),
        // r7 = the read **after** the writes; r8/r9 = the two written ports.
        ldrh(7, 1),
        ldrh(8, 2),
        ldrh(9, 3),
        swi(HALT),
    ];
    cartridge.code(&body);
    let rom = cartridge.build();

    let mut fixture = Fixture::new(&rom);
    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );

    // The ROM's own halfword at the data port's offset: the fixture's program
    // body starts at `0xC4`, so these are the first instruction's bytes.
    // Widened to the register file's width, so the two sides of every
    // comparison below have the same width.
    let data_offset = GPIO_DATA_OFFSET as usize;
    let rom_halfword = u32::from(u16::from_le_bytes([rom[data_offset], rom[data_offset + 1]]));

    assert_eq!(
        fixture.reg(4),
        rom_halfword,
        "before the game writes the GPIO registers, the read must answer with \
         the ROM's own bytes"
    );
    assert_eq!(
        fixture.reg(7),
        gpio_data_register_value(),
        "after the game writes the GPIO registers, the read must answer with \
         the GPIO data port"
    );
    assert_eq!(
        fixture.reg(8),
        u32::from(GPIO_DIRECTION_VALUE),
        "the direction port must answer with the value the game wrote"
    );
    assert_eq!(
        fixture.reg(9),
        u32::from(GPIO_CONTROL_VALUE),
        "the control port must answer with the value the game wrote"
    );
    assert_ne!(
        fixture.reg(4),
        fixture.reg(7),
        "the two reads must differ, or this fixture could not see the change"
    );
}
