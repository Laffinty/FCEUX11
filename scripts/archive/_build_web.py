#!/usr/bin/env python3
"""
hotfix3 docs/index.html -> web/ multi-lang SEO split.

Reads the single SPA docs/index.html, extracts the I18N dictionary, the LANGS
array, the OG_LOCALE map, and the CSS block, then emits:
  - web/assets/styles.css     (verbatim CSS, no edits)
  - web/assets/app.js         (scroll/IntersectionObserver, no i18n dict)
  - web/<code>/index.html     (12 per-language pages with baked-in content)
  - web/index.html            (English x-default; symlinked target of /)
  - web/sitemap.xml           (12 URLs with hreflang cluster)
  - web/robots.txt            (allow all + sitemap)
  - web/404.html              (friendly 404, redirects to /)

Style + content is 100% preserved from the original; the only change is the
hash-based language switcher becomes real <a href> links.
"""
from __future__ import annotations

import os
import re
import shutil
import sys
import xml.etree.ElementTree as ET
from html.parser import HTMLParser
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "docs" / "index.html"
WEB = REPO / "web"

CANONICAL_BASE = "https://laffinty.github.io/FCEUX11"
OG_LOCALE = {
    "en": "en_US", "zh-CN": "zh_CN", "zh-TW": "zh_TW", "ja": "ja_JP",
    "ko": "ko_KR", "es": "es_ES", "fr": "fr_FR", "de": "de_DE",
    "vi": "vi_VN", "th": "th_TH", "hi": "hi_IN", "ar": "ar_SA",
}


def extract_block(src: str, start_pat: str, end_pat: str) -> str:
    """Return substring starting at start_pat and ending at end_pat (inclusive
    of both). Caller is responsible for stripping the constant declaration
    prefix and trailing semicolon when emitting valid JS."""
    s = src.index(start_pat)
    e = src.index(end_pat, s) + len(end_pat)
    return src[s:e]


def parse_i18n(src: str) -> tuple[list[dict], dict, dict]:
    """Parse the LANGS array, OG_LOCALE map, and I18N dict from the JS source.

    We re-execute the JS in a minimal namespace via Python's ast + manual
    parsing since the input is valid JavaScript object literals (no actual
    logic to evaluate).
    """
    # Extract LANGS: between "const LANGS=[" and the matching "];"
    langs_raw = extract_block(src, "const LANGS=[", "];\n")
    # Extract OG_LOCALE
    og_raw = extract_block(src, "const OG_LOCALE=", ";\n")
    # Extract I18N: between "const I18N={" and the matching "};"
    i18n_raw = extract_block(src, "const I18N={", "};\n")

    # Strip the JS declaration prefix and the trailing `;` from each block
    # so we can re-emit them as plain object literals. extract_block returns
    # inclusive of both end_pat and start_pat; the suffix `;\n` is part of
    # the value.
    def _strip_decl(s: str) -> str:
        # Drop the leading "const NAME=" prefix.
        eq = s.index("=")
        body = s[eq + 1:]
        # Drop the trailing `;` (and the newline that follows).
        body = body.rstrip()
        if body.endswith(";"):
            body = body[:-1]
        return body.strip()

    langs_lit = _strip_decl(langs_raw)
    og_lit = _strip_decl(og_raw)
    i18n_lit = _strip_decl(i18n_raw)

    # We delegate parsing to Node since the JS is real and we want the same
    # semantics (string escapes, unicode escapes, etc.).
    import json
    import subprocess
    import tempfile

    # Write a temporary Node script to a file to avoid the Windows
    # command-line length limit (~32 KiB for CreateProcess).
    node_script = f"""const fs = require('fs');
const path = require('path');
const srcPath = process.argv[2];
const src = fs.readFileSync(srcPath, 'utf8');
const LANGS = {langs_lit};
const OG_LOCALE = {og_lit};
const I18N = {i18n_lit};
process.stdout.write(JSON.stringify({{
  langs: LANGS,
  og: OG_LOCALE,
  i18n: I18N,
}}));
"""
    script_path = WEB / "_build_extract.js"
    WEB.mkdir(parents=True, exist_ok=True)
    script_path.write_text(node_script, encoding="utf-8")
    try:
        out = subprocess.check_output(
            ["node", str(script_path), str(SRC)],
            stderr=subprocess.STDOUT, timeout=30,
        )
        data = json.loads(out.decode("utf-8"))
        return data["langs"], data["og"], data["i18n"]
    except subprocess.CalledProcessError as e:
        # The exception object's .output is bytes; decode so the user can
        # see what Node complained about.
        try:
            err = e.output.decode("utf-8", errors="replace")
        except Exception:
            err = repr(e.output)
        sys.exit(f"node failed to parse i18n data:\n{err}")
    except FileNotFoundError:
        sys.exit("node executable not found on PATH; install Node.js to run this build script.")
    finally:
        try:
            script_path.unlink()
        except OSError:
            pass


