// FCEUX11 — v1.2 Census: Global-state access facade (definition)
//
// Every accessor resolves to the existing file-scope `extern` declaration
// in its owning header (x6502.h, ppu.h, sound.h, cart.h, etc.). v1.2 adds
// no new state; v1.3+ replace these references with object members as the
// Cpu/Bus/Ppu/Apu/Cart classes come online. The implementation is a thin
// layer of static_casts to keep the accessors `noexcept` and inlinable.

#include "core_state.h"

#include "bus.h"        // fceu11::bus_instance() (State::bus()), inline aliases for ::ARead/::BWrite/::Page/::VPage/::PRGptr/::CHRptr
#include "cpu.h"        // fceu11::cpu_instance()
#include "x6502.h"      // X6502, legacy inline aliases
#include "x6502struct.h" // X6502 (struct definition)
#include "ppu.h"        // ::PPU, ::NTARAM, ::vnapage, ::VPage, ::PPUNTARAM, ::PPUCHRRAM, ::ppuphase, ::PPU_hook, ::GameHBIRQHook
#include "sound.h"      // ::Wave, ::WaveFinal, ::WaveHi, ::nesincsize, ::soundtsinc, ::soundtsoffs, ::swapDuty, ::GameExpSound
#include "fceu.h"       // ::PAL, ::dendy, ::movieSubtitles, ::FSettings, ::RAMInitOption, ::RAM, ::EmulationPaused, ::frameAdvance_Delay, ::GameAttributes, ::timestampbase, ::normalscanlines, ::totalscanlines, ::QTAIHack, ::QTAINTRAM, ::qtaintramreg
#include "ines.h"       // ::head
#include "debug.h"      // ::codecount, ::datacount, ::undefinedcount, ::iaPC, ::iapoffset, ::break_asap, ::total_cycles_base, etc.
#include "fceu11_core_types.h" // fceu11::MapIRQHook

