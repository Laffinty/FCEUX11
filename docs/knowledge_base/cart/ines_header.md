<!--
source: https://www.nesdev.org/wiki/iNES
retrieved: 2026-09-13
conversion: automated from the owner-provided HTML archive; conventions in ../../README.md
-->

# iNES (NESdev Wiki)

> Source: https://www.nesdev.org/wiki/iNES

The .NES file format (file name suffix .nes) is the de facto standard for distribution of NES binary programs, with use even in licensed emulators such as commercialized PocketNES and Wii Virtual Console. It is often called the iNES format, as it was created by Marat Fayzullin for an emulator called iNES. The format was later extended with NES 2.0 to fix many of its shortcomings.












## iNES emulator

iNES was an early NES emulator developed by Marat Fayzullin. Its most lasting contribution to the NES scene was its popularization of the iNES ROM file format and mapper numbering system.

## Name of file format

This file format is commonly referred to as the iNES file format/iNES header format. The file extension is .nes, so it is sometimes referred to as the .nes file format, and files in it as .nes files. Now that the NES 2.0 file format exists, which uses the same .nes extension, a .nes file/the .nes file format could mean the iNES file format or NES 2.0 format, so the full format names should be used where the differences in the formats are relevant, like specifications or format support.

## iNES file format

An iNES file consists of the following sections, in order:

- Header (16 bytes)

- Trainer, if present (0 or 512 bytes)

- PRG ROM data (16384 * x bytes)

- CHR ROM data, if present (8192 * y bytes)

- PlayChoice INST-ROM, if present (0 or 8192 bytes)

- PlayChoice PROM, if present (16 bytes Data, 16 bytes CounterOut) (this is often missing; see PC10 ROM-Images for details)

Some ROM-Images additionally contain a 128-byte (or sometimes 127-byte) title at the end of the file.

The format of the header is as follows:

| Bytes | Description |
|---|---|
| 0-3 | Constant $4E $45 $53 $1A (ASCII "NES" followed by MS-DOS end-of-file) |
| 4 | Size of PRG ROM in 16 KB units |
| 5 | Size of CHR ROM in 8 KB units (value 0 means the board uses CHR RAM) |
| 6 | Flags 6 – Mapper, mirroring, battery, trainer |
| 7 | Flags 7 – Mapper, VS/Playchoice, NES 2.0 |
| 8 | Flags 8 – PRG-RAM size (rarely used extension) |
| 9 | Flags 9 – TV system (rarely used extension) |
| 10 | Flags 10 – TV system, PRG-RAM presence (unofficial, rarely used extension) |
| 11-15 | Unused padding (should be filled with zero, but some rippers put their name across bytes 7-15) |

### Flags 6

76543210
||||||||
|||||||+- Nametable arrangement: 0: vertical arrangement ("horizontal mirrored") (CIRAM A10 = PPU A11)
||||||| 1: horizontal arrangement ("vertically mirrored") (CIRAM A10 = PPU A10)
||||||+-- 1: Cartridge contains battery-backed PRG RAM ($6000-7FFF) or other persistent memory
|||||+--- 1: 512-byte trainer at $7000-$71FF (stored before PRG data)
||||+---- 1: Alternative nametable layout
++++----- Lower nybble of mapper number

In the iNES format, cartridge boards are divided into classes called "mappers" based on similar board hardware and behavior, and each mapper has an 8-bit number (or 12-bit in NES 2.0). The low 4 bits of this mapper are given here in bits 7-4 of this field.

The presence of persistent saved memory is given by bit 1. This usually takes the form of battery-backed PRG-RAM at $6000, but there are some mapper-specific exceptions:

- UNROM 512 and GTROM use flash memory to store their game state by rewriting the PRG-ROM area.

#### Nametable Arrangement

See: Nametable Mirroring

For mappers with hard-wired nametable layout, connecting CIRAM A10 to PPU A10 or A11 for a vertical or horizontal arrangement is specified by bit 0.

The exact meaning of the "Alternative nametable layout" bit varies by the mapper, with both current and historical (deprecated) uses of this bit.
Some mappers have a 4-screen variation of the board, which is specified with bit 3:

- MMC3 for Rad Racer 2

- Mapper 206 for Gauntlet

Others use both nametable layout bits together:

- UNROM 512 uses %....1..0 to indicate a 1-screen board, and %....1..1 to indicate a 4-screen board.

- Mapper 218 (Magic Floor) has 4 unusual CIRAM configurations corresponding to each of the possible values.

Several mappers have some form of 4-screen as their only option. ROMs might be found with bit 3 set to redundantly indicate this:

- Napoleon Senki

- Vs. System

- GTROM

Historically a number of other ROM images have used it:

- Mapper 78. "Alternative" is H/V. "Normal" is 1scA/1scB. NES2.0 submappers were allocated for both.

- Mapper 70. "Alternative" was 1scA/1scB. "Normal" was hard-wired. "Alternative" was relocated to Mapper 152.

Ambiguity:

- Many mappers (MMC1, MMC3, AxROM, …) have mapper-controlled nametable mirroring. These will ignore bit 0.

