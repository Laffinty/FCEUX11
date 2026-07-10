/* FCEUXD SP - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2005 Sebastian Porst
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

#ifndef CONDDEBUG_H
#define CONDDEBUG_H

// v1.13 Purify H: #define → constexpr
inline constexpr int TYPE_NO         = 0;
inline constexpr int TYPE_REG        = 1;
inline constexpr int TYPE_FLAG       = 2;
inline constexpr int TYPE_NUM        = 3;
inline constexpr int TYPE_ADDR       = 4;
inline constexpr int TYPE_PC_BANK    = 5;
inline constexpr int TYPE_DATA_BANK  = 6;
inline constexpr int TYPE_VALUE_READ = 7;
inline constexpr int TYPE_VALUE_WRITE = 8;

inline constexpr int OP_NO    = 0;
inline constexpr int OP_EQ    = 1;
inline constexpr int OP_NE    = 2;
inline constexpr int OP_GE    = 3;
inline constexpr int OP_LE    = 4;
inline constexpr int OP_G     = 5;
inline constexpr int OP_L     = 6;
inline constexpr int OP_PLUS  = 7;
inline constexpr int OP_MINUS = 8;
inline constexpr int OP_MULT  = 9;
inline constexpr int OP_DIV   = 10;
inline constexpr int OP_OR    = 11;
inline constexpr int OP_AND   = 12;

extern uint16 debugLastAddress;
extern uint8 debugLastOpcode;

//mbg merge 7/18/06 turned into sane c++
struct Condition
{
	Condition* lhs;
	Condition* rhs;

	unsigned int type1;
	unsigned int value1;

	unsigned int op;

	unsigned int type2;
	unsigned int value2;

	Condition(void)
	{
		op = 0;
		lhs = rhs = nullptr;
		type1 = value1 = 0;
		type2 = value2 = 0;
	};

	~Condition(void)
	{
		if (lhs)
		{
			delete lhs;
		}
		if (rhs)
		{
			delete rhs;
		}
	}
};

Condition* generateCondition(const char* str);

#endif
