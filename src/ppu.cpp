/* FCE Ultra - NES/Famicom Emulator
 *
 * Copyright notice for this file:
 *  Copyright (C) 1998 BERO
 *  Copyright (C) 2003 Xodnizel
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

#include "types.h"
#include "x6502.h"
#include "fceu.h"
#include "ppu.h"
#include "ppu_rendering.h"
#include "ppu_state.h"
#include "nsf.h"
#include "sound.h"
#include "file.h"
#include "utils/endian.h"
#include "utils/memory.h"
		 
#include "cart.h"
#include "palette.h"
#include "state.h"
#include "video.h"
#include "input.h"
#include "core_api.h"
#include "io_api.h"
#include "net_api.h"
#include "diag_api.h"
#include "debug.h"
		 
#include <array>
#include <cassert>   // Stage-2 §九 L4: debug-only guard inside CALL_PPUREAD
#include <cstring>
#include <cstdio>
#include <cstdlib>

#define VBlankON    (PPU[0] & 0x80)	//Generate VBlank NMI
#define Sprite16    (PPU[0] & 0x20)	//Sprites 8x16/8x8
#define BGAdrHI     (PPU[0] & 0x10)	//BG pattern adr $0000/$1000
#define SpAdrHI     (PPU[0] & 0x08)	//Sprite pattern adr $0000/$1000
#define INC32       (PPU[0] & 0x04)	//auto increment 1/32

#define SpriteON    (PPU[1] & 0x10)	//Show Sprite
#define ScreenON    (PPU[1] & 0x08)	//Show screen
#define PPUON       (PPU[1] & 0x18)	//PPU should operate
#define GRAYSCALE   (PPU[1] & 0x01)	//Grayscale (AND palette entries with 0x30)

#define SpriteLeft8 (PPU[1] & 0x04)
#define BGLeft8     (PPU[1] & 0x02)

#define PPU_status  (PPU[2])

#define READPALNOGS(ofs)    (PALRAM[(ofs)])
#define READPAL(ofs)    (PALRAM[(ofs)] & (GRAYSCALE ? 0x30 : 0xFF))
#define READUPAL(ofs)   (UPALRAM[(ofs)] & (GRAYSCALE ? 0x30 : 0xFF))

// v1.13 Purify Phase A: forward decls for sprite eval.
extern void FetchSpriteData(void);
extern void RefreshSprites(void);
extern void CopySprites(uint8 *target);
bool new_ppu_reset = false;

// v1.12 Scissors Phase E-A: PPUPHASE / SPRITE_READ / idleSynch +
// new_ppu_reset globals. Struct definitions live in ppu_class.h.
PPUPHASE ppuphase;
SPRITE_READ spr_read;
uint8 idleSynch = 1;

PPUREGS ppur;

// v1.13 Purify Phase A: makeppulut definition in ppu_rendering.cpp.

int ppudead = 1;
int kook = 0;
int fceuindbg = 0;

// Configurable no-bg fill color. 0xFF = use palette[0].
uint8 gNoBGFillColor = 0xFF;

int MMC5Hack = 0;
uint32 MMC5HackVROMMask = 0;
uint8 *MMC5HackExNTARAMPtr = 0;
uint8 *MMC5HackVROMPTR = 0;
uint8 MMC5HackCHRMode = 0;
uint8 MMC5HackSPMode = 0;
uint8 MMC50x5130 = 0;
uint8 MMC5HackSPScroll = 0;
uint8 MMC5HackSPPage = 0;

int PEC586Hack = 0;

int QTAIHack = 0;
uint8 QTAINTRAM[2048];
uint8 qtaintramreg;

// hotfix2 P2-3 (DS-4): SPRBUF is now SPRB[64] (4-byte packed sprite
// descriptors) instead of a raw byte buffer. Same 256-byte footprint
// (64 × 4) but type-safe — callers in ppu_rendering.cpp can write
// `SPRBUF[ns] = dst;` directly and the compiler emits a single 4-byte
// store instead of going through memcpy.
alignas(64) SPRB SPRBUF[64];
alignas(64) std::array<uint8_t, 0x20> PALRAM;
std::array<uint8_t, 3> UPALRAM;

int g_rasterpos;
uint32 scanlines_per_frame;
uint8 PPUSPL;
uint8 VRAMBuffer = 0, PPUGenLatch = 0;

uint8* MMC5BGVRAMADR(uint32 A);

//Color deemphasis emulation.  Joy...
uint8 SpriteDMA = 0; // $4014 copies 256 bytes from $xx00-$xxFF to $2004 (OAM data)

#define MMC5SPRVRAMADR(V)   &MMC5SPRVPage[(V) >> 10][(V)]
#define VRAMADR(V)          &VPage[(V) >> 10][(V)]

uint8 READPAL_MOTHEROFALL(uint32 A)
{
	if(!(A & 3)) {
		if(!(A & 0xC))
			return READPAL(0x00);
		else
			return READUPAL(((A & 0xC) >> 2) - 1);
	}
	else
		return READPAL(A & 0x1F);
}

//this duplicates logic which is embedded in the ppu rendering code
//which figures out where to get CHR data from depending on various hack modes
//mostly involving mmc5.
//this might be incomplete.
uint8* FCEUPPU_GetCHR(uint32 vadr, uint32 refreshaddr) {
	if (MMC5Hack) {
		if (MMC5HackCHRMode == 1) {
			uint8 *C = MMC5HackVROMPTR;
			C += (((MMC5HackExNTARAMPtr[refreshaddr & 0x3ff]) & 0x3f & MMC5HackVROMMask) << 12) + (vadr & 0xfff);
			C += (MMC50x5130 & 0x3) << 18;	//11-jun-2009 for kuja_killer
			return C;
		} else {
			return MMC5BGVRAMADR(vadr);
		}
	} else return VRAMADR(vadr);
}

//likewise for ATTR
int FCEUPPU_GetAttr(int ntnum, int xt, int yt) {
	int attraddr = 0x3C0 + ((yt >> 2) << 3) + (xt >> 2);
	int temp = (((yt & 2) << 1) + (xt & 2));
	int refreshaddr = xt + yt * 32;
	if (MMC5Hack && MMC5HackCHRMode == 1)
		return (MMC5HackExNTARAMPtr[refreshaddr & 0x3ff] & 0xC0) >> 6;
	else
		return (vnapage[ntnum][attraddr] & (3 << temp)) >> temp;
}

//new ppu-----
inline void FFCEUX_PPUWrite_Default(uint32 A, uint8 V) {
	uint32 tmp = A;

	if (PPU_hook) PPU_hook(A);

	if (tmp < 0x2000) {
		if (PPUCHRRAM & (1 << (tmp >> 10)))
			VPage[tmp >> 10][tmp] = V;
	} else if (tmp < 0x3F00) {
		if (QTAIHack && (qtaintramreg & 1)) {
			QTAINTRAM[((((tmp & 0xF00) >> 10) >> ((qtaintramreg >> 1)) & 1) << 10) | (tmp & 0x3FF)] = V;
		} else {
			if (PPUNTARAM & (1 << ((tmp & 0xF00) >> 10)))
				vnapage[((tmp & 0xF00) >> 10)][tmp & 0x3FF] = V;
		}
	} else {
		if (!(tmp & 3)) {
			if (!(tmp & 0xC)) {
				PALRAM[0x00] = PALRAM[0x04] = PALRAM[0x08] = PALRAM[0x0C] = V & 0x3F;
				PALRAM[0x10] = PALRAM[0x14] = PALRAM[0x18] = PALRAM[0x1C] = V & 0x3F;
			}
			else
				UPALRAM[((tmp & 0xC) >> 2) - 1] = V & 0x3F;
		} else
			PALRAM[tmp & 0x1F] = V & 0x3F;
	}
}

volatile int rendercount, vromreadcount, undefinedvromcount, LogAddress = -1;
unsigned char *cdloggervdata = NULL;
unsigned int cdloggerVideoDataSize = 0;

int GetCHRAddress(int A) 
{
	if (cdloggerVideoDataSize) 
	{
		int result = -1;
		if ( (A >= 0) && (A < 0x2000) )
		{
			result = &VPage[A >> 10][A] - CHRptr[0];
		}
		if ((result >= 0) && (result < (int)cdloggerVideoDataSize))
		{
			return result;
		}
	}
	else
	{
		if ( (A >= 0) && (A < 0x2000) ) return A;
	}
	return -1;
}

int GetCHROffset(uint8 *ptr) {
	int result = ptr - CHRptr[0];
	if (cdloggerVideoDataSize) {
		if ((result >= 0) && (result < (int)cdloggerVideoDataSize))
			return result;
	} else {
		if ((result >= 0) && (result < 0x2000))
			return result;
	}
	return -1;
}

#define RENDER_LOG(tmp) { \
		if (debug_loggingCD) \
		{ \
			int addr = GetCHRAddress(tmp); \
			if (addr != -1)	\
			{ \
				if (!(cdloggervdata[addr] & 1))	\
				{ \
					cdloggervdata[addr] |= 1; \
					if(cdloggerVideoDataSize) { \
						if (!(cdloggervdata[addr] & 2)) undefinedvromcount--; \
						rendercount++; \
					} \
				} \
			} \
		} \
}

#define RENDER_LOGP(tmp) { \
		if (debug_loggingCD) \
		{ \
			int addr = GetCHROffset(tmp); \
			if (addr != -1)	\
			{ \
				if (!(cdloggervdata[addr] & 1))	\
				{ \
					cdloggervdata[addr] |= 1; \
					if(cdloggerVideoDataSize) { \
						if (!(cdloggervdata[addr] & 2)) undefinedvromcount--; \
						rendercount++; \
					} \
				} \
			} \
		} \
}

uint8 FASTCALL FFCEUX_PPURead_Default(uint32 A) {
	uint32 tmp = A;

	if (PPU_hook) PPU_hook(A);

	if (tmp < 0x2000) {
		if ((tmp & 0x3FF) < (0x400 - 64))
			FCEUX11_PREFETCH(&VPage[tmp >> 10][tmp + 64]);
		return VPage[tmp >> 10][tmp];
	} else if (tmp < 0x3F00) {
		return vnapage[(tmp >> 10) & 0x3][tmp & 0x3FF];
	} else {
		uint8 ret;
		if (!(tmp & 3)) {
			if (!(tmp & 0xC))
				ret = READPAL(0x00);
			else
				ret = READUPAL(((tmp & 0xC) >> 2) - 1);
		} else
			ret = READPAL(tmp & 0x1F);
		return ret;
	}
}


uint8 (FASTCALL *FFCEUX_PPURead)(uint32 A) = 0;
void (*FFCEUX_PPUWrite)(uint32 A, uint8 V) = 0;

// Stage-2 §九 L4: the missing NULL check here is deliberate (fail-fast — a NULL
// FFCEUX_PPURead means PPU_ResetHooks() was skipped, which is a lifecycle bug,
// not a recoverable condition). The assert turns "crash on a NULL call" into
// "crash with a diagnostic". Under NDEBUG assert() expands to ((void)0), so the
// comma expression collapses to the original call — zero Release cost.
#define CALL_PPUREAD(A) (assert(FFCEUX_PPURead != nullptr), FFCEUX_PPURead(A))

#define CALL_PPUWRITE(A, V) (FFCEUX_PPUWrite ? FFCEUX_PPUWrite(A, V) : FFCEUX_PPUWrite_Default(A, V))

//whether to use the new ppu
int newppu = 0;

void ppu_getScroll(int &xpos, int &ypos) {
	if (newppu) {
		ypos = ppur._vt * 8 + ppur._fv + ppur._v * 256;
		xpos = ppur._ht * 8 + ppur.fh + ppur._h * 256;
	} else {
		xpos = ((RefreshAddr & 0x400) >> 2) | ((RefreshAddr & 0x1F) << 3) | XOffset;

		ypos = ((RefreshAddr & 0x3E0) >> 2) | ((RefreshAddr & 0x7000) >> 12);
		if (RefreshAddr & 0x800) ypos += 240;
	}
}
//---------------

// Step 1.2 ($2002 VBL-set suppression, 2026-08-01): file-local marker set
// by A2002 when a read lands 1 PPU dot before the working-config VBL set
// boundary (sl240, cycle340), consumed by the new-PPU VBL block in
// ppu_rendering.cpp. Savestate-neutral (not serialized; re-derived each
// frame). See docs/history/e1_survey/vbl_step1_1_falsification_2026-08-01.md
// and docs/history/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md Step 1.2.
static bool g_vbl_set_suppressed = false;

void fceu11_ppu_mark_vbl_set_suppressed() {
	g_vbl_set_suppressed = true;
}

bool fceu11_ppu_take_vbl_set_suppressed() {
	bool r = g_vbl_set_suppressed;
	g_vbl_set_suppressed = false;
	return r;
}

// E-1 probe (Step 1.2, 2026-08-01): env-gated $2002 read dot recorder.
// Filters to reads near the VBL set boundary to keep output small
// (vbl tests otherwise do ~10k reads/sec). Same env var as ppu_rendering.
static bool e1_ppu_trace_on() {
	static const bool on = []() {
		const char* e = std::getenv("FCEUX11_E1_TRACE");
		return e && e[0] == '1' && e[1] == '\0';
	}();
	return on;
}

static DECLFR(A2002) {
	if (newppu) [[unlikely]] {
		//once we thought we clear latches here, but that caused midframe glitches.
		//i think we should only reset the state machine for 2005/2006
		//ppur.clear_latches();
	}

	uint8 ret;

	FCEUPPU_LineUpdate();

	// Step 1.2 ($2002 VBL-set suppression, 2026-08-01): newppu only.
	// Working-config VBL flag sets at the sl240→sl241 boundary (cycle 0 of
	// the VBL block). Per NESdev PPU_frame_timing:
	//  - read 1 PPU dot before the set (sl240, cycle340) → suppress the
	//    flag set + NMI entirely for this frame
	//  - read at the set dot or 1 dot after (sl241, cycle0-1) → reads as
	//    set, clears it, and suppresses the NMI (read pulls /NMI back up
	//    before the CPU samples it)
	// Reads ≥2 dots away behave normally.
	if (newppu) {
		const int rsl = ppur.status.sl;
		const int rcy = ppur.status.cycle;
		if (e1_ppu_trace_on()) {
			fprintf(stderr, "E1 P2002_READ abs=%llu sl=%d cycle=%d\n",
			 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()), rsl, rcy);
		}
		if (rsl == 240 && rcy == 340) {
			fceu11_ppu_mark_vbl_set_suppressed();
		} else if (rsl == 241 && rcy <= 1) {
			X6502_IRQEnd(FCEU_IQNMI);  // cancel pending VBL NMI
		}
	}

	ret = PPU_status;
	ret |= PPUGenLatch & 0x1F;

#ifdef FCEUDEF_DEBUGGER
	if (!fceuindbg)
#endif
	{
		vtoggle = 0;
		PPU_status &= 0x7F;
		PPUGenLatch = ret;
	}

	return ret;
}

static DECLFR(A2004) {
	if (newppu) [[unlikely]] {
		if ((ppur.status.sl < 241) && PPUON) {
			// from cycles 0 to 63, the
			// 32 byte OAM buffer gets init
			// to 0xFF
			if (ppur.status.cycle < 64)
				return spr_read.ret = 0xFF;
			else {
				for (int i = spr_read.last;
					 i != ppur.status.cycle; ++i) {
					if (i < 256) {
						switch (spr_read.mode) {
						case 0:
							if (spr_read.count < 2)
								spr_read.ret = (PPU[3] & 0xF8) + (spr_read.count << 2);
							else
								spr_read.ret = spr_read.count << 2;

							spr_read.found_pos[spr_read.found] = spr_read.ret;
							spr_read.ret = SPRAM[spr_read.ret];

							if (i & 1) {
								//odd cycle
								//see if in range
								if (!((ppur.status.sl - 1 - spr_read.ret) & ~(Sprite16 ? 0xF : 0x7))) {
									++spr_read.found;
									spr_read.fetch = 1;
									spr_read.mode = 1;
								} else {
									if (++spr_read.count == 64) {
										spr_read.mode = 4;
										spr_read.count = 0;
									} else if (spr_read.found == 8) {
										spr_read.fetch = 0;
										spr_read.mode = 2;
									}
								}
							}
							break;
						case 1:	//sprite is in range fetch next 3 bytes
							if (i & 1) {
								++spr_read.fetch;
								if (spr_read.fetch == 4) {
									spr_read.fetch = 1;
									if (++spr_read.count == 64) {
										spr_read.count = 0;
										spr_read.mode = 4;
									} else if (spr_read.found == 8) {
										spr_read.fetch = 0;
										spr_read.mode = 2;
									} else
										spr_read.mode = 0;
								}
							}

							if (spr_read.count < 2)
								spr_read.ret = (PPU[3] & 0xF8) + (spr_read.count << 2);
							else
								spr_read.ret = spr_read.count << 2;

							spr_read.ret = SPRAM[spr_read.ret | spr_read.fetch];
							break;
						case 2:	//8th sprite fetched
							spr_read.ret = SPRAM[(spr_read.count << 2) | spr_read.fetch];
							if (i & 1) {
								if (!((ppur.status.sl - 1 - SPRAM[((spr_read.count << 2) | spr_read.fetch)]) & ~((Sprite16) ? 0xF : 0x7))) {
									spr_read.fetch = 1;
									spr_read.mode = 3;
								} else {
									if (++spr_read.count == 64) {
										spr_read.count = 0;
										spr_read.mode = 4;
									}
									spr_read.fetch =
										(spr_read.fetch + 1) & 3;
								}
							}
							spr_read.ret = spr_read.count;
							break;
						case 3:	//9th sprite overflow detected
							spr_read.ret = SPRAM[spr_read.count | spr_read.fetch];
							if (i & 1) {
								if (++spr_read.fetch == 4) {
									spr_read.count = (spr_read.count + 1) & 63;
									spr_read.mode = 4;
								}
							}
							break;
						case 4:	//read OAM[n][0] until hblank
							if (i & 1)
								spr_read.count = (spr_read.count + 1) & 63;
							spr_read.fetch = 0;
							spr_read.ret = SPRAM[spr_read.count << 2];
							break;
						}
					} else if (i < 320) {
						spr_read.ret = (i & 0x38) >> 3;
						if (spr_read.found < (spr_read.ret + 1)) {
							if (spr_read.num) {
								spr_read.ret = SPRAM[252];
								spr_read.num = 0;
							} else
								spr_read.ret = 0xFF;
						} else if ((i & 7) < 4) {
							spr_read.ret =
								SPRAM[spr_read.found_pos[spr_read.ret] | spr_read.fetch++];
							if (spr_read.fetch == 4)
								spr_read.fetch = 0;
						} else
							spr_read.ret = SPRAM[spr_read.found_pos [spr_read.ret | 3]];
					} else {
						if (!spr_read.found)
							spr_read.ret = SPRAM[252];
						else
							spr_read.ret = SPRAM[spr_read.found_pos[0]];
						break;
					}
				}
				spr_read.last = ppur.status.cycle;
				return spr_read.ret;
			}
		} else
			return SPRAM[PPU[3]];
	} else {
		FCEUPPU_LineUpdate();
		return PPUGenLatch;
	}
}

static DECLFR(A200x) {	/* Not correct for $2004 reads. */
	FCEUPPU_LineUpdate();
	return PPUGenLatch;
}

