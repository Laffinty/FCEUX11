#!/usr/bin/env python3
"""Phase 8 配套 — 在所有 PreviousRun 构造点加 vendor_state: None 字段（兼容 v1.17 baseline）。"""
import pathlib
import re

files = [
    "D:/Project/FCEUX11/src/rust/crates/f11qa/src/report/baseline.rs",
    "D:/Project/FCEUX11/src/rust/crates/f11qa/src/report/matrix.rs",
    "D:/Project/FCEUX11/src/rust/crates/f11qa/src/report/grade.rs",
]

pattern = re.compile(r"(PreviousRun\s*\{)(.*?)(\n\s*\}\s*\n)", re.DOTALL)


def fix_block(body):
    if "vendor_state" in body:
        return body
    m = re.search(r"^(\s*)(results:\s*[^\n]+),?$", body, re.MULTILINE)
    if not m:
        return body
    indent = m.group(1)
    return body.replace(m.group(0), m.group(0) + "\n" + indent + "vendor_state: None,", 1)


for f in files:
    p = pathlib.Path(f)
    src = p.read_text(encoding="utf-8")
    matches = list(pattern.finditer(src))
    if not matches:
        print(f"  {p.name}: no PreviousRun blocks")
        continue
    new = src
    fixed_count = 0
    for m in matches:
        body = m.group(2)
        fixed = fix_block(body)
        if fixed != body:
            new = new.replace(m.group(0), m.group(1) + fixed + m.group(3), 1)
            fixed_count += 1
    if new != src:
        p.write_text(new, encoding="utf-8")
        print(f"  {p.name}: fixed {fixed_count}/{len(matches)} PreviousRun blocks")
    else:
        print(f"  {p.name}: no change ({len(matches)} blocks already have vendor_state)")