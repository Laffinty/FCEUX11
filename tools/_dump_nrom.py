#!/usr/bin/env python3
"""Dump nrom ROM bytes around divergence points."""
data = open("tests/fixtures/mapper_nrom.nes", "rb").read()
prg_size = 16384

def at(pc):
    return data[16 + (pc - 0x8000) % prg_size]

print("Bytes from $F5C0 (call 1 end) to $F5F9 (call 2 end Rust):")
for pc in range(0xF5C0, 0xF5FA):
    b = at(pc)
    marker = ""
    if pc == 0xF5C0: marker = "  <- call 1 end (both)"
    elif pc == 0xF5E8: marker = "  <- C++ call 2 end"
    elif pc == 0xF5E9: marker = "  <- Rust call 2 end (= Rust +1 vs C++)"
    print(f"  0x{pc:04X}: {b:02X}{marker}")
