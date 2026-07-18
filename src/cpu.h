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

#ifndef FCEU11_CPU_H
#define FCEU11_CPU_H

#include <cstdint>
#include <cstddef>
#include <atomic>

#include "x6502struct.h"
#include "fceu11_core_types.h"
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

    // hotfix3 B-5a (A-CRASH-06): the map_irq_hook pointer is set by mapper
    // init (cold path) and read by the CPU every instruction (hot path).
    // std::atomic<MapIRQHook> gives a happens-before guarantee between the
    // mapper-init completion and the first CPU read of the hook pointer.
    //
    // map_irq_hook_ref() is kept for back-compat with the ~50 mapper files
    // that still write `g_cpu.map_irq_hook_ref() = SomeFunc;` plus the
    // CpuView::irq_hook() debugger accessor. Because std::atomic<MapIRQHook>
    // does not expose a true C++ reference, map_irq_hook_ref() returns a
    // RefProxy value whose assignment operator= and conversion-to-MapIRQHook
    // forward to set_map_irq_hook() / map_irq_hook() respectively. The proxy
    // is a value, not a reference: it cannot be stored across statements.
    // B-5b will migrate all ~50 mapper files to set_map_irq_hook() and then
    // remove the proxy.
    class RefProxy;
    MapIRQHook map_irq_hook() const noexcept {
        return map_irq_hook_.load(std::memory_order_acquire);
    }
    void set_map_irq_hook(MapIRQHook h) noexcept {
        map_irq_hook_.store(h, std::memory_order_release);
    }
    RefProxy map_irq_hook_ref() noexcept { return RefProxy{this}; }

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

public:
    // hotfix3 B-5a: RefProxy — value-type proxy returned by
    // map_irq_hook_ref(). Lets the existing
    //   `g_cpu.map_irq_hook_ref() = SomeFunc;`
    // pattern (used in ~50 mapper files) compile unchanged while routing
    // both reads and writes through the std::atomic<MapIRQHook> field.
    class RefProxy {
        Cpu* cpu_;
    public:
        explicit RefProxy(Cpu* cpu) noexcept : cpu_(cpu) {}
        // Read conversion: load from atomic.
        operator MapIRQHook() const noexcept { return cpu_->map_irq_hook(); }
        // Write: atomic store via set_map_irq_hook().
        RefProxy& operator=(MapIRQHook h) noexcept {
            cpu_->set_map_irq_hook(h);
            return *this;
        }
    };
};

// Global singleton. Meyers pattern keeps initialization lazy and thread-safe.
Cpu& cpu_instance() noexcept;

} // namespace fceu11

// v1.14 Anvil §14.5: compatibility alias, deprecated for v2.0 removal.
FCEUX11_DEPRECATED("use fceu11::cpu_instance() instead")
inline auto& g_cpu = fceu11::cpu_instance();

#endif // FCEU11_CPU_H
