// FCEUX11 — v1.6 Resonance: fceu11::Apu class declaration.
//
// Phase B created the class shell and global instance. Phase C1 migrates
// the output buffers and resampling/timing state from sound.cpp into this
// class. The legacy v1.0 global names (Wave, WaveFinal, WaveHi,
// soundtsinc, soundtsoffs, soundtsi, nesincsize, swapDuty) are kept as
// `extern` reference aliases declared in sound.h and defined in apu.cpp;
// existing call sites continue to compile and write through to g_apu.

#ifndef FCEU11_APU_H
#define FCEU11_APU_H

#include <cstdint>
#include <cstddef>

#include "types.h"
#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN

namespace fceu11 {

class FCEUX11_CACHE_ALIGN Apu {
public:
    // ---- Lifecycle (Phase B: no-op; Phase C/E: real impls) ----
    Apu() noexcept = default;
    void init() noexcept {}
    void shutdown() noexcept {}
    void power() noexcept {}
    void reset() noexcept {}

    // ---- Phase C1 accessors: output buffers ----
    // Returned as reference-to-array so the v1.0 global aliases can bind
    // to the class storage with no per-use indirection.
    __forceinline int32_t (& wave() noexcept)[2048 + 512]       { return wave_; }
    __forceinline int32_t (& wave_final() noexcept)[2048 + 512] { return wave_final_; }
    __forceinline int32_t (& wave_hi() noexcept)[40000]         { return wave_hi_; }

    // ---- Phase C1 accessors: resampling / timing ----
    __forceinline uint32_t& soundtsinc()  noexcept { return soundtsinc_; }
    __forceinline uint32_t& soundtsoffs() noexcept { return soundtsoffs_; }
    __forceinline uint32_t& soundtsi()    noexcept { return soundtsi_; }
    __forceinline int32_t&  nesincsize()  noexcept { return nesincsize_; }
    __forceinline bool&     swap_duty()   noexcept { return swap_duty_; }

private:
    // Phase C1: output buffers and resampling/timing state
    // (migrated from sound.cpp / fceuWrapper.cpp).
    int32_t wave_[2048 + 512];
    int32_t wave_final_[2048 + 512];
    int32_t wave_hi_[40000];
    uint32_t soundtsinc_;
    uint32_t soundtsoffs_;
    uint32_t soundtsi_;
    int32_t  nesincsize_;
    bool     swap_duty_;
};

// Direct global instance (plan §1.2). Same pattern as g_bus / g_ppu.
extern Apu g_apu;

} // namespace fceu11

#endif // FCEU11_APU_H