static DECLFR(A2007) {
	uint8 ret;
	uint32 tmp = RefreshAddr & 0x3FFF;

	if (debug_loggingCD) {
		if (!DummyRead && (LogAddress != -1)) {
			if (!(cdloggervdata[LogAddress] & 2)) {
				cdloggervdata[LogAddress] |= 2;
				if ((!(cdloggervdata[LogAddress] & 1)) && cdloggerVideoDataSize) undefinedvromcount--;
				vromreadcount++;
			}
		} else
			DummyRead = 0;
	}

	if (newppu) [[unlikely]] {
		ret = VRAMBuffer;
		RefreshAddr = ppur.get_2007access() & 0x3FFF;
		if ((RefreshAddr & 0x3F00) == 0x3F00) {
			//if it is in the palette range bypass the
			//delayed read, and what gets filled in the temp
			//buffer is the address - 0x1000, also
			//if grayscale is set then the return is AND with 0x30
			//to get a gray color reading
			if (!(tmp & 3)) {
				if (!(tmp & 0xC))
					ret = READPAL(0x00);
				else
					ret = READUPAL(((tmp & 0xC) >> 2) - 1);
			} else
				ret = READPAL(tmp & 0x1F);
			VRAMBuffer = CALL_PPUREAD(RefreshAddr - 0x1000);
		} else {
			if (debug_loggingCD && (RefreshAddr < 0x2000))
				LogAddress = GetCHRAddress(RefreshAddr);
			VRAMBuffer = CALL_PPUREAD(RefreshAddr);
		}
		ppur.increment2007(ppur.status.sl >= 0 && ppur.status.sl < 241 && PPUON, INC32 != 0);
		RefreshAddr = ppur.get_2007access();
		return ret;
	} else {

		//OLDPPU
		FCEUPPU_LineUpdate();

		if (tmp >= 0x3F00) {	// Palette RAM tied directly to the output data, without VRAM buffer
			if (!(tmp & 3)) {
				if (!(tmp & 0xC))
					ret = READPAL(0x00);
				else
					ret = READUPAL(((tmp & 0xC) >> 2) - 1);
			} else
				ret = READPAL(tmp & 0x1F);
			#ifdef FCEUDEF_DEBUGGER
			if (!fceuindbg)
			#endif
			{
				if ((tmp - 0x1000) < 0x2000)
					VRAMBuffer = VPage[(tmp - 0x1000) >> 10][tmp - 0x1000];
				else
					VRAMBuffer = vnapage[((tmp - 0x1000) >> 10) & 0x3][(tmp - 0x1000) & 0x3FF];
				if (PPU_hook) PPU_hook(tmp);
			}
		} else {
			ret = VRAMBuffer;
			#ifdef FCEUDEF_DEBUGGER
			if (!fceuindbg)
			#endif
			{
				if (PPU_hook) PPU_hook(tmp);
				PPUGenLatch = VRAMBuffer;
				if (tmp < 0x2000) {

					if (debug_loggingCD)
						LogAddress = GetCHRAddress(tmp);
					if(MMC5Hack && newppu)
						VRAMBuffer = *MMC5BGVRAMADR(tmp);
					else
						VRAMBuffer = VPage[tmp >> 10][tmp];

				} else if (tmp < 0x3F00)
					VRAMBuffer = vnapage[(tmp >> 10) & 0x3][tmp & 0x3FF];
			}
		}

	#ifdef FCEUDEF_DEBUGGER
		if (!fceuindbg)
	#endif
		{
			if ((ScreenON || SpriteON) && (g_cpu.scanline_ref() < 240)) {
				uint32 rad = RefreshAddr;
				if ((rad & 0x7000) == 0x7000) {
					rad ^= 0x7000;
					if ((rad & 0x3E0) == 0x3A0)
						rad ^= 0xBA0;
					else if ((rad & 0x3E0) == 0x3e0)
						rad ^= 0x3e0;
					else
						rad += 0x20;
				} else
					rad += 0x1000;
				RefreshAddr = rad;
			} else {
				if (INC32)
					RefreshAddr += 32;
				else
					RefreshAddr++;
			}
			if (PPU_hook) PPU_hook(RefreshAddr & 0x3fff);
		}
		return ret;
	}
}

