// FCEUX11 — v1.3 Legion: CPU objectification skeleton
//
// Phase 1 introduces fceu11::Cpu, the single owner of the CPU execution
// state previously held by the file-scope globals ::X, ::timestamp,
// ::soundtimestamp, ::scanline and ::MapIRQHook. The underlying X6502
// layout remains at offset 0 inside the class so that savestate code
// can continue to treat native_layout() as a plain memory blob.
//
// Old global symbols are preserved as inline reference aliases (see
// bottom of this header and src/x6502.h) so the rest of the codebase
// can migrate file-by-file without a break-all commit.
//
// Phase 7 (2026-08-22): the C++ 6502 CPU implementation
// (src/x6502.{cpp,h,struct.h,abbrev.h} + src/ops.inc +
// src/ops_table.inc) was deleted; the Rust CPU in fceux11-core is the
// only implementation. The X6502 savestate-layout struct, the legacy
// register-access macros, the flag/IRQ constants, the opcode tables
// and the remaining X6502_* helper surface that other modules still
// consume were moved into this header (declarations) and src/cpu.cpp
// (definitions).

#ifndef FCEU11_CPU_H
#define FCEU11_CPU_H

#include <cstdint>
#include <cstddef>
#include <atomic>

#include "types.h"               // legacy uint8/uint16/uint32/int32 typedefs
#include "fceu11_core_types.h"   // fceu11::MapIRQHook
#include "utils/memory.h"  // v1.14 Anvil: FCEUX11_DEPRECATED macro

// Legacy global still read by Cpu::timestamp_base(). v1.4
// Post-Release Optimization Plan §1.3 — hoisted the function-local
// `extern uint64_t timestampbase;` from cpu.cpp into this header so
// the declaration lives next to the other CPU-state externs. The
// actual definition stays in fceu.cpp.
//
// (Plan §2.1 moved `overclocking` into Cpu as a private member with
//  public accessors — see Cpu::overclocking() / set_overclocking() —
//  and the legacy `extern bool overclocking;` is gone.)
extern uint64_t timestampbase;

// ---------------------------------------------------------------------------
// X6502 — the 64-byte savestate layout blob (formerly src/x6502struct.h).
// The Rust `X6502Layout` in fceux11-core is pinned to the same byte
// offsets by `offset_of!` asserts (src/rust/.../cpu/state.rs) and matches
// the C++ static_asserts in src/cpu.cpp. `Cpu::layout_` must remain at
// offset 0 inside the Cpu object so that legacy save/load code can copy
// the blob directly. Any future layout change must update both sides.
// ---------------------------------------------------------------------------
#ifdef _MSC_VER
#pragma warning(push)
#pragma warning(disable: 4324)
#endif
// hotfix1 P3-1: identifiers beginning with two underscores (or with an
// underscore followed by an upper-case letter) are reserved for the
// implementation per the C and C++ standards. `__X6502` was therefore
// technically a strict-aliasing / name-collision hazard on conforming
// toolchains. Rename the struct tag to `X6502` (the same name as the
// public typedef at the bottom of this file) and update the few
// internal references — the public typedef is unchanged so all existing
// `X6502 foo;` and `X6502 *bar;` callers keep working.
typedef struct alignas(64) X6502 {
  int32 tcount;     /* Temporary cycle counter */
  uint16 PC;        /* I'll change this to uint32 later... */
                                /* I'll need to AND PC after increments to 0xFFFF */
                                /* when I do, though.  Perhaps an IPC() macro? */
        uint8 A,X,Y,S,P,mooPI;
        uint8 jammed;

	int32 count;
  uint32 IRQlow;    /* Simulated IRQ pin held low(or is it high?).
                                   And other junk hooked on for speed reasons.*/
  uint8 DB;         /* Data bus "cache" for reads from certain areas */

  int preexec;      /* Pre-exec'ing for debug breakpoints. */

	#ifdef FCEUDEF_DEBUGGER
        void (*CPUHook)(struct X6502 *);
        uint8 (*ReadHook)(struct X6502 *, unsigned int);
        void (*WriteHook)(struct X6502 *, unsigned int, uint8);
	#endif

} X6502;
#ifdef _MSC_VER
#pragma warning(pop)
#endif

