//! The backup-chip fixtures — S1-a, defect domain **save-chip model**.
//!
//! | fixture | red/green | what it drives | belongs to |
//! |---|---|---|---|
//! | `eeprom_chip_capacity` | red | ROM carries `EEPROM_V124`; the save path runs | S3-b (E2) |
//! | `flash_status_reg` | red | ROM carries `FLASH_V124`; write `0x0E00_0000`, read `0x0E00_0006` | S3-b (E3) |
//! | `eeprom_protocol_lock` | **green lock** | the EEPROM command protocol itself | S3-b (regression lock) |
//!
//! # The defects (v2.0.3 plan §1.2.4)
//!
//! * **E2 — the chip type and its capacity are not modelled.**
//!   `BackupType::detect` folds `EEPROM_V` (a 512-byte chip) and
//!   `EEPROM_V124` (an 8 KB chip) into one `Eeprom`, and `buffer_size()`
//!   answers `0x8000` for it. A real EEPROM is 512 bytes or 8 KB, so the
//!   `.srm` file is always 32768 bytes no matter which cartridge it came
//!   from.
//! * **E3 — there is no flash status register.** Reading `0x0E00_0006/7`
//!   falls into the plain-read branch and hands back the *save data* instead
//!   of the chip's status, so any commercial cartridge that polls the flash
//!   waits forever.
//!
//! The **protocol** is not a defect: GBATEK and the vendor agree bit for bit
//! (see the plan's §1.7.1). `eeprom_protocol_lock` is therefore a **green
//! lock** — it exists so that the S3-b fix cannot break what already works.
//!
//! # Contract
//!
//! * `eeprom_protocol_lock` asserts the **current** behaviour (it is green
//!   today and must stay green); the other two assert the **target**
//!   behaviour (they are red today).
//! * The EEPROM protocol's bit layout comes from the GBATEK snapshot
//!   (`.zcode/gbatek_plain.txt`, EEPROM command protocol and addressing
//!   sections) — **read the snapshot, never write the bits from memory**.
//! * The expected `.srm` size is derived from the chip the ROM advertises,
//!   not hard-coded per cartridge.
//!
//! # Authority, line by line
//!
//! Every bit-level claim below is quoted from the local GBATEK snapshot
//! (`.zcode/gbatek_plain.txt`, 4.0 MB, fetched 2026-11-08). The line numbers
//! are the snapshot's own.
//!
//! | what | lines | quote |
//! |---|---|---|
//! | EEPROM capacities | `:8665`, `:8669` | "9853 - EEPROM 512 Bytes (0200h) (4Kbit) (eg. used by Super Mario Advance)" / "9854 - EEPROM 8 KBytes (2000h) (64Kbit) (eg. used by Boktai)" |
//! | EEPROM addressing widths | `:8702-8706` | "for 512 Bytes EEPROMS: an address range of 0-3Fh, 6bit bus width; and for 8KByte EEPROMs: a range of 0-3FFh, 14bit bus width" |
//! | EEPROM read request | `:8721-8723` | "2 bits \"11\" (Read Request)" / "n bits eeprom address (MSB first, 6 or 14 bits, depending on EEPROM)" / "1 bit \"0\"" |
//! | EEPROM read answer | `:8739`, `:8750-8751` | "Read a stream of 68 bits from EEPROM by using DMA" / "4 bits - ignore these" / "64 bits - data (conventionally MSB first)" |
//! | EEPROM write request | `:8772-8775` | "2 bits \"10\" (Write Request)" / "n bits eeprom address (MSB first, 6 or 14 bits, depending on EEPROM)" / "64 bits data (conventionally MSB first)" / "1 bit \"0\"" |
//! | EEPROM ready bit | `:8783-8786` | "keep reading from the chip, by normal LDRH [DFFFF00h], until Bit 0 of the returned data becomes \"1\" (Ready)" |
//! | EEPROM window | `:8692-8694` | "On carts with 16MB or smaller ROM, eeprom can be alternately accessed anywhere at D000000h-DFFFFFFh" |
//! | flash write byte | `:8972-8981` | "[E005555h]=AAh, [E002AAAh]=55h, [E005555h]=A0h (write byte command)" / "[E00xxxxh]=dat (write byte to address xxxx)" |
//! | flash data read | `:8898` | "dat=[E00xxxxh] (read byte from address xxxx)" |
//! | ID strings | `:8501-8502`, `:8509-8513` | "The ID string must be located at a word-aligned memory location, the string length should be a multiple of 4 bytes (padded with zero's)" / "EEPROM_Vnnn EEPROM 512 bytes or 8 Kbytes (4Kbit or 64Kbit)" / "FLASH_Vnnn FLASH 64 Kbytes (512Kbit) (ID used in older files)" |
//!
//! ## Two gaps, recorded rather than papered over
//!
//! 1. **The flash status register has no GBATEK authority.** The snapshot
//!    contains no `E000006h` at all (a grep for it over the whole 4.0 MB
//!    file returns zero hits), and GBATEK's flash polling is a read-back and
//!    compare — "wait until [E00xxxxh]=dat" (`:8981`) or "wait until
//!    [E000000h]=FFh" (`:8915`) — not a status register. The address
//!    `0x0E000006/7` and the "ready = bit 0" layout come from the **v2.0.3
//!    plan** (§1.2.4 and §3 S3-b item 2), not from GBATEK. This fixture
//!    asserts what the plan asks for, and **the plan's own wording ("按
//!    GBATEK 的状态位定义返回") is not backed by GBATEK** — that sentence
//!    should be corrected when S3-b starts.
//! 2. **GBATEK does not say which `EEPROM_Vnnn` version means which size.**
//!    Its ID table (`:8509`) says "EEPROM 512 bytes or 8 Kbytes" for the
//!    whole family. The split used here — `EEPROM_V124` = 8 KB, bare
//!    `EEPROM_V` = 512 B — is the **plan's** (§1.2.4), corroborated by the
//!    plan's real cartridges: Super Mario Advance 2 carries `EEPROM_V122`
//!    and GBATEK names Super Mario Advance as the 512-byte user (`:8665`),
//!    while Boktai is the 8 KB user (`:8669`).
//!
//! # Deviations from the plan's wording (recorded, not silent)
//!
//! * **The plan's plant for `flash_status_reg` cannot work as written.** It
//!   says "write `0x00` to `0x0E00_0000`, then read `0x0E00_0006`". On a
//!   flash cartridge a plain byte write does **not** reach the save data: it
//!   enters the chip's command state machine, and in the `Ready` state the
//!   vendor drops it (`internal_memory.rs:755-759`). Even if it landed,
//!   offset 0 is not offset 6, so the read would still see the erased `0xFF`
//!   — whose bit 0 is 1, the very value the ready bit would have, i.e. the
//!   "no tooth" case the plan itself warns about (§2.2). This fixture
//!   therefore plants `0x00` with GBATEK's **write-byte command**
//!   (`:8972-8981`) on the offsets it reads (0..=7), so the byte read at
//!   offset 6 has bit 0 clear, and it reads a **witness byte** at offset 1
//!   to prove the plant landed — a plant that silently fails is then a loud
//!   failure instead of a green test.
//! * **The `.srm` size is read from the fixture's own machine**, not from
//!   the process's one global machine. The export `gba_battery_size`
//!   (`frame.rs:781-792`) answers `machine.gba.cpu.bus.internal_memory
//!   .battery_data().len()`; the fixture evaluates the same expression on
//!   its own machine, because the global one is shared with the rest of the
//!   crate's tests — which serialise on a lock of their own
//!   (`frame.rs:1049`) that a fixture cannot take.
//! * **The chip's commands are driven from the host, not from ARM code.**
//!   The shared encoder set has no `STRB`, no loop, and no way to build the
//!   command port addresses (`0x0E005555` is not an ARM rotated immediate),
//!   so a bit-banged EEPROM transfer cannot be written with it. The host
//!   writes go through the **same bus entry points** a game uses
//!   (`Bus::write_byte` / `Bus::write_half_word`), so the chip model is
//!   exercised exactly as a game's own access would exercise it.
//!
//! # Where the ID string goes
//!
//! At [`ID_OFFSET`], which is word-aligned (GBATEK `:8501`) and below the
//! program body ([`super::CODE_OFFSET`]). The string is zero-padded to a
//! multiple of four bytes (GBATEK `:8502`). GBATEK would rather have it
//! *after* the 0xC0-byte header (`:8532-8533`), but the shared builder
//! starts the program body at `0xC4`, so there is no room after the header;
//! the vendor's `BackupType::detect` scans the whole image, so the position
//! does not change what is detected.

