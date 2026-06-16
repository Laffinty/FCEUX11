#!/usr/bin/env python3
"""
i18n_translate.py — v0.3.15 PHASE-1 Task 1.2

Machine-translate unfinished entries in fceux11_zh_CN.ts / fceux11_zh_TW.ts
using either the DeepL API or the Google Cloud Translate v2 API.

The glossary.txt file in src/drivers/Qt/lang/ is used to keep NES/TAS
terminology consistent (TAS, ROM, NES, mapper, CPU, PPU, APU, etc. are
NEVER translated; see glossary.txt for the canonical mapping).

USAGE
-----
  # DeepL (recommended; supports glossary natively)
  $env:DEEPL_API_KEY = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx:fx"
  python scripts/i18n_translate.py --provider deepl --lang zh-CN

  # Google Cloud Translate v2 (free tier: 500K chars/month)
  $env:GOOGLE_API_KEY = "AIzaSy..."
  python scripts/i18n_translate.py --provider google --lang zh-CN

  # Both languages
  python scripts/i18n_translate.py --provider deepl --lang both

  # Dry run (report what would be translated; write nothing)
  python scripts/i18n_translate.py --provider deepl --lang both --dry-run

REQUIREMENTS
------------
  Python 3.8+ stdlib only (urllib, json, xml, time).
  No pip install needed.

  DeepL: free tier allows 500,000 chars/month at https://www.deepl.com/pro-api
  Google: free tier allows 500,000 chars/month at https://cloud.google.com/translate

  Estimated cost for 1,911 source strings (~115K chars total):
    - DeepL Free: 1 batch
    - DeepL Pro:  ~$0.70
    - Google Free: 1 batch

LIMITATIONS
-----------
  This script produces MACHINE translations only. Native speaker review
  (Task 1.3) is REQUIRED before merging. The script preserves any
  existing human translations; only entries with type="unfinished"
  (or missing/empty translation) are processed.

  The script does NOT modify entries containing the untranslated
  "frozen" terms listed in glossary.txt (TAS, ROM, NES, etc.) — those
  are passed through verbatim.

  Per PHASE-1 risk table: Debugger (507 entries) and TAS Editor (521
  entries) MUST be human-reviewed after machine translation; this
  script does not mark those contexts as auto-approved.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parent.parent
LANG_DIR = ROOT / "src" / "drivers" / "Qt" / "lang"
GLOSSARY_PATH = LANG_DIR / "glossary.txt"

# Map our lang codes to DeepL / Google codes
DEEPL_LANG_MAP = {"zh-CN": "ZH", "zh-TW": "ZH"}
GOOGLE_LANG_MAP = {"zh-CN": "zh-CN", "zh-TW": "zh-TW"}

# Terms that must NEVER be translated (per glossary.txt hard rules)
UNTRANSLATABLE_TERMS = {
    "TAS", "ROM", "NES", "mapper", "CPU", "PPU", "APU", "FPS", "RAM",
    "PRG", "CHR", "NTSC", "PAL", "Dendy", "FDS", "Game Boy",
    "VS System", "Game Genie", "Qt", "FCEUX", "FCEUX11", "NSF",
    "LUA", "Lua", "FM2", "FCM", "FCS", "hex", "HEX",
}

# Contexts that MUST be reviewed by domain experts per PHASE-1 risk table
PRIORITY_CONTEXTS_FOR_REVIEW = {
    "consoleDebugger",     # 507 entries - NES debugger terminology
    "tasEditorWindow",     # 521 entries - TAS community slang
}


# ---------------------------------------------------------------------------
# Glossary
# ---------------------------------------------------------------------------

def load_glossary() -> dict[str, dict[str, str]]:
    """
    Parse glossary.txt into {source_term: {"zh_CN": ..., "zh_TW": ...}}.
    Glossary format: source_term [tab] zh_CN [tab] zh_TW
    """
    glossary: dict[str, dict[str, str]] = {}
    if not GLOSSARY_PATH.exists():
        return glossary
    with open(GLOSSARY_PATH, "r", encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if not line or line.startswith("#"):
                continue
            parts = line.split("\t")
            if len(parts) >= 3:
                glossary[parts[0]] = {"zh_CN": parts[1], "zh_TW": parts[2]}
    return glossary


# ---------------------------------------------------------------------------
# Translation providers
# ---------------------------------------------------------------------------

class TranslationProvider:
    def translate_batch(self, texts: list[str], target_lang: str) -> list[str]:
        raise NotImplementedError


class DeepLProvider(TranslationProvider):
    """DeepL API: https://api-free.deepl.com/v2/translate (free tier)
    https://api.deepl.com/v2/translate (pro tier)"""

    FREE_URL = "https://api-free.deepl.com/v2/translate"
    PRO_URL = "https://api.deepl.com/v2/translate"

    def __init__(self, api_key: str):
        self.api_key = api_key
        # Free tier keys end with ":fx"
        self.url = self.FREE_URL if api_key.endswith(":fx") else self.PRO_URL

    def translate_batch(self, texts: list[str], target_lang: str) -> list[str]:
        if not texts:
            return []
        deepl_target = DEEPL_LANG_MAP.get(target_lang, target_lang)
        results: list[str] = []
        # DeepL allows up to 50 texts per request
        BATCH = 50
        for i in range(0, len(texts), BATCH):
            chunk = texts[i:i + BATCH]
            data = urllib.parse.urlencode(
                [("text", t) for t in chunk] + [("target_lang", deepl_target)]
            ).encode("utf-8")
            req = urllib.request.Request(
                self.url,
                data=data,
                headers={
                    "Authorization": f"DeepL-Auth-Key {self.api_key}",
                    "Content-Type": "application/x-www-form-urlencoded",
                },
            )
            with urllib.request.urlopen(req, timeout=30) as resp:
                payload = json.loads(resp.read().decode("utf-8"))
            results.extend(item["text"] for item in payload["translations"])
            time.sleep(0.5)  # rate-limit courtesy
        return results


