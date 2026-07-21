#!/usr/bin/env python3
"""Phase B: rewrite 171 board file #include "mapinc.h" lines.

Maps each file to the minimal split header based on usage profiling.
Adds explicit #include "../unif.h", "../ppu.h", "../cheat.h" for files
that need them but lose them when mapinc.h is replaced.

Re-runnable: only rewrites files that still have the old #include "mapinc.h".
"""
from pathlib import Path
import re

BOARDS = Path("src/boards")

# Files that need extra headers that the old mapinc.h provided transitively.
UNIF_USERS = {
    "01-222.cpp", "354.cpp", "68.cpp", "coolboy.cpp", "coolgirl.cpp",
    "datalatch.cpp", "fk23c.cpp", "mmc3.cpp", "mmc5.cpp", "sa-9602b.cpp",
    "sachen.cpp", "unrom512.cpp", "vrc2and4.cpp",
}
PPU_USERS = {
    "mmc5.cpp", "coolgirl.cpp", "datalatch.cpp",
}
CHEAT_USERS = {
    "datalatch.cpp",
}

AUDIO = {"69.cpp", "mmc5.cpp", "n106.cpp", "vrc6.cpp", "vrc7.cpp"}

# Bus symbols: setprg*/setchr*/setmirror*/setntamem + their 'r' (RAM-bank) variants.
BUS_RE = re.compile(
    r"\bset(?:prg\d*r?|chr\d*r?|mirror(?:w)?|ntamem|ntamirroring|MIRROR)\b"
    r"|\bFCEU_MIRROR\b"
)
# MMC3 shared-state symbols (from mmc3.h).
MMC3_RE = re.compile(
    r"\bMMC3[A-Za-z_]+\b"
    r"|\bDRegBuf\b"
    r"|\bIRQ(?:Count|Latch|a|Reload)\b"
    r"|\bEXPREGS\b"
    r"|\bmmc3opts\b"
    r"|\b(?:pwrap|cwrap|mwrap)\b"
    r"|\bGenMMC3(?:Power|Restore|Init|Close)\b"
    r"|\bFixMMC3(?:PRG|CHR)\b"
)

def classify(text: str, fname: str) -> str:
    if fname in AUDIO:
        return "mapinc_audio.h"
    if MMC3_RE.search(text):
        return "mapinc_mmc3.h"
    if BUS_RE.search(text):
        return "mapinc_bus.h"
    return "mapinc_base.h"

def process(path: Path) -> None:
    text = path.read_text(encoding="utf-8", errors="surrogateescape")
    fname = path.name
    bucket = classify(text, fname)

    extras = []
    if fname in UNIF_USERS:
        extras.append('#include "../unif.h"')
    if fname in PPU_USERS:
        extras.append('#include "../ppu.h"')
    if fname in CHEAT_USERS:
        extras.append('#include "../cheat.h"')

    new_include = f'#include "{bucket}"'
    if extras:
        new_include += "\n" + "\n".join(extras)

    new_text, n = re.subn(r'#include\s+"mapinc\.h"', new_include, text, count=1)
    if n != 1:
        raise SystemExit(f"{fname}: expected 1 match, got {n}")
    path.write_text(new_text, encoding="utf-8", errors="surrogateescape")
    print(f"  {fname} -> {bucket}" + (f" + {len(extras)} extras" if extras else ""))

def main():
    targets = sorted(BOARDS.glob("*.cpp")) + sorted(BOARDS.glob("*.h"))
    targets = [p for p in targets if not p.name.startswith("mapinc_")]
    n_done = 0
    for p in targets:
        text = p.read_text(encoding="utf-8", errors="surrogateescape")
        if '#include "mapinc.h"' in text:
            process(p)
            n_done += 1
    print(f"Processed {n_done} files.")

if __name__ == "__main__":
    main()