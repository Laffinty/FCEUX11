#!/usr/bin/env python3
"""Add reset_after field (-1 default) to all blargg manifest ROM entries that lack it.

v1.17 H-1 task: 177 ROMs, 8 currently have reset_after; add to remaining 169.

Preserves the JSON structure as much as possible:
- Keeps BOM
- Keeps 4-space indent for the roms array entries
- Keeps key ordering: name, path, category, frames, reset_after, probe_addr, description

For entries that already have reset_after, the script is a no-op for them.
For entries without, it inserts reset_after between frames and probe_addr.
"""
import json
import sys
from pathlib import Path

MANIFEST = Path("tests/fixtures/blargg_manifest.json")

raw = MANIFEST.read_bytes()
has_bom = raw.startswith(b"\xef\xbb\xbf")
text = raw.decode("utf-8-sig" if has_bom else "utf-8")
data = json.loads(text)
roms = data["roms"]
total = len(roms)
existing = sum(1 for r in roms if "reset_after" in r)
missing = total - existing
print(f"manifest: {total} roms, {existing} with reset_after, {missing} without")

# Insert reset_after: -1 into entries that lack it
# Strategy: rebuild entries preserving the order: name, path, category, frames, reset_after, probe_addr, description
KEY_ORDER = ["name", "path", "category", "frames", "reset_after", "probe_addr", "description"]
new_roms = []
modified = 0
for r in roms:
    if "reset_after" in r:
        new_roms.append(r)
        continue
    out = {}
    for k in KEY_ORDER:
        if k == "reset_after":
            out[k] = -1
        if k in r:
            out[k] = r[k]
    # Catch any extra keys we didn't enumerate
    for k, v in r.items():
        if k not in out:
            out[k] = v
    new_roms.append(out)
    modified += 1

data["roms"] = new_roms

# Serialize with same indent style (4 spaces, no trailing newline changes)
new_text = json.dumps(data, indent=4, ensure_ascii=False)
new_bytes = (b"\xef\xbb\xbf" if has_bom else b"") + new_text.encode("utf-8")
# Don't add a trailing newline; match the existing file's behavior
MANIFEST.write_bytes(new_bytes)
print(f"modified: {modified}")
print(f"wrote: {MANIFEST} ({len(new_bytes)} bytes)")