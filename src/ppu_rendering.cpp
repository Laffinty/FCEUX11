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
#include "cpu.h"
#include "fceu.h"
#include "ppu.h"
#include "ppu_rendering.h"
#include "ppu_state.h"
#include "ppu_core.h"
#include "ppu_class.h"
#include "ppu_sprite_lut.h"   // hotfix2 P0-1: kSpriteIdxLUT
#include "pputile_template.h" // hotfix2 P0-3: template<uint8 Flags> FetchAndDrawTile
#include "compiler_attrs.h"   // hotfix2 §16.6: FCEU_BSWAP64, FCEU_UNLIKELY, ...
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
#include <tuple>   // hotfix2 P2-2: std::tuple_size_v for PALRAM size check
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
// CALL_PPUREAD — ppu.cpp line 401. Same expansion, including the Stage-2 §九 L4
// debug-only assert (NDEBUG → ((void)0), so Release is byte-identical).
#define CALL_PPUREAD(A)    (assert(FFCEUX_PPURead != nullptr), FFCEUX_PPURead(A))
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
	// hotfix2 P3-3 (MICRO-3): same [[unlikely]] guard as the per-tile
	// hook sites below. ResetRL fires once per visible scanline so it
	// is ~32x colder than the RefreshLine inner-loop site, but the
	// branch-predictor benefit still applies and the surrounding code
	// is the same shape.
	if (InputScanlineHook) [[unlikely]] {
		InputScanlineHook(0, 0, 0, 0);
	}
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

// hotfix2 P3-2 (MICRO-1): leading-edge detector for the ppudead
// XBuf memset. Set on entry to ppudead so the 60 KiB memset runs
// exactly once per power-on stretch; cleared on the trailing edge
// (next non-ppudead frame) so the next power-on cycle also gets a
// clean fill. File-scope because the reset site lives in the `else`
// branch of FCEUPPU_Loop below, which is outside the static block.
static bool s_ppudead_cleared = false;

// hotfix2 P3-4 (MAP-2): recursion guard for the PPU_hook callback
// path. Set to 1 when the dispatcher routes RefreshLine into the
// Hook / HookBGFetch case (PPU_hook fires from inside the inner
// loop). Cleared at the end of those case bodies. The corresponding
// re-entry check moved from the top of RefreshLine (where every call
// paid the load cost) into a `PPU_hook && norecurse` test guarded
// with [[unlikely]] (see RefreshLine body for the rationale).
static int norecurse = 0;