use super::{BUDGET, Cartridge, Fixture, HALT, ldrh, mov_imm, orr_imm, swi};

/// The Game Pak save-memory window. GBATEK writes it `[E00xxxxh]`.
const SAVE_WINDOW: usize = 0x0E00_0000;

/// The flash chip's first command port (GBATEK `[E005555h]`).
const FLASH_COMMAND_1: usize = 0x0E00_5555;

/// The flash chip's second command port (GBATEK `[E002AAAh]`).
const FLASH_COMMAND_2: usize = 0x0E00_2AAA;

/// The EEPROM's window.
///
/// GBATEK `:8694`: "On carts with 16MB or smaller ROM, eeprom can be
/// alternately accessed anywhere at D000000h-DFFFFFFh".
const EEPROM_WINDOW: usize = 0x0D00_0000;

/// Where the cartridge-type ID string goes inside the image.
///
/// Word-aligned (GBATEK `:8501`), and below the program body.
const ID_OFFSET: usize = 0x40;

/// The 8 KB EEPROM chip's capacity, in bytes.
///
/// GBATEK `:8669`: "9854 - EEPROM 8 KBytes (2000h) (64Kbit) (eg. used by
/// Boktai)". Written as a product so the number is derived, not typed.
const EEPROM_8K_BYTES: usize = 8 * 1024;

