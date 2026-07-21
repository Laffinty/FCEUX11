#!/usr/bin/env python3
"""v0.3.10 P4.3 helper: replace migrated FCEUI_* call sites with fceu11::* equivalents.

This script ONLY touches call sites for APIs that have already been moved into
fceu11:: namespace in core_api.h. The compatibility wrappers in core_api.h and
the expected_api_test are intentionally left untouched so the old symbols remain
verified.
"""

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent / "src"

# Mapping from legacy global name to fceu11:: qualified name.
# Sorted by key length descending so longer names (e.g. VSUniCoin2) are handled
# before their prefixes.
MAPPING = {
    "FCEUI_ClearEmulationFrameStepped": "fceu11::ClearEmulationFrameStepped",
    "FCEUI_GetCurrentVidSystem": "fceu11::GetCurrentVidSystem",
    "FCEUI_EmulationFrameStepped": "fceu11::EmulationFrameStepped",
    "FCEUI_ToggleEmulationPause": "fceu11::ToggleEmulationPause",
    "FCEUI_PauseFramesRemaining": "fceu11::PauseFramesRemaining",
    "FCEUI_LoadGameVirtual": "fceu11::LoadGameVirtual",
    "FCEUI_VSUniToggleDIP": "fceu11::VSUniToggleDIP",
    "FCEUI_SetEmulationPaused": "fceu11::SetEmulationPaused",
    "FCEUI_SetRenderedLines": "fceu11::SetRenderedLines",
    "FCEUI_PauseForDuration": "fceu11::PauseForDuration",
    "FCEUI_EmulationPaused": "fceu11::IsEmulationPaused",
    "FCEUI_VSUniGetDIPs": "fceu11::VSUniGetDIPs",
    "FCEUI_VSUniSetDIP": "fceu11::VSUniSetDIP",
    "FCEUI_FrameAdvanceEnd": "fceu11::FrameAdvanceEnd",
    "FCEUI_GetDesiredFPS": "fceu11::GetDesiredFPS",
    "FCEUI_SelectStateNext": "fceu11::SelectStateNext",
    "FCEUI_SaveSnapshotAs": "fceu11::SaveSnapshotAs",
    "FCEUI_VSUniService": "fceu11::VSUniService",
    "FCEUI_SetVidSystem": "fceu11::SetVidSystem",
    "FCEUI_VSUniCoin2": "fceu11::VSUniCoin2",
    "FCEUI_SaveSnapshot": "fceu11::SaveSnapshot",
    "FCEUI_DatachSet": "fceu11::DatachSet",
    "FCEUI_FDSSelect": "fceu11::FDSSelect",
    "FCEUI_FDSInsert": "fceu11::FDSInsert",
    "FCEUI_NSFGetInfo": "fceu11::NSFGetInfo",
    "FCEUI_VSUniCoin": "fceu11::VSUniCoin",
    "FCEUI_NSFChange": "fceu11::NSFChange",
    "FCEUI_NSFSetVis": "fceu11::NSFSetVis",
    "FCEUI_SetRegion": "fceu11::SetRegion",
    "FCEUI_GetRegion": "fceu11::GetRegion",
    "FCEUI_SaveState": "fceu11::SaveStateFile",
    "FCEUI_LoadState": "fceu11::LoadStateFile",
    "FCEUI_SelectState": "fceu11::SelectStateSlot",
    "FCEUI_SetGameGenie": "fceu11::SetGameGenie",
    "FCEUI_SetLowPass": "fceu11::SetLowPass",
    "FCEUI_FrameAdvance": "fceu11::FrameAdvance",
    "FCEUI_ResetNES": "fceu11::ResetNES",
    "FCEUI_PowerNES": "fceu11::PowerNES",
    "FCEUI_LoadGame": "fceu11::LoadGame",
    "FCEUI_Initialize": "fceu11::Initialize",
    "FCEUI_Emulate": "fceu11::Emulate",
    "FCEUI_CloseGame": "fceu11::CloseGame",
    "FCEUI_Kill": "fceu11::Kill",
    "FCEUI_NMI": "fceu11::NMI",
    "FCEUI_IRQ": "fceu11::IRQ",
}

# Build regex that matches any of the legacy names followed by '('.
# Use \b for word boundary to avoid matching inside identifiers.
keys = sorted(MAPPING.keys(), key=len, reverse=True)
pattern = re.compile(r"\b(" + "|".join(re.escape(k) for k in keys) + r")\(")

# Files / directories we must not touch.
SKIP_PATHS = {
    ROOT / "core_api.h",  # keep the compatibility wrappers
}
SKIP_DIRS = {
    ROOT / "tests",       # handled in P4.4 / left for compatibility tests
    ROOT / "archived",    # dead code
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
    total_replacements = 0
    changed_files = 0

    for path in ROOT.rglob("*"):
        if not should_process(path):
            continue
        text = path.read_text(encoding="utf-8")
        new_text, count = pattern.subn(lambda m: MAPPING[m.group(1)] + "(", text)
        if count:
            path.write_text(new_text, encoding="utf-8")
            total_replacements += count
            changed_files += 1
            print(f"{path.relative_to(ROOT)}: {count} replacements")

    print(f"\nTotal: {total_replacements} replacements in {changed_files} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