namespace fceu11 {

// ---------------------------------------------------------------------------
// CpuView
// ---------------------------------------------------------------------------
__X6502& CpuView::reg() noexcept { return fceu11::cpu_instance().native_layout(); }
uint32_t& CpuView::timestamp() noexcept { return fceu11::cpu_instance().timestamp_ref(); }
uint32_t& CpuView::sound_timestamp() noexcept { return fceu11::cpu_instance().sound_timestamp_ref(); }
int& CpuView::scanline() noexcept { return fceu11::cpu_instance().scanline_ref(); }
void (*& CpuView::irq_hook())(int) noexcept { return fceu11::cpu_instance().map_irq_hook_ref(); }

uint64_t& CpuView::timestamp_base() noexcept { return ::timestampbase; }
int& CpuView::normal_scanlines() noexcept { return ::normalscanlines; }
int& CpuView::total_scanlines() noexcept { return ::totalscanlines; }

// ---------------------------------------------------------------------------
// PpuView — use pointer-typed static_cast for vnapage/VPage (declared as
// `uint8_t* []` in ppu.h, returned as `uint8_t* (&)[]` here).
// ---------------------------------------------------------------------------
uint8_t (& PpuView::regs() noexcept)[4]   { return ::PPU; }
uint8_t (& PpuView::ntaram() noexcept)[0x800] { return ::NTARAM; }
uint8_t* (& PpuView::vnapage() noexcept)[4]   { return ::vnapage; }
uint8_t* (& PpuView::vpage() noexcept)[8]     { return ::VPage; }

uint8_t& PpuView::ppu_ntaram_flag() noexcept { return ::PPUNTARAM; }
uint8_t& PpuView::ppu_chrram_flag() noexcept { return ::PPUCHRRAM; }

int& PpuView::phase_raw() noexcept { return reinterpret_cast<int&>(::ppuphase); }

void (*& PpuView::hook())(uint32_t) noexcept { return ::PPU_hook; }
void (*& PpuView::hb_irq_hook())(void) noexcept  { return ::GameHBIRQHook;  }
void (*& PpuView::hb_irq_hook2())(void) noexcept { return ::GameHBIRQHook2; }

// ---------------------------------------------------------------------------
// ApuView
// ---------------------------------------------------------------------------
int32_t (& ApuView::wave() noexcept)[2048 + 512]       { return ::Wave; }
int32_t (& ApuView::wave_final() noexcept)[2048 + 512] { return ::WaveFinal; }
int32_t* ApuView::wave_hi() noexcept { return ::WaveHi; }
int32_t&  ApuView::nes_inc_size() noexcept { return ::nesincsize; }
uint32_t& ApuView::sound_ts_inc() noexcept { return ::soundtsinc; }
uint32_t& ApuView::sound_ts_offs() noexcept { return ::soundtsoffs; }
bool&     ApuView::swap_duty() noexcept { return ::swapDuty; }

// ApuView::exp_sound() deferred to v1.6 (EXPSOUND is an anonymous-struct
// typedef in sound.h; forward-declaring in core_state.h is not portable
// across MSVC/GCC/Clang. v1.6 will replace ::GameExpSound with the
// ExpansionAudio polymorphic interface.)
// struct EXPSOUND& ApuView::exp_sound() noexcept { return ::GameExpSound; }

// ---------------------------------------------------------------------------
// State::bus() — v1.4 Gateway: returns the Bus singleton. BusView is
// retired; the underlying accessors (`bus_instance().aread_table()`
// etc.) cover the same surface and are accessible from any TU via
// bus.h. CartView (below) keeps the related cart-owned tables.
// ---------------------------------------------------------------------------
Bus& State::bus() noexcept { return fceu11::bus_instance(); }

// ---------------------------------------------------------------------------
// CartView
// ---------------------------------------------------------------------------
CartInfo*& CartView::current() noexcept { return ::currCartInfo; }

uint8_t  (& CartView::prg_ram() noexcept)[32] { return ::PRGram; }
uint8_t  (& CartView::chr_ram() noexcept)[32] { return ::CHRram; }
uint8_t* (& CartView::prg_ptr() noexcept)[32] { return ::PRGptr; }
uint8_t* (& CartView::chr_ptr() noexcept)[32] { return ::CHRptr; }
uint32_t (& CartView::prg_size() noexcept)[32] { return ::PRGsize; }
uint32_t (& CartView::chr_size() noexcept)[32] { return ::CHRsize; }

uint32_t (& CartView::prg_mask2()  noexcept)[32] { return ::PRGmask2;  }
uint32_t (& CartView::prg_mask4()  noexcept)[32] { return ::PRGmask4;  }
uint32_t (& CartView::prg_mask8()  noexcept)[32] { return ::PRGmask8;  }
uint32_t (& CartView::prg_mask16() noexcept)[32] { return ::PRGmask16; }
uint32_t (& CartView::prg_mask32() noexcept)[32] { return ::PRGmask32; }
uint32_t (& CartView::chr_mask1()  noexcept)[32] { return ::CHRmask1;  }
uint32_t (& CartView::chr_mask2()  noexcept)[32] { return ::CHRmask2;  }
uint32_t (& CartView::chr_mask4()  noexcept)[32] { return ::CHRmask4;  }
uint32_t (& CartView::chr_mask8()  noexcept)[32] { return ::CHRmask8;  }

int& CartView::genie_stage() noexcept { return ::geniestage; }

int& CartView::qtai_hack() noexcept { return ::QTAIHack; }
uint8_t (& CartView::qtai_ntram() noexcept)[2048] { return ::QTAINTRAM; }
uint8_t& CartView::qtai_reg() noexcept { return ::qtaintramreg; }

struct iNES_HEADER& CartView::rom_header() noexcept { return ::head; }

// ---------------------------------------------------------------------------
// ConfigView
// ---------------------------------------------------------------------------
uint8_t& ConfigView::pal() noexcept { return ::PAL; }
int& ConfigView::dendy() noexcept { return ::dendy; }
bool& ConfigView::movie_subtitles() noexcept { return ::movieSubtitles; }
fceu_settings_struct& ConfigView::settings() noexcept { return ::FSettings; }

int& ConfigView::ram_init_option() noexcept { return ::RAMInitOption; }
uint8_t*& ConfigView::ram() noexcept { return ::RAM; }
int& ConfigView::emulation_paused() noexcept { return ::EmulationPaused; }
int& ConfigView::frame_advance_delay() noexcept { return ::frameAdvance_Delay; }
// ConfigView::game_attributes() deferred — see comment in core_state.h.

// ---------------------------------------------------------------------------
// DebugView — see debug.h for the source declarations.
// ---------------------------------------------------------------------------
int& DebugView::in_debug() noexcept { return ::fceuindbg; }

volatile int& DebugView::code_count() noexcept { return ::codecount; }
volatile int& DebugView::data_count() noexcept { return ::datacount; }
volatile int& DebugView::undefined_count() noexcept { return ::undefinedcount; }
volatile int& DebugView::render_count() noexcept { return ::rendercount; }
volatile int& DebugView::vrom_read_count() noexcept { return ::vromreadcount; }
volatile int& DebugView::undefined_vrom_count() noexcept { return ::undefinedvromcount; }

int& DebugView::ia_pc() noexcept { return ::iaPC; }
uint32_t& DebugView::ia_poffset() noexcept { return ::iapoffset; }

bool& DebugView::break_asap() noexcept { return ::break_asap; }
bool& DebugView::break_on_unlogged_code() noexcept { return ::break_on_unlogged_code; }
bool& DebugView::break_on_unlogged_data() noexcept { return ::break_on_unlogged_data; }
bool& DebugView::break_on_cycles() noexcept { return ::break_on_cycles; }
bool& DebugView::break_on_instructions() noexcept { return ::break_on_instructions; }

uint64_t& DebugView::total_cycles_base() noexcept { return ::total_cycles_base; }
uint64_t& DebugView::delta_cycles_base() noexcept { return ::delta_cycles_base; }
uint64_t& DebugView::break_cycles_limit() noexcept { return ::break_cycles_limit; }
uint64_t& DebugView::total_instructions() noexcept { return ::total_instructions; }
uint64_t& DebugView::delta_instructions() noexcept { return ::delta_instructions; }
uint64_t& DebugView::break_instructions_limit() noexcept { return ::break_instructions_limit; }

int& DebugView::num_watchpoints() noexcept { return ::numWPs; }

int& DebugView::debug_logging_cd() noexcept { return ::debug_loggingCD; }

// ---------------------------------------------------------------------------
// Singleton — Meyers singleton in the facade translation unit. First call
// initializes all view references (zero-cost; each is a pointer cast of an
// existing extern symbol). Thread-safe per C++11 [stmt.dcl] p4.
// ---------------------------------------------------------------------------
State& global_state() noexcept {
    static State s;
    return s;
}

} // namespace fceu11