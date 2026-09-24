#!/usr/bin/env python3
"""
F11QA v1.8 — fetch_roms_from_mirror.py
单一 fetch 脚本（取代 22 个 download_<suite>_roms.ps1）。
按 §四 §4.6.2 协议从 Laffinty/f11qa-rom-mirror 拉取 65 条 rom-suite 用例的 ROM 字节，
校验 SHA-256，并把 vendor_state=vendored 的 ROM 复制到 tests/fixtures/。

为什么用 Python 而不是 PowerShell：
- PowerShell 5.1 (Windows) 的 ConvertFrom-Json 对 UTF-8 无 BOM 解析有硬限制
- Python 3 原生 UTF-8 + JSON 跨 Windows / Linux 一致
- CI workflow 5.4 调用：python scripts/fetch_roms_from_mirror.py

用法：
  python scripts/fetch_roms_from_mirror.py                  # 完整流程
  python scripts/fetch_roms_from_mirror.py --dry-run       # 只打印动作不执行
  python scripts/fetch_roms_from_mirror.py --skip-verify    # 跳过 SHA-256 校验（不推荐）
  python scripts/fetch_roms_from_mirror.py --skip-fetch     # 复用现有 snapshot
  python scripts/fetch_roms_from_mirror.py --help
"""
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

def log(*a, **kw):
    print(*a, **kw)
    sys.stdout.flush()

def die(msg, code=1):
    log(f"ERROR: {msg}")
    sys.exit(code)

def read_pin(path: Path) -> dict:
    if not path.exists():
        die(f"Pin file not found: {path}")
    with path.open('r', encoding='utf-8') as f:
        pin = json.load(f)
    for field in ('mirror_repo', 'mirror_ref', 'mirror_commit_sha'):
        if not pin.get(field):
            die(f"Pin missing required field: {field}")
    return pin

def read_rom_suite_cases(tests_json_path: Path) -> list:
    if not tests_json_path.exists():
        die(f"tests.json not found: {tests_json_path}")
    with tests_json_path.open('r', encoding='utf-8') as f:
        j = json.load(f)
    if j.get('schema_version') != '1.8':
        die(f"tests.json schema_version must be '1.8', got {j.get('schema_version')}")
    return [c for c in j.get('cases', []) if 'rom-suite' in c.get('kind', [])]

def ensure_snapshot(pin: dict, snapshot_dir: Path, skip_fetch: bool, dry_run: bool):
    """git clone mirror snapshot to snapshot_dir.
    优先用 mirror_ref tag（如 v1.8.0-mirror），tag 不存在时 fallback 到 mirror_commit_sha。
    注：git clone 不支持 commit SHA 作为第一参数，必须先 clone 默认 branch，再 fetch + checkout。
    """
    if skip_fetch and snapshot_dir.exists():
        log(f"[snapshot] reusing existing {snapshot_dir}")
        return

    ref = pin['mirror_ref']
    sha = pin['mirror_commit_sha']

    # 探测 tag 是否存在
    tag_exists = False
    try:
        r = subprocess.run(['git', 'ls-remote', '--tags', pin['mirror_repo'], ref],
                           capture_output=True, text=True, encoding='utf-8')
        if r.returncode == 0 and r.stdout.strip():
            tag_exists = True
    except Exception:
        pass

    if dry_run:
        if tag_exists:
            log(f"[dry] git clone --depth=1 --branch {ref} {pin['mirror_repo']} {snapshot_dir}")
        else:
            log(f"[dry] (tag not published; fallback plan:)")
            log(f"[dry]   git clone --depth=50 {pin['mirror_repo']} {snapshot_dir}")
            log(f"[dry]   (cd {snapshot_dir} && git fetch --depth=1 origin {sha[:12]} && git checkout {sha[:12]})")
        return

    if snapshot_dir.exists():
        log(f"[snapshot] removing stale {snapshot_dir}")
        # Windows 文件锁容错: git pack 文件可能被 OS 暂时锁住
        def _on_rm_error(func, path, exc_info):
            try:
                os.chmod(path, 0o777)
            except Exception:
                pass
            try:
                func(path)
            except Exception as e:
                log(f"  [rm-warn] {func.__name__} {path}: {e}")
        shutil.rmtree(snapshot_dir, onerror=_on_rm_error)
        if snapshot_dir.exists():
            log(f"  [rm-warn] {snapshot_dir} still exists (partial cleanup); will be overwritten by git clone")

    if tag_exists:
        log(f"[snapshot] cloning tag {ref} from {pin['mirror_repo']}...")
        clone_args = ['git', 'clone', '--depth=1', '--branch', ref,
                      pin['mirror_repo'], str(snapshot_dir)]
        r = subprocess.run(clone_args, capture_output=True, text=True, encoding='utf-8')
        if r.returncode != 0:
            die(f"git clone (tag) failed: {r.stderr.strip()}")
    else:
        log(f"[snapshot] tag {ref!r} not yet published; fetching commit {sha[:12]}...")
        # 先 shallow clone 默认 branch，再 fetch + checkout 目标 SHA
        clone_args = ['git', 'clone', '--depth=50', pin['mirror_repo'], str(snapshot_dir)]
        r = subprocess.run(clone_args, capture_output=True, text=True, encoding='utf-8')
        if r.returncode != 0:
            die(f"git clone (depth=50) failed: {r.stderr.strip()}")
        # fetch + checkout 目标 SHA
        fetch_args = ['git', 'fetch', '--depth=1', 'origin', sha]
        r = subprocess.run(fetch_args, cwd=str(snapshot_dir),
                           capture_output=True, text=True, encoding='utf-8')
        if r.returncode != 0:
            die(f"git fetch {sha[:12]} failed: {r.stderr.strip()}")
        checkout_args = ['git', 'checkout', sha]
        r = subprocess.run(checkout_args, cwd=str(snapshot_dir),
                           capture_output=True, text=True, encoding='utf-8')
        if r.returncode != 0:
            die(f"git checkout {sha[:12]} failed: {r.stderr.strip()}")
        log(f"[snapshot] checked out {sha[:12]}")

