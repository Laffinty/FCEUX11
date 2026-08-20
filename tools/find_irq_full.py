#!/usr/bin/env python3
"""Compare the full irq_low value between two traces, find first diff."""
import csv
import sys


def read(path):
    rows = []
    with open(path, "r", newline="") as fp:
        for r in csv.DictReader(fp):
            rows.append({
                "frame": int(r["frame"]),
                "call": int(r["call_idx"]),
                "cyc": int(r["cycles_arg"]),
                "pc": int(r["pc_after"]),
                "cum": int(r["cum_count"]),
                "irq": int(r["irq_low"]),
            })
    return rows


def main():
    a = read(sys.argv[1])
    b = read(sys.argv[2])
    n = min(len(a), len(b))
    pc_div = irq_div = None
    for i in range(n):
        if pc_div is None and a[i]["pc"] != b[i]["pc"]:
            pc_div = i
        if irq_div is None and a[i]["irq"] != b[i]["irq"]:
            irq_div = i
        if pc_div is not None and irq_div is not None:
            break
    print(f"rows={n} first PC div={pc_div} first irq div={irq_div}")
    if irq_div is not None:
        lo = max(0, irq_div - 6)
        hi = min(n, irq_div + 8)
        for i in range(lo, hi):
            ra, rb = a[i], b[i]
            m1 = "<<" if i == pc_div else "  "
            m2 = "!!" if i == irq_div else "  "
            print(f"{m1}{m2} row {i:6d} f{ra['frame']:2d} cyc={ra['cyc']:6d} "
                  f"pc=${ra['pc']:04X}/{rb['pc']:04X} "
                  f"irq={ra['irq']:08X}/{rb['irq']:08X}")


if __name__ == "__main__":
    main()
