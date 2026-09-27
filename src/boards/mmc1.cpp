/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 1998 BERO
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
#include "simple_carts.h"          // v1.8 Phase E.2 step 9.1: Mapper105Cart

#include <cstdio>
#include <cstdlib>

// v1.18.2 kgmqa-078 probe: FCEUX11_MMC1_PROBE=1 dumps the board geometry the
// loader actually handed MMC1 (declared PRG/CHR vs. the ROM's real size) and,
// on every MMC1PRG sync, which 16K banks land at $8000 / $C000 and whether the
// PRG bank register was allowed to move them. Silent otherwise, so ctest 34/34
// is unaffected. Kept in-tree: it is the evidence behind the submapper-5
// "Fixed PRG" model documented in
// docs/plans/long-term-evolution/T0-指令时序与板级约束.md ②.
#define MMC1_PROBE_LOG(...) do { if (mmc1_probe_on()) { std::fprintf(stderr, __VA_ARGS__); std::fflush(stderr); } } while (0)

static bool mmc1_probe_on() {
	static const bool on = []() {
		const char* e = std::getenv("FCEUX11_MMC1_PROBE");
		return e && e[0] == '1' && e[1] == '\0';
	}();
	return on;
}

static void GenMMC1Power(void);
static void GenMMC1Init(CartInfo *info, int prg, int chr, int wram, int bram);

// v1.18.2 kgmqa-078: NES 2.0 header byte 8 high nibble. Per the NES 2.0
// submapper table, mapper 001 submapper 5 is "Fixed PRG" (SEROM / SHROM /
// SH1ROM): PRG ROM A14 goes straight to CPU A14 instead of through the MMC1,
// so the PRG bank register cannot move anything. Reference:
// https://www.nesdev.org/wiki/NES_2.0_submappers#001:_5_Fixed_PRG
#define MMC1_SUBMAPPER_FIXED_PRG 5

FCEUX11_MAPPER_HOT static uint8 DRegs[4];
static uint8 Buffer, BufferShift;

static uint32 WRAMSIZE;
static uint32 NONBRAMSIZE; // size of non-battery-backed portion of WRAM

static void (*MMC1CHRHook4)(uint32 A, uint8 V);
static void (*MMC1PRGHook16)(uint32 A, uint8 V);

static uint8 *WRAM = NULL;
static FceuMallocPtr WRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static uint8 *CHRRAM = NULL;
static FceuMallocPtr CHRRAM_owner;  // v0.3.6: RAII owner; FCEU_gfree on destruction
static int is155, is171;
static int isFixedPRG;	// v1.18.2 kgmqa-078: NES 2.0 submapper 5 (SEROM/SHROM/SH1ROM)

static DECLFW(MBWRAM) {
	if (!(DRegs[3] & 0x10) || is155)
		fceu11::g_bus.page()[A >> 11][A] = V;  // WRAM is enabled.
}

static DECLFR(MAWRAM) {
	if ((DRegs[3] & 0x10) && !is155)
		return g_cpu.native_layout().DB;          // WRAM is disabled
	return(fceu11::g_bus.page()[A >> 11][A]);
}

static void MMC1CHR(void) {
	if (WRAMSIZE > 0x2000) {
		if (WRAMSIZE > 0x4000)
			setprg8r(0x10, 0x6000, (DRegs[1] >> 2) & 3);
		else
			setprg8r(0x10, 0x6000, (DRegs[1] >> 3) & 1);
	}

	if (MMC1CHRHook4) {
		if (DRegs[0] & 0x10) {
			MMC1CHRHook4(0x0000, DRegs[1]);
			MMC1CHRHook4(0x1000, DRegs[2]);
		} else {
			MMC1CHRHook4(0x0000, (DRegs[1] & 0xFE));
			MMC1CHRHook4(0x1000, DRegs[1] | 1);
		}
	} else {
		if (DRegs[0] & 0x10) {
			setchr4(0x0000, DRegs[1]);
			setchr4(0x1000, DRegs[2]);
		} else
			setchr8(DRegs[1] >> 1);
	}
}

