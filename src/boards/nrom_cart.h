// FCEUX11 — v1.7 Cartograph Phase E: NromCart subclass.
//
// Strategy A (v1.7 build plan §3.3): the cart subclass's on_power() simply
// delegates to the legacy NROM_Init(currCartInfo) function pointer path.
// This keeps the NROM_Init body untouched (no behavior change risk) and
// exercises the v1.7 Cart virtual lifecycle against the simplest possible
// mapper as a PoC before tackling MMC1 / MMC3 in Phase F.
//
// NROM has no reset logic and no expansion audio, so on_reset() and
// install_expansion_audio() inherit the Cart defaults (no-op).

#ifndef FCEU11_NROM_CART_H
#define FCEU11_NROM_CART_H

#include "cart_class.h"

namespace fceu11 {

class NromCart : public Mapper {
public:
    explicit NromCart(Bus& bus) noexcept { attach_bus(bus); }

    void on_power() noexcept override;
    void on_reset() noexcept override {}  // NROM has no reset logic.
    void on_close() noexcept override {}  // CartInfo::ClearGameSave handles WRAM.

    // v1.8 Masonry §6.1: byte-diff snapshot of mapper state.
    std::vector<uint8_t> save_mapper_state() const noexcept override;
};

} // namespace fceu11

#endif // FCEU11_NROM_CART_H