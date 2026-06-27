// FCEUX11 — v1.6 Resonance §10.2: fceu11::Apu class scaffolding.
//
// Phase B creates the class shell and global instance only; no APU state
// is migrated yet. The lifecycle hooks (init/power/reset) are currently
// no-ops because sound.cpp still owns and manages all APU state through
// FCEUSND_* free functions. Phase C will migrate state into this class
// and fill the method bodies.
//
// The global instance follows the same direct-global pattern as
// fceu11::g_bus and fceu11::g_ppu so the class can later expose hot-path
// members that compile to direct fixed-address accesses.

#ifndef FCEU11_APU_H
#define FCEU11_APU_H

#include <cstdint>
#include <cstddef>

#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN

namespace fceu11 {

class FCEUX11_CACHE_ALIGN Apu {
public:
    // ---- Lifecycle (Phase B: no-op; Phase C/D/E: real impls) ----
    Apu() noexcept = default;
    void init() noexcept {}
    void shutdown() noexcept {}
    void power() noexcept {}
    void reset() noexcept {}
};

// Direct global instance (plan §1.2). Same pattern as g_bus / g_ppu.
extern Apu g_apu;

} // namespace fceu11

#endif // FCEU11_APU_H