static void MMC1PRG(void) {
	uint8 offs_16banks = DRegs[1] & 0x10;
	uint8 prg_reg = DRegs[3] & 0xF; //homebrewers arent allowed to use more banks on MMC1. use another mapper.
	if (mmc1_probe_on()) {
		// What a plain MMC1 WOULD have mapped here, versus what actually gets
		// mapped. On a submapper-5 board the two differ by construction: the
		// real hardware ignores the bank register. Note that with only 32K of
		// PRG the mask is 1, so `(prg_reg & ~1) & 1 == 0` and the plain path
		// is already numerically identical — i.e. this ROM cannot tell the two
		// models apart. See Mapper1_Init for why the fix is still correct.
		const uint32 mask16 = fceu11::g_bus.prg_mask16()[0];
		uint32 lo, hi;
		switch (DRegs[0] & 0xC) {
		case 0xC: lo = (uint32)(prg_reg + offs_16banks) & mask16;
		           hi = (uint32)(0xF + offs_16banks) & mask16; break;
		case 0x8: lo = (uint32)offs_16banks & mask16;
		           hi = (uint32)(prg_reg + offs_16banks) & mask16; break;
		default:  lo = (uint32)((prg_reg & ~1) + offs_16banks) & mask16;
		           hi = (uint32)((prg_reg & ~1) + offs_16banks + 1) & mask16; break;
		}
		const bool differs = (lo != 0) || (hi != mask16);
		MMC1_PROBE_LOG("MMC1PRG mode=%X DRegs1=%02X DRegs3=%02X prg_reg=%u | "
		            "PRGmask16=%u (%u x 16K) | plain_mmc1_would_map: "
		            "$8000=bank%u $C000=bank%u | fixed_prg=%d actual: $8000=bank%u "
		            "$C000=bank%u%s\n",
		            DRegs[0] & 0xC, DRegs[1], DRegs[3], prg_reg,
		            mask16, mask16 + 1, lo, hi, isFixedPRG ? 1 : 0,
		            0u, mask16,
		            differs ? "  <- banking suppressed by fixed-PRG" : "");
	}
	// v1.18.2 kgmqa-078: submapper-5 "Fixed PRG" boards (SEROM / SHROM /
	// SH1ROM) have no PRG banking at all — A14 is hardwired to CPU A14 — so
	// neither the mode bits in DRegs[0] nor the bank register in DRegs[3] (nor
	// the CHR0 bit-4 alias in DRegs[1]) may influence decoding. $8000-BFFF is
	// permanently the low 16K bank, $C000-FFFF the high one. The register
	// writes still latch into DRegs so that CHR/mirroring keep working and
	// savestates stay byte-identical in shape.
	if (isFixedPRG) {
		if (MMC1PRGHook16) {
			MMC1PRGHook16(0x8000, 0);
			MMC1PRGHook16(0xC000, PRGmask16[0]);
		} else {
			setprg16(0x8000, 0);
			setprg16(0xC000, PRGmask16[0]);
		}
		return;
	}
	if (MMC1PRGHook16) {
		switch (DRegs[0] & 0xC) {
		case 0xC:
			MMC1PRGHook16(0x8000, (prg_reg + offs_16banks));
			MMC1PRGHook16(0xC000, 0xF + offs_16banks);
			break;
		case 0x8:
			MMC1PRGHook16(0xC000, (prg_reg + offs_16banks));
			MMC1PRGHook16(0x8000, offs_16banks);
			break;
		case 0x0:
		case 0x4:
			MMC1PRGHook16(0x8000, ((prg_reg & ~1) + offs_16banks));
			MMC1PRGHook16(0xc000, ((prg_reg & ~1) + offs_16banks + 1));
			break;
		}
	} else {
		switch (DRegs[0] & 0xC) {
		case 0xC:
			setprg16(0x8000, (prg_reg + offs_16banks));
			setprg16(0xC000, 0xF + offs_16banks);
			break;
		case 0x8:
			setprg16(0xC000, (prg_reg + offs_16banks));
			setprg16(0x8000, offs_16banks);
			break;
		case 0x0:
		case 0x4:
			setprg16(0x8000, ((prg_reg & ~1) + offs_16banks));
			setprg16(0xc000, ((prg_reg & ~1) + offs_16banks + 1));
			break;
		}
	}
}

