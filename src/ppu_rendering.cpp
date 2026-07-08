// ppu_rendering.cpp
//
// v1.13 Purify Phase A: PPU rendering pipeline split.
//
// This translation unit hosts the PPU rendering pipeline:
//   - Background tile fetch + MMC5 hack variants
//   - Sprite evaluation (FetchSpriteData / RefreshSprites / CopySprites)
//   - Per-scanline compositing (RefreshLine / DoLine / EndRL /
//     CheckSpriteHit / ResetRL / FCEUPPU_LineUpdate / Fixit1 / Fixit2)
//   - Old-PPU main loop (FCEUPPU_Loop + PPU_MASTER)
//   - New-PPU main loop (FCEUX_PPU_Loop)
//   - Hot helpers (runppu / BGData / bgdata / PaletteAdjustPixel)
//   - Render-plane toggles (fceu11::SetRenderPlanes / GetRenderPlanes)
//   - Palette lookup tables (ppulut1/2/3 + makeppulut)
//   - BITREVLUT template + bitrevlut
//   - File-static rendering globals (Pline/Plinef/firsttile/tofix,
//     sphitx/sphitdata/spork, rendersprites/renderbg, numsprites/
//     SpriteBlurp, deemp/deempcnt, maxsprites, kLineTime/kFetchTime,
//     pputime/totpputime, framectr)
//
// Phase A activates this TU in the build (src/CMakeLists.txt SRC_CORE).
// Subsequent Phase A batches (B / C / D) incrementally move the code
// from ppu.cpp into this file. Phase E enforces the ≤800-line gate
// on ppu.cpp.

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
#include "ppu_core.h"
#include "ppu_class.h"
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
#include <cstring>
#include <cstdio>
#include <cstdlib>

// Phase A activation only: this TU is registered in the build with a
// valid include graph but no function definitions yet. All rendering
// bodies remain in ppu.cpp. Batches B/C/D incrementally move bodies
// here and adjust the matching extern decls in ppu_rendering.h.

// ----------------------------------------------------------------------------
// v1.13 Phase A Batch B: palette LUTs + makeppulut
// ----------------------------------------------------------------------------
// ppulut1/2/3 are file-static in v1.0 ppu.cpp (lines 75-77). RefreshLine
// reads them via pputile.inc (#include at line 1050/1052 etc.); RefreshSprites
// reads ppulut1/2 directly (line 1459). Both move to ppu_rendering.cpp
// in Batch C/D; for now they are extern in ppu_rendering.h so the
// existing ppu.cpp callers keep compiling.
alignas(64) std::array<uint32, 256> ppulut1;
alignas(64) std::array<uint32, 256> ppulut2;
alignas(64) std::array<uint32, 128> ppulut3;

// makeppulut populates ppulut1/2/3 once at FCEUPPU_Init time. Pure
// code move from ppu.cpp lines 127-151.
void makeppulut(void) {
	int x;
	int y;
	int cc, xo, pixel;

	for (x = 0; x < 256; x++) {
		ppulut1[x] = 0;
		for (y = 0; y < 8; y++)
			ppulut1[x] |= ((x >> (7 - y)) & 1) << (y * 4);
		ppulut2[x] = ppulut1[x] << 1;
	}

	for (cc = 0; cc < 16; cc++) {
		for (xo = 0; xo < 8; xo++) {
			ppulut3[xo | (cc << 3)] = 0;
			for (pixel = 0; pixel < 8; pixel++) {
				int shiftr;
				shiftr = (pixel + xo) / 8;
				shiftr *= 2;
				ppulut3[xo | (cc << 3)] |= ((cc >> shiftr) & 3) << (2 + pixel * 4);
			}
		}
	}
}

// ----------------------------------------------------------------------------
// v1.13 Phase A Batch C: per-scanline rendering pipeline
// ----------------------------------------------------------------------------
// Pure code move from ppu.cpp lines 856-1258 (ResetRL / FCEUPPU_LineUpdate /
// SetRenderPlanes / GetRenderPlanes / EndRL / CheckSpriteHit / RefreshLine /
// Fixit1 / Fixit2 / DoLine). These functions together with their file-static
// state (Pline/Plinef/firsttile/tofix, rendersprites/renderbg, sphitx/
// sphitdata/spork) form the hot path called from FCEUPPU_Loop. After this
// batch, FCEUPPU_Loop / FetchSpriteData / RefreshSprites / CopySprites
// stay in ppu.cpp for now (Batch D moves them).

// Macros local to this TU. Mirror the ones in ppu.cpp at lines 49-67 / 856
// / 858 so RefreshLine / DoLine / Fixit1 / Fixit2 / EndRL keep the same
// expression expansion. (C preprocessor macros are per-TU; redefining the
// same name in ppu_rendering.cpp is fine and has no effect on ppu.cpp.)
#define VBlankON    (PPU[0] & 0x80)
#define Sprite16    (PPU[0] & 0x20)
#define BGAdrHI     (PPU[0] & 0x10)
#define SpAdrHI     (PPU[0] & 0x08)
#define INC32       (PPU[0] & 0x04)
#define SpriteON    (PPU[1] & 0x10)
#define ScreenON    (PPU[1] & 0x08)
#define PPUON       (PPU[1] & 0x18)
#define GRAYSCALE   (PPU[1] & 0x01)
#define SpriteLeft8 (PPU[1] & 0x04)
#define BGLeft8     (PPU[1] & 0x02)
#define PPU_status  (PPU[2])
#define READPALNOGS(ofs)    (PALRAM[(ofs)])
#define READPAL(ofs)       (PALRAM[(ofs)] & (GRAYSCALE ? 0x30 : 0xFF))
#define READUPAL(ofs)      (UPALRAM[(ofs)] & (GRAYSCALE ? 0x30 : 0xFF))
#define PAL(c)             ((c) + cc)
#define GETLASTPIXEL       (PAL ? ((g_cpu.timestamp_ref() * 48 - linestartts) / 15) : ((g_cpu.timestamp_ref() * 48 - linestartts) >> 4))
// CALL_PPUREAD — ppu.cpp line 401. Same expansion.
#define CALL_PPUREAD(A)    (FFCEUX_PPURead(A))
// CHRptr / VPage / MMC5SPRVPage — declared as reference aliases in
// bus.h (already transitively included via cart.h / memory.h chain).
// VRAMADR / MMC5SPRVRAMADR macros mirror ppu.cpp lines 222-223.
#define MMC5SPRVRAMADR(V)  (&MMC5SPRVPage[(V) >> 10][(V)])
#define VRAMADR(V)         (&VPage[(V) >> 10][(V)])

