#!/usr/bin/env python3
"""Patch PALRAM/UPALRAM call sites for std::array migration."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

patches = {
    "src/drivers/Qt/NameTableViewer.cpp": [
        (r"memcmp\(palcache,PALRAM,32\)", "memcmp(palcache,PALRAM.data(),32)"),
        (r"memcpy\(palcache,PALRAM,32\)", "memcpy(palcache,PALRAM.data(),32)"),
    ],
    "src/drivers/Qt/ppuViewer.cpp": [
        (r"memcmp\(pallast, PALRAM, 32\)", "memcmp(pallast, PALRAM.data(), 32)"),
        (r"memcmp\(pallast\+32, UPALRAM, 3\)", "memcmp(pallast+32, UPALRAM.data(), 3)"),
        (r"memcpy\(pallast, PALRAM, 32\)", "memcpy(pallast, PALRAM.data(), 32)"),
        (r"memcpy\(pallast\+32, UPALRAM, 3\)", "memcpy(pallast+32, UPALRAM.data(), 3)"),
        (r"memcpy\(palcache,PALRAM,32\)", "memcpy(palcache,PALRAM.data(),32)"),
    ],
    "src/drivers/Qt/sdl-video.cpp": [
        (r"extern uint8 PALRAM\[0x20\];", "extern std::array<uint8_t, 0x20> PALRAM;"),
    ],
    "src/boards/mmc5.cpp": [
        (r"extern uint8 PALRAM\[0x20\];", "extern std::array<uint8_t, 0x20> PALRAM;"),
        (r"extern uint8 UPALRAM\[0x03\];", "extern std::array<uint8_t, 3> UPALRAM;"),
    ],
}

for rel_path, rules in patches.items():
    p = ROOT / rel_path
    text = p.read_text(encoding="utf-8")
    for old_re, new in rules:
        text = re.sub(old_re, new, text)
    p.write_text(text, encoding="utf-8")
    print(f"Patched {rel_path}")