static void MMC1MIRROR(void) {
	if (!is171)
		switch (DRegs[0] & 3) {
		case 2: setmirror(MI_V); break;
		case 3: setmirror(MI_H); break;
		case 0: setmirror(MI_0); break;
		case 1: setmirror(MI_1); break;
		}
}

static uint64 lreset;
static DECLFW(MMC1_write) {
	int n = (A >> 13) - 4;

	/* The MMC1 is busy so ignore the write. */
	/* As of version FCE Ultra 0.81, the timestamp is only
		increased before each instruction is executed(in other words
		precision isn't that great), but this should still work to
		deal with 2 writes in a row from a single RMW instruction.
	*/
	if ((timestampbase + g_cpu.timestamp_ref()) < (lreset + 2))
		return;
//	MMC1_PROBE_LOG("Write %04x:%02x\n",A,V);
	if (V & 0x80) {
		DRegs[0] |= 0xC;
		BufferShift = Buffer = 0;
		MMC1PRG();
		lreset = timestampbase + g_cpu.timestamp_ref();
		return;
	}

	Buffer |= (V & 1) << (BufferShift++);

	if (BufferShift == 5) {
		DRegs[n] = Buffer;
		BufferShift = Buffer = 0;
		switch (n) {
		case 0: MMC1MIRROR(); MMC1CHR(); MMC1PRG(); break;
		case 1: MMC1CHR(); MMC1PRG(); break;
		case 2: MMC1CHR(); break;
		case 3: MMC1PRG(); break;
		}
	}
}

static void MMC1_Restore(int version) {
	MMC1MIRROR();
	MMC1CHR();
	MMC1PRG();
	lreset = 0;		// timestamp(base) is not stored in save states.
}

static void MMC1CMReset(void) {
	int i;

	for (i = 0; i < 4; i++)
		DRegs[i] = 0;
	Buffer = BufferShift = 0;
	DRegs[0] = 0x1F;

	DRegs[1] = 0;
	DRegs[2] = 0;	// Should this be something other than 0?
	DRegs[3] = 0;

	MMC1MIRROR();
	MMC1CHR();
	MMC1PRG();
}

static int DetectMMC1WRAMSize(CartInfo *info, int *bs) {
	int ws = 8;
	switch (info->CRC32) {
	case 0xc6182024:	// Romance of the 3 Kingdoms
	case 0xabbf7217:	// ""        "" (J) (PRG0)
	case 0xccf35c02:	// ""        "" (J) (PRG1)
	case 0x2225c20f:	// Genghis Khan
	case 0xfb69743a:	// ""        "" (J)
	case 0x4642dda6:	// Nobunaga's Ambition
	case 0x3f7ad415:	// ""        "" (J) (PRG0)
	case 0x2b11e0b0:	// ""        "" (J) (PRG1)
		*bs = 8;
		ws = 16;
		break;
	case 0xb8747abf:	// Best Play Pro Yakyuu Special (J) (PRG0)
	case 0xc3de7c69:	// ""        "" (J) (PRG1)
	case 0xc9556b36:	// Final Fantasy I & II (J) [!]
		*bs = 32;
		ws = 32;
		break;
	default:
		if(info->ines2) {
			ws = (info->wram_size + info->battery_wram_size) / 1024;
			*bs = info->battery_wram_size / 1024;
			// we only support sizes between 8K and 32K
			if (ws > 0 && ws < 8) ws = 8;
			if (ws > 32) ws = 32;
			if (*bs > ws) *bs = ws;
		}
	}
	if (ws > 8)
		MMC1_PROBE_LOG(" >8KB external WRAM present.  Use NES 2.0 if you hack the ROM image.\n");
	return ws;
}

static uint32 NWCIRQCount;
static uint8 NWCRec;
#define NWCDIP 0xE

static void NWCIRQHook(int a) {
	if (!(NWCRec & 0x10)) {
		NWCIRQCount += a;
		if ((NWCIRQCount | (NWCDIP << 25)) >= 0x3e000000) {
			NWCIRQCount = 0;
			X6502_IRQBegin(FCEU_IQEXT);
		}
	}
}

