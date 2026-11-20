#!/usr/bin/env python3
"""Extract release notes from the GitHub Pages source.

The release notes are the "what's new" block that the site already publishes:
English from web/index.html and Simplified Chinese from web/zh-CN/index.html.
Nothing is written by hand here -- if a page does not carry the block for the
version being released, this script fails instead of producing empty notes.

Usage:
    python3 scripts/extract_release_notes.py --version 2.0.6 --out notes.md
    python3 scripts/extract_release_notes.py --version 2.0.6 --check
    python3 scripts/extract_release_notes.py            # version from version.h

--check performs the same parsing and prints a summary, but writes nothing and
is what the release workflow runs in its preflight gate.

Exit codes: 0 ok; 1 parse/validation failure; 2 usage error.
"""

import argparse
import html
import io
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# The 12 published locales. Every one must carry the same version heading; the
# two used for the notes themselves are checked in more detail below.
LOCALES = [
    "web/index.html",
    "web/ar/index.html",
    "web/de/index.html",
    "web/es/index.html",
    "web/fr/index.html",
    "web/hi/index.html",
    "web/ja/index.html",
    "web/ko/index.html",
    "web/th/index.html",
    "web/vi/index.html",
    "web/zh-CN/index.html",
    "web/zh-TW/index.html",
]

EN_PAGE = "web/index.html"
ZH_PAGE = "web/zh-CN/index.html"

TAG_RE = re.compile(r"<[^>]+>")
WS_RE = re.compile(r"[ \t\r\n]+")


class NotesError(Exception):
    pass


def read_text(path):
    with io.open(path, "r", encoding="utf-8", newline="") as f:
        return f.read()


def strip_tags(fragment):
    """HTML fragment -> single-line plain text."""
    text = TAG_RE.sub("", fragment)
    text = html.unescape(text)
    return WS_RE.sub(" ", text).strip()


def version_from_header(path=None):
    path = path or os.path.join(REPO_ROOT, "src", "version.h")
    src = read_text(path)
    values = {}
    for key in ("MAJOR", "MINOR", "PATCH"):
        m = re.search(r"FCEU_VERSION_%s\s*=\s*(\d+)\s*;" % key, src)
        if not m:
            raise NotesError("cannot read FCEU_VERSION_%s from %s" % (key, path))
        values[key] = m.group(1)
    return "%s.%s.%s" % (values["MAJOR"], values["MINOR"], values["PATCH"])


def whatsnew_section(page_html):
    """Return (kicker, heading, sub, [(title, body), ...]) for the page's block."""
    start = page_html.find('class="whatsnew"')
    if start < 0:
        raise NotesError("no whatsnew section")

    # The block ends at the changelog CTA, which every page carries.
    cta = page_html.find('class="wn-cta', start)
    block = page_html[start:cta] if cta > start else page_html[start:]

    kicker = re.search(r'<div class="kicker">(.*?)</div>', block, re.S)
    heading = re.search(r"<h2>(.*?)</h2>", block, re.S)
    sub = re.search(r'<div class="sub">(.*?)</div>', block, re.S)

    if not heading:
        raise NotesError("whatsnew section has no <h2> version heading")
    if not sub:
        raise NotesError("whatsnew section has no <div class=\"sub\"> summary")

    # Split on the item marker rather than trying to match the closing tags:
    # each item ends with two </div> and is followed either by the next item or
    # by the CTA, so a balanced-tag regex is fragile to re-indentation.
    items = []
    chunks = block.split('<div class="wn-item">')[1:]
    for chunk in chunks:
        h3 = re.search(r"<h3>(.*?)</h3>", chunk, re.S)
        p = re.search(r"<p>(.*?)</p>", chunk, re.S)
        if not h3 or not p:
            raise NotesError("wn-item without <h3>/<p> pair")
        items.append((strip_tags(h3.group(1)), strip_tags(p.group(1))))

    if not items:
        raise NotesError("no wn-item entries found")

    return (
        strip_tags(kicker.group(1)) if kicker else "",
        strip_tags(heading.group(1)),
        strip_tags(sub.group(1)),
        items,
    )


def check_all_locales(version):
    """Every locale page must carry the version heading and a non-empty block."""
    problems = []
    for rel in LOCALES:
        path = os.path.join(REPO_ROOT, rel)
        if not os.path.exists(path):
            problems.append("%s: missing file" % rel)
            continue
        try:
            _, heading, _, items = whatsnew_section(read_text(path))
        except NotesError as exc:
            problems.append("%s: %s" % (rel, exc))
            continue
        if heading.strip() != "v" + version:
            problems.append(
                '%s: heading is "%s", expected "v%s"' % (rel, heading, version)
            )
        if not items:
            problems.append("%s: whatsnew block is empty" % rel)
    return problems


