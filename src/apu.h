// FCEUX11 — v1.6 Resonance: fceu11::Apu class declaration.
//
// Phase B created the class shell and global instance. Phase C1 migrated
// the output buffers and resampling/timing state; Phase C2 migrates the
// channel/DMC/frame-counter state from sound.cpp into this class. The
// legacy v1.0 global names are kept as `extern` reference aliases declared
// in sound.h (and debug.h / ppu.h where they were previously declared)
// and defined in apu.cpp; existing call sites continue to compile and
// write through to g_apu.

#ifndef FCEU11_APU_H
#define FCEU11_APU_H

#include <cstdint>
#include <cstddef>

#include "types.h"
#include "expansion_audio.h"   // EXPSOUND, ExpansionAudio
#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN

// Envelope unit state (was in sound.h; moved here because it is now a
// member of Apu). Kept as a plain struct to preserve savestate layout.
typedef struct {
    uint8 Speed;
    uint8 Mode;    /* Fixed volume(1), and loop(2) */
    uint8 DecCountTo1;
    uint8 decvolume;
    int reloaddec;
} ENVUNIT;

namespace fceu11 {

#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable: 4324)
#endif
class FCEUX11_CACHE_ALIGN Apu {
public:
    // ---- Lifecycle (Phase B: no-op; Phase C/E: real impls) ----
    Apu() noexcept : exp_sound_{} {}
    void init() noexcept {}
    void shutdown() noexcept {}
    void power() noexcept {}
    void reset() noexcept {}

    // ---- Phase D: expansion audio adapter ----
    EXPSOUND& exp_sound() noexcept { return exp_sound_; }
    void set_exp_sound(const EXPSOUND& exp) noexcept { exp_sound_ = exp; }

    // ---- Phase C1 accessors: output buffers ----
    __forceinline int32_t (& wave() noexcept)[2048 + 512]       { return wave_; }
    __forceinline int32_t (& wave_final() noexcept)[2048 + 512] { return wave_final_; }
    __forceinline int32_t (& wave_hi() noexcept)[40000]         { return wave_hi_; }

    // ---- Phase C1 accessors: resampling / timing ----
    __forceinline uint32_t& soundtsinc()  noexcept { return soundtsinc_; }
    __forceinline uint32_t& soundtsoffs() noexcept { return soundtsoffs_; }
    __forceinline uint32_t& soundtsi()    noexcept { return soundtsi_; }
    __forceinline int32_t&  nesincsize()  noexcept { return nesincsize_; }
    __forceinline bool&     swap_duty()   noexcept { return swap_duty_; }

    // ---- Phase C2 accessors: channel registers / envelope / length ----
    __forceinline uint8_t (& psg() noexcept)[0x10]         { return psg_; }
    __forceinline ENVUNIT  (& env_units() noexcept)[3]     { return env_units_; }
    __forceinline uint8_t&   enabled_channels() noexcept   { return enabled_channels_; }
    __forceinline uint8_t&   irq_frame_mode() noexcept     { return irq_frame_mode_; }
    __forceinline uint16_t&  nreg() noexcept               { return nreg_; }
    __forceinline uint8_t&   tri_count() noexcept          { return tri_count_; }
    __forceinline uint8_t&   tri_mode() noexcept           { return tri_mode_; }
    __forceinline int32_t&   tristep() noexcept            { return tristep_; }
    __forceinline int32_t  (& wlcount() noexcept)[4]       { return wlcount_; }
    __forceinline int32_t  (& lengthcount() noexcept)[4]   { return lengthcount_; }

    // ---- Phase C2 accessors: square waves ----
    __forceinline int32_t  (& rect_duty_count() noexcept)[2] { return rect_duty_count_; }
    __forceinline uint8_t  (& sweepon() noexcept)[2]        { return sweepon_; }
    __forceinline int32_t  (& curfreq() noexcept)[2]        { return curfreq_; }
    __forceinline uint8_t  (& sweep_count() noexcept)[2]    { return sweep_count_; }
    __forceinline uint8_t  (& sweep_reload() noexcept)[2]   { return sweep_reload_; }
    __forceinline int32_t  (& sqacc() noexcept)[2]          { return sqacc_; }