static void NWCCHRHook(uint32 A, uint8 V) {
	if ((V & 0x10)) { // && !(NWCRec&0x10))
		NWCIRQCount = 0;
		X6502_IRQEnd(FCEU_IQEXT);
	}

	NWCRec = V;
	if (V & 0x08)
		MMC1PRG();
	else
		setprg32(0x8000, (V >> 1) & 3);
}

static void NWCPRGHook(uint32 A, uint8 V) {
	if (NWCRec & 0x8)
		setprg16(A, 8 | (V & 0x7));
	else
		setprg32(0x8000, (NWCRec >> 1) & 3);
}

static void NWCPower(void) {
	GenMMC1Power();
	setchr8r(0, 0);
}

void Mapper105_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 256, 8, 0);
	MMC1CHRHook4 = NWCCHRHook;
	MMC1PRGHook16 = NWCPRGHook;
	g_cpu.set_map_irq_hook(NWCIRQHook);
	info->Power = NWCPower;
}

static void GenMMC1Power(void) {
	lreset = 0;
	SetWriteHandler(0x8000, 0xFFFF, MMC1_write);
	SetReadHandler(0x8000, 0xFFFF, CartBR);

	if (WRAMSIZE) {
		FCEU_CheatAddRAM(8, 0x6000, WRAM);

		// clear non-battery-backed portion of WRAM
		if (NONBRAMSIZE)
			FCEU_MemoryRand(WRAM, NONBRAMSIZE, true);

		SetReadHandler(0x6000, 0x7FFF, MAWRAM);
		SetWriteHandler(0x6000, 0x7FFF, MBWRAM);
		setprg8r(0x10, 0x6000, 0);
	}

	MMC1CMReset();
}

static void GenMMC1Close(void) {
	CHRRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	WRAM_owner.reset();  // v0.3.6: RAII owner frees via FCEU_gfree
	CHRRAM = WRAM = NULL;
}

static void GenMMC1Init(CartInfo *info, int prg, int chr, int wram, int bram) {
	is155 = 0;
	isFixedPRG = 0;

	info->Close = GenMMC1Close;
	MMC1PRGHook16 = MMC1CHRHook4 = 0;
	WRAMSIZE = wram * 1024;
	NONBRAMSIZE = (wram - bram) * 1024;
	PRGmask16[0] &= (prg >> 14) - 1;
	CHRmask4[0] &= (chr >> 12) - 1;
	CHRmask8[0] &= (chr >> 13) - 1;

	if (mmc1_probe_on()) {
		// The masks above are derived from the *declared* board geometry, while
		// prg_size() is what the iNES loader really allocated. A mismatch means
		// setprg16r() will index past the end of the PRG-ROM buffer.
		MMC1_PROBE_LOG("MMC1INIT declared prg=%dK chr=%dK wram=%dK bram=%dK | "
		            "real prg_size=%uK chr_size=%uK ines2=%d submapper=%u mapper=%d "
		            "crc=%08X\n",
		            prg, chr, wram, bram,
		            fceu11::g_bus.prg_size()[0] / 1024,
		            fceu11::g_bus.chr_size()[0] / 1024,
		            info->ines2 ? 1 : 0, info->submapper, info->mapper_number,
		            info->CRC32);
		MMC1_PROBE_LOG("MMC1INIT masks: PRGmask16=%u (%u x 16K -> up to %uK) "
		            "CHRmask4=%u CHRmask8=%u\n",
		            PRGmask16[0], PRGmask16[0] + 1, (PRGmask16[0] + 1) * 16,
		            CHRmask4[0], CHRmask8[0]);
	}

	if (WRAMSIZE) {
		WRAM_owner = FCEU_gmalloc_unique(WRAMSIZE);  // v0.3.6: RAII-wrapped
		WRAM = WRAM_owner.get();
		SetupCartPRGMapping(0x10, WRAM, WRAMSIZE, 1);
		AddExState(WRAM, WRAMSIZE, 0, "WRAM");
		if (bram) {
			info->addSaveGameBuf( WRAM + NONBRAMSIZE, bram * 1024 );
		}
	}
	if (!chr) {
		CHRRAM_owner = FCEU_gmalloc_unique(8192);  // v0.3.6: RAII-wrapped
		CHRRAM = CHRRAM_owner.get();
		SetupCartCHRMapping(0, CHRRAM, 8192, 1);
		AddExState(CHRRAM, 8192, 0, "CHRR");
	}
	AddExState(DRegs, 4, 0, "DREG");

	info->Power = GenMMC1Power;
	GameStateRestore = MMC1_Restore;
	AddExState(&lreset, 8, 1, "LRST");
	AddExState(&Buffer, 1, 1, "BFFR");
	AddExState(&BufferShift, 1, 1, "BFRS");
}

