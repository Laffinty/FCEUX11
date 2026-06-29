// FCEUX11 — v1.8 Masonry Phase D.4: simple P0 mapper Cart subclasses.
//
// Five simple bank-switching mappers (UNROM 2, CNROM 3, ANROM 7, CPROM
// 13, Mapper28) all live in src/boards/datalatch.cpp (4) or
// src/boards/28.cpp (Mapper28).  Each Cart subclass is a single line
// inheriting MapperStrategyA; the per-mapper Init function (set in
// bmap[]) configures currCartInfo->Power/Reset/Close at load time.
//
// Registration happens at the bottom of each respective .cpp via a
// MapperEntryRegister static instance.

#ifndef FCEU11_BOARDS_DATALATCH_CARTS_H
#define FCEU11_BOARDS_DATALATCH_CARTS_H

#include "mapper_strategy_a.h"

namespace fceu11 {

// v1.8 Masonry Phase E.2 step 2a: UNROM/CNROM/ANROM/CPROM each capture the
// shared latche byte so mapper_byte_diff_test detects accidental latch
// register regressions.  Override body lives in datalatch.cpp.
class UnromCart    : public MapperStrategyA { public: explicit UnromCart(Bus& bus) noexcept    : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class CnromCart    : public MapperStrategyA { public: explicit CnromCart(Bus& bus) noexcept    : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class AnromCart    : public MapperStrategyA { public: explicit AnromCart(Bus& bus) noexcept    : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class CpromCart    : public MapperStrategyA { public: explicit CpromCart(Bus& bus) noexcept    : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper28Cart : public MapperStrategyA { public: explicit Mapper28Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_DATALATCH_CARTS_H