static DECLFW(B2000) {
	FCEUPPU_LineUpdate();
	PPUGenLatch = V;

	if (e1_ppu_trace_on())
		fprintf(stderr, "E1 W2000 abs=%llu V=0x%02X old=0x%02X vbl=%d\n",
		 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
		 (unsigned)V, (unsigned)PPU[0], (int)((PPU_status & 0x80) != 0));

	if (!(PPU[0] & 0x80) && (V & 0x80) && (PPU_status & 0x80))
		TriggerNMI2();

	PPU[0] = V;
	TempAddr &= 0xF3FF;
	TempAddr |= (V & 3) << 10;

	ppur._h = V & 1;
	ppur._v = (V >> 1) & 1;
	ppur.s = (V >> 4) & 1;
}

static DECLFW(B2001) {
	FCEUPPU_LineUpdate();
	// E-1 probe (Phase 1 Step 1.4, 2026-08-03): $2001 (PPUMASK) write dot
	// recorder. PPUON = PPU[1]&0x18 (BG/sprite enable) takes effect
	// immediately here; abs is in CPU cycles (1 cyc = 3 dots) and count is
	// the CPU budget residual in 1/16-dot units, which resolves the write's
	// sub-dot position within its instruction batch.
	if (e1_ppu_trace_on()) {
		fprintf(stderr, "E1 W2001 abs=%llu sl=%d cycle=%d count=%d val=0x%02X PPUON_after=%d\n",
		 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
		 ppur.status.sl, ppur.status.cycle, g_cpu.native_layout().count, V, (V & 0x18) ? 1 : 0);
	}
	if (paldeemphswap)
		V = (V&0x9F)|((V&0x40)>>1)|((V&0x20)<<1);
	PPUGenLatch = V;
	PPU[1] = V;
	if (V & 0xE0)
		deemp = V >> 5;
}