void Mapper1_Init(CartInfo *info) {
	int bs = info->battery ? 8 : 0;
	int ws = DetectMMC1WRAMSize(info, &bs);
	MMC1_PROBE_LOG("Mapper1_Init: ines2=%d submapper=%u mapper=%d battery=%d "
	                "ws=%d bs=%d real_prg=%u real_chr=%u -> %s\n",
	                info->ines2 ? 1 : 0, info->submapper, info->mapper_number,
	                info->battery ? 1 : 0, ws, bs,
	                fceu11::g_bus.prg_size()[0], fceu11::g_bus.chr_size()[0],
	                (info->ines2 && info->submapper == MMC1_SUBMAPPER_FIXED_PRG)
	                    ? "FIXED_PRG" : "plain MMC1");
	// v1.18.2 kgmqa-078. NES 2.0 mapper 001 submapper 5 is "Fixed PRG"
	// (SEROM / SHROM / SH1ROM). bmap[] is keyed on mapper number alone, so
	// this submapper never reached the mapper before. Two separate defects;
	// (1) masks (2) completely, so both are recorded here.
	//
	// (1) $6000 was unmapped, so the failure was not about banking at all.
	//     This NES 2.0 image declares no PRG-RAM, and DetectMMC1WRAMSize
	//     trusts the header for NES 2.0 carts — while a plain iNES MMC1 image
	//     falls through to that function's 8 KB default. The blargg $6000
	//     result register was therefore unwritable, and every run reported the
	//     same constant 0xC3 no matter what the mapper did. Submapper 5 places
	//     no requirement on PRG RAM (every real SEROM/SHROM board carries
	//     8 KB), so fall back to the conventional size when the header
	//     specifies nothing. Blast radius: only mapper 001 submapper 5 images
	//     with an empty PRG-RAM field — serom.nes is the only such image in
	//     the fixture set.
	//
	// (2) Submapper 5 also has no PRG banking: A14 is hardwired to CPU A14.
	if (info->ines2 && info->submapper == MMC1_SUBMAPPER_FIXED_PRG) {
		if (ws == 0) ws = 8;
		// Geometry args stay identical to the plain path on purpose: the only
		// behavioural delta should be the bank register going dead. GenMMC1Init
		// masks with &= against the loader's real-size masks (bus.cpp derives
		// those from the actual ROM), so the declared values can only narrow.
		GenMMC1Init(info, 512, 256, ws, bs);
		isFixedPRG = 1;
		MMC1PRG();  // GenMMC1Init synced before the flag was set
		return;
	}
	GenMMC1Init(info, 512, 256, ws, bs);
}

/* Same as mapper 1, without respect for WRAM enable bit. */
void Mapper155_Init(CartInfo *info) {
	GenMMC1Init(info, 512, 256, 8, info->battery ? 8 : 0);
	is155 = 1;
}

/* Same as mapper 1, with different (or without) mirroring control. */
/* Kaiser KS7058 board, KS203 custom chip */
void Mapper171_Init(CartInfo *info) {
	GenMMC1Init(info, 32, 32, 0, 0);
	is171 = 1;
}

void SAROM_Init(CartInfo *info) {
	GenMMC1Init(info, 128, 64, 8, info->battery ? 8 : 0);
}

void SBROM_Init(CartInfo *info) {
	GenMMC1Init(info, 128, 64, 0, 0);
}

