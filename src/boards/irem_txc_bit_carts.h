// FCEUX11 — v1.8 Masonry Phase D.7: IREM G-101 (32/33/34) + TXC (36) + Bit Corp (38).
//
// All four mapper families follow the MapperStrategyA pattern; each Cart
// subclass is a single-line forwarding ctor.

#ifndef FCEU11_BOARDS_IREM_TXC_BIT_CARTS_H
#define FCEU11_BOARDS_IREM_TXC_BIT_CARTS_H

#include "mapper_strategy_a.h"

namespace fceu11 {

class Mapper32Cart : public MapperStrategyA { public: explicit Mapper32Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper33Cart : public MapperStrategyA { public: explicit Mapper33Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper34Cart : public MapperStrategyA { public: explicit Mapper34Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper36Cart : public MapperStrategyA { public: explicit Mapper36Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper38Cart : public MapperStrategyA { public: explicit Mapper38Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_IREM_TXC_BIT_CARTS_H