// pputile_template.cpp
//
// hotfix2 P0-3 (ARCH-1a): out-of-line definition of the
// `template <uint8_t Flags> FetchAndDrawTile(...)` function
// replacing pputile.inc's `#include` sites in ppu_rendering.cpp.
//
// Body is a byte-for-byte port of the original pputile.inc
// (142 lines, kept on disk as `tests/golden/pputile.inc` for
// cross-check); the only structural change is `#ifdef X` /
// `#define X ... #endif` → `if constexpr ((Flags & XFlag) != 0)`.

#include "pputile_template.h"

#include "types.h"
#include "x6502.h"
#include "fceu.h"            // MMC5Hack*, qtaintramreg, PPU_hook
#include "ppu.h"
#include "ppu_rendering.h"
#include "ppu_state.h"
#include "ppu_class.h"
#include "bus.h"              // VPage, MMC5SPRVPage, CHRptr
#include "compiler_attrs.h"
#include "utils/format.h"     // FCEU_MAYBE_UNUSED

#include <cstdint>

// Hook callback. PPU_hook is `void (*)(uint32)` — see ppu.h.
extern void (*PPU_hook)(uint32 vadr);

// Local mirror of the VRAMADR macro from ppu_rendering.cpp:159.
// It is file-scope #define, so we reproduce it here. Keep in sync
// with ppu_rendering.cpp.
#define VRAMADR(V)         (&VPage[(V) >> 10][(V)])

