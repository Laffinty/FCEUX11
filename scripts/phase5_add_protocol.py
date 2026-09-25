#!/usr/bin/env python3
"""Phase 5 — stamp the real runner protocol onto every rom-suite case.

Adds to `tests/tests.json`:
  * `protocol`      — "$6000" (default NES test-ROM status port),
                      "nestest-trace" (kgmqa-048),
                      "aggregate-mapperel" (kgmqa-077).
  * `mirror_glob`   — only kgmqa-077 (Holy Mapperel 47-ROM aggregate).

Idempotent: re-running overwrites the same fields with the same values.
Validate afterwards with scripts/phase2_validate_tests_json.py.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TESTS_JSON = ROOT / "tests" / "tests.json"

PROTO_6000 = "$6000"
PROTO_NESTEST = "nestest-trace"
PROTO_AGGREGATE = "aggregate-mapperel"
HOLY_MAPPEREL_GLOB = "holy_mapperel/M*.nes"


def protocol_for(kgmqa_id: str) -> str:
    if kgmqa_id.startswith("kgmqa-048"):
        return PROTO_NESTEST
    if kgmqa_id.startswith("kgmqa-077"):
        return PROTO_AGGREGATE
    return PROTO_6000


def main() -> int:
    data = json.loads(TESTS_JSON.read_text(encoding="utf-8"))
    cases = data.get("cases", [])
    stamped = 0
    for case in cases:
        kinds = case.get("kind") or []
        if "rom-suite" not in kinds:
            continue
        kgid = case.get("kgmqa_id", "")
        proto = protocol_for(kgid)
        case["protocol"] = proto
        if proto == PROTO_AGGREGATE:
            case["mirror_glob"] = HOLY_MAPPEREL_GLOB
        stamped += 1

    # tests.json is CRLF + 2-space indent (PowerShell-friendly, no BOM)
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    TESTS_JSON.write_text(text.replace("\n", "\r\n"), encoding="utf-8", newline="")
    print(f"OK: stamped protocol on {stamped} rom-suite cases -> {TESTS_JSON}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
