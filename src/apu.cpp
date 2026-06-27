// FCEUX11 — v1.6 Resonance: fceu11::Apu global instance + v1.0 aliases.
//
// Phase B defined the global object. Phase C1 added output-buffer and
// timing aliases. Phase C2 adds the channel/DMC/frame-counter aliases.
// All aliases are reference-to-storage bindings into g_apu members.

#include "apu.h"

namespace fceu11 {

Apu g_apu;

} // namespace fceu11

// ---------------------------------------------------------------------------
// Phase C1 v1.0 compat aliases (plan §1.3 / §2.1).
// ---------------------------------------------------------------------------

int32_t   (& Wave      )[2048 + 512] = fceu11::g_apu.wave();
int32_t   (& WaveFinal )[2048 + 512] = fceu11::g_apu.wave_final();
int32_t   (& WaveHi    )[40000]      = fceu11::g_apu.wave_hi();

uint32_t& soundtsinc  = fceu11::g_apu.soundtsinc();
uint32_t& soundtsoffs = fceu11::g_apu.soundtsoffs();
uint32_t& soundtsi    = fceu11::g_apu.soundtsi();
int32_t&  nesincsize  = fceu11::g_apu.nesincsize();
bool&     swapDuty    = fceu11::g_apu.swap_duty();

// ---------------------------------------------------------------------------
// Phase C2 v1.0 compat aliases (plan §2.1).
//
// These replace the previous file-scope definitions in sound.cpp (and the
// DMC_7bit declaration in ppu.h). Static/internal-linkage variables from
// sound.cpp are now exposed as external aliases so sound.cpp can keep using
// their original names; the aliases still resolve to g_apu storage.
// ---------------------------------------------------------------------------

uint8_t   (& PSG            )[0x10] = fceu11::g_apu.psg();
ENVUNIT   (& EnvUnits       )[3]    = fceu11::g_apu.env_units();
uint8_t&    EnabledChannels         = fceu11::g_apu.enabled_channels();
uint8_t&    IRQFrameMode            = fceu11::g_apu.irq_frame_mode();
uint16_t&   nreg                    = fceu11::g_apu.nreg();
uint8_t&    TriCount                = fceu11::g_apu.tri_count();
uint8_t&    TriMode                 = fceu11::g_apu.tri_mode();
int32_t&    tristep                 = fceu11::g_apu.tristep();
int32_t   (& wlcount        )[4]    = fceu11::g_apu.wlcount();
int32_t   (& lengthcount    )[4]    = fceu11::g_apu.lengthcount();

int32_t   (& RectDutyCount  )[2]    = fceu11::g_apu.rect_duty_count();
uint8_t   (& sweepon         )[2]   = fceu11::g_apu.sweepon();
int32_t   (& curfreq         )[2]   = fceu11::g_apu.curfreq();
uint8_t   (& SweepCount      )[2]   = fceu11::g_apu.sweep_count();
uint8_t   (& SweepReload     )[2]   = fceu11::g_apu.sweep_reload();
int32_t   (& sqacc           )[2]   = fceu11::g_apu.sqacc();

uint8_t&    fcnt                    = fceu11::g_apu.fcnt();
int32_t&    fhcnt                   = fceu11::g_apu.fhcnt();
int32_t&    fhinc                   = fceu11::g_apu.fhinc();

uint8_t&    DMCFormat               = fceu11::g_apu.dmc_format();
uint8_t&    RawDALatch              = fceu11::g_apu.raw_da_latch();
uint8_t&    InitialRawDALatch       = fceu11::g_apu.initial_raw_da_latch();
bool&       DMC_7bit                = fceu11::g_apu.dmc_7bit();
int32_t&    DMCacc                  = fceu11::g_apu.dmc_acc();
int32_t&    DMCPeriod               = fceu11::g_apu.dmc_period();
uint8_t&    DMCBitCount             = fceu11::g_apu.dmc_bit_count();
uint32_t&   DMCAddress              = fceu11::g_apu.dmc_address();
uint8_t&    DMCAddressLatch         = fceu11::g_apu.dmc_address_latch();
int32_t&    DMCSize                 = fceu11::g_apu.dmc_size();
uint8_t&    DMCSizeLatch            = fceu11::g_apu.dmc_size_latch();
uint8_t&    DMCShift                = fceu11::g_apu.dmc_shift();
char&       DMCHaveDMA              = fceu11::g_apu.dmc_have_dma();
char&       DMCHaveSample           = fceu11::g_apu.dmc_have_sample();
uint8_t&    DMCDMABuf               = fceu11::g_apu.dmc_dma_buf();
uint8_t&    SIRQStat                = fceu11::g_apu.sirq_stat();