// RENDER_LOG / RENDER_LOGP — stubbed in non-debug builds; line-for-line
// port of ppu.cpp lines 333-353.
#ifndef FCEUDEF_DEBUGGER
#define RENDER_LOG(tmp)
#define RENDER_LOGP(tmp)
#else
extern volatile int rendercount, vromreadcount, undefinedvromcount;
extern int LogAddress;
#define RENDER_LOG(tmp) {  \
	if (undefinedvromcount) { \
		if (tmp >= 0x2000) { \
			if ((tmp & 0x3000) != 0x2000) \
				undefinedvromcount--; \
		} else { \
			rendercount--; \
			vromreadcount--; \
		} \
	} \
}
#define RENDER_LOGP(tmp) {  \
	if (cdloggervdata) { \
		cdloggerVideoDataSize++; \
	} \
}
#endif

// File-static rendering state. Was file-static in ppu.cpp lines 860-863 /
// 899 / 921-922 / 941; sphitx/sp_hitdata/spork promoted to extern via
// ppu_rendering.h because RefreshSprites (still in ppu.cpp) reads them.
static uint8 *Pline, *Plinef;
static int firsttile;
int linestartts;   // referenced by debugger; non-static
static int tofix = 0;
// rendersprites / renderbg have external linkage (extern in ppu_rendering.h)
// because CopySprites / FCEUPPU_Loop (still in ppu.cpp) read them.
bool rendersprites = true;
bool renderbg = true;
int32_t sphitx = 0x100;
uint8_t sphitdata = 0;
int spork = 0;

static void RefreshLine(int lastpixel);
static void CheckSpriteHit(int p);
static void EndRL(void);
static INLINE void Fixit2(void);
static void Fixit1(void);

// ResetRL is non-static: FCEUPPU_Loop (still in ppu.cpp) calls it.
void ResetRL(uint8 *target) {
	memset(target, 0xFF, 256);
	InputScanlineHook(0, 0, 0, 0);
	Plinef = target;
	Pline = target;
	firsttile = 0;
	linestartts = g_cpu.timestamp_ref() * 48 + g_cpu.native_layout().count;
	tofix = 0;
	FCEUPPU_LineUpdate();
	tofix = 1;
}

void FCEUPPU_LineUpdate(void) {
	if (newppu)
		return;

#ifdef FCEUDEF_DEBUGGER
	if (!fceuindbg)
#endif
	if (Pline) {
		int l = GETLASTPIXEL;
		RefreshLine(l);
	}
}

void fceu11::SetRenderPlanes(bool sprites, bool bg) {
	rendersprites = sprites;
	renderbg = bg;
}

void fceu11::GetRenderPlanes(bool& sprites, bool& bg) {
	sprites = rendersprites;
	bg = renderbg;
}

static void EndRL(void) {
	RefreshLine(272);
	if (tofix)
		Fixit1();
	CheckSpriteHit(272);
	Pline = 0;
}

static void CheckSpriteHit(int p) {
	int l = p - 16;
	int x;

	if (sphitx == 0x100) return;

	for (x = sphitx; x < (sphitx + 8) && x < l; x++) {
		if ((sphitdata & (0x80 >> (x - sphitx))) && !(Plinef[x] & 64) && x < 255) {
			PPU_status |= 0x40;
			sphitx = 0x100;
			break;
		}
	}
}

// lasttile is really "second to last tile."
static void RefreshLine(int lastpixel) {
	// v1.5 Prism §2.2 (Batch 2): pshift[2] / atlatch were static
	// locals here (persistent across scanlines, lifetime = process).
	// They migrate into fceu11::g_ppu.bg_latch_[] / bg_latch_h_; the
	// local aliases below rebind the names without touching
	// pputile.inc (which is included later in this function and uses
	// `pshift[0]`, `pshift[1]`, `atlatch`).
	uint32 (&pshift)[2] = fceu11::g_ppu.bg_latch();
	uint32 &atlatch     = fceu11::g_ppu.bg_latch_h();
	uint8 *sprlinebuf   = fceu11::g_ppu.line_buffer();
	uint32 smorkus = RefreshAddr;

	#define RefreshAddr smorkus
	uint32 vofs;
	int X1;

	uint8 *P = Pline;
	int lasttile = lastpixel >> 3;
	int numtiles;
	static int norecurse = 0;
	if (norecurse) return;

	if (sphitx != 0x100 && !(PPU_status & 0x40)) {
		if ((sphitx < (lastpixel - 16)) && !(sphitx < ((lasttile - 2) * 8)))
			lasttile++;
	}

	if (lasttile > 34) lasttile = 34;
	numtiles = lasttile - firsttile;

	if (numtiles <= 0) return;

	P = Pline;

	vofs = 0;

	if (PEC586Hack)
		vofs = ((RefreshAddr & 0x200) << 3) | ((RefreshAddr >> 12) & 7);
	else
		vofs = ((PPU[0] & 0x10) << 8) | ((RefreshAddr >> 12) & 7);

	if (!ScreenON && !SpriteON) [[unlikely]] {
		uint32 tem;
		tem = READPAL(0) | (READPAL(0) << 8) | (READPAL(0) << 16) | (READPAL(0) << 24);
		tem |= 0x40404040;
		FCEU_dwmemset(Pline, tem, numtiles * 8);
		P += numtiles * 8;
		Pline = P;

		firsttile = lasttile;

		#define TOFIXNUM (272 - 0x4)
		if (lastpixel >= TOFIXNUM && tofix) {
			Fixit1();
			tofix = 0;
		}

		if ((lastpixel - 16) >= 0) {
			InputScanlineHook(Plinef, spork ? sprlinebuf : 0, linestartts, lasttile * 8 - 16);
		}
		return;
	}

	//Priority bits, needed for sprite emulation.
	PALRAM[0] |= 64;
	PALRAM[4] |= 64;
	PALRAM[8] |= 64;
	PALRAM[0xC] |= 64;

	//This high-level graphics MMC5 emulation code was written for MMC5 carts in "CL" mode.
	//It's probably not totally correct for carts in "SL" mode.

#define PPUT_MMC5
	if (MMC5Hack && geniestage != 1) {
		if (MMC5HackCHRMode == 0 && (MMC5HackSPMode & 0x80)) {
			int tochange = MMC5HackSPMode & 0x1F;
			tochange -= firsttile;
			for (X1 = firsttile; X1 < lasttile; X1++) {
				if ((tochange <= 0 && MMC5HackSPMode & 0x40) || (tochange > 0 && !(MMC5HackSPMode & 0x40))) {
					#define PPUT_MMC5SP
					#include "pputile.inc"
					#undef PPUT_MMC5SP
				} else {
					#include "pputile.inc"
				}
				tochange--;
			}
		} else if (MMC5HackCHRMode == 1 && (MMC5HackSPMode & 0x80)) {
			int tochange = MMC5HackSPMode & 0x1F;
			tochange -= firsttile;

			#define PPUT_MMC5SP
			#define PPUT_MMC5CHR1
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
			#undef PPUT_MMC5CHR1
			#undef PPUT_MMC5SP
		} else if (MMC5HackCHRMode == 1) {
			#define PPUT_MMC5CHR1
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
			#undef PPUT_MMC5CHR1
		} else {
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
		}
	}
	#undef PPUT_MMC5
	else if (PPU_hook) {
		norecurse = 1;
		#define PPUT_HOOK
		if (PEC586Hack) {
			#define PPU_BGFETCH
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
			#undef PPU_BGFETCH
		} else {
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
		}
		#undef PPUT_HOOK
		norecurse = 0;
	} else {
		if (PEC586Hack) {
			#define PPU_BGFETCH
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
			#undef PPU_BGFETCH
		} if (QTAIHack) {
			#define PPU_VRC5FETCH
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
			#undef PPU_VRC5FETCH
		} else {
			for (X1 = firsttile; X1 < lasttile; X1++) {
				#include "pputile.inc"
			}
		}
	}

#undef vofs
#undef RefreshAddr

	//Reverse changes made before.
	PALRAM[0] &= 63;
	PALRAM[4] &= 63;
	PALRAM[8] &= 63;
	PALRAM[0xC] &= 63;

	RefreshAddr = smorkus;
	if (firsttile <= 2 && 2 < lasttile && !(PPU[1] & 2)) {
		uint32 tem;
		tem = READPAL(0) | (READPAL(0) << 8) | (READPAL(0) << 16) | (READPAL(0) << 24);
		tem |= 0x40404040;
		*(uint32*)Plinef = *(uint32*)(Plinef + 4) = tem;
	}

	if (!ScreenON) {
		uint32 tem;
		int tstart, tcount;
		tem = READPAL(0) | (READPAL(0) << 8) | (READPAL(0) << 16) | (READPAL(0) << 24);
		tem |= 0x40404040;

		tcount = lasttile - firsttile;
		tstart = firsttile - 2;
		if (tstart < 0) {
			tcount += tstart;
			tstart = 0;
		}
		if (tcount > 0)
			FCEU_dwmemset(Plinef + tstart * 8, tem, tcount * 8);
	}

	if (lastpixel >= TOFIXNUM && tofix) {
		Fixit1();
		tofix = 0;
	}

	//This only works right because of a hack earlier in this function.
	CheckSpriteHit(lastpixel);

	if ((lastpixel - 16) >= 0) {
		InputScanlineHook(Plinef, spork ? sprlinebuf : 0, linestartts, lasttile * 8 - 16);
	}
	Pline = P;
	firsttile = lasttile;
}

