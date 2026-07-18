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