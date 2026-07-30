"""Minimal 6502 disassembler for blargg vbl_05_nmi_timing.nes.

iNES: mapper 1 (MMC1), 32KB PRG, 8KB CHR.
Vectors: NMI=$E308, RESET=$E8E7, IRQ/BRK=$E681.
Bank 0 = $8000-$BFFF; fixed last bank = $C000-$FFFF.
"""
import struct, sys

NES_PATH = 'tests/fixtures/blargg/ppu/vbl_05_nmi_timing.nes'
PRG_OFF = 16
PRG_SIZE = 32768
data = open(NES_PATH, 'rb').read()

def prg_to_pc(addr):
    if 0x8000 <= addr < 0xC000:
        return PRG_OFF + (addr - 0x8000)
    elif 0xC000 <= addr < 0x10000:
        return PRG_OFF + 16384 + (addr - 0xC000)
    return None

OPS = {
0x00:('BRK','impl',2),0x01:('ORA','indx',2),0x05:('ORA','zp',2),0x06:('ASL','zp',2),
0x08:('PHP','impl',1),0x09:('ORA','imm',2),0x0A:('ASL','acc',1),0x0D:('ORA','abs',3),
0x0E:('ASL','abs',3),0x10:('BPL','rel',2),0x11:('ORA','indy',2),0x15:('ORA','zpx',2),
0x16:('ASL','zpx',2),0x18:('CLC','impl',1),0x19:('ORA','absy',3),0x1D:('ORA','absx',3),
0x1E:('ASL','absx',3),0x20:('JSR','abs',3),0x21:('AND','indx',2),0x24:('BIT','zp',2),
0x25:('AND','zp',2),0x26:('ROL','zp',2),0x28:('PLP','impl',1),0x29:('AND','imm',2),
0x2A:('ROL','acc',1),0x2C:('BIT','abs',3),0x2D:('AND','abs',3),0x2E:('ROL','abs',3),
0x30:('BMI','rel',2),0x31:('AND','indy',2),0x35:('AND','zpx',2),0x36:('ROL','zpx',2),
0x38:('SEC','impl',1),0x39:('AND','absy',3),0x3D:('AND','absx',3),0x3E:('ROL','absx',3),
0x40:('RTI','impl',1),0x41:('EOR','indx',2),0x45:('EOR','zp',2),0x46:('LSR','zp',2),
0x48:('PHA','impl',1),0x49:('EOR','imm',2),0x4A:('LSR','acc',1),0x4C:('JMP','abs',3),
0x4D:('EOR','abs',3),0x4E:('LSR','abs',3),0x50:('BVC','rel',2),0x51:('EOR','indy',2),
0x55:('EOR','zpx',2),0x56:('LSR','zpx',2),0x58:('CLI','impl',1),0x59:('EOR','absy',3),
0x5D:('EOR','absx',3),0x5E:('LSR','absx',3),0x60:('RTS','impl',1),0x61:('ADC','indx',2),
0x65:('ADC','zp',2),0x66:('ROR','zp',2),0x68:('PLA','impl',1),0x69:('ADC','imm',2),
0x6A:('ROR','acc',1),0x6C:('JMP','ind',3),0x6D:('ADC','abs',3),0x6E:('ROR','abs',3),
0x70:('BVS','rel',2),0x71:('ADC','indy',2),0x75:('ADC','zpx',2),0x76:('ROR','zpx',2),
0x78:('SEI','impl',1),0x79:('ADC','absy',3),0x7D:('ADC','absx',3),0x7E:('ROR','absx',3),
0x81:('STA','indx',2),0x84:('STY','zp',2),0x85:('STA','zp',2),0x86:('STX','zp',2),
0x88:('DEY','impl',1),0x8A:('TXA','impl',1),0x8C:('STY','abs',3),0x8D:('STA','abs',3),
0x8E:('STX','abs',3),0x90:('BCC','rel',2),0x91:('STA','indy',2),0x94:('STY','zpx',2),
0x95:('STA','zpx',2),0x96:('STX','zpy',2),0x98:('TYA','impl',1),0x99:('STA','absy',3),
0x9A:('TXS','impl',1),0x9D:('STA','absx',3),0xA0:('LDY','imm',2),0xA1:('LDA','indx',2),
0xA2:('LDX','imm',2),0xA4:('LDY','zp',2),0xA5:('LDA','zp',2),0xA6:('LDX','zp',2),
0xA8:('TAY','impl',1),0xA9:('LDA','imm',2),0xAA:('TAX','impl',1),0xAC:('LDY','abs',3),
0xAD:('LDA','abs',3),0xAE:('LDX','abs',3),0xB0:('BCS','rel',2),0xB1:('LDA','indy',2),
0xB4:('LDY','zpx',2),0xB5:('LDA','zpx',2),0xB6:('LDX','zpy',2),0xB8:('CLV','impl',1),
0xB9:('LDA','absy',3),0xBA:('TSX','impl',1),0xBC:('LDY','absx',3),0xBD:('LDA','absx',3),
0xBE:('LDX','absy',3),0xC0:('CPY','imm',2),0xC1:('CMP','indx',2),0xC4:('CPY','zp',2),
0xC5:('CMP','zp',2),0xC6:('DEC','zp',2),0xC8:('INY','impl',1),0xC9:('CMP','imm',2),
0xCA:('DEX','impl',1),0xCC:('CPY','abs',3),0xCD:('CMP','abs',3),0xCE:('DEC','abs',3),
0xD0:('BNE','rel',2),0xD1:('CMP','indy',2),0xD5:('CMP','zpx',2),0xD6:('DEC','zpx',2),
0xD8:('CLD','impl',1),0xD9:('CMP','absy',3),0xDD:('CMP','absx',3),0xDE:('DEC','absx',3),
0xE0:('CPX','imm',2),0xE1:('SBC','indx',2),0xE4:('CPX','zp',2),0xE5:('SBC','zp',2),
0xE6:('INC','zp',2),0xE8:('INX','impl',1),0xE9:('SBC','imm',2),0xEA:('NOP','impl',1),
0xEC:('CPX','abs',3),0xED:('SBC','abs',3),0xEE:('INC','abs',3),0xF0:('BEQ','rel',2),
0xF1:('SBC','indy',2),0xF5:('SBC','zpx',2),0xF6:('INC','zpx',2),0xF8:('SED','impl',1),
0xF9:('SBC','absy',3),0xFD:('SBC','absx',3),0xFE:('INC','absx',3),
}

