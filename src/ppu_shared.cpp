// ppu_shared.cpp
//
// FCEUX11 v2.1.1.7 — the surviving C++ PPU surface.
//
// History:
//   * Step B.2 (2026-09-11) created this TU for the FCEUPPU_LineUpdate no-op
//     (7 non-engine call sites + 8 retired register handlers + ResetRL were
//     deleted there; probe evidence: docs/history/v2.1.1.7_b2_line_update.md).
//   * Step C / M4 (2026-09-12, plan section C.1) retired the C++ PPU engine:
//     src/ppu.cpp (register handlers + old renderer helpers + opendecay
//     probe), src/ppu_rendering.cpp/h, src/ppu_core.cpp/h,
//     src/pputile_template.cpp/h, src/pputile.inc and
//     src/ppu_sprite_lut.cpp/h were deleted, and every symbol that still has
//     a live consumer moved here.
//
// What the C++ side still owns after Step C (plan section 0.1 / 0.3):
//   * lifecycle + dispatch entry points called by the rest of the core
//     (FCEUPPU_Init/Power/Reset/Loop/SetVideoSystem/PeekAddress,
//     PPU_ResetHooks, newppu_get_scanline/dot, newppu_hacky_emergency_reset);
//   * the mapper-facing hooks (PPU_hook, GameHBIRQHook, GameHBIRQHook2);
//   * the PPU-space bus dispatch slots (FFCEUX_PPURead/Write + defaults) that
//     the Rust bus callback and the MMC5 overrides route through;
//   * the C++-authoritative arrays and accessors (PALRAM / UPALRAM /
//     READPAL_MOTHEROFALL / FCEUPPU_GetCHR / FCEUPPU_GetAttr);
//   * the tombstone mirrors the debugger, the savestate staging and the
//     mapper hacks read (PPU[0..3], SPRAM, PPUSPL, VRAMBuffer, PPUGenLatch,
//     g_rasterpos, ...).
//
// Engine-only state that no longer has a writer lives in
// src/ppu_legacy_stub.cpp (ppur / spr_read / idleSynch / new_ppu_reset /
// SPRBUF / linestartts).

#include "types.h"
#include "cpu.h"             // fceu11::cpu_instance(), FCEUX11_PREFETCH
#include "fceu.h"            // dendy / FSettings / VPage / MMC5Hack* / QTAINTRAM
#include "ppu.h"             // public PPU surface + PPU[4] alias
#include "ppu_core.h"        // scanlines_per_frame + tombstone externs
#include "ppu_state.h"       // PALRAM / UPALRAM / PPUSPL
#include "ppu_rust_bridge.h" // Rust engine entry points (unconditional after C.2)
#include "debug.h"           // UPALRAM / READPAL_MOTHEROFALL declarations
#include "io_api.h"       // SetRenderPlanes / FCEUI_DisableSpriteLimitation
#include "utils/memory.h"    // FCEU_MemoryRand

#include <array>

// ---------------------------------------------------------------------------
// PPU-space read helpers. GRAYSCALE mirrors the $2001 bit the retired B2001
// handler used; the Qt viewers call READPAL_MOTHEROFALL for the
// $3F00/$3F04/$3F08/$3F0C mirror quirk.
// ---------------------------------------------------------------------------
#define GRAYSCALE   (PPU[1] & 0x01)
#define READPAL(ofs)    (PALRAM[(ofs)] & (GRAYSCALE ? 0x30 : 0xFF))
#define READUPAL(ofs)   (UPALRAM[(ofs)] & (GRAYSCALE ? 0x30 : 0xFF))
#define VRAMADR(V)      &VPage[(V) >> 10][(V)]

uint8 READPAL_MOTHEROFALL(uint32 A)
{
	if (!(A & 3)) {
		if (!(A & 0xC))
			return READPAL(0x00);
		else
			return READUPAL(((A & 0xC) >> 2) - 1);
	}
	else
		return READPAL(A & 0x1F);
}

// Defined in src/boards/mmc5.cpp; the MMC5 CL mode resolver.
uint8* MMC5BGVRAMADR(uint32 A);

