#!/usr/bin/env python3
"""Cross-language cycle-trace diff — Phase 4.5 cycle-drift diagnostic.

Reads two CSVs produced by `kagami-qa-cycle-trace` (one under
`FCEUX11_RUST_CPU=ON`, one under `OFF`) and finds the first row where
the two configurations diverge. The diff target is `pc_after` (the
CPU PC at end of each `Cpu::run` call); mismatches in either column
across the two CSVs localize the dominant cycle-drift root cause.

CSV format (one header + N data rows):
    frame,call_idx,cycles_arg,pc_after,cum_count

Usage:
    python tools/cross_lang_diff.py rust.csv cpp.csv [--max-mismatch N]
"""

import argparse
import csv
import dataclasses
import sys
from typing import Iterator, List, Optional, Tuple


@dataclasses.dataclass
class TraceRow:
    frame: int
    call_idx: int
    cycles_arg: int
    pc_after: int
    cum_count: int


def parse(path: str) -> List[TraceRow]:
    rows: List[TraceRow] = []
    with open(path, "r", newline="") as fp:
        reader = csv.DictReader(fp)
        for r in reader:
            rows.append(
                TraceRow(
                    frame=int(r["frame"]),
                    call_idx=int(r["call_idx"]),
                    cycles_arg=int(r["cycles_arg"]),
                    pc_after=int(r["pc_after"]),
                    cum_count=int(r["cum_count"]),
                )
            )
    return rows


def find_first_divergence(
    a: List[TraceRow], b: List[TraceRow]
) -> Optional[int]:
    """Return the index `i` such that `a[i] != b[i]` and `a[i-1] == b[i-1]`,
    or None if all rows match (modulo length difference)."""
    n = min(len(a), len(b))
    for i in range(n):
        ra, rb = a[i], b[i]
        if (
            ra.frame != rb.frame
            or ra.call_idx != rb.call_idx
            or ra.cycles_arg != rb.cycles_arg
            or ra.pc_after != rb.pc_after
            or ra.cum_count != rb.cum_count
        ):
            return i
    if len(a) != len(b):
        return n  # first extra row in the longer trace
    return None


def diff_csv(a_path: str, b_path: str, max_mismatch: int = 20) -> int:
    print(f"[diff] A: {a_path}")
    print(f"[diff] B: {b_path}")

    a = parse(a_path)
    b = parse(b_path)
    print(f"[diff] A has {len(a)} rows; B has {len(b)} rows")

    if len(a) == 0 or len(b) == 0:
        print("[diff] one trace is empty; cannot diff")
        return 1

    # PC divergence is the primary signal. cum_count polarity differs
    # (Rust ascending, C++ descending) so it's not directly comparable
    # — skip it in the per-frame summary but show it on mismatch rows.
    pc_drift_per_frame: dict[int, int] = {}
    for ra, rb in zip(a, b):
        if ra.frame != rb.frame:
            continue
        if ra.pc_after != rb.pc_after:
            # signed diff: positive = Rust ahead, negative = C++ ahead
            pc_drift_per_frame[ra.frame] = (ra.pc_after - rb.pc_after)

    print()
    print("Per-frame PC drift (Rust_pc − Cpp_pc, signed):")
    print("  frame  diff  interpretation")
    for f in sorted(pc_drift_per_frame.keys())[:max_mismatch]:
        d = pc_drift_per_frame[f]
        interp = (
            "Rust 1 byte ahead" if d == 1
            else "Rust 2 bytes ahead" if d == 2
            else "Rust 3 bytes ahead" if d == 3
            else "Rust 4 bytes ahead" if d == 4
            else "Rust ahead" if d > 0
            else "C++ ahead" if d < 0
            else "match"
        )
        print(f"  {f:4d}  {d:+4d}  {interp}")

    idx = find_first_pc_divergence(a, b)
    if idx is None:
        print("[diff] ✓ row-by-row PC identical (within shorter trace)")
        return 0

    print(f"[diff] ✗ first PC divergence at row {idx}")
    for i in range(max(idx - 2, 0), min(idx + max_mismatch, min(len(a), len(b)))):
        ra = a[i]
        rb = b[i]
        diff_marker = "  " if i < idx else ">>" if i == idx else "  "
        pc_diff = ra.pc_after - rb.pc_after
        pc_diff_s = f"{pc_diff:+d}" if pc_diff != 0 else " 0"
        cyc_match = "=" if ra.cycles_arg == rb.cycles_arg else "≠"
        print(
            f"  {diff_marker} row {i}: "
            f"A(frame={ra.frame},call={ra.call_idx},"
            f"cyc={ra.cycles_arg}{cyc_match}{rb.cycles_arg},"
            f"pc=${ra.pc_after:04X}/{rb.pc_after:04X} Δpc={pc_diff_s})"
        )

    print()
    print(
        "[diff] Hypothesis test:\n"
        "  if cycles_arg matches everywhere AND pc_after diverges,\n"
        "    the cycle-consumption per call differs by a fixed amount:\n"
        "    Rust consumes FEWER cycles per instruction than C++ (or vice\n"
        "    versa), so the same call budget advances PC differently.\n"
        "  Look for a miscounted CycTable entry — every cheap NOP /\n"
        "    read-NOP row in decode.rs is a candidate.\n"
        "  If PC divergence GROWS linearly with #calls, the rate is\n"
        "    (PC_drift_per_call) bytes/call — divide by 1 byte per\n"
        "    cheap-instruction undercount to find the offending opcode."
    )
    return 2


def find_first_pc_divergence(
    a: List[TraceRow], b: List[TraceRow]
) -> Optional[int]:
    """Return the index where pc_after first differs, ignoring cum_count
    (Rust ascending vs C++ descending polarity)."""
    n = min(len(a), len(b))
    for i in range(n):
        if a[i].pc_after != b[i].pc_after:
            return i
    if len(a) != len(b):
        return n
    return None


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("a", help="first CSV (e.g. Rust CPU = ON)")
    p.add_argument("b", help="second CSV (e.g. C++ CPU = OFF)")
    p.add_argument(
        "--max-mismatch",
        type=int,
        default=20,
        help="show at most this many row mismatches (default: 20)",
    )
    args = p.parse_args()

    try:
        return diff_csv(args.a, args.b, args.max_mismatch)
    except Exception as e:
        print(f"[diff] error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
