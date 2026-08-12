// FCEUX11 — PPU public interface.
//
// v1.5 Prism §1.1: PPU register file (`PPU[4]`), name-table RAM
// (`NTARAM[0x800]`), pointer table (`vnapage[4]`), and bank-switching
// masks (`PPUCHRRAM`, `PPUNTARAM`) are now `fceu11::g_ppu` internals
// (NTARAM/vnapage) or thin `extern` reference aliases binding to the
// v1.0 storage (PPU[4]/PPUCHRRAM/PPUNTARAM). The declarations live in
// ppu_class.h — see that file for the class definition and the alias
// binding rationale. This file keeps the FCEUPPU_* free-function
// surface that fceu.cpp / bus.cpp / cart.cpp etc. call into.

#ifndef FCEU11_PPU_H
#define FCEU11_PPU_H

#include <cstdint>

#include "ppu_class.h"   // fceu11::Ppu, g_ppu, PPU/NTARAM/vnapage/PPUCHRRAM/PPUNTARAM aliases

// P2 Phase 3 Step 3.2 桶 C — open-bus decay probe (2026-08-05).
// Implemented in ppu.cpp; called from ppu_rendering.cpp's frame loop.
void opendecay_log_decay_check();

void FCEUPPU_Init(void);
void FCEUPPU_Reset(void);
void FCEUPPU_Power(void);
int FCEUPPU_Loop(int skip);

void FCEUPPU_LineUpdate();
// Step 1.2 ($2002 VBL-set suppression, 2026-08-01): a $2002 read 1 PPU dot
// before the working-config VBL set boundary suppresses the flag set + NMI
// for that frame (NESdev PPU_frame_timing). ppu.cpp marks it, the new-PPU
// VBL block in ppu_rendering.cpp takes (and clears) it.
void fceu11_ppu_mark_vbl_set_suppressed();
bool fceu11_ppu_peek_vbl_set_suppressed();
bool fceu11_ppu_take_vbl_set_suppressed();
void FCEUPPU_SetVideoSystem(int w);

extern void (*PPU_hook)(uint32 A);
extern void (*GameHBIRQHook)(void), (*GameHBIRQHook2)(void);

int newppu_get_scanline();
int newppu_get_dot();
void newppu_hacky_emergency_reset();

// NTARAM, vnapage, PPUCHRRAM, PPUNTARAM — declared as reference
// aliases in ppu_class.h. The alias definitions there rebind these
// names to the canonical storage: NTARAM / vnapage point into
// fceu11::g_ppu; PPUCHRRAM / PPUNTARAM stay as v1.0 ppu.cpp globals
// per plan §1.3. Including ppu_class.h (above) is what makes these
// visible to ppu.h consumers.
//
// The old v1.0 declarations were:
//   extern uint8 NTARAM[0x800], *vnapage[4];
//   extern uint8 PPUNTARAM;
//   extern uint8 PPUCHRRAM;
// These are now superseded by ppu_class.h's reference aliases.

void FCEUPPU_SaveState(void);
void FCEUPPU_LoadState(int version);
uint32 FCEUPPU_PeekAddress();
uint8* FCEUPPU_GetCHR(uint32 vadr, uint32 refreshaddr);
int FCEUPPU_GetAttr(int ntnum, int xt, int yt);
void ppu_getScroll(int &xpos, int &ypos);


#ifdef _MSC_VER
#define FASTCALL __fastcall
#else
#define FASTCALL
#endif

void PPU_ResetHooks();
extern uint8 (FASTCALL *FFCEUX_PPURead)(uint32 A);
extern void (*FFCEUX_PPUWrite)(uint32 A, uint8 V);
extern uint8 FASTCALL FFCEUX_PPURead_Default(uint32 A);
void FFCEUX_PPUWrite_Default(uint32 A, uint8 V);

