#!/usr/bin/env python3
"""Generate src/ops_table.inc from src/ops.inc.

Each opcode becomes a standalone function in a std::array function-pointer table.
The original switch body is preserved verbatim except trailing `break;` is removed
and a final `return;` is appended.  The generated file is intended to be included
in src/x6502.cpp after the addressing-mode/operation macros are defined.

Variants (v2.0.4 V8 "A7 alias experiment"):

  base (default)  -> src/ops_table.inc      canonical build; the `_PC`/`_A`/...
                     macros keep routing through `g_cpu.native_layout()`
                     (src/x6502abbrev.h), so the handler parameter `xp` stays
                     unused (`(void)xp;`).

  a7              -> src/ops_table_a7.inc   experiment only; the same macros are
                     re-routed through the handler parameter (`(xp)->PC`), i.e.
                     the handler reads the CPU state it was handed instead of
                     re-loading the global `g_cpu` reference.  The canonical
                     definitions are restored verbatim at the end of the file.
                     Built only when the CMake option
                     FCEUX11_X6502_ALIAS_VARIANT=ON is set; the default build
                     never includes this file.

Both variants are mechanically produced from src/ops.inc -- the 256 handlers
are never hand-edited.
"""
import argparse
import re
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
OPS_INC = PROJECT_ROOT / "src" / "ops.inc"
ABBREV_H = PROJECT_ROOT / "src" / "x6502abbrev.h"
OUT_INC = PROJECT_ROOT / "src" / "ops_table.inc"
OUT_INC_A7 = PROJECT_ROOT / "src" / "ops_table_a7.inc"

CASE_RE = re.compile(r"^case\s+(0x[0-9A-Fa-f]{2})\s*:")

# Macro name -> X6502 member name (src/x6502struct.h).  The experiment variant
# re-routes these macros through the handler parameter instead of the global
# CPU accessor, so the mapping has to be explicit.
MACRO_MEMBERS = (
    ("_PC", "PC"),
    ("_A", "A"),
    ("_X", "X"),
    ("_Y", "Y"),
    ("_S", "S"),
    ("_P", "P"),
    ("_PI", "mooPI"),
    ("_DB", "DB"),
    ("_count", "count"),
    ("_tcount", "tcount"),
    ("_IRQlow", "IRQlow"),
    ("_jammed", "jammed"),
)

DEFINE_RE = re.compile(r"^#define\s+(_[A-Za-z_][A-Za-z0-9_]*)\b")


def parse_ops_inc(text: str):
    """Return dict opcode_int -> body_text."""
    lines = text.splitlines()
    bodies: dict[int, str] = {}
    current_labels: list[int] = []
    current_body_lines: list[str] = []
    body_started = False

    def flush():
        nonlocal current_labels, current_body_lines, body_started
        if not current_labels:
            return
        body = "\n".join(current_body_lines)
        # Remove trailing break; if present so we can统一 append return.
        # The original switch body used break; to exit the switch.  In a
        # standalone function we want to fall through to the trailing return.
        body = re.sub(r"\bbreak\s*;", "", body)
        body = body.rstrip()
        for op in current_labels:
            bodies[op] = body
        current_labels = []
        current_body_lines = []
        body_started = False

    for raw_line in lines:
        line = raw_line.rstrip()
        if not line.strip():
            continue
        m = CASE_RE.search(line)
        if m:
            op = int(m.group(1), 16)
            if body_started:
                # Previous body ended; flush it.
                flush()
            current_labels.append(op)
            # Code after the colon on the same line belongs to the body.
            rest = line[m.end():]
            if rest.strip() and not rest.strip().startswith("/*"):
                current_body_lines.append(rest)
                body_started = True
            elif rest.strip().startswith("/*"):
                # comment-only continuation line; do not mark body started yet
                pass
            continue

        if current_labels:
            current_body_lines.append(line)
            body_started = True

    flush()
    return bodies


def parse_abbrev_defines(text):
    """Return {macro: verbatim '#define ...' line} for the register macros.

    The canonical definitions live in src/x6502abbrev.h; the experiment
    variant restores them verbatim at the end of its output, so the header
    stays the single source of truth.
    """
    defines = {}
    for raw in text.splitlines():
        stripped = raw.strip()
        m = DEFINE_RE.match(stripped)
        if m and m.group(1) in dict(MACRO_MEMBERS):
            defines[m.group(1)] = stripped
    missing = [name for name, _ in MACRO_MEMBERS if name not in defines]
    if missing:
        raise RuntimeError(f"{ABBREV_H.name} is missing macro definitions: {missing}")
    return defines


