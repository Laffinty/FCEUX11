#!/usr/bin/env python3
"""v0.3.10 P4.4 helper: update tests / benchmark to use fceu11::* names for
APIs already migrated in P4.1. Leaves expected_api_test.cpp alone because it
intentionally exercises the legacy FCEUI_* compatibility wrappers.
"""

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent

# Only migrated APIs may be rewritten here.
MAPPING = {
    "FCEUI_LoadGame": "fceu11::LoadGame",
    "FCEUI_Initialize": "fceu11::Initialize",
    "FCEUI_Emulate": "fceu11::Emulate",
    "FCEUI_CloseGame": "fceu11::CloseGame",
    "FCEUI_Kill": "fceu11::Kill",
    "FCEUI_ResetNES": "fceu11::ResetNES",
    "FCEUI_PowerNES": "fceu11::PowerNES",
}

keys = sorted(MAPPING.keys(), key=len, reverse=True)
pattern = re.compile(r"\b(" + "|".join(re.escape(k) for k in keys) + r")\(")

SKIP_FILES = {
    ROOT / "tests" / "expected_api_test.cpp",
}


def main() -> int:
    total = 0
    changed = 0
    for subdir in (ROOT / "tests", ROOT / "src" / "tests"):
        if not subdir.exists():
            continue
        for path in subdir.rglob("*.cpp"):
            if path in SKIP_FILES:
                continue
            text = path.read_text(encoding="utf-8")
            new_text, count = pattern.subn(lambda m: MAPPING[m.group(1)] + "(", text)
            if count:
                path.write_text(new_text, encoding="utf-8")
                total += count
                changed += 1
                print(f"{path.relative_to(ROOT)}: {count} replacements")
    print(f"\nTotal: {total} replacements in {changed} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
