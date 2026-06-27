// FCEUX11 — v1.6 Resonance §3: ExpansionAudio interface + EXPSOUND adapter.
//
// Non-inline adapter parts. The simple routing helpers live in the header as
// static inline functions so the compiler can fold them at call sites. The
// C-style NeoFill callback (used by the Rust filter FFI) is defined here so
// it has a single stable address.

#include "expansion_audio.h"
#include "sound.h"   // GameExpSound

void FCEU11_ExpNeoFillCallback(int32_t* wave, int count) {
    FCEU11_ExpNeoFill(&GameExpSound, wave, count);
}
