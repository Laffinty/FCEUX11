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

#include "types.h"
#include "conddebug.h"
#include "utils/memory.h"
#include "rust/fceux11_rust.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <cassert>
#include <cctype>

uint16 debugLastAddress = 0; // used by 'T' and 'R' conditions
uint8 debugLastOpcode = 0; // used to evaluate 'W' condition

/* Root of the parser generator — calls Rust implementation */
Condition* generateCondition(const char* str)
{
    return static_cast<Condition*>(fceux11_rust_conddebug_generate_condition(str));
}

/* Free a Condition AST allocated by Rust */
void deleteCondition(Condition* c)
{
    fceux11_rust_conddebug_condition_destroy(static_cast<void*>(c));
}