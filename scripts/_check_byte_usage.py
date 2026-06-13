#!/usr/bin/env python3
"""FCEUX11 v0.3.10 — std::byte / uint8_t mixing risk scanner.

Scans the C++ source tree for patterns that are likely to break during the
P2 EMUFILE std::span<std::byte> migration. Reports per-file and aggregate
statistics; exit code is the number of HIGH-risk findings.

Risk categories:
  HIGH  - Arithmetic or ordering operations on std::byte (must use std::to_integer).
  HIGH  - reinterpret_cast between std::byte* and uint8_t*/u8* (use span + explicit conversion).
  HIGH  - C-style cast of std::byte* to arithmetic-pointer types.
  MED   - memcpy/memmove/memcmp with mixed std::byte and uint8_t/u8 pointers.
  MED   - File contains both std::vector<std::byte> and std::vector<u8>/std::vector<uint8_t>.
  LOW   - Raw std::byte pointer arithmetic (e.g. ptr + n).
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from dataclasses import dataclass, field
from typing import Dict, List


@dataclass
class Finding:
    path: Path
    line: int
    category: str
    risk: str
    text: str


@dataclass
class FileSummary:
    path: Path
    high: int = 0
    medium: int = 0
    low: int = 0
    mixed_vec: bool = False
    findings: List[Finding] = field(default_factory=list)


# Strip C++ line comments for cleaner matching.
def strip_line_comment(line: str) -> str:
    return re.sub(r"//.*", "", line)


def sanitize_templates(line: str) -> str:
    """Replace simple template arguments containing std::byte to avoid false
    positives from angle brackets (e.g. std::span<std::byte>)."""
    return re.sub(r"<[^<>]*\bstd::byte\b[^<>]*>", "<T>", line)


HIGH_PATTERNS = [
    # std::byte arithmetic / ordering (allowed: ==, !=, maybe explicit casts)
    ("std::byte arithmetic", re.compile(r"\bstd::byte\b.*[-+*/%]|[-+*/%].*\bstd::byte\b")),
    ("std::byte ordering", re.compile(r"\bstd::byte\b.*[<>=!]=|\bstd::byte\b\s*[<>]|[<>]\s*\bstd::byte\b")),
    # reinterpret_cast between byte and uint8_t/u8
    ("reinterpret_cast byte<->u8", re.compile(
        r"reinterpret_cast\s*<\s*(std::byte|uint8_t|u8)\s*\*\s*>|"
        r"reinterpret_cast\s*<\s*(?:const\s+)?(std::byte|uint8_t|u8)\s*\*\s*>"
    )),
    # C-style cast of std::byte pointer to arithmetic pointer
    ("C-style cast byte ptr", re.compile(r"\(\s*(uint8_t|u8|char|unsigned char|int8_t|s8)\s*\*\s*\)\s*.*std::byte")),
]

MEDIUM_PATTERNS = [
    # memcpy/memmove/memcmp with std::byte and u8/uint8_t in same call
    ("mixed byte/u8 memory op", re.compile(
        r"\b(memcpy|memmove|memcmp|memset)\s*\([^)]*\bstd::byte\b[^)]*\b(?:uint8_t|u8)\b|"
        r"\b(memcpy|memmove|memcmp|memset)\s*\([^)]*\b(?:uint8_t|u8)\b[^)]*\bstd::byte\b"
    )),
]

LOW_PATTERNS = [
    ("raw std::byte pointer arithmetic", re.compile(r"\bstd::byte\s*\*\s*\w+.*\+\s*\w+|\+\s*\w+.*\bstd::byte\s*\*")),
]

VEC_BYTE_RE = re.compile(r"std::vector\s*<\s*std::byte\s*>")
VEC_U8_RE = re.compile(r"std::vector\s*<\s*(?:u8|uint8_t)\s*>")


def scan_file(path: Path) -> FileSummary:
    summary = FileSummary(path=path)
    text = path.read_text(encoding="utf-8", errors="replace")
    has_vec_byte = bool(VEC_BYTE_RE.search(text))
    has_vec_u8 = bool(VEC_U8_RE.search(text))
    summary.mixed_vec = has_vec_byte and has_vec_u8

    for lineno, raw in enumerate(text.splitlines(), start=1):
        line = sanitize_templates(strip_line_comment(raw))

        for name, pattern in HIGH_PATTERNS:
            if pattern.search(line):
                summary.high += 1
                summary.findings.append(Finding(path, lineno, name, "HIGH", raw.strip()))

        for name, pattern in MEDIUM_PATTERNS:
            if pattern.search(line):
                summary.medium += 1
                summary.findings.append(Finding(path, lineno, name, "MED", raw.strip()))

        for name, pattern in LOW_PATTERNS:
            if pattern.search(line):
                summary.low += 1
                summary.findings.append(Finding(path, lineno, name, "LOW", raw.strip()))

    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description="Scan for std::byte / uint8_t mixing risks")
    parser.add_argument("--root", default="src", help="Root directory to scan")
    parser.add_argument("--include-tests", action="store_true", help="Also scan tests/")
    parser.add_argument("--details", action="store_true", help="Print every finding")
    parser.add_argument("--fail-high", type=int, default=0, help="Exit nonzero if HIGH findings exceed this")
    args = parser.parse_args()

    project_root = Path(__file__).resolve().parents[1]
    roots = [project_root / args.root]
    if args.include_tests:
        roots.append(project_root / "tests")

    extensions = {".cpp", ".c", ".h", ".hpp", ".cc"}
    summaries: List[FileSummary] = []

    for root in roots:
        if not root.exists():
            print(f"[WARN] path not found: {root}", file=sys.stderr)
            continue
        for path in root.rglob("*"):
            if path.is_file() and path.suffix.lower() in extensions:
                summaries.append(scan_file(path))

    total_high = sum(s.high for s in summaries)
    total_med = sum(s.medium for s in summaries)
    total_low = sum(s.low for s in summaries)
    mixed_vec_files = [s for s in summaries if s.mixed_vec]
    files_with_findings = [s for s in summaries if s.findings]

    print("=== FCEUX11 v0.3.10 std::byte / uint8_t Mixing Risk Scan ===")
    print(f"Roots          : {', '.join(str(r.relative_to(project_root)).replace(chr(92), '/') for r in roots)}")
    print(f"Files scanned  : {len(summaries)}")
    print(f"Files flagged  : {len(files_with_findings)}")
    print(f"HIGH findings  : {total_high}")
    print(f"MED findings   : {total_med}")
    print(f"LOW findings   : {total_low}")
    print(f"Mixed vec files: {len(mixed_vec_files)}")
    print()

    # Per-file summary table
    print("--- Per-file summary ---")
    print(f"{'HIGH':>5} {'MED':>5} {'LOW':>5} {'MIX':>4}  FILE")
    for s in sorted(summaries, key=lambda x: (x.high, x.medium, x.low), reverse=True):
        if not s.findings and not s.mixed_vec:
            continue
        rel = s.path.relative_to(project_root).as_posix()
        print(f"{s.high:>5} {s.medium:>5} {s.low:>5} {'Y' if s.mixed_vec else ' ':>4}  {rel}")
    print()

    if args.details:
        for s in sorted(files_with_findings, key=lambda x: x.path.as_posix()):
            print(f"--- {s.path.relative_to(project_root).as_posix()} ---")
            for f in s.findings:
                marker = {"HIGH": "!!!", "MED": " ! ", "LOW": " . "}[f.risk]
                print(f"{marker} L{f.line:4} [{f.risk}] {f.category}: {f.text}")
            print()

    if mixed_vec_files and not args.details:
        print("--- Files with both vector types (migration boundary) ---")
        for s in mixed_vec_files:
            print(f"  {s.path.relative_to(project_root).as_posix()}")
        print()

    exit_code = 0
    if args.fail_high and total_high > args.fail_high:
        print(f"[FAIL] HIGH findings {total_high} > threshold {args.fail_high}")
        exit_code = 1
    elif total_high == 0:
        print("[PASS] No HIGH-risk std::byte / uint8_t mixing patterns found.")
    else:
        print(f"[INFO] {total_high} HIGH-risk finding(s); review details with --details")

    return max(total_high, exit_code)


if __name__ == "__main__":
    sys.exit(main())