// lasttile is really "second to last tile."
static void RefreshLine(int lastpixel) {
	// v1.5 Prism §2.2 (Batch 2): pshift[2] / atlatch were static
	// locals here (persistent across scanlines, lifetime = process).
	// They migrate into fceu11::g_ppu.bg_latch_[] / bg_latch_h_; the
	// local aliases below rebind the names without touching
	// pputile.inc (which is included later in this function and uses
	// `pshift[0]`, `pshift[1]`, `atlatch`).
	//
	// hotfix2 P2-6 (DS-1): the v1.5 Prism reference aliasing above
	// (`uint32 (&pshift)[2]`) forces the compiler to treat pshift as
	// an addressable lvalue — every `pshift[0] <<= 8` and `(pshift[0]
	// >> x) & 0xFF` in pputile.inc generates a load/store against
	// g_ppu.bg_latch_[0], not a register-resident copy. Localising to
	// stack slots lets the compiler keep pshift[0]/[1]/atlatch in
	// registers across the 32-tile loop and write them back exactly
	// once at the end. The address-of-pshift[0] pattern in pputile.inc
	// (`pshift[0] <<= 8`) is unaffected — the compiler still sees a
	// plain uint32_t lvalue when we are inside the function.
	uint32_t pshift_local[2] = { fceu11::g_ppu.bg_latch()[0], fceu11::g_ppu.bg_latch()[1] };
	uint32_t atlatch_local   = fceu11::g_ppu.bg_latch_h();
	uint32 (&pshift)[2] = pshift_local;
	uint32 &atlatch     = atlatch_local;
	uint8 *sprlinebuf   = fceu11::g_ppu.line_buffer();
	uint32 smorkus = RefreshAddr;

	#define RefreshAddr smorkus
	uint32 vofs;
	int X1;

	uint8 *P = Pline;
	int lasttile = lastpixel >> 3;
	int numtiles;
	// hotfix2 P3-4 (MAP-2): the unconditional `if (norecurse) return;`
	// guard at the top of RefreshLine has been moved further down
	// (see comment near the PPU_hook-driven dispatch) so the >99%
	// non-hooked path no longer pays the load+cmp cost. The static
	// `norecurse` storage itself is hoisted to file scope below so the
	// set/clear sites in the dispatcher and the Hook / HookBGFetch
	// case bodies can share state across the recursion guard.

	if (sphitx != 0x100 && !(PPU_status & 0x40)) {
		if ((sphitx < (lastpixel - 16)) && !(sphitx < ((lasttile - 2) * 8)))
			lasttile++;
	}

	if (lasttile > 34) lasttile = 34;
	numtiles = lasttile - firsttile;

	if (numtiles <= 0) return;

	P = Pline;

	// hotfix2 P3-4 (MAP-2): `norecurse` is a recursion guard for the
	// PPU_hook callback path. The hook fires from inside the
	// pputile.inc include under PPUT_HOOK; if the hook ever calls
	// back into RefreshLine (e.g. debugger memory-search re-entrancy)
	// we must bail out before the second call reaches the inner loop
	// again. Previously the guard was checked unconditionally at the
	// top of RefreshLine so every call (including the >99%
	// non-hooked path) paid the read-and-compare cost. Now the
	// guard only fires when PPU_hook is set — the common non-hook
	// path takes the [[likely]] branch and skips the read entirely.
	// (The static variable itself is declared at file scope below
	// so the set/clear sites in the dispatcher and case bodies can
	// see it without an inner-static lifetime problem.)
	if (PPU_hook && norecurse) [[unlikely]] {
		// Re-entry from inside a PPU_hook callback while we are
		// still inside the first invocation's hook-driven inner
		// loop. Bail before the inner loop runs again to prevent
		// unbounded recursion. PPU_hook is non-null here so the
		// dispatcher below would otherwise route us back into the
		// Hook / HookBGFetch case and re-invoke the hook.
		return;
	}

	vofs = 0;

	if (PEC586Hack)
		vofs = ((RefreshAddr & 0x200) << 3) | ((RefreshAddr >> 12) & 7);
	else
		vofs = ((PPU[0] & 0x10) << 8) | ((RefreshAddr >> 12) & 7);

	// hotfix2 P1-2 (MASK-1): hoist the GRAYSCALE-dependent palette
	// mask out of the per-pixel READPAL inside pputile.inc. PPU[1]
	// bit 0 (GRAYSCALE) is normally constant for the duration of
	// RefreshLine; the FF1 "polygon" effect that flips GRAYSCALE
	// mid-frame goes through mapper PPU[1] writes which happen
	// during mapper tick handlers (separate from RefreshLine), so
	// the value sampled here is valid until the next RefreshLine.
	const uint8_t pal_mask = (PPU[1] & 0x01) ? 0x30 : 0xFF;

	if (!ScreenON && !SpriteON) [[unlikely]] {
		uint32 tem;
		tem = (PALRAM[0] & pal_mask) | ((PALRAM[0] & pal_mask) << 8)
		    | ((PALRAM[0] & pal_mask) << 16) | ((PALRAM[0] & pal_mask) << 24);
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
			// hotfix2 P3-3 (MICRO-3): InputScanlineHook is a TAS /
			// input-recording callback; for the >99% non-TAS users it
			// is the nullptr branch (input.cpp:473 is a no-op when no
			// input recorder is active). Marking the call [[unlikely]]
			// tells the compiler to lay out the call site cold and
			// keep the fast path's branch predictor history clean.
			if (InputScanlineHook) [[unlikely]] {
				InputScanlineHook(Plinef, spork ? sprlinebuf : 0, linestartts, lasttile * 8 - 16);
			}
		}
		return;
	}

	//Priority bits, needed for sprite emulation.
	//
	// hotfix2 P2-2 (DS-2): the four target bytes (PALRAM offsets 0/4/8/C)
	// live inside a single `alignas(64) std::array<uint8_t, 0x20>` (32 B,
	// one cache line) so the original 4 byte RMWs — 1 read + 1 OR + 1
	// write per byte = 12 μops in flight — are already cache-line-local.
	// A naive `|= 0x40404040u` on a 32-bit load of bytes [0..3] WOULD
	// alias bit 6 of bytes 1/2/3, leaking the priority mark into
	// adjacent palette indices and changing rendered pixels that hit
	// palette slot 1/2/3 BG (pputile.inc:25 `P[0] = S[pixdata & 0xF]`
	// then carries bit 6 into the framebuffer, where CopySprites and
	// the priority merge use it). The plan's literal recipe is therefore
	// INCORRECT for this layout; we keep the 4 byte RMWs which preserve
	// the original semantics exactly. See hotfix2 PLAN §十七/DS-2 audit
	// notes for the analysis.
	//
	// The PALRAM size guard uses tuple_size (compile-time) since the
	// `PALRAM` symbol is only forward-declared here; alignas(64) is a
	// property of the storage object at the definition site (ppu.cpp),
	// not of the type, so we cannot assert on `alignof` here.
	static_assert(std::tuple_size_v<decltype(PALRAM)> >= 0x10,
	              "PALRAM must hold offset 0xC access");
	PALRAM[0] |= 64;
	PALRAM[4] |= 64;
	PALRAM[8] |= 64;
	PALRAM[0xC] |= 64;

	//This high-level graphics MMC5 emulation code was written for MMC5 carts in "CL" mode.
	//It's probably not totally correct for carts in "SL" mode.

	// hotfix2 P1-6 (MAP-1): select RefreshKind once per RefreshLine
	// call instead of branching per-tile. The 4-way MMC5 dispatch +
	// 4-way non-MMC5 dispatch becomes one switch over 9 enum values;
	// the compiler emits each branch as a separate non-returning
	// function call, keeping the mapper-dependent code out of the
	// common path's I-cache. The macro-driven fallbacks below stay
	// intact for the special paths; the default path uses the
	// Phase-A-scaffolded FetchAndDrawTile template (P0-3 + P0-4).
	fceu11::ppu::RefreshKind kind = fceu11::ppu::RefreshKind::Normal;
	if (MMC5Hack && geniestage != 1) {
		if (MMC5HackCHRMode == 0 && (MMC5HackSPMode & 0x80))
			kind = fceu11::ppu::RefreshKind::MMC5SP;
		else if (MMC5HackCHRMode == 1 && (MMC5HackSPMode & 0x80))
			kind = fceu11::ppu::RefreshKind::MMC5CHR1SP;
		else if (MMC5HackCHRMode == 1)
			kind = fceu11::ppu::RefreshKind::MMC5CHR1;
		else
			kind = fceu11::ppu::RefreshKind::MMC5Only;
	} else if (PPU_hook) {
		norecurse = 1;
		kind = PEC586Hack ? fceu11::ppu::RefreshKind::HookBGFetch
		                  : fceu11::ppu::RefreshKind::Hook;
	} else if (PEC586Hack) {
		kind = fceu11::ppu::RefreshKind::BGFetch;
	} else if (QTAIHack) {
		kind = fceu11::ppu::RefreshKind::VRC5Fetch;
	}

	switch (kind) {
	case fceu11::ppu::RefreshKind::MMC5SP: {
		// MMC5SP has a per-tile branch between SP and non-SP variants.
		// The pre-dispatch kind selection only eliminates the outer
		// (MMC5 vs non-MMC5 vs hook) check; the inner tile counter
		// logic must stay because it varies dynamically within one
		// scanline.
#define PPUT_MMC5
		int tochange = MMC5HackSPMode & 0x1F;
		tochange -= firsttile;
		for (X1 = firsttile; X1 < lasttile; X1++) {
			if ((tochange <= 0 && (MMC5HackSPMode & 0x40))
			 || (tochange >  0 && !(MMC5HackSPMode & 0x40))) {
#define PPUT_MMC5SP
				#include "pputile.inc"
#undef PPUT_MMC5SP
			} else {
				#include "pputile.inc"
			}
			tochange--;
		}
#undef PPUT_MMC5
		break;
	}
	case fceu11::ppu::RefreshKind::MMC5CHR1SP:
#define PPUT_MMC5
#define PPUT_MMC5SP
#define PPUT_MMC5CHR1
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPUT_MMC5CHR1
#undef PPUT_MMC5SP
#undef PPUT_MMC5
		break;
	case fceu11::ppu::RefreshKind::MMC5CHR1:
#define PPUT_MMC5
#define PPUT_MMC5CHR1
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPUT_MMC5CHR1
#undef PPUT_MMC5
		break;
	case fceu11::ppu::RefreshKind::MMC5Only:
#define PPUT_MMC5
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPUT_MMC5
		break;
	case fceu11::ppu::RefreshKind::HookBGFetch: {
		// PPU_hook + PEC586Hack (BG-only fetch quirk): P3-4 will move
		// the norecurse guard; keep it scoped to hook paths for now.
#define PPUT_HOOK
#define PPU_BGFETCH
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPU_BGFETCH
#undef PPUT_HOOK
		norecurse = 0;
		break;
	}
	case fceu11::ppu::RefreshKind::Hook: {
#define PPUT_HOOK
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPUT_HOOK
		norecurse = 0;
		break;
	}
	case fceu11::ppu::RefreshKind::BGFetch:
#define PPU_BGFETCH
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPU_BGFETCH
		break;
	case fceu11::ppu::RefreshKind::VRC5Fetch:
#define PPU_VRC5FETCH
		for (X1 = firsttile; X1 < lasttile; X1++) {
			#include "pputile.inc"
		}
#undef PPU_VRC5FETCH
		break;
	case fceu11::ppu::RefreshKind::Normal:
		// hotfix2 P0-3 + P0-4: default template-instantiated path. The
		// most common case for non-MMC5, non-hook, non-special mapper
		// carts (NROM, MMC1/2/3, etc.). This branch covers ~99% of
		// games in the wild.
		for (X1 = firsttile; X1 < lasttile; X1++) {
			fceu11::ppu::FetchAndDrawTile<fceu11::ppu::kFNormal>(
				X1, pshift, atlatch, P,
				smorkus, vofs, vnapage, ScreenON != 0);
		}
		break;
	}


