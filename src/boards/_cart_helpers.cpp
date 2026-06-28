// FCEUX11 — v1.8 Masonry §1.3 release_mapper_resources implementation.

#include "_cart_helpers.h"

#include "../cart.h"            // currCartInfo, CartInfo::Close
#include "../x6502.h"           // GameHBIRQHook (declared in x6502.h via fceu.h)

namespace fceu11 {

void release_mapper_resources() noexcept {
    // Module-level IRQ hook persists across cart swaps; always reset.
    GameHBIRQHook = nullptr;

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