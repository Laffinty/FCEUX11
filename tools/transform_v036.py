#!/usr/bin/env python3
"""v0.3.6 transform: FCEU_gmalloc → FceuMallocPtr (RAII).

For each mapper .cpp file:
  1. Identify the static pointer declarations (e.g., `static uint8 *WRAM = NULL;`).
  2. After the first static declaration, add a `static FceuMallocPtr <NAME>_owner;` line.
  3. Replace `NAME = (uint8*)FCEU_gmalloc(N);` with
     `<NAME>_owner = FCEU_gmalloc_unique(N); NAME = <NAME>_owner.get();`.
  4. Replace `if (NAME) FCEU_gfree(NAME);` inside the *Close function with
     `<NAME>_owner.reset();`. The trailing `NAME = NULL;` (if present) is
     rewritten to `NAME = nullptr;` for C++11 consistency.

This script is idempotent: re-running it on an already-migrated file is a no-op
because the unique_ptr call site is detected and skipped.
"""

from __future__ import annotations
import re
import subprocess
import sys
from pathlib import Path

# Resolve the repo root from this script's location: tools/ is one level below
# the project root, so go up one parent. Avoids hardcoding the user's path.
REPO = Path(__file__).resolve().parent.parent
BOARDS_DIR = REPO / "src" / "boards"
EXTRA_FILES = [
    REPO / "src" / "fceu.cpp",
    REPO / "src" / "fds.cpp",
    REPO / "src" / "ines.cpp",
    REPO / "src" / "nsf.cpp",
    REPO / "src" / "state.cpp",
    REPO / "src" / "drivers" / "common" / "vidblit.cpp",
]


def _find_gmalloc_calls(text: str) -> list[tuple[str, str, str, int]]:
    """Find FCEU_gmalloc / FCEU_dmalloc call sites by scanning the file line by line.

    Returns a list of (NAME, size_expr, cast_type, line_index) tuples. The size
    expression is the text between the outer FCEU_gmalloc( and the matching
    close-paren, handling nested parens correctly (e.g. `sizeof(cfi_data) * 2`).
    The cast_type is the text inside the `(TYPE*)` cast (e.g. "uint8", "int",
    "uint32", "void") or "" if no cast is present.

    Both FCEU_gmalloc and FCEU_dmalloc are recognized — they are merged into
    the same backing allocator in v0.3.6 (see utils/memory.h).
    """
    results: list[tuple[str, str, str, int]] = []
    lines = text.splitlines(keepends=True)
    for i, line in enumerate(lines):
        m = re.match(
            r"^(?P<indent>[ \t]*)(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*"
            r"(?:\((?P<cast>[^()]*\*)\)\s*)?FCEU_(?:gmalloc|dmalloc)\s*\(",
            line,
        )
        if not m:
            continue
        paren_start = m.end() - 1  # the '(' character
        depth = 0
        end = paren_start
        for j in range(paren_start, len(line)):
            ch = line[j]
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth == 0:
                    end = j
                    break
        if depth != 0:
            continue
        size_expr = line[paren_start + 1 : end].strip()
        if not re.match(r"\s*;\s*$", line[end + 1 :]):
            continue
        cast_type = (m.group("cast") or "").strip().rstrip("*").strip()
        results.append((m.group("name"), size_expr, cast_type, i))
    return results


def _find_static_uint8_decls(text: str) -> list[tuple[str, int, str]]:
    """Find file-scope `TYPE* NAME;` declarations (any pointer type).

    Matches: `static TYPE* NAME;`, `extern TYPE* NAME;`, and bare
    `TYPE* NAME;` (mmc3.cpp-style). Both forms are treated as file-scope
    anchors for inserting the RAII owner. Trailing line comments after `;`
    are allowed (e.g. `static int *X; // some note`).
    Returns (NAME, line_index, indent).
    """
    results: list[tuple[str, int, str]] = []
    for i, line in enumerate(text.splitlines()):
        # Type can be a built-in or a user-defined identifier (e.g. nes_ntsc_t).
        m = re.match(
            r"^(?P<indent>[ \t]*)(?:(?:static|extern)\s+)?"
            r"(?:[A-Za-z_][A-Za-z0-9_]*\s+)*[A-Za-z_][A-Za-z0-9_]*\s*\*\s*"
            r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?:=\s*[^;]+)?\s*;",
            line,
        )
        if m:
            results.append((m.group("name"), i, m.group("indent")))
    return results


