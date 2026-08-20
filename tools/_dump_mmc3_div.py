#!/usr/bin/env python3
"""Dump bytes around the mmc3_4 divergence point and the two target PCs."""
import sys

path = "tests/fixtures/blargg/mmc3/mmc3_4_scanline_timing.nes"
data = open(path, "rb").read()
prg_banks = data[4]
prg_size = prg_banks * 16384
print(f"PRG banks: {prg_banks}, size: {prg_size}")

def at(pc):
    # Assume PRG mapped at $8000; mirror if needed.
    return data[16 + (pc - 0x8000) % prg_size]

# Start of the diverging call (both sides had pc=$EB23 at row 36229).
start = 0xEB23
print(f"\nBytes at ${start:04X} (both sides start here):")
for i in range(0, 24):
    pc = start + i
    print(f"  ${pc:04X}: {at(pc):02X}")

# The two landing PCs.
print(f"\nC++ landed at $E242, Rust landed at $E9B1")
print(f"  bytes at $E242: {[f'{at(0xE242+i):02X}' for i in range(8)]}")
print(f"  bytes at $E9B1: {[f'{at(0xE9B1+i):02X}' for i in range(8)]}")

# Check vectors
print(f"\nIRQ vector $FFFE/FFFF: {at(0xFFFE):02X}{at(0xFFFF):02X}")
print(f"NMI vector $FFFA/FFFB: {at(0xFFFA):02X}{at(0xFFFB):02X}")
