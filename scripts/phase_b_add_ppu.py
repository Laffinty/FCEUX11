#!/usr/bin/env python3
"""Phase B.5d: add explicit #include "../ppu.h" to files using PPU state
(vnapage / VPage / PPU_hook / PPUCHRRAM / PPUNTARAM) which originally
got it transitively from the aggregate mapinc.h.
"""
from pathlib import Path
import re

BOARDS = Path("src/boards")
PPU_STATE_USERS = {
    "68.cpp", "80.cpp", "90.cpp", "96.cpp", "coolgirl.cpp",
    "dance2000.cpp", "datalatch.cpp", "et-4320.cpp", "mmc2and4.cpp",
    "mmc3.cpp", "mmc5.cpp", "n106.cpp",
}

def main():
    n = 0
    for fname in sorted(PPU_STATE_USERS):
        p = BOARDS / fname
        text = p.read_text(encoding="utf-8", errors="surrogateescape")
        if '"../ppu.h"' in text:
            continue
        new = re.sub(
            r'(#include\s+"mapinc_[a-z0-9]+\.h")',
            r'\1\n#include "../ppu.h"',
            text,
            count=1,
        )
        if new == text:
            continue
        p.write_text(new, encoding="utf-8", errors="surrogateescape")
        print(f"  {fname}: added ppu.h")
        n += 1
    print(f"Added ppu.h to {n} files.")

if __name__ == "__main__":
    main()