namespace fceu11::ppu {

template <uint8_t Flags>
FCEU_ALWAYS_INLINE FCEU_HOT
void FetchAndDrawTile(int X1, uint32_t pshift[2], uint32_t& atlatch,
                      uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                      uint8_t** vnapage, bool ScreenON)
{
    // Forward declarations of variables used by the body, mirroring
    // pputile.inc:1-3 plus the per-path variants at lines 8-15.
    uint8_t* C;
    uint8_t  cc;
    uint32_t vadr;

    if constexpr ((Flags & kFlagVRC5Fetch) != 0) {
        uint8_t tmpd;
    }

    // pputile.inc:8-15 — variable initialisation for MMC5SP path
    // (vs the default `zz` alias used in the normal path).
    if constexpr ((Flags & kFlagMMC5SP) != 0) {
        uint8_t xs = static_cast<uint8_t>(X1);
        uint8_t ys = static_cast<uint8_t>(
            ((g_cpu.scanline_ref() >> 3) + MMC5HackSPScroll) & 0x1F);
        if (ys >= 0x1E) ys -= 0x1E;
        (void)xs; (void)ys;
    } else {
        FCEU_MAYBE_UNUSED uint8_t zz;
    }

    // pputile.inc:17-41 — pixel rendering for tile index >= 2.
    // Tiles 0 and 1 are first-two-buffer entries (pre-loaded); render
    // only when X1 reaches them in subsequent scanlines.
    if (X1 >= 2) {
        const uint8_t* S = PALRAM.data();
        uint32_t pixdata;

        pixdata = ppulut1[(pshift[0] >> (8 - XOffset)) & 0xFF]
                | ppulut2[(pshift[1] >> (8 - XOffset)) & 0xFF];

        pixdata |= ppulut3[XOffset | (atlatch << 3)];

        P[0] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[1] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[2] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[3] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[4] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[5] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[6] = S[pixdata & 0xF];
        pixdata >>= 4;
        P[7] = S[pixdata & 0xF];
        P += 8;
    }

    // pputile.inc:43-53 — name-table fetch: MMC5SP vs default.
    if constexpr ((Flags & kFlagMMC5SP) != 0) {
        // Body uses xs/ys here — must be in same branch (we re-
        // initialise in this branch to keep the if constexpr scope
        // flat for the compiler).
        const uint8_t xs = static_cast<uint8_t>(X1);
        const uint8_t ys = static_cast<uint8_t>(
            ((g_cpu.scanline_ref() >> 3) + MMC5HackSPScroll) & 0x1F);
        vadr = (MMC5HackExNTARAMPtr[xs | (ys << 5)] << 4) + (vofs & 7);
    } else {
        FCEU_MAYBE_UNUSED uint8_t zz = static_cast<uint8_t>(RefreshAddr & 0x1F);
        C = vnapage[(RefreshAddr >> 10) & 3];
        if constexpr ((Flags & kFlagVRC5Fetch) != 0) {
            uint8_t tmpd = QTAINTRAM[
                ((((RefreshAddr >> 10) & 3) >> ((qtaintramreg >> 1)) & 1) << 10)
                | (RefreshAddr & 0x3FF)];
            vofs = ((tmpd & 0x3F) << 12) | ((RefreshAddr >> 12) & 7);
        }
        vadr = (C[RefreshAddr & 0x3ff] << 4) + vofs;
    }

    // pputile.inc:55-57 — first hook opportunity (post-NT fetch).
    if constexpr ((Flags & kFlagHook) != 0) {
        PPU_hook(0x2000 | (RefreshAddr & 0xfff));
    }

    // pputile.inc:59-69 — attribute-table fetch.
    if constexpr ((Flags & kFlagMMC5SP) != 0) {
        const uint8_t xs = static_cast<uint8_t>(X1);
        const uint8_t ys = static_cast<uint8_t>(
            ((g_cpu.scanline_ref() >> 3) + MMC5HackSPScroll) & 0x1F);
        cc = MMC5HackExNTARAMPtr[0x3c0 + (xs >> 2) + ((ys & 0x1C) << 1)];
        cc = static_cast<uint8_t>(
            (cc >> ((xs & 2) + ((ys & 0x2) << 1))) & 3);
    } else if constexpr ((Flags & kFlagMMC5CHR1) != 0) {
        cc = (MMC5HackExNTARAMPtr[RefreshAddr & 0x3ff] & 0xC0) >> 6;
    } else {
        const uint8_t zz = static_cast<uint8_t>(RefreshAddr & 0x1F);
        FCEU_MAYBE_UNUSED uint8_t* C_dummy = vnapage[(RefreshAddr >> 10) & 3];
        cc = C_dummy[0x3c0 + (zz >> 2) + ((RefreshAddr & 0x380) >> 4)];
        cc = static_cast<uint8_t>(
            (cc >> ((zz & 2) + ((RefreshAddr & 0x40) >> 4))) & 3);
    }

    atlatch >>= 2;
    atlatch |= static_cast<uint32_t>(cc << 2);

    pshift[0] <<= 8;
    pshift[1] <<= 8;

    // pputile.inc:77-99 — pattern-table pointer resolution.
    if constexpr ((Flags & kFlagMMC5SP) != 0) {
        C = MMC5HackVROMPTR + vadr;
        C += ((MMC5HackSPPage & 0x3f & MMC5HackVROMMask) << 12);
    } else if constexpr ((Flags & kFlagMMC5CHR1) != 0) {
        C = MMC5HackVROMPTR;
        C += (((MMC5HackExNTARAMPtr[RefreshAddr & 0x3ff]) & 0x3f
               & MMC5HackVROMMask) << 12) + (vadr & 0xfff);
        C += (MMC50x5130 & 0x3) << 18;
    } else if constexpr ((Flags & kFlagMMC5) != 0) {
        C = MMC5BGVRAMADR(vadr);
    } else if constexpr ((Flags & kFlagVRC5Fetch) != 0) {
        // The VRC5 path reads tmpd again at this point — re-derive it
        // since the variable's scope ended in the NT-fetch branch.
        uint8_t tmpd = QTAINTRAM[
            ((((RefreshAddr >> 10) & 3) >> ((qtaintramreg >> 1)) & 1) << 10)
            | (RefreshAddr & 0x3FF)];
        if (tmpd & 0x40) {
            C = CHRptr[0] + vadr;
        } else {
            C = VRAMADR(vadr);
        }
    } else {
        C = VRAMADR(vadr);
    }
    (void)ScreenON;

    // pputile.inc:101-103 — second hook opportunity (post-pattern fetch).
    if constexpr ((Flags & kFlagHook) != 0) {
        PPU_hook(vadr);
    }

    // pputile.inc:105-132 — shift-register update: BG-only fetch vs
    // VRC5 vs default.
    if constexpr ((Flags & kFlagBGFetch) != 0) {
        if (RefreshAddr & 1) {
            if (ScreenON) {
                (void)C;  // RENDER_LOGP(C + 8) stub
            }
            pshift[0] |= C[8];
            pshift[1] |= C[8];
        } else {
            if (ScreenON) {
                (void)C;
            }
            pshift[0] |= C[0];
            pshift[1] |= C[0];
        }
    } else if constexpr ((Flags & kFlagVRC5Fetch) != 0) {
        pshift[0] |= C[0];
        uint8_t tmpd = QTAINTRAM[
            ((((RefreshAddr >> 10) & 3) >> ((qtaintramreg >> 1)) & 1) << 10)
            | (RefreshAddr & 0x3FF)];
        if (tmpd & 0x40) {
            pshift[1] |= (tmpd & 0x80) ? 0xFF : 0x00;
        } else {
            pshift[1] |= C[8];
        }
    } else {
        if (ScreenON) {
            (void)C;
        }
        pshift[0] |= C[0];
        if (ScreenON) {
            (void)(C + 8);
        }
        pshift[1] |= C[8];
    }

    // pputile.inc:134-137 — coarse X scroll wrap.
    if ((RefreshAddr & 0x1f) == 0x1f) {
        RefreshAddr ^= 0x41F;
    } else {
        RefreshAddr++;
    }

    // pputile.inc:139-141 — final hook opportunity.
    if constexpr ((Flags & kFlagHook) != 0) {
        PPU_hook(0x2000 | (RefreshAddr & 0xfff));
    }
}

}  // namespace fceu11::ppu

