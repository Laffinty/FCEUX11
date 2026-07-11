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
// Mmc3BaseCart exposes the common on_power / on_reset / on_close behavior.
// on_power() calls GenMMC3Power() directly (v1.15 B.1: eliminated legacy
// function pointer delegation). The Mmc3Cart subclass (Mapper 4) overrides
// on_power() to call M4Power() which adds the Karnov mirror hack.
//
// 24 derived cart classes (each in its own header) just set
// mapper_number_ in the constructor and rely on Mmc3BaseCart's behavior.

#ifndef FCEU11_BOARDS_MMC3_BASE_CART_H
#define FCEU11_BOARDS_MMC3_BASE_CART_H

#include "cart_class.h"

namespace fceu11 {

class Mmc3BaseCart : public Mapper {
public:
    explicit Mmc3BaseCart(Bus& bus) noexcept;

    // v1.15 B.1: Calls GenMMC3Power() directly — no legacy function pointer
    // indirection. All 23 non-Mapper4 MMC3 variants use this default.
    // Mmc3Cart (Mapper 4) overrides to call M4Power() instead.
    void on_power() noexcept override;

    // All 24 MMC3 variants share MMC3RegReset, so call it directly.
    void on_reset() noexcept override;

    // v1.8 Masonry Phase E.2 step 5: default body byte-diff for all 24 MMC3
    // variants.  Returns 16 bytes (mapper_number + mirror + WRAM + battery),
    // matching MapperStrategyA.  The PoC Mmc3Cart subclass overrides with
    // its 17-byte register file; the 21 derived variants get this default
    // until per-variant state capture lands.
    std::vector<uint8_t> save_mapper_state() const noexcept override;

    // GameHBIRQHook may have been wired by the per-mapper Init (Kick
    // Master / Shougi hacks).  Null it out on close.  WRAM/CHRRAM
    // release is handled by GenMMC3Close via iNESCart.Close().
    void on_close() noexcept override;
};

} // namespace fceu11

#endif // FCEU11_BOARDS_MMC3_BASE_CART_H