/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 2005-2019 CaH4e3
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
 * VRC-V (CAI Shogakko no Sansu)
 *
 */

#include "mapinc.h"

//#define CAI_DEBUG

// main tiles RAM is 8K in size, but unless other non-CHR ROM type carts,
// this one accesses the $0000 and $1000 pages based on extra NT RAM on board
// which is similar to MMC5 but much simpler because there are no additional
// bankings here.
// extra NT RAM handling is in PPU code now.

static uint16 CHRSIZE = 8192;
// there are two separate WRAMs 8K each, on main system cartridge (not battery
// backed), and one on the daughter cart (with battery). both are accessed
// via the same registers with additional selector flags.
static uint16 WRAMSIZE = 8192 + 8192;
static uint8 *CHRRAM = NULL;
static FceuMallocPtr CHRRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static uint8 *WRAM = NULL;
static FceuMallocPtr WRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction

static uint8 IRQa, K4IRQ;
static uint32 IRQLatch, IRQCount;

// some kind of 16-bit text  encoding (actually 14-bit) used in game resources
// may be converted by the hardware into the tile indexes for internal CHR ROM
// not sure whey they made it hardware, because most of calculations are just
// bit shifting. the main purpose of this table is to calculate actual CHR ROM
// bank for every character. there is a some kind of regularity, so this table
// may be calculated in software easily.

// table read out from hardware registers as is

///*
static uint8 conv_tbl[4][8] = {
	{ 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 },
	{ 0x00, 0x00, 0x40, 0x10, 0x28, 0x00, 0x18, 0x30 },
	{ 0x00, 0x00, 0x48, 0x18, 0x30, 0x08, 0x20, 0x38 },
	{ 0x00, 0x00, 0x80, 0x20, 0x38, 0x10, 0x28, 0xB0 }
};

static uint8 regs[16];
static SFORMAT StateRegs[] =
{
	{ &IRQCount, 1, "IRQC" },
	{ &IRQLatch, 1, "IRQL" },
	{ &IRQa, 1, "IRQA" },
	{ &K4IRQ, 1, "KIRQ" },
	{ regs, 16, "REGS" },
	{ 0 }
};

static void chrSync(void) {
	setchr4r(0x10, 0x0000, regs[5] & 1);
	// 30.06.19 CaH4e3 there is much more complicated behaviour with second banking register, you may actually
	// view the content of the internal character CHR rom via this window, but it is useless because hardware
	// does not use this area to access the internal ROM. not sure why they did this, but I see no need to
	// emulate this behaviour carefully, unless I find something that I missed...
	setchr4r(0x10, 0x1000, 1);
}

static void Sync(void) {
	chrSync();
	setprg4r(0x10, 0x6000, (regs[0] & 1) | (regs[0] >> 2));	// two 4K banks are identical, either internal or excernal
	setprg4r(0x10, 0x7000, (regs[1] & 1) | (regs[1] >> 2)); // SRAMs may be mapped in any bank independently
	if (PRGptr[1] == NULL) {	// for iNES 2.0 version it even more hacky lol
		setprg8(0x8000, (regs[2] & 0x3F) + ((regs[2] & 0x40) >> 2));
		setprg8(0xA000, (regs[3] & 0x3F) + ((regs[3] & 0x40) >> 2));
		setprg8(0xC000, (regs[4] & 0x3F) + ((regs[4] & 0x40) >> 2));
		setprg8(0xE000, 0x10 + 0x3F);
	} else {
		setprg8r((regs[2] >> 6) & 1, 0x8000, (regs[2] & 0x3F));
		setprg8r((regs[3] >> 6) & 1, 0xA000, (regs[3] & 0x3F));
		setprg8r((regs[4] >> 6) & 1, 0xC000, (regs[4] & 0x3F));
		setprg8r(1, 0xE000, ~0);	// always sees the last bank of the external cart, so can't be booted without it.
	}
	setmirror(((regs[0xA]&2)>>1)^1);
}

