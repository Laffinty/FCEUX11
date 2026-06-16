# FCEUX11 i18n Workflow (v0.3.15 PR-B)

## Quick start

```bash
# 1. Sync .ts from source (after adding/changing tr() calls)
lupdate -project translations.pro

# 2. Translate in Qt Linguist
linguist fceux11_zh_CN.ts
linguist fceux11_zh_TW.ts

# 3. Compile .ts -> .qm
lrelease -project translations.pro

# 4. CI coverage check (must be ≥ 90% for merge)
powershell scripts/i18n_coverage.ps1
```

## File layout

```
src/drivers/Qt/lang/
├── translations.pro        # Qt Linguist project
├── glossary.txt            # terminology table
├── README.md               # this file
├── fceux11_en.ts           # English (source language, 100% baseline)
├── fceux11_zh_CN.ts        # Simplified Chinese target
└── fceux11_zh_TW.ts        # Traditional Chinese target
```

## Why `src/drivers/Qt/lang/` instead of `assets/i18n/`?

The pre-v0.3.15 `assets/i18n/*.ts` files were hand-written menu placeholders
(101 entries, mostly the `consoleWin_t` menu items). They were NOT generated
by `lupdate` and never reflected the 3,481 `tr()` calls scattered across 59
source files.

v0.3.15 PR-B moves the authoritative source to `src/drivers/Qt/lang/` so that
`lupdate` can scan all source files automatically. The 101 hand-written
translations are preserved by `lupdate` source-string merging.

## Source string freeze

After v0.3.15 PR-B merges, the set of `tr()` source strings in
`src/drivers/Qt/` is **frozen**. Adding a new `tr()` call requires:

1. A v0.3.15.x hotfix PR with explicit `[i18n-string-add]` tag
2. Re-running `lupdate` to update all .ts files
3. Re-running translation for the new strings
4. CI re-runs the coverage gate

This freeze prevents the silent re-translation problem that occurs when
`lupdate` is re-run after every change (each new `tr()` source string is
marked `type="unfinished"` until translated).

## Translation rules

See `glossary.txt` for the canonical term table.

**Hard rules:**
- `TAS / ROM / NES / mapper / CPU / PPU / APU / FPS / RAM / PRG / CHR` — never translated
- File extensions (`.nes`, `.fm2`, `.fcm`, `.fcs`) — kept verbatim
- Shortcut mnemonics (`&`) — preserved as-is
- 4-byte hex addresses (`0x4010`) — kept as digits, do not translate "0x"

**Soft rules:**
- Prefer short noun phrases over full sentences
- Use the first match from `glossary.txt` when ambiguous
- zh_CN: avoid traditional glyphs (`軟體`, `檔案`, `網路`, `訊息`)
- zh_TW: avoid simplified glyphs (`软件`, `文件`, `网络`, `信息`)

## CI gate

`scripts/i18n_coverage.ps1` parses each `.ts` file and reports the
unfinished-percentage. **Coverage must be ≥ 90%** for zh_CN and zh_TW before
PR-C can merge. (v0.3.15 plan §11 originally required ≥ 95%; relaxed to
≥ 90% with machine translation + post-hoc native review per user
decision on 2026-06-16.)

`scripts/check_simp_trad.ps1` validates the simplified/traditional glyph
separation rule by checking the zh_CN.ts and zh_TW.ts files.

## What this PR does NOT cover

- Sub-dialog retranslateUi() — only `consoleWin_t` and `AboutWindow`
  currently override `changeEvent(QEvent::LanguageChange)`. 30+ other
  dialogs (ConsoleDebugger, TasEditor, ppuViewer, etc.) show stale text
  after language switch. This is tracked as deferred to v0.3.15.x.
- IME forwarding in 6 `keyPress override` files — partially fixed in PR-B;
  see commit message for the per-file patch list.
- TypedConfig<T> wrapper class — deferred to v0.4.x.