def extract_css(src: str) -> str:
    """Pull the inner contents of the <style> block (lines 97-246)."""
    s = src.index("<style>") + len("<style>")
    e = src.index("</style>")
    return src[s:e].rstrip() + "\n"


def extract_fonts_link(src: str) -> str:
    """The <link href='fonts.googleapis...'> line — used in every per-lang page."""
    m = re.search(r'<link href="https://fonts\.googleapis\.com[^"]+" rel="stylesheet" />', src)
    if not m:
        sys.exit("could not find Google Fonts link in source")
    return m.group(0)


def make_lang_hreflang_cluster(lang_codes: list[str]) -> str:
    """Build the <link rel="alternate" hreflang="..."> block, one per language,
    plus an x-default pointing to the English root."""
    out = [
        '  <!-- hreflang cluster: one URL per language, x-default for unspecified -->',
        '  <link rel="alternate" hreflang="x-default" href="{base}/" />'.format(base=CANONICAL_BASE),
    ]
    for c in lang_codes:
        # English is the root; everything else lives under /<code>/
        if c == "en":
            url = f"{CANONICAL_BASE}/"
        else:
            url = f"{CANONICAL_BASE}/{c}/"
        out.append(f'  <link rel="alternate" hreflang="{c}" href="{url}" />')
    return "\n".join(out)


def make_og_locale_alternates(og_map: dict) -> str:
    """Build the <meta property="og:locale:alternate"> lines for every lang."""
    return "\n".join(
        f'  <meta property="og:locale:alternate" content="{og_map[c]}" />'
        for c in og_map
    )


def make_lang_switcher_html(lang: dict, all_langs: list[dict], current_code: str) -> str:
    """Render the language dropdown as <a href="..."> real links instead of
    JS-driven buttons. The dropdown UI is preserved; only the underlying
    mechanism changes (link navigation instead of data-lang + JS)."""
    items = []
    for l in all_langs:
        if l["code"] == "en":
            href = "/"
        else:
            href = f"/{l['code']}/"
        active = ' class="lang-item active"' if l["code"] == current_code else ' class="lang-item"'
        beta = ' <span class="beta">beta</span>' if l.get("beta") else ""
        items.append(
            f'        <a class="lang-item{ " active" if l["code"] == current_code else ""}" '
            f'href="{href}" hreflang="{l["code"]}" role="menuitem" data-lang="{l["code"]}">'
            f'<span>{l["native"]}</span>'
            f'<span class="code">{l["code"]}{beta}</span>'
            f'</a>'
        )
    return "\n".join(items)