def disasm_one(pc_addr):
    off = prg_to_pc(pc_addr)
    if off is None or off >= PRG_OFF + PRG_SIZE:
        return None
    op = data[off]
    if op not in OPS:
        return (f'{pc_addr:04X}: DB ${op:02X}', 1)
    mnem, mode, sz = OPS[op]
    args = []
    if mode == 'acc':
        args = ['A']
    elif mode == 'impl':
        args = []
    elif mode == 'imm':
        args = [f'#${data[off+1]:02X}']
    elif mode == 'zp':
        args = [f'${data[off+1]:02X}']
    elif mode == 'zpx':
        args = [f'${data[off+1]:02X},X']
    elif mode == 'zpy':
        args = [f'${data[off+1]:02X},Y']
    elif mode == 'abs':
        args = [f'${data[off+2]<<8 | data[off+1]:04X}']
    elif mode == 'absx':
        args = [f'${data[off+2]<<8 | data[off+1]:04X},X']
    elif mode == 'absy':
        args = [f'${data[off+2]<<8 | data[off+1]:04X},Y']
    elif mode == 'ind':
        args = [f'(${data[off+2]<<8 | data[off+1]:04X})']
    elif mode == 'indx':
        args = [f'(${data[off+1]:02X},X)']
    elif mode == 'indy':
        args = [f'(${data[off+1]:02X}),Y']
    elif mode == 'rel':
        rel = data[off+1]
        if rel >= 0x80: rel -= 0x100
        tgt = pc_addr + 2 + rel
        args = [f'${tgt:04X}']
    s = f'{pc_addr:04X}: {mnem}'
    if args:
        s += ' ' + ', '.join(args)
    return (s, sz)

def disasm_range(addr, count):
    lines = []
    for _ in range(count):
        r = disasm_one(addr)
        if r is None: break
        lines.append(r[0])
        addr += r[1]
    return lines

print('=== NMI @ $E308 ===')
for l in disasm_range(0xE308, 60): print(l)

print()
print('=== RESET @ $E8E7 ===')
for l in disasm_range(0xE8E7, 60): print(l)

print()
print('=== IRQ @ $E681 ===')
for l in disasm_range(0xE681, 50): print(l)

print()
print('=== MMC1 init region $E000-$E0FF (likely) ===')
for l in disasm_range(0xE000, 64): print(l)

print()
print('=== $8000..$8040 (bank 0) ===')
for l in disasm_range(0x8000, 32): print(l)