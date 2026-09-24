"""
Phase 1 改名执行脚本（v0.2 + memory 适配）
- 按 §1.4 改名规则（KagamiQA→F11QA, kagamiqa→f11qa, kagami-qa→f11qa, kagami_qa→f11qa, kagami_→f11qa_, \\bkagami\\b→f11qa）
- 排除 docs/history/（保护历史归档命名）
- 排除 CHANGELOG.md / DERIVATIVE_WORK_NOTICE.txt（不可变历史）
- 修复 active 文件中指向 docs/history/*KagamiQA* 的链接（保留 history 命名避免 404）
- 写入保持 LF-only（避免 CRLF 污染）
- 实际操作 git mv + 文本替换

用法：
  python scripts/phase1_rename.py           # 实际执行
  python scripts/phase1_rename.py --dry     # dry-run，只打印
"""
import subprocess
import re
import sys
import os
import shutil

EXCLUDE_PREFIX = 'docs/history/'
EXCLUDE_EXACT = {
    'CHANGELOG.md',                       # 不可变历史
    'DERIVATIVE_WORK_NOTICE.txt',          # 衍生作品声明（法律文本）
    'LICENSE',                             # 仓库主许可证
    'COPYRIGHT_AUDIT.md',                  # 版权审计
    'readme.md',                           # 由独立 PR 维护（v1.8 §1.3 决定保留 README 简洁）
    '_dryrun_result.txt',                  # dry-run 产物（不进 git）
    'scripts/phase1_rename_dryrun.py',     # 本脚本自身
    'scripts/phase1_rename.py',            # 本脚本自身
}
TEXT_EXTS = ('.cpp','.h','.hpp','.cc','.c','.rs','.toml','.yml','.yaml','.md',
             '.json','.jsonc','.ps1','.sh','.py','.txt','.cmake','.gitignore',
             '.cfg','.in','.gradle','.properties')

def rename(name):
    out = name
    out = out.replace('KagamiQA', 'F11QA')
    out = out.replace('kagamiQA', 'F11QA')
    out = out.replace('kagamiqa', 'f11qa')
    out = out.replace('kagami-qa', 'f11qa')
    out = out.replace('kagami_qa', 'f11qa')
    out = re.sub(r'\bkagami\b', 'f11qa', out)
    out = re.sub(r'\bkagami_', 'f11qa_', out)
    return out

def is_excluded(path):
    if path.startswith(EXCLUDE_PREFIX):
        return True
    if path in EXCLUDE_EXACT:
        return True
    return False

def is_text_file(path):
    return path.endswith(TEXT_EXTS) or 'workflows' in path

def get_active_files():
    r = subprocess.run(['git', '-c', 'core.quotePath=false', 'ls-files'],
                       capture_output=True)
    return [f for f in r.stdout.decode('utf-8').splitlines() if f and not is_excluded(f)]

def stage1_git_mv(active, dry=False):
    """Step 1: git mv 所有路径含 kagami 的文件"""
    moves = []
    for src in active:
        if 'kagami' not in src.lower():
            continue
        dst = rename(src)
        if src == dst:
            continue
        moves.append((src, dst))

    print(f'[stage1] git mv: {len(moves)} moves')
    if dry:
        for src, dst in moves[:5]:
            print(f'   (dry) git mv {src} -> {dst}')
        if len(moves) > 5:
            print(f'   ... and {len(moves) - 5} more')
        return moves

    for src, dst in moves:
        # 先确保目标目录存在（git mv 不能自动建中间目录）
        src_dir = os.path.dirname(src)
        dst_dir = os.path.dirname(dst)
        if src_dir != dst_dir and dst_dir and not os.path.isdir(dst_dir):
            os.makedirs(dst_dir, exist_ok=True)
        r = subprocess.run(['git', 'mv', src, dst], capture_output=True)
        if r.returncode != 0:
            print(f'   FAIL: {src} -> {dst}')
            print(f'   stderr: {r.stderr.decode("utf-8", errors="replace")[:200]}')
        else:
            print(f'   OK   {src} -> {dst}')
    return moves