namespace fceu11 {

class alignas(64) Cpu {
public:
    // Register access
    uint16_t pc() const noexcept;
    void set_pc(uint16_t v) noexcept;
    uint8_t a() const noexcept;
    void set_a(uint8_t v) noexcept;
    uint8_t x() const noexcept;
    void set_x(uint8_t v) noexcept;
    uint8_t y() const noexcept;
    void set_y(uint8_t v) noexcept;
    uint8_t s() const noexcept;
    void set_s(uint8_t v) noexcept;
    uint8_t p() const noexcept;
    void set_p(uint8_t v) noexcept;
    bool jammed() const noexcept;

    // v1.4 Post-Release Optimization Plan §2.2 — non-A/X/Y/S/P
    // register accessors. `db()` covers the data-bus "cache" reads
    // (used by ::ANull / open-bus readback); `pi()` covers the
    // legacy "mooPI" register, kept under the friendlier accessor
    // name. Other modules (Bus::ANullImpl, savestate, etc.) should
    // use these instead of native_layout().DB / native_layout().mooPI.
    uint8_t db() const noexcept;
    void set_db(uint8_t v) noexcept;
    uint8_t pi() const noexcept;
    void set_pi(uint8_t v) noexcept;

    // v1.4 Post-Release Optimization Plan §2.1 — `overclocking`
    // moved from a free-floating `bool ::overclocking` global (set
    // by ppu.cpp, read by Cpu::add_cycles and x6502.cpp) into Cpu as
    // a private member with public accessors. Callers (ppu.cpp,
    // x6502.cpp, future v1.5+ modules) now go through
    // g_cpu.set_overclocking() / g_cpu.overclocking() instead of
    // touching a global.
    bool overclocking() const noexcept;
    void set_overclocking(bool v) noexcept;

    // Lifecycle
    void init() noexcept;
    void reset() noexcept;
    void power() noexcept;
    void run(int32_t cycles);

    // Interrupts
    void trigger_nmi() noexcept;
    void trigger_irq(uint32_t source) noexcept;
    void clear_irq(uint32_t source) noexcept;

    // v1.4 Post-Release Optimization Plan §1.4 — the
    // cpu_hook_ / read_hook_ / write_hook_ std::function members and
    // their set_*_hook setters were placeholders for the v1.3 Roadmap
    // §3.1 "Phase 3/4 接入 legacy hook 槽位" promise that never landed.
    // Removing them is safer than wiring them up: a caller that sets
    // a hook today would think it triggers, but no code path actually
    // invokes the std::function. v1.5+ real hook needs should go
    // through fceu11::State::debug() (Roadmap §2.2).
    //
    // (No replacement setters — the member fields below are gone too.)

    // Timestamps
    int32_t timestamp() const noexcept;
    uint64_t timestamp_base() const noexcept;

    // Cycle accounting (v1.3 Legion Phase 3).
    // Replaces the ADDCYC macro so the CPU hot path can operate on a Cpu&
    // parameter without going through the legacy global aliases.
    void add_cycles(int32_t c) noexcept {
        layout_.tcount += c;
        layout_.count -= c * 48;
        timestamp_ += c;
        if (!overclocking_) sound_timestamp_ += c;
    }

    // Savestate compatibility: only serialization code should use this.
    X6502& native_layout() noexcept;
    const X6502& native_layout() const noexcept;

    // Reference accessors used by the legacy inline global aliases.
    uint32_t& timestamp_ref() noexcept;
    uint32_t& sound_timestamp_ref() noexcept;
    int& scanline_ref() noexcept;

