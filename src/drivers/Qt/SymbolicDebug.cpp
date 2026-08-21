/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2020 mjbudd77
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
//
// SymbolicDebug.cpp
#include <stdio.h>
#include "utils/safe_string.h"
#include <stdlib.h>
#include <string.h>

#include "../../types.h"
#include "../../fceu.h"
#include "../../debug.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "../../cart.h"
#include "../../ines.h"
#include "../../asm.h"
#include "../../cpu.h"

#include "Qt/fceuWrapper.h"
#include "Qt/SymbolicDebug.h"
#include "Qt/ConsoleUtilities.h"
#include "Qt/ConsoleWindow.h"


//--------------------------------------------------------------
debugSymbol_t *replaceSymbols( int flags, int addr, char *str )
{
	debugSymbol_t *sym;
  
	if ( addr >= 0x8000 )
	{
		int bank = getBank(addr);

  		sym = debugSymbolTable.getSymbolAtBankOffset( bank, addr );
	}
	else
	{
  		sym = debugSymbolTable.getSymbolAtBankOffset( -1, addr );

		if ( (sym == NULL) && (flags & ASM_DEBUG_REGS) )
		{
  			sym = debugSymbolTable.getSymbolAtBankOffset( -2, addr );
		}
	}

	if ( sym )
	{
		if ( flags & ASM_DEBUG_REPLACE )
		{
			FCEU_strlcpy( str, sizeof(str), sym->name().c_str() );
		}
		else
		{
			if ( flags & ASM_DEBUG_ADDR_02X )
			{
				snprintf( str, sizeof(str), "$%02X ", addr );
			}
			else
			{
				snprintf( str, sizeof(str), "$%04X ", addr );
			}
			safe_strcat( str, sizeof(str), sym->name().c_str() );
		}
	}
	else
	{
		if ( flags & ASM_DEBUG_ADDR_02X )
		{
			snprintf( str, sizeof(str), "$%02X", addr );
		}
		else
		{
			snprintf( str, sizeof(str), "$%04X", addr );
		}
	}

	return sym;
}
//--------------------------------------------------------------
int DisassembleWithDebug(int addr, uint8_t *opcode, int flags, char *str, debugSymbol_t *symOut, debugSymbol_t *symOut2 )
{
	debugSymbol_t *sym  = NULL;
	debugSymbol_t *sym2 = NULL;
	static char chr[8]={0};
	uint16_t tmp,tmp2;
	char stmp[128], stmp2[128];
	bool symDebugEnable, showTrace;

	symDebugEnable = (flags & ASM_DEBUG_SYMS  ) ? true : false;
	showTrace      = (flags & ASM_DEBUG_TRACES) ? true : false;

	//these may be replaced later with passed-in values to make a lighter-weight disassembly mode that may not query the referenced values
	#define RX (g_cpu.native_layout().X)
	#define RY (g_cpu.native_layout().Y)

	switch (opcode[0]) 
	{
		#define relative(a) { \
			if (((a)=opcode[1])&0x80) (a) = addr-(((a)-1)^0xFF); \
			else (a)+=addr; \
		}
		#define absolute(a) { \
			(a) = opcode[1] | opcode[2]<<8; \
		}
		#define zpIndex(a,i) { \
			(a) = (opcode[1]+(i))&0xFF; \
		}
		#define indirectX(a) { \
			(a) = (opcode[1]+RX)&0xFF; \
			(a) = GetMem((a)) | (GetMem(((a)+1)&0xff))<<8; \
		}
		#define indirectY(a) { \
			(a) = GetMem(opcode[1]) | (GetMem((opcode[1]+1)&0xff))<<8; \
			(a) += RY; \
		}


		#ifdef BRK_3BYTE_HACK
			case 0x00:
			snprintf( str, sizeof(str),"BRK %02X %02X", opcode[1], opcode[2]);
			break;
		#else
			case 0x00: FCEU_strlcpy( str, sizeof(str),"BRK"); break;
		#endif

		//odd, 1-byte opcodes
		case 0x08: FCEU_strlcpy( str, sizeof(str),"PHP"); break;
		case 0x0A: FCEU_strlcpy( str, sizeof(str),"ASL"); break;
		case 0x18: FCEU_strlcpy( str, sizeof(str),"CLC"); break;
		case 0x28: FCEU_strlcpy( str, sizeof(str),"PLP"); break;
		case 0x2A: FCEU_strlcpy( str, sizeof(str),"ROL"); break;
		case 0x38: FCEU_strlcpy( str, sizeof(str),"SEC"); break;
		case 0x40: FCEU_strlcpy( str, sizeof(str),"RTI"); break;
		case 0x48: FCEU_strlcpy( str, sizeof(str),"PHA"); break;
		case 0x4A: FCEU_strlcpy( str, sizeof(str),"LSR"); break;
		case 0x58: FCEU_strlcpy( str, sizeof(str),"CLI"); break;
		case 0x60: FCEU_strlcpy( str, sizeof(str),"RTS"); break;
		case 0x68: FCEU_strlcpy( str, sizeof(str),"PLA"); break;
		case 0x6A: FCEU_strlcpy( str, sizeof(str),"ROR"); break;
		case 0x78: FCEU_strlcpy( str, sizeof(str),"SEI"); break;
		case 0x88: FCEU_strlcpy( str, sizeof(str),"DEY"); break;
		case 0x8A: FCEU_strlcpy( str, sizeof(str),"TXA"); break;
		case 0x98: FCEU_strlcpy( str, sizeof(str),"TYA"); break;
		case 0x9A: FCEU_strlcpy( str, sizeof(str),"TXS"); break;
		case 0xA8: FCEU_strlcpy( str, sizeof(str),"TAY"); break;
		case 0xAA: FCEU_strlcpy( str, sizeof(str),"TAX"); break;
		case 0xB8: FCEU_strlcpy( str, sizeof(str),"CLV"); break;
		case 0xBA: FCEU_strlcpy( str, sizeof(str),"TSX"); break;
		case 0xC8: FCEU_strlcpy( str, sizeof(str),"INY"); break;
		case 0xCA: FCEU_strlcpy( str, sizeof(str),"DEX"); break;
		case 0xD8: FCEU_strlcpy( str, sizeof(str),"CLD"); break;
		case 0xE8: FCEU_strlcpy( str, sizeof(str),"INX"); break;
		case 0xEA: FCEU_strlcpy( str, sizeof(str),"NOP"); break;
		case 0xF8: FCEU_strlcpy( str, sizeof(str),"SED"); break;

		//(Indirect,X)
		case 0x01: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _indirectx;
		case 0x21: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _indirectx;
		case 0x41: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _indirectx;
		case 0x61: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _indirectx;
		case 0x81: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _indirectx;
		case 0xA1: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _indirectx;
		case 0xC1: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _indirectx;
		case 0xE1: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _indirectx;
		_indirectx:
			indirectX(tmp);

			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				showTrace
					? snprintf( str, sizeof(str),"%s ($%02X,X) @ %s = #$%02X", chr,opcode[1],stmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s ($%02X,X)", chr,opcode[1]);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s ($%02X,X) @ $%04X = #$%02X", chr,opcode[1],tmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s ($%02X,X)", chr,opcode[1]);
			}
			break;

		//Zero Page
		case 0x05: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _zeropage;
		case 0x06: FCEU_strlcpy( chr, sizeof(chr),"ASL"); goto _zeropage;
		case 0x24: FCEU_strlcpy( chr, sizeof(chr),"BIT"); goto _zeropage;
		case 0x25: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _zeropage;
		case 0x26: FCEU_strlcpy( chr, sizeof(chr),"ROL"); goto _zeropage;
		case 0x45: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _zeropage;
		case 0x46: FCEU_strlcpy( chr, sizeof(chr),"LSR"); goto _zeropage;
		case 0x65: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _zeropage;
		case 0x66: FCEU_strlcpy( chr, sizeof(chr),"ROR"); goto _zeropage;
		case 0x84: FCEU_strlcpy( chr, sizeof(chr),"STY"); goto _zeropage;
		case 0x85: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _zeropage;
		case 0x86: FCEU_strlcpy( chr, sizeof(chr),"STX"); goto _zeropage;
		case 0xA4: FCEU_strlcpy( chr, sizeof(chr),"LDY"); goto _zeropage;
		case 0xA5: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _zeropage;
		case 0xA6: FCEU_strlcpy( chr, sizeof(chr),"LDX"); goto _zeropage;
		case 0xC4: FCEU_strlcpy( chr, sizeof(chr),"CPY"); goto _zeropage;
		case 0xC5: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _zeropage;
		case 0xC6: FCEU_strlcpy( chr, sizeof(chr),"DEC"); goto _zeropage;
		case 0xE4: FCEU_strlcpy( chr, sizeof(chr),"CPX"); goto _zeropage;
		case 0xE5: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _zeropage;
		case 0xE6: FCEU_strlcpy( chr, sizeof(chr),"INC"); goto _zeropage;
		_zeropage:
		// ################################## Start of SP CODE ###########################
		// Change width to %04X // don't!
			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags | ASM_DEBUG_ADDR_02X, opcode[1], stmp );
				showTrace
					? snprintf( str, sizeof(str),"%s %s = #$%02X", chr,stmp,GetMem(opcode[1]))
					: snprintf( str, sizeof(str),"%s %s", chr,stmp);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s $%02X = #$%02X", chr,opcode[1],GetMem(opcode[1]))
					: snprintf( str, sizeof(str),"%s $%02X", chr,opcode[1]);
			}
		// ################################## End of SP CODE ###########################
			break;

		//#Immediate
		case 0x09: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _immediate;
		case 0x29: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _immediate;
		case 0x49: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _immediate;
		case 0x69: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _immediate;
		//case 0x89: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _immediate;  //baka, no STA #imm!!
		case 0xA0: FCEU_strlcpy( chr, sizeof(chr),"LDY"); goto _immediate;
		case 0xA2: FCEU_strlcpy( chr, sizeof(chr),"LDX"); goto _immediate;
		case 0xA9: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _immediate;
		case 0xC0: FCEU_strlcpy( chr, sizeof(chr),"CPY"); goto _immediate;
		case 0xC9: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _immediate;
		case 0xE0: FCEU_strlcpy( chr, sizeof(chr),"CPX"); goto _immediate;
		case 0xE9: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _immediate;
		_immediate:
			snprintf( str, sizeof(str),"%s #$%02X", chr,opcode[1]);
			break;

		//Absolute
		case 0x0D: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _absolute;
		case 0x0E: FCEU_strlcpy( chr, sizeof(chr),"ASL"); goto _absolute;
		case 0x2C: FCEU_strlcpy( chr, sizeof(chr),"BIT"); goto _absolute;
		case 0x2D: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _absolute;
		case 0x2E: FCEU_strlcpy( chr, sizeof(chr),"ROL"); goto _absolute;
		case 0x4D: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _absolute;
		case 0x4E: FCEU_strlcpy( chr, sizeof(chr),"LSR"); goto _absolute;
		case 0x6D: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _absolute;
		case 0x6E: FCEU_strlcpy( chr, sizeof(chr),"ROR"); goto _absolute;
		case 0x8C: FCEU_strlcpy( chr, sizeof(chr),"STY"); goto _absolute;
		case 0x8D: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _absolute;
		case 0x8E: FCEU_strlcpy( chr, sizeof(chr),"STX"); goto _absolute;
		case 0xAC: FCEU_strlcpy( chr, sizeof(chr),"LDY"); goto _absolute;
		case 0xAD: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _absolute;
		case 0xAE: FCEU_strlcpy( chr, sizeof(chr),"LDX"); goto _absolute;
		case 0xCC: FCEU_strlcpy( chr, sizeof(chr),"CPY"); goto _absolute;
		case 0xCD: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _absolute;
		case 0xCE: FCEU_strlcpy( chr, sizeof(chr),"DEC"); goto _absolute;
		case 0xEC: FCEU_strlcpy( chr, sizeof(chr),"CPX"); goto _absolute;
		case 0xED: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _absolute;
		case 0xEE: FCEU_strlcpy( chr, sizeof(chr),"INC"); goto _absolute;
		_absolute:
			absolute(tmp);

			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				showTrace
					? snprintf( str, sizeof(str),"%s %s = #$%02X", chr,stmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s %s", chr,stmp);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s $%04X = #$%02X", chr,tmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s $%04X", chr,tmp);
			}
			break;

		//branches
		case 0x10: FCEU_strlcpy( chr, sizeof(chr),"BPL"); goto _branch;
		case 0x30: FCEU_strlcpy( chr, sizeof(chr),"BMI"); goto _branch;
		case 0x50: FCEU_strlcpy( chr, sizeof(chr),"BVC"); goto _branch;
		case 0x70: FCEU_strlcpy( chr, sizeof(chr),"BVS"); goto _branch;
		case 0x90: FCEU_strlcpy( chr, sizeof(chr),"BCC"); goto _branch;
		case 0xB0: FCEU_strlcpy( chr, sizeof(chr),"BCS"); goto _branch;
		case 0xD0: FCEU_strlcpy( chr, sizeof(chr),"BNE"); goto _branch;
		case 0xF0: FCEU_strlcpy( chr, sizeof(chr),"BEQ"); goto _branch;
		_branch:
			relative(tmp);

			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				snprintf( str, sizeof(str),"%s %s", chr,stmp);
			}
			else
			{
				snprintf( str, sizeof(str),"%s $%04X", chr,tmp);
			}
			break;

		//(Indirect),Y
		case 0x11: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _indirecty;
		case 0x31: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _indirecty;
		case 0x51: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _indirecty;
		case 0x71: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _indirecty;
		case 0x91: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _indirecty;
		case 0xB1: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _indirecty;
		case 0xD1: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _indirecty;
		case 0xF1: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _indirecty;
		_indirecty:
			indirectY(tmp);

			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				showTrace
					? snprintf( str, sizeof(str),"%s ($%02X),Y @ %s = #$%02X", chr,opcode[1],stmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s ($%02X),Y", chr,opcode[1]);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s ($%02X),Y @ $%04X = #$%02X", chr,opcode[1],tmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s ($%02X),Y", chr,opcode[1]);
			}
			break;

		//Zero Page,X
		case 0x15: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _zeropagex;
		case 0x16: FCEU_strlcpy( chr, sizeof(chr),"ASL"); goto _zeropagex;
		case 0x35: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _zeropagex;
		case 0x36: FCEU_strlcpy( chr, sizeof(chr),"ROL"); goto _zeropagex;
		case 0x55: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _zeropagex;
		case 0x56: FCEU_strlcpy( chr, sizeof(chr),"LSR"); goto _zeropagex;
		case 0x75: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _zeropagex;
		case 0x76: FCEU_strlcpy( chr, sizeof(chr),"ROR"); goto _zeropagex;
		case 0x94: FCEU_strlcpy( chr, sizeof(chr),"STY"); goto _zeropagex;
		case 0x95: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _zeropagex;
		case 0xB4: FCEU_strlcpy( chr, sizeof(chr),"LDY"); goto _zeropagex;
		case 0xB5: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _zeropagex;
		case 0xD5: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _zeropagex;
		case 0xD6: FCEU_strlcpy( chr, sizeof(chr),"DEC"); goto _zeropagex;
		case 0xF5: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _zeropagex;
		case 0xF6: FCEU_strlcpy( chr, sizeof(chr),"INC"); goto _zeropagex;
		_zeropagex:
			zpIndex(tmp,RX);
		// ################################## Start of SP CODE ###########################
		// Change width to %04X // don't!
			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				showTrace
					? snprintf( str, sizeof(str),"%s $%02X,X @ %s = #$%02X", chr,opcode[1],stmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s $%02X,X", chr,opcode[1]);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s $%02X,X @ $%04X = #$%02X", chr,opcode[1],tmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s $%02X,X", chr,opcode[1]);
			}
		// ################################## End of SP CODE ###########################
			break;

		//Absolute,Y
		case 0x19: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _absolutey;
		case 0x39: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _absolutey;
		case 0x59: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _absolutey;
		case 0x79: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _absolutey;
		case 0x99: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _absolutey;
		case 0xB9: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _absolutey;
		case 0xBE: FCEU_strlcpy( chr, sizeof(chr),"LDX"); goto _absolutey;
		case 0xD9: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _absolutey;
		case 0xF9: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _absolutey;
		_absolutey:
			absolute(tmp);
			tmp2=(tmp+RY);
			if ( symDebugEnable )
			{
				sym  = replaceSymbols( flags, tmp , stmp  );
				sym2 = replaceSymbols( flags, tmp2, stmp2 );
				showTrace
					? snprintf( str, sizeof(str),"%s %s,Y @ %s = #$%02X", chr,stmp,stmp2,GetMem(tmp2))
					: snprintf( str, sizeof(str),"%s %s,Y", chr,stmp);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s $%04X,Y @ $%04X = #$%02X", chr,tmp,tmp2,GetMem(tmp2))
					: snprintf( str, sizeof(str),"%s $%04X,Y", chr,tmp);
			}
			break;

		//Absolute,X
		case 0x1D: FCEU_strlcpy( chr, sizeof(chr),"ORA"); goto _absolutex;
		case 0x1E: FCEU_strlcpy( chr, sizeof(chr),"ASL"); goto _absolutex;
		case 0x3D: FCEU_strlcpy( chr, sizeof(chr),"AND"); goto _absolutex;
		case 0x3E: FCEU_strlcpy( chr, sizeof(chr),"ROL"); goto _absolutex;
		case 0x5D: FCEU_strlcpy( chr, sizeof(chr),"EOR"); goto _absolutex;
		case 0x5E: FCEU_strlcpy( chr, sizeof(chr),"LSR"); goto _absolutex;
		case 0x7D: FCEU_strlcpy( chr, sizeof(chr),"ADC"); goto _absolutex;
		case 0x7E: FCEU_strlcpy( chr, sizeof(chr),"ROR"); goto _absolutex;
		case 0x9D: FCEU_strlcpy( chr, sizeof(chr),"STA"); goto _absolutex;
		case 0xBC: FCEU_strlcpy( chr, sizeof(chr),"LDY"); goto _absolutex;
		case 0xBD: FCEU_strlcpy( chr, sizeof(chr),"LDA"); goto _absolutex;
		case 0xDD: FCEU_strlcpy( chr, sizeof(chr),"CMP"); goto _absolutex;
		case 0xDE: FCEU_strlcpy( chr, sizeof(chr),"DEC"); goto _absolutex;
		case 0xFD: FCEU_strlcpy( chr, sizeof(chr),"SBC"); goto _absolutex;
		case 0xFE: FCEU_strlcpy( chr, sizeof(chr),"INC"); goto _absolutex;
		_absolutex:
			absolute(tmp);
			tmp2=(tmp+RX);
			if ( symDebugEnable )
			{
				sym  = replaceSymbols( flags, tmp , stmp  );
				sym2 = replaceSymbols( flags, tmp2, stmp2 );
				showTrace
					? snprintf( str, sizeof(str),"%s %s,X @ %s = #$%02X", chr,stmp,stmp2,GetMem(tmp2))
					: snprintf( str, sizeof(str),"%s %s,X", chr,stmp);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s $%04X,X @ $%04X = #$%02X", chr,tmp,tmp2,GetMem(tmp2))
					: snprintf( str, sizeof(str),"%s $%04X,X", chr,tmp);
			}
			break;

		//jumps
		case 0x20: FCEU_strlcpy( chr, sizeof(chr),"JSR"); goto _jump;
		case 0x4C: FCEU_strlcpy( chr, sizeof(chr),"JMP"); goto _jump;
		case 0x6C: absolute(tmp); snprintf( str, sizeof(str),"JMP ($%04X) = $%04X", tmp,GetMem(tmp)|GetMem(tmp+1)<<8); break;
		_jump:
			absolute(tmp);

			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				snprintf( str, sizeof(str),"%s %s", chr,stmp);
			}
			else
			{
				snprintf( str, sizeof(str),"%s $%04X", chr,tmp);
			}
			break;

		//Zero Page,Y
		case 0x96: FCEU_strlcpy( chr, sizeof(chr),"STX"); goto _zeropagey;
		case 0xB6: FCEU_strlcpy( chr, sizeof(chr),"LDX"); goto _zeropagey;
		_zeropagey:
			zpIndex(tmp,RY);
		// ################################## Start of SP CODE ###########################
		// Change width to %04X // don't!
			if ( symDebugEnable )
			{
				sym = replaceSymbols( flags, tmp, stmp );
				showTrace
					? snprintf( str, sizeof(str),"%s $%02X,Y @ %s = #$%02X", chr,opcode[1],stmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s $%02X,Y", chr,opcode[1]);
			}
			else
			{
				showTrace
					? snprintf( str, sizeof(str),"%s $%02X,Y @ $%04X = #$%02X", chr,opcode[1],tmp,GetMem(tmp))
					: snprintf( str, sizeof(str),"%s $%02X,Y", chr,opcode[1]);
			}
		// ################################## End of SP CODE ###########################
			break;

		//UNDEFINED
		default: FCEU_strlcpy( str, sizeof(str),"ERROR"); break;

	}

	if ( symOut )
	{
		if ( sym )
		{
			*symOut = *sym;
		}
		else if ( sym2 )
		{
			*symOut = *sym2; sym2 = NULL;
		}
	}
	if ( symOut2 )
	{
		if ( sym2 )
		{
			*symOut2 = *sym2;
		}
	}
	
	return 0;
}
//--------------------------------------------------------------
// Symbol Add/Edit Window Object
//--------------------------------------------------------------
SymbolEditWindow::SymbolEditWindow(QWidget *parent)
	: QDialog(parent)
{
	QHBoxLayout *hbox;
	QVBoxLayout *mainLayout;
	QGridLayout *grid;
	QLabel      *lbl;
	fceuHexIntValidtor *validator;

	font.setFamily("Courier New");
	font.setStyle( QFont::StyleNormal );
	font.setStyleHint( QFont::Monospace );

	QFontMetrics fm(font);

#if QT_VERSION > QT_VERSION_CHECK(5, 11, 0)
    charWidth = fm.horizontalAdvance(QLatin1Char('2'));
#else
    charWidth = fm.width(QLatin1Char('2'));
#endif

	bank = -1;
	addr = -1;
	sym  = NULL;

	setWindowTitle( tr("Symbolic Debug Naming") );

	hbox       = new QHBoxLayout();
	mainLayout = new QVBoxLayout();

	lbl = new QLabel( tr("File") );
	filepath = new QLineEdit();
	filepath->setFont( font );
	filepath->setReadOnly( true );
	filepath->setMinimumWidth( charWidth * (filepath->text().size() + 4) );

	hbox->addWidget( lbl );
	hbox->addWidget( filepath );

	mainLayout->addLayout( hbox );

	hbox = new QHBoxLayout();
	lbl  = new QLabel( tr("Address") );
	addrEntry = new QLineEdit();
	addrEntry->setFont( font );
	addrEntry->setReadOnly( true );
	addrEntry->setAlignment(Qt::AlignCenter);
	addrEntry->setMaximumWidth( charWidth * 6 );

	hbox->addWidget( lbl, 1, Qt::AlignLeft );
	hbox->addWidget( addrEntry, 10, Qt::AlignLeft );

	isArrayBox = new QCheckBox( tr("Array") );
	hbox->addWidget( isArrayBox, 10, Qt::AlignRight );

	validator = new fceuHexIntValidtor( 0x00, 0xFF, this );
	arraySizeLbl[0] = new QLabel( tr("Size: 0x") );
	arraySize = new QLineEdit();
	arraySize->setFont( font );
	arraySize->setText( tr("01") );
	arraySize->setAlignment(Qt::AlignCenter);
	arraySize->setMaximumWidth( charWidth * 4 );
	arraySize->setValidator( validator );

	hbox->addWidget( arraySizeLbl[0], 0, Qt::AlignRight );
	hbox->addWidget( arraySize );

	arraySizeLbl[1] = new QLabel( tr("Bytes") );
	hbox->addWidget( arraySizeLbl[1], 0, Qt::AlignLeft );

	validator = new fceuHexIntValidtor( 0x00, 0xFF, this );
	arrayInitLbl = new QLabel( tr("Init: 0x") );
	arrayInit = new QLineEdit();
	arrayInit->setFont( font );
	arrayInit->setText( tr("00") );
	arrayInit->setAlignment(Qt::AlignCenter);
	arrayInit->setMaximumWidth( charWidth * 4 );
	arrayInit->setValidator( validator );

	hbox->addWidget( arrayInitLbl, 0, Qt::AlignRight );
	hbox->addWidget( arrayInit );

	mainLayout->addLayout( hbox );

	//hbox = new QHBoxLayout();
	grid = new QGridLayout();
	mainLayout->addLayout( grid );
	lbl  = new QLabel( tr("Name") );
	nameEntry = new QLineEdit();

	arrayNameOverWrite = new QCheckBox( tr("Overwrite Names in Array Body") );
	arrayNameOverWrite->setChecked( true );

	grid->addWidget( lbl, 0, 0 );
	grid->addWidget( nameEntry, 0, 1 );

	grid->addWidget( arrayNameOverWrite, 1, 1 );

	lbl  = new QLabel( tr("Comment") );
	commentEntry = new QPlainTextEdit();

	grid->addWidget( lbl, 2, 0 );
	grid->addWidget( commentEntry, 2, 1 );

	commentHeadOnly       = new QCheckBox( tr("Comment Head Address Only") );
	arrayCommentOverWrite = new QCheckBox( tr("Overwrite Comments in Array Body") );

	commentHeadOnly->setChecked(true);

	grid->addWidget( commentHeadOnly, 3, 1 );
	grid->addWidget( arrayCommentOverWrite, 4, 1 );

	hbox         = new QHBoxLayout();
	deleteBox    = new QCheckBox( tr("Delete") );
	okButton     = new QPushButton( tr("OK") );
	cancelButton = new QPushButton( tr("Cancel") );

	//okButton->setIcon( style()->standardIcon( QStyle::SP_DialogApplyButton ) );
	okButton->setIcon( style()->standardIcon( QStyle::SP_DialogOkButton ) );
	cancelButton->setIcon( style()->standardIcon( QStyle::SP_DialogCancelButton ) );

	mainLayout->addLayout( hbox );
	hbox->addWidget( deleteBox    );
	hbox->addWidget( cancelButton );
	hbox->addWidget(     okButton );

	connect(     okButton, SIGNAL(clicked(void)), this, SLOT(accept(void)) );
	connect( cancelButton, SIGNAL(clicked(void)), this, SLOT(reject(void)) );

	deleteBox->setEnabled( false );
	okButton->setDefault(true);

	if ( sym != NULL )
	{
		nameEntry->setText( tr(sym->name().c_str()) );
		commentEntry->setPlainText( tr(sym->comment().c_str()) );
	}

	setLayout( mainLayout );

	updateArraySensitivity();

	connect( isArrayBox     , SIGNAL(stateChanged(int)), this, SLOT(isArrayStateChanged(int)) );
	connect( commentHeadOnly, SIGNAL(stateChanged(int)), this, SLOT(arrayCommentHeadOnlyChanged(int)) );
}
//--------------------------------------------------------------
SymbolEditWindow::~SymbolEditWindow(void)
{

}
//--------------------------------------------------------------
void SymbolEditWindow::closeEvent(QCloseEvent *event)
{
	//printf("Symbolic Debug Close Window Event\n");
	done(0);
	deleteLater();
	event->accept();
}
//--------------------------------------------------------------
void SymbolEditWindow::retranslateUi(void)
{
	setWindowTitle(tr("Symbolic Debug Naming"));
	if (isArrayBox) isArrayBox->setText(tr("Is Array"));
	if (arrayNameOverWrite) arrayNameOverWrite->setText(tr("Overwrite Name"));
	if (arrayCommentOverWrite) arrayCommentOverWrite->setText(tr("Overwrite Comment"));
	if (commentHeadOnly) commentHeadOnly->setText(tr("Comment Head Only"));
	if (deleteBox) deleteBox->setText(tr("Delete"));
	if (okButton) okButton->setText(tr("OK"));
	if (cancelButton) cancelButton->setText(tr("Cancel"));
}
//--------------------------------------------------------------
void SymbolEditWindow::changeEvent(QEvent *event)
{
	if (event->type() == QEvent::LanguageChange)
	{
		retranslateUi();
	}
	QDialog::changeEvent(event);
}
//--------------------------------------------------------------
void SymbolEditWindow::closeWindow(void)
{
	//printf("Close Window\n");
	done(0);
	deleteLater();
}
//--------------------------------------------------------------
void SymbolEditWindow::updateArraySensitivity(void)
{
	if ( isArrayBox->isChecked() )
	{
		arraySize->setEnabled( true );
		arrayInit->setEnabled( true );
		arraySizeLbl[0]->setEnabled( true );
		arraySizeLbl[1]->setEnabled( true );
		arrayInitLbl->setEnabled( true );
		commentHeadOnly->setEnabled( true );
		arrayNameOverWrite->setEnabled( true );

		arrayCommentOverWrite->setEnabled( !commentHeadOnly->isChecked() );
	}
	else
	{
		arraySize->setEnabled( false );
		arrayInit->setEnabled( false );
		arraySizeLbl[0]->setEnabled( false );
		arraySizeLbl[1]->setEnabled( false );
		arrayInitLbl->setEnabled( false );
		commentHeadOnly->setEnabled( false );
		arrayNameOverWrite->setEnabled( false );
		arrayCommentOverWrite->setEnabled( false );
	}

}
//--------------------------------------------------------------
void SymbolEditWindow::isArrayStateChanged( int state )
{
	updateArraySensitivity();
}
//--------------------------------------------------------------
void SymbolEditWindow::arrayCommentHeadOnlyChanged( int state )
{
	updateArraySensitivity();
}
//--------------------------------------------------------------
void SymbolEditWindow::setAddr( int addrIn )
{
	char stmp[64];
	std::string filename;
	size_t size;

	addr = addrIn;

	snprintf( stmp, sizeof(stmp), "%04X", addr );

	addrEntry->setText( tr(stmp) );

	if ( bank < 0 )
	{
		if ( addr < 0x8000 )
		{
		   bank = -1;
		}
		else
		{
		   bank = getBank( addr );
		}
	}

	generateNLFilenameForAddress( addr, filename );

	filepath->setText( tr(filename.c_str()) );

	size = filepath->text().size();

	// Limit max size so that widget size doesn't explode on a large file path.
	if (size > 32) size = 32;

	filepath->setMinimumWidth( charWidth * (size + 4) );
}
//--------------------------------------------------------------
void SymbolEditWindow::setBank( int bankIn )
{
	bank = bankIn;
}
//--------------------------------------------------------------
void SymbolEditWindow::setSym( debugSymbol_t *symIn )
{
	sym = symIn;

	if ( sym != NULL )
	{
		nameEntry->setText( tr(sym->name().c_str()) );
		commentEntry->setPlainText( tr(sym->comment().c_str()) );
		deleteBox->setEnabled( true );

		determineArrayStart();
	}
	else
	{
		nameEntry->clear();
		commentEntry->clear();
		deleteBox->setEnabled( false );

		arrayInit->setText( tr("00") );
	}
}
//--------------------------------------------------------------
int SymbolEditWindow::exec(void)
{
	int ret, b, a, size, init, i;
	bool isNew = 0;

	ret = QDialog::exec();

	if ( ret == QDialog::Accepted )
	{
		FCEU_WRAPPER_LOCK();
		if ( isArrayBox->isChecked() )
		{
			size = atoi( arraySize->text().toStdString().c_str() );
			init = atoi( arrayInit->text().toStdString().c_str() );

			for (i=0; i<size; i++)
			{
				isNew = false;

				a = addr + i;

				if ( a < 0x8000 )
				{
				   b = -1;
				}
				else
				{
				   b = getBank( a );
				}
				
				sym = debugSymbolTable.getSymbolAtBankOffset( b, a );

				if ( deleteBox->isChecked() )
				{
					if ( sym != nullptr )
					{
						debugSymbolTable.deleteSymbolAtBankOffset( b, a );
					}
				}
				else
				{
					if ( sym == nullptr )
					{
						sym = new debugSymbol_t(a);

						if ( debugSymbolTable.addSymbolAtBankOffset( b, a, sym ) )
						{
							if (consoleWindow)
							{
								consoleWindow->QueueErrorMsgWindow( debugSymbolTable.errorMessage() );
							}
							delete sym;
						}
						isNew = true;
					}
					sym->setOffset(a);

					if ( (i == 0) || isNew || arrayNameOverWrite->isChecked() )
					{
						setSymNameWithArray( init + i );
					}
					if ( (i == 0) || !commentHeadOnly->isChecked() )
					{
						if ( isNew || arrayCommentOverWrite->isChecked() || (i == 0) )
						{
							sym->commentAssign( commentEntry->toPlainText().toStdString() );
						}
					}
					sym->trimTrailingSpaces();
				}
			}
		}
		else
		{
			if ( deleteBox->isChecked() )
			{
				if ( sym != NULL )
				{
					debugSymbolTable.deleteSymbolAtBankOffset( bank, addr );
				}
			}
			else if ( sym == NULL )
			{
				sym = new debugSymbol_t( addr, nameEntry->text().toStdString().c_str(), 
						commentEntry->toPlainText().toStdString().c_str());

				if ( debugSymbolTable.addSymbolAtBankOffset( bank, addr, sym ) )
				{
					if (consoleWindow)
					{
						consoleWindow->QueueErrorMsgWindow( debugSymbolTable.errorMessage() );
					}
					delete sym;
				}
			}
			else
			{
				if ( sym->updateName( nameEntry->text().toStdString().c_str() ) )
				{
					if (consoleWindow)
					{
						consoleWindow->QueueErrorMsgWindow( debugSymbolTable.errorMessage() );
					}
				}
				sym->commentAssign( commentEntry->toPlainText().toStdString().c_str() );
				sym->trimTrailingSpaces();
			}
		}
		debugSymbolTable.save(); // Save table to disk immediately after an add, edit, or delete
		FCEU_WRAPPER_UNLOCK();
	}
	return ret;
}
//--------------------------------------------------------------
void SymbolEditWindow::determineArrayStart(void)
{
	int i,j;
	char stmp[512];
	char digits[128];

	if ( sym == NULL )
	{
		return;
	}
	FCEU_strlcpy( stmp, sizeof(stmp), nameEntry->text().toStdString().c_str() );

	// Find Current Array Braces
	i=0;
	while ( stmp[i] != 0 )
	{
		if ( stmp[i] == '[' )
		{
			break;
		}
		i++;
	}

	if ( stmp[i] != '[' )
	{
		return;
	}
	i++;

	while ( isspace( stmp[i] ) ) i++;

	j=0;
	while ( isdigit( stmp[i] ) )
	{
		digits[j] = stmp[i]; i++; j++;
	}
	digits[j] = 0;

	while ( isspace( stmp[i] ) ) i++;

	if ( stmp[i] != ']' )
	{
		return;
	}

	if ( j > 0 )
	{
		int val;

		val = atoi( digits );

		if ( (val >= 0) && (val < 256) )
		{
			snprintf(digits, sizeof(digits), "%02X", val );

			arrayInit->setText( tr(digits) );
		}
	}

	return;
}
//--------------------------------------------------------------
void SymbolEditWindow::setSymNameWithArray(int idx)
{
	int i;
	char stmp[512];

	if ( sym == NULL )
	{
		return;
	}
	FCEU_strlcpy( stmp, sizeof(stmp), nameEntry->text().toStdString().c_str() );

	// Remove Current Array Braces
	i=0;
	while ( stmp[i] != 0 )
	{
		if ( stmp[i] == '[' )
		{
			stmp[i] = 0; break;
		}
		i++;
	}
	i--;

	// Remove Trailing Spaces
	while ( i >= 0 )
	{
		if ( isspace( stmp[i] ) )
		{
			stmp[i] = 0;
		}
		else
		{
			break;
		}
		i--;
	}

	// Reform with base string and new index.
	if ( sym->updateName( stmp, idx ) )
	{
		if (consoleWindow)
		{
			consoleWindow->QueueErrorMsgWindow( debugSymbolTable.errorMessage() );
		}
	}
}
//--------------------------------------------------------------