def verify_sha256sums(snapshot_dir: Path, skip_verify: bool, dry_run: bool):
    """sha256sum -c SHA256SUMS.txt --strict."""
    if skip_verify:
        log("[verify] SKIPPED (--skip-verify)")
        return
    if dry_run:
        log("[verify] SKIPPED (--dry-run; would run: sha256sum -c SHA256SUMS.txt --strict)")
        return
    sums_file = snapshot_dir / 'SHA256SUMS.txt'
    if not sums_file.exists():
        die(f"SHA256SUMS.txt missing in {snapshot_dir}")
    log("[verify] sha256sum -c SHA256SUMS.txt --strict")
    r = subprocess.run(['sha256sum', '-c', 'SHA256SUMS.txt', '--strict'],
                       cwd=snapshot_dir, capture_output=True, text=True, encoding='utf-8')
    if r.returncode != 0:
        log(r.stdout)
        log(r.stderr)
        die("sha256sum check FAILED")
    ok = sum(1 for line in r.stdout.splitlines() if line.endswith(': OK'))
    log(f"[verify] {ok} entries passed")

def parse_sha256sums(snapshot_dir: Path) -> dict:
    """返回 {relative_path: sha256_lowercase}"""
    sums = {}
    p = snapshot_dir / 'SHA256SUMS.txt'
    if not p.exists():
        return sums
    for line in p.read_text(encoding='utf-8', errors='replace').splitlines():
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        parts = line.split(None, 1)
        if len(parts) != 2:
            continue
        sha, path = parts[0].lower(), parts[1].lstrip('*').strip()
        sums[path] = sha
    return sums

def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(65536), b''):
            h.update(chunk)
    return h.hexdigest().lower()

def copy_roms(cases: list, snapshot_dir: Path, output_dir: Path,
              dry_run: bool, sha_index: dict) -> tuple:
    """复制 vendor_state=vendored 的 ROM 到 output_dir。
    返回 (ok_count, fail_count, skip_advisory, skip_pending)。"""
    ok = 0
    fail = 0
    skip_advisory = 0
    skip_pending = 0
    for c in cases:
        kgid = c['kgmqa_id']
        vs = c.get('vendor_state')
        mirror_path = c.get('mirror_path', '').replace('\\', '/')
        if vs == 'pending-vendor':
            log(f"  [skip-pending]   {kgid} vendor_state=pending-vendor")
            skip_pending += 1
            continue
        if vs == 'advisory':
            log(f"  [skip-advisory]  {kgid} vendor_state=advisory")
            skip_advisory += 1
            continue
        # vendored
        src = snapshot_dir / mirror_path
        if not src.exists() and not dry_run:
            log(f"  [MISS]           {kgid}: {mirror_path} not in snapshot")
            fail += 1
            continue
        leaf = Path(mirror_path).name
        dst = output_dir / leaf
        if dry_run:
            log(f"  [dry-vendored]   {kgid} <- {mirror_path}")
            ok += 1
            continue
        shutil.copy2(src, dst)
        # SHA-256 校验
        expected = sha_index.get(mirror_path)
        if expected is None:
            log(f"  [NO-EXPECTED-HASH] {kgid}: {mirror_path} not listed in SHA256SUMS.txt")
            fail += 1
            continue
        actual = sha256_file(dst)
        if actual != expected:
            log(f"  [HASH-MISMATCH] {kgid}: expected={expected[:12]} actual={actual[:12]}")
            fail += 1
            continue
        log(f"  [ok]             {kgid} <- {mirror_path}")
        ok += 1
    return ok, fail, skip_advisory, skip_pending

