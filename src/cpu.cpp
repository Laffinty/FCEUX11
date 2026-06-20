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
uint8_t Cpu::x() const noexcept { return layout_.X; }
uint8_t Cpu::y() const noexcept { return layout_.Y; }
uint8_t Cpu::s() const noexcept { return layout_.S; }
uint8_t Cpu::p() const noexcept { return layout_.P; }
bool Cpu::jammed() const noexcept { return layout_.jammed != 0; }

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
// Debug hooks
// ---------------------------------------------------------------------------
void Cpu::set_cpu_hook(std::function<void()> fn) { cpu_hook_ = std::move(fn); }
void Cpu::set_read_hook(std::function<void(uint32_t)> fn) { read_hook_ = std::move(fn); }
void Cpu::set_write_hook(std::function<void(uint32_t, uint8_t)> fn) { write_hook_ = std::move(fn); }

// ---------------------------------------------------------------------------
// Timestamps
// ---------------------------------------------------------------------------
int32_t Cpu::timestamp() const noexcept { return static_cast<int32_t>(timestamp_); }

uint64_t Cpu::timestamp_base() const noexcept {
    // ::timestampbase is still owned by fceu.cpp; this accessor is provided
    // for API symmetry with the legacy facade.
    extern uint64_t timestampbase;
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
MapIRQHook& Cpu::map_irq_hook_ref() noexcept { return map_irq_hook_; }

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------
Cpu& cpu_instance() noexcept {
    static Cpu instance;
    return instance;
}

} // namespace fceu11
