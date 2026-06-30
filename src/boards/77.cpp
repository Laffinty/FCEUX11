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

static uint8 latche;

static uint8 *CHRRAM=NULL;
static FceuMallocPtr CHRRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static uint32 CHRRAMSIZE;

static SFORMAT StateRegs[] =
{
	{ &latche, 1, "LATC" },
	{ 0 }
};

static void Sync(void) {
	setprg32(0x8000, latche & 7);
	setchr2(0x0000, latche >> 4);
	setchr2r(0x10, 0x0800, 2);
	setchr4r(0x10, 0x1000, 0);
}

static DECLFW(M77Write) {
	latche = V;
	Sync();
}

static void M77Power(void) {
	latche = 0;
	Sync();
	SetReadHandler(0x8000, 0xFFFF, CartBR);
	SetWriteHandler(0x8000, 0xFFFF, M77Write);
}

static void M77Close(void)
{
	CHRRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	CHRRAM = nullptr;
}

static void StateRestore(int version) {
	Sync();
}

void Mapper77_Init(CartInfo *info) {
	info->Power = M77Power;
	info->Close = M77Close;
	GameStateRestore = StateRestore;

	CHRRAMSIZE = 6 * 1024;
	CHRRAM_owner = FCEU_gmalloc_unique(CHRRAMSIZE);  // v0.3.6: RAII-wrapped
	CHRRAM = CHRRAM_owner.get();
	SetupCartCHRMapping(0x10, CHRRAM, CHRRAMSIZE, 1);
	AddExState(CHRRAM, CHRRAMSIZE, 0, "CRAM");

	AddExState(&StateRegs, ~0, 0, 0);
}

// v1.8 Masonry Phase E.2 step 9.3: MapperEntryRegister for mapper 77 (IREM LROG017).
namespace fceu11 {
namespace {
static MapperEntryRegister kMapper77Register{
    MapperEntry{77, "IREM LROG017", &Mapper77_Init,
        [](Bus& bus) { return std::make_unique<Mapper77Cart>(bus); }}
};
}  // namespace
}  // namespace fceu11