def _find_gfree_calls(text: str) -> list[tuple[str, int]]:
    """Find `if (NAME)\n  FCEU_gfree(NAME);` two-line patterns.

    Returns a list of (NAME, line_index) tuples. line_index is the line of
    the `if (NAME)` statement.
    """
    results: list[tuple[str, int]] = []
    lines = text.splitlines()
    for i in range(len(lines) - 1):
        m_if = re.match(r"^[ \t]*if\s*\(\s*(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\)\s*$", lines[i])
        if not m_if:
            continue
        name = m_if.group("name")
        m_free = re.match(
            r"^[ \t]*FCEU_gfree\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*;\s*$",
            lines[i + 1],
        )
        if not m_free:
            continue
        if m_free.group(1) != name:
            continue
        results.append((name, i))
    return results


def _find_fceu_free_single_lines(text: str) -> list[tuple[str, int]]:
    """Find single-line `FCEU_free(NAME);` (no `if (NAME)` guard).

    These are the cases where the script's two-line matcher misses. Returned
    tuples are (NAME, line_index).
    """
    results: list[tuple[str, int]] = []
    for i, line in enumerate(text.splitlines()):
        m = re.match(
            r"^[ \t]*FCEU_free\s*\(\s*(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\)\s*;\s*$",
            line,
        )
        if m:
            results.append((m.group("name"), i))
    return results


NULL_ASSIGN_RE = re.compile(
    r"^(?P<indent>[ \t]*)(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*=\s*(?:NULL|0)\s*;\s*$"
)


