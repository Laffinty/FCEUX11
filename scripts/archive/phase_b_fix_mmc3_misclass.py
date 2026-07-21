#!/usr/bin/env python3
"""Phase B.5b: reclassify mmc3-bucket files that don't actually use MMC3 shared state.

Files that previously got `#include "mapinc_mmc3.h"` but never included mmc3.h
explicitly nor call any MMC3_* function are using IRQCount/IRQa/IRQLatch as their
own local identifiers — not from mmc3.h.  Pulling mmc3.h now causes a redefinition
error.  Move them back to `mapinc_bus.h`.

Re-runnable.
"""
from pathlib import Path
import re

BOARDS = Path("src/boards")

# Only unambiguous MMC3 references — local IRQCount/IRQa/IRQLatch are excluded.
UNAMBIGUOUS_MMC3 = re.compile(
    r"\bMMC3_(?:cmd|CMDWrite|IRQWrite|Power|Restore|Init|Close)\b"
    r"|\bMMC3_cmd\b"
    r"|\bMMC3RegReset\b"
    r"|\bGenMMC3(?:Power|Restore|Init|Close)\b"
    r"|\bFixMMC3(?:PRG|CHR)\b"
    r"|\bDRegBuf\b"
    r"|\bmmc3opts\b"
    r"|\bEXPREGS\b"
)

def process(p: Path) -> bool:
    text = p.read_text(encoding="utf-8", errors="surrogateescape")
    if '#include "mapinc_mmc3.h"' not in text:
        return False
    has_explicit = bool(re.search(r'^#include\s+"mmc3\.h"', text, re.M))
    uses_unambiguous = bool(UNAMBIGUOUS_MMC3.search(text))
    if has_explicit or uses_unambiguous:
        return False
    new = text.replace('#include "mapinc_mmc3.h"', '#include "mapinc_bus.h"', 1)
    p.write_text(new, encoding="utf-8", errors="surrogateescape")
    print(f"  {p.name}: mmc3 -> bus")
    return True

def main():
    targets = sorted(BOARDS.glob("*.cpp")) + sorted(BOARDS.glob("*.h"))
    n = sum(process(p) for p in targets)
    print(f"Reclassified {n} files.")

if __name__ == "__main__":
    main()