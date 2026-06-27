// FCEUX11 — v1.7 Cartograph Phase F: Mmc3Cart subclass.
//
// Strategy A (v1.7 build plan §3.3): the cart subclass's on_power() simply
// delegates to the legacy Mapper4_Init body — specifically the M4Power()
// function pointer that Mapper4_Init installed (it adds the Karnov mirror
// hack on top of the generic GenMMC3Power). This keeps the Mapper4_Init /
// GenMMC3_Init bodies untouched (no behavior change risk) and exercises the
// v1.7 Cart virtual lifecycle against the largest popular NES mapper
// (~1500 lines of bank-switching + IRQ) as a PoC.
//
// MMC3 has a non-trivial reset (zero IRQ counter / latch / cmd, restore
// default bank registers) and registers a scanline IRQ hook during init, so
// on_reset() calls MMC3RegReset() and on_close() drops the IRQ hook the way
// Vrc6Cart does (fceu.cpp::ResetGameLoaded also nulls it but doing it in the
// cart subclass is more immediate and matches the VRC6 pattern).

#ifndef FCEU11_MMC3_CART_H
#define FCEU11_MMC3_CART_H

#include "cart_class.h"

namespace fceu11 {

class Mmc3Cart : public Mapper {
public:
    explicit Mmc3Cart(Bus& bus) noexcept { attach_bus(bus); }

    void on_power() noexcept override;
    void on_reset() noexcept override;   // calls MMC3RegReset() directly
    void on_close() noexcept override;    // releases MMC3 IRQ hook
};

} // namespace fceu11

#endif // FCEU11_MMC3_CART_H