def transform_file(path: Path) -> bool:
    """Transform a single .cpp file. Returns True if any change was made."""
    text = path.read_text(encoding="utf-8", errors="replace")
    original = text

    # Per-call-site migration: we always scan, but only act on call sites
    # that haven't been converted yet. This makes the script safe to re-run
    # on partially-transformed files.

    gmalloc_calls = _find_gmalloc_calls(text)
    if not gmalloc_calls:
        return False

    unique_names: list[str] = []
    for name, _, _, _ in gmalloc_calls:
        if name not in unique_names:
            unique_names.append(name)

    # Map NAME → cast type. If multiple call sites have different cast types,
    # prefer the first one (usually consistent within a file).
    cast_by_name: dict[str, str] = {}
    for name, _, cast_type, _ in gmalloc_calls:
        if name not in cast_by_name and cast_type:
            cast_by_name[name] = cast_type

    static_decls = _find_static_uint8_decls(text)
    decl_by_name: dict[str, tuple[int, str]] = {n: (i, ind) for n, i, ind in static_decls}

    lines = text.splitlines(keepends=True)

    # Step 1: insert `static FceuMallocPtr <NAME>_owner;` after the static
    # declaration of NAME. If NAME has no static declaration, fall back to
    # inserting before the first FCEU_gmalloc call for NAME.
    insertions: list[tuple[int, str]] = []
    for name in unique_names:
        owner_line = f"static FceuMallocPtr {name}_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction"
        if any(name + "_owner" in l for l in lines):
            continue
        if name in decl_by_name:
            decl_line_idx, decl_indent = decl_by_name[name]
            indent = decl_indent
            insertions.append((decl_line_idx + 1, indent + owner_line + "\n"))
        else:
            for name2, _, _, line_idx in gmalloc_calls:
                if name2 != name:
                    continue
                first_line = lines[line_idx]
                m_indent = re.match(r"^(?P<indent>[ \t]*)", first_line)
                indent = m_indent.group("indent") if m_indent else "\t"
                insertions.append(
                    (
                        line_idx,
                        f"{indent}// v0.3.6: FCEU_gmalloc RAII owner for {name} "
                        f"(no file-static decl found)\n"
                        f"{indent}{owner_line}\n",
                    )
                )
                break

    # Apply insertions in reverse order so indices remain stable.
    insertions.sort(key=lambda t: t[0], reverse=True)
    for idx, txt in insertions:
        lines.insert(idx, txt)

    # Re-scan the updated lines.
    new_text = "".join(lines)
    gmalloc_calls = _find_gmalloc_calls(new_text)
    gfree_calls = _find_gfree_calls(new_text)
    fceu_free_singles = _find_fceu_free_single_lines(new_text)
    lines = new_text.splitlines(keepends=True)

    # Step 2: replace FCEU_gmalloc call sites. Mark them for replacement.
    # Skip call sites that have already been converted (e.g., on re-run).
    actions: dict[int, str] = {}
    for name, size_expr, cast_type, line_idx in gmalloc_calls:
        first_line = lines[line_idx]
        if f"{name}_owner = FCEU_gmalloc_unique" in first_line:
            # Already converted (e.g., the unique_ptr line is on this line).
            continue
        # Check the following line for the assignment (since the original
        # pattern places the call and assignment on separate lines).
        if line_idx + 1 < len(lines) and f"{name}_owner = FCEU_gmalloc_unique" in lines[line_idx + 1]:
            continue
        m_indent = re.match(r"^(?P<indent>[ \t]*)", first_line)
        indent = m_indent.group("indent") if m_indent else "\t"
        # Emit a cast to the original pointer type if needed. FceuMallocPtr::get()
        # returns uint8_t*; for non-uint8 types we cast back.
        if cast_type and cast_type != "uint8" and cast_type != "":
            assign_rhs = f"({cast_type}*){name}_owner.get()"
        else:
            assign_rhs = f"{name}_owner.get()"
        actions[line_idx] = (
            f"{indent}{name}_owner = FCEU_gmalloc_unique({size_expr});  "
            f"// v0.3.6: RAII-wrapped\n"
            f"{indent}{name} = {assign_rhs};"
        )

    # Step 3: replace FCEU_gfree two-line patterns. The `if (NAME)\n  FCEU_gfree(NAME);`
    # becomes a single `<NAME>_owner.reset();` line; the FCEU_gfree line is deleted.
    # Only replace when the corresponding FCEU_gmalloc is in this file (i.e.
    # NAME is in unique_names). For extern pointers freed across files, the
    # original FCEU_gfree is preserved. Skip if already converted.
    for name, line_idx in gfree_calls:
        if name not in unique_names:
            continue
        first_line = lines[line_idx]
        if f"{name}_owner.reset()" in first_line:
            continue
        m_indent = re.match(r"^(?P<indent>[ \t]*)", first_line)
        indent = m_indent.group("indent") if m_indent else "\t"
        actions[line_idx] = (
            f"{indent}{name}_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree"
        )
        # Mark the FCEU_gfree line for deletion.
        actions[line_idx + 1] = "__DELETE__"

    # Step 3b: replace single-line `FCEU_free(NAME);` calls with `<NAME>_owner.reset();`.
    # FCEU_free is the same allocator as FCEU_gfree; the FceuMallocDeleter is
    # matched to FCEU_gfree but the underlying free() call works for both. Skip
    # if NAME is not in unique_names (the NAME is not owned by a unique_ptr
    # in this TU). Skip if already converted.
    for name, line_idx in fceu_free_singles:
        if name not in unique_names:
            continue
        first_line = lines[line_idx]
        if f"{name}_owner.reset()" in first_line:
            continue
        m_indent = re.match(r"^(?P<indent>[ \t]*)", first_line)
        indent = m_indent.group("indent") if m_indent else "\t"
        actions[line_idx] = (
            f"{indent}{name}_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree"
        )

    # Apply actions in reverse line order to keep indices stable.
    for line_idx in sorted(actions.keys(), reverse=True):
        replacement = actions[line_idx]
        if replacement == "__DELETE__":
            del lines[line_idx]
            continue
        if not replacement.endswith("\n"):
            replacement += "\n"
        lines[line_idx] = replacement

    # Step 4: rewrite trailing `NAME = NULL;` near `*_owner.reset();` to nullptr.
    for i, line in enumerate(lines):
        m = NULL_ASSIGN_RE.match(line.rstrip("\n"))
        if not m:
            continue
        name = m.group("name")
        window = "".join(lines[max(0, i - 3) : i + 4])
        if f"{name}_owner.reset()" in window:
            lines[i] = line.replace("= NULL;", "= nullptr;").replace("= 0;", "= nullptr;")

    final_text = "".join(lines)
    if final_text == original:
        return False
    path.write_text(final_text, encoding="utf-8")
    return True


def revert_files() -> None:
    """Revert only the files touched by the v0.3.6 transform script.

    Manual edits to fceu.h, state.cpp, memory.h, memory.cpp, types.h are
    preserved (they were applied by hand, not by the script).
    """
    paths = [
        "src/boards",
        "src/fceu.cpp",
        "src/fds.cpp",
        "src/ines.cpp",
        "src/nsf.cpp",
        "src/drivers/common/vidblit.cpp",
    ]
    subprocess.run(
        ["git", "checkout", "HEAD", "--", *paths],
        cwd=str(REPO),
        check=False,
    )


def main() -> int:
    changed = 0
    for path in sorted(BOARDS_DIR.glob("*.cpp")):
        if transform_file(path):
            print(f"updated: {path.relative_to(REPO)}")
            changed += 1
    for path in EXTRA_FILES:
        if transform_file(path):
            print(f"updated: {path.relative_to(REPO)}")
            changed += 1
    print(f"\nTotal files updated: {changed}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--revert":
        revert_files()
        print("reverted.")
        sys.exit(0)
    sys.exit(main())
