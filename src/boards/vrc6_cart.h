// FCEUX11 — v1.7 Cartograph Phase E: Vrc6Cart subclass.
//
// Strategy A (v1.7 build plan §3.3): the cart subclass's on_power() simply
// delegates to the legacy Mapper24_Init / Mapper26_Init(currCartInfo) function
// pointer path. This is the first PoC that carries a non-trivial mapper
// (~140 lines of bank-switching) and exercises the v1.6 §11.1
// install_expansion_audio contract (cart-side g_apu.set_exp_sound()).
//
// VRC6 has no reset logic and uses one mapper number (24 / 26 — both
// dispatched through this subclass based on which Init was called by the
// loader). on_close() releases the IRQ hook registered by Mapper24/26_Init.

#ifndef FCEU11_VRC6_CART_H
#define FCEU11_VRC6_CART_H

#include "cart_class.h"

namespace fceu11 {

class Vrc6Cart : public Mapper {
public:
    explicit Vrc6Cart(Bus& bus, uint32_t mapper_no) noexcept {
        attach_bus(bus);
        mapper_number_ = mapper_no;
    }

    void on_power() noexcept override;
    void on_reset() noexcept override {}  // VRC6 has no reset logic.
    void on_close() noexcept override;    // releases VRC6 IRQ hook.

    // v1.6 §11.1 contract: route g_vrc6_audio into g_apu via set_exp_sound().
    // Triggered by iNESLoad / UNIFLoad after cart assignment, before PowerNES.
    void install_expansion_audio(class Apu& apu) noexcept override;
};

} // namespace fceu11

#endif // FCEU11_VRC6_CART_H