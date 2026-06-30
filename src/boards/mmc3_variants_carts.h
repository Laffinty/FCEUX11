// FCEUX11 — v1.8 Masonry §1.4: 24 MMC3 variant Cart subclasses.
//
// Each derived class is header-only and just stores mapper_number_
// in the base via the Mapper ctor.  All behavior (on_power via
// Strategy A, on_reset = MMC3RegReset, on_close = GameHBIRQHook null)
// lives in Mmc3BaseCart.
//
// Mapping of mapper # → Init function name (used by MapperEntryRegister):
//
//   12 → Mapper12_Init   37 → Mapper37_Init   44 → Mapper44_Init
//   45 → Mapper45_Init   47 → Mapper47_Init   49 → Mapper49_Init
//   52 → Mapper52_Init   74 → Mapper74_Init   114 → Mapper114_Init
//  115 → Mapper115_Init  116 → UNLSL12_Init    118 → TKSROM_Init
//  119 → Mapper119_Init  165 → Mapper165_Init  192 → Mapper192_Init
//  194 → Mapper194_Init  195 → Mapper195_Init  198 → Mapper198_Init
//  205 → Mapper205_Init  245 → Mapper245_Init  249 → Mapper249_Init
//  250 → Mapper250_Init  254 → Mapper254_Init  406 → Mapper406_Init
//
// (Names verified against src/boards/mmc3.cpp in Phase D.3.)

#ifndef FCEU11_BOARDS_MMC3_VARIANTS_CARTS_H
#define FCEU11_BOARDS_MMC3_VARIANTS_CARTS_H

#include "mmc3_base_cart.h"

namespace fceu11 {

class Mapper12Cart  : public Mmc3BaseCart { public: explicit Mapper12Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper37Cart  : public Mmc3BaseCart { public: explicit Mapper37Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper44Cart  : public Mmc3BaseCart { public: explicit Mapper44Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper45Cart  : public Mmc3BaseCart { public: explicit Mapper45Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper47Cart  : public Mmc3BaseCart { public: explicit Mapper47Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper49Cart  : public Mmc3BaseCart { public: explicit Mapper49Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper52Cart  : public Mmc3BaseCart { public: explicit Mapper52Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper74Cart  : public Mmc3BaseCart { public: explicit Mapper74Cart(Bus& bus) noexcept  : Mmc3BaseCart(bus) {} };
class Mapper114Cart : public Mmc3BaseCart { public: explicit Mapper114Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper115Cart : public Mmc3BaseCart { public: explicit Mapper115Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper116Cart : public Mmc3BaseCart { public: explicit Mapper116Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper118Cart : public Mmc3BaseCart { public: explicit Mapper118Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper119Cart : public Mmc3BaseCart { public: explicit Mapper119Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper165Cart : public Mmc3BaseCart { public: explicit Mapper165Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper192Cart : public Mmc3BaseCart { public: explicit Mapper192Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper194Cart : public Mmc3BaseCart { public: explicit Mapper194Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper195Cart : public Mmc3BaseCart { public: explicit Mapper195Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper198Cart : public Mmc3BaseCart { public: explicit Mapper198Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper205Cart : public Mmc3BaseCart { public: explicit Mapper205Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper245Cart : public Mmc3BaseCart { public: explicit Mapper245Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper249Cart : public Mmc3BaseCart { public: explicit Mapper249Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper250Cart : public Mmc3BaseCart { public: explicit Mapper250Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper254Cart : public Mmc3BaseCart { public: explicit Mapper254Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };
class Mapper406Cart : public Mmc3BaseCart { public: explicit Mapper406Cart(Bus& bus) noexcept : Mmc3BaseCart(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_MMC3_VARIANTS_CARTS_H