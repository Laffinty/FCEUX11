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
// FCEUX11 — v1.7 Cartograph Phase E: Vrc6Cart subclass.
//
// Strategy A (v1.7 build plan §3.3): the cart subclass's on_power() simply
// delegates to the legacy Mapper24_Init / Mapper26_Init(currCartInfo) function
// pointer path. This is the first PoC that carries a non-trivial mapper
// (~140 lines of bank-switching) and exercises the v1.6 §11.1
// install_expansion_audio contract (cart-side g_apu.set_exp_sound()).
//
// VRC6 has no reset logic and uses one mapper number (24 / 26 — both
// dispatched through this subclass based on which Init was called by the
// loader). on_close() releases the IRQ hook registered by Mapper24/26_Init.

#ifndef FCEU11_VRC6_CART_H
#define FCEU11_VRC6_CART_H

#include "cart_class.h"

namespace fceu11 {

class Vrc6Cart : public Mapper {
public:
    explicit Vrc6Cart(Bus& bus, uint32_t mapper_no) noexcept {
        attach_bus(bus);
        mapper_number_ = mapper_no;
    }

    void on_power() noexcept override;
    void on_reset() noexcept override {}  // VRC6 has no reset logic.
    void on_close() noexcept override;    // releases VRC6 IRQ hook.

    // v1.6 §11.1 contract: route g_vrc6_audio into g_apu via set_exp_sound().
    // Triggered by iNESLoad / UNIFLoad after cart assignment, before PowerNES.
    void install_expansion_audio(class Apu& apu) noexcept override;

    // v1.8 Masonry §6.1: byte-diff snapshot of mapper state.
    std::vector<uint8_t> save_mapper_state() const noexcept override;
};

} // namespace fceu11

#endif // FCEU11_VRC6_CART_H