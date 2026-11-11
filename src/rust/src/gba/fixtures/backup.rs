//! The backup-chip fixtures — S1-a, defect domain **save-chip model**.
//!
//! | fixture | red/green | what it drives | belongs to |
//! |---|---|---|---|
//! | `eeprom_chip_capacity` | **green lock** | one `EEPROM_V124` cart and one bare `EEPROM_V` cart; the access width decides the size | S3-b (E2, fixed) |
//! | `flash_status_reg` | **green lock** | ROM carries `FLASH_V124`; plant `0x00`, read `0x0E000006/7` back | S3-b (E3, resolved) |
//! | `eeprom_protocol_lock` | **green lock** | the EEPROM command protocol itself | S3-b (regression lock) |
//!
//! # The defects (v2.0.3 plan §1.2.4)
//!
//! * **E2 — was "the chip type and its capacity are not modelled"; fixed by
//!   the dynamic-capacity model.** The buffer used to be a flat 32 KB for
//!   every EEPROM cartridge. The ID string cannot carry the capacity (GBATEK
//!   `:8521` — "EEPROM_Vnnn EEPROM 512 bytes or 8 Kbytes", `nnn` being a
//!   library version), so the buffer now starts at the smallest real chip,
//!   512 B (GBATEK `:8665`), and grows to the 8 KB ceiling (GBATEK `:8669`)
//!   the first time a transfer addresses past it — mGBA's `_ensureEeprom`
//!   semantics (`savedata.c:471-489`). The fixture below pins the observable
//!   consequence: a 6-bit transfer leaves the file at 512 B, a 14-bit
//!   transfer past the low half grows it to 8 KB with the block addressable
//!   there, and the bare-`EEPROM_V` cartridge behaves exactly like
//!   `EEPROM_V124` — the string decides "is an EEPROM", never "how big".
//! * **E3 — was "there is no flash status register"; resolved by RE.** The
//!   plan expected one, but four independent sources (GBATEK's whole FLASH
//!   chapter, Nintendo's in-cartridge SDK driver, mGBA, VBA-M — see the
//!   `flash_status_reg` doc below) agree GBA flash has **no** status-register
//!   command, and that what a game polls is the written data itself, read
//!   back and compared. The vendor's plain-read behaviour was therefore
//!   already right, and the reproducer became a **green lock** on that
//!   read-back semantics instead of a fix.
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
//! ## Two gaps — the first has been closed by the RE round
//!
//! 1. **The flash status register: falsified and re-locked.** The plan's
//!    status-register reading was **falsified** by the S3-b RE (four
//!    independent sources, listed in the `flash_status_reg` doc below): the
//!    snapshot contains no `E000006h` at all (a grep over the whole 4.0 MB
//!    file returns zero hits), GBATEK's flash polling is a read-back and
//!    compare — "wait until [E00xxxxh]=dat" (`:8981`) or "wait until
//!    [E000000h]=FFh" (`:8915`) — and Nintendo's own SDK driver polls with a
//!    plain array read. The fixture originally asserted the plan's
//!    "ready = bit 0" wording, which had no authority behind it; it now
//!    asserts the **read-back semantics** the four sources agree on, as a
//!    resident green lock.
//! 2. **GBATEK does not say which `EEPROM_Vnnn` version means which size.**
//!    Its ID table (`:8509`) says "EEPROM 512 bytes or 8 Kbytes" for the
//!    whole family. The original fixture split them — `EEPROM_V124` = 8 KB,
//!    bare `EEPROM_V` = 512 B — on the plan's word, and that split was
//!    self-contradictory: Super Mario Advance 2 carries `EEPROM_V122` while
//!    GBATEK names Super Mario Advance as the 512-byte user (`:8665`). The
//!    RE round settled it the other way: neither reference simulator reads
//!    the version (mGBA defaults to 512 B and grows on the first high
//!    access, `savedata.c:291-315`; VBA-M watches the bit-stream width). The
//!    lock now pins **that** behaviour, with a leg on the bare-`EEPROM_V`
//!    cartridge that goes red if anyone ever reintroduces a version→size
//!    mapping.
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

/// The width of the EEPROM's address field, in bits, for the 512 B chip.
///
/// GBATEK `:8702-8704`: "for 512 Bytes EEPROMS: an address range of 0-3Fh,
/// 6bit bus width".
const EEPROM_512_ADDRESS_BITS: usize = 6;

/// The byte the flash fixture plants over the offsets it reads back.
///
/// `0x00` on purpose: it is neither the erased value `0xFF` nor any bit
/// pattern a "status register" reading could be confused with, so a
/// read-back at `0x0E000006` can only be the bytes themselves. The chip's
/// byte reaches the reader replicated across the 8-bit bus's lanes
/// (`bus.rs:1643-1646`), so the expected halfword is this byte times
/// `0x0101`.
const PLANT_BYTE: u8 = 0x00;

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

