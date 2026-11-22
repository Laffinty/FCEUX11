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