/* FCE Ultra - NES/Famicom Emulator
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
 */
// FCEUX11 — v1.8 Masonry Phase D.8: SMB2j FDS variants (40-43, 50) + CALTRON (41) + Mapper46.
//
// Mapper 41 is officially iNES "Caltron 6-in-1" but is commonly listed alongside
// the SMB2j FDS-derived variants in PoP-frequency tables; both Init functions
// live in src/boards/41.cpp.
//
// 44-49 are MMC3 variants already covered by Phase D.3 (Mmc3BaseCart).
//
// Each subclass is a single-line forwarding ctor via MapperStrategyA.

#ifndef FCEU11_BOARDS_SMB2J_CARTS_H
#define FCEU11_BOARDS_SMB2J_CARTS_H

#include "mapper_strategy_a.h"

namespace fceu11 {

class Mapper40Cart : public MapperStrategyA { public: explicit Mapper40Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper41Cart : public MapperStrategyA { public: explicit Mapper41Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper42Cart : public MapperStrategyA { public: explicit Mapper42Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper43Cart : public MapperStrategyA { public: explicit Mapper43Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper46Cart : public MapperStrategyA { public: explicit Mapper46Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Mapper50Cart : public MapperStrategyA { public: explicit Mapper50Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_SMB2J_CARTS_H