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

#include <cstdlib>   // P2 Phase 3 Step 3.2 桶 C — opendecay_atexit (std::atexit)
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

// Phase 6 P2 shadow sync: peek without consuming (the shadow runner
// pushes the latch into Rust so both cores suppress the same frame's
// VBlank set; the C++ core consumes it itself at the next VBL_ENTER).
bool fceu11_ppu_peek_vbl_set_suppressed() {
	return g_vbl_set_suppressed;
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

// ----------------------------------------------------------------------------
// P2 Phase 3 Step 3.2 桶 C — ppu_open_bus decay probe (instrument-first, 2026-08-05)
// ----------------------------------------------------------------------------
// Env-gated, zero-intrusion probe for PPU open-bus (PPUGenLatch) decay
// behavior. Activated by setting FCEUX11_OPENDECAY_PROBE=1 in the
// environment (restart required). Silent otherwise (ctest 34/34
// unaffected). Goal: confirm the test 3 scenario (write $FF to $2002,
// wait 1000ms with no PPU register access, read $2000 expecting $00)
// in the current implementation — i.e. observe that PPUGenLatch never
// decays today, and gather timestamp spacing data to design the
// threshold for the upcoming fix. See:
//   docs/history/surveys/ppu_bucketC/opendecay_probe_2026-08-05.md
static bool opendecay_probe_on() {
	static const bool on = []() {
		const char* e = std::getenv("FCEUX11_OPENDECAY_PROBE");
		return e && e[0] == '1' && e[1] == '\0';
	}();
	return on;
}

// PPUGenLatch open-bus decay timestamp (P2 Phase 3 Step 3.2 桶 C).
// Defined here (before the probe functions) so that
// opendecay_log_write can update it without a forward declaration.
// See the longer P2 Phase 3 comment block below for the design.
uint64 PPUGenLatch_last_refresh_cycle = 0;

// PPUGenLatch write tracking — last refresh timestamp (CPU cycle units).
// Note: this is PROBE ONLY state; the actual fix will use a separate
// `PPUGenLatch_last_refresh` field on a per-write basis.
static uint64 s_probe_last_write_cycle = 0;
static uint8  s_probe_last_write_value = 0;
static uint32 s_probe_write_count = 0;
static uint32 s_probe_read2000_count = 0;
static uint32 s_probe_decay_check_count = 0;
static uint64 s_probe_first_read2000_after_write_cycle = 0;
static bool   s_probe_first_read2000_seen = false;
static uint64 s_probe_run_start_cycle = 0;

static inline uint64 opendecay_now_cycle() {
	return g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
}

static inline void opendecay_log_write(uint8 V) {
	// Always refresh the decay timestamp — the actual fix relies
	// on this. Probe logging is gated separately.
	PPUGenLatch_last_refresh_cycle = opendecay_now_cycle();
	if (!opendecay_probe_on()) return;
	const uint64 now = PPUGenLatch_last_refresh_cycle;
	const uint64 elapsed = s_probe_last_write_cycle
		? (now - s_probe_last_write_cycle) : 0;
	fprintf(stderr,
		"OPENDECAY W PPUGenLatch=0x%02X (prev=0x%02X) cycle=%llu "
		"elapsed_since_last_write_cycles=%llu writes_so_far=%u\n",
		(unsigned)V, (unsigned)PPUGenLatch,
		(unsigned long long)now, (unsigned long long)elapsed,
		(unsigned)s_probe_write_count);
	s_probe_last_write_cycle = now;
	s_probe_last_write_value = V;
	s_probe_write_count++;
	s_probe_first_read2000_seen = false;
	s_probe_first_read2000_after_write_cycle = 0;
}

static inline void opendecay_log_read2000(uint8 ret) {
	if (!opendecay_probe_on()) return;
	const uint64 now = opendecay_now_cycle();
	const uint64 since_write = s_probe_last_write_cycle
		? (now - s_probe_last_write_cycle) : 0;
	if (!s_probe_first_read2000_seen) {
		s_probe_first_read2000_after_write_cycle = since_write;
		s_probe_first_read2000_seen = true;
		fprintf(stderr,
			"OPENDECAY R2000 FIRST_AFTER_WRITE ret=0x%02X cycle=%llu "
			"elapsed_since_last_write_cycles=%llu\n",
			(unsigned)ret, (unsigned long long)now,
			(unsigned long long)since_write);
	}
	s_probe_read2000_count++;
	if ((s_probe_read2000_count & 0x3FF) == 0) {
		fprintf(stderr,
			"OPENDECAY R2000 count=%u ret=0x%02X cycle=%llu "
			"elapsed_since_last_write_cycles=%llu\n",
			(unsigned)s_probe_read2000_count, (unsigned)ret,
			(unsigned long long)now,
			(unsigned long long)since_write);
	}
}

// Forward decl: opendecay_init is defined below.
static bool opendecay_init();

// ----------------------------------------------------------------------------
// P2 Phase 3 Step 3.2 桶 C — PPUGenLatch open-bus decay (2026-08-05)
// ----------------------------------------------------------------------------
// Per blargg ppu_open_bus readme:
//   "The PPU effectively has a 'decay register', an 8-bit register.
//    Each bit can be refreshed with a 0 or 1. If a bit isn't refreshed
//    with a 1 for about 600 milliseconds, it will decay to 0."
// Implementation: track the last CPU cycle at which any PPU register
// was *written* (writes refresh the entire 8-bit latch to the written
// value; reads do NOT refresh — per blargg the read of write-only
// registers such as $2000/$2001/$2003/$2005/$2006 returns the decay
// value without refreshing). At each frame entry (FCEUPPU_Loop /
// FCEUX_PPU_Loop, ~60 Hz NTSC), check if the elapsed time since the
// last write exceeds the 600 ms threshold; if so, force PPUGenLatch
// to 0. Per-frame granularity is sufficient (1 frame ≈ 16.67 ms ≪
// 600 ms threshold), so decay fires within one frame of crossing the
// threshold.
//   Threshold rationale: blargg says "about 600 ms". NTSC CPU clock
// is 1.789773 MHz, so 600 ms = 1 073 864 cycles. Probe data
// (FCEUX11_OPENDECAY_PROBE=1 run on ppu_open_bus, see
// docs/history/surveys/ppu_bucketC/opendecay_probe_2026-08-05.md)
// confirmed that test 3's `setb $2002,$FF` → `delay_msec 1000` →
// `lda PPUCTRL` produces a gap of ~1.79 M cycles between the last
// write and the test read — comfortably above the 600 ms threshold,
// so a threshold of 1 073 864 cycles triggers decay well before the
// test reads.
static constexpr uint64 PPU_OPEN_BUS_DECAY_CYCLES = 1073864; // ~600 ms NTSC

// (PPUGenLatch_last_refresh_cycle is defined earlier in the file
// so that opendecay_log_write can update the timestamp without an
// ordering hazard. See the P2 Phase 3 Step 3.2 桶 C comment block.)

static inline void ppu_latch_decay_check() {
	// Skip work when latch is already 0 (decayed or never written) —
	// the common case in well-behaved games that don't abuse open
	// bus. Avoids one CPU timestamp call per frame in that path.
	if (PPUGenLatch == 0) return;
	const uint64 now = g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
	// `last_refresh_cycle == 0` means never written since power-on —
	// PPUGenLatch is 0 in that case (ppu.cpp:122), so we already
	// returned above. Guard against underflow defensively.
	if (PPUGenLatch_last_refresh_cycle != 0 &&
		now > PPUGenLatch_last_refresh_cycle &&
		now - PPUGenLatch_last_refresh_cycle > PPU_OPEN_BUS_DECAY_CYCLES) {
		PPUGenLatch = 0;
	}
}

// External linkage (not static, not inline) so the symbol is exported
// from fceux11_core.lib and linkable from ppu_rendering.cpp.obj. The
// forward declaration in ppu.h matches this linkage. Hot-path calls
// would not be a concern (only invoked from the per-frame loop),
// and inlining would force duplicate definitions across TUs that
// include ppu.h.
//
// This function is the single entry point that combines the
// production decay check (always runs, ~negligible cost) with the
// env-gated probe snapshot. Called once per frame from
// FCEUPPU_Loop / FCEUX_PPU_Loop.

// Forward decl: ppu_latch_decay_check is defined above this point in
// the file but in source order is below the first call site. Keep
// the prototype here so the call inside opendecay_log_decay_check
// resolves correctly even with -Werror.
static inline void ppu_latch_decay_check();

void opendecay_log_decay_check() {
	// Production decay check — always runs.
	ppu_latch_decay_check();
	if (!opendecay_probe_on()) return;
	opendecay_init();
	const uint64 now = opendecay_now_cycle();
	const uint64 since_write = s_probe_last_write_cycle
		? (now - s_probe_last_write_cycle) : 0;
	s_probe_decay_check_count++;
	if (s_probe_decay_check_count < 16 ||
		(s_probe_decay_check_count & 0x3F) == 0) {
		fprintf(stderr,
			"OPENDECAY CHECK n=%u PPUGenLatch=0x%02X cycle=%llu "
			"elapsed_since_last_write_cycles=%llu\n",
			(unsigned)s_probe_decay_check_count,
			(unsigned)PPUGenLatch, (unsigned long long)now,
			(unsigned long long)since_write);
	}
}

// P2 Phase 3 Step 3.2 桶 D — DMC + SPR DMA bus contention probe (2026-08-05).
// Records B4014 sprite-DMA entry/exit cycle counts and byte count, so
// we can correlate the test's "T+ Clocks" output (527/528 per row)
// against actual SPR DMA duration and any DMC arbitration that fires
// during the transfer. Env-gated by FCEUX11_OPENDECAY_PROBE (same
// gate as the open-bus decay probe — the project standard pattern).
// Defined as a free function so the env-gated branch can be skipped
// when the probe is off (zero-cost in the hot path).
static void opendecay_log_sprdma_event(uint32 cycle_before,
                                        uint32 cycle_after,
                                        uint32 byte_count) {
	extern bool opendecay_probe_on();
	if (!opendecay_probe_on()) return;
	const uint32 elapsed = cycle_after - cycle_before;
	const uint32 parity = cycle_before & 1;
	std::fprintf(stderr,
		"OPENDECAY SPRDMA bytes=%u cycle_before=%u cycle_after=%u "
		"elapsed=%u parity_before=%u\n",
		byte_count, cycle_before, cycle_after, elapsed, parity);
}

static inline void opendecay_log_run_end() {
	if (!opendecay_probe_on()) return;
	const uint64 now = opendecay_now_cycle();
	const uint64 total = now - s_probe_run_start_cycle;
	fprintf(stderr,
		"OPENDECAY SUMMARY writes=%u reads2000=%u decay_checks=%u "
		"final_PPUGenLatch=0x%02X total_cycles=%llu "
		"first_R2000_after_write_cycles=%llu\n",
		(unsigned)s_probe_write_count,
		(unsigned)s_probe_read2000_count,
		(unsigned)s_probe_decay_check_count,
		(unsigned)PPUGenLatch, (unsigned long long)total,
		(unsigned long long)s_probe_first_read2000_after_write_cycle);
}

static void opendecay_reset() {
	s_probe_last_write_cycle = 0;
	s_probe_last_write_value = 0;
	s_probe_write_count = 0;
	s_probe_read2000_count = 0;
	s_probe_decay_check_count = 0;
	s_probe_first_read2000_after_write_cycle = 0;
	s_probe_first_read2000_seen = false;
	s_probe_run_start_cycle = opendecay_now_cycle();
}

// One-shot atexit handler to dump final stats when probe is on.
static void opendecay_atexit() {
	if (!opendecay_probe_on()) return;
	opendecay_log_run_end();
}

// One-shot reset on first activation so per-run stats are clean.
static bool opendecay_init() {
	static bool done = false;
	if (done) return false;
	done = true;
	opendecay_reset();
	// atexit fires when the emulator exits cleanly; capture final stats.
	std::atexit(opendecay_atexit);
	return true;
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
				// v1.16 P2 Phase 3 Step 3.2 桶 C fix (2026-08-05): per
				// blargg ppu_open_bus readme, a $2004 read refreshes all
				// 8 bits of the decay register with the SPRAM byte
				// driven on the bus. Without this, test 11 ("Reading
				// third byte of a sprite from $2004 should refresh all
				// bits of decay value") fails: after `setb $2002,$FF /
				// lda SPRDATA / lda PPUCTRL / and #$1C` the latch still
				// holds $FF (bits 2-4 set) instead of the attribute byte
				// at SPRAM[2] (bits 2-4 always clear). The next test's
				// `cmp #0` (and #$1C) then fails.
				PPUGenLatch = spr_read.ret;
				return spr_read.ret;
			}
		} else {
			// v1.16 P2 Phase 3 Step 3.2 桶 C fix (2026-08-05): same
			// per-blargg open-bus refresh for the non-rendering path
			// (VBL or rendering-off). See the inner comment above.
			const uint8 ret = SPRAM[PPU[3]];
			PPUGenLatch = ret;
			return ret;
		}
	} else {
		FCEUPPU_LineUpdate();
		return PPUGenLatch;
	}
}

