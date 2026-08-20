import csv, sys

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

a = read("trace_instr_r300b.csv")
b = read("trace_instr_c300b.csv")
n = min(len(a), len(b))
pc_div = None
cum_div = None
for i in range(n):
    if pc_div is None and a[i]["pc"] != b[i]["pc"]:
        pc_div = i
    if cum_div is None and a[i]["cum"] != b[i]["cum"]:
        cum_div = i
    if pc_div is not None and cum_div is not None:
        break
print(f"rows={n} first PC div={pc_div} first cum div={cum_div}")
for i in range(max(0, pc_div - 8), min(n, pc_div + 12)):
    ra, rb = a[i], b[i]
    m1 = "<<" if i == pc_div else "  "
    m2 = "##" if i == cum_div else "  "
    print(f"{m1}{m2} row {i:6d} f{ra['frame']:3d} cyc={ra['cyc']:6d} pc=${ra['pc']:04X}/{rb['pc']:04X} "
          f"cum={ra['cum']}/{rb['cum']} irq={ra['irq']:04X}/{rb['irq']:04X}")