// ---------------------------------------------------------------------------
// CHR / attribute accessors. Used by mmc5.cpp ($2007 CL-mode reads) and
// NameTableViewer.cpp. These read the C++-authoritative window pointers, so
// they are unaffected by the C++ engine retirement.
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// PPU-space write path. Step C: this is the only writer left; it served the
// retired B200x handlers ($2007 routing) and the MMC5 overrides
// (boards/mmc5.cpp:1091 installs mmc5_PPUWrite, which delegates here).
// ---------------------------------------------------------------------------
void FFCEUX_PPUWrite_Default(uint32 A, uint8 V) {
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

// The mapper-facing PPU read/write slots. PPU_ResetHooks() (re)installs the
// defaults; boards/mmc5.cpp replaces both at reset time.
uint8 (FASTCALL *FFCEUX_PPURead)(uint32 A) = 0;
void (*FFCEUX_PPUWrite)(uint32 A, uint8 V) = 0;

// ---------------------------------------------------------------------------
// Surviving globals. Definitions moved from src/ppu.cpp (Step C).
// ---------------------------------------------------------------------------

// Frame phase. The retired C++ renderer was the only writer
// (docs/history/v2.1.1.7_cpp_ppu_removal_archived_2026-09-12.md section 0.4), so under the Rust PPU
// this stays at its power-on value; boards/mmc5.cpp still reads it in the
// MMC5 CL-mode resolver (pre-existing since Phase 7, not introduced by
// Step C).
PPUPHASE ppuphase;

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

// C++-authoritative palette arrays (Rust renders from a pointer into PALRAM;
// UPALRAM holds the $3F04/$3F08/$3F0C read-back mirrors).
alignas(64) std::array<uint8_t, 0x20> PALRAM;
std::array<uint8_t, 3> UPALRAM;

int g_rasterpos;
uint32 scanlines_per_frame;
uint8 PPUSPL;
uint8 VRAMBuffer = 0, PPUGenLatch = 0;

// $4014 copies 256 bytes from $xx00-$xxFF to $2004 (OAM data).
uint8 SpriteDMA = 0;

// Code/Data Logger counters (src/drivers/Qt/CodeDataLogger.cpp).
volatile int rendercount, vromreadcount, undefinedvromcount;
unsigned char *cdloggervdata = NULL;
unsigned int cdloggerVideoDataSize = 0;

// Legacy `newppu` switch. Since Step B.5 (D3-a) it no longer selects an
// engine - the Rust PPU is the only one - but it is still read by the movie
// PPUflag, the config/menu plumbing and the zapper beam-scan branch
// (src/fceu.cpp:139-148, input/zapper.cpp:93, movie_io.cpp:138).
int newppu = 0;

// ---------------------------------------------------------------------------
// Scroll introspection for the debugger (ConsoleDebugger, NameTableViewer).
// ---------------------------------------------------------------------------
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

// ---------------------------------------------------------------------------
// Mapper-facing hooks. Definitions moved from src/ppu_core.cpp (Step C).
// The defaults are NULL; mmc5.cpp and other boards assign them at reset time.
// ---------------------------------------------------------------------------
void (*GameHBIRQHook)(void), (*GameHBIRQHook2)(void);
void (*PPU_hook)(uint32 A);

void PPU_ResetHooks() {
	FFCEUX_PPURead = FFCEUX_PPURead_Default;
	FFCEUX_PPUWrite = FFCEUX_PPUWrite_Default;
}

// ---------------------------------------------------------------------------
// Timing / raster configuration.
// ---------------------------------------------------------------------------
void FCEUPPU_SetVideoSystem(int w) {
	if (w) {
		scanlines_per_frame = dendy ? 262: 312;
		FSettings.FirstSLine = FSettings.UsrFirstSLine[1];
		FSettings.LastSLine = FSettings.UsrLastSLine[1];
		//paldeemphswap = 1; // dendy has pal ppu, and pal ppu has these swapped
	} else {
		scanlines_per_frame = 262;
		FSettings.FirstSLine = FSettings.UsrFirstSLine[0];
		FSettings.LastSLine = FSettings.UsrLastSLine[0];
		//paldeemphswap = 0;
	}
	// v2.1.1.7 Step B.5-2b: the Rust PPU drives its raster from the region
	// table; `w` is PAL and `dendy` is the Dendy quirk (312 lines, VBL set at
	// 291). Both feed the same table the crate unit tests pin.
	ppu_rust_bridge_set_video_system(w != 0, dendy != 0);
}

// ---------------------------------------------------------------------------
// Lifecycle.
// ---------------------------------------------------------------------------

// v1.5 Prism §1.1 hotfix3 C-1: route the Ppu class reset() through this entry
// point so mapper Power() (which runs *after* this in the PowerNES sequence)
// can override vnapage_ with the board-specific mapping.
void FCEUPPU_Reset(void) {
	fceu11::g_ppu.reset();

	VRAMBuffer = PPU[0] = PPU[1] = PPU[2] = PPU[3] = 0;
	PPUSPL = 0;
	PPUGenLatch = 0;
	RefreshAddr = TempAddr = 0;
	vtoggle = 0;
	ppudead = 2;
	kook = 0;
	idleSynch = 1;

	new_ppu_reset = true; // retired: kept so the tombstone bytes match pre-Step-C state
}

// Power-on PPU state. Step C removed the C++ register-handler install loop
// that used to occupy the tail of this function: A2002/A200x/A2004/A2007,
// B2000..B2007 and B4014 were the only writers of the $2000-$3FFF / $4014
// dispatch entries, and the Rust PPU now owns the whole decode -
// `ppu_rust_bridge_power()` below installs the routing ($2000-$3FFF reads,
// $2000-$2007 writes, $4014 read + write) exactly as it did before, in the
// same position of the PowerNES sequence. Entries the bridge does not claim
// (writes in $2008-$3FFF) keep the Bus default no-op handler, which is
// observably identical to the C++ tombstones they used to land on.
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
	// Restore the default PPU read/write hooks before resetting: the Rust bus
	// callback dereferences FFCEUX_PPURead without a NULL guard, and the only
	// other writer besides PPU_ResetHooks is ResetGameLoaded() (which NULLs it
	// on every LoadGame). Board Power() handlers (e.g. MMC5) run after this
	// and may override the hooks themselves.
	PPU_ResetHooks();
	FCEUPPU_Reset();

	// Reset the Rust PPU state and prime the CHR/NT/palette windows from
	// whatever the bus currently holds. Called again at the end of PowerNES
	// (src/fceu.cpp) once the mapper has populated the window pointers.
	ppu_rust_bridge_power();
}