static INLINE void Fixit2(void) {
	if (ScreenON || SpriteON) {
		uint32 rad = RefreshAddr;
		rad &= 0xFBE0;
		rad |= TempAddr & 0x041f;
		RefreshAddr = rad;
	}
}

static void Fixit1(void) {
	if (ScreenON || SpriteON) {
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
	}
}

extern void MMC5_hb(int);
// DoLine is non-static: FCEUPPU_Loop (still in ppu.cpp) calls it.
void DoLine(void) {
	if (g_cpu.scanline_ref() >= 240 && g_cpu.scanline_ref() != totalscanlines) {
		X6502_Run(256 + 69);
		g_cpu.scanline_ref()++;
		X6502_Run(16);
		return;
	}

	int x;
	uint8 *target = XBuf + ((g_cpu.scanline_ref() < 240 ? g_cpu.scanline_ref() : 240) << 8);
	u8* dtarget = XDBuf + ((g_cpu.scanline_ref() < 240 ? g_cpu.scanline_ref() : 240) << 8);

	if (MMC5Hack) MMC5_hb(g_cpu.scanline_ref());

	X6502_Run(256);
	EndRL();

	if (!renderbg) {
		uint32 tem;
		uint8 col;
		if (gNoBGFillColor == 0xFF)
			col = READPAL(0);
		else col = gNoBGFillColor;
		tem = col | (col << 8) | (col << 16) | (col << 24);
		tem |= 0x40404040;
		FCEU_dwmemset(target, tem, 256);
	}

	if (SpriteON)
		CopySprites(target);

	if (ScreenON || SpriteON)
	{
		if (PPU[1] & 0x01) {
			for (x = 63; x >= 0; x--)
				*(uint32*)&target[x << 2] = (*(uint32*)&target[x << 2]) & 0x30303030;
		}
	}

	if ((PPU[1] >> 5) == 0x7) {
		for (x = 63; x >= 0; x--)
			*(uint32*)&target[x << 2] = ((*(uint32*)&target[x << 2]) & 0x3f3f3f3f) | 0xc0c0c0c0;
	} else if (PPU[1] & 0xE0)
		for (x = 63; x >= 0; x--)
			*(uint32*)&target[x << 2] = (*(uint32*)&target[x << 2]) | 0x40404040;
	else
		for (x = 63; x >= 0; x--)
			*(uint32*)&target[x << 2] = ((*(uint32*)&target[x << 2]) & 0x3f3f3f3f) | 0x80808080;

	for (x = 63; x >= 0; x--)
		*(uint32*)&dtarget[x << 2] = ((PPU[1]>>5)<<0)|((PPU[1]>>5)<<8)|((PPU[1]>>5)<<16)|((PPU[1]>>5)<<24);

	sphitx = 0x100;

	if (ScreenON || SpriteON)
		FetchSpriteData();

	if (GameHBIRQHook && (ScreenON || SpriteON) && ((PPU[0] & 0x38) != 0x18)) {
		X6502_Run(6);
		Fixit2();
		X6502_Run(4);
		GameHBIRQHook();
		X6502_Run(85 - 16 - 10);
	} else {
		X6502_Run(6);
		Fixit2();
		X6502_Run(85 - 6 - 16);

		if (GameHBIRQHook && (ScreenON || SpriteON) && ((PPU[0] & 0x38) != 0x18))
			GameHBIRQHook();
	}

	DEBUG(FCEUD_UpdateNTView(g_cpu.scanline_ref(), 0));

	if (SpriteON)
		RefreshSprites();
	if (GameHBIRQHook2 && (ScreenON || SpriteON))
		GameHBIRQHook2();
	g_cpu.scanline_ref()++;
	if (g_cpu.scanline_ref() < 240) {
		ResetRL(XBuf + (g_cpu.scanline_ref() << 8));
	}
	X6502_Run(16);
}// ----------------------------------------------------------------------------
// v1.13 Phase A Batch D: sprite evaluation + main loops + new PPU engine
// ----------------------------------------------------------------------------
// Pure code move from ppu.cpp:
//   - BITREVLUT template + bitrevlut instance (line 96-125)
//   - deemp / deempcnt / maxsprites file-static globals
//   - V_FLIP / H_FLIP / SP_BACK macros + SPR / SPRB structs (line 865-875)
//   - FetchSpriteData / RefreshSprites / CopySprites (line 877-1158)
//   - FCEUI_DisableSpriteLimitation
//   - FCEUPPU_Loop (line 1205-1349) + PPU_MASTER pointer
//   - pputime / totpputime / kLineTime / kFetchTime
//   - runppu + BGData + bgdata
//   - PaletteAdjustPixel
//   - FCEUX_PPU_Loop (line 1466-end) + framectr + int test

