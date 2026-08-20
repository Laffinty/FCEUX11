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
n = min(len(a), len(b))
first = None
count = 0
for i in range(n):
    if a[i] != b[i]:
        if first is None:
            first = i
        count += 1
        if count <= 20:
            print(f"row {i}: rust=[{a[i]}] cpp=[{b[i]}]")
print(f"lines rust={len(a)} cpp={len(b)} first_diff={first} total_diff_rows={count}")
