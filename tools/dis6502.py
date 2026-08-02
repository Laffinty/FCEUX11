#!/usr/bin/env python3
"""Minimal 6502 disassembler for NES ROMs (iNES format).

Usage: python tools/dis6502.py <rom.nes> [start_addr] [count]
Disassembles from the reset vector by default.
"""
import sys

def load_ines(path):
    with open(path, "rb") as f:
        d = f.read()
    assert d[0:4] == b"NES\x1a", "not iNES"
    prg_size = d[4] * 16384
    prg = d[16:16 + prg_size]
    return prg

OPS = {
    0x00: ("BRK", 1), 0x01: ("ORA (zp,X)", 2), 0x05: ("ORA zp", 2),
    0x06: ("ASL zp", 2), 0x08: ("PHP", 1), 0x09: ("ORA #imm", 2),
    0x0A: ("ASL A", 1), 0x0D: ("ORA abs", 3), 0x0E: ("ASL abs", 3),
    0x10: ("BPL rel", 2), 0x11: ("ORA (zp),Y", 2), 0x15: ("ORA zp,X", 2),
    0x16: ("ASL zp,X", 2), 0x18: ("CLC", 1), 0x19: ("ORA abs,Y", 3),
    0x1D: ("ORA abs,X", 3), 0x1E: ("ASL abs,X", 3),
    0x20: ("JSR abs", 3), 0x21: ("AND (zp,X)", 2), 0x24: ("BIT zp", 2),
    0x25: ("AND zp", 2), 0x26: ("ROL zp", 2), 0x28: ("PLP", 1),
    0x29: ("AND #imm", 2), 0x2A: ("ROL A", 1), 0x2C: ("BIT abs", 3),
    0x2D: ("AND abs", 3), 0x2E: ("ROL abs", 3),
    0x30: ("BMI rel", 2), 0x31: ("AND (zp),Y", 2), 0x35: ("AND zp,X", 2),
    0x36: ("ROL zp,X", 2), 0x38: ("SEC", 1), 0x39: ("AND abs,Y", 3),
    0x3D: ("AND abs,X", 3), 0x3E: ("ROL abs,X", 3),
    0x40: ("RTI", 1), 0x41: ("EOR (zp,X)", 2), 0x45: ("EOR zp", 2),
    0x46: ("LSR zp", 2), 0x48: ("PHA", 1), 0x49: ("EOR #imm", 2),
    0x4A: ("LSR A", 1), 0x4C: ("JMP abs", 3), 0x4D: ("EOR abs", 3),
    0x4E: ("LSR abs", 3),
    0x50: ("BVC rel", 2), 0x51: ("EOR (zp),Y", 2), 0x55: ("EOR zp,X", 2),
    0x56: ("LSR zp,X", 2), 0x58: ("CLI", 1), 0x59: ("EOR abs,Y", 3),
    0x5D: ("EOR abs,X", 3), 0x5E: ("LSR abs,X", 3),
    0x60: ("RTS", 1), 0x61: ("ADC (zp,X)", 2), 0x65: ("ADC zp", 2),
    0x66: ("ROR zp", 2), 0x68: ("PLA", 1), 0x69: ("ADC #imm", 2),
    0x6A: ("ROR A", 1), 0x6C: ("JMP (abs)", 3), 0x6D: ("ADC abs", 3),
    0x6E: ("ROR abs", 3),
    0x70: ("BVS rel", 2), 0x71: ("ADC (zp),Y", 2), 0x75: ("ADC zp,X", 2),
    0x76: ("ROR zp,X", 2), 0x78: ("SEI", 1), 0x79: ("ADC abs,Y", 3),
    0x7D: ("ADC abs,X", 3), 0x7E: ("ROR abs,X", 3),
    0x81: ("STA (zp,X)", 2), 0x84: ("STY zp", 2), 0x85: ("STA zp", 2),
    0x86: ("STX zp", 2), 0x88: ("DEY", 1), 0x8A: ("TXA", 1),
    0x8C: ("STY abs", 3), 0x8D: ("STA abs", 3), 0x8E: ("STX abs", 3),
    0x90: ("BCC rel", 2), 0x91: ("STA (zp),Y", 2), 0x94: ("STY zp,X", 2),
    0x95: ("STA zp,X", 2), 0x96: ("STX zp,Y", 2), 0x98: ("TYA", 1),
    0x99: ("STA abs,Y", 3), 0x9A: ("TXS", 1), 0x9D: ("STA abs,X", 3),
    0xA0: ("LDY #imm", 2), 0xA1: ("LDA (zp,X)", 2), 0xA2: ("LDX #imm", 2),
    0xA4: ("LDY zp", 2), 0xA5: ("LDA zp", 2), 0xA6: ("LDX zp", 2),
    0xA8: ("TAY", 1), 0xA9: ("LDA #imm", 2), 0xAA: ("TAX", 1),
    0xAC: ("LDY abs", 3), 0xAD: ("LDA abs", 3), 0xAE: ("LDX abs", 3),
    0xB0: ("BCS rel", 2), 0xB1: ("LDA (zp),Y", 2), 0xB4: ("LDY zp,X", 2),
    0xB5: ("LDA zp,X", 2), 0xB6: ("LDX zp,Y", 2), 0xB8: ("CLV", 1),
    0xB9: ("LDA abs,Y", 3), 0xBA: ("TSX", 1), 0xBC: ("LDY abs,X", 3),
    0xBD: ("LDA abs,X", 3), 0xBE: ("LDX abs,Y", 3),
    0xC0: ("CPY #imm", 2), 0xC1: ("CMP (zp,X)", 2), 0xC4: ("CPY zp", 2),
    0xC5: ("CMP zp", 2), 0xC6: ("DEC zp", 2), 0xC8: ("INY", 1),
    0xC9: ("CMP #imm", 2), 0xCA: ("DEX", 1), 0xCC: ("CPY abs", 3),
    0xCD: ("CMP abs", 3), 0xCE: ("DEC abs", 3),
    0xD0: ("BNE rel", 2), 0xD1: ("CMP (zp),Y", 2), 0xD5: ("CMP zp,X", 2),
    0xD6: ("DEC zp,X", 2), 0xD8: ("CLD", 1), 0xD9: ("CMP abs,Y", 3),
    0xDD: ("CMP abs,X", 3), 0xDE: ("DEC abs,X", 3),
    0xE0: ("CPX #imm", 2), 0xE1: ("SBC (zp,X)", 2), 0xE4: ("CPX zp", 2),
    0xE5: ("SBC zp", 2), 0xE6: ("INC zp", 2), 0xE8: ("INX", 1),
    0xE9: ("SBC #imm", 2), 0xEA: ("NOP", 1), 0xEC: ("CPX abs", 3),
    0xED: ("SBC abs", 3), 0xEE: ("INC abs", 3),
    0xF0: ("BEQ rel", 2), 0xF1: ("SBC (zp),Y", 2), 0xF5: ("SBC zp,X", 2),
    0xF6: ("INC zp,X", 2), 0xF8: ("SED", 1), 0xF9: ("SBC abs,Y", 3),
    0xFD: ("SBC abs,X", 3), 0xFE: ("INC abs,X", 3),
}

