#!/usr/bin/env python3
"""Quick state check for .ts files after lupdate."""
import xml.etree.ElementTree as ET
from pathlib import Path

LANG_DIR = Path(__file__).resolve().parent.parent / "src" / "drivers" / "Qt" / "lang"

for fname in ["fceux11_en.ts", "fceux11_zh_CN.ts", "fceux11_zh_TW.ts"]:
    ts = LANG_DIR / fname
    if not ts.exists():
        print(f"{fname}: NOT FOUND")
        continue
    tree = ET.parse(ts)
    root = tree.getroot()
    total = sum(len(c.findall("message")) for c in root.findall("context"))
    ctx_count = len(root.findall("context"))
    unfinished = 0
    translated_samples = []
    unfinished_samples = []
    for c in root.findall("context"):
        ctx_name = c.get("name", "?")
        for m in c.findall("message"):
            src = (m.find("source").text or "") if m.find("source") is not None else ""
            tr = m.find("translation")
            is_unfinished = False
            tr_text = ""
            if tr is None:
                is_unfinished = True
            else:
                if tr.get("type") == "unfinished":
                    is_unfinished = True
                tr_text = (tr.text or "")
                if not tr_text.strip():
                    is_unfinished = True
            if is_unfinished:
                unfinished += 1
                if len(unfinished_samples) < 3:
                    unfinished_samples.append(f"[{ctx_name}] {src[:60]}")
            else:
                if len(translated_samples) < 3:
                    translated_samples.append(f"[{ctx_name}] {src[:40]} -> {tr_text[:40]}")
    pct = ((total - unfinished) / total * 100) if total else 0.0
    print(f"\n{fname}: {ctx_count} contexts, {total} messages, {unfinished} unfinished ({pct:.2f}% translated)")
    print("  Translated samples:")
    for s in translated_samples:
        print(f"    {s}")
    print("  Unfinished samples:")
    for s in unfinished_samples:
        print(f"    {s}")