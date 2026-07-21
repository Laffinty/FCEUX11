#!/usr/bin/env python3
"""v0.3.10 P4.3: migrate sound / wave / avi call sites to fceu11::."""

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent / "src"

MAPPING = {
    "FCEUI_Sound": "fceu11::Sound",
    "FCEUI_SetSoundQuality": "fceu11::SetSoundQuality",
    "FCEUI_SetSoundVolume": "fceu11::SetSoundVolume",
    "FCEUI_SetTriangleVolume": "fceu11::SetTriangleVolume",
    "FCEUI_SetSquare1Volume": "fceu11::SetSquare1Volume",
    "FCEUI_SetSquare2Volume": "fceu11::SetSquare2Volume",
    "FCEUI_SetNoiseVolume": "fceu11::SetNoiseVolume",
    "FCEUI_SetPCMVolume": "fceu11::SetPCMVolume",
    "FCEUI_BeginWaveRecord": "fceu11::BeginWaveRecord",
    "FCEUI_EndWaveRecord": "fceu11::EndWaveRecord",
    "FCEUI_WaveRecordRunning": "fceu11::WaveRecordRunning",
    "FCEUI_AviVideoUpdate": "fceu11::AviVideoUpdate",
    "FCEUI_AviIsRecording": "fceu11::AviIsRecording",
    "FCEUI_AviEnableHUDrecording": "fceu11::AviEnableHUDrecording",
    "FCEUI_SetAviEnableHUDrecording": "fceu11::SetAviEnableHUDrecording",
    "FCEUI_AviDisableMovieMessages": "fceu11::AviDisableMovieMessages",
    "FCEUI_SetAviDisableMovieMessages": "fceu11::SetAviDisableMovieMessages",
}

keys = sorted(MAPPING.keys(), key=len, reverse=True)
pattern = re.compile(r"\b(" + "|".join(re.escape(k) for k in keys) + r")\(")

SKIP_PATHS = {
    ROOT / "core_api.h",
    ROOT / "io_api.h",
    ROOT / "wave.h",
    ROOT / "drivers" / "Qt" / "sdl-video.h",
}


def main() -> int:
    total = 0
    changed = 0
    for path in ROOT.rglob("*.cpp"):
        if path in SKIP_PATHS:
            continue
        text = path.read_text(encoding="utf-8")
        new_text, count = pattern.subn(lambda m: MAPPING[m.group(1)] + "(", text)
        if count:
            path.write_text(new_text, encoding="utf-8")
            total += count
            changed += 1
            print(f"{path.relative_to(ROOT.parent)}: {count} replacements")
    print(f"\nTotal: {total} replacements in {changed} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
