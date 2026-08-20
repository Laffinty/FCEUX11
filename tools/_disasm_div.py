#!/usr/bin/env python3
"""Dump and disassemble instr_v5 code around the frame-135 divergence."""
data = open("tests/fixtures/blargg/cpu/instr_v5_all.nes", "rb").read()
prg = 16384

def atb(bank, pc):
    if pc >= 0xC000:
        return data[16 + bank * prg + (pc - 0xC000)]
    return data[16 + (pc - 0x8000) % (16 * prg)]

mn = {
    0xA9: ("LDA #$", 2), 0x8D: ("STA $", 3), 0xAD: ("LDA $", 3), 0x20: ("JSR $", 3),
    0x60: ("RTS", 1), 0x4C: ("JMP $", 3), 0x48: ("PHA", 1), 0x68: ("PLA", 1),
    0xA2: ("LDX #$", 2), 0x4A: ("LSR A", 1), 0xC9: ("CMP #$", 2), 0xD0: ("BNE $", 2),
    0xF0: ("BEQ $", 2), 0xA0: ("LDY #$", 2), 0xE8: ("INX", 1), 0xC8: ("INY", 1),
    0x88: ("DEY", 1), 0xCA: ("DEX", 1), 0xAA: ("TAX", 1), 0xA8: ("TAY", 1),
    0x8A: ("TXA", 1), 0x98: ("TYA", 1), 0x85: ("STA $", 2), 0xA5: ("LDA $", 2),
    0xEA: ("NOP", 1), 0x18: ("CLC", 1), 0x38: ("SEC", 1), 0x86: ("STX $", 2),
    0x84: ("STY $", 2), 0xB0: ("BCS $", 2), 0x90: ("BCC $", 2), 0x10: ("BPL $", 2),
    0x30: ("BMI $", 2), 0x50: ("BVC $", 2), 0x70: ("BVS $", 2), 0x2C: ("BIT $", 3),
    0x24: ("BIT $", 2), 0x6C: ("JMP ($", 3), 0x40: ("RTI", 1), 0x00: ("BRK", 1),
    0x08: ("PHP", 1), 0x28: ("PLP", 1), 0x69: ("ADC #$", 2), 0x65: ("ADC $", 2),
    0x75: ("ADC $", 2), 0xE0: ("CPX #$", 2), 0xE4: ("CPX $", 2), 0xC0: ("CPY #$", 2),
    0xC4: ("CPY $", 2), 0x0A: ("ASL A", 1), 0x2A: ("ROL A", 1), 0x6A: ("ROR A", 1),
    0x3A: ("DEC A", 1), 0x1A: ("INC A", 1), 0x5A: ("PHY", 1), 0x7A: ("PLY", 1),
    0xDA: ("PHX", 1), 0xFA: ("PLX", 1), 0x9C: ("STY $", 3), 0x9E: ("STX $", 3),
    0x99: ("STA $", 3), 0xBD: ("LDA $", 3), 0xB9: ("LDA $", 3), 0xB5: ("LDA $", 2),
    0xB4: ("LDY $", 2), 0xB6: ("LDX $", 2), 0xAE: ("LDX $", 3), 0xAC: ("LDY $", 3),
    0x8C: ("STY $", 3), 0x8E: ("STX $", 3), 0xC5: ("CMP $", 2), 0xE6: ("INC $", 2),
    0xC6: ("DEC $", 2), 0xE9: ("SBC #$", 2), 0xE5: ("SBC $", 2), 0x09: ("ORA #$", 2),
    0x05: ("ORA $", 2), 0x29: ("AND #$", 2), 0x25: ("AND $", 2), 0x49: ("EOR #$", 2),
    0x45: ("EOR $", 2), 0x0D: ("ORA $", 3), 0x2D: ("AND $", 3), 0x4D: ("EOR $", 3),
    0x6D: ("ADC $", 3), 0xED: ("SBC $", 3), 0xCD: ("CMP $", 3), 0xEE: ("INC $", 3),
    0xCE: ("DEC $", 3), 0xA6: ("LDX $", 2), 0x91: ("STA ($", 2), 0xB1: ("LDA ($", 2),
    0x92: ("STA ($", 2), 0xB2: ("LDA ($", 2), 0x96: ("STX $", 2), 0x97: ("STA ($", 2),
    0x76: ("ROR $", 2), 0x16: ("ASL $", 2), 0x06: ("ASL $", 2), 0x26: ("ROL $", 2),
    0x46: ("LSR $", 2), 0x56: ("LSR $", 2), 0x66: ("ROR $", 2), 0x0E: ("ASL $", 3),
    0x1E: ("ASL $", 3), 0x2E: ("ROL $", 3), 0x3E: ("ROL $", 3), 0x4E: ("LSR $", 3),
    0x5E: ("LSR $", 3), 0x6E: ("ROR $", 3), 0x7E: ("ROR $", 3), 0x7D: ("ADC $", 3),
    0x79: ("ADC $", 3), 0x7D: ("ADC $", 3), 0x39: ("AND $", 3), 0x3D: ("AND $", 3),
    0x19: ("ORA $", 3), 0x1D: ("ORA $", 3), 0x59: ("EOR $", 3), 0x5D: ("EOR $", 3),
    0xD9: ("CMP $", 3), 0xDD: ("CMP $", 3), 0xF9: ("SBC $", 3), 0xFD: ("SBC $", 3),
}

def disasm(pc, n, bank=15):
    print(f"--- bank {bank} ${pc:04X} ---")
    p = pc
    for _ in range(n):
        op = atb(bank, p)
        name, size = mn.get(op, (f"{op:02X}?", 1))
        if name.endswith("$") and size == 3:
            opstr = f"{name}{atb(bank, p+2):02X}{atb(bank, p+1):02X}"
        elif name.endswith("$") and size == 2:
            opstr = f"{name}{atb(bank, p+1):02X}"
        else:
            opstr = name
        if op in (0xD0, 0xF0, 0xB0, 0x90, 0x10, 0x30, 0x50, 0x70) and size == 2:
            off = atb(bank, p + 1)
            tgt = (p + 2 + (off - 256 if off >= 0x80 else off)) & 0xFFFF
            opstr = f"{name}${tgt:04X}"
        print(f"  ${p:04X}: {opstr}")
        p += size

# The divergence start: both sides at $E303, then rust->$E2DD, cpp->$E2F3
disasm(0xE2D0, 20)
disasm(0xE300, 20)