def check_changelog(version):
    path = os.path.join(REPO_ROOT, "CHANGELOG.md")
    if not os.path.exists(path):
        return ["CHANGELOG.md: missing file"]
    src = read_text(path)
    if not re.search(r"^##\s*\[%s\]" % re.escape(version), src, re.M):
        return ["CHANGELOG.md: no '## [%s] - ...' section" % version]
    return []


SENTENCE_END = "。．.!?！？"


def as_sentence(text):
    """Headline -> sentence, the way the shipped releases read.

    The v2.0.5 release body is a flat list of full sentences; the page's <h3>
    headlines are that sentence minus the terminator, so add one rather than
    paraphrasing anything.
    """
    text = text.strip()
    if text and text[-1] not in SENTENCE_END:
        text += "。" if any("\u4e00" <= c <= "\u9fff" for c in text) else "."
    return text


def render(version, items_en, items_zh):
    """Release body: one English line then one Chinese line per item.

    This mirrors the shipped releases (v2.0.5 and earlier): a flat list, no
    headings, one sentence per language per item. The SourceForge download
    badge that follows the list on the release page is produced elsewhere and
    is deliberately not part of this text.
    """
    out = []
    for (title_en, _), (title_zh, _) in zip(items_en, items_zh):
        out.append("- " + as_sentence(title_en))
        out.append("- " + as_sentence(title_zh))
    out.append("")
    return "\n".join(out)


def render_detailed(version, kicker_en, sub_en, items_en, kicker_zh, sub_zh, items_zh):
    """Long form: both blocks in full, for reading outside GitHub Releases."""
    out = ["## FCEUX11 v%s" % version, ""]
    out.append("### 简体中文 · %s" % (kicker_zh or "本次更新"))
    out.append("")
    out.append(sub_zh)
    out.append("")
    for title, body in items_zh:
        out.append("- **%s**" % title)
        out.append("")
        out.append("  %s" % body)
        out.append("")
    out.append("---")
    out.append("")
    out.append("### English · %s" % (kicker_en or "What's new"))
    out.append("")
    out.append(sub_en)
    out.append("")
    for title, body in items_en:
        out.append("- **%s**" % title)
        out.append("")
        out.append("  %s" % body)
        out.append("")
    out.append("Full changelog: [CHANGELOG.md](CHANGELOG.md)")
    out.append("")
    return "\n".join(out)


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--version", help="version to release (default: src/version.h)")
    ap.add_argument("--out", help="write the notes here (default: stdout)")
    ap.add_argument(
        "--check",
        action="store_true",
        help="validate only: locales, changelog and both notes sources",
    )
    ap.add_argument(
        "--format",
        choices=["release", "detailed"],
        default="release",
        help="release: flat EN/ZH sentence list + download badge (matches the "
             "shipped GitHub Releases); detailed: both blocks in full",
    )
    args = ap.parse_args(argv)

    try:
        version = args.version or version_from_header()
    except NotesError as exc:
        print("ERROR: %s" % exc, file=sys.stderr)
        return 1

    problems = check_all_locales(version)
    problems += check_changelog(version)

    try:
        kicker_en, heading_en, sub_en, items_en = whatsnew_section(
            read_text(os.path.join(REPO_ROOT, EN_PAGE))
        )
        kicker_zh, heading_zh, sub_zh, items_zh = whatsnew_section(
            read_text(os.path.join(REPO_ROOT, ZH_PAGE))
        )
        if heading_en != "v" + version:
            problems.append('%s: heading is "%s"' % (EN_PAGE, heading_en))
        if heading_zh != "v" + version:
            problems.append('%s: heading is "%s"' % (ZH_PAGE, heading_zh))
        if len(items_en) != len(items_zh):
            problems.append(
                "entry count differs: %s has %d, %s has %d"
                % (EN_PAGE, len(items_en), ZH_PAGE, len(items_zh))
            )
    except NotesError as exc:
        print("ERROR: %s" % exc, file=sys.stderr)
        return 1

    if problems:
        print("release notes are not ready for v%s:" % version, file=sys.stderr)
        for p in problems:
            print("  - %s" % p, file=sys.stderr)
        return 1

    if args.format == "detailed":
        notes = render_detailed(version, kicker_en, sub_en, items_en,
                                kicker_zh, sub_zh, items_zh)
    else:
        notes = render(version, items_en, items_zh)

    if args.check:
        print("OK  v%s: %d entries, %d locales, CHANGELOG section present"
              % (version, len(items_en), len(LOCALES)))
        return 0

    if args.out:
        with io.open(args.out, "w", encoding="utf-8", newline="\n") as f:
            f.write(notes)
        print("wrote %s (%d bytes)" % (args.out, len(notes.encode("utf-8"))))
    else:
        sys.stdout.write(notes)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
