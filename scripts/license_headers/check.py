#!/usr/bin/env python3
"""check.py -- v2.0.8 batch1 L3: every C/C++ file under src/ carries GPL text
or is whitelisted (third-party / generated). CI-facing: exit 1 on violations.

Usage:
  python scripts/license_headers/check.py
  python scripts/license_headers/check.py --root <dir> --whitelist <json>
"""
import argparse
import json
import os
import sys

GPL_MARK = "GNU General Public License"
EXTS = {".cpp", ".h", ".c"}
SCAN_SKIP_DIRS = {"target", ".git"}

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="src")
    ap.add_argument("--whitelist", default=os.path.join(HERE, "third_party_whitelist.json"))
    args = ap.parse_args()

    with open(args.whitelist, "r", encoding="utf-8") as fh:
        wl = json.load(fh)
    allowed = set(wl.get("third_party", {})) | set(wl.get("generated", {}))

    root = args.root if os.path.isabs(args.root) else os.path.join(REPO, args.root)
    missing = []
    total = 0
    for dp, dn, fn in os.walk(root):
        dn[:] = [d for d in dn if d not in SCAN_SKIP_DIRS]
        for f in sorted(fn):
            if os.path.splitext(f)[1] not in EXTS:
                continue
            path = os.path.join(dp, f)
            try:
                rel = os.path.relpath(path, REPO).replace(os.sep, "/")
            except ValueError:  # cross-drive scan root (mutation testing)
                rel = os.path.relpath(path, root).replace(os.sep, "/")
            total += 1
            with open(path, "rb") as fh:
                if GPL_MARK.encode() in fh.read()[:2048]:
                    continue
            if rel not in allowed:
                missing.append(rel)

    print("== license header check ==")
    print("scanned :", total)
    print("allowed (whitelist):", len(allowed))
    if missing:
        print("MISSING GPL header (not whitelisted):", len(missing))
        for rel in missing:
            print("  ", rel)
        return 1
    print("OK: every file carries GPL text or is whitelisted")
    return 0


if __name__ == "__main__":
    sys.exit(main())
