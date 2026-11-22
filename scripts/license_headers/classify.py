#!/usr/bin/env python3
"""classify.py -- v2.0.8 batch1 L1: classify src/ C/C++ files for GPL header stamping.

Rules (docs/plans/FCEUX11-v2.0.8_许可证合规与崩溃韧性-构建计划.md section 2, r2):
  R1 -> bucket C: third-party/generated whitelist (never stamped)
  R2 -> bucket B: fork-base (1c35642) path membership, OR same filename existed in
                  the fork base at another path (content-origin, e.g. src/fir -> src/archived/fir),
                  OR upstream copyright lines in the current text
  R3 -> bucket A: project-original (post-fork, pure project)

Read-only: touches no source file. Output: manifest.json beside this script + stdout report.
"""
import json
import os
import re
import subprocess
import sys
from collections import Counter

FORK_BASE = "1c35642"
GPL_MARK = "GNU General Public License"
EXTS = {".cpp", ".h", ".c"}
COPYRIGHT_RE = re.compile(r"Copyright \(C\) (?:19|20)\d\d.*")
SCAN_SKIP_DIRS = {"target", ".git"}

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))
SRC = os.path.join(REPO, "src")


def load_whitelist():
    with open(os.path.join(HERE, "third_party_whitelist.json"), "r", encoding="utf-8") as f:
        data = json.load(f)
    files = {}
    files.update(data.get("third_party", {}))
    files.update(data.get("generated", {}))
    return files


def fork_base_tree():
    out = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", FORK_BASE],
        cwd=REPO, capture_output=True, text=True, encoding="utf-8", errors="replace",
    ).stdout
    paths = [p.strip() for p in out.splitlines() if p.strip()]
    by_name = {}
    for p in paths:
        by_name.setdefault(os.path.basename(p), []).append(p)
    return set(paths), by_name


def extract_copyright_lines(text):
    """Return deduped 'Copyright (C) ...' statements from the first 2 KB."""
    found = []
    for line in text[:2048].splitlines():
        m = COPYRIGHT_RE.search(line)
        if m:
            s = m.group(0).strip()
            if s not in found:
                found.append(s)
    return found


def fork_base_copyright(path):
    try:
        out = subprocess.run(
            ["git", "show", FORK_BASE + ":" + path],
            cwd=REPO, capture_output=True, text=True, encoding="utf-8", errors="replace",
        ).stdout
    except subprocess.CalledProcessError:
        return []
    return extract_copyright_lines(out)


def detect_bom_eol(data):
    bom = data.startswith(b"\xef\xbb\xbf")
    eol = "\n"
    idx = data.find(b"\n")
    if idx > 0 and data[idx - 1:idx] == b"\r":
        eol = "\r\n"
    return bom, eol


def main():
    whitelist = load_whitelist()
    base_paths, base_by_name = fork_base_tree()

    manifest = {}
    already = 0
    per_dir = Counter()

    for dp, dn, fn in os.walk(SRC):
        dn[:] = [d for d in dn if d not in SCAN_SKIP_DIRS]
        for f in sorted(fn):
            if os.path.splitext(f)[1] not in EXTS:
                continue
            path = os.path.join(dp, f).replace(os.sep, "/")
            rel = os.path.relpath(path, REPO).replace(os.sep, "/")
            with open(path, "rb") as fh:
                data = fh.read()
            text = data.decode("utf-8", errors="replace")
            bom, eol = detect_bom_eol(data)

            if GPL_MARK in text:
                already += 1
                continue

            # R1: whitelist
            if rel in whitelist:
                manifest[rel] = {"bucket": "C", "rule": "R1-whitelist",
                                 "evidence": whitelist[rel],
                                 "copyright_lines": None, "bom": bom, "eol": eol}
                per_dir[(rel.split("/")[1], "C")] += 1
                continue

            # R2: fork base
            if rel in base_paths:
                rule = "R2-base-path"
            elif os.path.basename(rel) in base_by_name:
                rule = "R2-base-content"
            elif COPYRIGHT_RE.search(text[:2048]):
                rule = "R2-upstream-copyright"
            else:
                rule = None

            if rule:
                lines = extract_copyright_lines(text)
                ev = rule
                if rule == "R2-base-content":
                    ev += " (base: " + ", ".join(base_by_name[os.path.basename(rel)]) + ")"
                if not lines and rule != "R2-upstream-copyright":
                    bl = fork_base_copyright(rel)
                    if bl:
                        lines = bl
                        ev += " +copyright-from-fork-base"
                manifest[rel] = {"bucket": "B", "rule": ev,
                                 "evidence": None,
                                 "copyright_lines": lines or None,
                                 "bom": bom, "eol": eol}
                per_dir[(rel.split("/")[1], "B")] += 1
                continue

            # R3: project-original
            manifest[rel] = {"bucket": "A", "rule": "R3-project-original",
                             "evidence": None, "copyright_lines": None,
                             "bom": bom, "eol": eol}
            per_dir[(rel.split("/")[1], "A")] += 1

    counts = Counter(v["bucket"] for v in manifest.values())
    with open(os.path.join(HERE, "manifest.json"), "w", encoding="utf-8") as fh:
        json.dump({"fork_base": FORK_BASE, "counts": dict(counts),
                   "already_with_gpl_text": already, "files": manifest},
                  fh, indent=1, ensure_ascii=True, sort_keys=True)

    print("== classify report ==")
    print("scanned C/C++ files        :", already + len(manifest))
    print("already with GPL text      :", already)
    print("manifest (no GPL text)     :", len(manifest))
    for b in ("A", "B", "C"):
        print(f"  bucket {b}:", counts.get(b, 0))
    print()
    print("-- bucket C (never stamped) --")
    for p, v in sorted(manifest.items()):
        if v["bucket"] == "C":
            print("  ", p)
    print()
    print("-- per-dir breakdown --")
    dirs = sorted({d for d, _ in per_dir})
    for d in dirs:
        row = " ".join(f"{b}={per_dir[(d, b)]}" for b in ("A", "B", "C") if per_dir[(d, b)])
        print(f"  src/{d:<24} {row}")
    print()
    print("-- B with copyright lines (will be preserved) --")
    n = 0
    for p, v in sorted(manifest.items()):
        if v["bucket"] == "B" and v["copyright_lines"]:
            n += 1
            if n <= 15:
                print(f"  {p}: {v['copyright_lines']}")
    print(f"  ... total {n} B files carry copyright lines")
    print()
    print("-- A sample (first 15) --")
    for i, p in enumerate(sorted(p for p, v in manifest.items() if v["bucket"] == "A")):
        if i >= 15:
            break
        print("  ", p)
    print()
    print("manifest written:", os.path.join(HERE, "manifest.json"))
    return 0


if __name__ == "__main__":
    sys.exit(main())
