#!/usr/bin/env python3
data = open("tests/fixtures/blargg/cpu/instr_v5_all.nes", "rb").read()
prg = 16384
def atb(bank, pc):
    return data[16 + bank * prg + (pc - 0xC000)]
print("bank 15 $EA82-$EA86:", [f"{atb(15, 0xEA82+i):02X}" for i in range(5)])
print("bank 15 $EA7B-$EA83:", [f"{atb(15, 0xEA7B+i):02X}" for i in range(9)])
print("bank 14 $EA82-$EA86:", [f"{atb(14, 0xEA82+i):02X}" for i in range(5)])
print("bank 0  $EA82-$EA86:", [f"{atb(0, 0xEA82+i):02X}" for i in range(5)])
