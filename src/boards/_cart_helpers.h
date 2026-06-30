// FCEUX11 — v1.8 Masonry §1.3 release_mapper_resources helper.
//
// v1.7 Phase G handoff §3.2 noted that the 4 PoC Cart subclasses
// (NromCart / Mmc1Cart / Mmc3Cart / Vrc6Cart) had empty on_close()
// implementations. While the existing GI_CLOSE handler in
// src/ines.cpp:113 invokes iNESCart.Close() (which the legacy
// mapper Init functions set via info->Close = &GenMMC1Close etc.),
// GameHBIRQHook is a module-level function pointer that persists
// across cart swaps and is NOT cleared automatically.
//
// This header declares a single utility that:
//   1. Always nulls out GameHBIRQHook (cheap; always safe).
//   2. Optionally invokes the legacy currCartInfo->Close if set.
//
// Called from each P0+ cart subclass's on_close() override.

#ifndef FCEU11_BOARDS_CART_HELPERS_H
#define FCEU11_BOARDS_CART_HELPERS_H

#include <cstdint>

namespace fceu11 {

// Release module-level resources tied to the current mapper.
// Always safe to call; idempotent.  No-op when no mapper loaded.
void release_mapper_resources() noexcept;

} // namespace fceu11

#endif // FCEU11_BOARDS_CART_HELPERS_H