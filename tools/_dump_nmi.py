#!/usr/bin/env python3
"""Dump NMI vector and target."""
data = open("tests/fixtures/mapper_nrom.nes", "rb").read()
prg_size = 16384

def at(pc):
    return data[16 + (pc - 0x8000) % prg_size]

print("NMI vector $FFFA/$FFFB:")
a = at(0xFFFA)
b = at(0xFFFB)
print(f"  $FFFA: {a:02X}")
print(f"  $FFFB: {b:02X}")
print(f"  NMI vector = ${b:02X}{a:02X}")
pc = (b << 8) | a
print()
print(f"NMI target = ${pc:04X}")
print("First 16 bytes:")
for i in range(16):
    print(f"  +{i:02d}: {at(pc+i):02X}")
print()
print("Bytes at $F5C0 (call 1 end, call 2 start):")
for i in range(40):
    print(f"  0x{0xF5C0+i:04X}: {at(0xF5C0+i):02X}")
