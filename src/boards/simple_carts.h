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
// v1.8 Phase E.2 step 9.5: FFE mappers 6/17 and Namco 163 mappers 19/210.
class Mapper6Cart   : public MapperStrategyA { public: explicit Mapper6Cart(Bus& bus) noexcept   : MapperStrategyA(bus) {} };
class Mapper17Cart  : public MapperStrategyA { public: explicit Mapper17Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper19Cart  : public MapperStrategyA { public: explicit Mapper19Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper210Cart : public MapperStrategyA { public: explicit Mapper210Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
// v1.8 Phase E.2 step 9.6: Sunsoft 5B (mapper 69) as simple cart.
// ExpansionAudio subclass deferred to Phase G.
class Mapper69Cart  : public MapperStrategyA { public: explicit Mapper69Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper206Cart : public MapperStrategyA { public: explicit Mapper206Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
// v1.8 Phase E.2 audit: mapper 59 was active in bmap[] but missing from registry.
class Mapper59Cart  : public MapperStrategyA { public: explicit Mapper59Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };

// v1.8 Phase F batch 1.
class Mapper103Cart  : public MapperStrategyA { public: explicit Mapper103Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper106Cart  : public MapperStrategyA { public: explicit Mapper106Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper108Cart  : public MapperStrategyA { public: explicit Mapper108Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper112Cart  : public MapperStrategyA { public: explicit Mapper112Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper117Cart  : public MapperStrategyA { public: explicit Mapper117Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper120Cart  : public MapperStrategyA { public: explicit Mapper120Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper121Cart  : public MapperStrategyA { public: explicit Mapper121Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper151Cart  : public MapperStrategyA { public: explicit Mapper151Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper156Cart  : public MapperStrategyA { public: explicit Mapper156Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper177Cart  : public MapperStrategyA { public: explicit Mapper177Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };

// v1.8 Phase F incremental.
class Mapper111Cart  : public MapperStrategyA { public: explicit Mapper111Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper123Cart  : public MapperStrategyA { public: explicit Mapper123Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper125Cart  : public MapperStrategyA { public: explicit Mapper125Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper132Cart  : public MapperStrategyA { public: explicit Mapper132Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper133Cart  : public MapperStrategyA { public: explicit Mapper133Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper136Cart  : public MapperStrategyA { public: explicit Mapper136Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper137Cart  : public MapperStrategyA { public: explicit Mapper137Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper138Cart  : public MapperStrategyA { public: explicit Mapper138Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper139Cart  : public MapperStrategyA { public: explicit Mapper139Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper141Cart  : public MapperStrategyA { public: explicit Mapper141Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper142Cart  : public MapperStrategyA { public: explicit Mapper142Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper143Cart  : public MapperStrategyA { public: explicit Mapper143Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper145Cart  : public MapperStrategyA { public: explicit Mapper145Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper146Cart  : public MapperStrategyA { public: explicit Mapper146Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper147Cart  : public MapperStrategyA { public: explicit Mapper147Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper148Cart  : public MapperStrategyA { public: explicit Mapper148Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper149Cart  : public MapperStrategyA { public: explicit Mapper149Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper150Cart  : public MapperStrategyA { public: explicit Mapper150Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper160Cart  : public MapperStrategyA { public: explicit Mapper160Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper162Cart  : public MapperStrategyA { public: explicit Mapper162Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper163Cart  : public MapperStrategyA { public: explicit Mapper163Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper164Cart  : public MapperStrategyA { public: explicit Mapper164Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper166Cart  : public MapperStrategyA { public: explicit Mapper166Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper167Cart  : public MapperStrategyA { public: explicit Mapper167Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper168Cart  : public MapperStrategyA { public: explicit Mapper168Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper170Cart  : public MapperStrategyA { public: explicit Mapper170Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper172Cart  : public MapperStrategyA { public: explicit Mapper172Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper173Cart  : public MapperStrategyA { public: explicit Mapper173Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper175Cart  : public MapperStrategyA { public: explicit Mapper175Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper176Cart  : public MapperStrategyA { public: explicit Mapper176Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper181Cart  : public MapperStrategyA { public: explicit Mapper181Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper183Cart  : public MapperStrategyA { public: explicit Mapper183Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper185Cart  : public MapperStrategyA { public: explicit Mapper185Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper186Cart  : public MapperStrategyA { public: explicit Mapper186Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };

// v1.8 Phase F incremental.
class Mapper187Cart  : public MapperStrategyA { public: explicit Mapper187Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper188Cart  : public MapperStrategyA { public: explicit Mapper188Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper189Cart  : public MapperStrategyA { public: explicit Mapper189Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper190Cart  : public MapperStrategyA { public: explicit Mapper190Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };
class Mapper193Cart  : public MapperStrategyA { public: explicit Mapper193Cart(Bus& bus) noexcept  : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_SIMPLE_CARTS_H
