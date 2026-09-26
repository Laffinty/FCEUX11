#!/usr/bin/env python3
"""Oracle B suite gate: run f11qa_blargg_runner over blargg_manifest.json
and exit 0 iff every FAIL is in blargg_known_fail.json (no unexpected fails).

The raw batch exits 1 whenever any known-fail ROM is still failing (33 today),
which is the correct runner behaviour but not a matrix pass/fail signal.
This wrapper is the kgmqa-043 oracle: 'suite ran + no unexpected FAILs'.
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
MANIFEST = REPO / "tests" / "fixtures" / "blargg_manifest.json"
KNOWN_FAIL = REPO / "tests" / "fixtures" / "blargg_known_fail.json"
RUNNER_CANDIDATES = [
    REPO / "build" / "tests" / "f11qa_blargg_runner.exe",
    REPO / "build" / "tests" / "f11qa_blargg_runner",
]


def main() -> int:
    runner = next((p for p in RUNNER_CANDIDATES if p.exists()), None)
    if runner is None:
        print("[fail] f11qa_blargg_runner not found", file=sys.stderr)
        return 2
    if not MANIFEST.exists():
        print(f"[fail] missing {MANIFEST}", file=sys.stderr)
        return 2

    known = set()
    if KNOWN_FAIL.exists():
        data = json.loads(KNOWN_FAIL.read_text(encoding="utf-8-sig"))
        for item in data.get("failures", []):
            known.add(Path(item.get("rom", "")).stem)

    r = subprocess.run(
        [str(runner), "--manifest", "fixtures/blargg_manifest.json"],
        cwd=str(REPO / "tests"),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    out = (r.stdout or "") + "\n" + (r.stderr or "")
    fails = []
    for m in re.finditer(r'"rom"\s*:\s*"([^"]+)"[^}]*"status"\s*:\s*"FAIL"', out):
        fails.append(m.group(1))
    # summary fallback
    passed = re.search(r"Passed:\s*(\d+)", out)
    failed = re.search(r"Failed:\s*(\d+)", out)
    print(f"batch exit={r.returncode} parsed_fail={len(fails)} "
          f"summary={passed.group(1) if passed else '?'}/{failed.group(1) if failed else '?'}")

    unexpected = [f for f in fails if f not in known]
    if unexpected:
        print(f"[fail] {len(unexpected)} unexpected FAIL(s) not in known_fail:")
        for u in unexpected[:20]:
            print(f"  - {u}")
        return 1
    print(f"[ok] suite ran; {len(fails)} FAIL(s) all covered by known_fail ({len(known)} entries)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
