import re

lines = open(r"tests/fixtures/nestest.log", encoding="utf-8", errors="replace").read().splitlines()
# find lines whose disassembly contains PLP / PHP / RTI, and show the next line's P
for i, ln in enumerate(lines):
    m = re.search(r"(PLP|PHP|RTI|BRK|IRQ)", ln)
    if m:
        nxt = lines[i+1] if i+1 < len(lines) else ""
        p_this = re.search(r"P:([0-9A-F]{2})", ln)
        p_next = re.search(r"P:([0-9A-F]{2})", nxt)
        print(f"{i:5d} {ln[:70]:70s} P={p_this.group(1) if p_this else '?':>2}  next: {nxt[:56]:56s} P={p_next.group(1) if p_next else '?'}")
        if i > 500:
            break
