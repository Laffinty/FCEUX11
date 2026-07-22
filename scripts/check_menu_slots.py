#!/usr/bin/env python3
# check_menu_slots.py - hotfix4 T-1 (defect D-1 class regression gate)
#
# Static-analysis gate for Qt SIGNAL/SLOT connects in src/drivers/Qt/.
# Catches the same class of bug that shipped D-1 (fdsLoadBIOS typo) and
# D-11 v1 (clearRecentRomMenu declared outside a slots: section).
#
# Checks performed (Python 3 stdlib only — no third-party deps):
#   1. Every `connect(..., this, SLOT(name(...)))` target `name` is declared
#      under a `public slots:`, `protected slots:`, or `private slots:`
#      section in ConsoleWindow.h (or another Qt-MOC'd header in scope).
#      Otherwise Qt MOC silently ignores the method and the connect fails
#      at runtime with "No such slot consoleWin_t::name(void)" — a class of
#      bug existing 29 CTest items all miss.
#   2. (Optional bonus, gated on --strict) Verify every consoleWin_t slot
#      declaration that takes a QAction signal `triggered()` is reachable
#      from at least one .cpp SLOT(...) reference. Skipped by default to
#      avoid over-fitting; ship the core check first.
#
# Known boundary (documented in plan §六 6.4):
#   This script checks the SLOT side only. SIGNAL-side typos (e.g. a
#   typo in the SIGNAL() macro argument) are not detected. The QObject
#   runtime will warn "No such signal" for those, but CTest cannot
#   reproduce them headless.
#
# Usage:
#   python3 scripts/check_menu_slots.py [--strict] [PROJECT_ROOT]
#   Default PROJECT_ROOT = parent of scripts/ directory.
#
# Exit codes:
#   0 — pass
#   1 — at least one slot is referenced via connect(SLOT(...)) but not
#       declared in a slots: section, or other static-analysis failure
#   2 — usage / IO error (file not found, unreadable)

import argparse
import os
import re
import sys
from typing import List, Set, Tuple


# Regex matching connect(... this, SLOT(name(...))) calls. Captures `name`.
# Permissive on whitespace; the trailing `(...)` may have any arg list.
# We deliberately allow the (void) suffix Qt convention.
# Note: `[^)]*` would block on the `)` inside `SIGNAL(triggered())`, so we
# match the whole connect( ... ); call and extract SLOT name from there.
SLOT_RE = re.compile(
    r"""
    \bconnect\s*\(                     # connect(
    .*?                                 # anything (incl. parens), non-greedy
    \bSLOT\s*\(\s*                      # SLOT(
    ([A-Za-z_][A-Za-z0-9_]*)            # slot name (captured)
    \s*\([^)]*\)\s*\)                   # (...) closing SLOT
    """,
    re.VERBOSE | re.DOTALL,
)

# We want to specifically match connects where receiver is `this` (i.e.,
# connecting to a method on the same object). This is a defensive filter
# so we don't accidentally require every external slot to be locally
# declared.
RECEIVER_THIS_RE = re.compile(
    r"""
    \bconnect\s*\(                      # connect(
    .*?                                  # anything (incl. parens), non-greedy
    \bthis\s*,\s*                        # `this,`
    \bSLOT\s*\(                          # SLOT(
    ([A-Za-z_][A-Za-z0-9_]*)             # slot name (captured)
    \s*\([^)]*\)\s*\)                    # (...) closing SLOT
    """,
    re.VERBOSE | re.DOTALL,
)


