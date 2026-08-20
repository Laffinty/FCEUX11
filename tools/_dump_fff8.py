#!/usr/bin/env python3
data = open("tests/fixtures/blargg/cpu/instr_v5_all.nes", "rb").read()
prg = 16384
def atb(bank, pc):
    return data[16 + bank * prg + (pc - 0xC000)]
for b in [0, 13, 14, 15]:
    a = atb(b, 0xFFF8)
    c = atb(b, 0xFFF9)
    print(f"bank {b:2d} $FFF8: {a:02X} {c:02X} -> target ${c:02X}{a:02X}")