static DECLFW(B2002) {
	PPUGenLatch = V;
}

static DECLFW(B2003) {
	PPUGenLatch = V;
	PPU[3] = V;
	PPUSPL = V & 0x7;
}

static DECLFW(B2004) {
	PPUGenLatch = V;
	if (newppu) [[unlikely]] {
		//the attribute upper bits are not connected
		//so AND them out on write, since reading them
		//should return 0 in those bits.
		if ((PPU[3] & 3) == 2)
			V &= 0xE3;
		SPRAM[PPU[3]] = V;
		PPU[3] = (PPU[3] + 1) & 0xFF;
	} else {
		if (PPUSPL >= 8) {
			if (PPU[3] >= 8)
				SPRAM[PPU[3]] = V;
		} else {
			SPRAM[PPUSPL] = V;
		}
		PPU[3]++;
		PPUSPL++;
	}
}

static DECLFW(B2005) {
	uint32 tmp = TempAddr;
	FCEUPPU_LineUpdate();
	PPUGenLatch = V;
	if (!vtoggle) {
		tmp &= 0xFFE0;
		tmp |= V >> 3;
		XOffset = V & 7;
		ppur._ht = V >> 3;
		ppur.fh = V & 7;
	} else {
		tmp &= 0x8C1F;
		tmp |= ((V & ~0x7) << 2);
		tmp |= (V & 7) << 12;
		ppur._vt = V >> 3;
		ppur._fv = V & 7;
	}
	TempAddr = tmp;
	vtoggle ^= 1;
}