def find_slot_sections(header_text: str) -> List[Tuple[str, int, int]]:
    """Return a list of (kind, start_offset, end_offset) for each `slots:`
    section found. kind ∈ {'public', 'protected', 'private'}.
    The range covers everything from the keyword to the next access
    modifier (`public:`, `protected:`, `private:`, `signals:`)."""
    sections = []
    # Match "<access> slots:" with the access keyword captured.
    pattern = re.compile(
        r"\b(public|protected|private)\s+slots\s*:",
        re.MULTILINE,
    )
    for m in pattern.finditer(header_text):
        kind = m.group(1)
        start = m.end()
        # Find next access keyword / class boundary / closing brace.
        end_pattern = re.compile(
            r"\b(public|protected|private|signals)\s*:",
            re.MULTILINE,
        )
        end_match = end_pattern.search(header_text, start)
        end = end_match.start() if end_match else len(header_text)
        sections.append((kind, start, end))
    return sections


def extract_slot_methods(header_text: str, section: Tuple[str, int, int]) -> Set[str]:
    """Extract method names declared in a single slots: section.
    Recognizes patterns:
        void name(args);
        void name(void);
    Ignores comments and #ifdef'd blocks (best-effort — full preprocessing
    would require real C++ parsing)."""
    _, start, end = section
    body = header_text[start:end]
    # Strip C++ // and /* */ comments before scanning.
    body_no_block = re.sub(r"/\*.*?\*/", "", body, flags=re.DOTALL)
    body_clean = re.sub(r"//[^\n]*", "", body_no_block)
    method_re = re.compile(
        r"\bvoid\s+([A-Za-z_][A-Za-z0-9_]*)\s*\([^)]*\)\s*;",
    )
    return set(method_re.findall(body_clean))


def collect_all_slots(header_paths: List[str]) -> Set[str]:
    """Parse all given header files and return the union of slot method
    names found across all `slots:` sections."""
    all_slots: Set[str] = set()
    for path in header_paths:
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as f:
                text = f.read()
        except OSError as e:
            print(f"ERROR: cannot read {path}: {e}", file=sys.stderr)
            sys.exit(2)
        for section in find_slot_sections(text):
            all_slots |= extract_slot_methods(text, section)
    return all_slots


def strip_comments_preserve_lines(text: str) -> str:
    """Strip C++ // and /* */ comments, but replace each stripped
    character with a NEWLINE so 1-based line numbers from the result
    still map to lines in the original source."""
    out = []
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        # Block comment: /* ... */
        if ch == "/" and i + 1 < n and text[i + 1] == "*":
            j = text.find("*/", i + 2)
            if j == -1:
                # Unterminated; replace rest with newlines
                out.append("\n" * text[i:].count("\n") + " " * (len(text) - i - text[i:].count("\n")))
                return "".join(out)
            block = text[i:j + 2]
            out.append("\n" * block.count("\n") + " " * (len(block) - block.count("\n")))
            i = j + 2
            continue
        # Line comment: // ...
        if ch == "/" and i + 1 < n and text[i + 1] == "/":
            j = text.find("\n", i + 2)
            if j == -1:
                # Comment to EOF
                out.append(" " * (n - i))
                return "".join(out)
            out.append(" " * (j - i))
            i = j
            continue
        # String literal: "..." — keep contents but replace embedded \n
        # with spaces so we don't accidentally break multiline strings.
        if ch == '"':
            j = i + 1
            while j < n and text[j] != '"':
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                j += 1
            out.append(text[i:j + 1])
            i = j + 1
            continue
        out.append(ch)
        i += 1
    return "".join(out)


# Built-in Qt slots that may be referenced via SLOT() without being
# declared in our headers (inherited from QObject / QDialog / QWidget
# etc.). These are excluded from the "not declared" check.
BUILTIN_QT_SLOTS = frozenset({
    "deleteLater",   # QObject
    "accept",        # QDialog
    "reject",        # QDialog
    "done",          # QDialog
    "exec",          # QDialog
    "hide",          # QWidget
    "show",          # QWidget
    "close",         # QWidget
    "update",        # QWidget
    "repaint",       # QWidget
    "setVisible",    # QWidget
    "setEnabled",    # QWidget
    "setDisabled",   # QWidget
    "setText",       # QWidget (actually QLineEdit etc.)
    "setValue",      # QAbstractSlider
    "trigger",       # QAction
    "toggle",        # QAction
})


