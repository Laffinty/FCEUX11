import re, collections, sys

def read_lines(path):
    raw = open(path, "rb").read()
    if b"\x00" in raw[:200]:
        text = raw.decode("utf-16")
    else:
        text = raw.decode("utf-8", "replace")
    return text.splitlines()

# address distribution of reads (top-level 4KB page buckets)
a = read_lines(r"__fr_rust2.txt")
pat = re.compile(r"\[fr\] ([0-9A-F]{4})=[0-9A-F]{2}")
buckets = collections.Counter()
addrs = []
for ln in a:
    m = pat.match(ln)
    if m:
        ad = int(m.group(1), 16)
        addrs.append(ad)
        buckets[ad >> 12] += 1
print("reads:", len(addrs))
for k in sorted(buckets):
    print(f"  ${k:01X}000-${k:01X}FFF: {buckets[k]}")
# min/max in E000-FFFF
e = [x for x in addrs if 0xE000 <= x <= 0xFFFF]
print(f"E000-FFFF reads: {len(e)} min=${min(e):04X} max=${max(e):04X}")
# contiguous coverage summary: count reads per 256-byte page
p256 = collections.Counter(x >> 8 for x in addrs if 0xE000 <= x <= 0xFFFF)
for k in sorted(p256):
    print(f"  ${(k<<8):04X}-${((k<<8)|0xFF):04X}: {p256[k]}")
