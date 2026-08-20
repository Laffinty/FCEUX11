#!/usr/bin/env python3
"""Find the first call where irq_low (EXTERNAL bit, 0x001) differs between
the Rust and C++ traces, relative to the PC divergence."""
import csv
import sys

FCEU_IQEXT = 0x001


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

    # 1. First PC divergence
    pc_div = None
    for i in range(n):
        if a[i]["pc"] != b[i]["pc"]:
            pc_div = i
            break

    # 2. First irq EXTERNAL divergence
    irq_div = None
    for i in range(n):
        ia = a[i]["irq"] & FCEU_IQEXT
        ib = b[i]["irq"] & FCEU_IQEXT
        if ia != ib:
            irq_div = i
            break

    print(f"rows={n}")
    print(f"first PC divergence: row {pc_div}")
    print(f"first EXTERNAL-irq divergence: row {irq_div}")

    if irq_div is not None:
        lo = max(0, irq_div - 6)
        hi = min(n, irq_div + 8)
        print(f"\nIRQ EXTERNAL bit around divergence:")
        for i in range(lo, hi):
            ra, rb = a[i], b[i]
            ia = "I" if (ra["irq"] & FCEU_IQEXT) else "."
            ib = "I" if (rb["irq"] & FCEU_IQEXT) else "."
            pc_mark = "<<" if i == pc_div else "  "
            irq_mark = "!!" if i == irq_div else "  "
            print(f"{pc_mark}{irq_mark} row {i:6d} f{ra['frame']:2d} "
                  f"cyc={ra['cyc']:6d} pc=${ra['pc']:04X}/{rb['pc']:04X} "
                  f"irqext={ia}/{ib}")


if __name__ == "__main__":
    main()