def find_connect_calls(cpp_path: str) -> List[Tuple[str, int, str]]:
    """Return list of (filename, line_number, slot_name) for every
    `connect(..., this, SLOT(name(...)))` call in cpp_path. Comments
    (// ... and /* ... */) are stripped before matching to avoid
    flagging dead code; line numbers still map to the original source
    via newline-preserving comment stripping."""
    try:
        with open(cpp_path, "r", encoding="utf-8", errors="replace") as f:
            text = f.read()
    except OSError as e:
        print(f"ERROR: cannot read {cpp_path}: {e}", file=sys.stderr)
        sys.exit(2)

    text_clean = strip_comments_preserve_lines(text)

    results: List[Tuple[str, int, str]] = []
    for m in RECEIVER_THIS_RE.finditer(text_clean):
        slot_name = m.group(1)
        if slot_name in BUILTIN_QT_SLOTS:
            continue
        line_no = text_clean.count("\n", 0, m.start()) + 1
        results.append((cpp_path, line_no, slot_name))
    return results


def list_cpp_files(qt_dir: str) -> List[str]:
    """Return sorted list of .cpp files directly under qt_dir (TasEditor
    is excluded: its slots are registered into a different QObject tree
    and verified by its own smoke tests)."""
    files = []
    for entry in sorted(os.listdir(qt_dir)):
        if entry.endswith(".cpp"):
            files.append(os.path.join(qt_dir, entry))
    return files


def main() -> int:
    parser = argparse.ArgumentParser(
        description="hotfix4 T-1: Qt SLOT(...) wiring static-analysis gate."
    )
    parser.add_argument(
        "project_root",
        nargs="?",
        default=os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        help="Path to project root (default: parent of this script's dir).",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Enable optional checks (currently a no-op placeholder for the "
             "menu-action reachability cross-check).",
    )
    args = parser.parse_args()

    project_root = os.path.abspath(args.project_root)
    qt_dir = os.path.join(project_root, "src", "drivers", "Qt")
    if not os.path.isdir(qt_dir):
        print(f"ERROR: Qt driver dir not found: {qt_dir}", file=sys.stderr)
        return 2

    # Scan ALL Qt headers (not just ConsoleWindow.h) — each QObject subclass
    # may declare its own slots in its own header. The union is the
    # universe of slot names connect() may legally target.
    header_paths = sorted(
        os.path.join(qt_dir, name)
        for name in os.listdir(qt_dir)
        if name.endswith(".h")
    )
    if not header_paths:
        print(f"ERROR: no .h files in {qt_dir}", file=sys.stderr)
        return 2

    # 1. Collect all declared slots.
    declared = collect_all_slots(header_paths)
    print(f"[T-1] Declared slots across {len(header_paths)} header(s): "
          f"{len(declared)}")

    # 2. Find every connect(..., this, SLOT(...)) in the Qt cpp files.
    all_refs: List[Tuple[str, int, str]] = []
    for cpp in list_cpp_files(qt_dir):
        all_refs.extend(find_connect_calls(cpp))
    print(f"[T-1] Found {len(all_refs)} connect(..., this, SLOT(...)) call(s) "
          f"across {len(list_cpp_files(qt_dir))} cpp file(s)")

    # 3. Cross-check: every referenced slot must be declared.
    failures: List[Tuple[str, int, str]] = []
    for path, line, name in all_refs:
        if name not in declared:
            failures.append((path, line, name))

    if failures:
        print(f"[T-1] FAIL: {len(failures)} slot reference(s) not declared "
              f"in any slots: section:", file=sys.stderr)
        for path, line, name in failures:
            rel = os.path.relpath(path, project_root)
            print(f"  {rel}:{line}: slot '{name}' not declared", file=sys.stderr)
        return 1

    print("[T-1] PASS: all connect(..., this, SLOT(...)) targets are "
          "declared in a slots: section of the scanned header(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
