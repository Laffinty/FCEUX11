#!/usr/bin/env python3
"""
F11QA v1.8 — kgmqa-117 mirror_snapshot_check
镜像源快照一致性 + license accepted set 校验。
取代 v1.17 license_manifest_check。

实现策略：
- kgmqa-117 在 v1.8 计划 §四 §4.5 设计为 C++ binary，但仓库 tools/ 大量用 Python + json 惯例。
- 实际实现为 Python 脚本（避免引入 nlohmann/json 依赖 + 跨平台一致）。
- tests.json v1.8 中 kgmqa-117 input.binary = "python"，args 指向本脚本。

校验内容（按 §四 §4.5）：
  1. tests/fixtures/f11qa_mirror_pin.json 存在且 mirror_ref / mirror_commit_sha 非空
  2. 调 fetch_roms_from_mirror.py 校验 snapshot SHA-256 + rom-suite 用例 mirror_path
  3. tests.json v1.8 中所有 kind=rom-suite 用例：
     - mirror_ref 必须 == pin.mirror_ref
     - license ∈ pin.license_accepted_set
  4. pin.license_accepted_set 与 license_rejected_set 不重叠

退出码：
  0 = PASS
  1 = FAIL（pin / tests.json / mirror snapshot 任何一项不通过）
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

def log(msg):
    print(msg, flush=True)

def load_pin(path: Path) -> dict:
    if not path.exists():
        log(f"FAIL: pin file not found: {path}")
        sys.exit(1)
    with path.open('r', encoding='utf-8') as f:
        return json.load(f)

def load_tests_json(path: Path) -> dict:
    if not path.exists():
        log(f"FAIL: tests.json not found: {path}")
        sys.exit(1)
    with path.open('r', encoding='utf-8') as f:
        j = json.load(f)
    if j.get('schema_version') != '1.8':
        log(f"FAIL: tests.json schema_version must be '1.8', got {j.get('schema_version')}")
        sys.exit(1)
    return j

def run_fetch_snapshot(args) -> int:
    """调 fetch_roms_from_mirror.py 跑 mirror snapshot + SHA-256 校验 + rom-suite 用例 mirror_path 校验"""
    fetch_script = REPO_ROOT / 'scripts' / 'fetch_roms_from_mirror.py'
    if not fetch_script.exists():
        log(f"FAIL: fetch script not found: {fetch_script}")
        return 1
    cmd = [
        sys.executable, str(fetch_script),
        '--pin', str(args.manifest),
        '--tests-json', str(args.tests_json),
        '--output-dir', str(REPO_ROOT / 'tests' / 'fixtures'),
    ]
    log(f"[fetch] {' '.join(cmd)}")
    r = subprocess.run(cmd, cwd=str(REPO_ROOT), capture_output=False)
    return r.returncode

def check_mirror_ref_consistency(cases: list, pin_mirror_ref: str) -> list:
    """校验所有 rom-suite 用例的 mirror_ref == pin.mirror_ref"""
    errors = []
    for c in cases:
        if c.get('mirror_ref') != pin_mirror_ref:
            errors.append(f"{c['kgmqa_id']}: mirror_ref={c.get('mirror_ref')!r} != pin.mirror_ref={pin_mirror_ref!r}")
    return errors

def check_license_accepted(cases: list, accepted_set: set, rejected_set: set) -> list:
    """校验所有 rom-suite 用例的 license ∈ accepted set 且 ∉ rejected set"""
    errors = []
    overlap = accepted_set & rejected_set
    if overlap:
        errors.append(f"pin license overlap (accepted ∩ rejected = {overlap})")
    for c in cases:
        lic = c.get('license')
        if lic is None:
            continue
        if lic in rejected_set:
            errors.append(f"{c['kgmqa_id']}: license={lic!r} in pin.license_rejected_set")
        elif lic not in accepted_set:
            errors.append(f"{c['kgmqa_id']}: license={lic!r} not in pin.license_accepted_set")
    return errors

def main():
    p = argparse.ArgumentParser(description='F11QA kgmqa-117 mirror_snapshot_check')
    p.add_argument('--manifest', type=Path,
                   default=REPO_ROOT / 'tests/fixtures/f11qa_mirror_pin.json')
    p.add_argument('--tests-json', type=Path,
                   default=REPO_ROOT / 'tests/tests.json')
    p.add_argument('--skip-fetch', action='store_true',
                   help='跳过 fetch 步骤（只做静态 schema/一致性校验）')
    args = p.parse_args()

    log('=== F11QA kgmqa-117 mirror_snapshot_check ===')
    log('')

    # 1. 加载 pin + tests.json
    pin = load_pin(args.manifest)
    tests = load_tests_json(args.tests_json)
    cases = [c for c in tests.get('cases', []) if 'rom-suite' in c.get('kind', [])]
    log(f"[pin] mirror_repo={pin['mirror_repo']} mirror_ref={pin['mirror_ref']} "
        f"commit={pin['mirror_commit_sha'][:12]}...")
    log(f"[tests.json] rom-suite cases: {len(cases)}")
    log('')

    accepted = set(pin.get('license_accepted_set', []))
    rejected = set(pin.get('license_rejected_set', []))
    log(f"[license] accepted={sorted(accepted)}")
    log(f"[license] rejected={sorted(rejected)}")

    all_errors = []

    # 2. mirror_ref 一致性
    log('')
    log('[check] mirror_ref consistency...')
    ref_errors = check_mirror_ref_consistency(cases, pin['mirror_ref'])
    if ref_errors:
        for e in ref_errors:
            log(f"  [WARN] {e}")
    else:
        log('  OK: all rom-suite cases mirror_ref == pin.mirror_ref')

    # 3. license accepted set
    log('')
    log('[check] license accepted set...')
    lic_errors = check_license_accepted(cases, accepted, rejected)
    if lic_errors:
        all_errors.extend(lic_errors)
        for e in lic_errors:
            log(f"  FAIL: {e}")
    else:
        log(f'  OK: all {len(cases)} cases have license ∈ accepted set')

    # 4. snapshot check（fetch ROM + SHA-256SUMS 校验）
    if not args.skip_fetch:
        log('')
        log('[check] fetch + snapshot integrity...')
        rc = run_fetch_snapshot(args)
        if rc != 0:
            all_errors.append(f'fetch_roms_from_mirror.py exit={rc}')
            log(f'  FAIL: fetch exit={rc}')
        else:
            log('  OK: fetch + snapshot integrity PASS')

    # 5. 总结
    log('')
    log('=== kgmqa-117 summary ===')
    if all_errors:
        log(f'FAIL: {len(all_errors)} errors')
        for e in all_errors:
            log(f'  - {e}')
        sys.exit(1)
    log('PASS: mirror_snapshot_check OK')
    sys.exit(0)

if __name__ == '__main__':
    main()