// BITREVLUT template + bitrevlut instance (used by FCEUX_PPU_Loop).
// Also dead `int test` (was at ppu.cpp line 96; kept as byte-compatible
// placeholder per v1.13 §1.4 future-scope list).
int test = 0;

template<typename T, int BITS>
struct BITREVLUT {
	T* lut;
	BITREVLUT() {
		int bits = BITS;
		int n = 1 << BITS;
		lut = new T[n];

		int m = 1;
		int a = n >> 1;
		int j = 2;

		lut[0] = 0;
		lut[1] = a;

		while (--bits) {
			m <<= 1;
			a >>= 1;
			for (int i = 0; i < m; i++)
				lut[j++] = lut[i] + a;
		}
	}

	T operator[](int index) {
		return lut[index];
	}
};
BITREVLUT<uint8, 8> bitrevlut;

// V_FLIP / H_FLIP / SP_BACK macros + SPR / SPRB structs (used by
// FetchSpriteData / RefreshSprites / CopySprites).
#define V_FLIP  0x80
#define H_FLIP  0x40
#define SP_BACK 0x20

typedef struct {
	uint8 y, no, atr, x;
} SPR;

typedef struct {
	uint8 ca[2], atr, x;
} SPRB;

// File-static sprite evaluation state.
// deemp is also externed via ppu_rendering.h (B2001 in ppu.cpp writes
// it via the early-update path), so the definition here is non-static.
// deempcnt / maxsprites / numsprites / SpriteBlurp stay file-static
// because they are only read/written inside the rendering TU.
static int deempcnt[8];
static int maxsprites = 8;
static uint8 numsprites, SpriteBlurp;
uint8_t deemp = 0;  // extern via ppu_rendering.h; non-static due to B2001

// Public sprite-limiter toggle. Definition migrated from ppu.cpp line 877.
void FCEUI_DisableSpriteLimitation(int a) {
	maxsprites = a ? 64 : 8;
}

// ----------------------------------------------------------------------------
// Sprite evaluation pipeline (called from FCEUPPU_Loop / DoLine)
// ----------------------------------------------------------------------------
void FetchSpriteData(void) {
	uint8 ns, sb;
	SPR *spr;
	uint8 H;
	int n;
	int vofs;
	uint8 P0 = PPU[0];

	spr = (SPR*)SPRAM;
	H = 8;

	ns = sb = 0;

	vofs = (uint32)(P0 & 0x8 & (((P0 & 0x20) ^ 0x20) >> 2)) << 9;
	H += (P0 & 0x20) >> 2;

	if (!PPU_hook)
		for (n = 63; n >= 0; n--, spr++) {
			if ((uint32)(g_cpu.scanline_ref() - spr->y) >= H) continue;
			if (ns < maxsprites) {
				if (n == 63) sb = 1;

				{
					SPRB dst;
					uint8 *C;
					int t;
					uint32 vadr;

					t = (int)g_cpu.scanline_ref() - (spr->y);

					if (Sprite16)
						vadr = ((spr->no & 1) << 12) + ((spr->no & 0xFE) << 4);
					else
						vadr = (spr->no << 4) + vofs;

					if (spr->atr & V_FLIP) {
						vadr += 7;
						vadr -= t;
						vadr += (P0 & 0x20) >> 1;
						vadr -= t & 8;
					} else {
						vadr += t;
						vadr += t & 8;
					}

					/* Fix this geniestage hack */
					if (MMC5Hack && geniestage != 1)
						C = MMC5SPRVRAMADR(vadr);
					else
						C = VRAMADR(vadr);

					if (SpriteON)
						RENDER_LOGP(C);
					dst.ca[0] = C[0];
					if (SpriteON)
						RENDER_LOGP(C + 8);
					dst.ca[1] = C[8];
					dst.x = spr->x;
					dst.atr = spr->atr;

					*(uint32*)&SPRBUF[ns << 2] = *(uint32*)&dst;
				}

				ns++;
			} else {
				PPU_status |= 0x20;
				break;
			}
		}
	else
		for (n = 63; n >= 0; n--, spr++) {
			if ((uint32)(g_cpu.scanline_ref() - spr->y) >= H) continue;

			if (ns < maxsprites) {
				if (n == 63) sb = 1;

				{
					SPRB dst;
					uint8 *C;
					int t;
					uint32 vadr;

					t = (int)g_cpu.scanline_ref() - (spr->y);

					if (Sprite16)
						vadr = ((spr->no & 1) << 12) + ((spr->no & 0xFE) << 4);
					else
						vadr = (spr->no << 4) + vofs;

					if (spr->atr & V_FLIP) {
						vadr += 7;
						vadr -= t;
						vadr += (P0 & 0x20) >> 1;
						vadr -= t & 8;
					} else {
						vadr += t;
						vadr += t & 8;
					}

					if (MMC5Hack)
						C = MMC5SPRVRAMADR(vadr);
					else
						C = VRAMADR(vadr);
					if (SpriteON)
						RENDER_LOGP(C);
					dst.ca[0] = C[0];
					if (ns < 8) {
						PPU_hook(0x2000);
						PPU_hook(vadr);
					}
					if (SpriteON)
						RENDER_LOGP(C + 8);
					dst.ca[1] = C[8];
					dst.x = spr->x;
					dst.atr = spr->atr;


					*(uint32*)&SPRBUF[ns << 2] = *(uint32*)&dst;
				}

				ns++;
			} else {
				PPU_status |= 0x20;
				break;
			}
		}

	//Handle case when >8 sprites per scanline option is enabled.
	if (ns > 8) PPU_status |= 0x20;
	else if (PPU_hook) {
		for (n = 0; n < (8 - ns); n++) {
			PPU_hook(0x2000);
			PPU_hook(vofs);
		}
	}
	numsprites = ns;
	SpriteBlurp = sb;
}

