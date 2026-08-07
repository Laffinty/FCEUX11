#!/usr/bin/env python3
"""v1.17 Task1-C1 switch-over: repoint tests.json blargg entries to the Rust runner.

Changes binary: fceux11_blargg_runner -> kagami_qa_blargg_runner and adds a
provenance marker with the parity commit for each blargg_* entry.
"""
import json
import sys
from pathlib import Path

TESTS_JSON = Path("tests/tests.json")
PARITY_COMMIT = "9488773"

raw = TESTS_JSON.read_bytes()
has_bom = raw.startswith(b"\xef\xbb\xbf")
text = raw.decode("utf-8-sig" if has_bom else "utf-8")
data = json.loads(text)
entries = data if isinstance(data, list) else data.get("tests", data)
if isinstance(entries, dict):
    entries = list(entries.values())

changed = 0
for e in entries:
    if not isinstance(e, dict):
        continue
    eid = str(e.get("id", ""))
    if not eid.startswith("blargg_"):
        continue
    inp = e.get("input", {})
    if inp.get("binary") == "fceux11_blargg_runner":
        inp["binary"] = "kagami_qa_blargg_runner"
        e["provenance"] = f"Task1-C1 rust migration, parity {PARITY_COMMIT} (177/177)"
        changed += 1
    # blargg_suite has no binary; add provenance note only
    elif eid == "blargg_suite":
        e["provenance"] = f"Task1-C1 rust migration, parity {PARITY_COMMIT} (177/177)"

print(f"repointed {changed} blargg entries to kagami_qa_blargg_runner")

new_text = json.dumps(data, indent=2, ensure_ascii=False)
new_bytes = (b"\xef\xbb\xbf" if has_bom else b"") + new_text.encode("utf-8")
TESTS_JSON.write_bytes(new_bytes)
print(f"wrote {TESTS_JSON} ({len(new_bytes)} bytes)")