def stage2_text_replace(active, dry=False):
    """Step 2: 文本替换。处理 active 文件中的 kagami 字串。"""
    candidates = []
    for f in active:
        if not is_text_file(f):
            continue
        if 'kagami' in f.lower():
            continue  # 路径里已含 kagami，由 stage1 处理
        # 检查是否真含 kagami 文本
        r = subprocess.run(['git', 'grep', '-lIE', 'kagami|Kagami', '--', f],
                          capture_output=True)
        if r.returncode == 0 and r.stdout:
            candidates.append(f)

    print(f'[stage2] text replace: {len(candidates)} files')
    if dry:
        for f in candidates[:5]:
            print(f'   (dry) {f}')
        if len(candidates) > 5:
            print(f'   ... and {len(candidates) - 5} more')
        return candidates

    for f in candidates:
        # 读 working tree 当前内容（已被 stage1 改名后）
        try:
            with open(f, 'rb') as fp:
                data = fp.read()
        except FileNotFoundError:
            print(f'   SKIP (not found): {f}')
            continue
        # 保留原 line endings（BOM/CRLF 不破坏）
        # 但 Git index 是 CRLF（autocrlf=true），working tree 是 LF
        # 这里直接当文本处理，输出 LF
        text = data.decode('utf-8', errors='replace')
        new_text = rename(text)
        if new_text != text:
            # 写入 LF-only，避免 Windows 默认 CRLF 污染
            with open(f, 'wb') as fp:
                fp.write(new_text.encode('utf-8'))
            print(f'   OK   {f}')
    return candidates

def stage3_fix_history_links(active, dry=False):
    """Step 3: 强制把 active 文件中 docs/history/... 路径段还原成 KagamiQA/kagamiqa 命名。
    关键：
    - 只对 'docs/history/...' 路径段做反向 rename
    - 不影响同行的 active 路径引用（如 docs/tech/F11QA.md）
    - 独立运行，不依赖 stage2 是否写文件
    - 即使 stage2 没跑过，history 路径段也是 KagamiQA 命名（stage3 是 idempotent）
    """
    path_pat = re.compile(r'docs/history/[^\s`\'"<>\)\]\|]+')
    def reverse_path(m):
        p = m.group()
        p = p.replace('F11QA', 'KagamiQA')
        p = p.replace('f11qa', 'kagamiqa')
        return p

    fixes = []
    for f in active:
        if not is_text_file(f):
            continue
        try:
            with open(f, 'rb') as fp:
                text = fp.read().decode('utf-8', errors='replace')
        except FileNotFoundError:
            continue
        new_text = path_pat.sub(reverse_path, text)
        if new_text != text:
            for old_line, new_line in zip(text.splitlines(), new_text.splitlines()):
                if old_line != new_line:
                    fixes.append((f, old_line, new_line))
            if not dry:
                with open(f, 'wb') as fp:
                    fp.write(new_text.encode('utf-8'))
    print(f'[stage3] history link fixes: {len(fixes)} lines across {len({f for f,_,_ in fixes})} files')
    for f, old, new in fixes[:15]:
        print(f'   {f}')
        print(f'     - {old[:140]}')
        print(f'     + {new[:140]}')
    if len(fixes) > 15:
        print(f'   ... and {len(fixes) - 15} more')

def main():
    dry = '--dry' in sys.argv
    if dry:
        print('=== DRY RUN MODE ===')
    active = get_active_files()
    print(f'[init] {len(active)} active files')
    moves = stage1_git_mv(active, dry=dry)
    cands = stage2_text_replace(active, dry=dry)
    stage3_fix_history_links(active, dry=dry)
    print('[done]')
    if dry:
        print('Run without --dry to apply changes.')

if __name__ == '__main__':
    main()
