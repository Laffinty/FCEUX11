// FCEUX11 — v1.3 Legion: CPU objectification skeleton (implementation)

#include "cpu.h"

#include "x6502.h"

#include <cstring>

namespace fceu11 {

// Layout assertions required for savestate binary compatibility.
// The X6502 blob must remain at offset 0 inside the Cpu object so that
// legacy save/load code can copy it directly.
static_assert(offsetof(Cpu, layout_) == 0,
              "Cpu::layout_ must remain at offset 0 for savestate compatibility");
static_assert(alignof(Cpu) == 64,
              "Cpu must remain 64-byte aligned");

// ---------------------------------------------------------------------------
// Register access
// ---------------------------------------------------------------------------
uint16_t Cpu::pc() const noexcept { return layout_.PC; }
void Cpu::set_pc(uint16_t v) noexcept { layout_.PC = v; }
uint8_t Cpu::a() const noexcept { return layout_.A; }
void Cpu::set_a(uint8_t v) noexcept { layout_.A = v; }
uint8_t Cpu::x() const noexcept { return layout_.X; }
void Cpu::set_x(uint8_t v) noexcept { layout_.X = v; }
uint8_t Cpu::y() const noexcept { return layout_.Y; }
void Cpu::set_y(uint8_t v) noexcept { layout_.Y = v; }
uint8_t Cpu::s() const noexcept { return layout_.S; }
void Cpu::set_s(uint8_t v) noexcept { layout_.S = v; }
uint8_t Cpu::p() const noexcept { return layout_.P; }
void Cpu::set_p(uint8_t v) noexcept { layout_.P = v; }
bool Cpu::jammed() const noexcept { return layout_.jammed != 0; }

// v1.4 Post-Release Optimization Plan §2.2 — non-A/X/Y/S/P
// accessors. `db()` is the data-bus "cache" that ::ANull readback
// uses; `pi()` covers the legacy "mooPI" register. Other modules
// (Bus::ANullImpl, savestate, etc.) should call these instead of
// reaching into native_layout() for plain register reads/writes.
uint8_t Cpu::db() const noexcept { return layout_.DB; }
void Cpu::set_db(uint8_t v) noexcept { layout_.DB = v; }
uint8_t Cpu::pi() const noexcept { return layout_.mooPI; }
void Cpu::set_pi(uint8_t v) noexcept { layout_.mooPI = v; }

// v1.4 Post-Release Optimization Plan §2.1 — overclocking state.
bool Cpu::overclocking() const noexcept { return overclocking_; }
void Cpu::set_overclocking(bool v) noexcept { overclocking_ = v; }

// ---------------------------------------------------------------------------
// Lifecycle — delegate to the existing free functions while globals are
// still backed by this Cpu object via inline aliases.
// ---------------------------------------------------------------------------
void Cpu::init() noexcept { X6502_Init(); }
void Cpu::reset() noexcept { X6502_Reset(); }
void Cpu::power() noexcept { X6502_Power(); }
void Cpu::run(int32_t cycles) { X6502_RunDebug(*this, cycles); }

// ---------------------------------------------------------------------------
// Interrupts
// ---------------------------------------------------------------------------
void Cpu::trigger_nmi() noexcept { ::TriggerNMI(); }
void Cpu::trigger_irq(uint32_t source) noexcept { ::X6502_IRQBegin(static_cast<int>(source)); }
void Cpu::clear_irq(uint32_t source) noexcept { ::X6502_IRQEnd(static_cast<int>(source)); }

// ---------------------------------------------------------------------------
// Timestamps
// ---------------------------------------------------------------------------
int32_t Cpu::timestamp() const noexcept { return static_cast<int32_t>(timestamp_); }

uint64_t Cpu::timestamp_base() const noexcept {
    // ::timestampbase is declared in cpu.h (v1.4 Post-Release
    // Optimization Plan §1.3 — hoisted out of this function body so
    // the declaration is grouped with the other CPU-state externs).
    // The actual definition lives in fceu.cpp.
    return timestampbase;
}

// ---------------------------------------------------------------------------
// Savestate compatibility
// ---------------------------------------------------------------------------
X6502& Cpu::native_layout() noexcept { return layout_; }
const X6502& Cpu::native_layout() const noexcept { return layout_; }

// ---------------------------------------------------------------------------
// Reference accessors for legacy inline global aliases
// ---------------------------------------------------------------------------
uint32_t& Cpu::timestamp_ref() noexcept { return timestamp_; }
uint32_t& Cpu::sound_timestamp_ref() noexcept { return sound_timestamp_; }
int& Cpu::scanline_ref() noexcept { return scanline_; }
// hotfix3 B-5a: map_irq_hook_ref() is now an inline header method that
// returns a Cpu::RefProxy value. The proxy forwards both reads and writes
// to the std::atomic<MapIRQHook> field via map_irq_hook() /
// set_map_irq_hook() — see cpu.h for the rationale and B-5b for the
// eventual removal once all ~50 mapper files migrate to set_map_irq_hook().

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------
Cpu& cpu_instance() noexcept {
    static Cpu instance;
    return instance;
}

} // namespace fceu11
