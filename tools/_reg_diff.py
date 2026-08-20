import re, sys

cpp = open(r"__reg_cpp.txt", "rb").read().decode("utf-16").splitlines()
rust = open(r"__reg_rust.txt", "rb").read().decode("utf-16").splitlines()

pat = re.compile(r"\[reg\] pc=\$?(?P<pc>[0-9A-Fa-f]+) op=(?P<op>[0-9A-Fa-f]+) a=(?P<a>[0-9A-Fa-f]+) x=(?P<x>[0-9A-Fa-f]+) y=(?P<y>[0-9A-Fa-f]+) s=(?P<s>[0-9A-Fa-f]+) p=(?P<p>[0-9A-Fa-f]+)")

n_lines = min(len(cpp), len(rust))
only_b = 0
non_b = 0
first_non_b = None
p_only_b = 0
any_other_field = 0
first_any_other = None

for i in range(n_lines):
    m1 = pat.match(cpp[i])
    m2 = pat.match(rust[i])
    if not m1 or not m2:
        if cpp[i] != rust[i]:
            if first_non_b is None:
                first_non_b = (i, cpp[i], rust[i])
            non_b += 1
        continue
    d1, d2 = m1.groupdict(), m2.groupdict()
    if all(d1[k] == d2[k] for k in d1):
        continue
    # all fields equal except possibly p
    fields_other = [k for k in d1 if k != "p" and d1[k] != d2[k]]
    if fields_other:
        if first_any_other is None:
            first_any_other = (i, cpp[i], rust[i])
        any_other_field += 1
    else:
        p1, p2 = int(d1["p"], 16), int(d2["p"], 16)
        if (p1 ^ p2) == 0x10:
            p_only_b += 1
        else:
            if first_non_b is None:
                first_non_b = (i, cpp[i], rust[i])
            non_b += 1

print(f"lines={n_lines}")
print(f"identical={n_lines - (only_b + non_b + any_other_field)}")
print(f"p-differs-only-by-B={p_only_b}")
print(f"non-B-p-diff={non_b}")
print(f"other-field-diff={any_other_field}")
if first_any_other:
    print("FIRST_ANY_OTHER:", first_any_other)
if first_non_b:
    print("FIRST_NON_B:", first_non_b)