void RefreshSprites(void) {
	// v1.5 Prism §2.2 (Batch 2): sprlinebuf now aliases g_ppu.line_buffer().
	uint8 *sprlinebuf = fceu11::g_ppu.line_buffer();
	int n;
	SPRB *spr;

	spork = 0;
	if (!numsprites) return;

	FCEU_dwmemset(sprlinebuf, 0x80808080, 256);
	numsprites--;
	spr = (SPRB*)SPRBUF + numsprites;

	for (n = numsprites; n >= 0; n--, spr--) {
		uint32 pixdata;
		uint8 J, atr;

		int x = spr->x;
		uint8 *C;
		int VB;

		pixdata = ppulut1[spr->ca[0]] | ppulut2[spr->ca[1]];
		J = spr->ca[0] | spr->ca[1];
		atr = spr->atr;

		if (J) {
			if (n == 0 && SpriteBlurp && !(PPU_status & 0x40)) {
				sphitx = x;
				sphitdata = J;
				if (atr & H_FLIP)
					sphitdata = ((J << 7) & 0x80) |
								((J << 5) & 0x40) |
								((J << 3) & 0x20) |
								((J << 1) & 0x10) |
								((J >> 1) & 0x08) |
								((J >> 3) & 0x04) |
								((J >> 5) & 0x02) |
								((J >> 7) & 0x01);
			}

			C = sprlinebuf + x;
			VB = (0x10) + ((atr & 3) << 2);

			if (atr & SP_BACK) {
				if (atr & H_FLIP) {
					if (J & 0x80) C[7] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x40) C[6] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x20) C[5] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x10) C[4] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x08) C[3] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x04) C[2] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x02) C[1] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x01) C[0] = READPAL(VB | pixdata) | 0x40;
				} else {
					if (J & 0x80) C[0] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x40) C[1] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x20) C[2] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x10) C[3] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x08) C[4] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x04) C[5] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x02) C[6] = READPAL(VB | (pixdata & 3)) | 0x40;
					pixdata >>= 4;
					if (J & 0x01) C[7] = READPAL(VB | pixdata) | 0x40;
				}
			} else {
				if (atr & H_FLIP) {
					if (J & 0x80) C[7] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x40) C[6] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x20) C[5] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x10) C[4] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x08) C[3] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x04) C[2] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x02) C[1] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x01) C[0] = READPAL(VB | pixdata);
				} else {
					if (J & 0x80) C[0] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x40) C[1] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x20) C[2] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x10) C[3] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x08) C[4] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x04) C[5] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x02) C[6] = READPAL(VB | (pixdata & 3));
					pixdata >>= 4;
					if (J & 0x01) C[7] = READPAL(VB | pixdata);
				}
			}
		}
	}
	SpriteBlurp = 0;
	spork = 1;
}

void CopySprites(uint8 *target) {
	// v1.5 Prism §2.2 (Batch 2): sprlinebuf now aliases g_ppu.line_buffer().
	uint8 *sprlinebuf = fceu11::g_ppu.line_buffer();
	uint8 *P = target;

	if (!spork) return;
	spork = 0;

	if (!rendersprites) return;	//User asked to not display sprites.

	if(!SpriteON) return;
	
	int start=8;
	if(PPU[1] & 0x04)
		start = 0;

	for(int i=start;i<256;i++)
	{
		uint8 t = sprlinebuf[i];
		if(!(t&0x80))
			if (!(t & 0x40) || (P[i] & 0x40))		// Normal sprite || behind bg sprite
				P[i] = t;
	}
}

// ----------------------------------------------------------------------------
// Old-PPU main loop (called from fceu.cpp frame driver)
// ----------------------------------------------------------------------------
int FCEUPPU_Loop(int skip) {
	if ((newppu) && (GameInfo->type != GIT_NSF)) [[unlikely]] {
		int FCEUX_PPU_Loop(int skip);
		return FCEUX_PPU_Loop(skip);
	}

	//Needed for Knight Rider, possibly others.
	if (ppudead) {
		memset(XBuf, 0x80, 256 * 240);
		X6502_Run(scanlines_per_frame * (256 + 85));
		ppudead--;
	} else {
		X6502_Run(256 + 85);
		PPU_status |= 0x80;

		//Not sure if this is correct.  According to Matt Conte and my own tests, it is.
		//Timing is probably off, though.
		//NOTE:  Not having this here breaks a Super Donkey Kong game.
		PPU[3] = PPUSPL = 0;

		//I need to figure out the true nature and length of this delay.
		X6502_Run(12);
		if (GameInfo->type == GIT_NSF)
			DoNSFFrame();
		else {
			if (VBlankON)
				TriggerNMI();
		}
		X6502_Run((scanlines_per_frame - 242) * (256 + 85) - 12);
		if (overclock_enabled && vblankscanlines) {
			if (!DMC_7bit || !skip_7bit_overclocking) {
				g_cpu.set_overclocking(true);
				X6502_Run(vblankscanlines * (256 + 85) - 12);
				g_cpu.set_overclocking(false);
			}
		}
		PPU_status &= 0x1f;
		X6502_Run(256);

		{
			int x;

			if (ScreenON || SpriteON) {
				if (GameHBIRQHook && ((PPU[0] & 0x38) != 0x18))
					GameHBIRQHook();
				if (PPU_hook)
					for (x = 0; x < 42; x++) {
						PPU_hook(0x2000); PPU_hook(0);
					}
				if (GameHBIRQHook2)
					GameHBIRQHook2();
			}
			X6502_Run(85 - 16);
			if (ScreenON || SpriteON) {
				RefreshAddr = TempAddr;
				if (PPU_hook) PPU_hook(RefreshAddr & 0x3fff);
			}

			//Clean this stuff up later.
			spork = numsprites = 0;
			ResetRL(XBuf);

			X6502_Run(16 - kook);
			kook ^= 1;
		}
		if (GameInfo->type == GIT_NSF)
			X6502_Run((256 + 85) * normalscanlines);
		#ifdef FRAMESKIP
		else if (skip) {
			int y;

			y = SPRAM[0];
			y++;

			PPU_status |= 0x20;	// Fixes "Bee 52".  Does it break anything?
			if (GameHBIRQHook) {
				X6502_Run(256);
				for (g_cpu.scanline_ref() = 0; g_cpu.scanline_ref() < 240; g_cpu.scanline_ref()++) {
					if (ScreenON || SpriteON)
						GameHBIRQHook();
					if (g_cpu.scanline_ref() == y && SpriteON) PPU_status |= 0x40;
					X6502_Run((g_cpu.scanline_ref() == 239) ? 85 : (256 + 85));
				}
			} else if (y < 240) {
				X6502_Run((256 + 85) * y);
				if (SpriteON) PPU_status |= 0x40;	// Quick and very dirty hack.
				X6502_Run((256 + 85) * (240 - y));
			} else
				X6502_Run((256 + 85) * 240);
		}
		#endif
		else {
			deemp = PPU[1] >> 5;

			// manual samples can't play correctly with overclocking
			if (DMC_7bit && skip_7bit_overclocking) // 7bit sample started before 240th line
				totalscanlines = normalscanlines;
			else
				totalscanlines = normalscanlines + (overclock_enabled ? postrenderscanlines : 0);

			for (g_cpu.scanline_ref() = 0; g_cpu.scanline_ref() < totalscanlines; ) {	//scanline is incremented in  DoLine.  Evil. :/
				deempcnt[deemp]++;

				if (g_cpu.scanline_ref() < 240)
					DEBUG(FCEUD_UpdatePPUView(g_cpu.scanline_ref(), 1));

				DoLine();

				if (g_cpu.scanline_ref() < normalscanlines || g_cpu.scanline_ref() == totalscanlines)
					g_cpu.set_overclocking(false);
				else {
					if (DMC_7bit && skip_7bit_overclocking) // 7bit sample started after 240th line
						break;
					g_cpu.set_overclocking(true);
				}
			}
			DMC_7bit = 0;

			if (MMC5Hack) MMC5_hb(g_cpu.scanline_ref());

			//deemph nonsense, kept for complicated reasons (see SetNESDeemph_OldHacky implementation)
			int maxref = 0;
			for (int x = 1, max = 0; x < 7; x++) {
				if (deempcnt[x] > max) {
					max = deempcnt[x];
					maxref = x;
				}
				deempcnt[x] = 0;
			}
			SetNESDeemph_OldHacky(maxref, 0);
		}
	}	//else... to if(ppudead)

	#ifdef FRAMESKIP
	if (skip) {
		FCEU_PutImageDummy();
		return(0);
	} else
	#endif
	{
		return(1);
	}
}