static DECLFW(B2006) {
	FCEUPPU_LineUpdate();

	PPUGenLatch = V;
	if (!vtoggle) {
		TempAddr &= 0x00FF;
		TempAddr |= (V & 0x3f) << 8;

		ppur._vt &= 0x07;
		ppur._vt |= (V & 0x3) << 3;
		ppur._h = (V >> 2) & 1;
		ppur._v = (V >> 3) & 1;
		ppur._fv = (V >> 4) & 3;
	} else {
		TempAddr &= 0xFF00;
		TempAddr |= V;

		RefreshAddr = TempAddr;
		DummyRead = 1;
		if (PPU_hook)
			PPU_hook(RefreshAddr);

		ppur._vt &= 0x18;
		ppur._vt |= (V >> 5);
		ppur._ht = V & 31;

		ppur.install_latches();
	}

	vtoggle ^= 1;
}

static DECLFW(B2007) {
	uint32 tmp = RefreshAddr & 0x3FFF;

	if (debug_loggingCD) {
		if(!cdloggerVideoDataSize && (tmp < 0x2000))
			cdloggervdata[tmp] = 0;
	}

	if (newppu) {
		PPUGenLatch = V;
		RefreshAddr = ppur.get_2007access() & 0x3FFF;
		CALL_PPUWRITE(RefreshAddr, V);
		ppur.increment2007(ppur.status.sl >= 0 && ppur.status.sl < 241 && PPUON, INC32 != 0);
		RefreshAddr = ppur.get_2007access();
	} else {
		PPUGenLatch = V;
		if (tmp < 0x2000) {
			if (PPUCHRRAM & (1 << (tmp >> 10)))
				VPage[tmp >> 10][tmp] = V;
		} else if (tmp < 0x3F00) {
			if (QTAIHack && (qtaintramreg & 1)) {
				QTAINTRAM[((((tmp & 0xF00) >> 10) >> ((qtaintramreg >> 1)) & 1) << 10) | (tmp & 0x3FF)] = V;
			} else {
				if (PPUNTARAM & (1 << ((tmp & 0xF00) >> 10)))
					vnapage[((tmp & 0xF00) >> 10)][tmp & 0x3FF] = V;
			}
		} else {
			if (!(tmp & 3)) {
				if (!(tmp & 0xC))
					PALRAM[0x00] = PALRAM[0x04] = PALRAM[0x08] = PALRAM[0x0C] = V & 0x3F;
				else
					UPALRAM[((tmp & 0xC) >> 2) - 1] = V & 0x3F;
			} else
				PALRAM[tmp & 0x1F] = V & 0x3F;
		}
		if (INC32)
			RefreshAddr += 32;
		else
			RefreshAddr++;
		if (PPU_hook)
			PPU_hook(RefreshAddr & 0x3fff);
	}
}

