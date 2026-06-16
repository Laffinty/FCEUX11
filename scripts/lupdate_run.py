#!/usr/bin/env python3
"""
lupdate_run.py — v0.3.15 PHASE-1 Task 1.1
Wrapper around Qt's lupdate.exe that works around the comma-separated
-ts list bug by passing each .ts file as a separate argument.

Usage:
    python scripts/lupdate_run.py
"""
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANG_DIR = ROOT / "src" / "drivers" / "Qt" / "lang"
LUPDATE = ROOT / "vcpkg_installed" / "x64-windows" / "tools" / "Qt6" / "bin" / "lupdate.exe"

# Source dirs to scan (matches translations.pro SOURCES)
SRC_DIRS = [
    ROOT / "src" / "drivers" / "Qt",
]
SRC_SUB_DIRS = [
    ROOT / "src" / "drivers" / "Qt" / "TasEditor",
]

# .ts output files
TS_FILES = [
    LANG_DIR / "fceux11_en.ts",
    LANG_DIR / "fceux11_zh_CN.ts",
    LANG_DIR / "fceux11_zh_TW.ts",
]

# Exclude patterns
EXCLUDE_DIRS = {"build", "vcpkg_installed", "Testing", ".git"}


def collect_sources() -> list[str]:
    """Recursively collect .cpp/.h files under SRC_DIRS, excluding EXCLUDE_DIRS."""
    sources: list[str] = []
    seen: set[str] = set()
    roots = list(SRC_DIRS) + list(SRC_SUB_DIRS)
    for root in roots:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file():
                continue
            if path.suffix.lower() not in (".cpp", ".h"):
                continue
            # Skip excluded dirs
            if any(part in EXCLUDE_DIRS for part in path.parts):
                continue
            ap = str(path.resolve())
            if ap in seen:
                continue
            seen.add(ap)
            sources.append(ap)
    return sources


def main() -> int:
    if not LUPDATE.exists():
        print(f"ERROR: lupdate.exe not found at {LUPDATE}", file=sys.stderr)
        print("Install Qt 6.8 LTS LinguistTools via vcpkg.", file=sys.stderr)
        return 1

    sources = collect_sources()
    print(f"[lupdate_run] Sources: {len(sources)} files")
    print(f"[lupdate_run] Targets: {[t.name for t in TS_FILES]}")

    # CRITICAL: lupdate requires source files BEFORE -ts. If -ts comes
    # first, lupdate rejects the source files with "no recognized
    # extension" error. This is a v6.11.0 quirk on Windows.
    # Also: comma-separated -ts list is rejected; pass each .ts separately.
    args: list[str] = [str(LUPDATE), "-no-obsolete", "-silent"]
    args += sources
    for ts in TS_FILES:
        args += ["-ts", str(ts)]

    print(f"[lupdate_run] Running lupdate in {LANG_DIR}")
    proc = subprocess.run(
        args,
        cwd=str(LANG_DIR),
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )

    # lupdate prints diagnostics to stderr but exits 0 on success
    if proc.stdout:
        print("--- STDOUT ---")
        print(proc.stdout)
    if proc.stderr:
        # Filter out the well-known namespace warnings (cosmetic only)
        meaningful = [
            line for line in proc.stderr.splitlines()
            if "Qualifying with unknown namespace/class" not in line
        ]
        if meaningful:
            print("--- STDERR (warnings/errors) ---")
            for line in meaningful:
                print(line)
    print(f"[lupdate_run] lupdate exit code: {proc.returncode}")

    if proc.returncode != 0:
        return proc.returncode

    # Summary: count messages per .ts file
    import xml.etree.ElementTree as ET
    print("\n[lupdate_run] Coverage summary after lupdate:")
    for ts in TS_FILES:
        try:
            tree = ET.parse(ts)
            root = tree.getroot()
            total = 0
            unfinished = 0
            for ctx in root.findall("context"):
                for msg in ctx.findall("message"):
                    total += 1
                    tr = msg.find("translation")
                    if tr is None:
                        unfinished += 1
                        continue
                    if tr.get("type") == "unfinished":
                        unfinished += 1
                        continue
                    txt = (tr.text or "").strip()
                    if not txt:
                        unfinished += 1
            translated = total - unfinished
            pct = (translated / total * 100) if total else 0.0
            print(f"  {ts.name}: {translated}/{total} translated ({pct:.2f}%)")
        except Exception as e:
            print(f"  {ts.name}: parse error: {e}")

    return 0


if __name__ == "__main__":
    sys.exit(main())