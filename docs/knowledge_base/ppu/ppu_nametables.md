<!--
source: https://www.nesdev.org/wiki/PPU_nametables
retrieved: 2026-09-13
conversion: automated from the owner-provided HTML archive; conventions in ../../README.md
-->

# PPU nametables (NESdev Wiki)

> Source: https://www.nesdev.org/wiki/PPU_nametables

A nametable is a 1024 byte area of memory used by the PPU to lay out backgrounds.
Each byte in the nametable controls one 8x8 pixel character cell, and each nametable has 30 rows of 32 tiles each, for 960 ($3C0) bytes; the 64 ($40) remaining bytes are used by each nametable's attribute table.
With each tile being 8x8 pixels, this makes a total of 256x240 pixels in one map, the same size as one full screen.

(0,0) (256,0) (511,0)
+-----------+-----------+
| | |
| | |
| $2000 | $2400 |
| | |
| | |
(0,240)+-----------+-----------+(511,240)
| | |
| | |
| $2800 | $2C00 |
| | |
| | |
+-----------+-----------+
(0,479) (256,479) (511,479)

See also: PPU memory map

## Mirroring

Main article: Mirroring

The NES has four logical nametables, arranged in a 2x2 pattern. Each occupies a 1 KiB chunk of PPU address space, starting at $2000 at the top left, $2400 at the top right, $2800 at the bottom left, and $2C00 at the bottom right.

But the NES system board itself has only 2 KiB of VRAM (called CIRAM, stored in a separate SRAM chip), enough for two physical nametables; hardware on the cartridge controls address bit 10 of CIRAM to map one nametable on top of another.

- Horizontal arrangement: $2000 and $2800 contain the first nametable, and $2400 and $2C00 contain the second nametable (e.g. Super Mario Bros.), accomplished by connecting CIRAM A10 to PPU A10

- Vertical arrangement: $2000 and $2400 contain the first nametable, and $2800 and $2C00 contain the second nametable (e.g. Kid Icarus), accomplished by connecting CIRAM A10 to PPU A11

- Single-screen: All nametables refer to the same memory at any given time, and the mapper directly manipulates CIRAM A10 (e.g. many Rare games using AxROM)

- Four-screen nametables: The cartridge contains additional VRAM used for all nametables (e.g. Gauntlet, Rad Racer 2)

- Other: Some advanced mappers can present arbitrary combinations of CIRAM, VRAM, or even CHR ROM in the nametable area. Such exotic setups are rarely used.

## Background evaluation

Main article: PPU rendering

Conceptually, the PPU does this 33 times for each scanline:

- Fetch a nametable entry from $2000-$2FFF.

- Fetch the corresponding attribute table entry from $23C0-$2FFF and increment the current VRAM address within the same row.

- Fetch the low-order byte of an 8x1 pixel sliver of pattern table from $0000-$0FF7 or $1000-$1FF7.

- Fetch the high-order byte of this sliver from an address 8 bytes higher.

- Turn the attribute data and the pattern table data into palette indices, and combine them with data from sprite data using priority.

It also does a fetch of a 34th (nametable, attribute, pattern) tuple that is never used, but some mappers rely on this fetch for timing purposes.

## See also

- PPU attribute tables