int (*PPU_MASTER)(int skip) = FCEUPPU_Loop;

// ----------------------------------------------------------------------------
// New-PPU helpers + main loop
// ----------------------------------------------------------------------------
int pputime = 0;
int totpputime = 0;
const int kLineTime = 341;
const int kFetchTime = 2;

void runppu(int x) {
	ppur.status.cycle = (ppur.status.cycle + x) % ppur.status.end_cycle;
	if (!new_ppu_reset) // if resetting, suspend CPU until the first frame
	{
		X6502_Run(x);
	}
}

//todo - consider making this a 3 or 4 slot fifo to keep from touching so much memory
struct BGData {
	struct Record {
		uint8 nt, pecnt, at, pt[2], qtnt;
		uint8 ppu1[8];

		INLINE void Read() {
			NTRefreshAddr = RefreshAddr = ppur.get_ntread();
			if (PEC586Hack)
				ppur.s = (RefreshAddr & 0x200) >> 9;
			else if (QTAIHack) {
				qtnt = QTAINTRAM[((((RefreshAddr >> 10) & 3) >> ((qtaintramreg >> 1)) & 1) << 10) | (RefreshAddr & 0x3FF)];
				ppur.s = qtnt & 0x3F;
			}
			pecnt = (RefreshAddr & 1) << 3;
			nt = CALL_PPUREAD(RefreshAddr);
			ppu1[0] = PPU[1];
			runppu(1);
			ppu1[1] = PPU[1];
			runppu(1);



			RefreshAddr = ppur.get_atread();
			at = CALL_PPUREAD(RefreshAddr);

			//modify at to get appropriate palette shift
			if (ppur.vt & 2) at >>= 4;
			if (ppur.ht & 2) at >>= 2;
			at &= 0x03;
			at <<= 2;
			//horizontal scroll clocked at cycle 3 and then
			//vertical scroll at 251
			ppu1[2] = PPU[1];
			runppu(1);
			if (PPUON) [[likely]] {
				ppur.increment_hsc();
				if (ppur.status.cycle == 251)
					ppur.increment_vs();
			}
			ppu1[3] = PPU[1];
			runppu(1);

			ppur.par = nt;
			RefreshAddr = ppur.get_ptread();
			if (PEC586Hack) {
				pt[0] = CALL_PPUREAD(RefreshAddr | pecnt);
				ppu1[4] = PPU[1];
				runppu(1);
				ppu1[5] = PPU[1];
				runppu(1);
				pt[1] = CALL_PPUREAD(RefreshAddr | pecnt);
				ppu1[6] = PPU[1];
				runppu(1);
				ppu1[7] = PPU[1];
				runppu(1);
			} else if (QTAIHack && (qtnt & 0x40)) {
				pt[0] = *(CHRptr[0] + RefreshAddr);
				ppu1[4] = PPU[1];
				runppu(1);
				ppu1[5] = PPU[1];
				runppu(1);
				RefreshAddr |= 8;
				pt[1] = *(CHRptr[0] + RefreshAddr);
				ppu1[6] = PPU[1];
				runppu(1);
				ppu1[7] = PPU[1];
				runppu(1);
			} else {
				if (ScreenON)
					RENDER_LOG(RefreshAddr);
				pt[0] = CALL_PPUREAD(RefreshAddr);
				ppu1[4] = PPU[1];
				runppu(1);
				ppu1[5] = PPU[1];
				runppu(1);
				RefreshAddr |= 8;
				if (ScreenON)
					RENDER_LOG(RefreshAddr);
				pt[1] = CALL_PPUREAD(RefreshAddr);
				ppu1[6] = PPU[1];
				runppu(1);
				ppu1[7] = PPU[1];
				runppu(1);
			}
		}
	};

	Record main[34];	//one at the end is junk, it can never be rendered
} bgdata;

static inline int PaletteAdjustPixel(int pixel) {
	if ((PPU[1] >> 5) == 0x7)
		return (pixel & 0x3f) | 0xc0;
	else if (PPU[1] & 0xE0)
		return pixel | 0x40;
	else
		return (pixel & 0x3F) | 0x80;
}


