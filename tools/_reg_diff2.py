import re, sys

def read_lines(path):
    raw = open(path, "rb").read()
    if b"\x00" in raw[:200]:
        text = raw.decode("utf-16")
    else:
        text = raw.decode("utf-8", "replace")
    return text.splitlines()

pat = re.compile(r"\[reg\] pc=\$?(?P<pc>[0-9A-Fa-f]{4}) op=(?P<op>[0-9A-Fa-f]{2}) a=(?P<a>[0-9A-Fa-f]{2}) x=(?P<x>[0-9A-Fa-f]{2}) y=(?P<y>[0-9A-Fa-f]{2}) s=(?P<s>[0-9A-Fa-f]{2}) p=(?P<p>[0-9A-Fa-f]{2})")

def main():
    cpp_path, rust_path = sys.argv[1], sys.argv[2]
    cpp = read_lines(cpp_path)
    rust = read_lines(rust_path)
    n = min(len(cpp), len(rust))
    ident = 0
    only_b = 0
    only_u = 0
    other = []
    for i in range(n):
        if cpp[i] == rust[i]:
            ident += 1
            continue
        m1, m2 = pat.match(cpp[i]), pat.match(rust[i])
        if not m1 or not m2:
            other.append((i, cpp[i], rust[i]))
            continue
        d1, d2 = m1.groupdict(), m2.groupdict()
        fields = [k for k in d1 if d1[k] != d2[k]]
        if not fields:
            ident += 1
        elif fields == ["p"]:
            p1, p2 = int(d1["p"], 16), int(d2["p"], 16)
            x = p1 ^ p2
            if x == 0x10:
                only_b += 1
            elif x == 0x20:
                only_u += 1
            else:
                other.append((i, cpp[i], rust[i]))
        else:
            other.append((i, cpp[i], rust[i]))
    print(f"lines_cpp={len(cpp)} lines_rust={len(rust)} compared={n}")
    print(f"identical={ident} p-only-B={only_b} p-only-U={only_u} other={len(other)}")
    for i, c, r in other[:12]:
        print(f"  row {i}: cpp=[{c}] rust=[{r}]")
    if len(cpp) != len(rust):
        print(f"  LINE-COUNT MISMATCH: cpp={len(cpp)} rust={len(rust)}")

main()