/// The 512 B EEPROM chip's capacity, in bytes.
///
/// GBATEK `:8665`: "9853 - EEPROM 512 Bytes (0200h) (4Kbit) (eg. used by
/// Super Mario Advance)".
const EEPROM_512_BYTES: usize = 512;

/// The width of the EEPROM's address field, in bits, for the 8 KB chip.
///
/// GBATEK `:8705-8706`: "for 8KByte EEPROMs: a range of 0-3FFh, 14bit bus
/// width (only the lower 10 address bits are used, upper 4 bits should be
/// zero)".
const EEPROM_8K_ADDRESS_BITS: usize = 14;

/// The flash chip's "ready" bit in its status byte: bit 0.
///
/// The plan's §1.2.4 wording. **Not** a GBATEK definition — see the module
/// docs. The chip's byte reaches the register replicated across the lanes
/// (`bus.rs:1644-1645`), so the bit is looked for in lane 0.
const FLASH_STATUS_READY: u32 = 0x0001;

/// The last save-area offset the flash fixture plants.
const PLANT_LAST_OFFSET: usize = 7;

/// The save-area offset the flash fixture reads back to prove the plant
/// landed. It is deliberately **not** an offset the status register shadows.
const PLANT_WITNESS_OFFSET: usize = 1;

/// The block the protocol lock writes and reads back.
///
/// Every byte differs, so a bit-order mistake cannot cancel itself out.
const EEPROM_TEST_DATA: u64 = 0x0123_4567_89AB_CDEF;

/// A cartridge image that advertises `id` as its save chip and runs `body`.
///
/// The ID string is zero-padded to a multiple of four bytes, as GBATEK
/// `:8502` asks.
fn cartridge_with(id: &str, body: &[u32]) -> Vec<u8> {
    let mut bytes = id.as_bytes().to_vec();
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    let mut cartridge = Cartridge::new();
    cartridge.ascii(ID_OFFSET, &bytes);
    cartridge.code(body);
    cartridge.build()
}

/// The `.srm` size this machine's save file would have.
///
/// The export `gba_battery_size` (`frame.rs:781-792`) answers exactly this
/// expression — `machine.gba.cpu.bus.internal_memory.battery_data().len()`.
/// The fixture evaluates it on its **own** machine rather than the process's
/// single global one, which the rest of the crate's tests share under a lock
/// of their own (`frame.rs:1049`).
fn battery_size(fixture: &Fixture) -> usize {
    fixture.gba.cpu.bus.internal_memory.battery_data().len()
}

