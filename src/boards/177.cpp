/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2007 CaH4e3
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
#include "simple_carts.h"          // v1.8 Phase F

static uint8 reg;

static uint8 *WRAM = NULL;
static FceuMallocPtr WRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static uint32 WRAMSIZE=0;

static SFORMAT StateRegs[] =
{
	{ &reg, 1, "REG" },
	{ 0 }
};

static void Sync(void) {
	setchr8(0);
	setprg8r(0x10, 0x6000, 0);
	setprg32(0x8000, reg & 0x1f);
	setmirror(((reg & 0x20) >> 5) ^ 1);
}

static DECLFW(M177Write) {
	reg = V;
	Sync();
}

static void M177Power(void) {
	reg = 0;
	Sync();
	SetReadHandler(0x6000, 0x7fff, CartBR);
	SetWriteHandler(0x6000, 0x7fff, CartBW);
	SetReadHandler(0x8000, 0xFFFF, CartBR);
	SetWriteHandler(0x8000, 0xFFFF, M177Write);
	FCEU_CheatAddRAM(WRAMSIZE >> 10, 0x6000, WRAM);
}

static void M177Close(void) {
	WRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	WRAM = nullptr;
}

static void StateRestore(int version) {
	Sync();
}

void Mapper177_Init(CartInfo *info) {
	info->Power = M177Power;
	info->Close = M177Close;
	GameStateRestore = StateRestore;

	WRAMSIZE = 8192;
	WRAM_owner = FCEU_gmalloc_unique(WRAMSIZE);  // v0.3.6: RAII-wrapped
	WRAM = WRAM_owner.get();
	SetupCartPRGMapping(0x10, WRAM, WRAMSIZE, 1);
	AddExState(WRAM, WRAMSIZE, 0, "WRAM");
	if (info->battery) {
		info->addSaveGameBuf( WRAM, WRAMSIZE );
	}

	AddExState(&StateRegs, ~0, 0, 0);
}

// v1.8 Phase F: MapperEntryRegister.
namespace fceu11 {
namespace {
static MapperEntryRegister kMapper177Register{
    MapperEntry{177, "", &Mapper177_Init,
        [](Bus& bus) { return std::make_unique<Mapper177Cart>(bus); }}
};
}  // namespace
}  // namespace fceu11
