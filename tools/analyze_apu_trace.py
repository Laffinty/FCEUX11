#!/usr/bin/env python3
"""Analyze E3 APU traces: compute absolute-cycle deltas for $4017 writes,
quarter boundaries (FSU), and $4015 reads, accounting for frame-boundary
timestamp resets (NTSC frame = 29780 cycles).

Usage: python tools/analyze_apu_trace.py <trace.err> [--all]
Prints events with abs cycle and delta from the most recent $00 write.
"""
import sys, re

FRAME = 29780

def main(path, show_all):
    cur_frame = 0
    last_ts = None
    write_abs = None
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        for li, line in enumerate(f):
            line = line.strip()
            if not line.startswith("E3 "):
                continue
            m = re.search(r"W4017_IN ts=(\d+) .*V=0x([0-9A-Fa-f]+)", line)
            if m:
                ts = int(m.group(1))
                v = int(m.group(2), 16)
                if last_ts is not None and ts < last_ts:
                    cur_frame += 1
                abs_c = cur_frame * FRAME + ts
                last_ts = ts
                if v == 0x00:
                    write_abs = abs_c
                delta = f"-{abs_c - write_abs}" if write_abs is not None else "?"
                print(f"L{li:6d} {abs_c:8d} WRITE V=0x{v:02X} delta={delta}")
                continue
            m = re.search(r"FSU ts=(\d+) fcnt=(\d+)", line)
            if m:
                ts = int(m.group(1))
                fcnt = int(m.group(2))
                if last_ts is not None and ts < last_ts:
                    cur_frame += 1
                abs_c = cur_frame * FRAME + ts
                last_ts = ts
                if show_all or fcnt in (0, 2):
                    delta = f"-{abs_c - write_abs}" if write_abs is not None else "?"
                    print(f"L{li:6d} {abs_c:8d} FSU{fcnt} delta={delta}")
                continue
            m = re.search(r"R4015 ts=(\d+) .*sirq=0x([0-9A-Fa-f]+)", line)
            if m:
                ts = int(m.group(1))
                sirq = int(m.group(2), 16)
                if last_ts is not None and ts < last_ts:
                    cur_frame += 1
                abs_c = cur_frame * FRAME + ts
                last_ts = ts
                delta = f"-{abs_c - write_abs}" if write_abs is not None else "?"
                print(f"L{li:6d} {abs_c:8d} READ  sirq={sirq:02X} delta={delta}")
                continue

if __name__ == "__main__":
    main(sys.argv[1], "--all" in sys.argv)
