#!/usr/bin/env python3
"""Compare C++ vs Rust blargg runner outputs for value-level parity.

Reads build/parity_cpp.json and build/parity_rust.json (each a JSON array
of per-ROM result objects with "name"/"value" fields), plus the stderr
PASS/FAIL lines as a cross-check.

Exit 0 if every ROM's value matches; 1 otherwise (prints the diff).
"""
import json
import re
import sys
from pathlib import Path

CPP = Path("build/parity_cpp.json")
RUST = Path("build/parity_rust.json")
CPP_ERR = Path("build/parity_cpp.stderr")
RUST_ERR = Path("build/parity_rust.stderr")


def load_json(path: Path):
    if not path.exists() or path.stat().st_size == 0:
        print(f"FAIL: {path} missing or empty")
        sys.exit(2)
    data = json.loads(path.read_text(encoding="utf-8", errors="replace"))
    # Accept both a bare list and {"roms": [...]}
    if isinstance(data, dict):
        data = data.get("roms", data.get("results", []))
    out = {}
    for r in data:
        if isinstance(r, dict):
            name = r.get("name") or r.get("rom") or r.get("rom_name")
            value = r.get("value")
            if name is not None and value is not None:
                # Normalise: strip .nes / .fds extension so C++ ("apu_01.nes")
                # and Rust ("apu_01") keys collide.
                stem = str(name)
                for ext in (".nes", ".fds", ".nsf", ".fc0"):
                    if stem.lower().endswith(ext):
                        stem = stem[: -len(ext)]
                        break
                out[stem] = value
    return out


def parse_stderr(path: Path):
    """Parse '[name] N frames... PASS/FAIL (0x..)' lines from stderr."""
    text = path.read_text(encoding="utf-8", errors="replace")
    pat = re.compile(r"\[([^\]]+)\]\s+\d+\s+frames.*?(PASS|FAIL)\s+\(0x([0-9A-Fa-f]+)\)")
    out = {}
    for m in pat.finditer(text):
        out[m.group(1)] = int(m.group(3), 16)
    return out


def main():
    cpp = load_json(CPP)
    rust = load_json(RUST)
    print(f"CPP  JSON entries: {len(cpp)}")
    print(f"RUST JSON entries: {len(rust)}")

    # Prefer JSON; fall back to stderr parse for either side if empty
    cpp_stderr = parse_stderr(CPP_ERR)
    rust_stderr = parse_stderr(RUST_ERR)
    print(f"CPP  stderr entries: {len(cpp_stderr)}")
    print(f"RUST stderr entries: {len(rust_stderr)}")

    if not cpp:
        cpp = cpp_stderr
    if not rust:
        rust = rust_stderr

    all_names = sorted(set(cpp) | set(rust))
    print(f"Union of ROMs: {len(all_names)}")

    mismatches = []
    missing = []
    for name in all_names:
        c = cpp.get(name)
        r = rust.get(name)
        if c is None or r is None:
            missing.append((name, c, r))
        elif c != r:
            mismatches.append((name, c, r))

    print()
    if missing:
        print(f"MISSING on one side ({len(missing)}):")
        for name, c, r in missing:
            print(f"  {name}: cpp={c} rust={r}")
    if mismatches:
        print(f"VALUE MISMATCHES ({len(mismatches)}):")
        for name, c, r in mismatches:
            try:
                ci = int(c, 16) if isinstance(c, str) else int(c)
                ri = int(r, 16) if isinstance(r, str) else int(r)
            except (ValueError, TypeError):
                print(f"  {name}: cpp={c!r} rust={r!r}")
                continue
            print(f"  {name}: cpp=0x{ci:02X} rust=0x{ri:02X}")
    if not missing and not mismatches:
        print(f"PASS: all {len(all_names)} ROMs byte-identical (value-level parity)")
        sys.exit(0)
    print(f"RESULT: FAIL ({len(mismatches)} mismatches, {len(missing)} missing)")
    sys.exit(1)


if __name__ == "__main__":
    main()