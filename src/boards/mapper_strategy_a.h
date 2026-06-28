// FCEUX11 — v1.8 Masonry: Strategy A Mapper base.
//
// v1.7 §3.3 Strategy A: the cart subclass's on_power() / on_reset()
// defer to the legacy CartInfo function pointers set by the per-board
// Init function (info->Power = M12Power, info->Reset = MMC3RegReset,
// etc.).  This is the minimum-risk migration path: zero changes to the
// 20000+ lines of mapper-specific code in src/boards/*.cpp.
//
// Most P0 mappers (UNROM/CNROM/ANROM/CPROM/Mapper28 and the 24 MMC3
// variants and the Phase E/F expansion audio boards) use this pattern.
// Subclasses just inherit MapperStrategyA and provide an Init name
// when registering with MapperEntryRegister.

#ifndef FCEU11_BOARDS_MAPPER_STRATEGY_A_H
#define FCEU11_BOARDS_MAPPER_STRATEGY_A_H

#include "cart_class.h"
#include "_cart_helpers.h"
#include "../cart.h"

namespace fceu11 {

class MapperStrategyA : public Mapper {
public:
    explicit MapperStrategyA(Bus& bus) noexcept { attach_bus(bus); }

    void on_power() noexcept override {
        if (currCartInfo && currCartInfo->Power) currCartInfo->Power();
    }
    void on_reset() noexcept override {
        if (currCartInfo && currCartInfo->Reset) currCartInfo->Reset();
    }
    void on_close() noexcept override {
        release_mapper_resources();
    }
};

} // namespace fceu11

#endif // FCEU11_BOARDS_MAPPER_STRATEGY_A_H