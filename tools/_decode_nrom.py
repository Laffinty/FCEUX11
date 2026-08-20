#!/usr/bin/env python3
"""Quick look at the nrom fixture ROM bytes around the divergence points."""
import sys

data = open("tests/fixtures/mapper_nrom.nes", "rb").read()
print("size:", len(data))
print("header:", data[:16].hex())

prg_banks = data[4]
prg_size = prg_banks * 16384
print(f"PRG banks: {prg_banks}, size: {prg_size}")

# nrom mirrors 16K PRG to $C000-$FFFF if PRG is 16K
def prg_offset(pc):
    offset_in_prg = pc - 0x8000
    return 16 + offset_in_prg % prg_size

# Annotated PC list. cpp_pc is the C++ PC, rust_pc is the Rust CPU PC.
# diff = rust_pc - cpp_pc (positive = Rust ahead).
points = [
    ("first diff", 0xF5F8, 0xF5F9),
    ("call 3",     0xF5FA, 0xF5FB),
    ("call 4",     0xFA69, 0xFA6A),
    ("call 5",     0xFA94, 0xFA95),
    ("call 6",     0xFA9F, 0xFAA1),
    ("call 8",     0xFACD, 0xFACF),
    ("call 10",    0xFAD8, 0xFADB),
    ("call 12",    0xFB05, 0xFB09),
    ("call 14",    0xFB11, 0xFB15),
    ("call 16",    0xFB3E, 0xFB43),
]

# Cheap 6502 read-NOP opcodes (cycle cost 3 unless base+extras bump)
# Quick op-table for what we expect at typical addresses.
read_nop = {
    0x04: ("NOP zp", 2, 3), 0x44: ("NOP zp", 2, 3), 0x64: ("NOP zp", 2, 3),
    0x0C: ("NOP abs", 3, 4),
    0x14: ("NOP zpx", 2, 4), 0x54: ("NOP zpx", 2, 4), 0x74: ("NOP zpx", 2, 4),
    0x1C: ("NOP absx", 3, 4), 0x3C: ("NOP absx", 3, 4), 0x5C: ("NOP absx", 3, 4),
    0x7C: ("NOP absx", 3, 4), 0xDC: ("NOP absx", 3, 4), 0xFC: ("NOP absx", 3, 4),
    0x80: ("NOP imm", 2, 2), 0x82: ("NOP imm", 2, 2), 0x89: ("NOP imm", 2, 2),
    0xC2: ("NOP imm", 2, 2), 0xE2: ("NOP imm", 2, 2),
    0xEA: ("NOP impl", 1, 2),
    0x1A: ("NOP impl", 1, 2), 0x3A: ("NOP impl", 1, 2),
    0x5A: ("NOP impl", 1, 2), 0x7A: ("NOP impl", 1, 2),
    0xDA: ("NOP impl", 1, 2), 0xFA: ("NOP impl", 1, 2),
}

for label, cpp_pc, rust_pc in points:
    cpp_off = prg_offset(cpp_pc)
    print(f"\n{label}: cpp PC ${cpp_pc:04X} (=file {cpp_off:5d}) rust PC ${rust_pc:04X}")
    cpp_bytes = data[cpp_off:cpp_off+8]
    print(f"  C++ bytes:   {cpp_bytes.hex(' ')}")
    opc = cpp_bytes[0]
    base = read_nop.get(opc, ("?", 1, 0))
    print(f"  opcode at offset 0:    {opc:02X} {base[0]:<14} {base[1]}B+{base[2]}C")

    # Now show the Rust CPU's instruction list ending at the Rust PC
    # by walking backwards from cpp_pc until we hit rust_pc.
    # Heuristic: walk back instruction-by-instruction.
    walk = [cpp_pc]
    p = cpp_pc
    seen = set()
    while p > rust_pc and len(seen) < 32:
        seen.add(p)
        # try 1-byte then 2-byte then 3-byte
        for sz in (1, 2, 3):
            off = prg_offset(p - sz)
            if data[off] in read_nop:
                p -= sz
                walk.append(p)
                break
        else:
            # unknown opcode, assume 1-byte for safety
            p -= 1
            walk.append(p)
    print(f"  walk back {len(walk)-1} instrs to get from ${cpp_pc:04X} to ${rust_pc:04X}")
    print(f"  Rust CPU executed {len(walk)-1} more instructions with same cycles_arg")
