// FCEUX11 — v1.8 Masonry §1.4: Mmc3BaseCart implementation.

#include "mmc3_base_cart.h"

#include "../fceu.h"            // DECLFR/DECLFW macros (must precede cart.h + mmc3.h)
#include "../cart.h"           // currCartInfo + CartInfo (uses DECLFR/DECLFW)
#include "_cart_helpers.h"     // release_mapper_resources
#include "mmc3.h"              // MMC3RegReset (uses DECLFW)

namespace fceu11 {

Mmc3BaseCart::Mmc3BaseCart(Bus& bus) noexcept {
    attach_bus(bus);
    // v1.8 Masonry Phase E.1: capture the legacy Power function pointer
    // set by the per-mapper Init (Mapper12_Init, Mapper37_Init, ...)
    // before the iNES loader overwrites it with CartInfo_PowerForward.
    // Without this, Mmc3BaseCart::on_power() would route through the
    // forwarder back to itself -> infinite recursion -> SEGFAULT.
    // Same pattern as MapperStrategyA (mapper_strategy_a.h:25-52).
    if (currCartInfo && currCartInfo->Power &&
        currCartInfo->Power != &CartInfo_PowerForward) {
        legacy_power_ = currCartInfo->Power;
    }
}

void Mmc3BaseCart::on_power() noexcept {
    // Strategy A (v1.7 §3.3): call the legacy power function.
    // v1.8 Masonry Phase E.1: the 24 MMC3 variants use the same
    // capture-in-constructor pattern as MapperStrategyA to avoid the
    // iNES loader's overwrite of currCartInfo->Power with
    // CartInfo_PowerForward (which would cause infinite recursion).
    if (legacy_power_) legacy_power_();
}

void Mmc3BaseCart::on_reset() noexcept {
    // All 24 MMC3 variants share MMC3RegReset.
    MMC3RegReset();
}

void Mmc3BaseCart::on_close() noexcept {
    // Module-level GameHBIRQHook may have been wired by per-mapper
    // Init (Kick Master / Shougi hacks).  Null it out.
    release_mapper_resources();
}

} // namespace fceu11
// v1.8 Masonry Phase D.3: include cart.h AFTER mmc3.h to ensure CartInfo
// is fully defined (mmc3.h only forward-declares CartInfo; the function
// body in mmc3_base_cart.cpp doesn't dereference info so this is fine).
#include "../cart.h"
