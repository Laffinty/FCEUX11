// FCEUX11 — v1.6 Resonance §10.2: fceu11::Apu global instance.
//
// Phase B only defines the global object; all method bodies are inline
// no-ops in apu.h. This TU exists so the linker has exactly one
// definition of g_apu.

#include "apu.h"

namespace fceu11 {

Apu g_apu;

} // namespace fceu11
