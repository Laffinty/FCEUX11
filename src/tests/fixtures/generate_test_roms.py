#!/usr/bin/env python3
"""Generate minimal iNES test ROMs for representative mappers."""

import os
import struct

MAPPERS = [
    ("nrom",    0),
    ("mmc1",    1),
    ("uxrom",   2),
    ("mmc3",    4),
    ("mmc5",    5),
    ("axrom",   7),
    ("vrc6",   24),
    ("vrc7",   85),
]

PRG_SIZE = 1       # 1 x 16KB
CHR_SIZE = 0       # 0 x 8KB, use CHR-RAM

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))


def make_header(mapper: int) -> bytes:
    # FCEUX extracts mapper number as:
    #   MapperNo = (head.ROM_type >> 4) | (head.ROM_type2 & 0xF0)
    # So mapper low nibble goes to byte 6 high bits,
    # and mapper high nibble goes to byte 7 high bits.
    byte6 = (mapper & 0x0F) << 4
    byte7 = (mapper & 0xF0)
    header = struct.pack(
        "<4sBBBBBBBBBBBB",
        b"NES\x1A",
        PRG_SIZE,
        CHR_SIZE,
        byte6,
        byte7,
        0, 0, 0, 0, 0, 0, 0, 0
    )
    return header


def main():
    for suffix, mapper in MAPPERS:
        path = os.path.join(SCRIPT_DIR, f"mapper_{suffix}.nes")
        header = make_header(mapper)
        prg = bytes([0xEA] * (PRG_SIZE * 16384))  # NOP fill
        with open(path, "wb") as f:
            f.write(header)
            f.write(prg)
        print(f"Generated {path} (mapper {mapper}, {len(header) + len(prg)} bytes)")


if __name__ == "__main__":
    main()
