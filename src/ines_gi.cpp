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
// iNES game interface hooks — extracted from ines.cpp for v1.10 Cryptex Phase A.3.

#include "types.h"
#include "x6502.h"
#include "fceu.h"
#include "cart.h"
#include "ines.h"
#include "utils/memory.h"

#include <cstdlib>

extern uint8 *trainerpoo;
extern FceuMallocPtr trainerpoo_owner;
extern uint8 *ExtraNTARAM;
extern FceuMallocPtr ExtraNTARAM_owner;
extern int CHRRAMSize;
extern CartInfo iNESCart;

static DECLFR(TrainerRead) {
	return(trainerpoo[A & 0x1FF]);
}

static void iNES_ExecPower() {
	if (CHRRAMSize != -1)
		FCEU_MemoryRand(VROM, CHRRAMSize);

	if (iNESCart.Power)
		iNESCart.Power();

	if (trainerpoo) {
		int x;
		for (x = 0; x < 512; x++) {
			X6502_DMW(0x7000 + x, trainerpoo[x]);
			if (X6502_DMR(0x7000 + x) != trainerpoo[x]) {
				SetReadHandler(0x7000, 0x71FF, TrainerRead);
				break;
			}
		}
	}
}

void iNESGI(GI h) {
	switch (h) {
	case GI_RESETSAVE:
		FCEU_ClearGameSave(&iNESCart);
		break;

	case GI_RESETM2:
		if (iNESCart.Reset)
			iNESCart.Reset();
		break;
	case GI_POWER:
		iNES_ExecPower();
		break;
	case GI_SAVESRAM:
		// v2.0.8 C1 (P1): periodic flush -- save only, no Close teardown
		// (that stays GI_CLOSE's job).
		FCEU_SaveGameSave(&iNESCart);
		break;
	case GI_CLOSE:
	{
		FCEU_SaveGameSave(&iNESCart);
		if (iNESCart.Close)
			iNESCart.Close();
		if (ROM) {
			FCEU_free(ROM);
			ROM = NULL;
		}
		if (VROM) {
			FCEU_free(VROM);
			VROM = NULL;
		}
		if (trainerpoo) {
			trainerpoo_owner.reset();
			trainerpoo = NULL;
		}
		if (ExtraNTARAM) {
			ExtraNTARAM_owner.reset();
			ExtraNTARAM = NULL;
		}
	}
	break;
	}
}