void FCEUPPU_Init(void) {
	// v2.1.1.7 Step C (M4): the C++ renderer's palette LUT (makeppulut() +
	// ppulut1/2/3, src/ppu_rendering.cpp) was deleted with the engine; the
	// Rust PPU builds its own tables (fceux11-ppu/src/luts.rs). The entry
	// point is kept because the core and the test harnesses call it.
}

// ---------------------------------------------------------------------------
// Frame driver. The whole per-dot CPU/PPU interleave loop lives in Rust
// (`fceux11_run_frame_interleaved`); this is the only frame entry point left.
// ---------------------------------------------------------------------------
int FCEUPPU_Loop(int skip) {
#ifdef FCEUX11_RUST_PPU
	// Phase 5.3: ONE FFI per frame instead of three per dot (~268k boundary
	// crossings/frame in the Phase 5.1 C++ loop, which was the dominant
	// bench_tolerance_test regression). The per-dot operation sequence is
	// unchanged: tick 1 PPU dot (render / mapper hooks / OAM DMA pump) ->
	// pulse TriggerNMI() when the PPU latch fires -> advance the CPU by one
	// dot unit (1 CPU cycle = 3 dot units, the legacy X6502_Run convention).
	if (ppu_rust_bridge_active()) [[likely]] {
		if (ppu_rust_bridge_run_frame_interleaved(ppu_rust_bridge_ppu_dots_per_frame()) == 0) {
			ppu_rust_bridge_copy_framebuffer();
			return 0;
		}
	}
#endif
	// Defensive fallback (bridge not initialised, or no Rust PPU state): the
	// Phase 5.1 CPU-only frame advance. Step C removed the C++ engine, so
	// there is no renderer to fall back to any more.
	for (uint32_t dot = 0; dot < ppu_rust_bridge_ppu_dots_per_frame(); ++dot) {
		fceu11::cpu_instance().run(1);
	}
	return 0;
}