// ----------------------------------------------------------------------------
// New-PPU main loop (selected when newppu != 0)
// ----------------------------------------------------------------------------
int framectr = 0;
int FCEUX_PPU_Loop(int skip) {

	if (new_ppu_reset) // first frame since reset, time to initialize
	{
		ppur.reset();
		spr_read.reset();
		new_ppu_reset = false;
	}

	//262 scanlines
	if (ppudead) {
		// not quite emulating all the NES power up behavior
		// since it is known that the NES ignores writes to some
		// register before around a full frame, but no games
		// should write to those regs during that time, it needs
		// to wait for vblank
		ppur.status.sl = 241;
		if (PAL)
			runppu(70 * kLineTime);
		else
			runppu(20 * kLineTime);
		ppur.status.sl = 0;
		runppu(242 * kLineTime);
		--ppudead;
		goto finish;
	}

	{
		PPU_status |= 0x80;
		ppuphase = PPUPHASE_VBL;

		//Not sure if this is correct.  According to Matt Conte and my own tests, it is.
		//Timing is probably off, though.
		//NOTE:  Not having this here breaks a Super Donkey Kong game.
		PPU[3] = PPUSPL = 0;
		const int delay = 20;	//fceu used 12 here but I couldnt get it to work in marble madness and pirates.

		ppur.status.sl = 241;	//for sprite reads

		//formerly: runppu(delay);
		for(int dot=0;dot<delay;dot++)
			runppu(1);

		if (VBlankON) TriggerNMI();
		int sltodo = PAL?70:20;
		
		//formerly: runppu(20 * (kLineTime) - delay);
		for(int S=0;S<sltodo;S++)
		{
			for(int dot=(S==0?delay:0);dot<kLineTime;dot++)
				runppu(1);
			ppur.status.sl++;
		}

		//this seems to run just before the dummy scanline begins
		PPU_status = 0;
		//this early out caused metroid to fail to boot. I am leaving it here as a reminder of what not to do
		//if(!PPUON) { runppu(kLineTime*242); goto finish; }

		//There are 2 conditions that update all 5 PPU scroll counters with the
		//contents of the latches adjacent to them. The first is after a write to
		//2006/2. The second, is at the beginning of scanline 20, when the PPU starts
		//rendering data for the first time in a frame (this update won't happen if
		//all rendering is disabled via 2001.3 and 2001.4).

		//if(PPUON)
		//	ppur.install_latches();

		static uint8 oams[2][64][8];//[7] turned to [8] for faster indexing
		static int oamcounts[2] = { 0, 0 };
		static int oamslot = 0;
		static int oamcount;

		//capture the initial xscroll
		//int xscroll = ppur.fh;
		//render 241/291 scanlines (1 dummy at beginning, dendy's 50 at the end)
		//ignore overclocking!
		for (int sl = 0; sl < normalscanlines; sl++) 
		{
			spr_read.start_scanline();

			g_rasterpos = 0;
			ppur.status.sl = sl;

			linestartts = g_cpu.timestamp_ref() * 48 + g_cpu.native_layout().count; // pixel timestamp for debugger

			const int yp = sl - 1;
			ppuphase = PPUPHASE_BG;

			if (sl != 0 && sl < 241)  // ignore the invisible
			{
				DEBUG(FCEUD_UpdatePPUView(g_cpu.scanline_ref() = yp, 1));
				DEBUG(FCEUD_UpdateNTView(g_cpu.scanline_ref() = yp, 1));
			}

			//hack to fix SDF ship intro screen with split. is it right?
			//well, if we didnt do this, we'd be passing in a negative scanline, so that's a sign something is fishy..
			if(sl != 0)
				if (MMC5Hack) MMC5_hb(yp);


			//twiddle the oam buffers
			const int scanslot = oamslot ^ 1;
			const int renderslot = oamslot;
			oamslot ^= 1;

			oamcount = oamcounts[renderslot];

			//the main scanline rendering loop:
			//32 times, we will fetch a tile and then render 8 pixels.
			//two of those tiles were read in the last scanline.
			for (int xt = 0; xt < 32; xt++) {
				bgdata.main[xt + 2].Read();

				const uint8 blank = (gNoBGFillColor == 0xFF) ? READPAL(0) : gNoBGFillColor;

				//ok, we're also going to draw here.
				//unless we're on the first dummy scanline
				if (sl != 0 && sl < 241) { // cape at 240 for dendy, its PPU does nothing afterwards
					int xstart = xt << 3;
					oamcount = oamcounts[renderslot];
					uint8 * const target = XBuf + (yp << 8) + xstart;
					uint8 * const dtarget = XDBuf + (yp << 8) + xstart;
					uint8 *ptr = target;
					uint8 *dptr = dtarget;
					int rasterpos = xstart;

					//check all the conditions that can cause things to render in these 8px
					const bool renderspritenow = SpriteON && (xt > 0 || SpriteLeft8);
					const bool renderbgnow = ScreenON && (xt > 0 || BGLeft8);
					for (int xp = 0; xp < 8; xp++, rasterpos++, g_rasterpos++) {
						//bg pos is different from raster pos due to its offsetability.
						//so adjust for that here
						const int bgpos = rasterpos + ppur.fh;
						const int bgpx = bgpos & 7;
						const int bgtile = bgpos >> 3;

						uint8 pixel = 0;
						uint8 pixelcolor = blank;

						//according to qeed's doc, use palette 0 or $2006's value if it is & 0x3Fxx
						if (!ScreenON && !SpriteON)
						{
							// if there's anything wrong with how we're doing this, someone please chime in
							int addr = ppur.get_2007access();
							if ((addr & 0x3F00) == 0x3F00)
							{
								pixel = addr & 0x1F;
							}
							pixelcolor = READPAL_MOTHEROFALL(pixel);
						}

						//generate the BG data
						if (renderbgnow) {
							uint8* pt = bgdata.main[bgtile].pt;
							pixel = ((pt[0] >> (7 - bgpx)) & 1) | (((pt[1] >> (7 - bgpx)) & 1) << 1) | bgdata.main[bgtile].at;
						}
						if (renderbg)
							pixelcolor = READPALNOGS(pixel);

						//look for a sprite to be drawn
						bool havepixel = false;
						for (int s = 0; s < oamcount; s++) {
							uint8* oam = oams[renderslot][s];
							int x = oam[3];
							if (rasterpos >= x && rasterpos < x + 8) {
								//build the pixel.
								//fetch the LSB of the patterns
								uint8 spixel = oam[4] & 1;
								spixel |= (oam[5] & 1) << 1;

								//shift down the patterns so the next pixel is in the LSB
								oam[4] >>= 1;
								oam[5] >>= 1;

								if (!renderspritenow) continue;

								//bail out if we already have a pixel from a higher priority sprite
								if (havepixel) continue;

								//transparent pixel bailout
								if (spixel == 0) continue;

								//spritehit:
								//1. is it sprite#0?
								//2. is the bg pixel nonzero?
								//then, it is spritehit.
								if (oam[6] == 0 && (pixel & 3) != 0 &&
									rasterpos < 255) {
									PPU_status |= 0x40;
								}
								havepixel = true;

								//priority handling
								if (oam[2] & 0x20) {
									//behind background:
									if ((pixel & 3) != 0) continue;
								}

								//bring in the palette bits and palettize
								spixel |= (oam[2] & 3) << 2;

								if (rendersprites)
									pixelcolor = READPALNOGS(0x10 + spixel);
							}
						}

						//apply grayscale.. kind of clunky
						//really we need to read the entire palette instead of just ppu1
						//this will be needed for special color effects probably (very fine rainbows and whatnot?)
						//are you allowed to chang the palette mid-line anyway? well you can definitely change the grayscale flag as we know from the FF1 "polygon" effect
						if(bgdata.main[xt+2].ppu1[xp]&1)
							pixelcolor &= 0x30;

						//this does deemph stuff inside it.. which is probably wrong...
						*ptr = PaletteAdjustPixel(pixelcolor);

						ptr++;

						//grab deemph..
						//I guess this works the same way as the grayscale, ideally?
						*dptr++ = bgdata.main[xt+2].ppu1[xp]>>5;
					}
				}
			}

			//look for sprites (was supposed to run concurrent with bg rendering)
			oamcounts[scanslot] = 0;
			oamcount = 0;
			const int spriteHeight = Sprite16 ? 16 : 8;
			for (int i = 0; i < 64; i++) {
				oams[scanslot][oamcount][7] = 0;
				uint8* spr = SPRAM + i * 4;
				if (yp >= spr[0] && yp < spr[0] + spriteHeight) {
					//if we already have maxsprites, then this new one causes an overflow,
					//set the flag and bail out.
					if (oamcount >= 8 && PPUON) {
						PPU_status |= 0x20;
						if (maxsprites == 8)
							break;
					}

					//just copy some bytes into the internal sprite buffer
					for (int j = 0; j < 4; j++)
						oams[scanslot][oamcount][j] = spr[j];
					oams[scanslot][oamcount][7] = 1;

					//note that we stuff the oam index into [6].
					//i need to turn this into a struct so we can have fewer magic numbers
					oams[scanslot][oamcount][6] = (uint8)i;
					oamcount++;
				}
			}
			oamcounts[scanslot] = oamcount;

			//FV is clocked by the PPU's horizontal blanking impulse, and therefore will increment every scanline.
			//well, according to (which?) tests, maybe at the end of hblank.
			//but, according to what it took to get crystalis working, it is at the beginning of hblank.

			//this is done at cycle 251
			//rendering scanline, it doesn't need to be scanline 0,
			//because on the first scanline when the increment is 0, the vs_scroll is reloaded.
			//if(PPUON && sl != 0)
			//	ppur.increment_vs();

			//todo - think about clearing oams to a predefined value to force deterministic behavior

			ppuphase = PPUPHASE_OBJ;

			//fetch sprite patterns
			for (int s = 0; s < maxsprites; s++) {
				//if we have hit our eight sprite pattern and we dont have any more sprites, then bail
				if (s == oamcount && s >= 8)
					break;

				//if this is a real sprite sprite, then it is not above the 8 sprite limit.
				//this is how we support the no 8 sprite limit feature.
				//not that at some point we may need a virtual CALL_PPUREAD which just peeks and doesnt increment any counters
				//this could be handy for the debugging tools also
				const bool realSprite = (s < 8);

				uint8* const oam = oams[scanslot][s];
				uint32 line = yp - oam[0];
				if (oam[2] & 0x80)	//vflip
					line = spriteHeight - line - 1;

				uint32 patternNumber = oam[1];
				uint32 patternAddress;

				//create deterministic dummy fetch pattern
				if (!oam[7]) {
					patternNumber = 0;
					line = 0;
				}

				//8x16 sprite handling:
				if (Sprite16) {
					uint32 bank = (patternNumber & 1) << 12;
					patternNumber = patternNumber & ~1;
					patternNumber |= (line >> 3);
					patternAddress = (patternNumber << 4) | bank;
				} else {
					patternAddress = (patternNumber << 4) | (SpAdrHI << 9);
				}

				//offset into the pattern for the current line.
				//tricky: tall sprites have already had lines>8 taken care of by getting a new pattern number above.
				//so we just need the line offset for the second pattern
				patternAddress += line & 7;

				//garbage nametable fetches
				int garbage_todo = 2;
				if (PPUON)
				{
					if (sl == 0 && ppur.status.cycle == 304)
					{
						runppu(1);
						if (PPUON) ppur.install_latches();
						runppu(1);
						garbage_todo = 0;
					}
					if ((sl != 0 && sl < 241) && ppur.status.cycle == 256)
					{
						runppu(1);
						//at 257: 3d world runner is ugly if we do this at 256
						if (PPUON) ppur.install_h_latches();
						runppu(1);
						garbage_todo = 0;
					}
				}
				if (realSprite) runppu(garbage_todo);

				//Dragon's Lair (Europe version mapper 4)
				//does not set SpriteON in the beginning but it does
				//set the bg on so if using the conditional SpriteON the MMC3 counter
				//the counter will never count and no IRQs will be fired so use PPUON
				if (((PPU[0] & 0x38) != 0x18) && s == 2 && PPUON) {
					//(The MMC3 scanline counter is based entirely on PPU A12, triggered on rising edges (after the line remains low for a sufficiently long period of time))
					//http://nesdevwiki.org/wiki/index.php/Nintendo_MMC3
					//test cases for timing: SMB3, Crystalis
					//crystalis requires deferring this til somewhere in sprite [1,3]
					//kirby requires deferring this til somewhere in sprite [2,5..
					//if (PPUON && GameHBIRQHook) {
					if (GameHBIRQHook) {
						GameHBIRQHook();
					}
				}

				//blind attempt to replicate old ppu functionality
				if(s == 2 && PPUON)
				{
					if (GameHBIRQHook2) {
						GameHBIRQHook2();
					}
				}

				if (realSprite) runppu(kFetchTime);


				//pattern table fetches
				RefreshAddr = patternAddress;
				if (SpriteON)
					RENDER_LOG(RefreshAddr);
				oam[4] = CALL_PPUREAD(RefreshAddr);
				if (realSprite) runppu(kFetchTime);

				RefreshAddr += 8;
				if (SpriteON)
					RENDER_LOG(RefreshAddr);
				oam[5] = CALL_PPUREAD(RefreshAddr);
				if (realSprite) runppu(kFetchTime);

				//hflip
				if (!(oam[2] & 0x40)) {
					oam[4] = bitrevlut[oam[4]];
					oam[5] = bitrevlut[oam[5]];
				}
			}

			ppuphase = PPUPHASE_BG;

			//fetch BG: two tiles for next line
			for (int xt = 0; xt < 2; xt++)
				bgdata.main[xt].Read();

			//I'm unclear of the reason why this particular access to memory is made.
			//The nametable address that is accessed 2 times in a row here, is also the
			//same nametable address that points to the 3rd tile to be rendered on the
			//screen (or basically, the first nametable address that will be accessed when
			//the PPU is fetching background data on the next scanline).
			//(not implemented yet)
			runppu(kFetchTime);
			if (sl == 0) {
				if (idleSynch && PPUON && !PAL)
					ppur.status.end_cycle = 340;
				else
					ppur.status.end_cycle = 341;
				idleSynch ^= 1;
			} else
				ppur.status.end_cycle = 341;
			runppu(kFetchTime);

			//After memory access 170, the PPU simply rests for 4 cycles (or the
			//equivelant of half a memory access cycle) before repeating the whole
			//pixel/scanline rendering process. If the scanline being rendered is the very
			//first one on every second frame, then this delay simply doesn't exist.
			if (ppur.status.end_cycle == 341)
				runppu(1);
		}	//scanline loop

		DMC_7bit = 0;

		if (MMC5Hack) MMC5_hb(240);

		//idle for one line
		runppu(kLineTime);
		framectr++;
	}

finish:
	return 0;
}
