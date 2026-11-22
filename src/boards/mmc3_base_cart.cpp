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
// FCEUX11 — v1.8 Masonry §1.4: Mmc3BaseCart implementation.

#include "mmc3_base_cart.h"

#include "../fceu.h"            // DECLFR/DECLFW macros (must precede cart.h + mmc3.h)
#include "../cart.h"           // currCartInfo + CartInfo (uses DECLFR/DECLFW)
#include "_cart_helpers.h"     // release_mapper_resources
#include "mmc3.h"              // MMC3RegReset (uses DECLFW)

namespace fceu11 {

Mmc3BaseCart::Mmc3BaseCart(Bus& bus) noexcept {
    attach_bus(bus);
    // v1.15 B.1: Removed legacy_power_ capture. on_power() now calls
    // the per-mapper power function directly (e.g. Mapper12Power,
    // Mapper37Power) instead of going through the captured function
    // pointer. This eliminates the Strategy A delegation pattern.
}

void Mmc3BaseCart::on_power() noexcept {
    // v1.15 B.1: Direct call to the per-mapper power function.
    // Each MMC3 variant subclass overrides on_power() to call its
    // specific power function (e.g. Mapper12Power, Mapper37Power).
    // The base class provides a default that calls GenMMC3Power.
    GenMMC3Power();
}

void Mmc3BaseCart::on_reset() noexcept {
    // All 24 MMC3 variants share MMC3RegReset.
    MMC3RegReset();
}

void Mmc3BaseCart::on_close() noexcept {
    // Module-level GameHBIRQHook may have been wired by per-mapper
    // Init (Kick Master / Shougi hacks).  Null it out.
    release_mapper_resources();
}

// v1.8 Masonry Phase E.2 step 5: see mmc3_base_cart.h:42-46 doc.
std::vector<uint8_t> Mmc3BaseCart::save_mapper_state() const noexcept {
    std::vector<uint8_t> out;
    out.reserve(16);
    pack_u32(out, mapper_number());
    pack_u32(out, static_cast<uint32_t>(mirror_raw()));
    pack_u32(out, static_cast<uint32_t>(wram_size()));
    pack_u32(out, static_cast<uint32_t>(battery_wram_size()));
    return out;  // 16 bytes
}

} // namespace fceu11
// v1.8 Masonry Phase D.3: include cart.h AFTER mmc3.h to ensure CartInfo
// is fully defined (mmc3.h only forward-declares CartInfo; the function
// body in mmc3_base_cart.cpp doesn't dereference info so this is fine).
#include "../cart.h"