static DECLFR(A200x) {	/* Not correct for $2004 reads. */
	FCEUPPU_LineUpdate();
	uint8 ret = PPUGenLatch;
	if (A == 0x2000) opendecay_log_read2000(ret);
	return ret;
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
			// v1.16 P2 Bucket C fix (2026-08-04): index the palette RAM
			// with the LIVE address (RefreshAddr from get_2007access()),
			// not the stale `tmp` captured at function entry. The PPU
			// register file (fv/v/h/vt/ht) is the authoritative source
			// for the current VRAM address; the global RefreshAddr can
			// lag behind it (e.g. a preceding $2007 read auto-incremented
			// ppur but an intervening open-bus/$2005 access left the
			// global stale). Using `tmp` here made palette reads return
			// whatever PALRAM[tmp&0x1F] held instead of PALRAM[addr&0x1F],
			// so blargg ppu_read_buffer TEST_PALETTE_READS_UNRELIABLE
			// (0x30=48) saw non-$0E values at $3F0F.
			const uint32 paddr = RefreshAddr & 0x1F;
			if (!(paddr & 3)) {
				if (!(paddr & 0xC))
					ret = READPAL(0x00);
				else
					ret = READUPAL(((paddr & 0xC) >> 2) - 1);
			} else
				ret = READPAL(paddr);
			// v1.16 P2 Phase 3 Step 3.2 桶 C fix (2026-08-05): per blargg
			// ppu_open_bus readme, a $2007 *palette* read returns
			// "DD-- ----" — high 2 bits are open-bus (from PPUGenLatch),
			// low 6 bits are PALRAM. Without this OR, the test reads
			// `lda PPUDATA / and #$C0` against a plain PALRAM value and
			// blargg ppu_open_bus test 8 ("High 2 bits from $2007 from
			// palette should be from decay value") sees the wrong high
			// bits. The PPUGenLatch update below (`PPUGenLatch = ret`)
			// then naturally does the right per-bit refresh: bits 0-5
			// come from the new PALRAM data (refreshed) and bits 6-7
			// come from the old latch (preserved), exactly matching
			// blargg's "D - " row for the palette column.
			ret |= (PPUGenLatch & 0xC0);
			VRAMBuffer = CALL_PPUREAD(RefreshAddr - 0x1000);
		} else {
			if (debug_loggingCD && (RefreshAddr < 0x2000))
				LogAddress = GetCHRAddress(RefreshAddr);
			VRAMBuffer = CALL_PPUREAD(RefreshAddr);
		}
		ppur.increment2007(ppur.status.sl >= 0 && ppur.status.sl < 241 && PPUON, INC32 != 0);
		RefreshAddr = ppur.get_2007access();
		// v1.16 P2 Bucket C fix (2026-08-04): mirror the old-PPU path's
		// `PPUGenLatch = VRAMBuffer`. A $2007 read places the returned
		// value on the PPU data bus; subsequent reads of registers with
		// no read function ($2000-$2006, open bus) must repeat it.
		// Without this, blargg ppu_read_buffer TEST_PPU_OPENBUS_MUST_
		// NOT_COPY_READBUFFER (0x13=19) fails: `cpy PPUCTRL` after
		// `ldy PPUDATA` reads a stale PPUGenLatch instead of the
		// just-transferred buffer value.
		PPUGenLatch = ret;
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
	opendecay_log_write(V);

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
	opendecay_log_write(V);
	PPU[1] = V;
	if (V & 0xE0)
		deemp = V >> 5;
}

