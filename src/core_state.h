// FCEUX11 — v1.2 Census: Global-state access facade
//
// This header provides a single entry point (`fceu11::global_state()`) that
// aggregates read/write views of every subsystem-owned global. The v1.2
// implementation is a "reference view": every accessor returns a reference
// to the existing global variable, so the facade is purely additive — no
// call site needs to change. v1.3 (Cpu), v1.4 (Bus), v1.5 (Ppu), v1.6 (Apu),
// v1.7 (Cart) will gradually replace the underlying globals with object
// members; the public surface of `fceu11::State` is designed to remain
// stable so callers can migrate file-by-file without a break-all commit.
//
// Per v1.x Modernization Roadmap §2.2: "不改动任何现有代码的调用路径，仅
// 提供新入口". Every accessor is `noexcept`, returns by reference, and adds
// zero indirection overhead vs. the global directly (the compiler folds the
// reference through to the underlying symbol).
//
// See docs/internal/global_state_audit.md for the full A–G classification
// of the 101 file-scope `extern` declarations this facade covers.

#ifndef FCEU11_CORE_STATE_H
#define FCEU11_CORE_STATE_H

#include <array>
#include <cstdint>

#include "types.h"
#include "cpu.h"  // hotfix3 B-5a: Cpu::RefProxy return type for irq_hook()

// Forward declarations at GLOBAL scope so the accessors can reference the
// real type (defined in x6502struct.h, cart.h, sound.h, etc.). Putting
// them inside namespace fceu11 would create distinct types — the global
// X6502 used by `::X` is not the same C++ type as fceu11::X6502.
//
// Note: for typedef'd types (FCEUS → fceu_settings_struct, X6502 → X6502
// struct tag) we forward-declare the underlying struct tag, not the
// typedef alias. The .cpp uses the typedef names; C++ treats both
// spellings as the same type.
// hotfix1 P3-1: the previous forward decl was `struct __X6502`, an
// identifier reserved for the implementation. Renamed to `struct X6502`
// in x6502struct.h; this forward declaration follows.
struct X6502;
struct fceu_settings_struct;
struct CartInfo;
struct iNES_HEADER;

namespace fceu11 {

// ---------------------------------------------------------------------------
// CPU view (分类 A — owned by x6502.cpp; v1.3 will fold into Cpu class)
// ---------------------------------------------------------------------------
struct CpuView {
    X6502&     reg()      noexcept;   // X (typedef X6502 = X6502 struct tag)
    uint32_t&    timestamp()   noexcept; // CPU master clock
    uint32_t&    sound_timestamp() noexcept; // APU sub-clock
    int&         scanline() noexcept;
    // hotfix3 B-5a: was `void (*&)(int)` returning a true reference to the
    // Cpu::map_irq_hook_ field. The atomic-backed map_irq_hook_ref() now
    // returns a Cpu::RefProxy by value; the view accessor matches that
    // signature so callers can keep using `view.irq_hook() = SomeFunc;`.
    Cpu::RefProxy irq_hook() noexcept;  // ::MapIRQHook, function-pointer slot

    uint64_t&    timestamp_base() noexcept; // ::timestampbase
    int&         normal_scanlines()  noexcept;
    int&         total_scanlines()   noexcept;
};

// ---------------------------------------------------------------------------
// PPU view (分类 B — owned by ppu.cpp; v1.5 will fold into Ppu class)
// ---------------------------------------------------------------------------
struct PpuView {
    uint8_t (&  regs() noexcept)[4];          // ::PPU
    uint8_t (&  ntaram() noexcept)[0x800];     // ::NTARAM
    uint8_t* (& vnapage() noexcept)[4];        // ::vnapage
    uint8_t* (& vpage() noexcept)[8];          // ::VPage (PPU address page table)

    uint8_t&   ppu_ntaram_flag() noexcept;     // ::PPUNTARAM
    uint8_t&   ppu_chrram_flag() noexcept;     // ::PPUCHRRAM

    int& phase_raw() noexcept;                 // ::ppuphase (cast by caller)

    void (*& hook())(uint32_t) noexcept;       // ::PPU_hook
    void (*& hb_irq_hook())(void) noexcept;    // ::GameHBIRQHook
    void (*& hb_irq_hook2())(void) noexcept;   // ::GameHBIRQHook2
};

// ---------------------------------------------------------------------------
// APU view (分类 C — owned by sound.cpp; v1.6 will fold into Apu class)
// ---------------------------------------------------------------------------
struct ApuView {
    int32_t (& wave() noexcept)[2048 + 512];
    int32_t (& wave_final() noexcept)[2048 + 512];
    int32_t*  wave_hi() noexcept;              // ::WaveHi[] (decayed; size not visible in header)
    int32_t&   nes_inc_size() noexcept;        // ::nesincsize
    uint32_t&  sound_ts_inc() noexcept;        // ::soundtsinc
    uint32_t&  sound_ts_offs() noexcept;       // ::soundtsoffs
    bool&      swap_duty() noexcept;          // ::swapDuty

    // Expansion-audio interface (v1.6 Apu will expose ExpansionAudio* here;
    // v1.2 keeps the legacy ::GameExpSound reachable via direct symbol.)
    // struct EXPSOUND& exp_sound() noexcept;     // ::GameExpSound (deferred to v1.6)
};

// ---------------------------------------------------------------------------
// Bus (分类 D — folded into fceu11::Bus in v1.4 Gateway Phase 2).
// core_state.h forward-declares the class; the full definition is in
// bus.h. State::bus() returns a reference to fceu11::g_bus.
// (Forward decl is placed inside the outer namespace fceu11; no
// extra `namespace fceu11 { ... }` wrapper needed.)
// ---------------------------------------------------------------------------
class Bus;

// ---------------------------------------------------------------------------
// Cart view (分类 E — owned by ines/unif + cart.cpp; v1.7 will fold into Cart)
// ---------------------------------------------------------------------------
struct CartView {
    CartInfo*& current() noexcept;            // ::currCartInfo

