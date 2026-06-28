// FCEUX11 — v1.8 Masonry §1.4: Mmc3BaseCart implementation.

#include "mmc3_base_cart.h"

#include "../cart.h"           // currCartInfo
#include "_cart_helpers.h"     // release_mapper_resources
#include "mmc3.h"              // MMC3RegReset

namespace fceu11 {

void Mmc3BaseCart::on_power() noexcept {
    // Strategy A (v1.7 §3.3): call the legacy power function via
    // currCartInfo->Power.  Each of the 24 Init helpers (Mapper12_Init,
    // Mapper37_Init, Mapper44_Init, ...) sets info->Power = M<N>Power,
    // which we invoke here.  This preserves byte-identical behavior.
    if (currCartInfo && currCartInfo->Power) {
        currCartInfo->Power();
    }
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