/// Write one byte into the flash save area, the way GBATEK `:8972-8981`
/// says:
///
/// ```text
///   [E005555h]=AAh, [E002AAAh]=55h, [E005555h]=A0h  (write byte command)
///   [E00xxxxh]=dat                                  (write byte to address xxxx)
/// ```
///
/// The writes go through the same bus entry point a game's store uses, so
/// the chip's command state machine is the one under test.
fn flash_write_byte(fixture: &mut Fixture, offset: usize, value: u8) {
    let bus = &mut fixture.gba.cpu.bus;
    bus.write_byte(FLASH_COMMAND_1, 0xAA);
    bus.write_byte(FLASH_COMMAND_2, 0x55);
    bus.write_byte(FLASH_COMMAND_1, 0xA0);
    bus.write_byte(SAVE_WINDOW + offset, value);
}

/// Read one byte of the flash save area back.
fn flash_save_byte(fixture: &mut Fixture, offset: usize) -> u8 {
    fixture.gba.cpu.bus.read_byte(SAVE_WINDOW + offset)
}

/// Clock one bit into the EEPROM.
fn eeprom_clock_in(fixture: &mut Fixture, bit: bool) {
    let bus = &mut fixture.gba.cpu.bus;
    bus.write_half_word(EEPROM_WINDOW, u16::from(bit));
}

/// Clock one bit out of the EEPROM.
fn eeprom_clock_out(fixture: &mut Fixture) -> bool {
    fixture.gba.cpu.bus.read_half_word(EEPROM_WINDOW) & 1 == 1
}

/// Clock a write request for the 64-bit block at `address`.
///
/// GBATEK `:8772-8775`: "2 bits \"10\" (Write Request)", "n bits eeprom
/// address (MSB first, 6 or 14 bits, depending on EEPROM)", "64 bits data
/// (conventionally MSB first)", "1 bit \"0\"".
fn eeprom_write_block(fixture: &mut Fixture, address: u32, data: u64) {
    eeprom_clock_in(fixture, true);
    eeprom_clock_in(fixture, false);
    for bit in (0..EEPROM_8K_ADDRESS_BITS).rev() {
        eeprom_clock_in(fixture, address & (1 << bit) != 0);
    }
    for bit in (0..64).rev() {
        eeprom_clock_in(fixture, data & (1 << bit) != 0);
    }
    eeprom_clock_in(fixture, false);
}

/// Clock a read request for the 64-bit block at `address`, and collect the
/// answer.
///
/// GBATEK `:8721-8723` for the request, `:8739`/`:8750-8751` for the answer:
/// 68 bits, of which the first four are to be ignored and the next 64 are
/// the data, MSB first.
fn eeprom_read_block(fixture: &mut Fixture, address: u32) -> u64 {
    eeprom_clock_in(fixture, true);
    eeprom_clock_in(fixture, true);
    for bit in (0..EEPROM_8K_ADDRESS_BITS).rev() {
        eeprom_clock_in(fixture, address & (1 << bit) != 0);
    }
    eeprom_clock_in(fixture, false);

    // Four bits the reader is told to ignore (GBATEK `:8750`). Their value is
    // not the reader's business, so it is not asserted.
    for _ in 0..4 {
        eeprom_clock_out(fixture);
    }
    let mut data = 0u64;
    for _ in 0..64 {
        data = (data << 1) | u64::from(eeprom_clock_out(fixture));
    }
    data
}

