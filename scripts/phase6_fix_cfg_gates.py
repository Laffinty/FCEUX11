#!/usr/bin/env python3
"""Phase 6 配套 — 把 FFI cfg gate 从 any(feature, not(test)) 改成 feature-only。
理由：cargo test 会把 lib 当成 non-test 编译（与测试 bin 分开），导致 lib.rlib
里仍有未解析的 extern "C" 符号，传递到测试 bin 链接时失败。修复办法：把这些
FFI 模块和 impl 完全 feature-gate，只在 CMake direct-adapter 构建时启用。
"""
from pathlib import Path

OLD = '#[cfg(any(feature = "direct-adapter", not(test)))]'
NEW = '#[cfg(feature = "direct-adapter")]'

files = [
    "D:/Project/FCEUX11/src/rust/crates/f11qa/src/runner/mapper_byte_diff.rs",
    "D:/Project/FCEUX11/src/rust/crates/f11qa/src/runner/rom_regression.rs",
    "D:/Project/FCEUX11/src/rust/crates/f11qa/src/runner/savestate_regression.rs",
]

for f in files:
    p = Path(f)
    src = p.read_text(encoding="utf-8")
    cnt = src.count(OLD)
    if cnt == 0:
        print(f"  {p.name}: no change (0 occurrences)")
        continue
    new = src.replace(OLD, NEW)
    p.write_text(new, encoding="utf-8")
    print(f"  {p.name}: replaced {cnt} occurrences")