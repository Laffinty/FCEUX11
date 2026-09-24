#!/usr/bin/env python3
"""
F11QA v1.8 Phase 8 — 生成 v1.8 frozen baseline (120 项 kgmqa_id keyed)

输入:
  - tests/tests.json (v1.8 schema, 120 cases)
  - tests/fixtures/f11qa_baseline_frozen.json (v1.17 baseline, 47 entries keyed by v1.17 id)

策略:
  - v1.17 baseline 通过 legacy_id 字段映射到 v1.8 kgmqa_id，保留 v1.17 known-limit surface
  - 73 个新 v1.8 用例按 vendor_state 默认值:
      vendored + non-rom-suite → True (期望 PASS)
      advisory / pending-vendor → False (跑挂也 skip，对齐 v1.7 fail_to_fail 通道)
  - 输出新文件 tests/fixtures/f11qa_baseline_frozen.json (v1.8 schema)
  - 备份 v1.17 到 tests/fixtures/f11qa_baseline_frozen.v1.17.json

输出 schema (v1.8):
{
  "schema_version": "1.8",
  "generated_at": "...",
  "run_id": "v1.8-baseline-frozen",
  "results": { kgmqa_id: bool, ... },        # 120 项
  "rom_suite_vendor_state": { kgmqa_id: "vendored"|"advisory"|"pending-vendor" }  # 78 项
}
"""
import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TESTS_JSON = REPO_ROOT / "tests" / "tests.json"
BASELINE_V17 = REPO_ROOT / "tests" / "fixtures" / "f11qa_baseline_frozen.json"
BASELINE_V17_BACKUP = REPO_ROOT / "tests" / "fixtures" / "f11qa_baseline_frozen.v1.17.json"
BASELINE_V18 = REPO_ROOT / "tests" / "fixtures" / "f11qa_baseline_frozen.json"


def default_pass_for(case: dict) -> bool:
    """新 v1.8 用例的默认值。
    - rom-suite + vendored → True (期望 PASS)
    - rom-suite + advisory / pending-vendor → False (skip 通道，fail_to_pass 不计)
    - 非 rom-suite → True (期望 PASS)
    """
    vs = case.get("vendor_state")
    kinds = case.get("kind", [])
    is_rom_suite = "rom-suite" in kinds
    if is_rom_suite and vs in ("advisory", "pending-vendor"):
        return False
    return True


def lookup_v17(baseline_results: dict, case: dict) -> bool | None:
    """从 v1.17 baseline 通过 legacy_id / kgmqa_id 查 v1.17 PASS/FAIL；
    找不到返回 None（调用方走 default_pass_for）。"""
    legacy = case.get("legacy_id")
    if legacy and legacy in baseline_results:
        return baseline_results[legacy]
    if case["kgmqa_id"] in baseline_results:
        return baseline_results[case["kgmqa_id"]]
    return None


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dry-run", action="store_true", help="只打印，不写文件")
    args = ap.parse_args()

    with TESTS_JSON.open(encoding="utf-8") as f:
        tests = json.load(f)
    if tests.get("schema_version") != "1.8":
        print(f"ERROR: tests.json schema_version is {tests.get('schema_version')!r}, expected '1.8'", file=sys.stderr)
        return 1
    cases = tests["cases"]
    print(f"tests.json: {len(cases)} cases (schema_version 1.8)")

    v17_results: dict = {}
    if BASELINE_V17.exists():
        with BASELINE_V17.open(encoding="utf-8") as f:
            v17 = json.load(f)
        v17_results = v17.get("results", {})
        print(f"v1.17 baseline: {len(v17_results)} entries (generated_at={v17.get('generated_at')})")

    # 合并
    results: dict[str, bool] = {}
    vendor_state_map: dict[str, str] = {}
    mapped = 0
    unmapped_kept_default = 0
    for c in cases:
        kgid = c["kgmqa_id"]
        v17_pass = lookup_v17(v17_results, c)
        if v17_pass is not None:
            results[kgid] = v17_pass
            mapped += 1
        else:
            results[kgid] = default_pass_for(c)
            unmapped_kept_default += 1
        vs = c.get("vendor_state")
        if vs:
            vendor_state_map[kgid] = vs

    print(f"\nmerge:")
    print(f"  v1.17 baseline preserved (legacy_id lookup): {mapped}")
    print(f"  new v1.8 cases (default by vendor_state):  {unmapped_kept_default}")
    print(f"  total: {len(results)} entries")
    print(f"  rom_suite_vendor_state: {len(vendor_state_map)} entries (78 expected)")

    # 校验 vendored/advisory/pending-vendor 总数与 tests.json 一致
    for vs_name in ("vendored", "advisory", "pending-vendor"):
        cnt = sum(1 for v in vendor_state_map.values() if v == vs_name)
        print(f"    {vs_name}: {cnt}")
    assert len(vendor_state_map) == 78, f"vendor_state map should have 78 entries, got {len(vendor_state_map)}"
    assert len(results) == 120, f"results should have 120 entries, got {len(results)}"

    new_baseline = {
        "schema_version": "1.8",
        "generated_at": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "run_id": "v1.8-baseline-frozen",
        "results": results,
        "rom_suite_vendor_state": vendor_state_map,
    }

    if args.dry_run:
        print("\n[dry-run] would write:", BASELINE_V18)
        print("[dry-run] would backup v1.17 to:", BASELINE_V17_BACKUP)
        return 0

    # 备份 v1.17 baseline
    if BASELINE_V17.exists() and not BASELINE_V17_BACKUP.exists():
        import shutil

        shutil.copy2(BASELINE_V17, BASELINE_V17_BACKUP)
        print(f"\nbackup: {BASELINE_V17} -> {BASELINE_V17_BACKUP}")

    # 写 v1.8 baseline
    with BASELINE_V18.open("w", encoding="utf-8") as f:
        json.dump(new_baseline, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"write: {BASELINE_V18}")
    return 0


if __name__ == "__main__":
    sys.exit(main())