static DECLFW(QTAiWrite) {
	regs[(A & 0x0F00) >> 8] = V;	// IRQ pretty the same as in other VRC mappers by Konami
	switch (A) {
	case 0xd600: IRQLatch &= 0xFF00; IRQLatch |= V; break;
	case 0xd700: IRQLatch &= 0x00FF; IRQLatch |= V << 8; break;
	case 0xd900: IRQCount = IRQLatch; IRQa = V & 2; K4IRQ = V & 1; X6502_IRQEnd(FCEU_IQEXT); break;
	case 0xd800: IRQa = K4IRQ; X6502_IRQEnd(FCEU_IQEXT); break;
	case 0xda00: qtaintramreg = regs[0xA] & 3;	break; // register shadow to share it with ppu
	}
	Sync();
}

static DECLFR(QTAiRead) {

//	uint8 res1 = conv_tbl[(regs[0xD] & 0x7F) >> 1][(regs[0xC] >> 5) & 3];
//	uint8 res2 = ((regs[0xD] & 1) << 7) | ((regs[0xC] & 0x1F) << 2) | (regs[0xB] & 3);

	uint8 tabl = conv_tbl[(regs[0xC] >> 5) & 3][(regs[0xD] & 0x7F) >> 4];
	uint8 res1 = 0x40 | (tabl & 0x3F) | ((regs[0xD] >> 1) & 7) | ((regs[0xB] & 4) << 5);
	uint8 res2 = ((regs[0xD] & 1) << 7) | ((regs[0xC] & 0x1F) << 2) | (regs[0xB] & 3);
	
	if (tabl & 0x40)
		res1 &= 0xFB;
	else if (tabl & 0x80)
		res1 |= 0x04;

	if (A == 0xDD00) {
		return res1;
	} else if (A == 0xDC00) {
#ifdef CAI_DEBUG
		FCEU_printf("%02x:%02x+%d -> %02x:%02x\n", regs[0xD], regs[0xC], regs[0xB], res1, res2);
#endif
		return res2;
	} else
		return 0;
}

static void VRC5IRQ(int a) {
	if (IRQa) {
		IRQCount += a;
		if (IRQCount & 0x10000) {
			X6502_IRQBegin(FCEU_IQEXT);
			IRQCount = IRQLatch;
		}
	}
}

static void QTAiPower(void) {
	SetReadHandler(0x6000, 0xFFFF, CartBR);
	SetWriteHandler(0x6000, 0x7FFF, CartBW);
	SetWriteHandler(0x8000, 0xFFFF, QTAiWrite);
	SetReadHandler(0xDC00, 0xDC00, QTAiRead);
	SetReadHandler(0xDD00, 0xDD00, QTAiRead);
	FCEU_CheatAddRAM(WRAMSIZE >> 10, 0x6000, WRAM);
	Sync();
}

static void QTAiClose(void) {
	CHRRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	CHRRAM = nullptr;
	WRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	WRAM = nullptr;
}

static void StateRestore(int version) {
	Sync();
}

void QTAi_Init(CartInfo *info) {
	QTAIHack = 1;

	info->Power = QTAiPower;
	info->Close = QTAiClose;
	GameStateRestore = StateRestore;

	g_cpu.map_irq_hook_ref() = VRC5IRQ;

	CHRRAM_owner = FCEU_gmalloc_unique(CHRSIZE);  // v0.3.6: RAII-wrapped
	CHRRAM = CHRRAM_owner.get();
	SetupCartCHRMapping(0x10, CHRRAM, CHRSIZE, 1);
	AddExState(CHRRAM, CHRSIZE, 0, "CRAM");

	WRAM_owner = FCEU_gmalloc_unique(WRAMSIZE);  // v0.3.6: RAII-wrapped
	WRAM = WRAM_owner.get();
	SetupCartPRGMapping(0x10, WRAM, WRAMSIZE, 1);
	AddExState(WRAM, WRAMSIZE, 0, "WRAM");

	if (info->battery) {
		// note, only extrnal cart's SRAM is battery backed, the the part on the main cartridge is just
		// an additional work ram. so we may save only half here, but I forgot what part is saved lol, will 
		// find out later.
		info->addSaveGameBuf( WRAM, WRAMSIZE );
	}

	AddExState(&StateRegs, ~0, 0, 0);
}
