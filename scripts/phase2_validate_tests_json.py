"""
Phase 2 — tests.json v1.8 schema validator
按 v1.8 计划 §二 §2.4 + §六 §6.2 校验 120 项用例。
"""
import json
import re
import sys
from collections import Counter

ALLOWED_KINDS = {
    'unit-cpp', 'unit-hdr', 'unit-rust',
    'harness-cpp', 'harness-rust',
    'rom-suite',
    'static-analysis', 'static-license',
    'perf', 'lua-api', 'smoke',
}

ALLOWED_LAYERS = {'core', 'boards', 'driver', 'lua', 'benchmark', 'script'}

ALLOWED_LICENSES = {
    'PD', 'CC0', 'zlib', 'GPL-2.0', 'GPL-2.0-only', 'GPL-2.0-or-later',
    'GPL-3.0', 'GPL-3.0-only', 'GPL-3.0-or-later',
    'MIT', 'BSD', 'Apache', 'CC-BY',
}

ALLOWED_VENDOR_STATES = {'vendored', 'advisory', 'pending-vendor'}

ALLOWED_FAILURE_MEANS = {'blocking', 'advisory'}

KGMQA_ID_RE = re.compile(r'^kgmqa-(\d{3})-([a-z0-9-]+)$')

# 顶层 policy 默认值（v1.8 §六 §6.1）
REQUIRED_TOP = {'schema_version', 'suite_id', 'generated_at', 'policy', 'cases'}
REQUIRED_POLICY = {'license_allowed', 'mirror_repo', 'r4_gate_thresholds'}
REQUIRED_R4 = {'total_min', 'fail_to_pass_max', 'mirror_snapshot_check_required'}

REQUIRED_CASE = {
    'kgmqa_id', 'legacy_id', 'title', 'kind', 'layer',
    'spec_source', 'license', 'input', 'expected',
    'timeout_seconds', 'tags', 'failure_means', 'provenance',
}

REQUIRED_CASE_FOR_ROM_SUITE = {'mirror_ref', 'mirror_path', 'vendor_state'}

def fail(errors, msg):
    errors.append(msg)

