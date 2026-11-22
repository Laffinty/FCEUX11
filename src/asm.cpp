/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
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