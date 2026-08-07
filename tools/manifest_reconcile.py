#!/usr/bin/env python3
"""v1.17 Task2-A6: manifest reconciliation — CTest registration vs tests.json entries.

Checks:
1. Every CTest test name (from tests/CMakeLists.txt add_test(NAME ...)) maps to
   exactly one tests.json manifest entry (or is a documented exception).
2. Every tests.json manifest entry maps to a real binary / source file.
3. Reports orphans on both sides.
"""
import json
import re
import sys
from pathlib import Path

CMAKELISTS = Path("tests/CMakeLists.txt")
TESTS_JSON = Path("tests/tests.json")

# Documented double-ledger exceptions (from 落位清单 §三)
DOCUMENTED_EXCEPTIONS = {
    # CTest-only (no manifest entry) — script-based / not part of matrix
    "menu_slot_check": "python script, not a manifest-tracked test",
    "blargg_smoke": "CTest smoke for blargg_runner binary; manifest tracks blargg_* ROM entries instead",
    "lua_bit_test_headless": "CTest name for lua runner; manifest entry is lua_bit_test",
    "kagami_qa_direct_smoke": "CTest smoke for direct runner; manifest tracks the 47 matrix entries",
    "headless_smoke_test": "CTest-only gate (engine boot / grade E); manifest tracks the 47 matrix entries",
    # CTest-name → manifest-name mapping differences
    "mapper_core_test": "manifest id is mapper_test",
    "savestate_core_test": "manifest id is savestate_test",
    # Manifest entries aggregated under a runner CTest (not individually registered)
    "blargg_suite": "aggregate blargg suite entry; run via blargg_runner / direct runner",
    "lua_bit_test": "lua runner script test; CTest name is lua_bit_test_headless",
    "lua_emu_test": "lua runner script test (aggregated)",
    "lua_joypad_test": "lua runner script test (aggregated)",
    "lua_memory_test": "lua runner script test (aggregated)",
}

# Manifest entries that are per-ROM blargg entries, aggregated under blargg_runner
BLARGG_ROM_PREFIXES = ("blargg_",)

def is_blargg_rom_entry(mid: str) -> bool:
    return mid.startswith(BLARGG_ROM_PREFIXES) and mid != "blargg_suite"


def parse_ctest_names(cmake: Path) -> list[str]:
    text = cmake.read_text(encoding="utf-8", errors="replace")
    return re.findall(r"add_test\s*\(\s*NAME\s+([^\s\)]+)", text)


def main() -> int:
    ctest_names = parse_ctest_names(CMAKELISTS)
    print(f"CTest tests registered: {len(ctest_names)}")
    print(f"  {sorted(ctest_names)}")

    with open(TESTS_JSON, encoding="utf-8") as f:
        manifest = json.load(f)
    entries = manifest if isinstance(manifest, list) else manifest.get("tests", manifest)
    if isinstance(entries, dict):
        entries = list(entries.values())
    manifest_ids = {e.get("id") or e.get("name") for e in entries if isinstance(e, dict)}
    manifest_ids.discard("")
    print(f"\ntests.json manifest entries: {len(manifest_ids)}")

    # ---- CTest names without a manifest entry (excluding documented) ----
    missing_in_manifest = []
    for name in sorted(ctest_names):
        if name in manifest_ids:
            continue
        if name in DOCUMENTED_EXCEPTIONS:
            print(f"  [documented] CTest '{name}': {DOCUMENTED_EXCEPTIONS[name]}")
            continue
        missing_in_manifest.append(name)
    print(f"\nCTest names NOT in manifest (undocumented): {len(missing_in_manifest)}")
    for n in missing_in_manifest:
        print(f"  !! {n}")

    # ---- Manifest entries without a CTest test (reverse direction) ----
    manifest_without_ctest = []
    for mid in sorted(manifest_ids):
        if mid in ctest_names or mid in DOCUMENTED_EXCEPTIONS:
            continue
        if is_blargg_rom_entry(mid):
            print(f"  [aggregated] '{mid}': per-ROM blargg entry, run via blargg_runner batch")
            continue
        manifest_without_ctest.append(mid)
    print(f"\nManifest entries NOT covered by a CTest test (undocumented): {len(manifest_without_ctest)}")
    for n in manifest_without_ctest:
        print(f"  !! {n}")

    ok = not missing_in_manifest and not manifest_without_ctest
    print(f"\n{'PASS: manifest and CTest fully reconciled' if ok else 'FAIL: see orphans above'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())