def validate(path):
    with open(path, encoding='utf-8') as f:
        data = json.load(f)
    errors = []

    # 顶层字段
    missing_top = REQUIRED_TOP - set(data.keys())
    if missing_top:
        fail(errors, f'missing top-level fields: {sorted(missing_top)}')
        return errors

    if data['schema_version'] != '1.8':
        fail(errors, f"schema_version must be '1.8', got {data['schema_version']!r}")
    if data['suite_id'] != 'f11qa-v1.8':
        fail(errors, f"suite_id must be 'f11qa-v1.8', got {data['suite_id']!r}")

    # policy
    policy = data.get('policy', {})
    missing_policy = REQUIRED_POLICY - set(policy.keys())
    if missing_policy:
        fail(errors, f'missing policy fields: {sorted(missing_policy)}')
    r4 = policy.get('r4_gate_thresholds', {})
    missing_r4 = REQUIRED_R4 - set(r4.keys())
    if missing_r4:
        fail(errors, f'missing r4_gate_thresholds fields: {sorted(missing_r4)}')

    # cases
    cases = data.get('cases', [])
    if not isinstance(cases, list):
        fail(errors, f'cases must be a list, got {type(cases).__name__}')
        return errors

    # kgmqa_id 唯一 + 连续
    seen_ids = set()
    numbers = []
    for c in cases:
        kgid = c.get('kgmqa_id', '<missing>')
        if kgid in seen_ids:
            fail(errors, f'duplicate kgmqa_id: {kgid}')
        seen_ids.add(kgid)
        m = KGMQA_ID_RE.match(kgid)
        if not m:
            fail(errors, f'invalid kgmqa_id format: {kgid!r} (expected kgmqa-NNN-kebab)')
            continue
        numbers.append(int(m.group(1)))
    if numbers:
        numbers.sort()
        expected = list(range(1, len(numbers) + 1))
        if numbers != expected:
            fail(errors, f'kgmqa_id numbers not contiguous 1..N: missing={set(expected)-set(numbers)} extra={set(numbers)-set(expected)}')

    # total == 120 (R4 gate, §七)
    total_min = r4.get('total_min', 120)
    if len(cases) != total_min:
        fail(errors, f'total case count {len(cases)} != r4_gate_thresholds.total_min {total_min}')

    # 每条 case 字段校验
    for i, c in enumerate(cases):
        kgid = c.get('kgmqa_id', f'<case[{i}]>')
        prefix = f'[{kgid}]'

        # 必填字段
        missing = REQUIRED_CASE - set(c.keys())
        if missing:
            fail(errors, f'{prefix} missing fields: {sorted(missing)}')
            continue

        # kind
        kind = c.get('kind', [])
        if not isinstance(kind, list) or not kind:
            fail(errors, f'{prefix} kind must be non-empty list')
        else:
            bad_kinds = set(kind) - ALLOWED_KINDS
            if bad_kinds:
                fail(errors, f'{prefix} kind contains invalid: {sorted(bad_kinds)}')

        # layer
        if c.get('layer') not in ALLOWED_LAYERS:
            fail(errors, f"{prefix} layer {c.get('layer')!r} not in {sorted(ALLOWED_LAYERS)}")

        # license
        lic = c.get('license', '')
        if lic not in ALLOWED_LICENSES:
            fail(errors, f"{prefix} license {lic!r} not in accepted set")

        # rom-suite 额外必填
        if 'rom-suite' in kind:
            missing_rom = REQUIRED_CASE_FOR_ROM_SUITE - set(c.keys())
            if missing_rom:
                fail(errors, f'{prefix} (rom-suite) missing fields: {sorted(missing_rom)}')
            mirror_ref = c.get('mirror_ref', '')
            if mirror_ref and not re.match(r'^v\d+\.\d+\.\d+(-[a-z0-9-]+)?$', mirror_ref):
                # 允许 git tag 形式 (vX.Y.Z) 或 commit SHA (40 hex)
                if not re.match(r'^[a-f0-9]{40}$', mirror_ref):
                    fail(errors, f'{prefix} mirror_ref {mirror_ref!r} not in tag form vX.Y.Z or 40-hex commit SHA')
            vs = c.get('vendor_state', '')
            if vs not in ALLOWED_VENDOR_STATES:
                fail(errors, f"{prefix} vendor_state {vs!r} not in {sorted(ALLOWED_VENDOR_STATES)}")
            # vendor_state=advisory/pending-vendor 时，failure_means 默认应是 advisory
            if vs in ('advisory', 'pending-vendor') and c.get('failure_means') == 'blocking':
                # warning, not error
                print(f'  [WARN] {prefix} vendor_state={vs} but failure_means=blocking')

        # failure_means
        if c.get('failure_means') not in ALLOWED_FAILURE_MEANS:
            fail(errors, f"{prefix} failure_means {c.get('failure_means')!r} not in {sorted(ALLOWED_FAILURE_MEANS)}")

        # mirror_ref 一致性：所有 rom-suite 用例的 mirror_ref 应一致（指向同一 git tag/SHA）
        # 在 policy.mirror_repo + mirror_ref 字段组合下，建议同源；这里放宽：只校验格式

        # timeout_seconds
        if not isinstance(c.get('timeout_seconds'), int) or c['timeout_seconds'] <= 0:
            fail(errors, f'{prefix} timeout_seconds must be positive int')

        # kgmqa-117 应是 static-license
        if kgid == 'kgmqa-117-mirror-snapshot-check':
            if 'static-license' not in kind:
                fail(errors, f'kgmqa-117 must have static-license kind')

    # kgmqa-117 必须在 cases 列表
    kg117_present = any(c.get('kgmqa_id', '').startswith('kgmqa-117') for c in cases)
    if not kg117_present:
        fail(errors, 'kgmqa-117-mirror-snapshot-check missing from cases')

    return errors

def main():
    path = sys.argv[1] if len(sys.argv) > 1 else 'tests/tests.json'
    errors = validate(path)
    if errors:
        print(f'VALIDATION FAILED ({len(errors)} errors):')
        for e in errors:
            print(f'  - {e}')
        sys.exit(1)
    else:
        with open(path, encoding='utf-8') as f:
            data = json.load(f)
        print(f'OK: {len(data["cases"])} cases validated against schema v1.8')

if __name__ == '__main__':
    main()