    // hotfix3 B-5a (A-CRASH-06): the map_irq_hook function pointer is set
    // by mapper init (cold path) and read by the CPU every instruction
    // (hot path). std::atomic<MapIRQHook> gives a happens-before guarantee
    // between the mapper-init completion and the first CPU read of the
    // hook pointer.
    //
    // hotfix3 B-5b: the legacy map_irq_hook_ref() accessor (returning
    // MapIRQHook&) is removed. All ~50 mapper files now go through
    // set_map_irq_hook() (which stores atomically with release
    // ordering), and the hot path reads via map_irq_hook() (atomic
    // acquire load).
    MapIRQHook map_irq_hook() const noexcept {
        return map_irq_hook_.load(std::memory_order_acquire);
    }
    void set_map_irq_hook(MapIRQHook h) noexcept {
        map_irq_hook_.store(h, std::memory_order_release);
    }

    // hotfix2 P1-7 (MAP-4): value-return / setter pair for the
    // scanline counter. `scanline_ref()` returns `int&` which forces
    // the value into memory and inhibits register caching in the
    // hot DoLine / RefreshLine paths. The new `scanline()` (value)
    // + `set_scanline(v)` (write) pair let the compiler keep the
    // counter in a register across the call. The legacy
    // `scanline_ref()` is kept for back-compat with all the
    // mapper / debug call sites that aren't in the hot path.
    int scanline() const noexcept { return scanline_; }
    void set_scanline(int v) noexcept { scanline_ = v; }

    X6502 layout_{};                 // must remain at offset 0

private:
    uint32_t timestamp_ = 0;         // ::timestamp
    uint32_t sound_timestamp_ = 0;   // ::soundtimestamp
    int scanline_ = 0;               // ::scanline
    // hotfix3 B-5a: std::atomic<MapIRQHook> (function pointer). Same
    // sizeof as the old plain MapIRQHook (8 bytes on 64-bit), so the
    // surrounding Cpu class layout is unchanged and savestate assertions
    // on layout_ still hold.
    std::atomic<MapIRQHook> map_irq_hook_{nullptr}; // ::MapIRQHook
    bool overclocking_ = false;      // ::overclocking (Plan §2.1)
};

// Global singleton. Meyers pattern keeps initialization lazy and thread-safe.
Cpu& cpu_instance() noexcept;

} // namespace fceu11

// v1.14 Anvil §14.5: compatibility alias, deprecated for v2.0 removal.
FCEUX11_DEPRECATED("use fceu11::cpu_instance() instead")
inline auto& g_cpu = fceu11::cpu_instance();

// ---------------------------------------------------------------------------
// Phase 7: legacy CPU-facing surface preserved for the remaining C++
// consumers (migrated from the deleted src/x6502.h and
// src/x6502abbrev.h). The C++ X6502 dispatch loop is gone; the free
// functions below are thin helpers around the Rust-backed Cpu facade
// and are defined in src/cpu.cpp.
// ---------------------------------------------------------------------------

// v1.3 Legion Phase 1: legacy CPU-state globals are now inline reference
// aliases into the single fceu11::Cpu instance. Existing source files that
// read/write X, timestamp, soundtimestamp, scanline or MapIRQHook continue
// to compile and link without changes.
FCEUX11_DEPRECATED("use fceu11::cpu_instance().native_layout() instead")
inline auto& X = fceu11::cpu_instance().native_layout();

FCEUX11_DEPRECATED("use fceu11::cpu_instance().timestamp_ref() instead")
inline auto& timestamp = fceu11::cpu_instance().timestamp_ref();
FCEUX11_DEPRECATED("use fceu11::cpu_instance().sound_timestamp_ref() instead")
inline auto& soundtimestamp = fceu11::cpu_instance().sound_timestamp_ref();
FCEUX11_DEPRECATED("use fceu11::cpu_instance().scanline_ref() instead")
inline auto& scanline = fceu11::cpu_instance().scanline_ref();

// v1.13 Purify H: #define → constexpr (6502 P-register flag masks)
inline constexpr uint8_t N_FLAG = 0x80;
inline constexpr uint8_t V_FLAG = 0x40;
inline constexpr uint8_t U_FLAG = 0x20;
inline constexpr uint8_t B_FLAG = 0x10;
inline constexpr uint8_t D_FLAG = 0x08;
inline constexpr uint8_t I_FLAG = 0x04;
inline constexpr uint8_t Z_FLAG = 0x02;
inline constexpr uint8_t C_FLAG = 0x01;