static DECLFW(B4014) {
	uint32 t = V << 8;
	int x;

	for (x = 0; x < 256; x++)
		X6502_DMW(0x2004, X6502_DMR(t + x));
	SpriteDMA = V;
}

// v1.13 Purify Phase A Batch C: Pline/Plinef/firsttile/tofix/rendersprites/
// renderbg/sphitx/sphitdata/spork/linestartts + PAL/GETLASTPIXEL macros
// moved to ppu_rendering.cpp.

// v1.13 Purify Phase A Batch C: ResetRL moved to ppu_rendering.cpp.

void FCEUPPU_Power(void) {
	int x;

	// initialize PPU memory regions according to settings
	FCEU_MemoryRand(NTARAM, 0x800, true);
	FCEU_MemoryRand(PALRAM.data(), 0x20, true);
	FCEU_MemoryRand(SPRAM, 0x100, true);
	// palettes can only store values up to $3F, and PALRAM X4/X8/XC are mirrors of X0 for rendering purposes (UPALRAM is used for $2007 readback)
	for (x = 0; x < 0x20; ++x) PALRAM[x] &= 0x3F;
	UPALRAM[0] = PALRAM[0x04];
	UPALRAM[1] = PALRAM[0x08];
	UPALRAM[2] = PALRAM[0x0C];
	PALRAM[0x0C] = PALRAM[0x08] = PALRAM[0x04] = PALRAM[0x00];
	PALRAM[0x1C] = PALRAM[0x18] = PALRAM[0x14] = PALRAM[0x10];
	// Restore the default PPU read/write hooks before resetting. This is
	// essential for the new-PPU rendering path: CALL_PPUREAD dereferences
	// FFCEUX_PPURead without a NULL guard, and the only other writer of
	// this pointer besides PPU_ResetHooks is ResetGameLoaded() (which
	// NULLs it on every LoadGame). Without this call, the first
	// bgdata::Record::Read() of the first rendered frame calls a NULL
	// function pointer and crashes. Board Power() handlers (e.g. MMC5)
	// run after FCEUPPU_Power via GameInterface(GI_POWER) and may
	// override the hook themselves.
	PPU_ResetHooks();
	FCEUPPU_Reset();

	for (x = 0x2000; x < 0x4000; x += 8) {
		ARead[x] = A200x;
		BWrite[x] = B2000;
		ARead[x + 1] = A200x;
		BWrite[x + 1] = B2001;
		ARead[x + 2] = A2002;
		BWrite[x + 2] = B2002;
		ARead[x + 3] = A200x;
		BWrite[x + 3] = B2003;
		ARead[x + 4] = A2004;
		BWrite[x + 4] = B2004;
		ARead[x + 5] = A200x;
		BWrite[x + 5] = B2005;
		ARead[x + 6] = A200x;
		BWrite[x + 6] = B2006;
		ARead[x + 7] = A2007;
		BWrite[x + 7] = B2007;
	}
	BWrite[0x4014] = B4014;
}

void FCEUPPU_Init(void) {
	makeppulut();
}
