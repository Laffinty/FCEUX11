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
 *
 * FDS Conversion
 *
 */

#include "mapinc_bus.h"
#include "boards/smb2j_carts.h"
#include "boards/registry.h"

static uint8 preg, creg, mirr;
static uint32 IRQCount, IRQa;

static SFORMAT StateRegs[] =
{
	{ &preg, 1, "PREG" },
	{ &creg, 1, "CREG" },
	{ &mirr, 1, "MIRR" },
	{ &IRQCount, 4, "IRQC" },
	{ &IRQa, 4, "IRQA" },
	{ 0 }
};

static void Sync(void) {
	setprg8(0x6000, preg);
	setprg32(0x8000, ~0);
	setchr8(creg);
	setmirror(mirr);
}

static DECLFW(M42Write) {
	switch (A & 0xE003) {
	case 0x8000: creg = V; Sync(); break;
	case 0xE000: preg = V & 0x0F; Sync(); break;
	case 0xE001: mirr = ((V >> 3) & 1 ) ^ 1; Sync(); break;
	case 0xE002: IRQa = V & 2; if (!IRQa) IRQCount = 0; X6502_IRQEnd(FCEU_IQEXT); break;
	}
}

static void M42Power(void) {
	preg = 0;
	mirr = 1;   // Ai Senshi Nicol actually has fixed mirroring, but mapper forcing it's default value now
	Sync();
	SetReadHandler(0x6000, 0xffff, CartBR);
	SetWriteHandler(0x6000, 0xffff, M42Write);
}

static void M42IRQHook(int a) {
	if (IRQa) {
		IRQCount += a;
		if (IRQCount >= 32768) IRQCount -= 32768;
		if (IRQCount >= 24576)
			X6502_IRQBegin(FCEU_IQEXT);
		else
			X6502_IRQEnd(FCEU_IQEXT);
	}
}

static void StateRestore(int version) {
	Sync();
}

void Mapper42_Init(CartInfo *info) {
	info->Power = M42Power;
	g_cpu.set_map_irq_hook(M42IRQHook);
	GameStateRestore = StateRestore;
	AddExState(&StateRegs, ~0, 0, 0);
}

namespace fceu11 {
namespace {
static MapperEntryRegister kMapper42Register{
    MapperEntry{42, "SMB2j FDS 42", &Mapper42_Init,
        [](Bus& bus) { return std::make_unique<Mapper42Cart>(bus); }}
};
}  // namespace
}  // namespace fceu11