void SCROM_Init(CartInfo *info) {
	GenMMC1Init(info, 128, 128, 0, 0);
}

void SEROM_Init(CartInfo *info) {
	GenMMC1Init(info, 32, 64, 0, 0);
}

void SGROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 0, 0, 0);
}

void SKROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 64, 8, info->battery ? 8 : 0);
}

void SLROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 128, 0, 0);
}

void SL1ROM_Init(CartInfo *info) {
	GenMMC1Init(info, 128, 128, 0, 0);
}

/* Begin unknown - may be wrong - perhaps they use different MMC1s from the
	similarly functioning boards?
*/

void SL2ROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 256, 0, 0);
}

void SFROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 256, 0, 0);
}

void SHROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 256, 0, 0);
}

/* End unknown  */
/*              */
/*              */

void SNROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 0, 8, info->battery ? 8 : 0);
}

void SOROM_Init(CartInfo *info) {
	GenMMC1Init(info, 256, 0, 16, info->battery ? 8 : 0);
}

// ---------------------------------------------------------------------------
// v1.7 Phase F: Mmc1Cart subclass (Strategy A).
//
// Definitions live in mmc1.cpp because they need access to the static
// GenMMC1Power() and MMC1CMReset() helpers. The cart subclass only owns
// the *call sequence* (Power -> cart_obj->on_power -> GenMMC1Power; Reset ->
// cart_obj->on_reset -> MMC1CMReset), not the cart wiring logic itself.
// ---------------------------------------------------------------------------

#include "boards/mmc1_cart.h"
#include "boards/registry.h"

namespace fceu11 {

void Mmc1Cart::on_power() noexcept {
	// v1.15 B.1: Direct call to GenMMC1Power — no legacy function pointer
	// indirection. Mapper1_Init was already called during iNES_Init
	// (registers SFORMAT, allocates WRAM/CHRRAM, sets up DRegs).
	// on_power() only needs to fire the bank-sync + read/write handlers.
	GenMMC1Power();
}

void Mmc1Cart::on_reset() noexcept {
	// GenMMC1Init does NOT set info->Reset (MMC1 historically relied on the
	// full Power cycle to restore registers), so the v1.7 factory leaves
	// info->Reset pointing at CartInfo_ResetForward. Our on_reset does the
	// actual reset work: zero the shift register / buffer / four data regs,
	// then re-sync mirroring, CHR, and PRG banking.
	MMC1CMReset();
}

// v1.8 Masonry §6.1: Mmc1Cart::save_mapper_state() — capture MMC1 register
// file for byte-diff regression.  The state lives in mmc1.cpp's file-scope
// globals (DRegs[4] / Buffer / BufferShift / WRAMSIZE / is155 / is171) so
// the override is placed in the same TU.  8 bytes total.
std::vector<uint8_t> Mmc1Cart::save_mapper_state() const noexcept {
	std::vector<uint8_t> out;
	out.reserve(8);
	pack_u8_array(out, DRegs, 4);    // 4 data registers (control, CHR0, CHR1, PRG)
	pack_u8(out, Buffer);
	pack_u8(out, BufferShift);
	pack_u8(out, is155 ? 1 : 0);
	pack_u8(out, is171 ? 1 : 0);
	return out;  // 8 bytes
}

namespace {

// v1.8 Masonry §2: MapperEntryRegister for MMC1 (mapper 1).
static MapperEntryRegister kMmc1Register{
    MapperEntry{
        /*mapper_number=*/1,
        /*name=*/"MMC1",
        /*legacy_init=*/&Mapper1_Init,
        /*factory=*/[](Bus& bus) { return std::make_unique<Mmc1Cart>(bus); }
    }
};

// v1.8 Masonry Phase E.2 step 9.2: NES-EVENT NWC1990 (mapper 105) is
// MMC1-based; Cart subclass uses MapperStrategyA default (16-byte body).
static MapperEntryRegister kMapper105Register{
    MapperEntry{105, "NES-EVENT NWC1990", &Mapper105_Init,
        [](Bus& bus) { return std::make_unique<Mapper105Cart>(bus); }}
};

}  // namespace

} // namespace fceu11
