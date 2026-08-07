#!/usr/bin/env python3
"""v1.17 H-2 verifier: scan blargg_full_output.txt and manifest for 0x80 ROMs.

A 0x80 return from a blargg ROM means "still running" — the ROM was not
allowed enough frames to converge. This is a frames-budget calibration
defect, not a real precision failure. H-2's job is to ensure no manifest
entry hits 0x80 at its current `frames` setting.

Usage:
  python tools/verify_no_0x80.py [--output build/blargg_full_output.txt] [--manifest tests/fixtures/blargg_manifest.json]

Exit code: 0 if no 0x80 ROMs, 1 if any found.
"""
import argparse
import json
import re
import sys
from pathlib import Path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", default="build/blargg_full_output.txt", help="blargg output file")
    ap.add_argument("--manifest", default="tests/fixtures/blargg_manifest.json", help="manifest file")
    ap.add_argument("--ci", action="store_true", help="CI mode: print status to stdout only")
    args = ap.parse_args()

    out_path = Path(args.output)
    man_path = Path(args.manifest)

    if not out_path.exists():
        print(f"FAIL: blargg output not found: {out_path}")
        sys.exit(2)
    if not man_path.exists():
        print(f"FAIL: manifest not found: {man_path}")
        sys.exit(2)

    # Try UTF-16 LE first (PowerShell default), then UTF-8.
    raw = out_path.read_bytes()
    if raw[:2] in (b"\xff\xfe", b"\xfe\xff") or (raw[0:1] == b"\x00" and raw[2:3] != b"\x00"):
        text = raw.decode("utf-16", errors="replace")
    else:
        text = raw.decode("utf-8", errors="replace")
    pat = re.compile(
        r"\[(?P<rom>[^\]]+)\]\s+(?P<frames>\d+)\s+frames(?:\s+\(reset @ (?P<reset>\d+)\))?\.\.\."
        r"\s+(?P<status>PASS|FAIL)\s+\((?P<value>0x[0-9A-Fa-f]+)\)"
    )
    rows = []
    for m in pat.finditer(text):
        rom = m.group("rom")
        frames = int(m.group("frames"))
        reset = int(m.group("reset")) if m.group("reset") else None
        status = m.group("status")
        value = int(m.group("value"), 16)
        rows.append({"rom": rom, "frames": frames, "reset_after": reset, "status": status, "value": value})
    if not rows:
        print(f"FAIL: no rows parsed from {out_path}")
        sys.exit(2)

    # Find 0x80 hits
    hits_80 = [r for r in rows if r["value"] == 0x80]
    hits_81 = [r for r in rows if r["value"] == 0x81]

    # Load manifest for cross-reference
    raw = man_path.read_bytes()
    text = raw.decode("utf-8-sig" if raw.startswith(b"\xef\xbb\xbf") else "utf-8")
    data = json.loads(text)
    roms = data["roms"]

    # Cross-check: for each 0x80 ROM, show the manifest's frames / reset_after
    print(f"=== H-2 Calibration Status ({out_path}) ===")
    print(f"Total ROMs parsed: {len(rows)}")
    print(f"Manifest entries: {len(roms)}")
    print(f"0x80 (still-running) hits: {len(hits_80)}")
    print(f"0x81 (reset-needed)   hits: {len(hits_81)}")
    print()

    if hits_80 or hits_81:
        print("!! Calibration defect detected. ROMs needing more frames:")
        manifest_by_name = {r["name"]: r for r in roms}
        for h in hits_80 + hits_81:
            m = manifest_by_name.get(h["rom"], {})
            cur_frames = m.get("frames", "?")
            cur_reset = m.get("reset_after", "?")
            print(f"  {h['rom']:40s}  status={h['status']:4s} value=0x{h['value']:02X} "
                  f"ran_with_frames={h['frames']} reset_after={h['reset']}  "
                  f"manifest.frames={cur_frames} manifest.reset_after={cur_reset}")
        print()
        print("Action: bump manifest `frames` (and optionally `reset_after`) for "
              "each hit, then re-run blargg_runner. The H-2 task is to eliminate "
              "these.")
        sys.exit(1)
    else:
        print("PASS: no 0x80 or 0x81 hits — all ROMs converge within manifest budget.")
        sys.exit(0)


if __name__ == "__main__":
    main()