#!/usr/bin/env python3
"""Clean up the distinguishing char lists in check_simp_trad.ps1.

The original lists from PR-B mistakenly include some characters that
have the SAME form in both Simplified and Traditional Chinese
(件/存/入/出/etc.). Those chars cannot be used to detect cross-script
contamination. This script rewrites the lists to only include chars
that are TRULY distinguishing.

Strategy:
  1. Take the union of both original lists
  2. From that union, exclude any char that appears in BOTH lists
     (i.e., shared by both scripts = not distinguishing)
  3. Classify remaining chars as Traditional-only or Simplified-only
     based on which list they came from
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ps1_path = ROOT / "scripts" / "check_simp_trad.ps1"
content = ps1_path.read_text(encoding="utf-8")

# Parse out the two original lists from the .ps1 file
import re

def extract_list(text, var_name):
    # Find: $var = @(\n    'a', 'b', ...\n)
    pattern = rf'\$\s*{var_name}\s*=\s*@\((.*?)\)' 
    m = re.search(pattern, text, re.DOTALL)
    if not m:
        return []
    block = m.group(1)
    chars = re.findall(r"'([^'])'", block)
    return chars

trad_orig = set(extract_list(content, "TraditionalOnly"))
simp_orig = set(extract_list(content, "SimplifiedOnly"))
print(f"Original TraditionalOnly: {len(trad_orig)} chars")
print(f"Original SimplifiedOnly:  {len(simp_orig)} chars")

# Find truly distinguishing chars: chars that appear in only one list
shared = trad_orig & simp_orig
print(f"Shared (BOTH lists, NOT distinguishing): {len(shared)} chars: {sorted(shared)}")

trad_only = sorted(trad_orig - simp_orig)
simp_only = sorted(simp_orig - trad_orig)
print(f"True Traditional-only: {len(trad_only)}")
print(f"True Simplified-only:  {len(simp_only)}")

# Reformat into PowerShell list syntax (one line per group of 10)
def fmt_list(chars):
    rows = []
    for i in range(0, len(chars), 10):
        chunk = chars[i:i+10]
        rows.append("    '" + "', '".join(chunk) + "'")
    return "(\n" + ",\n".join(rows) + "\n)"

new_trad_block = f"$TraditionalOnly = {fmt_list(trad_only)}"
new_simp_block = f"$SimplifiedOnly = {fmt_list(simp_only)}"

# Replace the old blocks
new_content = re.sub(
    r'\$\s*TraditionalOnly\s*=\s*@\(.*?\)',
    new_trad_block,
    content,
    count=1,
    flags=re.DOTALL,
)
new_content = re.sub(
    r'\$\s*SimplifiedOnly\s*=\s*@\(.*?\)',
    new_simp_block,
    new_content,
    count=1,
    flags=re.DOTALL,
)

# Add a header note about the cleanup
header_note = """# v0.3.15 PHASE-1 fix: rebuilt TraditionalOnly/SimplifiedOnly lists
# to remove chars that appear in BOTH lists (i.e., shared between
# scripts and thus NOT distinguishing). Previously the script
# flagged shared chars like 件/存/入/出 as "traditional", which
# gave false positives on legitimate simplified text.
#
# True distinguishing character pairs covered (sample):
#   軟/软, 體/体, 網/网, 訊/讯, 時/时, 當/当, 機/机, 電/电, 腦/脑
#   頭/头, 麵/面, 麥/麦, 齒/齿, 輪/轮, 蘭/兰, 關/关, 閉/闭, 開/开
#   ... (full list is {0} trad-only, {1} simp-only chars)
""".format(len(trad_only), len(simp_only))

new_content = new_content.replace(
    "# Traditional-only / Simplified-only representative glyphs.",
    "# Traditional-only / Simplified-only representative glyphs.\n" + header_note,
    1,
)

ps1_path.write_text(new_content, encoding="utf-8")
# Re-add BOM
data = ps1_path.read_bytes()
if data[:3] != b"\xef\xbb\xbf":
    ps1_path.write_bytes(b"\xef\xbb\xbf" + data)
print(f"OK: {ps1_path} cleaned and BOM verified")