/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2012 CaH4e3
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

static uint8 latch;
static uint8 *WRAM = NULL;
static FceuMallocPtr WRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static uint32 WRAMSIZE;
static writefunc old4016;

static SFORMAT StateRegs[] =
{
	{ &latch, 1, "LATC" },
	{ 0 }
};

static void Sync(void) {
	setchr8((latch >> 2) & 1);
	setprg8r(0x10, 0x6000, 0);
	setprg32(0x8000, 0);
	setprg8(0x8000, latch & 4);        /* Special for VS Gumshoe */
}

static DECLFW(M99Write) {
	latch = V;
	Sync();
	old4016(A, V);
}

static void M99Power(void) {
	latch = 0;
	Sync();
	old4016 = GetWriteHandler(0x4016);
	SetWriteHandler(0x4016, 0x4016, M99Write);
	SetReadHandler(0x6000, 0xFFFF, CartBR);
	SetWriteHandler(0x6000, 0x7FFF, CartBW);
	FCEU_CheatAddRAM(WRAMSIZE >> 10, 0x6000, WRAM);
}

static void M99Close(void)
{
	WRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	WRAM = nullptr;
}

static void StateRestore(int version) {
	Sync();
}

void Mapper99_Init(CartInfo *info) {
	info->Power = M99Power;
	info->Close = M99Close;

	WRAMSIZE = 8192;
	WRAM_owner = FCEU_gmalloc_unique(WRAMSIZE);  // v0.3.6: RAII-wrapped
	WRAM = WRAM_owner.get();
	SetupCartPRGMapping(0x10, WRAM, WRAMSIZE, 1);
	AddExState(WRAM, WRAMSIZE, 0, "WRAM");

	GameStateRestore = StateRestore;
	AddExState(&StateRegs, ~0, 0, 0);
}


// v1.8 Masonry Phase E.2 step 9.3: MapperEntryRegister for mapper 99
// (VS Uni/Dual-system).  Cart subclass inherits MapperStrategyA (16-byte default body).
namespace fceu11 {
namespace {
static MapperEntryRegister kMapper99Register{
    MapperEntry{99, "VS Uni/Dual-system", &Mapper99_Init,
        [](Bus& bus) { return std::make_unique<Mapper99Cart>(bus); } }
};
}  // namespace
}  // namespace fceu11
