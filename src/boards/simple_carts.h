#ifndef FCEU11_BOARDS_SIMPLE_CARTS_H
#define FCEU11_BOARDS_SIMPLE_CARTS_H

#include "mapper_strategy_a.h"
#include "registry.h"          // MapperEntry, MapperEntryRegister

namespace fceu11 {

// Phase E.2 step 7: Cart subclasses for simple P0 mappers that didn't
// get caught in Phase D's batches.  All inherit MapperStrategyA; each
// registers a MapperEntryRegister in its own source file.  save_mapper_state
// stays the 16-byte MapperStrategyA default until per-mapper state capture
// lands in a followup.
//
// mapper_number() in MapperStrategyA's default emit mapper_number()
// automatically, so per-mapper subclass overrides aren't needed for
// the byte-diff to differentiate between these classes.
class Mmc2Cart : public MapperStrategyA { public: explicit Mmc2Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mmc4Cart : public MapperStrategyA { public: explicit Mmc4Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper15Cart : public MapperStrategyA { public: explicit Mapper15Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper48Cart : public MapperStrategyA { public: explicit Mapper48Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_SIMPLE_CARTS_H
