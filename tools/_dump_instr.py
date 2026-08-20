#!/usr/bin/env python3
"""Dump instr_v5_all reset vector and initialization code."""
data = open("tests/fixtures/blargg/cpu/instr_v5_all.nes", "rb").read()
prg_banks = data[4]
prg_size = prg_banks * 16384
print(f"PRG banks: {prg_banks}, size: {prg_size}")

def at(pc):
    return data[16 + (pc - 0x8000) % prg_size]

def dump(pc, n=16, label=""):
    print(f"\n{label} ${pc:04X}:")
    for i in range(0, n, 16):
        row = " ".join(f"{at(pc+i+j):02X}" for j in range(min(16, n-i)))
        print(f"  ${pc+i:04X}: {row}")

# Vectors
print("Reset vector $FFFC/FFFD:")
r = at(0xFFFC) | (at(0xFFFD) << 8)
print(f"  ${r:04X}")
dump(r, 64, "Reset handler")

print("\nNMI vector $FFFA/FFFB:")
n = at(0xFFFA) | (at(0xFFFB) << 8)
print(f"  ${n:04X}")
dump(n, 16, "NMI handler")

print("\nIRQ vector $FFFE/FFFF:")
i = at(0xFFFE) | (at(0xFFFF) << 8)
print(f"  ${i:04X}")
dump(i, 16, "IRQ handler")

# The Rust frame0 end PC ($E74A) vs C++ ($E84A)
dump(0xE74A, 16, "Rust frame0 end")
dump(0xE84A, 16, "C++ frame0 end")
