// FCEUX11 — v1.8 Masonry §1.4: Mmc3BaseCart.
//
// 24 MMC3-derived iNES mappers (12, 37, 44, 45, 47, 49, 52, 74, 114,
// 115, 116, 118, 119, 165, 192, 194, 195, 198, 205, 245, 249, 250, 254,
// 406) share the same MMC3 IRQ + bank-switching core (mmc3.cpp) but
// each has a distinct Power() helper (M12Power, M37Power, M44Power, ...)
// that selects CHR/PRG mask sizes and optionally wires extra registers.
//
// Rather than 24 near-identical Cart subclasses — each calling a
// distinct Power function via the legacy info->Power function pointer —
// Mmc3BaseCart exposes the common on_reset / on_close behavior and
// lets on_power dispatch via currCartInfo->Power (Strategy A, same
// pattern as the v1.7 PoC subclasses).
//
// 24 derived cart classes (each in its own header) just set
// mapper_number_ in the constructor and rely on Mmc3BaseCart's behavior.

#ifndef FCEU11_BOARDS_MMC3_BASE_CART_H
#define FCEU11_BOARDS_MMC3_BASE_CART_H

#include "cart_class.h"

namespace fceu11 {

class Mmc3BaseCart : public Mapper {
public:
    explicit Mmc3BaseCart(Bus& bus) noexcept { attach_bus(bus); }

    // Strategy A: defer to the legacy currCartInfo->Power function pointer
    // that the per-mapper Init() (Mapper12_Init, Mapper37_Init, ...) sets
    // before Cart creation.  All 24 MMC3 variants follow this pattern.
    void on_power() noexcept override;

    // All 24 MMC3 variants share MMC3RegReset, so call it directly.
    void on_reset() noexcept override;

    // GameHBIRQHook may have been wired by the per-mapper Init (Kick
    // Master / Shougi hacks).  Null it out on close.  WRAM/CHRRAM
    // release is handled by GenMMC3Close via iNESCart.Close().
    void on_close() noexcept override;

    // All 24 MMC3 variants share MMC3's register file; the default
    // empty snapshot is correct for now.  Phase D.9 (this phase)
    // overrides on Mmc3Cart proper (the v1.7 PoC subclass); the
    // 24 derived classes inherit its implementation.
};

} // namespace fceu11

#endif // FCEU11_BOARDS_MMC3_BASE_CART_H