    // ---- Phase C2 accessors: frame counter ----
    __forceinline uint8_t&  fcnt() noexcept  { return fcnt_; }
    __forceinline int32_t&  fhcnt() noexcept { return fhcnt_; }
    __forceinline int32_t&  fhinc() noexcept { return fhinc_; }

    // ---- Phase C2 accessors: DMC ----
    __forceinline uint8_t&  dmc_format() noexcept             { return dmc_format_; }
    __forceinline uint8_t&  raw_da_latch() noexcept           { return raw_da_latch_; }
    __forceinline uint8_t&  initial_raw_da_latch() noexcept   { return initial_raw_da_latch_; }
    __forceinline bool&     dmc_7bit() noexcept               { return dmc_7bit_; }
    __forceinline int32_t&  dmc_acc() noexcept                { return dmc_acc_; }
    __forceinline int32_t&  dmc_period() noexcept             { return dmc_period_; }
    __forceinline uint8_t&  dmc_bit_count() noexcept          { return dmc_bit_count_; }
    __forceinline uint32_t& dmc_address() noexcept            { return dmc_address_; }
    __forceinline uint8_t&  dmc_address_latch() noexcept      { return dmc_address_latch_; }
    __forceinline int32_t&  dmc_size() noexcept               { return dmc_size_; }
    __forceinline uint8_t&  dmc_size_latch() noexcept         { return dmc_size_latch_; }
    __forceinline uint8_t&  dmc_shift() noexcept              { return dmc_shift_; }
    __forceinline char&     dmc_have_dma() noexcept           { return dmc_have_dma_; }
    __forceinline char&     dmc_have_sample() noexcept        { return dmc_have_sample_; }
    __forceinline uint8_t&  dmc_dma_buf() noexcept            { return dmc_dma_buf_; }
    __forceinline uint8_t&  sirq_stat() noexcept              { return sirq_stat_; }

private:
    // Phase C1: output buffers and resampling/timing state.
    int32_t wave_[2048 + 512];
    int32_t wave_final_[2048 + 512];
    int32_t wave_hi_[40000];
    uint32_t soundtsinc_;
    uint32_t soundtsoffs_;
    uint32_t soundtsi_;
    int32_t  nesincsize_;
    bool     swap_duty_;

    // Phase C2: channel registers / envelope / length.
    uint8_t psg_[0x10];
    ENVUNIT env_units_[3];
    uint8_t enabled_channels_;
    uint8_t irq_frame_mode_;
    uint16_t nreg_;
    uint8_t tri_count_;
    uint8_t tri_mode_;
    int32_t tristep_;
    int32_t wlcount_[4];
    int32_t lengthcount_[4];

    // Phase C2: square waves.
    int32_t rect_duty_count_[2];
    uint8_t sweepon_[2];
    int32_t curfreq_[2];
    uint8_t sweep_count_[2];
    uint8_t sweep_reload_[2];
    int32_t sqacc_[2];

    // Phase C2: frame counter.
    uint8_t fcnt_;
    int32_t fhcnt_;
    int32_t fhinc_;

    // Phase C2: DMC.
    uint8_t dmc_format_;
    uint8_t raw_da_latch_;
    uint8_t initial_raw_da_latch_;
    bool    dmc_7bit_;
    int32_t dmc_acc_;
    int32_t dmc_period_;
    uint8_t dmc_bit_count_;
    uint32_t dmc_address_;
    uint8_t  dmc_address_latch_;
    int32_t  dmc_size_;
    uint8_t  dmc_size_latch_;
    uint8_t dmc_shift_;
    char    dmc_have_dma_;
    char    dmc_have_sample_;
    uint8_t dmc_dma_buf_;
    uint8_t sirq_stat_;

    // Phase D: expansion audio adapter (v1.0 EXPSOUND ABI).
    EXPSOUND exp_sound_;
};
#ifdef _MSC_VER
#pragma warning(pop)
#endif

// Direct global instance (plan §1.2). Same pattern as g_bus / g_ppu.
extern Apu g_apu;

} // namespace fceu11

#endif // FCEU11_APU_H
