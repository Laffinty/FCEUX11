/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2012 CaH4e3
 *  Copyright (C) 2002 Xodnizel
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

#include "mapinc_bus.h"
#include "simple_carts.h"          // v1.8 Phase E.2 step 9.3

static uint8 creg, preg;
static SFORMAT StateRegs[] =
{
	{ &creg, 1, "CREG" },
	{ &preg, 1, "PREG" },
	{ 0 }
};

static void Sync(void) {
	setprg32(0x8000, preg);
	setchr8(creg);
}

static DECLFW(M79Write) {
	if ((A < 0x8000) && ((A ^ 0x4100) == 0)) {
		preg = (V >> 3) & 1;
	}
	creg = V & 7;
	Sync();
}

static void M79Power(void) {
	preg = ~0;
	Sync();
	SetWriteHandler(0x4100, 0x5FFF, M79Write);
	SetWriteHandler(0x8000, 0xFFFF, M79Write);
	SetReadHandler(0x8000, 0xFFFF, CartBR);
}

static void StateRestore(int version) {
	Sync();
}

void Mapper79_Init(CartInfo *info) {
	info->Power = M79Power;
	AddExState(&StateRegs, ~0, 0, 0);
	GameStateRestore = StateRestore;
}


// v1.8 Masonry Phase E.2 step 9.3: MapperEntryRegister for mapper 79
// (AVE/C&E/TXC BOARD).  Cart subclass inherits MapperStrategyA (16-byte default body).
namespace fceu11 {
namespace {
static MapperEntryRegister kMapper79Register{
    MapperEntry{79, "AVE/C&E/TXC BOARD", &Mapper79_Init,
        [](Bus& bus) { return std::make_unique<Mapper79Cart>(bus); } }
};
}  // namespace
}  // namespace fceu11
