// iNES mapper initialization â€?extracted from ines.cpp for v1.10 Cryptex Phase A.3.

#include "types.h"
#include "utils/memory.h"
#include "fceu.h"
#include "cart.h"
#include "ines.h"
#include "state.h"
#include "vsuni.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"

#include "rust/fceux11_rust.h"

#include <cstdlib>
#include <cstring>

extern SFORMAT FCEUVSUNI_STATEINFO[];
extern BMAPPINGLocal bmap[];
extern int CHRRAMSize;
extern uint8 *UNIFchrrama;
extern CartInfo iNESCart;
extern uint8 *VROM;
extern uint32 VROM_size;
extern uint8 *ExtraNTARAM;
extern iNES_HEADER head;

int iNES_Init(int num) {
	BMAPPINGLocal *tmp = bmap;

	iNESCart.mapper_number = num;
	CHRRAMSize = -1;

	if (GameInfo->type == GIT_VSUNI)
		AddExState(FCEUVSUNI_STATEINFO, ~0, 0, 0);

	while (tmp->init) {
		if (num == tmp->number) {
			UNIFchrrama = NULL;	// need here for compatibility with UNIF mapper code
			if (!VROM_size) {
				CHRRAMSize = fceux11_rust_cart_compute_chrram_size(
					num, iNESCart.ines2, iNESCart.vram_size,
					iNESCart.battery_vram_size, false);
				if (!iNESCart.ines2)
				{
					iNESCart.vram_size = CHRRAMSize;
				}
				if (CHRRAMSize > 0)
				{
					// v2.0.4 V10 (upstream commit 8510fb8): setchr8() maps a
					// full 8k window from the start of the chip even when the
					// chip is smaller, so anything under 8k has to be padded
					// or PPU $2007 writes walk off the end of the buffer.
					int mCHRRAMSize = (CHRRAMSize < 8192) ? 8192 : CHRRAMSize;
					if ((UNIFchrrama = VROM = (uint8*)FCEU_malloc(mCHRRAMSize)) == NULL) return 2;
					// Real size only: the RAMInitOption pattern on the live
					// CHR bytes must not change (movie stability for >=8k carts).
					FCEU_MemoryRand(VROM, CHRRAMSize);
					// Padding beyond the real size is zeroed deterministically
					// (double insurance; FCEU_malloc already zeroes the block).
					memset(VROM + CHRRAMSize, 0, mCHRRAMSize - CHRRAMSize);
					SetupCartCHRMapping(0, VROM, CHRRAMSize, 1);
					AddExState(VROM, CHRRAMSize, 0, "CHRR");
				}
				else {
					VROM = NULL;
				}
			}
			if (head.ROM_type & 8)
			{
				if (ExtraNTARAM != NULL)
				{
					AddExState(ExtraNTARAM, 2048, 0, "EXNR");
				}
			}
			tmp->init(&iNESCart);
			return 0;
		}
		tmp++;
	}
	return 1;
}
