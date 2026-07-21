#!/usr/bin/env python3
"""Fix i18n script bugs from PR-B that block Task 1.4 CI gate validation."""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Fix 1: i18n_coverage.ps1 — replace GetAttribute() with XmlElement property syntax.
# Also: count type="needs-review" as unfinished (used by i18n_translate.py to
# flag priority contexts as needing human review after machine translation).
cov_path = ROOT / "scripts" / "i18n_coverage.ps1"
content = cov_path.read_text(encoding="utf-8")
old = """            $tr = $msg.translation
            $isUnfinished = $false
            if ($null -eq $tr) {
                $isUnfinished = $true
            } else {
                # If translation has type="unfinished" attribute
                if ($tr.GetAttribute('type') -eq 'unfinished') {
                    $isUnfinished = $true
                }"""
new = """            $tr = $msg.translation
            $isUnfinished = $false
            if ($null -eq $tr) {
                $isUnfinished = $true
            } else {
                # XmlElement attribute access via property syntax
                $trType = $tr.type
                if ($null -ne $trType -and $trType -eq 'unfinished') {
                    $isUnfinished = $true
                }
                # type='needs-review' is set by i18n_translate.py for
                # priority contexts (Debugger / TAS Editor) that need
                # human review after machine translation.
                if ($null -ne $trType -and $trType -eq 'needs-review') {
                    $isUnfinished = $true
                }"""
if old in content:
    content = content.replace(old, new)
    cov_path.write_text(content, encoding="utf-8")
    print(f"OK: {cov_path} GetAttribute bug fixed")
else:
    print(f"WARN: {cov_path} pattern not found")

# Add UTF-8 BOM to both .ps1 files (PowerShell 5.1 needs BOM to detect UTF-8)
for fname in ["scripts/i18n_coverage.ps1", "scripts/check_simp_trad.ps1"]:
    p = ROOT / fname
    data = p.read_bytes()
    if data[:3] != b"\xef\xbb\xbf":
        p.write_bytes(b"\xef\xbb\xbf" + data)
        print(f"OK: {p} BOM added")
    else:
        print(f"OK: {p} already has BOM")