    uint8_t  (& prg_ram()  noexcept)[32];     // ::PRGram
    uint8_t  (& chr_ram()  noexcept)[32];     // ::CHRram
    uint8_t* (& prg_ptr()  noexcept)[32];     // ::PRGptr
    uint8_t* (& chr_ptr()  noexcept)[32];     // ::CHRptr
    uint32_t (& prg_size() noexcept)[32];
    uint32_t (& chr_size() noexcept)[32];

    uint32_t (& prg_mask2()  noexcept)[32];
    uint32_t (& prg_mask4()  noexcept)[32];
    uint32_t (& prg_mask8()  noexcept)[32];
    uint32_t (& prg_mask16() noexcept)[32];
    uint32_t (& prg_mask32() noexcept)[32];
    uint32_t (& chr_mask1()  noexcept)[32];
    uint32_t (& chr_mask2()  noexcept)[32];
    uint32_t (& chr_mask4()  noexcept)[32];
    uint32_t (& chr_mask8()  noexcept)[32];

    int& genie_stage() noexcept;              // ::geniestage

    int&         qtai_hack() noexcept;
    uint8_t (&   qtai_ntram() noexcept)[2048];
    uint8_t&     qtai_reg() noexcept;

    struct iNES_HEADER& rom_header() noexcept; // ::head
};

// ---------------------------------------------------------------------------
// Config view (分类 F — process-wide configuration; retained as globals in
// v1.x; v1.11 Bridge folds these into the State singleton.)
// ---------------------------------------------------------------------------
struct ConfigView {
    uint8_t& pal() noexcept;                  // ::PAL
    int&     dendy() noexcept;                // ::dendy
    bool&    movie_subtitles() noexcept;
    fceu_settings_struct& settings() noexcept; // ::FSettings (typedef FCEUS = fceu_settings_struct)

    int&     ram_init_option() noexcept;      // ::RAMInitOption
    uint8_t*& ram() noexcept;                 // ::RAM (main memory pointer)
    int&     emulation_paused() noexcept;
    int&     frame_advance_delay() noexcept;
    // ::GameAttributes is declared in fceu.h but has no definition in the
    // v1.0 codebase (only the forward extern exists). Deferred to v1.11
    // Bridge, which will define the symbol alongside the other core globals.
    // int&     game_attributes() noexcept;
};

// ---------------------------------------------------------------------------
// Debug view (分类 G — only meaningful when FCEUDEF_DEBUGGER is set; kept
// as globals to avoid overhead in Release builds. Accessors still resolve at
// link time, but downstream code is expected to guard with #ifdef.)
// ---------------------------------------------------------------------------
struct DebugView {
    int& in_debug() noexcept;                 // ::fceuindbg

    // The 6 below are declared `volatile int` in their owning headers
    // (debug.h / ppu.h). The volatile qualifier must propagate through
    // the facade so writes from another thread are not elided by the
    // compiler.
    volatile int& code_count() noexcept;
    volatile int& data_count() noexcept;
    volatile int& undefined_count() noexcept;
    volatile int& render_count() noexcept;
    volatile int& vrom_read_count() noexcept;
    volatile int& undefined_vrom_count() noexcept;

    int&         ia_pc() noexcept;
    uint32_t&    ia_poffset() noexcept;

    bool&   break_asap() noexcept;
    bool&   break_on_unlogged_code() noexcept;
    bool&   break_on_unlogged_data() noexcept;
    bool&   break_on_cycles() noexcept;
    bool&   break_on_instructions() noexcept;

    uint64_t& total_cycles_base() noexcept;
    uint64_t& delta_cycles_base() noexcept;
    uint64_t& break_cycles_limit() noexcept;
    uint64_t& total_instructions() noexcept;
    uint64_t& delta_instructions() noexcept;
    uint64_t& break_instructions_limit() noexcept;

    int& num_watchpoints() noexcept;          // ::numWPs

    int& debug_logging_cd() noexcept;
};

// ---------------------------------------------------------------------------
// Aggregate entry point
// ---------------------------------------------------------------------------
class State {
public:
    CpuView&   cpu()    noexcept { return cpu_;    }
    PpuView&   ppu()    noexcept { return ppu_;    }
    ApuView&   apu()    noexcept { return apu_;    }
    Bus&       bus()    noexcept;  // defined in core_state.cpp; returns g_bus
    CartView&  cart()   noexcept { return cart_;   }
    ConfigView& config() noexcept { return config_; }
    DebugView& debug()  noexcept { return debug_;  }

    // Convenience: structural version marker. Bumped by each release.
    static constexpr int census_version_v1_2 = 1;

private:
    CpuView    cpu_{};
    PpuView    ppu_{};
    ApuView    apu_{};
    CartView   cart_{};
    ConfigView config_{};
    DebugView  debug_{};
};

// Returns the process-wide singleton. The underlying storage lives in
// core_state.cpp as a function-local static (Meyers singleton) — this is
// the only allowed initialization path, since direct construction by a
// caller would silently fork the State and break the facade's contract
// that "every accessor sees the same global".
State& global_state() noexcept;

} // namespace fceu11

#endif // FCEU11_CORE_STATE_H