static DECLFW(B2002) {
	PPUGenLatch = V;
	opendecay_log_write(V);
}

static DECLFW(B2003) {
	PPUGenLatch = V;
	opendecay_log_write(V);
	PPU[3] = V;
	PPUSPL = V & 0x7;
}

static DECLFW(B2004) {
	PPUGenLatch = V;
	opendecay_log_write(V);
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
	opendecay_log_write(V);
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
	opendecay_log_write(V);
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
		opendecay_log_write(V);
		RefreshAddr = ppur.get_2007access() & 0x3FFF;
		CALL_PPUWRITE(RefreshAddr, V);
		ppur.increment2007(ppur.status.sl >= 0 && ppur.status.sl < 241 && PPUON, INC32 != 0);
		RefreshAddr = ppur.get_2007access();
	} else {
		PPUGenLatch = V;
		opendecay_log_write(V);
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

	// P2 Phase 3 Step 3.2 桶 D — instrument-first probe (2026-08-05).
	// The blargg sprdma_dmc_dma test fails with 0x01 because the current
	// 512-cycle loop has no DMC bus arbitration. Per NESdev wiki: when
	// both DMAs are active, the DMC DMA is inserted into the sprite DMA
	// stream (each fetch ~4 CPU cycles). A correct fix requires a full
	// Mesen2-style per-cycle DMA state machine with cycle parity
	// tracking (see NesCpu.cpp::ProcessPendingDma) — a deep-model
	// change beyond the "改动面小" budget. Probe records the actual
	// SPR DMA duration for future investigation.
	const uint32 cycle_before = g_cpu.timestamp_base() + (uint32)g_cpu.timestamp_ref();

	for (x = 0; x < 256; x++)
		X6502_DMW(0x2004, X6502_DMR(t + x));

	const uint32 cycle_after = g_cpu.timestamp_base() + (uint32)g_cpu.timestamp_ref();
	opendecay_log_sprdma_event(cycle_before, cycle_after, (uint32)x);
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
