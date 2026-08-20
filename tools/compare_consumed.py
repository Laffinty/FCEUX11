#!/usr/bin/env python3
"""Compare per-call consumed count-units between two cycle-trace CSVs.

The trace records `cum_count` = the `count` accumulator AFTER each
X6502_Run call. `count` is int32 (may be negative / overdrawn). Per
call:
    count_end = count_start + cycles_arg*16 - consumed_units
where `consumed_units` is the per-call decrement total (CycTable*48 +
dispatch*48, in the mixed 1/16-budget / 1/48-consumption unit).
So consumed_units = cycles_arg*16 - (count_end - count_start).

If both CSVs have identical PC per row, the instruction streams are
identical, so consumed_units MUST match; if they differ, the caller
(C++ side) advances timestamps differently and PPU sync diverges.
"""
import csv
import sys


def read_rows(path):
    rows = []
    with open(path, "r", newline="") as fp:
        for r in csv.DictReader(fp):
            rows.append({
                "frame": int(r["frame"]),
                "call": int(r["call_idx"]),
                "cyc": int(r["cycles_arg"]),
                "pc": int(r["pc_after"]),
                "cum": int(r["cum_count"]),  # stored as uint32
            })
    return rows


def i32(v):
    """Reinterpret uint32 as int32."""
    if v >= 0x80000000:
        return v - 0x100000000
    return v


def main():
    a = read_rows(sys.argv[1])
    b = read_rows(sys.argv[2])
    n = min(len(a), len(b))
    print(f"rows: A={len(a)} B={len(b)} comparing {n}")

    prev_a = 0
    prev_b = 0
    mismatches = 0
    first = None
    for i in range(n):
        ra, rb = a[i], b[i]
        # Per-call consumed units.
        ca = i32(ra["cum"])
        cb = i32(rb["cum"])
        da = ra["cyc"] * 16 - (ca - prev_a)
        db = rb["cyc"] * 16 - (cb - prev_b)
        if ra["pc"] != rb["pc"] or da != db:
            mismatches += 1
            if first is None:
                first = i
                print(f"first mismatch at row {i}:")
                print(f"  A(frame={ra['frame']},call={ra['call']},cyc={ra['cyc']},"
                      f"pc=${ra['pc']:04X},consumed_units={da})")
                print(f"  B(frame={rb['frame']},call={rb['call']},cyc={rb['cyc']},"
                      f"pc=${rb['pc']:04X},consumed_units={db})")
        prev_a = ca
        prev_b = cb

    if mismatches == 0:
        print("OK: PC and per-call consumed_units identical across all rows")
    else:
        print(f"{mismatches} mismatching rows out of {n}")


if __name__ == "__main__":
    main()