// v1.14 Anvil: #define → inline function (depends on runtime ::dendy).
// `dendy` is also declared in fceu.h; the definition lives in the
// driver layer (drivers/Qt/fceu_globals.cpp, drivers/null/null_driver.cpp).
extern int dendy;
inline double NTSC_CPU_freq() { return dendy ? 1773447.467 : 1789772.7272727272727272; }
#define NTSC_CPU NTSC_CPU_freq()
inline constexpr double PAL_CPU = 1662607.125;  // v1.13 Purify H: #define → constexpr

// v1.13 Purify H: #define → constexpr (IRQ source bitmasks)
inline constexpr uint32_t FCEU_IQEXT    = 0x001;
inline constexpr uint32_t FCEU_IQEXT2   = 0x002;
inline constexpr uint32_t FCEU_IQRESET  = 0x020;
inline constexpr uint32_t FCEU_IQNMI2   = 0x040;  // Delayed NMI, gets converted to FCEU_IQNMI
inline constexpr uint32_t FCEU_IQNMI    = 0x080;
inline constexpr uint32_t FCEU_IQDPCM   = 0x100;
inline constexpr uint32_t FCEU_IQFCOUNT = 0x200;
inline constexpr uint32_t FCEU_IQTEMP   = 0x800;

// v1.3 Legion Phase 3: `X6502_Run(cycles)` — the legacy dispatch entry.
// The C++ `X6502_RunDebug` function was deleted in Phase 7; the macro
// now forwards to the Rust-backed Cpu facade (same behaviour as the
// Phase 3-6 ON build, where X6502_RunDebug was a one-line forward to
// Cpu::run).
#define X6502_Run(cycles) fceu11::cpu_instance().run(cycles)

//the opsize table is used to quickly grab the instruction sizes (in bytes)
extern const uint8 opsize[256];

//the optype table is a quick way to grab the addressing mode for any 6502 opcode
extern const uint8 optype[256];

// the opwrite table aids in predicting the value written for any 6502 opcode
extern const uint8 opwrite[256];

// E-1 probe (Phase 1 Step 1.3): last instruction PC observed at the current
// CPU boundary (set at the start of each instruction fetch in the run loop).
uint16 fceu11_e1_last_pc();

// Phase 4 closeout: NMI-fresh flag bridge accessors (see cpu.cpp).
bool x6502_nmi_fresh_get(void);
void x6502_nmi_fresh_set(bool v);

// Legacy bus-access helpers (still used by ines_gi.cpp / ppu.cpp /
// sound.cpp). Charge one CPU cycle and fire the Lua memory hooks, like
// the C++ dispatch loop's RdMem/WrMem did.
uint8 X6502_DMR(uint32 A);
void X6502_DMW(uint32 A, uint8 V);

// Legacy IRQ pin control. These OR/AND the `IRQlow` blob bits; the Rust
// CPU re-reads that blob from the host at every dispatch boundary via
// the IRQ bridge, so the semantics are unchanged from the C++ CPU.
void X6502_IRQBegin(int w);
void X6502_IRQEnd(int w);

int X6502_GetOpcodeCycles(int op);

// Legacy NMI assert paths (VBL NMI from the PPU loop, W2000-edge NMI2).
void TriggerNMI(void);
void TriggerNMI2(void);

// Legacy register-access macros (formerly src/x6502abbrev.h), still used
// by src/debug.cpp for watchpoint/breakpoint bookkeeping.
#define _PC        g_cpu.native_layout().PC
#define _A         g_cpu.native_layout().A
#define _X         g_cpu.native_layout().X
#define _Y         g_cpu.native_layout().Y
#define _S         g_cpu.native_layout().S
#define _P         g_cpu.native_layout().P
#define _PI        g_cpu.native_layout().mooPI
#define _DB        g_cpu.native_layout().DB
#define _count     g_cpu.native_layout().count
#define _tcount    g_cpu.native_layout().tcount
#define _IRQlow    g_cpu.native_layout().IRQlow
#define _jammed    g_cpu.native_layout().jammed

#endif // FCEU11_CPU_H