def make_html_page(
    code: str, lang: dict, i18n: dict, all_langs: list[dict], og_map: dict,
    css_text: str, fonts_link: str, body_template: str,
) -> str:
    """Render a single per-language HTML page.

    Layout matches the original docs/index.html exactly:
    - <head> with localized title/description, OG, JSON-LD (single inLanguage)
    - <body> identical structure with data-i18n content baked in
    - hreflang cluster pointing to all 12 sibling URLs
    """
    d = i18n[code]
    is_rtl = bool(lang.get("rtl"))
    dir_attr = ' dir="rtl"' if is_rtl else ""
    if code == "en":
        canonical = f"{CANONICAL_BASE}/"
        page_path = "/"
    else:
        canonical = f"{CANONICAL_BASE}/{code}/"
        page_path = f"/{code}/"

    title = d.get("meta.title", "FCEUX11")
    desc = d.get("meta.desc", "")
    og_locale = og_map.get(code, "en_US")
    hreflang_block = make_lang_hreflang_cluster([l["code"] for l in all_langs])
    og_alternates = make_og_locale_alternates(og_map)
    switcher = make_lang_switcher_html(lang, all_langs, code)

    # Render the body with all data-i18n placeholders substituted.
    body = body_template
    for k, v in d.items():
        # Use a regex that matches data-i18n="<key>" and captures the
        # surrounding element so we only substitute the inner text.
        # Simpler: substitute by key in two known patterns (element inner
        # and element attribute). Since data-i18n always wraps the
        # translated text as inner HTML, we do a precise replacement of
        # `data-i18n="<key>">...<` (the opening attribute, the original
        # text, and the closing tag).
        pat = re.compile(
            r'(data-i18n="' + re.escape(k) + r'">)([^<]*)(<)',
        )
        body = pat.sub(lambda m: m.group(1) + v + m.group(3), body)

    head = f"""<!doctype html>
<html lang="{code}"{dir_attr}>
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
<meta name="color-scheme" content="dark" />
<meta name="theme-color" content="#080a09" />
<title>{title}</title>
<meta name="description" content="{desc}" />
<meta name="keywords" content="FCEUX11, FCEUX, NES emulator, FC emulator, Famicom emulator, Windows emulator, TAS editor, Qt6, C++20, game emulator, 8-bit, multi-language, 12 languages, Arabic, RTL, ROM hacking, debugger, Lua scripting" />
<meta name="author" content="Laffinty" />
<meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1" />
<meta name="generator" content="FCEUX11" />
<meta name="application-name" content="FCEUX11" />
<link rel="canonical" href="{canonical}" />
<link rel="icon" type="image/x-icon" href="https://raw.githubusercontent.com/Laffinty/FCEUX11/refs/heads/main/icons/app.ico" />

{hreflang_block}

<meta property="og:site_name" content="FCEUX11" />
<meta property="og:type" content="website" />
<meta property="og:title" content="{title}" />
<meta property="og:description" content="{desc}" />
<meta property="og:url" content="{canonical}" />
<meta property="og:image" content="https://raw.githubusercontent.com/Laffinty/FCEUX11/refs/heads/main/icons/app.ico" />
<meta property="og:locale" content="{og_locale}" />
{og_alternates}

<meta name="google-site-verification" content="2be70qD04OCrme3y8dDxUj_h1QwY4no9IWoFQpepryE" />
<meta name="yandex-verification" content="" />
<meta name="baidu-site-verification" content="" />
<meta name="msvalidate.01" content="" />

<meta name="twitter:card" content="summary" />
<meta name="twitter:title" content="{title}" />
<meta name="twitter:description" content="{desc}" />
<meta name="twitter:image" content="https://raw.githubusercontent.com/Laffinty/FCEUX11/refs/heads/main/icons/app.ico" />

<link rel="preconnect" href="https://fonts.googleapis.com" />
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<!-- Noto family covers CJK, Korean, Thai, Devanagari, Arabic, Vietnamese diacritics — no FOUT on any of the 12 locales -->
{fonts_link}

<script type="application/ld+json">
{{
  "@context":"https://schema.org",
  "@type":"SoftwareApplication",
  "name":"FCEUX11",
  "applicationCategory":"GameApplication",
  "operatingSystem":"Windows 11",
  "description":"{desc}",
  "softwareVersion":"1.15",
  "license":"https://www.gnu.org/licenses/old-licenses/gpl-2.0.txt",
  "url":"{canonical}",
  "downloadUrl":"https://github.com/Laffinty/FCEUX11/releases",
  "codeRepository":"https://github.com/Laffinty/FCEUX11",
  "image":"https://raw.githubusercontent.com/Laffinty/FCEUX11/refs/heads/main/icons/app.ico",
  "author":{{"@type":"Person","name":"Laffinty"}},
  "inLanguage":"{code}",
  "offers":{{"@type":"Offer","price":"0","priceCurrency":"USD"}}
}}
</script>
<script type="application/ld+json">
{{
  "@context":"https://schema.org",
  "@type":"WebSite",
  "name":"FCEUX11",
  "url":"{canonical}",
  "inLanguage":"{code}",
  "publisher":{{"@type":"Person","name":"Laffinty"}}
}}
</script>

<link rel="stylesheet" href="{('/' if code == 'en' else '/' + code + '/')}assets/styles.css" />
<script src="{('/' if code == 'en' else '/' + code + '/')}assets/app.js" defer></script>
</head>"""
    return head + "\n" + body


