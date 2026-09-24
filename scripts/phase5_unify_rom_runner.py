#!/usr/bin/env python3
"""
F11QA v1.8 — Phase 5 批量改写 tests.json 中 78 项 rom-suite 用例的 input 块。

目的：把 30+ 个 `f11qa_*_runner` 独立 binary 统一到 `f11qa-rom-runner` dispatcher。
同时修复 Phase 3 镜像源接入时残留的 `--rom tests/fixtures/<suite>/<basename>`
路径不一致问题（fetch 脚本复制到 `tests/fixtures/<basename>`，没保留 suite 子目录）。

改写规则：
- input.binary       → "f11qa-rom-runner"
- input.working_dir  → 不变（保持 "tests"）
- input.args         → ["--kgmqa-id", <kgmqa_id>, "--tests-json", "tests/tests.json", <forwarded>]
                        其中 forwarded 仅保留非路由、非派生的参数：
                          - --frames N    透传给底层 blargg runner
                        丢弃：
                          - --rom / --rom=    dispatcher 从 mirror_path 派生
                          - --log             kgmqa-048 nestest 由 dispatcher 派生
                          - --batch           kgmqa-043 由 dispatcher 用 blargg_manifest.json
- 其他字段（mirror_path / vendor_state / mirror_ref / license / expected / 等）不变

用法：
  python scripts/phase5_unify_rom_runner.py [--dry-run]
"""
import argparse
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TESTS_JSON = REPO_ROOT / "tests" / "tests.json"


def rewrite_args(case: dict) -> list:
    """根据 kgmqa_id 派生新的 input.args。"""
    kgmqa_id = case["kgmqa_id"]
    old_args = case.get("input", {}).get("args", [])

    # 透传规则：保留 --frames / --frames=N；丢弃 --rom / --rom= / --log / --log= / --batch
    forwarded = []
    i = 0
    while i < len(old_args):
        a = old_args[i]
        if a == "--rom" or a.startswith("--rom="):
            i += 2 if a == "--rom" else 1
            continue
        if a == "--log" or a.startswith("--log="):
            i += 2 if a == "--log" else 1
            continue
        if a == "--batch" or a.startswith("--batch="):
            i += 2 if a == "--batch" else 1
            continue
        forwarded.append(a)
        i += 1

    return ["--kgmqa-id", kgmqa_id, "--tests-json", "tests/tests.json", *forwarded]


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--dry-run", action="store_true", help="打印变更但不写回")
    args = ap.parse_args()

    if not TESTS_JSON.exists():
        print(f"ERROR: {TESTS_JSON} not found", file=sys.stderr)
        return 1

    with TESTS_JSON.open("r", encoding="utf-8") as f:
        data = json.load(f)

    cases = data.get("cases", [])
    n_rom_suite = 0
    n_changed = 0
    for c in cases:
        kind = c.get("kind", [])
        if isinstance(kind, str):
            kind = [kind]
        if "rom-suite" not in kind:
            continue
        n_rom_suite += 1
        old_binary = c.get("input", {}).get("binary", "")
        new_args = rewrite_args(c)
        new_binary = "f11qa-rom-runner"
        if c.get("input", {}).get("binary") == new_binary and c["input"]["args"] == new_args:
            continue
        if "input" not in c:
            c["input"] = {}
        c["input"]["binary"] = new_binary
        c["input"]["args"] = new_args
        # working_dir 保持不变（如有）；缺省补 "tests"
        c["input"].setdefault("working_dir", "tests")
        n_changed += 1
        if args.dry_run:
            print(f"  {c['kgmqa_id']}: {old_binary} → {new_binary}")

    print(f"rom-suite 用例总数: {n_rom_suite}")
    print(f"改写数量: {n_changed}")
    if args.dry_run:
        print("(dry-run: 未写回)")
        return 0

    if n_changed == 0:
        print("无需改写")
        return 0

    # 写回（保留 schema_version 与元字段）
    with TESTS_JSON.open("w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")
    print(f"已写回 {TESTS_JSON}")
    return 0


if __name__ == "__main__":
    sys.exit(main())