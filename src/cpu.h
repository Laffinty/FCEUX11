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
#include <functional>

#include "x6502struct.h"
#include "fceu11_core_types.h"

namespace fceu11 {

class alignas(64) Cpu {
public:
    // Register access
    uint16_t pc() const noexcept;
    void set_pc(uint16_t v) noexcept;
    uint8_t a() const noexcept;
    uint8_t x() const noexcept;
    uint8_t y() const noexcept;
    uint8_t s() const noexcept;
    uint8_t p() const noexcept;
    bool jammed() const noexcept;

    // Lifecycle
    void init() noexcept;
    void reset() noexcept;
    void power() noexcept;
    void run(int32_t cycles);

    // Interrupts
    void trigger_nmi() noexcept;
    void trigger_irq(uint32_t source) noexcept;
    void clear_irq(uint32_t source) noexcept;

    // Debug hooks (stored now; wired to the legacy hook slots in Phase 3/4)
    void set_cpu_hook(std::function<void()> fn);
    void set_read_hook(std::function<void(uint32_t)> fn);
    void set_write_hook(std::function<void(uint32_t, uint8_t)> fn);

    // Timestamps
    int32_t timestamp() const noexcept;
    uint64_t timestamp_base() const noexcept;

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

    std::function<void()> cpu_hook_;
    std::function<void(uint32_t)> read_hook_;
    std::function<void(uint32_t, uint8_t)> write_hook_;
};

// Global singleton. Meyers pattern keeps initialization lazy and thread-safe.
Cpu& cpu_instance() noexcept;

} // namespace fceu11

// Compatibility alias used by new call sites.
inline auto& g_cpu = fceu11::cpu_instance();

#endif // FCEU11_CPU_H