def build_handler_block(bodies):
    """Return the 256 handler functions as a list of lines (variant-agnostic)."""
    out = []
    for i in range(256):
        body = bodies[i]
        out.append(f"static void x6502_op_{i:02X}(X6502* xp)")
        out.append("{")
        out.append("    (void)xp;")
        if body:
            # Preserve relative indentation from ops.inc.
            for ln in body.splitlines():
                out.append("    " + ln)
        out.append("    return;")
        out.append("}")
        out.append("")
    return out


def build_dispatch_table():
    """Return the 256-entry function-pointer table as a list of lines."""
    out = []
    out.append("FCEUX11_CACHE_ALIGN static const std::array<void(*)(X6502*), 256> x6502_dispatch = {{")
    row = []
    for i in range(256):
        row.append(f"    x6502_op_{i:02X}")
        if len(row) == 16:
            out.append(",".join(row) + ",")
            row = []
    if row:
        out.append(",".join(row) + ",")
    out.append("}};")
    out.append("")
    return out


def generate(variant="base"):
    ops_text = OPS_INC.read_text(encoding="utf-8")
    bodies = parse_ops_inc(ops_text)

    if len(bodies) != 256:
        missing = [f"0x{i:02X}" for i in range(256) if i not in bodies]
        raise RuntimeError(f"Expected 256 opcodes, got {len(bodies)}. Missing: {missing}")

    out = ["// Auto-generated by scripts/generate_x6502_dispatch.py from src/ops.inc",
           "// Do not edit manually; rerun the generator after ops.inc changes.",
           "",
           "#include <array>",
           ""]

    if variant == "base":
        out.append("// Per-opcode handlers. v1.3 Legion Phase 2: the _PC / _A / ... macros")
        out.append("// from x6502abbrev.h now route through g_cpu.native_layout(), so the")
        out.append("// X6502* parameter is kept only for the existing function-pointer table")
        out.append("// signature. It is explicitly void-cast to avoid unused-parameter warnings.")
        out.append("")
        out.extend(build_handler_block(bodies))
        out.extend(build_dispatch_table())
        target = OUT_INC
    elif variant == "a7":
        defines = parse_abbrev_defines(ABBREV_H.read_text(encoding="utf-8"))
        out.append("// ============================================================")
        out.append("// A7 ALIAS EXPERIMENT VARIANT (v2.0.4 V8) -- NOT the canonical table.")
        out.append("//")
        out.append("// Regenerate with:")
        out.append("//   python scripts/generate_x6502_dispatch.py --variant a7")
        out.append("// Build it with:")
        out.append("//   cmake -S . -B build-a7 -DFCEUX11_X6502_ALIAS_VARIANT=ON")
        out.append("//")
        out.append("// The canonical macros in src/x6502abbrev.h route every register access")
        out.append("// through the global `g_cpu` reference.  This variant re-routes them")
        out.append("// through the handler parameter `xp` -- the state the dispatcher was")
        out.append("// handed -- so the compiler no longer has to re-load the global")
        out.append("// reference after each store through the layout.  The canonical")
        out.append("// definitions are restored verbatim at the end of this file.")
        out.append("// ============================================================")
        out.append("")
        for macro, member in MACRO_MEMBERS:
            out.append(f"#undef {macro}")
            out.append(f"#define {macro} (xp)->{member}")
        out.append("")
        out.extend(build_handler_block(bodies))
        out.extend(build_dispatch_table())
        out.append("// ---- restore the canonical definitions (verbatim from")
        out.append("// ---- src/x6502abbrev.h) -------------------------------------------")
        for macro, _member in MACRO_MEMBERS:
            out.append(f"#undef {macro}")
            out.append(defines[macro])
        out.append("")
        target = OUT_INC_A7
    else:
        raise RuntimeError(f"unknown variant: {variant}")

    target.write_text("\n".join(out), encoding="utf-8")
    print(f"Generated {target} ({len(bodies)} opcode handlers, variant={variant})")


def main():
    ap = argparse.ArgumentParser(description="Generate the x6502 opcode dispatch table.")
    ap.add_argument("--variant", choices=("base", "a7"), default="base",
                    help="base = canonical src/ops_table.inc; "
                         "a7 = V8 experiment table src/ops_table_a7.inc")
    args = ap.parse_args()
    generate(args.variant)


if __name__ == "__main__":
    main()
