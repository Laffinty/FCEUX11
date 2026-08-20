import sys

def read_lines(path):
    raw = open(path, "rb").read()
    if b"\x00" in raw[:200]:
        text = raw.decode("utf-16")
    else:
        text = raw.decode("utf-8", "replace")
    return text.splitlines()

a = read_lines(r"__reg_rust2.txt")
b = read_lines(r"__reg_cpp2.txt")

# Find E3D3 LDY executions and print following in-range lines.
# The divergent execution is the one right after an RTS; the reg log
# can't show that directly, so print ALL E3D3 hits with 5 following lines.
def hits(lines, needle):
    return [i for i, ln in enumerate(lines) if needle in ln]

ha = hits(a, "pc=$E3D3 op=A4")
print(f"rust E3D3 hits: {len(ha)}")
# Show every hit's next 5 lines (rust only; cpp identical per earlier diff)
for n, i in enumerate(ha[:12]):
    print(f"--- hit {n} at line {i} ---")
    for j in range(i, min(len(a), i + 6)):
        print(f"{j:6d} {a[j]}")
    print()
print("...")
for n, i in enumerate(ha[-4:]):
    print(f"--- hit {len(ha)-4+n} at line {i} ---")
    for j in range(i, min(len(a), i + 6)):
        print(f"{j:6d} {a[j]}")
    print()
