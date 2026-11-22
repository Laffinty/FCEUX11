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
#ifndef _guid_h_
#define _guid_h_

#include <string>
#include "../types.h"
#include "valuearray.h"

struct FCEU_Guid : public ValueArray<uint8,16>
{
	void newGuid();
	std::string toString();
	static FCEU_Guid fromString(std::string str);
	void scan(std::string& str);
};

// R2.1 (docs/internal/refactor_plan_R1_R5_archive.md §Phase R2): layout-assert that the const-correctness
// refactor of ValueArray did not change the byte layout. SFORMAT
// savestate serialization in src/state.cpp writes FCEU_Guid as a raw
// 16-byte blob, so any size change here is a savestate-compat break.
// Note: ValueArray<T,N> is `T data[N]`, so sizeof(ValueArray<uint8,16>)
// is exactly 16; this assert is paranoid but cheap.
static_assert(sizeof(FCEU_Guid) == 16,
              "FCEU_Guid layout changed: 16-byte SFORMAT serialization in "
              "src/state.cpp will break. Check valuearray.h refactor.");


#endif