extern int g_rasterpos;
// PPU[4] / PPUCHRRAM / PPUNTARAM — declared as reference-to-storage
// aliases in ppu_class.h. The alias bindings there point to the v1.0
// storage in ppu.cpp (the actual `uint8 PPU[4]`, `uint8 PPUCHRRAM`,
// `uint8 PPUNTARAM` definitions remain in ppu.cpp per plan §1.3).
// Including ppu_class.h (above) provides these declarations; no
// separate `extern uint8 PPU[4];` etc. are needed here.

// v1.5 Prism §2.1 (Batch 1) + §2.3 (Batch 3): SPRBUF / VRAMBuffer /
// PPUGenLatch are v1.0 PPU-internal globals still defined in ppu.cpp.
// They used to be forward-declared in debug.h alongside PPU/vnapage;
// that header's declarations were removed when ppu_class.h took
// ownership of PPU/vnapage. Forward them here so debug.cpp (and any
// other PPU-internal state reader) sees them via the ppu.h include
// chain.
//
// SPRAM was previously listed here too; v1.5 Prism §2.3 migrates it
// into fceu11::g_ppu as oam_[256]. The `extern uint8_t (& SPRAM)[0x100]`
// reference alias in ppu_class.h is the new public declaration — old
// `extern uint8 SPRAM[0x100];` variable form is gone.
//
// XOffset was previously listed here too; v1.5 Prism §2.1 migrates
// it into fceu11::g_ppu as fine_x_scroll_. The `extern uint8_t
// (& XOffset)` reference alias in ppu_class.h is the new public
// declaration — old `extern uint8 XOffset;` variable form is gone.

// hotfix2 P2-3 (DS-4): SPRB moved up from src/ppu_rendering.cpp so
// the typed `SPRBUF[ns] = SPRB{...}` writes don't need a memcpy.
// SPRB is a 4-byte packed sprite descriptor fetched from pattern
// memory during sprite evaluation. Its byte-level layout must match
// the v1.0 definition exactly (offset 0/1 = ca[0]/ca[1], offset 2 =
// atr, offset 3 = x); the static_asserts below guard against future
// reordering / padding drift.
//
// All members are uint8_t — the struct has natural alignment 1 with
// no padding, so the v1.0 buffer layout is preserved bit-for-bit.
struct SPRB {
    uint8_t ca[2];    // offset 0..1 (plane 0 / plane 1 of sprite tile)
    uint8_t atr;      // offset 2 (attribute byte: palette / flip / pri)
    uint8_t x;        // offset 3 (horizontal position)
};
static_assert(sizeof(SPRB) == 4,
              "SPRB must remain 4 bytes so SPRBUF[64] matches v1.0 SPRBUF[0x100] byte layout");
static_assert(alignof(SPRB) == 1,
              "SPRB must remain 1-byte aligned so we can memcpy from/to a uint32_t");

// hotfix2 P2-3 (DS-4): SPRBUF is now an array of SPRB (typed sprite
// descriptors) instead of a raw byte buffer. 64 entries × 4 bytes each
// = 256 bytes total — same byte footprint, type-safe access. Defined
// in src/ppu.cpp:109; consumed by src/ppu_rendering.cpp
// (FetchSpriteData / RefreshSprites) without the hotfix1 P2-5
// (H-06) memcpy round-trip.
extern SPRB SPRBUF[64];
extern uint8 VRAMBuffer, PPUGenLatch;

extern bool& DMC_7bit;
extern bool paldeemphswap;

// v1.12 Scissors Phase E-B: scanlines_per_frame promoted from file-
// static in ppu.cpp to extern. Written by FCEUPPU_SetVideoSystem
// (now in ppu_core.cpp); read by the main scanline loops in
// ppu_rendering.cpp (post-E-C). Definition stays in ppu.cpp until
// E-C moves it.
extern unsigned int scanlines_per_frame;

// PPUPHASE — moved to ppu_class.h (which ppu.h includes). Keeping a
// declaration here would re-introduce the circular include.

extern PPUPHASE ppuphase;

extern unsigned char *cdloggervdata;
extern unsigned int cdloggerVideoDataSize;
extern volatile int rendercount, vromreadcount, undefinedvromcount;

#endif // FCEU11_PPU_H