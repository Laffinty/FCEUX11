/// \file
/// \brief 6502 assembler and disassembler

#include "types.h"
#include "utils/xstring.h"
#include "debug.h"
#include "asm.h"
#include "x6502.h"
#include "rust/fceux11_rust.h"

#include <cstring>
#include <cstdlib>
#include <cstdio>

///assembles the string to an instruction located at addr, storing opcodes in output buffer
int Assemble(unsigned char *output, int addr, char *str) {
	return fceux11_rust_asm_assemble(output, addr, str);
}

///disassembles the opcodes in the buffer assuming the provided address.
///returns a static string buffer.
char *Disassemble(int addr, uint8 *opcode) {
	static char result[64];
	fceux11_rust_asm_disassemble(addr, opcode, result, 64);
	return result;
}