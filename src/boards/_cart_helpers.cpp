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
// FCEUX11 — v1.8 Masonry §1.3 release_mapper_resources implementation.

#include "_cart_helpers.h"

#include "../fceu.h"            // DECLFR/DECLFW, currCartInfo via cart.h
#include "../cart.h"            // currCartInfo, CartInfo::Close
#include "../ppu.h"             // GameHBIRQHook (declared in ppu.h:26)
#include "../cpu.h"             // g_cpu, set_map_irq_hook (hotfix3 B-5b)

namespace fceu11 {

void release_mapper_resources() noexcept {
    // Module-level IRQ hooks persist across cart swaps; always reset.
    GameHBIRQHook = nullptr;
    g_cpu.set_map_irq_hook(nullptr);

    // The legacy iNES loader path (src/ines.cpp:113 GI_CLOSE) already
    // invokes iNESCart.Close() when a ROM is unloaded.  Cart subclass
    // on_close() is called in addition when the Cart unique_ptr is
    // destroyed, so we don't double-invoke the Close function pointer
    // here — that would risk double-free of WRAM/CHRRAM.
    //
    // The two paths are intentionally redundant: the GI_CLOSE path
    // covers legacy CartInfo-driven loads; the Cart::on_close() path
    // covers Cart-subclass-driven loads.  Both null GameHBIRQHook,
    // which is the one piece of state neither path would otherwise
    // touch on every close.
}

} // namespace fceu11