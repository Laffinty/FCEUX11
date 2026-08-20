#!/usr/bin/env python3
"""Decode mmc3_4 code around $E23C-$E270 and $EB23 to understand the IRQ setup."""
data = open("tests/fixtures/blargg/mmc3/mmc3_4_scanline_timing.nes", "rb").read()
prg_size = 32768

def at(pc):
    return data[16 + (pc - 0x8000) % prg_size]

mn = {
    0xA9: "LDA #$", 0x20: "JSR $", 0x60: "RTS", 0x4C: "JMP $",
    0x48: "PHA", 0x68: "PLA", 0x08: "PHP", 0x28: "PLP",
    0x18: "CLC", 0x38: "SEC", 0x78: "SEI", 0x58: "CLI",
    0xC9: "CMP #$", 0xD0: "BNE +", 0xF0: "BEQ +", 0xB0: "BCS +",
    0x90: "BCC +", 0x4A: "LSR A", 0xE6: "INC $", 0xA5: "LDA $",
    0x69: "ADC #$", 0xEA: "NOP", 0x85: "STA $", 0x8D: "STA $",
    0x06: "ASL $", 0x40: "RTI", 0xE0: "CPX #$", 0x8A: "TXA",
    0xAA: "TAX", 0xA8: "TAY", 0x98: "TYA", 0xCA: "DEX",
    0x2C: "BIT $", 0xAD: "LDA $", 0xB2: "?", 0x86: "STX $",
}

def disasm(pc, n):
    print(f"  ${pc:04X}: ", end="")
    i = 0
    while i < n:
        op = at(pc + i)
        name = mn.get(op, f"{op:02X}?")
        size = 1
        operand = ""
        if op in (0xA9, 0xC9, 0x69, 0xE0):  # immediate
            size = 2; operand = f"{at(pc+i+1):02X}"
        elif op in (0x20, 0x4C):  # abs
            size = 3; operand = f"{at(pc+i+2):02X}{at(pc+i+1):02X}"
        elif op in (0x8D, 0xAD, 0x2C):  # abs
            size = 3; operand = f"{at(pc+i+2):02X}{at(pc+i+1):02X}"
        elif op in (0xE6, 0xA5, 0x85, 0x06, 0x86):  # zp
            size = 2; operand = f"{at(pc+i+1):02X}"
        elif op in (0xD0, 0xF0, 0xB0, 0x90):  # branch
            size = 2; operand = f"{at(pc+i+1):02X}"
        if name.endswith("$"):
            print(f"{name}{operand} ", end="")
        elif name.startswith(("BNE", "BEQ", "BCS", "BCC")):
            off = at(pc+i+1)
            tgt = (pc + i + 2 + (off - 256 if off >= 0x80 else off)) & 0xFFFF
            print(f"{name}${tgt:04X} ", end="")
        else:
            print(f"{name} ", end="")
        i += size
    print()

print("$E23C-$E270:")
disasm(0xE23C, 0x34)
print("\n$EB20-$EB40:")
disasm(0xEB20, 0x20)
