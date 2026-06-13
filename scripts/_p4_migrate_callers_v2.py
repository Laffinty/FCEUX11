#!/usr/bin/env python3
"""v0.3.10 P4.3 v2: replace migrated io_api.h call sites with fceu11::* equivalents.

Targets APIs migrated in the second P4.1 wave (input / palette / NTSC /
base-directory / render-planes). Leaves the wrapper headers alone.
"""

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent / "src"

MAPPING = {
    "FCEUI_GetBaseDirectory": "fceu11::GetBaseDirectory",
    "FCEUI_SetBaseDirectory": "fceu11::SetBaseDirectory",
    "FCEUI_SetInputFourscore": "fceu11::SetInputFourscore",
    "FCEUI_GetInputFourscore": "fceu11::GetInputFourscore",
    "FCEUI_GetInputMicrophone": "fceu11::GetInputMicrophone",
    "FCEUI_UseInputPreset": "fceu11::UseInputPreset",
    "FCEUI_SetRenderPlanes": "fceu11::SetRenderPlanes",
    "FCEUI_GetRenderPlanes": "fceu11::GetRenderPlanes",
    "FCEUI_GetUserPaletteAvail": "fceu11::GetUserPaletteAvail",
    "FCEUI_SetUserPalette": "fceu11::SetUserPalette",
    "FCEUI_GetNTSCTH": "fceu11::GetNTSCTH",
    "FCEUI_SetNTSCTH": "fceu11::SetNTSCTH",
    "FCEUI_SetInputFC": "fceu11::SetInputFC",
    "FCEUI_SetInput": "fceu11::SetInput",
}

keys = sorted(MAPPING.keys(), key=len, reverse=True)
pattern = re.compile(r"\b(" + "|".join(re.escape(k) for k in keys) + r")\(")

SKIP_PATHS = {
    ROOT / "core_api.h",
    ROOT / "io_api.h",
}

SKIP_DIRS = {
    ROOT / "archived",
}


def should_process(path: Path) -> bool:
    if path.suffix not in (".cpp", ".h", ".c"):
        return False
    if path in SKIP_PATHS:
        return False
    for skip_dir in SKIP_DIRS:
        try:
            path.relative_to(skip_dir)
            return False
        except ValueError:
            pass
    return True


def main() -> int:
    total = 0
    changed = 0
    for path in ROOT.rglob("*"):
        if not should_process(path):
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
