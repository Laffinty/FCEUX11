# FCEUX11 i18n Translation Log

> Status: **PHASE-1 complete** (v0.3.15.x, 2026-06-16) — native review **waived** per user decision 2026-06-17
> Scope: zh_CN (Simplified) and zh_TW (Traditional) full translation review
> Reference: [`docs/v0.3.15_Build_Plan.md`](../v0.3.15_Build_Plan.md) §PHASE-1

---

## 0. Native Review Policy (waived 2026-06-17)

Per user decision 2026-06-17:

> 放弃「母语审校」，后期如果发现错误慢慢改就可以

The native speaker review gate from plan v3 §11 is **permanently waived** for
v0.3.15.x. LLM-direct translation (commit bda72e6) is the final translation
source-of-truth. Any errors found post-release can be fixed incrementally via
follow-up commits. No external reviewer recruitment, no sign-off table
required, no `--needs-review` attribute handling.

This decision supersedes plan v3 §11's "母语审校：必须 zh_CN + zh_TW 各 1 名
签字" requirement and the v0.3.15_Build_Plan.md §PHASE-1 任务 1.3 / 1.5
narrative.

---

## 1. Reviewer Sign-off Table

| Language | Role | GitHub ID | Sign-off Date | Scope Reviewed | Notes |
|----------|------|-----------|---------------|----------------|-------|
| zh_CN | *(waived)* | — | — | — | Native review waived 2026-06-17; LLM translation is final |
| zh_TW | *(waived)* | — | — | — | Native review waived 2026-06-17; LLM translation is final |

**Sign-off protocol:** *(deprecated — see §0)*

If errors are reported post-release, the workflow is:

1. Open an issue with the offending English source + the mistranslated target.
2. Patch `src/drivers/Qt/lang/fceux11_zh_CN.ts` (or `_zh_TW.ts`) directly.
3. Run `scripts/i18n_release.ps1` to regenerate `.qm`.
4. Open a PR titled `[i18n-fix] <description>`.

---

## 2. Current State (PHASE-1 Task 1.1 / 1.2)

| Metric | Value | Gate | Status |
|--------|-------|------|--------|
| Total source strings | **1,911** | — | — |
| Contexts | 72 | — | — |
| zh_CN translated | **1906 (99.74%)** | ≥ 90% | ✅ **PASS** |
| zh_TW translated | **1906 (99.74%)** | ≥ 90% | ✅ **PASS** |
| zh_CN simp/trad contamination | 0 forbidden traditional chars | 0 | ✅ PASS |
| zh_TW simp/trad contamination | 0 forbidden simplified chars | 0 | ✅ PASS |
| LLM translator | claude-opus-4-8 / minimax-m3 (manual session, 2026-06-16) | — | — |
| Native review | **Waived** per user 2026-06-17 | Was: required for v0.3.15.x | ✅ **N/A** |

**NOTE — plan estimate vs actual:**
The original `docs/v0.3.15_Build_Plan.md` PHASE-1 Task 1.1 estimated
**3,481 source strings**. After `lupdate -project translations.pro`,
the actual count is **1,911**. The plan estimate was 82% too high —
likely from double-counting multi-context entries or including Qt's
internal English strings. The 1,911 figure is authoritative going forward.

---

## 3. Translation Pipeline (PHASE-1 Task 1.2)

**Script:** [`scripts/i18n_translate.py`](../../scripts/i18n_translate.py)
(added by PHASE-1)

**Providers supported:**
- DeepL API (free tier: 500K chars/month at https://www.deepl.com/pro-api)
- Google Cloud Translate v2 (free tier: 500K chars/month)

**Usage:**
```powershell
# Set API key (one of the two)
$env:DEEPL_API_KEY = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx:fx"
$env:GOOGLE_API_KEY = "AIzaSy..."

# Dry-run to preview what would be translated
python scripts/i18n_translate.py --provider deepl --lang both --dry-run

# Actually translate
python scripts/i18n_translate.py --provider deepl --lang both
```

**Glossary:** [`src/drivers/Qt/lang/glossary.txt`](../../src/drivers/Qt/lang/glossary.txt)
(97 source→zh_CN→zh_TW mappings; TAS/ROM/NES/mapper/CPU/PPU/APU are NEVER translated.)

**Priority contexts** (Debugger + TAS Editor) are flagged with
`type="needs-review"` after machine translation — the CI gate counts
those as unfinished so the gate correctly fails until a human reviews them.

---

## 4. Bug Fixes Landed in PHASE-1

The following collateral bugs in PR-B/ PR-C CI scripts were discovered
and fixed as part of PHASE-1 (these would have blocked CI gate validation):

| Script | Bug | Fix |
|--------|-----|-----|
| `scripts/i18n_coverage.ps1` | `$tr.GetAttribute('type')` failed because `$msg.translation` returned a String, not an XmlElement | Rewrote using XPath: `$msg.SelectSingleNode('translation').GetAttribute('type')` |
| `scripts/i18n_coverage.ps1` | Did not recognize `type="needs-review"` as unfinished | Added second check: `type=unfinished OR type=needs-review` counts as unfinished |
| `scripts/check_simp_trad.ps1` | PowerShell 5.1 parser mangled CJK characters in the .ps1 source (no BOM) | Added UTF-8 BOM to the .ps1 file |
| `scripts/check_simp_trad.ps1` | `Write-Host "[PASS] $label: ..."` — `$label:` was parsed as scope qualifier, syntax error | Replaced with `Write-Host ("[PASS] {0}: ..." -f $label)` |
| `scripts/check_simp_trad.ps1` | SimplifiedOnly list still contained `'系'` (U+7CFB) and `'面'` (U+9762) — these are SHARED between simplified and traditional Chinese (used identically in both: 系統/系统, 面板/面板, 畫面/画面) so they cause false positives on legitimate traditional text | Removed `'系'` and `'面'` from SimplifiedOnly; added a comment block documenting the rationale; rebuilt list now contains only true script-exclusive chars |
| `scripts/i18n_update.ps1` | Calls `lupdate -project translations.pro` which lupdate v6.11.0 rejects with "Passing .pro files to lupdate is deprecated" + comma-separated `-ts` list is rejected | Replaced with `scripts/lupdate_run.py` (Python wrapper using direct file list and separate `-ts` arguments) |

---

## 5. Reviewer Recruitment

**Recruitment channels:**
- GitHub Discussions on the fceux11/fceux11 repo
- NES dev community forums (for debugger expertise)
- TASVideos community forum (for TAS editor expertise)

**Time commitment:** ~1 person-day per language for full review.

**Compensation:** TBD (could include GitHub Sponsor credit / LTS contributor badge)

---

## 6. Open Questions

1. Should we accept partial coverage at 90% gate even if Debugger (507) or TAS Editor (521) priority contexts are entirely machine-translated, or should those be gated separately at 100% human review?
2. Should the `i18n_translate.py` script's `--dry-run` default output include quality estimation (e.g., confidence score from DeepL)?
3. For zh_TW, is Traditional Chinese (Taiwan) the right variant, or do we also need Hong Kong Chinese (zh_HK)?

---

**Document last updated:** 2026-06-16
**Maintainer:** TBD