def disasm(prg, start, count, base=0x8000):
    lines = []
    pc = start
    end = start + count
    while pc < end:
        addr = base + (pc - start)
        op = prg[pc]
        if op in OPS:
            name, n = OPS[op]
        else:
            name, n = "???", 1
        operand = ""
        if n == 2:
            operand = f" ${prg[pc+1]:02X}"
            if name.endswith("rel"):
                rel = prg[pc+1]
                target = (addr + 2 + (rel - 256 if rel >= 0x80 else rel)) & 0xFFFF
                operand = f" ${target:04X}"
            elif "zp" in name:
                operand = f" ${prg[pc+1]:02X}"
        elif n == 3:
            operand = f" ${prg[pc+2]:02X}{prg[pc+1]:02X}"
            if "abs" in name:
                operand = f" ${prg[pc+2]:02X}{prg[pc+1]:02X}"
        bytes_hex = " ".join(f"{b:02X}" for b in prg[pc:pc+n])
        lines.append(f"{addr:04X}: {bytes_hex:9s} {name}{operand}")
        pc += n
    return lines

if __name__ == "__main__":
    path = sys.argv[1]
    prg = load_ines(path)
    start = 0
    count = len(prg)
    if len(sys.argv) > 2:
        start = int(sys.argv[2], 16) - 0x8000
    if len(sys.argv) > 3:
        count = int(sys.argv[3])
    lines = disasm(prg, start, count)
    for l in lines:
        print(l)
