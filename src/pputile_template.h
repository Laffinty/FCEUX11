// pputile_template.h
//
// hotfix2 P0-3 (ARCH-1a): `pputile.inc` rewritten as a single
// template <uint8_t Flags> function with `if constexpr` isolating
// cold paths. Replaces the 10 dispatcher `#include "pputile.inc"`
// sites in ppu_rendering.cpp:340-413 with template specializations.
//
// Flag bits:
//   0x01  PPUT_MMC5       (MMC5 BG path: name/attribute via MMC5 BG fetcher)
//   0x02  PPUT_MMC5SP     (MMC5 split-screen scroll: name/attr from ExNTARAM)
//   0x04  PPUT_MMC5CHR1   (MMC5 CHR-mode 1: pattern from CHR banking)
//   0x08  PPUT_HOOK       (PPU_hook callback after each fetch)
//   0x10  PPU_BGFETCH     (debugger BG-only fetch — PEC586 quirk)
//   0x20  PPU_VRC5FETCH   (VRC5 nametable redirection — QTAI mapper)
//
// P0-3 strategy (per §一 ARCH-1): one function body, eight
// specialisations (`if constexpr` collapses to no runtime branch for
// unused flags). The dispatcher explicitly instantiates one of
// `kFlagNormal`, `kFlagMMC5`, `kFlagMMC5SP`, `kFlagMMC5CHR1SP`,
// `kFlagMMC5CHR1`, `kFlagHook`, `kFlagHookBGFetch`, `kFlagBGFetch`,
// `kFlagVRC5Fetch` — see ppu_rendering.cpp:340.
//
// The body of the function is preserved byte-for-byte from the
// original `pputile.inc` (kept on disk as `tests/golden/pputile.inc`
// for cross-checking after this commit), with `#ifdef X` →
// `if constexpr ((Flags & XFlag) != 0)` and `#define X` →
// `if constexpr ((Flags & XFlag) != 0) { /* ... */ }` rewrites.
//
// Inputs (passed by reference because they are mutated in place):
//   X1            current tile index (0..32)
//   XOffset       fine X scroll (passed via g_ppu.fine_x_scroll())
//   pshift[2]     bg pattern shift registers (in/out)
//   atlatch       bg attribute latch (in/out)
//   P             output pointer (advanced by 8 per call when render)
//   RefreshAddr   PPU internal video address (in/out)
//   vofs          VROM bank offset (in/out)
//   vnapage       4-entry pointer array for nametables
//   ScreenON      runtime bit (PPU[1] & 0x08)
//   PEC586Hack    bool: PEC586 BG-only fetch quirk
//
// Globals referenced (read by template body, looked up by caller):
//   PALRAM, ppulut1/2/3, PPU_hook, MMC5Hack* (when MMC5 flag set),
//   QTAINTRAM / qtaintramreg / CHRptr (when VRC5 flag set),
//   VRAMADR/MMC5BGVRAMADR/MMC5HackVROMPTR macros.

#ifndef FCEU11_PPUTILE_TEMPLATE_H
#define FCEU11_PPUTILE_TEMPLATE_H

#include <cstdint>
#include <tuple>

#include "compiler_attrs.h"
#include "ppu_rendering.h"
#include "ppu_state.h"   // hotfix3 C-2: PALRAM type guard for FetchAndDrawTile template

// hotfix3 C-2: PALRAM size guard. Mirror of ppu_rendering.cpp:438 (>= 0x10);
// stricter here (>= 0x20) because the FetchAndDrawTile template body in
// pputile_template.cpp:69 uses PALRAM.data() and the dispatcher instantiates
// 9 specialisations — any shrink of PALRAM below 32 bytes must break compile,
// not silently produce out-of-range reads.
static_assert(std::tuple_size_v<decltype(PALRAM)>> = 0x20,
              "PALRAM must hold offsets 0..0x1F (32 bytes); "
              "pputile_template accesses PALRAM via PALRAM.data() in "
              "FetchAndDrawTile. Shrinking below 32 B will silently corrupt "
              "palette-indexed pixel output.");

namespace fceu11::ppu {

// Flag constants — see file header comment for meaning.
inline constexpr uint8_t kFlagMMC5       = 0x01;
inline constexpr uint8_t kFlagMMC5SP     = 0x02;
inline constexpr uint8_t kFlagMMC5CHR1   = 0x04;
inline constexpr uint8_t kFlagHook       = 0x08;
inline constexpr uint8_t kFlagBGFetch    = 0x10;
inline constexpr uint8_t kFlagVRC5Fetch  = 0x20;

// Convenience pre-composed flag values for the dispatcher paths.
// The base flag bits (kFlagMMC5/kFlagMMC5SP/...) defined above are
// the "atomic" flags; these pre-composites mirror the original
// `#define` state combinations in the 10 dispatcher include sites.
// (Renamed to `kF*` to avoid clashing with the atomic names above.)
inline constexpr uint8_t kFNormal        = uint8_t{0};
inline constexpr uint8_t kFMMC5          = kFlagMMC5;
inline constexpr uint8_t kFMMC5SP        = uint8_t{kFlagMMC5 | kFlagMMC5SP};
inline constexpr uint8_t kFMMC5CHR1SP    = uint8_t{kFlagMMC5 | kFlagMMC5SP | kFlagMMC5CHR1};
inline constexpr uint8_t kFMMC5CHR1      = uint8_t{kFlagMMC5 | kFlagMMC5CHR1};
inline constexpr uint8_t kFHook          = kFlagHook;
inline constexpr uint8_t kFHookBGFetch   = uint8_t{kFlagHook | kFlagBGFetch};
inline constexpr uint8_t kFBGFetch       = kFlagBGFetch;
inline constexpr uint8_t kFVRC5Fetch     = kFlagVRC5Fetch;

template <uint8_t Flags>
FCEU_ALWAYS_INLINE FCEU_HOT
void FetchAndDrawTile(int X1, uint32_t pshift[2], uint32_t& atlatch,
                      uint8_t*& P, uint32_t& RefreshAddr, uint32_t& vofs,
                      uint8_t** vnapage, bool ScreenON);

// hotfix2 P0-4 (ARCH-1b): the RefreshLine dispatcher used to live as
// one chunky `if (MMC5Hack) { ... } else if (PPU_hook) { ... } else ...`
// chain inside `ppu_rendering.cpp:340-413`, which forced every kind to
// pull in all mapper dispatch code into one TU. P0-4 splits that into
// per-kind helper functions, each invoked through a `RefreshKind`
// enum selected once per `RefreshLine` call. This reduces mapper-
// switch I-cache churn and makes each kind's emitted code easier to
// audit.
enum class RefreshKind {
    Normal = 0,
    MMC5SP,
    MMC5CHR1SP,
    MMC5CHR1,
    MMC5Only,
    Hook,
    HookBGFetch,
    BGFetch,
    VRC5Fetch,
};

inline constexpr int kNumRefreshKinds = 9;

}  // namespace fceu11::ppu

// Out-of-line definition lives in pputile_template.cpp.
#include "ppu_rendering.h"  // for ScreenON macro; PPU_hook declaration; etc.

#endif  // FCEU11_PPUTILE_TEMPLATE_H
