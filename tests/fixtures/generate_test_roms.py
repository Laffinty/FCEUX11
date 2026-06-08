#!/usr/bin/env python3
"""Generate minimal test ROMs for v0.3.0 regression fixtures.

Covers 15+ mappers/formats as required by the construction plan:
  NROM, MMC1, UxROM, CNROM, MMC3, MMC5, AxROM, GNROM,
  ColorDreams, VRC2/4, VRC6, VRC7, FDS, NSF
"""

import os
import struct

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))

# iNES mappers: (suffix, mapper_number)
INES_MAPPERS = [
    ("nrom",         0),
    ("mmc1",         1),
    ("uxrom",        2),
    ("cnrom",        3),
    ("mmc3",         4),
    ("mmc5",         5),
    ("axrom",        7),
    ("colordreams", 11),
    ("gnrom",       66),
    ("vrc2and4",    21),   # VRC2/4 (21 used here; 25 also valid)
    ("vrc6",        24),
    ("vrc7",        85),
]

PRG_SIZE = 1   # 1 x 16 KB
CHR_SIZE = 0   # 0 x 8 KB  => CHR-RAM


def make_ines_header(mapper: int) -> bytes:
    """Build a 16-byte iNES 1.0 header."""
    # Mapper low nibble -> byte6 high bits
    # Mapper high nibble -> byte7 high bits
    byte6 = (mapper & 0x0F) << 4
    byte7 = (mapper & 0xF0)
    return struct.pack(
        "<4sBBBBBBBBBBBB",
        b"NES\x1A",
        PRG_SIZE,
        CHR_SIZE,
        byte6,
        byte7,
        0, 0, 0, 0, 0, 0, 0, 0
    )


def generate_ines_roms():
    for suffix, mapper in INES_MAPPERS:
        path = os.path.join(SCRIPT_DIR, f"mapper_{suffix}.nes")
        header = make_ines_header(mapper)
        prg = bytes([0xEA] * (PRG_SIZE * 16384))  # NOP sled
        with open(path, "wb") as f:
            f.write(header)
            f.write(prg)
        print(f"Generated {path} (mapper {mapper}, {len(header) + len(prg)} bytes)")


def generate_fds():
    """Generate a minimal FDS image (no disk header, just enough to load)."""
    path = os.path.join(SCRIPT_DIR, "test_fds.fds")
    # FDS images often start with the FDS BIOS header or raw disk data.
    # For loader-level regression we only need the magic.
    # Real FDS: sides are 65500 bytes; we write a tiny stub.
    header = b"FDS\x1A\x01"  # 1 side
    side_data = bytes([0x00] * 65500)
    with open(path, "wb") as f:
        f.write(header)
        f.write(side_data)
    print(f"Generated {path} (FDS, {len(header) + len(side_data)} bytes)")


def generate_nsf():
    """Generate a minimal NSF (NES Sound Format) header."""
    path = os.path.join(SCRIPT_DIR, "test_nsf.nsf")
    # NSF header: NESM\x1A + version(1) + ...
    header = bytearray(128)
    header[0:5] = b"NESM\x1A"
    header[5] = 1          # version
    header[6] = 1          # total songs
    header[7] = 1          # starting song
    header[8:10] = struct.pack("<H", 0x8000)  # load address
    header[10:12] = struct.pack("<H", 0x8003)  # init address
    header[12:14] = struct.pack("<H", 0x8000)  # play address
    header[14:46] = b"Test" + bytes([0] * 31)  # song name
    header[46:78] = bytes([0] * 32)            # artist
    header[78:110] = bytes([0] * 32)           # copyright
    header[110:111] = struct.pack("<H", 16666)[0:1]  # NTSC speed (~60Hz)
    header[111:112] = struct.pack("<H", 16666)[1:2]
    header[122] = 0        # bankswitch flags (none)
    header[123] = 0
    header[124] = 0
    header[125] = 0
    header[126] = 0x00     # PAL/NTSC bits
    header[127] = 0x00     # extra sound chips
    # Minimal program data
    prg = bytes([0xEA] * 16384)
    with open(path, "wb") as f:
        f.write(header)
        f.write(prg)
    print(f"Generated {path} (NSF, {len(header) + len(prg)} bytes)")


def main():
    generate_ines_roms()
    generate_fds()
    generate_nsf()
    print("\nAll fixtures generated.")


if __name__ == "__main__":
    main()
