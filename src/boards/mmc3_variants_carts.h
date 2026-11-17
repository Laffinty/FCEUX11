// FCEUX11 — v1.8 Masonry §1.4: 26 MMC3 variant Cart subclasses.
//
// Each derived class is header-only and just stores mapper_number_
// in the base via the Mapper ctor.  All behavior (on_power via
// Strategy A, on_reset = MMC3RegReset, on_close = GameHBIRQHook null)
// lives in Mmc3BaseCart.
//
// Mapping of mapper # → Init function name (used by MapperEntryRegister):
//
//   12 → Mapper12_Init   37 → Mapper37_Init   44 → Mapper44_Init
//   45 → Mapper45_Init   47 → Mapper47_Init   49 → Mapper49_Init
//   52 → Mapper52_Init   74 → Mapper74_Init   114 → Mapper114_Init
//  115 → Mapper115_Init  116 → UNLSL12_Init    118 → TKSROM_Init
//  119 → Mapper119_Init  165 → Mapper165_Init  192 → Mapper192_Init
//  191 → Mapper191_Init  194 → Mapper194_Init  195 → Mapper195_Init
//  198 → Mapper198_Init  199 → Mapper199_Init  205 → Mapper205_Init
//  245 → Mapper245_Init  249 → Mapper249_Init  250 → Mapper250_Init
//  254 → Mapper254_Init  406 → Mapper406_Init
//
// (Names verified against src/boards/mmc3.cpp in Phase D.3.)
//
// Two of the 26 do not fit the one-line pattern:
//
//   * 191 has no power function of its own — GenMMC3Power() is what the
//     legacy path ran for it — so the plain base class is exactly right.
//   * 199 has its own power function (boards/199.cpp: M199Power) which
//     seeds the DBZ expanded registers and installs the 0x8001 write
//     handler.  Mmc3BaseCart::on_power() calls GenMMC3Power() instead,
//     which would leave EXPREGS at zero — the 0xC000/0xE000 windows would
//     then point at PRG bank 0 and the game would run the wrong code — so
//     Mapper199Cart overrides on_power().
//
// v2.0.5 A2 (2026-11-17): the same defect applies to 18 MORE of these
// classes, not just 198/199.  The loader overwrites info->Power with
// CartInfo_PowerForward whenever the registry produced a Cart object
// (src/ines.cpp), so for every REGISTERED mapper the board's own
// `info->Power = MxxPower` assignment never runs.  Mmc3BaseCart::on_power()
// hardcodes GenMMC3Power(), which installs only the generic MMC3 command /
// IRQ handlers — the per-board write and read windows, the EXPREGS seeds
// and the extra IRQ wiring of those 18 boards were therefore NEVER
// installed, and the boards could not work.
//
// The fix is per class: each of the 18 overrides calls its own board power
// function directly (the same pattern as the pre-existing Mapper198Cart /
// Mapper199Cart overrides; the board power functions are no longer static,
// they are declared in mmc3.cpp and, for mapper 116, in 116.cpp).
//
// The numbers (18): 12 37 44 45 47 49 52 114 115 116 165 195 205 245 249
// 250 254 406.  The v2.0.5 plan §1.4(B) listed 17 of them — mapper 165 was
// missing from that list (Mapper165_Init sets info->Power = M165Power and
// mapper 165 IS registered, so M165Power was being swallowed as well); the
// list was corrected to 18 when the wiring was implemented.
//
// The boards that must keep the base class behavior are the ones whose Init
// leaves info->Power at GenMMC3Power: 74, 118, 119, 191, 192, 194 (plus 165
// before this change).  Those must NOT get an override — the base class is
// exactly right for them.
//
// ⚠️ The board-specific RESET functions (info->Reset = M12Reset / M37Reset
// / M45Reset / M49Reset / M52Reset / M114Reset / M205Reset) are swallowed by
// CartInfo_ResetForward in exactly the same way and are still NOT wired —
// see the v2.0.5 plan's observation register.  This change is on_power only.

#ifndef FCEU11_BOARDS_MMC3_VARIANTS_CARTS_H
#define FCEU11_BOARDS_MMC3_VARIANTS_CARTS_H

#include "mmc3_base_cart.h"

namespace fceu11 {

// v2.0.5 A2: the 18 classes whose board has its own power function override
// on_power() and call that function directly.  See the file header comment
// above for why the legacy `info->Power` assignment cannot be relied on.
#define FCEUX11_MMC3_VARIANT_CART_WITH_POWER(name)                          \
    class name : public Mmc3BaseCart {                                      \
    public:                                                                \
        explicit name(Bus& bus) noexcept : Mmc3BaseCart(bus) {}            \
        void on_power() noexcept override;                                  \
    }

FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper12Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper37Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper44Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper45Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper47Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper49Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper52Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper114Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper115Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper116Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper165Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper195Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper205Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper245Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper249Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper250Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper254Cart);
FCEUX11_MMC3_VARIANT_CART_WITH_POWER(Mapper406Cart);

#undef FCEUX11_MMC3_VARIANT_CART_WITH_POWER

// Boards with NO power function of their own -- the base class
// (GenMMC3Power) is exactly right for these, so they must NOT override
// on_power().  Do not add an override here by analogy: the base class
// would then run the wrong power function.
class Mapper74Cart  : public Mmc3BaseCart { public: explicit Mapper74Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper118Cart : public Mmc3BaseCart { public: explicit Mapper118Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper119Cart : public Mmc3BaseCart { public: explicit Mapper119Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper192Cart : public Mmc3BaseCart { public: explicit Mapper192Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper191Cart : public Mmc3BaseCart { public: explicit Mapper191Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper194Cart : public Mmc3BaseCart { public: explicit Mapper194Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper198Cart : public Mmc3BaseCart {
public:
    explicit Mapper198Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {}
    // Mapper 198 uses mapper 195's power function (upstream:
    // `info->Power = M195Power`), which restores the 16 KiB WRAM window at
    // $5000 and installs its read/write handlers.  The base class's
    // GenMMC3Power() does neither, so the board would come up without WRAM.
    // See the header comment above for why the legacy `info->Power`
    // assignment cannot be relied on.
    void on_power() noexcept override;
};
class Mapper199Cart : public Mmc3BaseCart {
public:
    explicit Mapper199Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {}
    // Mapper 199 has a power function of its own (boards/199.cpp).  See the
    // header comment above for why the base class's GenMMC3Power() is not
    // enough for this one.
    void on_power() noexcept override;
};

} // namespace fceu11

#endif // FCEU11_BOARDS_MMC3_VARIANTS_CARTS_H