def main():
    p = argparse.ArgumentParser(description='F11QA v1.8 fetch ROMs from mirror')
    p.add_argument('--pin', type=Path,
                   default=REPO_ROOT / 'tests/fixtures/f11qa_mirror_pin.json')
    p.add_argument('--tests-json', type=Path,
                   default=REPO_ROOT / 'tests/tests.json')
    p.add_argument('--output-dir', type=Path,
                   default=REPO_ROOT / 'tests/fixtures')
    p.add_argument('--cache-dir', type=Path,
                   default=REPO_ROOT / 'tests/fixtures/_mirror_cache')
    p.add_argument('--snapshot-dir', type=Path,
                   default=REPO_ROOT / 'tests/fixtures/_mirror_snapshot')
    p.add_argument('--dry-run', action='store_true')
    p.add_argument('--skip-verify', action='store_true')
    p.add_argument('--skip-fetch', action='store_true')
    args = p.parse_args()

    log('=== F11QA fetch_roms_from_mirror.py ===')
    log('')

    pin = read_pin(args.pin)
    log(f"[pin] mirror_repo       = {pin['mirror_repo']}")
    log(f"[pin] mirror_ref        = {pin['mirror_ref']}")
    log(f"[pin] mirror_commit_sha = {pin['mirror_commit_sha'][:12]}...")
    log('')

    cases = read_rom_suite_cases(args.tests_json)
    n_vendored = sum(1 for c in cases if c.get('vendor_state') == 'vendored')
    n_advisory = sum(1 for c in cases if c.get('vendor_state') == 'advisory')
    n_pending = sum(1 for c in cases if c.get('vendor_state') == 'pending-vendor')
    log(f"[tests.json] rom-suite cases: total={len(cases)} "
        f"vendored={n_vendored} advisory={n_advisory} pending-vendor={n_pending}")
    log('')

    # 一致性：rom-suite 用例 mirror_ref 应等于 pin.mirror_ref
    ref_mismatch = [c for c in cases if c.get('mirror_ref') != pin['mirror_ref']]
    if ref_mismatch:
        log(f"[WARN] {len(ref_mismatch)} cases have mirror_ref != pin.mirror_ref:")
        for c in ref_mismatch[:5]:
            log(f"  - {c['kgmqa_id']}: mirror_ref={c.get('mirror_ref')}")

    # 1. git clone snapshot
    ensure_snapshot(pin, args.snapshot_dir, args.skip_fetch, args.dry_run)
    log('')

    # 2. SHA-256SUMS.txt 完整性
    verify_sha256sums(args.snapshot_dir, args.skip_verify, args.dry_run)
    log('')

    # 3. 解析 SHA-256SUMS.txt index
    sha_index = parse_sha256sums(args.snapshot_dir) if not args.dry_run else {}
    if sha_index:
        log(f"[index] {len(sha_index)} entries parsed from SHA256SUMS.txt")
        log('')

    # 4. 复制 ROM
    ok, fail, skip_adv, skip_pend = copy_roms(
        cases, args.snapshot_dir, args.output_dir, args.dry_run, sha_index)
    log('')
    log('=== fetch summary ===')
    log(f"  copied (vendored + sha256-ok): {ok}")
    log(f"  missing/mismatch/failed:       {fail}")
    log(f"  skipped (advisory):            {skip_adv}")
    log(f"  skipped (pending-vendor):      {skip_pend}")
    if fail > 0:
        die(f"{fail} ROM(s) failed fetch", code=4)

if __name__ == '__main__':
    main()
