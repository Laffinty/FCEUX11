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
// Phase E.2 step 8: round out P0 mappers (mapper 16 Bandai, 18 Magic Floor).
class Mapper16Cart : public MapperStrategyA { public: explicit Mapper16Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper18Cart : public MapperStrategyA { public: explicit Mapper18Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
// Phase E.2 step 9.1: backfill Cart subclasses for the 4 P0 mappers that
// step 1 added to mapper_byte_diff_test but never received a Cart subclass.
// Color Dreams (11) and GNROM (66) live in src/boards/datalatch.cpp; their
// save_mapper_state bodies mirror the UNROM/CNROM/ANROM/CPROM pattern
// (MapperStrategyA 16-byte default + 1 byte latche).  VRC7 (85) and MMC5 (5)
// are larger boards; they stay on the 16-byte MapperStrategyA default for
// now and gain per-board state in Phase E.2 step 11/12 (ExpansionAudio
// subclassing).  Cart subclass declarations are forward-only here; the
// save_mapper_state() overrides live next to the MapperEntryRegister in
// src/boards/datalatch.cpp / vrc7.cpp / mmc5.cpp.
class ColorDreamsCart : public MapperStrategyA { public: explicit ColorDreamsCart(Bus& bus) noexcept : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class GnromCart : public MapperStrategyA { public: explicit GnromCart(Bus& bus) noexcept : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Vrc7Cart : public MapperStrategyA { public: explicit Vrc7Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mmc5Cart : public MapperStrategyA { public: explicit Mmc5Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
// v1.8 Masonry Phase E.2 step 9.2: NES-EVENT NWC1990 (mapper 105) is
// MMC1-based but lives in mmc1.cpp.  Cart subclass + registration lands
// in mmc1.cpp's MapperEntryRegister block.
class Mapper105Cart : public MapperStrategyA { public: explicit Mapper105Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
// v1.8 Masonry Phase E.2 step 9.2: P1 Latch-family mappers (70, 78, 86, 87,
// 89, 94, 97) live in src/boards/datalatch.cpp and share the Latch_*
// infrastructure with the P0 batch.  Each emits a 17-byte body
// (MapperStrategyA 16-byte default + 1-byte latche).
class Mapper70Cart  : public MapperStrategyA { public: explicit Mapper70Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper78Cart  : public MapperStrategyA { public: explicit Mapper78Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper86Cart  : public MapperStrategyA { public: explicit Mapper86Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper87Cart  : public MapperStrategyA { public: explicit Mapper87Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper89Cart  : public MapperStrategyA { public: explicit Mapper89Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper94Cart  : public MapperStrategyA { public: explicit Mapper94Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
class Mapper97Cart  : public MapperStrategyA { public: explicit Mapper97Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} std::vector<uint8_t> save_mapper_state() const noexcept override; };
// v1.8 Masonry Phase E.2 step 9.3: 24 P1 mappers outside the Latch family.
// Each inherits MapperStrategyA (16-byte default body).  Registrations land
// in their respective source files (51.cpp / 57.cpp / 62.cpp / 65.cpp / 67.cpp
// / 68.cpp / 71.cpp / 72.cpp / 73.cpp / 75.cpp / 77.cpp / 79.cpp / 80.cpp /
// 82.cpp / 83.cpp / 88.cpp / 90.cpp / 91.cpp / 96.cpp / 99.cpp / addrlatch.cpp
// / tangen.cpp / vrc1.cpp / vrc3.cpp / yoko.cpp).
class Mapper51Cart  : public MapperStrategyA { public: explicit Mapper51Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper57Cart  : public MapperStrategyA { public: explicit Mapper57Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper61Cart  : public MapperStrategyA { public: explicit Mapper61Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper62Cart  : public MapperStrategyA { public: explicit Mapper62Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper64Cart  : public MapperStrategyA { public: explicit Mapper64Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper65Cart  : public MapperStrategyA { public: explicit Mapper65Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper67Cart  : public MapperStrategyA { public: explicit Mapper67Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper68Cart  : public MapperStrategyA { public: explicit Mapper68Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper71Cart  : public MapperStrategyA { public: explicit Mapper71Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper72Cart  : public MapperStrategyA { public: explicit Mapper72Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper73Cart  : public MapperStrategyA { public: explicit Mapper73Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper75Cart  : public MapperStrategyA { public: explicit Mapper75Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper77Cart  : public MapperStrategyA { public: explicit Mapper77Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper79Cart  : public MapperStrategyA { public: explicit Mapper79Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper80Cart  : public MapperStrategyA { public: explicit Mapper80Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper82Cart  : public MapperStrategyA { public: explicit Mapper82Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper83Cart  : public MapperStrategyA { public: explicit Mapper83Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper88Cart  : public MapperStrategyA { public: explicit Mapper88Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper90Cart  : public MapperStrategyA { public: explicit Mapper90Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper91Cart  : public MapperStrategyA { public: explicit Mapper91Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper92Cart  : public MapperStrategyA { public: explicit Mapper92Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper93Cart  : public MapperStrategyA { public: explicit Mapper93Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper96Cart  : public MapperStrategyA { public: explicit Mapper96Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper99Cart  : public MapperStrategyA { public: explicit Mapper99Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
// v1.8 Phase E.2 step 9.4: remaining P1 mappers (53, 58, 60, 76, 95).
class Mapper53Cart  : public MapperStrategyA { public: explicit Mapper53Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper58Cart  : public MapperStrategyA { public: explicit Mapper58Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper60Cart  : public MapperStrategyA { public: explicit Mapper60Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper76Cart  : public MapperStrategyA { public: explicit Mapper76Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper95Cart  : public MapperStrategyA { public: explicit Mapper95Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_SIMPLE_CARTS_H
