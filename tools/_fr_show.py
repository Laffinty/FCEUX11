import sys

def read_lines(path):
    raw = open(path, "rb").read()
    if b"\x00" in raw[:200]:
        text = raw.decode("utf-16")
    else:
        text = raw.decode("utf-8", "replace")
    return text.splitlines()

a = read_lines(sys.argv[1])
b = read_lines(sys.argv[2])
lo, hi = int(sys.argv[3]), int(sys.argv[4])
for i in range(lo, hi):
    m1 = "" if i >= len(a) or i >= len(b) or a[i] == b[i] else "<-DIFF"
    line_a = a[i] if i < len(a) else "(eof)"
    line_b = b[i] if i < len(b) else "(eof)"
    print(f"{i:5d} rust={line_a:22s} cpp={line_b:22s} {m1}")