#undef RefreshAddr

	// hotfix2 P2-6 (DS-1): write back the localised pshift[2] / atlatch
	// copies to the canonical g_ppu.bg_latch_* storage so the next
	// RefreshLine call observes the same state we just produced. The
	// locals live on the stack for the duration of this function and
	// fall out of scope as soon as we return.
	fceu11::g_ppu.bg_latch()[0] = pshift_local[0];
	fceu11::g_ppu.bg_latch()[1] = pshift_local[1];
	fceu11::g_ppu.bg_latch_h()  = atlatch_local;

	//Reverse changes made before.
	// hotfix2 P2-2 (DS-2): see comment above. Original 4 byte RMWs
	// restored unchanged — no batched 32-bit form is safe here because
	// the affected bytes are not contiguous and a wider store would
	// clobber the unmodified byte pairs between them.
	PALRAM[0] &= 63;
	PALRAM[4] &= 63;
	PALRAM[8] &= 63;
	PALRAM[0xC] &= 63;

	RefreshAddr = smorkus;
	if (firsttile <= 2 && 2 < lasttile && !(PPU[1] & 2)) {
		uint32 tem;
		tem = (PALRAM[0] & pal_mask) | ((PALRAM[0] & pal_mask) << 8)
		    | ((PALRAM[0] & pal_mask) << 16) | ((PALRAM[0] & pal_mask) << 24);
		tem |= 0x40404040;
		// hotfix1 P2-5 (H-06): Plinef points at uint8[] (XBuf row).
		// Aliasing it as uint32* to write 4 bytes at a time is undefined
		// under the strict-aliasing rules (the underlying dynamic type is
		// uint8). memcpy is the only well-defined way to perform a
		// multi-byte store into a byte buffer. Cost is negligible — this
		// fires at most once per scanline.
		std::memcpy(Plinef + 4, &tem, 4);
		std::memcpy(Plinef, &tem, 4);
	}

	if (!ScreenON) {
		uint32 tem;
		int tstart, tcount;
		tem = (PALRAM[0] & pal_mask) | ((PALRAM[0] & pal_mask) << 8)
		    | ((PALRAM[0] & pal_mask) << 16) | ((PALRAM[0] & pal_mask) << 24);
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
		// hotfix2 P3-3 (MICRO-3): see the matching comment in
		// RefreshLine above. The hook is nullptr for non-TAS runs;
		// [[unlikely]] keeps the inline branch out of the hot path.
		if (InputScanlineHook) [[unlikely]] {
			InputScanlineHook(Plinef, spork ? sprlinebuf : 0, linestartts, lasttile * 8 - 16);
		}
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
	// hotfix2 P1-7 (MAP-4): cache scanline into a local int; use the
	// value-return `g_cpu.scanline()` accessor so the compiler can
	// keep the counter in a register across the function. The old
	// `scanline_ref()` returns int& which forced memory traffic on
	// every read.
	int sl = g_cpu.scanline();
	if (sl >= 240 && sl != totalscanlines) {
		X6502_Run(256 + 69);
		g_cpu.set_scanline(sl + 1);
		X6502_Run(16);
		return;
	}

	int x;
	const int row = (sl < 240 ? sl : 240);
	uint8 *target = XBuf + (row << 8);
	u8* dtarget = XDBuf + (row << 8);

	if (MMC5Hack) MMC5_hb(sl);

	X6502_Run(256);
	EndRL();

	if (!renderbg) {
		uint32 tem;
		uint8 col;
		if (gNoBGFillColor == 0xFF)
			col = PALRAM[0] & ((PPU[1] & 0x01) ? 0x30 : 0xFF);
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
			// hotfix1 P2-5 (H-06): target is uint8* (XBuf row); reading
			// and writing it through a uint32* lvalue is strict-aliasing
			// UB. Stage the 4-byte value through a local uint32_t using
			// memcpy so every byte round-trip is well-defined. The four
			// nested `for` loops below follow the same pattern.
			for (x = 63; x >= 0; x--) {
				uint32_t tmp;
				std::memcpy(&tmp, &target[x << 2], 4);
				tmp &= 0x30303030;
				std::memcpy(&target[x << 2], &tmp, 4);
			}
		}
	}

	if ((PPU[1] >> 5) == 0x7) {
		for (x = 63; x >= 0; x--) {
			uint32_t tmp;
			std::memcpy(&tmp, &target[x << 2], 4);
			tmp = (tmp & 0x3f3f3f3f) | 0xc0c0c0c0;
			std::memcpy(&target[x << 2], &tmp, 4);
		}
	} else if (PPU[1] & 0xE0)
		for (x = 63; x >= 0; x--) {
			uint32_t tmp;
			std::memcpy(&tmp, &target[x << 2], 4);
			tmp |= 0x40404040;
			std::memcpy(&target[x << 2], &tmp, 4);
		}
	else
		for (x = 63; x >= 0; x--) {
			uint32_t tmp;
			std::memcpy(&tmp, &target[x << 2], 4);
			tmp = (tmp & 0x3f3f3f3f) | 0x80808080;
			std::memcpy(&target[x << 2], &tmp, 4);
		}

	// hotfix1 P2-5 (H-06): dtarget is uint8*; uint32* store is
	// strict-aliasing UB. Compute into a local and memcpy the 4 bytes.
	for (x = 63; x >= 0; x--) {
		uint32_t tmp = ((PPU[1]>>5)<<0) | ((PPU[1]>>5)<<8)
		             | ((PPU[1]>>5)<<16) | ((PPU[1]>>5)<<24);
		std::memcpy(&dtarget[x << 2], &tmp, 4);
	}

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

	DEBUG(FCEUD_UpdateNTView(sl, 0));

	if (SpriteON)
		RefreshSprites();
	if (GameHBIRQHook2 && (ScreenON || SpriteON))
		GameHBIRQHook2();
	++sl;
	g_cpu.set_scanline(sl);
	if (sl < 240) {
		ResetRL(XBuf + (sl << 8));
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

// hotfix2 P3-1 (DS-5): constexpr 256-entry bit-reversal LUT. The
// original BITREVLUT<T,BITS> template allocated with `new[]` and
// relied on atexit-time cleanup (OS reclaims the heap). Replacing
// it with `alignas(64) constexpr std::array<uint8_t, 256>` removes
// the heap allocation entirely (the table now lives in .rodata),
// keeps the same 8-bit reversal mapping, and makes the per-call
// `bitrevlut[i]` access a plain array subscript. The mapping is
// identical to the recursive-doubling algorithm the old template
// produced: bit `b` of the input maps to bit `(7-b)` of the output.
//
// `bitrevlut` is kept as an `inline constexpr` reference into the
// table so existing `bitrevlut[oam[4]]` call sites in FCEUX_PPU_Loop
// compile unchanged (std::array::operator[](size_t) returns the
// same uint8_t the BITREVLUT::operator[] used to return). The
// constexpr value lives in .rodata, so the only address loaded
// per call is the table base — no heap indirection, no atexit
// teardown ordering concerns.
alignas(64) inline constexpr std::array<uint8_t, 256> kBitRevLUT = []{
	std::array<uint8_t, 256> t{};
	for (int i = 0; i < 256; i++) {
		uint8_t r = 0;
		for (int b = 0; b < 8; b++) {
			if (i & (1 << b)) r |= static_cast<uint8_t>(1 << (7 - b));
		}
		t[i] = r;
	}
	return t;
}();
// Backwards-compatible alias. std::array subscript matches the
// old BITREVLUT::operator[] semantics 1:1.
static constexpr const std::array<uint8_t, 256>& bitrevlut = kBitRevLUT;

// V_FLIP / H_FLIP / SP_BACK + SPR / SPRB structs (used by
// FetchSpriteData / RefreshSprites / CopySprites).
// v1.13 Purify H: #define → constexpr
inline constexpr uint8_t V_FLIP  = 0x80;
inline constexpr uint8_t H_FLIP  = 0x40;
inline constexpr uint8_t SP_BACK = 0x20;

typedef struct {
	uint8 y, no, atr, x;
} SPR;

// SPRB is now defined in ppu.h (hotfix2 P2-3, see ds-4 audit there).

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

	spr = reinterpret_cast<SPR*>(SPRAM);
	H = 8;

	ns = sb = 0;

	vofs = static_cast<uint32>(P0 & 0x8 & (((P0 & 0x20) ^ 0x20) >> 2)) << 9;
	H += (P0 & 0x20) >> 2;

	// hotfix2 P1-7 (MAP-4): cache scanline locally; the inner loop
	// reads it 64 times per scanline, register-cached via value-return.
	const int sl = g_cpu.scanline();

	if (!PPU_hook)
		for (n = 63; n >= 0; n--, spr++) {
			if (static_cast<uint32>(sl - spr->y) >= H) continue;
			if (ns < maxsprites) {
				if (n == 63) sb = 1;

				{
					SPRB dst;
					uint8 *C;
					int t;
					uint32 vadr;

					t = sl - (spr->y);

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

					// hotfix2 P2-3 (DS-4): SPRBUF is now typed as
					// `SPRB SPRBUF[64]` (defined in ppu.h, layout matches
					// the v1.0 256-byte byte buffer byte-for-byte). The
					// compiler emits a single 4-byte store here instead
					// of the hotfix1 P2-5 (H-06) memcpy round-trip that
					// existed only to dodge the byte-array / uint32
					// lvalue strict-aliasing UB.
					SPRBUF[ns] = dst;
				}

				ns++;
			} else {
				PPU_status |= 0x20;
				break;
			}
		}
	else
		for (n = 63; n >= 0; n--, spr++) {
			if (static_cast<uint32>(sl - spr->y) >= H) continue;

			if (ns < maxsprites) {
				if (n == 63) sb = 1;

				{
					SPRB dst;
					uint8 *C;
					int t;
					uint32 vadr;

					t = sl - (spr->y);

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


					// hotfix2 P2-3 (DS-4): see mirror site above.
					// SPRBUF is `SPRB SPRBUF[64]` typed, so a single
					// 4-byte store replaces the hotfix1 P2-5 (H-06)
					// memcpy round-trip.
					SPRBUF[ns] = dst;
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
	// hotfix2 P2-3 (DS-4): SPRBUF is now a typed SPRB[64] array; the
	// reinterpret_cast<SPRB*> becomes a plain address-of-elem. Same
	// 4-byte packed layout preserved (static_assert in ppu.h).
	spr = &SPRBUF[numsprites];

	// hotfix2 P1-2 (MASK-1, hoisted in lockstep with P0-1): GRAYSCALE
	// (PPU[1] bit 0) is constant for the duration of one RefreshSprites
	// call. Lifting the mask out of the per-pixel READPAL into a single
	// multiply on the small palette table (built per sprite below)
	// eliminates a branch + load per pixel × 8 pixels × ≤8 sprites.
	const uint8_t pal_mask = (PPU[1] & 0x01) ? 0x30 : 0xFF;

	// hotfix3 D-2: hoist PALRAM[0x10..0x1F] (the 4 sprite-palette windows)
	// to a 16-byte op/bk mirror table, computed once per scanline.
	// PALRAM[0x10..0x1F] is invariant for one scanline: writes via
	// $2001 propagate to the next scanline only. Note that the prior
	// PLAN §五 wording "VB 跨 sprite 不变" was incorrect - VB =
	// (atr&3)<<2 | 0x10 depends on each sprite's atr bits 0-1, so
	// per-sprite tables cannot be hoisted. The full 16-byte window
	// is, in contrast, scanline-invariant (max 8 entries read per
	// sprite, never touched mid-scanline). Each sprite then takes its
	// 4 entries by palette index rather than re-reading PALRAM.
	uint8_t palram_op_bk[4][2][4];        // [palette_idx][op=0|bk=1][i]
	for (int p = 0; p < 4; ++p) {
		for (int i = 0; i < 4; ++i) {
			const uint8_t c = PALRAM[0x10 + p*4 + i] & pal_mask;
			palram_op_bk[p][0][i] = c;
			palram_op_bk[p][1][i] = c | 0x40;
		}
	}

	for (n = numsprites; n >= 0; n--, spr--) {
		uint8 J, atr;

		int x = spr->x;
		uint8 *C;
		int VB;

		// hotfix2 P0-1 (ARCH-2): two-stage sprite decode. The
		// 65536-entry kSpriteIdxLUT maps (ca0 | (ca1 << 8)) to 8 packed
		// bytes — one per pixel position — where each byte is the
		// 2-bit palette index (0 = transparent). This replaces the
		// 8 data-dependent `if (J & mask) ... pixdata >>= 4;` chains
		// plus the four (SP_BACK × H_FLIP) path duplications.
		const uint64_t packed = fceu11::ppu::kSpriteIdxLUT[
			(uint32_t)spr->ca[0] | ((uint32_t)spr->ca[1] << 8)];

		J = spr->ca[0] | spr->ca[1];
		atr = spr->atr;

		if (J) [[likely]] {
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

			// Stage 2 (per-sprite runtime): pull the precomputed
			// 4-entry op/bk table from palram_op_bk (D-2 hoist). The
			// pointer-select picks op vs bk directly; SP_BACK OR 0x40
			// is already in palram_op_bk[p][1][i]. With the hoist,
			// this path does zero PALRAM reads - they all happened in
			// the scanline-entry loop above.
			const uint8_t *pal_tab = palram_op_bk[atr & 3][(atr & SP_BACK) ? 1 : 0];

			// H_FLIP: byte-reverse the 64-bit packed indices via a
			// single bswap instruction instead of indexing [7-i] in a
			// loop body (which would force un-indexed loads through
			// the loop counter). After bswap, idx[k] gives the
			// natural-order pixel that should land at C[k]. D-2 reads
			// the form as one ternary instead of init-then-cond-store
			// (semantically equivalent machine code on x86-64 with
			// cmov; readability win, no perf delta).
			const uint64_t flipped = (atr & H_FLIP) ? FCEU_BSWAP64(packed) : packed;
			const uint8_t *idx = reinterpret_cast<const uint8_t *>(&flipped);

			// Single 8-pixel loop covering all four (SP_BACK × H_FLIP)
			// combinations. Each iteration is an L1-resident load
			// (idx[i]) gated on visibility (idx[i] & 0x80). The
			// colour-low-2-bits (idx[i] & 0x03) feeds the 4-entry
			// `pal_tab` indexed load. No serial dependency chain on a
			// shift register. The 4-way branch tree in the original
			// code collapses to two branch-free selects (pal_tab and
			// bswap gate at compile time).
			for (int i = 0; i < 8; ++i) {
				const uint8_t v = idx[i];
				if (v & 0x80) [[likely]] C[i] = pal_tab[v & 0x03];
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

	// P2 Phase 3 Step 3.2 桶 C — open-bus decay probe (2026-08-05).
	// Per-frame PPUGenLatch state snapshot for instrument-first analysis.
	opendecay_log_decay_check();

	//Needed for Knight Rider, possibly others.
	if (ppudead) {
		// hotfix2 P3-2 (MICRO-1): ppudead fires for the first ~2-3
		// frames after power-on while the NES PPU stabilises. The
		// frame buffer is rendered to 0x80 (mid-grey, "screen off")
		// during this period — the 60 KiB memset is unnecessary
		// after the first ppudead frame because nothing in the
		// emulation writes to XBuf while ppudead is set (no BG/sprite
		// fetches run; the runppu() call below only advances CPU
		// and mapper clocks). We clear once on the leading edge of
		// ppudead and skip the memset on subsequent frames; the
		// guard resets on the trailing edge so the next power-on
		// cycle still gets a clean fill.
		if (!s_ppudead_cleared) {
			memset(XBuf, 0x80, 256 * 240);
			s_ppudead_cleared = true;
		}
		X6502_Run(scanlines_per_frame * (256 + 85));
		ppudead--;
	} else {
		// Trailing edge of ppudead: re-arm the leading-edge guard so
		// the next power-on cycle still gets the memset exactly once.
		// ResetRL writes XBuf as soon as ppudead returns to zero, so
		// the stale 0x80 fill from the previous ppudead stretch is
		// overwritten before any visible frame is drawn.
		s_ppudead_cleared = false;
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
				// hotfix2 P1-7 (MAP-4): FRAMESKIP path uses scanline_ref()
				// as the loop variable; keep behaviour identical but
				// drop the int& round-trip via set_scanline.
				for (g_cpu.set_scanline(0); g_cpu.scanline() < 240; g_cpu.set_scanline(g_cpu.scanline() + 1)) {
					const int sl = g_cpu.scanline();
					if (ScreenON || SpriteON)
						GameHBIRQHook();
					if (sl == y && SpriteON) PPU_status |= 0x40;
					X6502_Run((sl == 239) ? 85 : (256 + 85));
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

			// hotfix2 P1-7 (MAP-4): cache scanline locally; the for-loop
			// previously read scanline_ref() 5 times per iteration.
			// DoLine() is responsible for advancing the counter via
			// set_scanline (see DoLine). We pick up the new value at
			// the top of each iteration.
			g_cpu.set_scanline(0);
			for (int sl = 0; sl < totalscanlines; ) {	//scanline is incremented in  DoLine.  Evil. :/
				deempcnt[deemp]++;

				if (sl < 240)
					DEBUG(FCEUD_UpdatePPUView(sl, 1));

				DoLine();

				sl = g_cpu.scanline();

				if (sl < normalscanlines || sl == totalscanlines)
					g_cpu.set_overclocking(false);
				else {
					if (DMC_7bit && skip_7bit_overclocking) // 7bit sample started after 240th line
						break;
					g_cpu.set_overclocking(true);
				}
			}
			DMC_7bit = 0;

			if (MMC5Hack) MMC5_hb(g_cpu.scanline());

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
	// hotfix2 P1-3 (MICRO-4): replace the % with a wrap-around branch.
	// `end_cycle` is 341 (constant per scanline) — a 32-bit DIV costs
	// ~20-40 cycles on modern x86; the branch is a single highly-
	// predictable cmp+jcc (only one branch taken per scanline, when
	// the cycle wraps back to 0). x=1 in the hot BGData::Read path
	// makes the modulo cost cumulative: 8 modulos per record × 32
	// records × ~262 scanlines = ~67k modulos per frame.
	int c = ppur.status.cycle + x;
	// P4-bridge: while for ppudead multi-wrap safety.
	while (c >= ppur.status.end_cycle) c -= ppur.status.end_cycle;
	ppur.status.cycle = c;
	if (!new_ppu_reset) // if resetting, suspend CPU until the first frame
	{
		X6502_Run(x);
	}
}

// hotfix2 P1-4 (INLINE-1): always-inline hot path for the most common
// `runppu(1)` invocation. The hot BGData::Read path calls `runppu(1)`
// eight times per record × 32 records × ~262 scanlines = ~67k times
// per frame; FCEU_ALWAYS_INLINE keeps the body in the caller so the
// compiler can CSE the ppur.status.cycle field load and the
// X6502_RunDebug arg-binding to a global reference. The legacy
// `runppu(x)` wrapper handles multi-cycle call sites (rare in hot
// paths — most are in VBlank bookkeeping).
FCEU_ALWAYS_INLINE
inline void runppu1_inline() noexcept {
	int c = ppur.status.cycle + 1;
	if (c >= ppur.status.end_cycle) c -= ppur.status.end_cycle;
	ppur.status.cycle = c;
	if (!new_ppu_reset) X6502_Run(1);
}

//todo - consider making this a 3 or 4 slot fifo to keep from touching so much memory
// hotfix2 P1-1 (DS-3): `ppu1[8]` split out of Record into a separate
// alignas(64) SoA array. The original layout packed nt/pecnt/at/pt[2]/
// qtnt (6 bytes) with ppu1[8] (8 bytes) and 2 bytes of padding, so the
// pixel loop's per-record `ppu1[xp]` access straddled cache-line
// boundaries depending on alignment. With SoA, the pixel loop's hot
// `bgdata.ppu1[xt+2][xp]` reads stay inside one 64-byte cache line for
// the full 8-pixel run (the SoA slice for one record is exactly 8
// bytes). Read() takes a `slot` argument so it can write into the
// shared SoA array directly (no post-Read copy).
struct BGData {
	struct Record {
		uint8 nt, pecnt, at, pt[2], qtnt;
		uint8 _pad[8];  // hotfix2 P1-1: reserved; old ppu1[8] lives in SoA now

		INLINE void Read(int slot) {
			NTRefreshAddr = RefreshAddr = ppur.get_ntread();
			if (PEC586Hack)
				ppur.s = (RefreshAddr & 0x200) >> 9;
			else if (QTAIHack) {
				qtnt = QTAINTRAM[((((RefreshAddr >> 10) & 3) >> ((qtaintramreg >> 1)) & 1) << 10) | (RefreshAddr & 0x3FF)];
				ppur.s = qtnt & 0x3F;
			}
			pecnt = (RefreshAddr & 1) << 3;
			nt = CALL_PPUREAD(RefreshAddr);
			bgdata.ppu1[slot][0] = PPU[1];
			runppu1_inline();
			bgdata.ppu1[slot][1] = PPU[1];
			runppu1_inline();



			RefreshAddr = ppur.get_atread();
			at = CALL_PPUREAD(RefreshAddr);

			//modify at to get appropriate palette shift
			if (ppur.vt & 2) at >>= 4;
			if (ppur.ht & 2) at >>= 2;
			at &= 0x03;
			at <<= 2;
			//horizontal scroll clocked at cycle 3 and then
			//vertical scroll at 251
			bgdata.ppu1[slot][2] = PPU[1];
			runppu1_inline();
			if (PPUON) [[likely]] {
				ppur.increment_hsc();
				if (ppur.status.cycle == 251)
					ppur.increment_vs();
			}
			bgdata.ppu1[slot][3] = PPU[1];
			runppu1_inline();

			ppur.par = nt;
			RefreshAddr = ppur.get_ptread();
			if (PEC586Hack) {
				pt[0] = CALL_PPUREAD(RefreshAddr | pecnt);
				bgdata.ppu1[slot][4] = PPU[1];
				runppu1_inline();
				bgdata.ppu1[slot][5] = PPU[1];
				runppu1_inline();
				pt[1] = CALL_PPUREAD(RefreshAddr | pecnt);
				bgdata.ppu1[slot][6] = PPU[1];
				runppu1_inline();
				bgdata.ppu1[slot][7] = PPU[1];
				runppu1_inline();
			} else if (QTAIHack && (qtnt & 0x40)) {
				pt[0] = *(CHRptr[0] + RefreshAddr);
				bgdata.ppu1[slot][4] = PPU[1];
				runppu1_inline();
				bgdata.ppu1[slot][5] = PPU[1];
				runppu1_inline();
				RefreshAddr |= 8;
				pt[1] = *(CHRptr[0] + RefreshAddr);
				bgdata.ppu1[slot][6] = PPU[1];
				runppu1_inline();
				bgdata.ppu1[slot][7] = PPU[1];
				runppu1_inline();
			} else {
				if (ScreenON)
					RENDER_LOG(RefreshAddr);
				pt[0] = CALL_PPUREAD(RefreshAddr);
				bgdata.ppu1[slot][4] = PPU[1];
				runppu1_inline();
				bgdata.ppu1[slot][5] = PPU[1];
				runppu1_inline();
				RefreshAddr |= 8;
				if (ScreenON)
					RENDER_LOG(RefreshAddr);
				pt[1] = CALL_PPUREAD(RefreshAddr);
				bgdata.ppu1[slot][6] = PPU[1];
				runppu1_inline();
				bgdata.ppu1[slot][7] = PPU[1];
				runppu1_inline();
			}
		}
	};

	Record main[34];	//one at the end is junk, it can never be rendered
	// hotfix2 P1-1 (DS-3): SoA split — ppu1[34][8] is its own
	// 64-byte-aligned buffer. Each slot row (8 bytes) is the
	// grayscale/deemph byte stream for one 8-pixel tile fetch,
	// written sequentially by Read() and read sequentially by the
	// pixel loop. The 272-byte SoA fits comfortably in 5 cache lines
	// (≤ 320 bytes) so all 34 rows stay hot across one scanline.
	alignas(64) uint8 ppu1[34][8];
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
// E-1 instrument-first probe (Step 1.2, 2026-08-01)
// ----------------------------------------------------------------------------
// Env-gated, zero-intrusion probe for PPU VBL/NMI dot timing. Activated by
// setting FCEUX11_E1_TRACE=1 in the environment (restart required). Silent
// otherwise (ctest 34/34 unaffected). See docs/history/e1_survey/.
static bool e1_trace_on() {
	static const bool on = []() {
		const char* e = std::getenv("FCEUX11_E1_TRACE");
		return e && e[0] == '1' && e[1] == '\0';
	}();
	return on;
}

// E-1 probe (Phase 1 Step 1.3, 2026-08-02): sweep knob for the CPU budget
// granted before the NMI latch is asserted in the VBL block (replaces the
// hard-coded R5 Step 3 runppu(3)). Units are PPU dots (1 dot = 1/3 CPU cycle
// of budget, granted via X6502_Run so the PPU is NOT advanced — the NMI-on
// frame stays a true 6820-dot VBL). Env FCEUX11_E1_NMIDELAY, default 8.
// 2026-08-02 Step 1.3 deep calibration: with the runppu(3) frame distortion
// removed, NMIDELAY=8 makes vbl_05_nmi_timing PASS (X=[4,4,4,3,3,3,3,3,3,2]);
// 7 -> [4,3,...], 9 -> [4,4,4,4,3,...] (transition 1 line late), 8 is the
// unique sweet spot. Sweep data in docs/history/surveys/e1_vbl/.
static int e1_nmi_delay() {
	static const int d = []() {
		const char* e = std::getenv("FCEUX11_E1_NMIDELAY");
		return (e && e[0]) ? std::atoi(e) : 8;
	}();
	return d;
}

// ----------------------------------------------------------------------------
// New-PPU main loop (selected when newppu != 0)
// ----------------------------------------------------------------------------
int framectr = 0;
int FCEUX_PPU_Loop(int skip) {
	// P2 Phase 3 Step 3.2 桶 C — open-bus decay probe (2026-08-05).
	// Per-frame PPUGenLatch state snapshot (newppu path).
	opendecay_log_decay_check();

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
		//
		// P4-bridge: Correct VBL timing during ppudead.
		// Real NES asserts VBL at cycle 1 of sl 241 for exactly
		// 20 NTSC (70 PAL) scanlines, then clears it at cycle 1
		// of sl 261 (NTSC). Previously VBL was set at the END of
		// ppudead and never cleared, leaking across frames and
		// corrupting blargg ROM state before normal rendering.
		// Now: VBL set at cycle 0 → VBlank → clear → render (6820-cycle VBL).
		PPU_status |= 0x80;
		ppuphase = PPUPHASE_VBL;
		if (VBlankON) TriggerNMI();

		ppur.status.sl = 241;
		if (PAL)
			runppu(70 * kLineTime);
		else
			runppu(20 * kLineTime);
		PPU_status = 0;
		ppur.status.sl = 0;
		runppu(242 * kLineTime);

		--ppudead;
		goto finish;
	}

	{
		// Step 1.2 (2026-08-01): a $2002 read 1 PPU dot before this boundary
		// (A2002 at sl240 cycle340) suppresses the VBL flag set + NMI for
		// this frame (NESdev PPU_frame_timing: "Reading one PPU clock before
		// VBL-set reads it as clear and never sets the flag or generates NMI").
		// The VBL period still advances (20 scanlines); only the flag/NMI are
		// skipped. Marker is consumed (cleared) here each frame.
		const bool vbl_set_suppressed = fceu11_ppu_take_vbl_set_suppressed();
		if (e1_trace_on()) {
			fprintf(stderr, "E1 VBL_ENTER abs=%llu sl=%d cycle=%d count=%d lastpc=%04X suppressed=%d VBlankON=%d\n",
			 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
			 ppur.status.sl, ppur.status.cycle, g_cpu.native_layout().count, (unsigned)fceu11_e1_last_pc(),
			 (int)vbl_set_suppressed, (int)(VBlankON != 0));
		}
		if (!vbl_set_suppressed) {
			// Working config: VBL at cycle 0, clear at cycle 0 = 6820 (01-vbl_basics PASS).
			// Cycle 0->1 shift (02-vbl_set_time) deferred to focused follow-up.
			// E-1 Track-B probe (v1.17 R5 task, 2026-08-08): VBL_SET
			// recorder. Fires immediately before PPU_status|=0x80, recording
			// the PRE-set PPU_status byte alongside the existing VBL_ENTER
			// footprint. Distinct probe name so VBL_SET (pre-) and VBL_CLR
			// (pre-, post-verify via next probe) are co-traceable per frame.
			if (e1_trace_on()) {
				fprintf(stderr, "E1B VBL_SET abs=%llu sl=%d cycle=%d count=%d lastpc=%04X PPU_status_pre=0x%02X\n",
				 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
				 ppur.status.sl, ppur.status.cycle, g_cpu.native_layout().count,
				 (unsigned)fceu11_e1_last_pc(), (unsigned)PPU_status);
			}
			PPU_status |= 0x80;
			ppuphase = PPUPHASE_VBL;
		} else {
			ppuphase = PPUPHASE_VBL;
		}

		//Not sure if this is correct.  According to Matt Conte and my own tests, it is.
		//Timing is probably off, though.
		//NOTE:  Not having this here breaks a Super Donkey Kong game.
		PPU[3] = PPUSPL = 0;
		const int delay = 20;

		ppur.status.sl = 241;	//for sprite reads

		// R5 Step 3 (2026-08-01, path d): delay NMI dispatch by 1 CPU cycle
		// (3 PPU dots) after VBL flag set. Real-hardware timing: VBL flag
		// asserts at sl 241 cycle 1, NMI is dispatched ~1 CPU cycle later
		// on the rising edge. Previously dispatched at the same dot, causing
		// vbl_05's NMI to fire ~1 iteration earlier than expected.
		// Step 4 (runppu(6)) was tried and REVERTED: X became
		// [3,2,2,2,2,2,2,1,1,1] — row 0 overshoot to 3 proves single-param
		// NMI delay cannot fix vbl_05's per-row phase drift (see
		// docs/history/e1_survey/vbl_step3_fix_data_2026-08-01.md §9).
		// Step 1.2: X6502_Run(nd) is ALWAYS done when NMI is enabled so the
		// frame length stays identical on suppressed and non-suppressed
		// frames (6820 with NMI on OR off — the R5 Step 3 runppu(3) wart that
		// lengthened NMI-on frames to 6823 is removed; hardware VBL is
		// exactly 20 scanlines = 6820 dots regardless of NMI state).
		// Phase 1 Step 1.3 deep (2026-08-02): run the CPU budget WITHOUT
		// advancing the PPU. runppu(nd) advanced the PPU nd dots AND granted
		// nd*16 units of CPU budget; the PPU advance distorted NMI-on frames
		// (+3 dots, vbl_05 VBL_ENTER cycle drifted +3/frame), which skews the
		// per-frame NMI phase drift that blargg's nmi_timing test measures.
		// X6502_Run(nd) grants the same CPU budget (1 CPU cycle per 3 dots)
		// with zero PPU advance, keeping the frame at a true 6820-dot VBL.
		if (VBlankON) {
			const int nd = e1_nmi_delay();
			if (nd > 0) X6502_Run(nd);
			if (e1_trace_on()) {
				fprintf(stderr, "E1 VBL_AFTER_NMIDELAY abs=%llu sl=%d cycle=%d delay=%d\n",
				 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
				 ppur.status.sl, ppur.status.cycle, nd);
			}
			// E-1 Track-B probe (v1.17 R5 task, 2026-08-08): NMI_LATCH
			// recorder. Fires immediately BEFORE TriggerNMI() (vs the
			// existing E1 NMI_SET inside x6502.cpp::TriggerNMI which fires
			// AFTER _IRQlow|=FCEU_IQNMI). Captures the dispatch site's
			// PPU status + count + lastpc so vbl_05 / vbl_07 / vbl_08 NMI
			// dispatch latency can be measured from the CALLER's frame
			// of reference, not the callee's. Distinct probe name
			// (E1B NMI_LATCH vs E1 NMI_SET) so dispatcher/callee sites
			// are both recorded.
			if (!vbl_set_suppressed && e1_trace_on()) {
				fprintf(stderr, "E1B NMI_LATCH abs=%llu sl=%d cycle=%d count=%d lastpc=%04X PPU_status=0x%02X VBlankON=%d\n",
				 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
				 ppur.status.sl, ppur.status.cycle, g_cpu.native_layout().count,
				 (unsigned)fceu11_e1_last_pc(), (unsigned)PPU_status, 1);
			}
			if (!vbl_set_suppressed) TriggerNMI();
		}
		if (e1_trace_on() && vbl_set_suppressed) {
			fprintf(stderr, "E1 VBL_SUPPRESSED frame=%d\n", framectr);
		}

		//formerly: runppu(delay);
		for(int dot=0;dot<delay;dot++)
			runppu(1);
		int sltodo = PAL?70:20;

		//formerly: runppu(20 * (kLineTime) - delay);
		for(int S=0;S<sltodo;S++)
			{
		for(int dot=(S==0?delay:0);dot<kLineTime;dot++)
				runppu(1);
			ppur.status.sl++;
		}

		// E-1 Track-B probe (v1.17 R5 task, 2026-08-08): VBL_CLR recorder.
		// Captures the pre-clear PPU_status byte plus scanline/cycle/count
		// so vbl_03 / vbl_09 timing-window FAILs can be characterized with
		// the exact dot the flag was cleared, regardless of when the existing
		// VBL_ENTER / VBL_AFTER_NMIDELAY probes fire. Zero-intrusion; env-gated
		// by FCEUX11_E1_TRACE. Marks the second event of one frame's VBL
		// span (VBL_SET -> NMI dispatch -> VBL_CLR).
		if (e1_trace_on()) {
			fprintf(stderr, "E1B VBL_CLR abs=%llu sl=%d cycle=%d count=%d lastpc=%04X PPU_status_pre=0x%02X\n",
			 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
			 ppur.status.sl, ppur.status.cycle, g_cpu.native_layout().count,
			 (unsigned)fceu11_e1_last_pc(), (unsigned)PPU_status);
		}
		PPU_status = 0;
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

		// hotfix2 P0-2 (ARCH-3): sprite X-bucket + per-pixel shift
		// pre-computation. Built once per scanline from the `oams`
		// snapshot that FetchSpriteData populated, then consumed by
		// the inner pixel loop without further per-pixel state
		// mutation. Layout:
		//   - `oam_pat_lo[s][k]` / `oam_pat_hi[s][k]` =
		//     oams[..][s][4] (or [5]) shifted right k times.
		//   - `oam_bucket_idx[bi][k]` = k-th sprite index whose start
		//     x satisfies x>>3 == bi; count is in `oam_bucket_count`.
		alignas(64) static uint8 oam_pat_lo[64][8];
		alignas(64) static uint8 oam_pat_hi[64][8];
		// hotfix2 P0-2 (post-Phase-A review): widen each X-bucket
		// from 8 to 64 sprite slots. The PPU supports
		// FCEUI_DisableSpriteLimitation(1) which raises maxsprites
		// to 64; in that mode multiple sprites can share the same
		// `x >> 3` and overflow the bucket (silent OOB write). The
		// extra ~1.75 KiB BSS is negligible — `alignas(64)` keeps
		// it on cache-line boundaries either way.
		alignas(64) static uint8 oam_bucket_idx[32][64];
		static int   oam_bucket_count[32] = {0};

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
				// hotfix2 P1-7 (MAP-4): use set_scanline + scanline() instead
				// of int&-style assignment through scanline_ref().
				g_cpu.set_scanline(yp);
				DEBUG(FCEUD_UpdatePPUView(g_cpu.scanline(), 1));
				DEBUG(FCEUD_UpdateNTView(g_cpu.scanline(), 1));
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

			// hotfix2 P0-2: precompute per-sprite shift tables and
			// X-bucket classification once per scanline. This moves
			// the per-pixel `oam[4] >>= 1; oam[5] >>= 1;` work out of
			// the hot 256×8 inner loop, and replaces the O(sprites)
			// sprite-search with O(bucket-size) where the average
			// bucket size is 0..1 sprite.
			for (int bi = 0; bi < 32; ++bi) oam_bucket_count[bi] = 0;
			if (sl != 0 && sl < 241) {
				for (int s = 0; s < oamcount; ++s) {
					uint8 p0 = oams[renderslot][s][4];
					uint8 p1 = oams[renderslot][s][5];
					for (int k = 0; k < 8; ++k) {
						oam_pat_lo[s][k] = p0;
						oam_pat_hi[s][k] = p1;
						p0 >>= 1;
						p1 >>= 1;
					}
					const int bi = (oams[renderslot][s][3] >> 3) & 31;
					oam_bucket_idx[bi][oam_bucket_count[bi]++] =
						static_cast<uint8_t>(s);
				}
			}

			//the main scanline rendering loop:
			//32 times, we will fetch a tile and then render 8 pixels.
			//two of those tiles were read in the last scanline.
			// hotfix2 P1-2 (MASK-1): hoist pal_mask out of the inner 32
			// tile loop. PPU[1] bit 0 (GRAYSCALE) is constant for the
			// scanline's visible region; the FF1 polygon effect flips
			// GRAYSCALE only via mid-scanline mapper tick (rarely).
			const uint8_t scanline_pal_mask = (PPU[1] & 0x01) ? 0x30 : 0xFF;
			const uint8 blank = (gNoBGFillColor == 0xFF) ? (PALRAM[0] & scanline_pal_mask) : gNoBGFillColor;
			for (int xt = 0; xt < 32; xt++) {
				bgdata.main[xt + 2].Read(xt + 2);

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
					// hotfix2 P0-2: per-tile bucket index (constant for the
					// whole 8-pixel xp loop body). Bucket 0..31 covers all
					// possible sprite start-x tile positions.
					const uint8 *bucket_idx =
						oam_bucket_idx[rasterpos >> 3];
					const int bucket_n = oam_bucket_count[rasterpos >> 3];
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
						// hotfix2 P0-2: pre-bucketed sprite scan.
						//
						// Average bucket size is ~0-1 sprite (not all 8),
						// because most sprite x positions in a 32-tile
						// scanline are unique. The original `for s =
						// 0..oamcount` would scan all 8 sprites at every
						// pixel; this iterates `bucket_n` sprites (often
						// 0-2). The per-sprite pattern shift is read from
						// `oam_pat_lo/_hi[s][k]` rather than mutating
						// `oams[..][s][4/5]` inside the pixel loop.
						for (int sb = 0; sb < bucket_n; ++sb) {
							const int s = bucket_idx[sb];
							uint8* oam = oams[renderslot][s];
							const int x = oam[3];
							if (rasterpos < x || rasterpos >= x + 8) continue;

							const int k = rasterpos - x;
							uint8 spixel = (oam_pat_lo[s][k] & 1)
								| ((oam_pat_hi[s][k] & 1) << 1);

							if (!renderspritenow) continue;
							if (havepixel) continue;
							if (spixel == 0) continue;

							//spritehit:
							//1. is it sprite#0?
							//2. is the bg pixel nonzero?
							if (oam[6] == 0 && (pixel & 3) != 0 &&
								rasterpos < 255) {
								PPU_status |= 0x40;
							}
							havepixel = true;

							//priority handling
							if (oam[2] & 0x20) {
								if ((pixel & 3) != 0) continue;
							}

							spixel |= (oam[2] & 3) << 2;

							if (rendersprites)
								pixelcolor = READPALNOGS(0x10 + spixel);
						}

						//apply grayscale.. kind of clunky
						//really we need to read the entire palette instead of just ppu1
						//this will be needed for special color effects probably (very fine rainbows and whatnot?)
						//are you allowed to chang the palette mid-line anyway? well you can definitely change the grayscale flag as we know from the FF1 "polygon" effect
						if(bgdata.ppu1[xt+2][xp]&1)
							pixelcolor &= 0x30;

						//this does deemph stuff inside it.. which is probably wrong...
						*ptr = PaletteAdjustPixel(pixelcolor);

						ptr++;

						//grab deemph..
						//I guess this works the same way as the grayscale, ideally?
						*dptr++ = bgdata.ppu1[xt+2][xp]>>5;
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
					oams[scanslot][oamcount][6] = static_cast<uint8>(i);
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
						// E-1 Track-B probe (v1.17 R5 task, 2026-08-08):
						// EVEN_ODD_GATE recorder. Fires the FIRST time the
						// (sl==0 && cycle==304) gate is entered for a frame
						// (idleSynch side), capturing the dot-level PPU/CPU
						// phase BEFORE the existing SKIP_DEC probe decides
						// end_cycle=340 vs 341. Distinct from E1 SKIP_DEC
						// (which fires post-decision in kFetchTime block) —
						// this one captures the gate's PPUON state at the
						// precise (sl=0,cycle=304) frame boundary.
						if (e1_trace_on()) {
							fprintf(stderr, "E1B EVEN_ODD_GATE abs=%llu frame=%d sl=%d cycle=%d count=%d idleSynch=%d PPUON=%d\n",
							 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
							 framectr, sl, ppur.status.cycle, g_cpu.native_layout().count,
							 idleSynch, PPUON ? 1 : 0);
						}
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
				bgdata.main[xt].Read(xt);

			//I'm unclear of the reason why this particular access to memory is made.
			//The nametable address that is accessed 2 times in a row here, is also the
			//same nametable address that points to the 3rd tile to be rendered on the
			//screen (or basically, the first nametable address that will be accessed when
			//the PPU is fetching background data on the next scanline).
			//(not implemented yet)
			runppu(kFetchTime);
			if (sl == 0) {
				// E-1 probe (Phase 1 Step 1.4, 2026-08-03): even/odd skip
				// decision recorder. Logs the PPU dot where the end_cycle
				// decision is made, the PPUON state sampled there, and the CPU
				// budget residual (count, 1/16-dot units) that carries the
				// sub-dot frame phase (vbl_10 investigation — see
				// docs/history/surveys/e1_vbl/vbl_step1_4_*).
				if (e1_trace_on()) {
					fprintf(stderr, "E1 SKIP_DEC abs=%llu frame=%d sl=%d cycle=%d count=%d PPUON=%d idleSynch=%d PAL=%d end_cycle=%d\n",
					 (unsigned long long)(g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref()),
					 framectr, sl, ppur.status.cycle, g_cpu.native_layout().count,
					 PPUON ? 1 : 0, idleSynch, PAL ? 1 : 0,
					 (idleSynch && PPUON && !PAL) ? 340 : 341);
				}
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
