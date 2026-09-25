#!/usr/bin/env python3
"""Phase 5 — route-completeness probe for f11qa-rom-runner.

Runs the dispatcher once per rom-suite case and asserts every exit code is
in the legal set {0, 1, 2, 3} (no panics / no unknown protocols). Also
prints the distribution so CI can eyeball skip vs missing-rom vs spawn-fail.

Legal exits:
  0 = PASS or vendor_state skip (advisory / pending-vendor)
  1 = real FAIL (ROM ran, $6000 != 0x00, or aggregate member failed)
  2 = arg / case / ROM-file missing (fetch hint printed)
  3 = underlying runner spawn failure (environment problem)

Usage:
  python scripts/phase5_probe_routes.py [--bin <path>] [--tests-json <path>]
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_BIN = ROOT / "src/rust/target/x86_64-pc-windows-msvc/debug/f11qa-rom-runner.exe"
DEFAULT_JSON = ROOT / "tests/tests.json"
LEGAL = {0, 1, 2, 3}


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--bin", type=Path, default=DEFAULT_BIN)
    p.add_argument("--tests-json", type=Path, default=DEFAULT_JSON)
    args = p.parse_args()

    if not args.bin.exists():
        print(f"FAIL: dispatcher not found: {args.bin} (cargo build -p f11qa --bin f11qa-rom-runner)")
        return 2

    data = json.loads(args.tests_json.read_text(encoding="utf-8"))
    cases = [c for c in data.get("cases", []) if "rom-suite" in (c.get("kind") or [])]
    if not cases:
        print("FAIL: no rom-suite cases in tests.json")
        return 2

    dist: dict[int, int] = {}
    illegal: list[tuple[str, int]] = []
    for c in cases:
        kgid = c["kgmqa_id"]
        r = subprocess.run(
            [str(args.bin), "--kgmqa-id", kgid, "--tests-json", str(args.tests_json), "--frames", "30"],
            capture_output=True,
            text=True,
            cwd=ROOT,
        )
        dist[r.returncode] = dist.get(r.returncode, 0) + 1
        if r.returncode not in LEGAL:
            illegal.append((kgid, r.returncode))

    total = sum(dist.values())
    print(f"rom-suite probed : {total}")
    print(f"exit distribution: {dict(sorted(dist.items()))}")
    if illegal:
        print(f"FAIL: illegal exits (panic / unknown): {illegal}")
        return 1
    print(f"OK: all {total} rom-suite routes exited in {sorted(LEGAL)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
