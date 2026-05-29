/// \file
/// \brief Sound filtering code
///
/// Phase 9 (v0.2.10): Audio filter backend migrated to Rust.
/// C++ side retains the original function signatures; all static state
/// (FIR coefficients, resampling indices, IIR accumulators) lives in an
/// opaque Rust `FilterState` handle.

#include "types.h"

#include "sound.h"
#include "x6502.h"
#include "fceu.h"
#include "filter.h"

#include "rust/fceux11_rust.h"

static FceuFilterState *g_filter_state = nullptr;

static FceuFilterState* get_filter_state() {
    if (!g_filter_state) {
        g_filter_state = fceux11_rust_filter_state_create();
    }
    return g_filter_state;
}

void SexyFilter2(int32 *in, int32 count)
{
    fceux11_rust_filter_sexy2(get_filter_state(), in, count);
}

void SexyFilter(int32 *in, int32 *out, int32 count)
{
    fceux11_rust_filter_sexy(
        get_filter_state(),
        in, out, count,
        FSettings.SndRate,
        FSettings.SoundVolume,
        FSettings.soundq
    );
}

int32 NeoFilterSound(int32 *in, int32 *out, uint32 inlen, int32 *leftover)
{
    return fceux11_rust_filter_neo(
        get_filter_state(),
        in, out, inlen, leftover,
        FSettings.soundq,
        FSettings.lowpass,
        GameExpSound.NeoFill,
        FSettings.SndRate,
        FSettings.SoundVolume
    );
}

void MakeFilters(int32 rate)
{
    fceux11_rust_filter_make(
        get_filter_state(),
        rate,
        FSettings.soundq,
        PAL ? 1 : 0,
        NTSC_CPU,
        PAL_CPU
    );
}
