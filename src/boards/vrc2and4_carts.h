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
// FCEUX11 — v1.8 Masonry Phase D.6: VRC2/VRC4 Cart subclasses.
//
// vrc2and4.cpp covers 4 iNES entries (21, 22, 23, 25) sharing the same
// VRC24_Init function with minor variations.  Each Cart subclass is a
// single-line forwarding ctor.

#ifndef FCEU11_BOARDS_VRC2AND4_CARTS_H
#define FCEU11_BOARDS_VRC2AND4_CARTS_H

#include "mapper_strategy_a.h"

namespace fceu11 {

class Vrc2and4_21Cart : public MapperStrategyA { public: explicit Vrc2and4_21Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Vrc2and4_22Cart : public MapperStrategyA { public: explicit Vrc2and4_22Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Vrc2and4_23Cart : public MapperStrategyA { public: explicit Vrc2and4_23Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };
class Vrc2and4_25Cart : public MapperStrategyA { public: explicit Vrc2and4_25Cart(Bus& bus) noexcept : MapperStrategyA(bus) {} };

} // namespace fceu11

#endif // FCEU11_BOARDS_VRC2AND4_CARTS_H