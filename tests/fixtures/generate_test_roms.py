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
    # Phase E.2 step 3a: P0 mappers with existing MapperStrategyA Cart
    # subclasses from Phase D.4/D.6/D.8 (Action 53 28, IREM/TXC/Bit Corp
    # 32-38, SMB2j/CALTRON 40-50, VRC2/4 variants 22/23/25).  Each shares
    # the 16-byte MapperStrategyA::save_mapper_state default until
    # per-mapper state capture lands in a followup commit.
    ("cprom",       13),   # CPROM (mapper 13) -- distinct from anrom/axrom
    ("mapper28",    28),   # Action 53
    ("mapper32",    32),   # IREM G-101
    ("mapper33",    33),   # TENGEN RBI Baseball
    ("mapper34",    34),   # NINA-001
    ("mapper36",    36),   # TXC Policeman
    ("mapper38",    38),   # Bit Corp Crime Busters
    ("mapper40",    40),   # SMB2j FDS
    ("mapper41",    41),   # CALTRON 6-in-1
    ("mapper42",    42),   # SMB2j FDS 42
    ("mapper43",    43),   # SMB2j FDS 43
    ("mapper46",    46),   # GameStar Smarty
    ("mapper50",    50),   # SMB2j FDS 50
    ("vrc22",       22),   # VRC2/4 variant 22
    ("vrc23",       23),   # VRC2/4 variant 23
    ("vrc25",       25),   # VRC2/4 variant 25
    # Phase E.2 step 5: 21 MMC3 variants (share Mmc3BaseCart factory).
    ("mapper12",    12),   # MMC3 variant 12
    ("mapper37",    37),   # MMC3 variant 37
    ("mapper44",    44),   # MMC3 variant 44
    ("mapper45",    45),   # MMC3 variant 45
    ("mapper47",    47),   # MMC3 variant 47
    ("mapper49",    49),   # MMC3 variant 49
    ("mapper52",    52),   # MMC3 variant 52
    ("mapper74",    74),   # MMC3 variant 74
    ("mapper105",  105),   # MMC3 variant 105
    ("mapper114",  114),   # MMC3 variant 114
    ("mapper115",  115),   # MMC3 variant 115
    ("mapper116",  116),   # MMC3 variant 116
    ("mapper118",  118),   # MMC3 variant 118
    ("mapper119",  119),   # MMC3 variant 119
    ("mapper165",  165),   # MMC3 variant 165
    ("mapper205",  205),   # MMC3 variant 205
    ("mapper245",  245),   # MMC3 variant 245
    ("mapper249",  249),   # MMC3 variant 249
    ("mapper250",  250),   # MMC3 variant 250
    ("mapper254",  254),   # MMC3 variant 254
    ("mapper406",  406),   # MMC3 variant 406
    # Phase E.2 step 6: 4 more MMC3 variants (192/194/195/198 -- BMC pirates).
    ("mapper192",  192),   # MMC3 variant 192
    ("mapper194",  194),   # MMC3 variant 194
    ("mapper195",  195),   # MMC3 variant 195
    ("mapper198",  198),   # MMC3 variant 198
    # Phase E.2 step 7: 4 simple P0 mappers with new Cart subclasses.
    ("mmc2",         9),  # MMC2 (Pine Bros, Castlevania II)
    ("mmc4",        10),  # MMC4 (Wario Land II, etc.)
    ("mapper15",    15),  # 100-in-1 (contra)
    ("mapper48",    48),  # Taito MMC3 variant
    # Phase E.2 step 8: round out P0 mappers + VRC6 variant.
    ("bandai",      16),   # Bandai (mapper 16)
    ("mapper18",    18),   # Magic Floor (mapper 18)
    ("vrc6var26",   26),   # Konami VRC6 variant 26 (shares Vrc6Cart)
    # Phase E.2 step 9.2: 7 P1 Latch-family mappers from datalatch.cpp.
    # Each emits a 17-byte body (MapperStrategyA 16-byte default + latche).
    ("mapper70",    70),
    ("mapper78",    78),
    ("mapper86",    86),
    ("mapper87",    87),
    ("mapper89",    89),
    ("mapper94",    94),
    ("mapper97",    97),
    # Phase E.2 step 9.3: 23 P1 mappers outside the Latch family.  All use
    # MapperStrategyA default body (16 bytes).
    ("mapper51",    51),
    ("mapper57",    57),
    ("mapper61",    61),
    ("mapper62",    62),
    ("mapper64",    64),
    ("mapper65",    65),
    ("mapper67",    67),
    ("mapper68",    68),
    ("mapper71",    71),
    ("mapper72",    72),
    ("mapper73",    73),
    ("mapper75",    75),
    ("mapper77",    77),
    ("mapper79",    79),
    ("mapper80",    80),
    ("mapper82",    82),
    ("mapper83",    83),
    ("mapper88",    88),
    ("mapper90",    90),
    ("mapper91",    91),
    ("mapper92",    92),
    ("mapper93",    93),
    ("mapper96",    96),
    ("mapper99",    99),
]

PRG_SIZE = 1   # 1 x 16 KB
CHR_SIZE = 0   # 0 x 8 KB  => CHR-RAM


def make_ines_header(mapper: int) -> bytes:
    """Build a 16-byte iNES header.  Uses iNES 2.0 format for mapper numbers
    > 255 (e.g. mapper 406) since iNES 1.0 only encodes mappers 0-255."""
    if mapper < 256:
        # iNES 1.0: byte 6 = mapper low nibble, byte 7 = mapper high nibble
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
    else:
        # iNES 2.0: byte 6 = mapper low nibble, byte 7 = (mapper mid nibble << 4 | iNES 2.0
        # marker in bits 2-3).  iNES parser reads (byte7 & 0xF0) for the mapper mid
        # nibble and ((byte7 & 0x0C) == 0x08) to detect iNES 2.0.
        byte6 = (mapper & 0x0F) << 4
        byte7 = ((mapper >> 4) & 0x0F) << 4 | 0x08
        byte8 = (mapper >> 8) & 0x0F
        # 16-byte iNES header: indices 0-3 magic, 4 PRG, 5 CHR, 6 byte6, 7 byte7,
        # 8 byte8 (mapper high nibble), 9-15 padding.
        return struct.pack(
            "<4sBBBBBBBBBBBB",
            b"NES\x1A",
            PRG_SIZE, CHR_SIZE,
            byte6, byte7,
            byte8,
            0, 0, 0, 0, 0, 0, 0
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