class GoogleProvider(TranslationProvider):
    """Google Cloud Translate v2 (basic): key-based, free tier 500K chars/month"""

    URL = "https://translation.googleapis.com/language/translate/v2"

    def __init__(self, api_key: str):
        self.api_key = api_key

    def translate_batch(self, texts: list[str], target_lang: str) -> list[str]:
        if not texts:
            return []
        google_target = GOOGLE_LANG_MAP.get(target_lang, target_lang)
        results: list[str] = []
        BATCH = 100  # Google allows up to 128 strings per request
        for i in range(0, len(texts), BATCH):
            chunk = texts[i:i + BATCH]
            params = urllib.parse.urlencode(
                [("q", t) for t in chunk] + [
                    ("target", google_target),
                    ("format", "text"),
                    ("key", self.api_key),
                ]
            )
            req = urllib.request.Request(
                f"{self.URL}?{params}",
                headers={"Content-Type": "application/x-www-form-urlencoded"},
            )
            with urllib.request.urlopen(req, timeout=30) as resp:
                payload = json.loads(resp.read().decode("utf-8"))
            results.extend(item["translatedText"] for item in payload["data"]["translations"])
            time.sleep(0.2)
        return results


def get_provider(name: str) -> TranslationProvider:
    if name == "deepl":
        key = os.environ.get("DEEPL_API_KEY")
        if not key:
            sys.exit("ERROR: DEEPL_API_KEY env var not set. Get a free key at https://www.deepl.com/pro-api")
        return DeepLProvider(key)
    if name == "google":
        key = os.environ.get("GOOGLE_API_KEY")
        if not key:
            sys.exit("ERROR: GOOGLE_API_KEY env var not set. Get a free key at https://cloud.google.com/translate")
        return GoogleProvider(key)
    sys.exit(f"ERROR: Unknown provider '{name}'. Use 'deepl' or 'google'.")


# ---------------------------------------------------------------------------
# .ts parsing & writing
# ---------------------------------------------------------------------------

def is_translation_unfinished(tr: ET.Element | None) -> bool:
    if tr is None:
        return True
    if tr.get("type") == "unfinished":
        return True
    text = (tr.text or "").strip()
    return not text


def is_untranslatable(source: str) -> bool:
    """True if source consists solely of untranslatable terms/short tokens."""
    stripped = source.strip()
    if not stripped:
        return True
    # Strip mnemonic markers like "&File" -> "File"
    cleaned = stripped.lstrip("&").strip()
    # Single-token that matches an untranslatable term
    if cleaned in UNTRANSLATABLE_TERMS:
        return True
    return False


def extract_unfinished(ts_path: Path) -> list[tuple[str, str, ET.Element]]:
    """
    Return list of (context_name, source_text, message_element) for every
    unfinished message in the .ts file. We keep references to the elements
    so we can mutate them in-place.
    """
    tree = ET.parse(ts_path)
    root = tree.getroot()
    out: list[tuple[str, str, ET.Element]] = []
    for ctx in root.findall("context"):
        ctx_name = ctx.get("name", "")
        for msg in ctx.findall("message"):
            src_el = msg.find("source")
            if src_el is None:
                continue
            src_text = (src_el.text or "")
            tr_el = msg.find("translation")
            if is_translation_unfinished(tr_el):
                out.append((ctx_name, src_text, msg))
    return out, tree