// ---------------------------------------------------------------------------
// Debugger accessors moved from src/ppu_core.cpp (Step C).
// ---------------------------------------------------------------------------

// Step B.5 (D3-a): `newppu` no longer selects an engine. Under the Rust PPU
// these report the live Rust raster; the C++ `ppur` counters are tombstones
// there (plan section 0.1).
int newppu_get_scanline() {
	if (ppu_rust_bridge_active()) {
		return ppu_rust_bridge_get_scanline();
	}
	return ppur.status.sl;
}
int newppu_get_dot() {
	if (ppu_rust_bridge_active()) {
		return ppu_rust_bridge_get_dot();
	}
	return ppur.status.cycle;
}
void newppu_hacky_emergency_reset()
{
	if(ppur.status.end_cycle == 0)
		ppur.reset();
}

// ---------------------------------------------------------------------------
// FCEUPPU_PeekAddress - read the current PPU address without side effects.
//
// Used by the debugger (debug.cpp) and the cheat/trace tools to peek at the
// next address the PPU will read or write.
// ---------------------------------------------------------------------------
uint32 FCEUPPU_PeekAddress()
{
	// Step B.4: under the Rust PPU the v latch lives in the Rust engine; the
	// C++ ppur / RefreshAddr latches are tombstones there (plan section 0.1).
	// Falls back to the legacy path when the bridge is inactive (not
	// initialised yet). Do NOT regress this to RefreshAddr: B.4-2 fixed the
	// stale-address read that way.
	if (ppu_rust_bridge_active())
	{
		return ppu_rust_bridge_get_v() & 0x3FFF;
	}

	if (newppu)
	{
		return ppur.get_2007access() & 0x3FFF;
	}

	return RefreshAddr & 0x3FFF;
}

// ---------------------------------------------------------------------------
// Retired flush point (Step B.2). The Rust renderer consumes the CHR / NT
// window copies that ppu_rust_bridge.cpp re-installs at scanline granularity,
// so a mid-line bank switch needs no flush. Probe evidence in
// docs/history/v2.1.1.7_b2_line_update.md: Pline is never non-zero under the
// Rust engine, i.e. the old flush never executed.
// ---------------------------------------------------------------------------
void FCEUPPU_LineUpdate(void) {
}

// ---------------------------------------------------------------------------
// Render-plane / sprite-limit toggles (declared in src/io_api.h).
//
// The retired C++ renderer was the only reader of these three flags: it tested
// `rendersprites` / `renderbg` in RefreshLine and `maxsprites` in the sprite
// fetch loop. They stay as stored state so the hotkey and Qt paths
// (input.cpp::BackgroundDisplayToggle / ObjectDisplayToggle,
// drivers/Qt/ConsoleActions.cpp, drivers/Qt/config.cpp) keep their previous
// behaviour - which, under the Rust PPU, is "store and return the flag, do not
// affect the rendered frame", exactly as before Step C (the Rust engine never
// read them either). Recorded as an M5 owner-QA item.
// ---------------------------------------------------------------------------
namespace {
[[maybe_unused]] bool g_render_sprites = true;
[[maybe_unused]] bool g_render_bg      = true;
[[maybe_unused]] int  g_max_sprites    = 8;
}  // namespace

void fceu11::SetRenderPlanes(bool sprites, bool bg) {
	g_render_sprites = sprites;
	g_render_bg = bg;
}

void fceu11::GetRenderPlanes(bool& sprites, bool& bg) {
	sprites = g_render_sprites;
	bg = g_render_bg;
}

// Definition migrated from the retired ppu_rendering.cpp (originally
// ppu.cpp line 877).
void FCEUI_DisableSpriteLimitation(int a) {
	g_max_sprites = a ? 64 : 8;
}