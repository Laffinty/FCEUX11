#!/usr/bin/env python3
"""Dump instr_v5_all bytes around $EA71-$EA95 and decode (bank 15, fixed)."""
data = open("tests/fixtures/blargg/cpu/instr_v5_all.nes", "rb").read()
prg_size = 16384

def at(pc):
    # $C000-$FFFF fixed to bank 15 (MMC1 mode 3, 16 banks).
    if pc >= 0xC000:
        bank_off = 15 * prg_size + (pc - 0xC000)
    else:
        bank_off = (pc - 0x8000) % (16 * prg_size)
    return data[16 + bank_off]

print("$EA71-$EA95 bytes (bank 15):")
for i in range(0x71, 0x96, 8):
    row = " ".join(f"{at(0xE000+i+j):02X}" for j in range(min(8, 0x96 - i)))
    print(f"  ${0xE000+i:04X}: {row}")

# Decode with a simple table
mn = {
    0xA9: ("LDA #$", 2), 0x8D: ("STA $", 3), 0xAD: ("LDA $", 3), 0x20: ("JSR $", 3),
    0x60: ("RTS", 1), 0x4C: ("JMP $", 3), 0x6C: ("JMP ($", 3), 0x48: ("PHA", 1),
    0x68: ("PLA", 1), 0x08: ("PHP", 1), 0x28: ("PLP", 1), 0xA2: ("LDX #$", 2),
    0xBD: ("LDA $", 3), 0x9D: ("STA $", 3), 0xE8: ("INX", 1), 0xD0: ("BNE $", 2),
    0xF0: ("BEQ $", 2), 0xA0: ("LDY #$", 2), 0xC8: ("INY", 1), 0x88: ("DEY", 1),
    0xCA: ("DEX", 1), 0xAA: ("TAX", 1), 0xA8: ("TAY", 1), 0x8A: ("TXA", 1),
    0x98: ("TYA", 1), 0x85: ("STA $", 2), 0xA5: ("LDA $", 2), 0xE6: ("INC $", 2),
    0xC6: ("DEC $", 2), 0xEA: ("NOP", 1), 0x18: ("CLC", 1), 0x38: ("SEC", 1),
    0x4A: ("LSR A", 1), 0x0A: ("ASL A", 1), 0x86: ("STX $", 2), 0x84: ("STY $", 2),
    0xB0: ("BCS $", 2), 0x90: ("BCC $", 2), 0xD0: ("BNE $", 2), 0xC9: ("CMP #$", 2),
    0x10: ("BPL $", 2), 0x30: ("BMI $", 2), 0x50: ("BVC $", 2), 0x70: ("BVS $", 2),
    0x2C: ("BIT $", 3), 0x5C: ("???", 3), 0x1C: ("???", 3), 0x5A: ("PHY", 1),
    0x7A: ("PLY", 1), 0xDA: ("PHX", 1), 0xFA: ("PLX", 1), 0x3A: ("DEC A", 1),
    0x1A: ("INC A", 1), 0x80: ("NOP #$", 2), 0x89: ("NOP #$", 2),
}

print("\nDecode from $EA71:")
pc = 0xEA71
for _ in range(20):
    op = at(pc)
    name, size = mn.get(op, (f"{op:02X}?", 1))
    if name.endswith("$") and size == 3:
        opstr = f"{name}{at(pc+2):02X}{at(pc+1):02X}"
    elif name.endswith("$") and size == 2:
        opstr = f"{name}{at(pc+1):02X}"
    elif name.endswith("$") and size == 1:
        opstr = name + "zp"
    else:
        opstr = name
    print(f"  ${pc:04X}: {opstr}")
    pc += size