namespace fceu11::ppu {

// hotfix2 P0-4 (ARCH-1b): per-kind RefreshLine helpers. Each is a
// thin wrapper around the templated `FetchAndDrawTile<Flags>(...)`
// body. The dispatcher in ppu_rendering.cpp:340-413 will be moved
// here one branch at a time (Phase A's smoke-tested branch is
// `RefreshLineKind_Normal`, the rest remain macro-driven until
// per-mapper verification extends in v1.16 Phase B+).

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_Normal(int X1, uint32_t pshift[2], uint32_t& atlatch,
                            uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                            uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFNormal>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                               vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_MMC5SP(int X1, uint32_t pshift[2], uint32_t& atlatch,
                            uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                            uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFMMC5SP>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                               vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_MMC5CHR1(int X1, uint32_t pshift[2], uint32_t& atlatch,
                              uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                              uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFMMC5CHR1>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                                 vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_MMC5CHR1SP(int X1, uint32_t pshift[2], uint32_t& atlatch,
                                uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                                uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFMMC5CHR1SP>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                                   vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_MMC5Only(int X1, uint32_t pshift[2], uint32_t& atlatch,
                              uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                              uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFMMC5>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                             vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_Hook(int X1, uint32_t pshift[2], uint32_t& atlatch,
                         uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                         uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFHook>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                             vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_HookBGFetch(int X1, uint32_t pshift[2], uint32_t& atlatch,
                                 uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                                 uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFHookBGFetch>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                                    vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_BGFetch(int X1, uint32_t pshift[2], uint32_t& atlatch,
                             uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                             uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFBGFetch>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                                vnapage, ScreenON);
}

FCEU_ALWAYS_INLINE FCEU_HOT
void RefreshLineKind_VRC5Fetch(int X1, uint32_t pshift[2], uint32_t& atlatch,
                               uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                               uint8_t** vnapage, bool ScreenON)
{
    FetchAndDrawTile<kFVRC5Fetch>(X1, pshift, atlatch, P, RefreshAddr, vofs,
                                  vnapage, ScreenON);
}

// hotfix2 P0-3 + P0-4: explicit instantiations of every kind that the
// dispatcher (in ppu_rendering.cpp:340-413, when fully migrated) and
// the per-kind helpers above call. The template body's definition
// lives in this TU; explicit instantiation puts the symbol here so
// callers in other TUs (notably ppu_rendering.cpp) can link against
// it without re-emitting the body. The dispatcher currently calls
// only kFNormal (Phase A smoke-test scope); the remaining flags are
// pre-instantiated here so that follow-up PRs in Phase A can wire
// the dispatcher one branch at a time without needing to rebuild
// this TU.

template void FetchAndDrawTile<kFNormal       >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFMMC5         >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFMMC5SP       >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFMMC5CHR1     >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFMMC5CHR1SP   >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFHook         >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFHookBGFetch  >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFBGFetch      >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);
template void FetchAndDrawTile<kFVRC5Fetch    >(int, uint32_t[], uint32_t&, uint8_t*&, uint32_t&, uint32_t&, uint8_t**, bool);

}  // namespace fceu11::ppu