/// **E2 — the save file must be sized by the chip the ROM advertises.**
///
/// A cartridge whose ROM carries `EEPROM_V124` has an 8 KB EEPROM
/// (GBATEK `:8669`), so its `.srm` is 8192 bytes; one carrying the bare
/// `EEPROM_V` has the 512-byte chip (GBATEK `:8665`), so its `.srm` is 512
/// bytes.
///
/// Today both are 32768: `BackupType::detect` folds the two strings into
/// one `Eeprom` and `buffer_size()` answers `0x8000` for it
/// (`internal_memory.rs:75-98`).
#[test]
#[ignore = "S3-b: 修复后转常驻锁测试"]
fn eeprom_chip_capacity() {
    let mut eight_k = Fixture::new(&cartridge_with("EEPROM_V124", &[swi(HALT)]));
    let mut small = Fixture::new(&cartridge_with("EEPROM_V", &[swi(HALT)]));

    assert!(
        eight_k.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    assert!(
        small.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );

    let eight_k_size = battery_size(&eight_k);
    let small_size = battery_size(&small);

    assert_eq!(
        eight_k_size, EEPROM_8K_BYTES,
        "an `EEPROM_V124` cartridge has an 8 KB EEPROM, so its save file must \
         be {EEPROM_8K_BYTES} bytes"
    );
    assert_ne!(
        eight_k_size, small_size,
        "the two EEPROM chips must not collapse into one size: the chip type \
         is what the size comes from"
    );
    assert_eq!(
        small_size, EEPROM_512_BYTES,
        "a bare `EEPROM_V` cartridge has the 512-byte EEPROM, so its save \
         file must be {EEPROM_512_BYTES} bytes"
    );
}

/// **E3 — reading the flash's status register must not return save data.**
///
/// The plan's recipe is "write `0x00` to `0x0E00_0000`, then read
/// `0x0E00_0006`", and the expectation is "the chip's status (ready = bit
/// 0), not the save byte". Two things make the plan's recipe unusable as
/// written, so this fixture does the corrected version of it:
///
/// * A plain byte write on a flash cartridge does **not** reach the save
///   data — it enters the chip's command state machine, and in the `Ready`
///   state the vendor drops it (`internal_memory.rs:755-759`). The plant
///   therefore uses GBATEK's write-byte command (`:8972-8981`).
/// * The byte has to land on the offset that is read (6), because the
///   erased byte is `0xFF` and `0xFF`'s bit 0 is 1 — the same value the
///   ready bit would have. Offsets 0..=7 are planted with `0x00`, so the
///   byte at offset 6 has bit 0 clear.
///
/// The assertion reads the **register** the program loaded, not a memory
/// round trip.
#[test]
#[ignore = "S3-b: 修复后转常驻锁测试"]
fn flash_status_reg() {
    let body = [
        // r1 = 0x0E00_0000, then OR in the offset: 0x0E00_0006.
        mov_imm(1, 0x0E00_0000),
        orr_imm(1, 1, 0x06),
        // r2 = the halfword the game would read at the status address.
        ldrh(2, 1),
        swi(HALT),
    ];
    let rom = cartridge_with("FLASH_V124", &body);
    let mut fixture = Fixture::new(&rom);

    for offset in 0..=PLANT_LAST_OFFSET {
        flash_write_byte(&mut fixture, offset, 0x00);
    }

    // The plant must have landed: without it the read below could not tell
    // "status" from "data", and the test would pass on the erased byte.
    assert_eq!(
        flash_save_byte(&mut fixture, PLANT_WITNESS_OFFSET),
        0x00,
        "the plant must have landed in the save data, or the status read below \
         cannot tell status from data"
    );

    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    let status = fixture.reg(2);
    assert_eq!(
        status & FLASH_STATUS_READY,
        FLASH_STATUS_READY,
        "reading the flash's status register must answer the chip's status \
         (ready = bit 0), not the save byte that was written there"
    );
}

/// **The EEPROM's command protocol, bit for bit, as GBATEK writes it.**
///
/// This is a **green lock**: the vendor's protocol is already correct (plan
/// §1.2.4, r3), so the test is green today and must stay green — its job
/// is to catch an S3-b fix that breaks what already works.
///
/// The exchange (GBATEK `:8710-8786`):
///
/// ```text
/// write:  2 bits "10" | n bits address (MSB first) | 64 bits data (MSB first) | 1 bit "0"
/// read:   2 bits "11" | n bits address (MSB first) | 1 bit "0"
/// answer: 68 bits = 4 ignored + 64 data (MSB first)
/// ```
///
/// The address field is 14 bits wide, because the chip is the 8 KB one
/// (GBATEK `:8705-8706`).
#[test]
fn eeprom_protocol_lock() {
    let rom = cartridge_with("EEPROM_V124", &[swi(HALT)]);
    let mut fixture = Fixture::new(&rom);
    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );

    // Write block 0.
    eeprom_write_block(&mut fixture, 0, EEPROM_TEST_DATA);
    // GBATEK `:8783-8786`: keep reading until bit 0 says "Ready".
    assert!(
        eeprom_clock_out(&mut fixture),
        "the chip must report Ready after a write"
    );

    // Read block 0 back.
    let data = eeprom_read_block(&mut fixture, 0);
    assert_eq!(
        data, EEPROM_TEST_DATA,
        "the EEPROM must give back the block it was given, bit for bit"
    );
}
