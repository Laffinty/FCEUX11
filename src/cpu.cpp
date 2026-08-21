// FCEUX11 — v1.3 Legion: CPU objectification skeleton (implementation)

#include "cpu.h"

#include "x6502.h"

// Phase 4.5 cycle-drift diagnostic: Cpu::run invokes the cycle-trace
// sink whenever `FCEUX11_CYCLE_LOG` is set (env-var-gated, single
// branch per call). The C++ bridge owns the sink; see
// src/kagami_bridge.cpp::CycleTraceSink.
#include "kagami_bridge.h"

#if FCEUX11_RUST_CPU
// Phase 3 (revised) step 3: when the Rust 6502 is wired in, every
// facade method that previously called a free `X6502_*` function
// dispatches into the Rust side via the cbindgen-emitted
// `fceux11_rust.h`. The 64-byte X6502 layout is byte-compatible
// between Rust (`X6502Layout`) and C++ (`X6502`); only the bus
// surface needs special handling (see below).
#include "rust/fceux11_rust.h"
#include "bus.h"
#include "sound.h"   // PR-A: FCEU_SoundCPUHook for the per-instruction tick thunk
#include <atomic>
#endif

#include <cstdlib>  // std::getenv
#include <cstring>

namespace fceu11 {

// Layout assertions required for savestate binary compatibility.
// The X6502 blob must remain at offset 0 inside the Cpu object so that
// legacy save/load code can copy it directly.
static_assert(offsetof(Cpu, layout_) == 0,
              "Cpu::layout_ must remain at offset 0 for savestate compatibility");
static_assert(alignof(Cpu) == 64,
              "Cpu must remain 64-byte aligned");
// hotfix3 E-4: pin the total size too. alignas(64) on the underlying
// X6502 (x6502struct.h:16) forces sizeof to a 64-byte multiple.
// Release payload is 32 B; adding the three debugger-only function
// pointers in FCEUDEF_DEBUGGER builds brings payload to 56 B - both
// pad up to 64. A future field addition that shifts either count
// must break this assert, not silently corrupt the X6502 memory blob.
static_assert(sizeof(Cpu::layout_) == 64,
              "Cpu::layout_ must remain 64 bytes total (X6502 + padding); "
              "see alignas(64) on X6502 in x6502struct.h");

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
#if FCEUX11_RUST_CPU
// Phase 3 (revised) step 3: Cpu facade dispatches into the Rust 6502.
// Bus access flows back through two thin thunks (see below) that call
// `fceu11::g_bus.read/write`, mirroring what the C++ X6502_RunDebug
// loop's `RdMem` / `WrMem` inlines do.

namespace {
// Thin shims that turn the C-side FFI signature into a fceu11::Bus
// call. Installed once at process start by `cpu_rust_install_bus`.
// The shims are `extern "C"` because that's what the Rust FFI's
// `ReadFn` / `WriteFn` typedefs require; the C++ side wraps them in
// function-pointer casts that match the typedef.
extern "C" uint8_t cpu_rust_read_thunk(uint16_t addr) {
    return fceu11::g_bus.read(addr);
}
extern "C" void cpu_rust_write_thunk(uint16_t addr, uint8_t val) {
    fceu11::g_bus.write(addr, val);
}

// One-shot installation of the bus callbacks. Called from the Cpu
// facade before the first FFI call. Safe to call multiple times —
// the Rust side just overwrites the slots. We avoid static-init
// order issues by lazily installing on the first Cpu facade call.
// `fceux11_cpu_set_irq_bridge` is declared manually here because the
// cbindgen-emitted `fceux11_rust.h` does not yet carry the IRQ-sync
// symbols (Phase 4.5 addition).
extern "C" void fceux11_cpu_set_irq_bridge(
    uint32_t (*get_fn)(void), void (*set_fn)(uint32_t));
extern "C" void fceux11_cpu_set_nmi_fresh_bridge(
    bool (*get_fn)(void), void (*set_fn)(bool));
extern "C" void fceux11_cpu_set_tick_cycles(void (*fn)(int));
void cpu_rust_install_bus_once() noexcept {
    static std::atomic<bool> installed{false};
    bool expected = false;
    if (installed.compare_exchange_strong(expected, true)) {
        fceux11_cpu_set_bus(&cpu_rust_read_thunk, &cpu_rust_write_thunk);
        // Phase 4.5: install the IRQ-low sync callbacks. The Rust CPU
        // re-reads / re-writes the C++ `X6502::IRQlow` blob around
        // every dispatch boundary so IRQs asserted by the C++ side
        // during a Rust call (mapper hooks, APU frame-counter IRQ)
        // are visible to the Rust dispatch, and bits the Rust dispatch
        // consumed are not re-asserted on the next call.
        fceux11_cpu_set_irq_bridge(&kagami_bridge_get_cpu_irq_low,
                                   &kagami_bridge_set_cpu_irq_low);
        // Phase 4 closeout: sync the NMI-fresh deferral flag too. The
        // C++ `g_e1_nmi_fresh` is the reference for the VBL NMI's
        // one-instruction deferral; without this bridge the Rust CPU
        // dispatches the NMI one boundary early (nestest NMI test).
        fceux11_cpu_set_nmi_fresh_bridge(&kagami_bridge_get_cpu_nmi_fresh,
                                         &kagami_bridge_set_cpu_nmi_fresh);
    }
}

// PR-A (Phase 4 sub-step 6 part 2 follow-up): per-instruction tick
// thunk. The Rust CPU's `run_with_tick` loop calls `f(cycles)` once
// per executed instruction with the instruction's CPU cycle count.
// We forward `cycles` to the mapper-installed IRQ hook (if any) and
// the APU's CPU clock — matching the body of the C++
// `X6502_RunDebug` loop at `src/x6502.cpp:611-614`, which exists only
// in the `#if !FCEUX11_RUST_CPU` branch.
//
// The thunk is `extern "C"` so it can be passed to
// `fceux11_cpu_set_tick` (which takes `extern "C" fn(i32)`); the body
// freely uses C++ (atomic acquire load on `map_irq_hook_` per hotfix3
// B-5a, free function `FCEU_SoundCPUHook`).
//
// Mirrors the C++ reference loop's exact ordering:
//   1. mapper IRQ hook (skipped if no mapper hook installed)
//   2. APU sound CPU hook (skipped under overclocking)
//
// Phase 4.5 (IRQ/bank-sync fix): the thunk ALSO advances
// `timestamp_` / `sound_timestamp_` per instruction, exactly like the
// C++ `add_cycles()` does per `ADDCYC`. Without this, `timestamp_`
// only moves once per `Cpu::run` call (the `consumed` return), so
// hardware that checks `timestamp` BETWEEN instructions within a
// single call — e.g. the MMC1 `lreset` write-throttle at
// `src/boards/mmc1.cpp:136-138` ("busy, ignore the write") — sees a
// frozen timestamp and drops legitimate writes. Per-instruction
// advancement restores the C++ reference behaviour.
// Phase 4 closeout: the pre-body hook call now receives C++'s exact
// `temp = _tcount` (prev extras + dispatch + base), and the post-body
// timestamp advance uses the iteration's full total (dispatch + base
// + extras), matching C++ `add_cycles` totals. See
// docs/plans/phase4-closeout-2026-08-20.md.
extern "C" void cpu_rust_tick_thunk(int temp) {
    if (const auto hook = g_cpu.map_irq_hook()) [[unlikely]] hook(temp);
    if (!g_cpu.overclocking()) [[likely]] FCEU_SoundCPUHook(temp);
}
extern "C" void cpu_rust_tick_cycles_thunk(int cycles) {
    if (!g_cpu.overclocking()) [[likely]] {
        g_cpu.timestamp_ref() += cycles;
        g_cpu.sound_timestamp_ref() += cycles;
    } else {
        g_cpu.timestamp_ref() += cycles;
    }
}

// One-shot installation of the tick thunk. Same lazy pattern as
// `cpu_rust_install_bus_once` to avoid static-init order issues.
// Called from `Cpu::run()` before the first `fceux11_cpu_run_with_tick`.
void cpu_rust_install_tick_once() noexcept {
    static std::atomic<bool> installed{false};
    bool expected = false;
    if (installed.compare_exchange_strong(expected, true)) {
        fceux11_cpu_set_tick(&cpu_rust_tick_thunk);
        // Phase 4 closeout: post-body timestamp advance (see thunk).
        fceux11_cpu_set_tick_cycles(&cpu_rust_tick_cycles_thunk);
    }
}
} // namespace
#endif // FCEUX11_RUST_CPU

void Cpu::init() noexcept {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    fceux11_cpu_init(reinterpret_cast<uint8_t*>(&layout_));
#else
    X6502_Init();
#endif
}
void Cpu::reset() noexcept {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    fceux11_cpu_reset(reinterpret_cast<uint8_t*>(&layout_));
#else
    X6502_Reset();
#endif
}
void Cpu::power() noexcept {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    fceux11_cpu_power(reinterpret_cast<uint8_t*>(&layout_));
#else
    X6502_Power();
#endif
}
// Cached lookup of `FCEUX11_CYCLE_LOG`. The env-var is read once
// at first call; subsequent calls read the cached bool. This is
// the difference between 0-cycle overhead (disabled) and a
// 3.5× empirical regression caused by per-call std::getenv.
static bool cycle_trace_enabled() {
    static const bool enabled = []() {
        const char* p = std::getenv("FCEUX11_CYCLE_LOG");
        return p != nullptr && *p != '\0';
    }();
    return enabled;
}

void Cpu::run(int32_t cycles) {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    // PR-A: install the tick thunk before each run so that the Rust
    // CPU's `run_with_tick` loop forwards each per-instruction cycle
    // count to the mapper IRQ hook + APU sound hook. Matches the
    // C++ `X6502_RunDebug` body in the OFF build (the reference
    // branch at src/x6502.cpp:611-614).
    cpu_rust_install_tick_once();
    // PR-A: use the tick-aware FFI entry point so per-instruction
    // mapper/APU hooks fire under `FCEUX11_RUST_CPU=ON`. Without
    // this, mappers whose IRQ counters depend on CPU cycles (MMC3
    // scanline, VRC6 clock divider, DMC rate timer) never advance,
    // producing the per-frame cycle-accounting drift documented in
    // docs/plans/phase4-interrupts-2026-08-18.md §3.2.
    //
    // The FFI returns the total CPU cycles consumed during this run
    // (sum of every opcode's base cycle cost + page-cross / branch-
    // taken extras). We need to advance `timestamp_` /
    // `sound_timestamp_` ourselves because the Rust CPU doesn't have
    // direct access to the `Cpu` object — those fields live outside
    // the 64-byte X6502 layout.
    int32_t consumed = fceux11_cpu_run_with_tick(
        reinterpret_cast<uint8_t*>(&layout_), cycles);
    // Phase 4.5: timestamp_ / sound_timestamp_ are advanced by the
    // per-instruction tick thunk (see `cpu_rust_tick_thunk`), NOT by
    // the whole-call `consumed` return. Advancing them here once per
    // call would leave them frozen BETWEEN instructions within the
    // call, breaking hardware that reads `timestamp` mid-call (the
    // MMC1 `lreset` write throttle at src/boards/mmc1.cpp:136-138).
    (void)consumed;
#else
    X6502_RunDebug(*this, cycles);
#endif

    // Phase 4.5 cycle-trace diagnostic: when `FCEUX11_CYCLE_LOG` is
    // set, record this call's input cycle budget, the post-call PC,
    // and `state.count` (1/16-dot-unit accumulator that survived the
    // call). The env-var lookup is cached once at first call (see
    // `cycle_trace_enabled()` below) so the disabled path is a
    // single load-and-branch — no syscalls per `Cpu::run` call.
    if (cycle_trace_enabled()) [[unlikely]] {
        uint16_t pc = layout_.PC;
        uint32_t cum_count = static_cast<uint32_t>(layout_.count);
        uint32_t irq_low = static_cast<uint32_t>(layout_.IRQlow);
        kagami_bridge_cycle_trace_record(
            static_cast<uint32_t>(cycles), pc, cum_count, irq_low);
    }
}

// ---------------------------------------------------------------------------
// Interrupts
// ---------------------------------------------------------------------------
void Cpu::trigger_nmi() noexcept {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    fceux11_cpu_trigger_nmi(reinterpret_cast<uint8_t*>(&layout_));
#else
    ::TriggerNMI();
#endif
}
void Cpu::trigger_irq(uint32_t source) noexcept {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    fceux11_cpu_irq_begin(reinterpret_cast<uint8_t*>(&layout_), source);
#else
    ::X6502_IRQBegin(static_cast<int>(source));
#endif
}
void Cpu::clear_irq(uint32_t source) noexcept {
#if FCEUX11_RUST_CPU
    cpu_rust_install_bus_once();
    fceux11_cpu_irq_end(reinterpret_cast<uint8_t*>(&layout_), source);
#else
    ::X6502_IRQEnd(static_cast<int>(source));
#endif
}

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
// hotfix3 B-5b: Cpu::map_irq_hook_ref() removed. map_irq_hook() and
// set_map_irq_hook() are inline in cpu.h and operate on the
// std::atomic<MapIRQHook> field directly.

// ---------------------------------------------------------------------------
// Singleton
// ---------------------------------------------------------------------------
Cpu& cpu_instance() noexcept {
    static Cpu instance;
    return instance;
}

} // namespace fceu11