def make_body_template(src: str) -> str:
    """Pull the <body>...</body> block from the source, with:
    - the original inline <style> + <script> removed (those are extracted
      to web/assets/styles.css + web/assets/app.js)
    - the language dropdown contents replaced with a marker we can later
      populate per-language with real <a href> links
    - the body structure preserved 1:1
    """
    s = src.index("<body>")
    e = src.index("</body>") + len("</body>")
    body = src[s:e]

    # Strip the inline <script> block — its i18n logic is no longer needed
    # because content is baked in. The scroll/IntersectionObserver portion
    # goes into web/assets/app.js.
    body = re.sub(r"<script>.*?</script>", "<!-- app.js loaded externally -->",
                  body, count=1, flags=re.DOTALL)

    # Replace the empty <div class="lang-menu" id="langMenu">...</div>
    # with a placeholder we substitute per-language.
    body = re.sub(
        r'(<div class="lang-menu" id="langMenu"[^>]*>)(.*?)(</div>)',
        r'\1\n__LANG_SWITCHER__\n      \3',
        body, count=1, flags=re.DOTALL,
    )

    return body


def write_lang_pages(src: str, all_langs: list[dict], i18n: dict, og: dict,
                     css: str, fonts_link: str, body_template: str) -> None:
    for lang in all_langs:
        code = lang["code"]
        # Substitute the data-i18n content with this language's values.
        body = substitute_i18n_content(body_template, i18n[code])
        page = make_html_page(code, lang, i18n, all_langs, og, css, fonts_link,
                              body)
        if code == "en":
            out = WEB / "index.html"
        else:
            out = WEB / code / "index.html"
            out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(page, encoding="utf-8", newline="\n")
        print(f"  wrote {out.relative_to(REPO)}  ({len(page):,} bytes)")


