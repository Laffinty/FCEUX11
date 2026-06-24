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

#include "x6502struct.h"
#include "fceu11_core_types.h"

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
    MapIRQHook& map_irq_hook_ref() noexcept;

    X6502 layout_{};                 // must remain at offset 0

private:
    uint32_t timestamp_ = 0;         // ::timestamp
    uint32_t sound_timestamp_ = 0;   // ::soundtimestamp
    int scanline_ = 0;               // ::scanline
    MapIRQHook map_irq_hook_ = nullptr; // ::MapIRQHook
    bool overclocking_ = false;      // ::overclocking (Plan §2.1)
};

// Global singleton. Meyers pattern keeps initialization lazy and thread-safe.
Cpu& cpu_instance() noexcept;

} // namespace fceu11

// Compatibility alias used by new call sites.
inline auto& g_cpu = fceu11::cpu_instance();

#endif // FCEU11_CPU_H
