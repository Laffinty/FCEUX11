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
    explicit MapperStrategyA(Bus& bus) noexcept {
        attach_bus(bus);
        // v1.8 Masonry Phase E.1: capture the legacy Power/Reset function
        // pointers set by the per-mapper Init (Mapper21_Init, Mapper2_Init,
        // etc.) BEFORE the iNES loader (src/ines.cpp:849-851) overwrites
        // them with CartInfo_PowerForward / ResetForward.  Without this
        // capture, MapperStrategyA::on_power() and on_reset() would call
        // currCartInfo->Power() / Reset() unconditionally, which after the
        // overwrite routes back through CartInfo_*Forward -> cart_obj's
        // virtual on_power / on_reset -> infinite recursion -> stack
        // overflow -> SEGFAULT.  Confirmed by E.1 diagnostic commit
        // 41d779a+1: the diagnostic showed info->Power and PowerForward
        // printing identical addresses during the second mapper load.
        if (currCartInfo) {
            // Only capture if the function pointer is a real legacy
            // function (not already a forwarder — that means a previous
            // Cart subclass is somehow still wired up, which shouldn't
            // happen in the iNES loader path but we guard defensively).
            if (currCartInfo->Power &&
                currCartInfo->Power != &CartInfo_PowerForward) {
                legacy_power_ = currCartInfo->Power;
            }
            if (currCartInfo->Reset &&
                currCartInfo->Reset != &CartInfo_ResetForward) {
                legacy_reset_ = currCartInfo->Reset;
            }
        }
    }

    void on_power() noexcept override {
        if (legacy_power_) legacy_power_();
    }
    void on_reset() noexcept override {
        if (legacy_reset_) legacy_reset_();
    }
    void on_close() noexcept override {
        release_mapper_resources();
    }

private:
    void (*legacy_power_)(void) = nullptr;
    void (*legacy_reset_)(void) = nullptr;
};

} // namespace fceu11

#endif // FCEU11_BOARDS_MAPPER_STRATEGY_A_H