def substitute_i18n_content(template: str, d: dict) -> str:
    """Replace the inner HTML of every element that has a `data-i18n="<key>"`
    attribute with the corresponding value from `d`. The inner HTML can
    contain nested tags (e.g. the `<p data-i18n="tagline">` element wraps
    a <span> and a <br>), so we walk the source with a tiny state machine
    that tracks element nesting and only rewrites content for elements
    whose open tag carries a data-i18n attribute.

    The data-i18n attribute itself is dropped from the output (it has no
    runtime meaning once content is baked in). The data-lang attribute on
    <a> tags inside the language switcher is preserved (the per-language
    switcher is inserted by `make_lang_switcher_html`, not by this fn).
    """
    # Collect the start offsets of every data-i18n="<key>" opening tag.
    # Walk the source character-by-character tracking tag boundaries; when
    # we encounter an opening tag with data-i18n, remember (start_of_inner,
    # key) and continue until the matching close tag, then record (end_pos,
    # key) so we know what range to replace.
    # We use a small hand-rolled scanner that knows about <tag attrs>text
    # </tag> and <tag/> (self-closing). The template is well-formed HTML
    # from a single source file, so we don't need a full parser.

    out_parts: list[str] = []
    pending: list[tuple[int, str]] = []  # stack of (start_of_inner, key)
    # Maps from start of inner content to the end of the matching close tag.
    replacements: dict[int, tuple[int, str]] = {}
    i = 0
    n = len(template)
    while i < n:
        ch = template[i]
        if ch != "<":
            i += 1
            continue
        # We are at a tag.
        end = template.find(">", i)
        if end < 0:
            break
        raw = template[i:end + 1]
        is_close = raw.startswith("</")
        is_self_closing = raw.endswith("/>")
        # Extract tag name and attributes.
        if is_close:
            tag_name = raw[2:raw.find(">")].strip().split()[0].rstrip(">")
        else:
            tag_name_match = re.match(r"<([\w-]+)", raw)
            tag_name = tag_name_match.group(1) if tag_name_match else ""
        if is_close:
            # Pop from stack; if top matches, register replacement.
            if pending and pending[-1][0] is not None:
                start, key = pending[-1]
                # Store (close_start, close_end, text) so the rebuild loop
                # can skip both the open tag's data-i18n attribute and the
                # entire original inner content (replaced by `text`).
                replacements[start] = (i, end + 1, d.get(key, ""))
                pending.pop()
            else:
                pending.pop()
        elif not is_self_closing:
            # HTML5 void elements: they have no end tag, so they shouldn't
            # be pushed onto the stack (otherwise a parent's </tag> would
            # pop the void element first and leave the parent un-closed).
            if tag_name in ("br", "hr", "img", "input", "meta", "link", "source",
                            "area", "base", "col", "embed", "param", "track",
                            "wbr"):
                i = end + 1
                continue
            # Look for data-i18n="<key>" in the open tag.
            m = re.search(r'data-i18n="([^"]+)"', raw)
            key = m.group(1) if m else None
            # Push; for elements with data-i18n, remember the start of inner
            # content (just after the closing ">" of the open tag) and the
            # key. We use a sentinel start of None to skip non-i18n
            # elements in the stack.
            inner_start = end + 1
            pending.append((inner_start if key else None, key or tag_name))
        # Always advance past the open tag we just processed; otherwise the
        # main loop would spin on the same `<` character forever.
        i = end + 1

    # Now we have a dict of inner_start -> (end_of_close, translated_text).
    # Walk the source again, copying chunks and inserting translations.
    # The replacements are non-overlapping because each top-level data-i18n
    # element is independent.
    if not replacements:
        return template
    # Sort by start position.
    sorted_repl = sorted(replacements.items())
    result: list[str] = []
    cursor = 0
    for inner_start, (close_start, close_end, text) in sorted_repl:
        # Append the text from cursor up to the open tag's closing ">".
        # The open tag's `>` is at inner_start - 1; we want to strip the
        # data-i18n="..." attribute from the open tag as we copy.
        open_end = inner_start - 1  # position of `>` in the open tag
        # Find the open tag start by walking back to the preceding `<`.
        open_start = template.rfind("<", cursor, open_end + 1)
        if open_start < 0:
            continue
        # Copy text up to open_start, then a slightly cleaned open tag
        # (with data-i18n removed) + the new inner content + the close
        # tag (extracted from the original close_start..close_end range).
        result.append(template[cursor:open_start])
        open_tag = template[open_start:open_end + 1]
        clean_open = re.sub(r'\s*data-i18n="[^"]*"', "", open_tag)
        result.append(clean_open)
        result.append(text)
        result.append(template[close_start:close_end])
        cursor = close_end
    result.append(template[cursor:])
    return "".join(result)


def substitute_switcher(html: str, lang_code: str, switcher_html: str) -> str:
    return html.replace("__LANG_SWITCHER__", switcher_html)


