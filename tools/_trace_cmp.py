import csv, sys

def read(path):
    rows = []
    with open(path, "r", newline="") as fp:
        for r in csv.DictReader(fp):
            rows.append(f"{r['frame']},{r['call_idx']},{r['cycles_arg']},{r['pc_after']},{r['cum_count']},{r['irq_low']}")
    return rows

a = read("trace_fr_rust.csv")   # 136-frame run (LOG_FRAME only)
b = read("trace_instr_r300b.csv")  # 300-frame run (LOG_REG only)
n = min(len(a), len(b))
first = None
for i in range(n):
    if a[i] != b[i]:
        first = i
        break
print(f"a={len(a)} b={len(b)} first_diff={first}")
if first is not None:
    print(f"  a: {a[first]}")
    print(f"  b: {b[first]}")
