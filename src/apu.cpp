// FCEUX11 — v1.6 Resonance: fceu11::Apu global instance + v1.0 aliases.
//
// Phase B only defined the global object. Phase C1 adds the canonical
// storage for the migrated output-buffer and timing state, plus the
// reference-to-storage aliases that let legacy code keep using the v1.0
// names (Wave, WaveFinal, WaveHi, soundtsinc, soundtsoffs, soundtsi,
// nesincsize, swapDuty).

#include "apu.h"

namespace fceu11 {

Apu g_apu;

} // namespace fceu11

// ---------------------------------------------------------------------------
// Phase C1 v1.0 compat aliases (plan §1.3 / §2.1).
//
// These reference-to-storage aliases bind the legacy global names to
// fceu11::g_apu members. They are defined here (in the same TU as g_apu)
// so static-init order is well-defined: g_apu is constructed first, then
// the aliases bind to its members.
//
// The alias definitions replace the previous file-scope array/object
// definitions that lived in sound.cpp (and swapDuty in
// drivers/Qt/fceuWrapper.cpp). Every existing call site that does
// `Wave[i] += v`, `soundtsinc = x`, or `swapDuty = b` writes through to
// the g_apu storage without code change.
// ---------------------------------------------------------------------------

int32_t   (& Wave      )[2048 + 512] = fceu11::g_apu.wave();
int32_t   (& WaveFinal )[2048 + 512] = fceu11::g_apu.wave_final();
int32_t   (& WaveHi    )[40000]      = fceu11::g_apu.wave_hi();

uint32_t& soundtsinc  = fceu11::g_apu.soundtsinc();
uint32_t& soundtsoffs = fceu11::g_apu.soundtsoffs();
uint32_t& soundtsi    = fceu11::g_apu.soundtsi();
int32_t&  nesincsize  = fceu11::g_apu.nesincsize();
bool&     swapDuty    = fceu11::g_apu.swap_duty();