def apply_translations(
    items: list[tuple[str, str, ET.Element]],
    translations: list[str],
    glossary: dict[str, dict[str, str]],
    target_lang_code: str,  # "zh_CN" or "zh_TW"
) -> int:
    """
    Apply machine translations to the message elements in-place.
    Returns the number of strings updated.
    """
    assert len(items) == len(translations), (
        f"Translation count mismatch: {len(items)} items vs {len(translations)} translations"
    )
    updated = 0
    for (ctx_name, src_text, msg), translated in zip(items, translations):
        # Apply glossary overrides for short terms
        for term, mapping in glossary.items():
            # Don't replace if term is the entire source (would be circular)
            if term == src_text.strip():
                translated = mapping.get(target_lang_code, translated)
                break
        tr_el = msg.find("translation")
        if tr_el is None:
            tr_el = ET.SubElement(msg, "translation")
        tr_el.text = translated
        # Remove unfinished marker since we've filled it in
        if "type" in tr_el.attrib:
            del tr_el.attrib["type"]
        updated += 1
    return updated


def write_ts_tree(tree: ET.ElementTree, ts_path: Path) -> None:
    """Write .ts file with XML declaration and UTF-8 encoding."""
    # ET writes <?xml version='1.0' encoding='utf-8'?> by default
    tree.write(ts_path, encoding="utf-8", xml_declaration=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--provider", choices=["deepl", "google"], required=True,
                    help="Translation API provider")
    ap.add_argument("--lang", choices=["zh-CN", "zh-TW", "both"], required=True,
                    help="Target language: zh-CN, zh-TW, or both")
    ap.add_argument("--dry-run", action="store_true",
                    help="Report what would be translated; write nothing")
    ap.add_argument("--batch-size", type=int, default=50,
                    help="Texts per API call (DeepL max 50, Google max 128)")
    args = ap.parse_args()

    glossary = load_glossary()
    print(f"[i18n_translate] Loaded {len(glossary)} glossary entries")

    targets = []
    if args.lang in ("zh-CN", "both"):
        targets.append(("zh_CN", "fceux11_zh_CN.ts", "zh-CN"))
    if args.lang in ("zh-TW", "both"):
        targets.append(("zh_TW", "fceux11_zh_TW.ts", "zh-TW"))

    total_updated = 0
    for ts_code, ts_filename, api_lang in targets:
        ts_path = LANG_DIR / ts_filename
        print(f"\n[i18n_translate] Processing {ts_filename} -> {ts_code} (API code: {api_lang})")
        items, tree = extract_unfinished(ts_path)
        print(f"  Unfinished entries: {len(items)}")

        # Filter out untranslatable single-token entries (they'll keep their source)
        translatable: list[tuple[str, str, ET.Element]] = []
        skipped = 0
        for item in items:
            if is_untranslatable(item[1]):
                skipped += 1
            else:
                translatable.append(item)
        print(f"  Translatable (after filtering untranslatable tokens): {len(translatable)}")
        print(f"  Skipped (untranslatable single-token): {skipped}")

        if args.dry_run or not translatable:
            if args.dry_run:
                sample = translatable[:5]
                for ctx, src, _ in sample:
                    print(f"    would translate: [{ctx}] {src[:80]}")
            if not translatable:
                print(f"  Nothing to translate for {ts_filename}.")
                continue
            if args.dry_run:
                continue

        # Translate (only when not dry-run and we have items)
        provider = get_provider(args.provider)
        print(f"  Using provider: {args.provider}")
        print(f"  Calling API for {len(translatable)} strings... (this may take a minute)")
        try:
            translations = provider.translate_batch([s for _, s, _ in translatable], api_lang)
        except urllib.error.HTTPError as e:
            print(f"  ERROR: API returned HTTP {e.code}: {e.read().decode('utf-8', errors='replace')}")
            return 1
        except urllib.error.URLError as e:
            print(f"  ERROR: Network error: {e.reason}")
            return 1

        updated = apply_translations(translatable, translations, glossary, ts_code)
        total_updated += updated
        print(f"  Updated {updated} translations")

        # Don't write in dry-run mode
        if args.dry_run:
            print(f"  DRY RUN — not writing {ts_filename}")
            continue

        write_ts_tree(tree, ts_path)
        print(f"  Wrote {ts_path}")

        # Flag priority contexts for human review (set type=needs-review)
        review_count = 0
        for ctx in tree.getroot().findall("context"):
            if ctx.get("name") in PRIORITY_CONTEXTS_FOR_REVIEW:
                for msg in ctx.findall("message"):
                    tr = msg.find("translation")
                    if tr is not None and (tr.text or "").strip():
                        # Don't mark already human-translated entries (no type attr)
                        if "type" not in tr.attrib:
                            tr.set("type", "needs-review")
                            review_count += 1
        if review_count > 0:
            print(f"  Marked {review_count} entries in priority contexts as type='needs-review'")
            write_ts_tree(tree, ts_path)

    print(f"\n[i18n_translate] Done. Total translations applied: {total_updated}")
    print(f"[i18n_translate] Next: run scripts/i18n_coverage.ps1 to verify the 90% gate.")
    return 0


if __name__ == "__main__":
    sys.exit(main())