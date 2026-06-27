// FCEUX11 — v1.7 Cartograph Phase F: Mmc1Cart subclass.
//
// Strategy A (v1.7 build plan §3.3): the cart subclass's on_power() simply
// delegates to the legacy GenMMC1Power(currCartInfo's Power function pointer)
// function pointer path. This keeps the GenMMC1Init body untouched (no
// behavior change risk) and exercises the v1.7 Cart virtual lifecycle against
// the most popular NES mapper (~300 lines of bank-switching) as a PoC.
//
// MMC1 has a non-trivial reset (the buffer + shift register, plus four
// data registers), so on_reset() calls the legacy MMC1CMReset() helper
// directly. MMC1 does not register an IRQ hook or expansion audio, so
// on_close() and install_expansion_audio() inherit the Cart defaults (no-op).

#ifndef FCEU11_MMC1_CART_H
#define FCEU11_MMC1_CART_H

#include "cart_class.h"

namespace fceu11 {

class Mmc1Cart : public Mapper {
public:
    explicit Mmc1Cart(Bus& bus) noexcept { attach_bus(bus); }

    void on_power() noexcept override;
    void on_reset() noexcept override;   // calls MMC1CMReset() directly
    void on_close() noexcept override {} // CartInfo clear path frees WRAM.
};

} // namespace fceu11

#endif // FCEU11_MMC1_CART_H