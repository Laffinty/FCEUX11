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
// FCEUX11 — v1.7 Cartograph Phase E: NromCart subclass.
//
// Strategy A (v1.7 build plan §3.3): the cart subclass's on_power() simply
// delegates to the legacy NROM_Init(currCartInfo) function pointer path.
// This keeps the NROM_Init body untouched (no behavior change risk) and
// exercises the v1.7 Cart virtual lifecycle against the simplest possible
// mapper as a PoC before tackling MMC1 / MMC3 in Phase F.
//
// NROM has no reset logic and no expansion audio, so on_reset() and
// install_expansion_audio() inherit the Cart defaults (no-op).

#ifndef FCEU11_NROM_CART_H
#define FCEU11_NROM_CART_H

#include "cart_class.h"

namespace fceu11 {

class NromCart : public Mapper {
public:
    explicit NromCart(Bus& bus) noexcept { attach_bus(bus); }

    void on_power() noexcept override;
    void on_reset() noexcept override {}  // NROM has no reset logic.
    void on_close() noexcept override {}  // CartInfo::ClearGameSave handles WRAM.

    // v1.8 Masonry §6.1: byte-diff snapshot of mapper state.
    std::vector<uint8_t> save_mapper_state() const noexcept override;
};

} // namespace fceu11

#endif // FCEU11_NROM_CART_H