#!/usr/bin/env python3
"""v0.3.10 P4.3 cheat engine migration: move cheat APIs into fceu11:: namespace."""

import re
from pathlib import Path

ROOT = Path(__file__).parent.parent / "src"

# (legacy name, fceu11 name)
CHEAT_APIS = [
    ("FCEUI_DecodePAR", "fceu11::DecodePAR"),
    ("FCEUI_DecodeGG", "fceu11::DecodeGG"),
    ("FCEUI_AddCheat", "fceu11::AddCheat"),
    ("FCEUI_DelCheat", "fceu11::DelCheat"),
    ("FCEUI_ToggleCheat", "fceu11::ToggleCheat"),
    ("FCEUI_GlobalToggleCheat", "fceu11::GlobalToggleCheat"),
    ("FCEUI_CheatSearchGetCount", "fceu11::CheatSearchGetCount"),
    ("FCEUI_CheatSearchGetRange", "fceu11::CheatSearchGetRange"),
    ("FCEUI_CheatSearchGet", "fceu11::CheatSearchGet"),
    ("FCEUI_CheatSearchBegin", "fceu11::CheatSearchBegin"),
    ("FCEUI_CheatSearchEnd", "fceu11::CheatSearchEnd"),
    ("FCEUI_ListCheats", "fceu11::ListCheats"),
    ("FCEUI_GetCheat", "fceu11::GetCheat"),
    ("FCEUI_SetCheat", "fceu11::SetCheat"),
    ("FCEUI_CheatSearchShowExcluded", "fceu11::CheatSearchShowExcluded"),
    ("FCEUI_CheatSearchSetCurrentAsOriginal", "fceu11::CheatSearchSetCurrentAsOriginal"),
    ("FCEUI_CreateCheatMap", "fceu11::CreateCheatMap"),
    ("FCEUI_RefreshCheatMap", "fceu11::RefreshCheatMap"),
    ("FCEUI_SetCheatMapByte", "fceu11::SetCheatMapByte"),
]

legacy_names = sorted([n for n, _ in CHEAT_APIS], key=len, reverse=True)
call_re = re.compile(r"\b(" + "|".join(re.escape(n) for n in legacy_names) + r")\(")


def replace_calls(text: str) -> tuple[str, int]:
    return call_re.subn(lambda m: dict(CHEAT_APIS)[m.group(1)] + "(", text)


def update_header(path: Path, namespace_name: str = "fceu11") -> None:
    """Convert legacy declarations into namespace declarations + inline wrappers."""
    text = path.read_text(encoding="utf-8")
    new_lines = []
    ns_decls = []
    wrappers = []
    removed = 0

    for line in text.splitlines(keepends=True):
        matched = None
        for legacy, qualified in CHEAT_APIS:
            # Match declaration patterns like "int FCEUI_AddCheat(...);"
            if re.match(rf"^[ \t]*(int|void|int32|bool)[ \t]+{re.escape(legacy)}\(.*\);\s*$", line):
                matched = (legacy, qualified)
                break
        if matched:
            legacy, qualified = matched
            # Extract return type and signature
            m = re.match(rf"^([ \t]*)(int|void|int32|bool)[ \t]+{re.escape(legacy)}(\(.*\));\s*$", line)
            if m:
                indent, ret, sig = m.group(1), m.group(2), m.group(3)
                ns_name = qualified.split("::")[1]
                ns_decls.append(f"{indent}    {ret} {ns_name}{sig};\n")
                wrappers.append(f"{indent}inline {ret} {legacy}{sig} {{ {qualified}{sig}; }}\n")
                removed += 1
                continue
        new_lines.append(line)

    if ns_decls:
        # Insert namespace block + wrappers where the first removed declaration was.
        # For simplicity append at end of file (headers are forgiving).
        new_lines.append(f"\nnamespace {namespace_name} {{\n")
        new_lines.extend(ns_decls)
        new_lines.append(f"}} // namespace {namespace_name}\n")
        new_lines.append("\n")
        new_lines.extend(wrappers)

    path.write_text("".join(new_lines), encoding="utf-8")
    print(f"{path.relative_to(ROOT.parent)}: {removed} declarations migrated")


def update_source(path: Path) -> None:
    text = path.read_text(encoding="utf-8")
    new_text, count = replace_calls(text)
    if count:
        path.write_text(new_text, encoding="utf-8")
        print(f"{path.relative_to(ROOT.parent)}: {count} replacements")


def main() -> int:
    update_header(ROOT / "core_api.h")
    update_header(ROOT / "cheat.h")
    update_source(ROOT / "cheat.cpp")

    # Replace call sites across src/ (excluding headers we just rewrote).
    skip = {ROOT / "core_api.h", ROOT / "cheat.h"}
    for path in ROOT.rglob("*.cpp"):
        if path in skip:
            continue
        text = path.read_text(encoding="utf-8")
        new_text, count = replace_calls(text)
        if count:
            path.write_text(new_text, encoding="utf-8")
            print(f"{path.relative_to(ROOT.parent)}: {count} replacements")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
