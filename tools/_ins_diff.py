import re, sys

def read_lines(path):
    raw = open(path, "rb").read()
    if b"\x00" in raw[:200]:
        text = raw.decode("utf-16")
    else:
        text = raw.decode("utf-8", "replace")
    return text.splitlines()

pat = re.compile(r"\[ins\] pc=\$?(?P<pc>[0-9A-Fa-f]{4}) op=(?P<op>[0-9A-Fa-f]{2}) cnt=(?P<cnt>-?\d+)")

def parse(lines):
    out = []
    for ln in lines:
        m = pat.match(ln)
        if not m:
            continue
        out.append((int(m.group("pc"), 16), int(m.group("op"), 16), int(m.group("cnt"))))
    return out

a = parse(read_lines(sys.argv[1]))
b = parse(read_lines(sys.argv[2]))
n = min(len(a), len(b))
print(f"ins rust={len(a)} cpp={len(b)}")
first = None
for i in range(n):
    if a[i][0] != b[i][0] or a[i][1] != b[i][1]:
        print(f"STREAM DIVERGENCE at line {i}: rust pc=${a[i][0]:04X} op={a[i][1]:02X}  cpp pc=${b[i][0]:04X} op={b[i][1]:02X}")
        first = i
        break
    if a[i][2] != b[i][2]:
        print(f"CNT DIVERGENCE at line {i}: pc=${a[i][0]:04X} op={a[i][1]:02X}  cnt rust={a[i][2]} cpp={b[i][2]} (diff {a[i][2]-b[i][2]})")
        first = i
        break
if first is None:
    print("no divergence in first", n, "lines")
    sys.exit(0)
lo = max(0, first - 10)
hi = min(n, first + 10)
print(f"--- context (per-instruction cost = (cnt[i]-cnt[i+1])/48) ---")
for i in range(lo, hi):
    cost_a = (a[i][2] - a[i+1][2]) // 48 if i + 1 < n else None
    cost_b = (b[i][2] - b[i+1][2]) // 48 if i + 1 < n else None
    mark = "  "
    if i == first:
        mark = "<<"
    print(f"{mark} {i:7d} pc=${a[i][0]:04X} op={a[i][1]:02X} cnt={a[i][2]:8d}/{b[i][2]:8d} "
          f"cost={cost_a}/{cost_b}")
