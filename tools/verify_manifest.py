#!/usr/bin/env python3
"""Verify manifest after reset_after backfill."""
import json
from pathlib import Path

p = Path("tests/fixtures/blargg_manifest.json")
raw = p.read_bytes()
text = raw.decode("utf-8-sig" if raw.startswith(b"\xef\xbb\xbf") else "utf-8")
data = json.loads(text)
roms = data["roms"]
total = len(roms)
existing_reset = [r for r in roms if "reset_after" in r]
missing = [r for r in roms if "reset_after" not in r]
print(f"total: {total}")
print(f"with reset_after: {len(existing_reset)}")
print(f"without: {len(missing)}")
assert len(missing) == 0, "some entries still missing reset_after"
print("PASS: all 177 entries have reset_after field")
# Show distribution of reset_after values
from collections import Counter
c = Counter(r.get("reset_after") for r in roms)
print(f"reset_after distribution: {dict(c)}")
print(f"sample: {roms[0]}")
print(f"sample (with positive): {[r for r in roms if r.get('reset_after', -1) > 0][0]}")