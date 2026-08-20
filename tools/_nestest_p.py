import re, collections

lines = open(r"tests/fixtures/nestest.log", encoding="utf-8", errors="replace").read().splitlines()
pat = re.compile(r"P:([0-9A-F]{2})")
vals = collections.Counter()
first = []
for ln in lines:
    m = pat.search(ln)
    if m:
        v = m.group(1)
        vals[v] += 1
        if len(first) < 12:
            first.append((ln[:38].strip(), v))
print("total lines:", len(lines))
print("P column distribution:", dict(sorted(vals.items())))
print("first 12 (pc, P):", first)
