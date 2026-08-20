#!/usr/bin/env python3
import csv

a = list(csv.DictReader(open("trace_instr_r300.csv")))
b = list(csv.DictReader(open("trace_instr_c300.csv")))
for j in range(129180, 129242):
    ra, rb = a[j], b[j]
    ia = int(ra["irq_low"])
    ib = int(rb["irq_low"])
    mark = " " if ia == ib else "!"
    print(f"{mark} row {j} f{int(ra['frame']):3d} cyc={int(ra['cycles_arg']):4d} "
          f"pc=${int(ra['pc_after']):04X}/{int(rb['pc_after']):04X} "
          f"irq={ia:08X}/{ib:08X}")
