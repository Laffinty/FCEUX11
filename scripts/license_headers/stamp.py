#!/usr/bin/env python3
"""stamp.py -- v2.0.8 batch1 L2: prepend GPL banners from classify manifest.

Byte-level mechanics (r2, pitfall 11):
  - read file as bytes; existing content bytes are NEVER translated or rewritten
  - UTF-8 BOM preserved (header inserted after it)
  - header EOL = the file's first-line EOL (CRLF or LF)
  - banner shape = the repo's existing "FCE Ultra" banner (no SPDX anywhere)
  - idempotent: files already containing GPL text in their first 2 KB are skipped

Modes:
  replace       leading comment banner is a recognized title+copyright banner
                (no other prose) -> replaced by the full standard banner
  prepend_gpl   leading comment banner exists but is unrecognized/prose-only ->
                GPL-text-only block prepended above it (old banner untouched)
  prepend_full  no leading comment banner -> full standard banner prepended

Default is a dry run; pass --write to modify files.
"""
import json
import os
import sys

GPL_MARK = b"GNU General Public License"
BOM = b"\xef\xbb\xbf"

TITLE = "/* FCE Ultra - NES/Famicom Emulator"
GPL_BODY = [
    " *",
    " * This program is free software; you can redistribute it and/or modify",
    " * it under the terms of the GNU General Public License as published by",
    " * the Free Software Foundation; either version 2 of the License, or",
    " * (at your option) any later version.",
    " *",
    " * This program is distributed in the hope that it will be useful,",
    " * but WITHOUT ANY WARRANTY; without even the implied warranty of",
    " * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the",
    " * GNU General Public License for more details.",
    " *",
    " * You should have received a copy of the GNU General Public License",
    " * along with this program; if not, write to the Free Software",
    " * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA",
    " */",
]

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(HERE, "..", ".."))


def build_full(copyright_lines, eol):
    lines = [TITLE]
    if copyright_lines:
        lines.append(" *")
        lines.append(" * Copyright notice for this file:")
        lines += [" *  " + c for c in copyright_lines]
    lines += GPL_BODY
    return eol.join(lines) + eol


def build_gpl_only(eol):
    return eol.join(["/*"] + GPL_BODY) + eol


def banner_mode(body_text):
    """Classify the leading comment banner of body_text (str)."""
    stripped = body_text.lstrip("\ufeff")
    if not stripped.startswith("/*"):
        return "prepend_full", None
    end = stripped.find("*/")
    if end == -1:
        return "prepend_full", None
    banner = stripped[: end + 2]
    prose = []
    for raw in banner.splitlines():
        s = raw.strip()
        s = s.lstrip("/*").lstrip("*").strip()
        if not s:
            continue
        if s.startswith("FCE Ultra - NES/Famicom Emulator"):
            continue
        if s.startswith("Copyright notice for this file"):
            continue
        if s.startswith("Copyright (C)"):
            continue
        prose.append(s)
    if prose:
        return "prepend_gpl", banner
    if GPL_MARK.decode() in banner:
        return "skip", banner
    if any(s.startswith("Copyright (C)") for s in banner.splitlines()) or \
       "FCE Ultra" in banner:
        return "replace", banner
    return "prepend_gpl", banner


def detect_eol(body_bytes):
    idx = body_bytes.find(b"\n")
    if idx > 0 and body_bytes[idx - 1:idx] == b"\r":
        return b"\r\n"
    return b"\n"


def main():
    write = "--write" in sys.argv
    with open(os.path.join(HERE, "manifest.json"), "r", encoding="utf-8") as fh:
        manifest = json.load(fh)["files"]

    stats = {"stamped": 0, "skip": 0, "replace": 0, "prepend_gpl": 0, "prepend_full": 0}
    details = []

    for rel in sorted(manifest):
        v = manifest[rel]
        if v["bucket"] == "C":
            continue
        path = os.path.join(REPO, rel)
        with open(path, "rb") as fh:
            data = fh.read()

        bom = BOM if data.startswith(BOM) else b""
        body = data[len(bom):]
        if GPL_MARK in body[:2048]:
            stats["skip"] += 1
            continue

        eol = detect_eol(body)
        mode, old_banner = banner_mode(body.decode("utf-8", errors="replace"))

        if mode == "skip":
            stats["skip"] += 1
            continue
        if mode == "replace":
            header = build_full(v["copyright_lines"], eol.decode())
            # strip the old banner: body after the first '*/'
            end = body.find(b"*/")
            rest = body[end + 2:]
            if rest.startswith(b"\r\n"):
                pass  # keep as-is; header already ends with one eol
            new_data = bom + header.encode("ascii") + rest
            stats["replace"] += 1
        elif mode == "prepend_gpl":
            header = build_gpl_only(eol.decode())
            new_data = bom + header.encode("ascii") + body
            stats["prepend_gpl"] += 1
        else:
            header = build_full(v["copyright_lines"], eol.decode())
            new_data = bom + header.encode("ascii") + body
            stats["prepend_full"] += 1

        details.append((rel, mode, len(new_data) - len(data)))
        stats["stamped"] += 1

        if write:
            with open(path, "wb") as fh:
                fh.write(new_data)
            with open(path, "rb") as fh:
                assert fh.read() == new_data, "readback mismatch: " + rel
            # content fidelity: original body bytes must still be present
            assert body in new_data or mode == "replace", "body lost: " + rel

    print("== stamp %s ==" % ("WRITE" if write else "DRY-RUN"))
    print("stamped total :", stats["stamped"])
    print("  replace     :", stats["replace"])
    print("  prepend_gpl :", stats["prepend_gpl"])
    print("  prepend_full:", stats["prepend_full"])
    print("skipped (already GPL):", stats["skip"])
    if details:
        print()
        print("-- mode per file --")
        for rel, mode, delta in details:
            print(f"  [{mode:<12}] +{delta:>5}B  {rel}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