- Mappers that share 4-screen nametable RAM with CHR-RAM may interact with the NES 2.0 CHR-RAM in byte 11.

- Historical belief that bit 3 always meant 4-screen nametables results in many emulators poorly supporting this, with various bad side effects if the game then tries to write the mirroring control register.

#### Trainer

The trainer usually contains mapper register translation and CHR-RAM caching code for

- early RAM cartridges that could not mimic mapper ASICs and only had 32 KiB of CHR-RAM;

- Nesticle, an old but influential NES emulator for DOS.

It is not used on unmodified dumps of original ROM cartridges.

### Flags 7

76543210
||||||||
|||||||+- VS Unisystem
||||||+-- PlayChoice-10 (8 KB of Hint Screen data stored after CHR data)
||||++--- If equal to 2, flags 8-15 are in NES 2.0 format
++++----- Upper nybble of mapper number

The PlayChoice-10 bit is not part of the official specification, and most emulators simply ignore the extra 8 KB of data.
PlayChoice games are designed to look good with the 2C03 RGB PPU, which handles color emphasis differently from a standard NES PPU.

Vs. games have a coin slot and different palettes.
The detection of which palette a particular game uses is left unspecified.

NES 2.0 is a more recent extension to the format that allows more flexibility in ROM and RAM size, among other things.

### Flags 8

76543210
||||||||
++++++++- PRG RAM size

Size of PRG RAM in 8 KB units (Value 0 infers 8 KB for compatibility; see PRG RAM circuit)

This was a later extension to the iNES format and not widely used. NES 2.0 is recommended for specifying PRG RAM size instead.

### Flags 9

76543210
||||||||
|||||||+- TV system (0: NTSC; 1: PAL)
+++++++-- Reserved, set to zero

Though in the official specification, very few emulators honor this bit as virtually no ROM images in circulation make use of it.

### Flags 10

76543210
|| ||
|| ++- TV system (0: NTSC; 2: PAL; 1/3: dual compatible)
|+----- PRG RAM ($6000-$7FFF) (0: present; 1: not present)
+------ 0: Board has no bus conflicts; 1: Board has bus conflicts

This byte is not part of the official specification, and relatively few emulators honor it.

The PRG RAM Size value (stored in byte 8) was recently added to the official specification; as such, virtually no ROM images in circulation make use of it.

Older versions of the iNES emulator ignored bytes 7-15, and several ROM management tools wrote messages in there. Commonly, these will be filled with "DiskDude!", which results in 64 being added to the mapper number.

A general rule of thumb: if the last 4 bytes are not all zero, and the header is not marked for NES 2.0 format, an emulator should either mask off the upper 4 bits of the mapper number or simply refuse to load the ROM.

### Variant comparison

Over the years, the header of the .NES file format has changed as new features became needed.
There are three discernable generations:

Archaic iNES

Created by Marat Fayzullin and used in very old versions of iNES and in NESticle. ROM image conversion and auditing tools tended to store signature strings at offsets 7-15.

iNES 0.7

Created by Marat Fayzullin when the scene discovered the diversity of NES cartridge hardware. Mapper high nibble is supported in emulators since roughly 2000.

iNES

Later revisions added byte 8 (PRG RAM size) and byte 9 (TV system), though few other emulators supported these fields.

NES 2.0

Created by kevtris for the FPGA Kevtendo and maintained by the NESdev community to clarify ambiguous cases that previous headers did not clarify. Became widely adopted starting in the 2010s.

| Thing | Archaic iNES | iNES | NES 2.0 |
|---|---|---|---|
| Byte 7 | Unused | Mapper high nibble, Vs. | Mapper high nibble, NES 2.0 signature, PlayChoice, Vs. |
| Byte 8 | Unused | Total PRG RAM size (linear) | Mapper highest nibble, mapper variant |
| Byte 9 | Unused | TV system | Upper bits of ROM size |
| Byte 10 | Unused | Unused | PRG RAM size (logarithmic; battery and non-battery) |
| Byte 11 | Unused | Unused | VRAM size (logarithmic; battery and non-battery) |
| Byte 12 | Unused | Unused | TV system |
| Byte 13 | Unused | Unused | Vs. PPU variant |
| Byte 14 | Unused | Unused | Miscellaneous ROMs |
| Byte 15 | Unused | Unused | Default expansion device |
| Mappers supported | 0-15 | 0-255 | 0-4095 |

Recommended detection procedure:

- If byte 7 AND $0C = $08, and the size taking into account byte 9 does not exceed the actual size of the ROM image, then NES 2.0.

- If byte 7 AND $0C = $04, archaic iNES.

- If byte 7 AND $0C = $00, and bytes 12-15 are all 0, then iNES.

- Otherwise, iNES 0.7 or archaic iNES.

## See also

- INES 1.0 Mapper Grid

- List of INES mappers

- TNES – file format used by 3DS Virtual Console

## References

- Official iNES file format specification

- iNES format by rvu

- iNES header/format by VmprHntrD

- iNES emulator


