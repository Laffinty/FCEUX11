#!/usr/bin/env python3
"""Dump mmc3_4 test ROM code around the IRQ setup and the two landing PCs."""
data = open("tests/fixtures/blargg/mmc3/mmc3_4_scanline_timing.nes", "rb").read()
prg_size = 32768

def at(pc):
    return data[16 + (pc - 0x8000) % prg_size]

def dump(pc, n=24, label=""):
    print(f"\n{label} ${pc:04X}:")
    for i in range(0, n, 8):
        row = "  ".join(f"{at(pc+i+j):02X}" for j in range(min(8, n-i)))
        print(f"  ${pc+i:04X}: {row}")

# The JSR target in the diverging sequence.
dump(0xE258, 32, "JSR $E258 target")
# C++ landing
dump(0xE242, 24, "C++ landed")
# Rust landing (IRQ handler region)
dump(0xE9AF, 32, "IRQ vector target $E9AF")
# The start sequence
dump(0xEB23, 24, "Divergence start")