/// Clock a write request for the 64-bit block at `address`, with the
/// address field `address_bits` wide (6 for the 512 B chip, 14 for 8 KB).
///
/// GBATEK `:8772-8775`: "2 bits \"10\" (Write Request)", "n bits eeprom
/// address (MSB first, 6 or 14 bits, depending on EEPROM)", "64 bits data
/// (conventionally MSB first)", "1 bit \"0\"".
///
/// The command only lands when something reads the chip afterwards — the
/// write is interpreted on the next clock-out — so a caller that wants the
/// write committed asserts a `Ready` clock-out first.
fn eeprom_write_block(fixture: &mut Fixture, address: u32, data: u64, address_bits: usize) {
    eeprom_clock_in(fixture, true);
    eeprom_clock_in(fixture, false);
    for bit in (0..address_bits).rev() {
        eeprom_clock_in(fixture, address & (1 << bit) != 0);
    }
    for bit in (0..64).rev() {
        eeprom_clock_in(fixture, data & (1 << bit) != 0);
    }
    eeprom_clock_in(fixture, false);
}

/// Clock a read request for the 64-bit block at `address` (address field
/// `address_bits` wide), and collect the answer.
///
/// GBATEK `:8721-8723` for the request, `:8739`/`:8750-8751` for the answer:
/// 68 bits, of which the first four are to be ignored and the next 64 are
/// the data, MSB first.
fn eeprom_read_block(fixture: &mut Fixture, address: u32, address_bits: usize) -> u64 {
    eeprom_clock_in(fixture, true);
    eeprom_clock_in(fixture, true);
    for bit in (0..address_bits).rev() {
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

/// **E2 — the save file's size is the chip's, decided by the accesses, not
/// by the ID string.**
///
/// The dynamic-capacity model (v2.0.3 S3-b): an EEPROM buffer starts at the
/// smallest real chip, 512 B (GBATEK `:8665`), and the first transfer that
/// addresses past it grows the buffer to the 8 KB ceiling the 14-bit address
/// width spans (GBATEK `:8669`, `:8705-8706`; mGBA's `_ensureEeprom`,
/// `savedata.c:471-489`). The ID string decides only that the cartridge has
/// an EEPROM — GBATEK `:8521` gives the whole family one entry, "EEPROM_Vnnn
/// EEPROM 512 bytes or 8 Kbytes", where `nnn` is the library version — so a
/// bare `EEPROM_V` cartridge and an `EEPROM_V124` one must behave alike.
///
/// Every size is asserted against a number derived from GBATEK's own
/// 0200h/2000h ([`EEPROM_512_BYTES`] / [`EEPROM_8K_BYTES`]), and the grown
/// block is read back, so a buffer that merely *reported* 8 KB without being
/// addressable there would still fail. Each write is committed with a
/// `Ready` clock-out before its size is read: an uninterpreted command sits
/// in the transfer buffer and would make the size assertion green for the
/// wrong reason.
#[test]
fn eeprom_chip_capacity() {
    let mut eight_k = Fixture::new(&cartridge_with("EEPROM_V124", &[swi(HALT)]));
    assert!(
        eight_k.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );

    // A 6-bit transfer fills a slice of the 512 B chip: the file stays at
    // the floor, and must not be the flat 32 KB the old model answered for
    // every EEPROM cartridge.
    eeprom_write_block(&mut eight_k, 0, EEPROM_TEST_DATA, EEPROM_512_ADDRESS_BITS);
    assert!(
        eeprom_clock_out(&mut eight_k),
        "the chip must report Ready after the 6-bit write"
    );
    assert_eq!(
        battery_size(&eight_k),
        EEPROM_512_BYTES,
        "a 6-bit transfer never leaves the 512 B floor (GBATEK :8665)"
    );

    // A 14-bit transfer to block 0x1FF (byte 4088) is past the low half, so
    // the buffer grows to the ceiling — and the block is addressable there.
    eeprom_write_block(&mut eight_k, 0x1FF, EEPROM_TEST_DATA, EEPROM_8K_ADDRESS_BITS);
    assert!(
        eeprom_clock_out(&mut eight_k),
        "the chip must report Ready after the 14-bit write"
    );
    assert_eq!(
        battery_size(&eight_k),
        EEPROM_8K_BYTES,
        "a 14-bit transfer past the low half grows the file to 8 KB (GBATEK :8669)"
    );
    let data = eeprom_read_block(&mut eight_k, 0x1FF, EEPROM_8K_ADDRESS_BITS);
    assert_eq!(
        data, EEPROM_TEST_DATA,
        "the grown region must read back what was written, not just be longer"
    );

    // The bare `EEPROM_V` cartridge has the same chip behaviour: the ID
    // string says "is an EEPROM", never "how big". Pinning this on a second
    // cartridge keeps anyone from reintroducing a version-to-size mapping
    // (the plan's original split was contradicted by GBATEK `:8521`).
    let mut bare = Fixture::new(&cartridge_with("EEPROM_V", &[swi(HALT)]));
    assert!(
        bare.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    eeprom_write_block(&mut bare, 0x1FF, EEPROM_TEST_DATA, EEPROM_8K_ADDRESS_BITS);
    assert!(
        eeprom_clock_out(&mut bare),
        "the chip must report Ready after the bare cartridge's write"
    );
    assert_eq!(
        battery_size(&bare),
        EEPROM_8K_BYTES,
        "a bare `EEPROM_V` cartridge grows the same way: the ID string does \
         not carry the capacity"
    );
}

/// **E3, resolved — `0x0E000006/7` is the save array, and the completion
/// signal is the data read back.**
///
/// The plan (§1.2.4) expected a flash *status register* here, whose bit 0
/// would say "ready". Four independent sources agree that GBA flash has no
/// such register — there is no `0x70` command at all — and that the wait a
/// game runs is a **read-back compare** against the bytes just written:
///
/// 1. **GBATEK** (the snapshot's whole FLASH chapter, `:8858-9146`): every
///    polling line is an array read and compare — "wait until
///    [E00xxxxh]=dat" (`:8981`), "wait until [E000000h]=FFh" (`:8915`) — a
///    grep for `E000006` over the 4.0 MB snapshot returns zero hits, and
///    `:9119-9126` adds a software verify-retry on top of the polling.
/// 2. **Nintendo's own SDK flash driver**, decompiled by pret
///    (`agb_flash_1m.c:51-86`, `agb_flash.c:106-116`) — the code actually
///    burned into commercial cartridges — polls with a plain array read
///    (`ReadFlash1`) and compares against the last byte written.
/// 3. **mGBA 0.10.5** (command set `savedata.h:38-49`, decode
///    `savedata.c:367-381`) implements no `0x70` either.
/// 4. **VBA-M** (`vbam_gbaFlash.cpp`) likewise: zero occurrences of `0x70`.
///
/// So the target behaviour this lock asserts is the vendor's current one: a
/// byte written with GBATEK's write-byte command (`:8972-8981`) must read
/// back as **itself** — not as `0xFF`, not as anything else. Offsets
/// 0..=7 are planted with [`PLANT_BYTE`], so the halfword at `0x0E000006/7`
/// is that byte replicated across both lanes; a witness byte proves the
/// plant landed before the read is trusted, and a plain byte write would not
/// do — on a flash cartridge it enters the command state machine and is
/// dropped in the `Ready` state (`internal_memory.rs:755-759`).
///
/// The assertion reads the **register** the program loaded, not a memory
/// round trip. (A busy-window simulation — the read "not yet ready" for some
/// cycles after a write, mGBA's dust timer — was considered and is **not**
/// implemented: GBATEK `:9119`'s verify-retry and the SDK's immediate-success
/// path mean games cope without it, and MKSC plays with saves on the current
/// behaviour. Revisit only with real-cartridge evidence.)
#[test]
fn flash_status_reg() {
    let body = [
        // r1 = 0x0E00_0000, then OR in the offset: 0x0E00_0006.
        mov_imm(1, 0x0E00_0000),
        orr_imm(1, 1, 0x06),
        // r2 = the halfword the game would poll after a write: the planted
        // array byte, replicated across the 8-bit bus's lanes
        // (`bus.rs:1643-1646`).
        ldrh(2, 1),
        swi(HALT),
    ];
    let rom = cartridge_with("FLASH_V124", &body);
    let mut fixture = Fixture::new(&rom);

    for offset in 0..=PLANT_LAST_OFFSET {
        flash_write_byte(&mut fixture, offset, PLANT_BYTE);
    }

    // The expected halfword, assembled from the plant and the 8-bit bus's
    // lane replication rather than typed as a literal: the single byte at
    // the polled offset appears in both lanes of the halfword the CPU sees.
    // The register is 32 bits wide, so the comparison happens at that width.
    let expected = u32::from(PLANT_BYTE) * 0x0101;
    assert_ne!(
        expected, 0xFFFF,
        "the planted halfword must differ from the erased one, or this lock \
         could not tell a read-back from an untouched chip"
    );

    // The plant must have landed: without it the read below could not tell
    // "the bytes we wrote" from "the erased value", and the assertion would
    // pass or fail for a reason that has nothing to do with the read path.
    assert_eq!(
        flash_save_byte(&mut fixture, PLANT_WITNESS_OFFSET),
        PLANT_BYTE,
        "the plant must have landed in the save data, or the read-back below \
         proves nothing"
    );

    assert!(
        fixture.run_to_halt(BUDGET),
        "the program never reached its Halt"
    );
    let read_back = fixture.reg(2);
    assert_eq!(
        read_back, expected,
        "reading 0x0E000006/7 after planting must return the bytes just \
         written — the read-back compare that is this chip's only completion \
         signal — not 0xFF and not a status word"
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
    eeprom_write_block(&mut fixture, 0, EEPROM_TEST_DATA, EEPROM_8K_ADDRESS_BITS);
    // GBATEK `:8783-8786`: keep reading until bit 0 says "Ready".
    assert!(
        eeprom_clock_out(&mut fixture),
        "the chip must report Ready after a write"
    );

    // Read block 0 back.
    let data = eeprom_read_block(&mut fixture, 0, EEPROM_8K_ADDRESS_BITS);
    assert_eq!(
        data, EEPROM_TEST_DATA,
        "the EEPROM must give back the block it was given, bit for bit"
    );
}
