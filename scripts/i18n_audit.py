#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""i18n_audit.py — v1.15 hotfix5
Read-only analysis tool: parse every non-English .ts file and output the
coverage matrix (native / identical-to-EN / unfinished / empty) plus the
actual native coverage percentage, matching the table in PLAN §2.2.

Usage:
    python scripts/i18n_audit.py                    # print matrix to stdout
    python scripts/i18n_audit.py -o output/i18n_audit.txt  # also write to file

Zero third-party dependencies (Python 3 stdlib only).
"""
import argparse
import os
import re
import xml.etree.ElementTree as ET

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
LANG_DIR = os.path.join(PROJECT_ROOT, "src", "drivers", "Qt", "lang")

LANGS = ["zh_CN", "zh_TW", "ja", "ko", "es", "fr", "de", "vi", "th", "hi", "ar"]

def norm(s: str) -> str:
    """Collapse whitespace and strip, matching the canonical normalization."""
    return re.sub(r"\s+", " ", s).strip()

def unesc(s: str) -> str:
    """Unescape XML entities."""
    return (s.replace("&amp;", "&").replace("&apos;", "'")
             .replace("&quot;", '"').replace("&lt;", "<").replace("&gt;", ">"))

def load_allowlist() -> set:
    """Load the en_keep_allowlist.txt whitelist."""
    path = os.path.join(LANG_DIR, "en_keep_allowlist.txt")
    allow = set()
    if not os.path.exists(path):
        return allow
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line and not line.startswith("#"):
                allow.add(line)
    return allow

def audit_language(lang: str, allow_set: set) -> dict:
    """Audit one .ts file and return counts."""
    ts_path = os.path.join(LANG_DIR, f"fceux11_{lang}.ts")
    if not os.path.exists(ts_path):
        return {"native": 0, "identical": 0, "unfinished": 0, "empty": 0, "total": 0}

    tree = ET.parse(ts_path)
    native = identical = unfinished = empty = 0

    for msg in tree.iter("message"):
        src_el = msg.find("source")
        tr_el = msg.find("translation")
        if src_el is None or not src_el.text:
            continue
        src_norm = norm(src_el.text)
        if not src_norm:
            continue

        if tr_el is None or tr_el.text is None or not tr_el.text.strip():
            empty += 1
            continue

        tr_type = tr_el.get("type", "")
        tr_norm = norm(tr_el.text)

        if tr_type in ("unfinished", "needs-review"):
            unfinished += 1
            continue

        # For non-English languages: translation == source means it stayed English.
        # Only acceptable if the source is in the allowlist.
        if tr_norm == src_norm:
            if src_norm in allow_set:
                native += 1  # whitelisted, count as OK
            else:
                identical += 1  # fake translation (EN pasted in)
        else:
            native += 1

    total = native + identical + unfinished + empty
    return {"native": native, "identical": identical,
            "unfinished": unfinished, "empty": empty, "total": total}

def main():
    parser = argparse.ArgumentParser(description="i18n coverage audit matrix")
    parser.add_argument("-o", "--output", help="Write matrix to this file as well")
    args = parser.parse_args()

    allow_set = load_allowlist()

    results = {}
    for lang in LANGS:
        results[lang] = audit_language(lang, allow_set)

    # Format output
    header = f"{'lang':<8} {'native':>8} {'identical':>10} {'unfinished':>11} {'empty':>6} {'total':>6} {'native%':>8}"
    sep = "-" * len(header)
    lines = [header, sep]

    for lang in LANGS:
        r = results[lang]
        total = r["total"]
        pct = (r["native"] / total * 100) if total > 0 else 0
        lines.append(f"{lang:<8} {r['native']:>8} {r['identical']:>10} {r['unfinished']:>11} {r['empty']:>6} {total:>6} {pct:>7.1f}%")

    lines.append(sep)
    lines.append(f"Total languages audited: {len(LANGS)}")
    lines.append(f"Allowlist entries: {len(allow_set)}")
    lines.append("")
    lines.append("Legend:")
    lines.append("  native      = non-empty translation that differs from English source")
    lines.append("  identical   = non-empty, no 'unfinished' flag, but text == English source (fake)")
    lines.append("  unfinished  = has type='unfinished' or 'needs-review'")
    lines.append("  empty       = empty or missing <translation>")

    output = "\n".join(lines)
    print(output)

    if args.output:
        os.makedirs(os.path.dirname(args.output) or ".", exist_ok=True)
        with open(args.output, "w", encoding="utf-8") as f:
            f.write(output + "\n")
        print(f"\nMatrix written to {args.output}")

if __name__ == "__main__":
    main()