def main() -> int:
    if not SRC.exists():
        sys.exit(f"source not found: {SRC}")
    if WEB.exists():
        print(f"removing existing {WEB}")
        shutil.rmtree(WEB)
    WEB.mkdir(parents=True)
    (WEB / "assets").mkdir()

    src = SRC.read_text(encoding="utf-8")
    all_langs, og, i18n = parse_i18n(src)
    css = extract_css(src)
    fonts_link = extract_fonts_link(src)
    body_template = make_body_template(src)

    # Write shared assets
    (WEB / "assets" / "styles.css").write_text(css, encoding="utf-8", newline="\n")
    print(f"  wrote web/assets/styles.css  ({len(css):,} bytes)")

    app_js = """/* hotfix3 B-7-equivalent: scroll + IntersectionObserver, no i18n */
(function () {
  var nav = document.getElementById('nav');
  window.addEventListener('scroll', function () {
    nav.classList.toggle('scrolled', window.scrollY > 8);
  }, { passive: true });

  if (!('IntersectionObserver' in window)) {
    document.querySelectorAll('.reveal').forEach(function (e) { e.classList.add('in'); });
    return;
  }
  var io = new IntersectionObserver(function (entries) {
    entries.forEach(function (e) {
      if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); }
    });
  }, { threshold: 0.12, rootMargin: '0px 0px -30px 0px' });
  document.querySelectorAll('.reveal').forEach(function (e) { io.observe(e); });
})();
"""
    (WEB / "assets" / "app.js").write_text(app_js, encoding="utf-8", newline="\n")
    print(f"  wrote web/assets/app.js  ({len(app_js):,} bytes)")

    # Write per-language pages
    print("per-language pages:")
    write_lang_pages(src, all_langs, i18n, og, css, fonts_link, body_template)

    # Substitute the per-language switcher HTML into every page (post-hoc
    # because build_lang_pages is generic and we need the final code-specific
    # switcher markup with the active state).
    for lang in all_langs:
        code = lang["code"]
        if code == "en":
            p = WEB / "index.html"
        else:
            p = WEB / code / "index.html"
        text = p.read_text(encoding="utf-8")
        switcher = make_lang_switcher_html(lang, all_langs, code)
        text = substitute_switcher(text, code, switcher)
        p.write_text(text, encoding="utf-8", newline="\n")

    # Also localize the language-switcher trigger label
    for lang in all_langs:
        code = lang["code"]
        if code == "en":
            p = WEB / "index.html"
        else:
            p = WEB / code / "index.html"
        text = p.read_text(encoding="utf-8")
        # The trigger label is the current language's native script.
        text = text.replace(
            '<span id="langBtnLabel">English</span>',
            f'<span id="langBtnLabel">{lang["native"]}</span>',
        )
        p.write_text(text, encoding="utf-8", newline="\n")

    # sitemap.xml
    urls = []
    for l in all_langs:
        code = l["code"]
        loc = f"{CANONICAL_BASE}/" if code == "en" else f"{CANONICAL_BASE}/{code}/"
        urls.append((loc, code))
    sm = ['<?xml version="1.0" encoding="UTF-8"?>']
    sm.append('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" '
              'xmlns:xhtml="http://www.w3.org/1999/xhtml">')
    for loc, code in urls:
        sm.append('  <url>')
        sm.append(f'    <loc>{loc}</loc>')
        for alt in [x["code"] for x in all_langs]:
            alt_loc = f"{CANONICAL_BASE}/" if alt == "en" else f"{CANONICAL_BASE}/{alt}/"
            sm.append(f'    <xhtml:link rel="alternate" hreflang="{alt}" href="{alt_loc}" />')
        sm.append(f'    <xhtml:link rel="alternate" hreflang="x-default" href="{CANONICAL_BASE}/" />')
        sm.append('  </url>')
    sm.append('</urlset>')
    (WEB / "sitemap.xml").write_text("\n".join(sm) + "\n", encoding="utf-8", newline="\n")
    print("  wrote web/sitemap.xml")

    # robots.txt
    robots = f"""User-agent: *
Allow: /

Sitemap: {CANONICAL_BASE}/sitemap.xml
"""
    (WEB / "robots.txt").write_text(robots, encoding="utf-8", newline="\n")
    print("  wrote web/robots.txt")

    # 404.html — same dark theme, English content, link to root
    notfound = f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>404 — FCEUX11</title>
<meta name="robots" content="noindex" />
<link rel="canonical" href="{CANONICAL_BASE}/" />
<link rel="stylesheet" href="/assets/styles.css" />
<style>
  body{{display:grid;place-items:center;min-height:100vh;padding:24px;text-align:center}}
  .box{{max-width:520px}}
  h1{{font-family:'Space Grotesk',sans-serif;font-size:88px;letter-spacing:-.04em;margin-bottom:8px}}
  p{{color:#8a958c;margin-bottom:24px}}
  a{{color:#34d399;text-decoration:none}}
  a:hover{{text-decoration:underline}}
</style>
</head>
<body>
<div class="box">
  <h1>404</h1>
  <p>The page you requested doesn't exist.</p>
  <p><a href="/">← Back to FCEUX11</a></p>
</div>
</body>
</html>
"""
    (WEB / "404.html").write_text(notfound, encoding="utf-8", newline="\n")
    print("  wrote web/404.html")

    print()
    print(f"Done. {len(all_langs)} language pages + shared assets in web/.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
