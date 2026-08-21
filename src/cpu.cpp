// FCEUX11 — v1.3 Legion: CPU objectification skeleton (implementation)
//
// Phase 7 (2026-08-22): the C++ 6502 CPU was deleted; the Rust 6502 in
// fceux11-core is the only implementation. Every facade method
// dispatches into the Rust side via the cbindgen-emitted
// `fceux11_rust.h`. The 64-byte X6502 layout is byte-compatible
// between Rust (`X6502Layout`) and C++ (`X6502`); only the bus
// surface needs special handling (see below).

#include "cpu.h"

#include "types.h"
#include "fceu.h"         // ::fceuindbg (FCEUI_GetIVectors)
#include "core_api.h"     // fceu11::NMI/IRQ, FCEUI_GetIVectors
#include "utils/cache.h"  // FCEUX11_CACHE_ALIGN (opcode tables)

// Phase 4.5 cycle-drift diagnostic: Cpu::run invokes the cycle-trace
// sink whenever `FCEUX11_CYCLE_LOG` is set (env-var-gated, single
// branch per call). The C++ bridge owns the sink; see
// src/kagami_bridge.cpp::CycleTraceSink.
#include "kagami_bridge.h"

#include "rust/fceux11_rust.h"
#include "bus.h"
#include "sound.h"   // PR-A: FCEU_SoundCPUHook for the per-instruction tick thunk
#ifdef _S9XLUA_H
#include "fceulua.h" // CallRegisteredLuaMemHook (X6502_DMR/X6502_DMW)
#endif
#include <atomic>
#include <cstdio>   // e1 trace probes (TriggerNMI)
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
// Lifecycle — delegate to the Rust 6502 (the only CPU implementation
// since Phase 7).
// ---------------------------------------------------------------------------
// Cpu facade dispatches into the Rust 6502.
// Bus access flows back through two thin thunks (see below) that call
// `fceu11::g_bus.read/write`, mirroring what the deleted C++
// `X6502_RunDebug` loop's `RdMem` / `WrMem` inlines do.

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
// the APU's CPU clock — matching the body of the deleted C++
// `X6502_RunDebug` loop (which existed only in the pre-Phase-7 OFF
// build).
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

void Cpu::init() noexcept {
    cpu_rust_install_bus_once();
    fceux11_cpu_init(reinterpret_cast<uint8_t*>(&layout_));
}
void Cpu::reset() noexcept {
    cpu_rust_install_bus_once();
    fceux11_cpu_reset(reinterpret_cast<uint8_t*>(&layout_));
}
void Cpu::power() noexcept {
    cpu_rust_install_bus_once();
    fceux11_cpu_power(reinterpret_cast<uint8_t*>(&layout_));
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
    cpu_rust_install_bus_once();
    // PR-A: install the tick thunk before each run so that the Rust
    // CPU's `run_with_tick` loop forwards each per-instruction cycle
    // count to the mapper IRQ hook + APU sound hook. Matches the
    // body of the deleted C++ `X6502_RunDebug` loop (which existed
    // only in the pre-Phase-7 OFF build).
    cpu_rust_install_tick_once();
    // PR-A: use the tick-aware FFI entry point so per-instruction
    // mapper/APU hooks fire. Without
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
    cpu_rust_install_bus_once();
    fceux11_cpu_trigger_nmi(reinterpret_cast<uint8_t*>(&layout_));
}
void Cpu::trigger_irq(uint32_t source) noexcept {
    cpu_rust_install_bus_once();
    fceux11_cpu_irq_begin(reinterpret_cast<uint8_t*>(&layout_), source);
}
void Cpu::clear_irq(uint32_t source) noexcept {
    cpu_rust_install_bus_once();
    fceux11_cpu_irq_end(reinterpret_cast<uint8_t*>(&layout_), source);
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

// ---------------------------------------------------------------------------
// Phase 7: legacy CPU-facing free functions migrated from the deleted
// src/x6502.cpp. These are thin helpers around the Rust-backed Cpu
// facade that the remaining C++ modules (boards, ppu, sound, debugger,
// ines loader) still call. The C++ X6502 dispatch loop itself is gone.
// ---------------------------------------------------------------------------

// v1.3 Legion Phase 3: cycle accounting method on fceu11::Cpu (kept for
// the migrated DMR/DMW helpers below, matching the deleted C++ loop).
#define ADDCYC(x) g_cpu.add_cycles(x)

// Legacy direct-memory read/write (DMC DMA, trainer check, OAM readback).
// Charge one CPU cycle and fire the Lua memory hooks exactly like the
// deleted C++ dispatch loop's RdMem/WrMem did.
uint8 X6502_DMR(uint32 A)
{
 ADDCYC(1);
 _DB=fceu11::g_bus.read(static_cast<uint16_t>(A));
 #ifdef _S9XLUA_H
 CallRegisteredLuaMemHook(A, 1, _DB, LUAMEMHOOK_READ);
 #endif
 return(_DB);
}

void X6502_DMW(uint32 A, uint8 V)
{
 ADDCYC(1);
 fceu11::g_bus.write(static_cast<uint16_t>(A), V);
 #ifdef _S9XLUA_H
 CallRegisteredLuaMemHook(A, 1, V, LUAMEMHOOK_WRITE);
 #endif
 _DB = V;
}

// IRQ pin control: OR/AND the `IRQlow` blob bits. The Rust CPU
// re-reads that blob from the host at every dispatch boundary via the
// IRQ bridge (cpu/bus.rs `sync_irq_from_host`), so IRQs asserted
// between Rust calls are visible to the next dispatch — same semantics
// as the deleted C++ loop.
void X6502_IRQBegin(int w)
{
 _IRQlow|=w;
}

void X6502_IRQEnd(int w)
{
 _IRQlow&=~w;
}

FCEUX11_CACHE_ALIGN static uint8 CycTable[256] =
{
/*0x00*/ 7,6,2,8,3,3,5,5,3,2,2,2,4,4,6,6,
/*0x10*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x20*/ 6,6,2,8,3,3,5,5,4,2,2,2,4,4,6,6,
/*0x30*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x40*/ 6,6,2,8,3,3,5,5,3,2,2,2,3,4,6,6,
/*0x50*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x60*/ 6,6,2,8,3,3,5,5,4,2,2,2,5,4,6,6,
/*0x70*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0x80*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
/*0x90*/ 2,6,2,6,4,4,4,4,2,5,2,5,5,5,5,5,
/*0xA0*/ 2,6,2,6,3,3,3,3,2,2,2,2,4,4,4,4,
/*0xB0*/ 2,5,2,5,4,4,4,4,2,4,2,4,4,4,4,4,
/*0xC0*/ 2,6,2,8,3,3,5,5,2,2,2,2,4,4,6,6,
/*0xD0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
/*0xE0*/ 2,6,3,8,3,3,5,5,2,2,2,2,4,4,6,6,
/*0xF0*/ 2,5,2,8,4,4,6,6,2,4,2,7,4,4,7,7,
};

int X6502_GetOpcodeCycles( int op )
{
	return CycTable[op];
}

// E-1 probe (Phase 1 Step 1.3, 2026-08-02): env-gated absolute-cycle trace
// for NMI latch set / CPU sampling. Uses FCEUX11_E1_TRACE like the PPU-side
// probe so one env var drives the whole frame-boundary investigation.
static bool e1_cpu_trace_on() {
 static const bool on = []() {
  const char* e = std::getenv("FCEUX11_E1_TRACE");
  return e && e[0] == '1' && e[1] == '\0';
 }();
 return on;
}

// E-1 probe (Phase 1 Step 1.3): last instruction PC observed by the CPU at
// the current boundary, so the PPU VBL block can log exactly where the CPU
// instruction stream is when the frame boundary fires. Under the Rust CPU
// the probe is diagnostic-only (the Rust dispatch does not update it).
static uint16 e1_last_pc = 0;
uint16 fceu11_e1_last_pc() { return e1_last_pc; }

static uint64 e1_cpu_abs() {
 return g_cpu.timestamp_base() + (uint64)g_cpu.timestamp_ref();
}

// E-1 probe (Phase 1 Step 1.3, 2026-08-02): NMI-latch freshness flag.
// The VBL path (TriggerNMI) is called from the PPU loop between CPU runs;
// the CPU's next loop-top check happens at the boundary where the previous
// instruction ENDED (before the latch was asserted), so dispatching there
// would fire NMI one instruction too early. Per 6502 semantics the line is
// sampled at the END of each instruction; a latch asserted at/after boundary
// B must be observed at the NEXT boundary. TriggerNMI2 already defers via
// the NMI2->NMI conversion on the same check; give TriggerNMI the same
// one-boundary deferral (04-nmi_control #11 "after NEXT instruction").
static bool g_e1_nmi_fresh = false;

void TriggerNMI(void)
{
 // E-1 Track-B probe (v1.17 R5 task, 2026-08-08): enhanced NMI_LATCH
 // recorder at the CPU-side callee of the NMI dispatch path. The
 // existing E1 NMI_SET prints only the absolute timestamp; this E1B
 // NMI_LATCH_CALLEE adds the CPU budget residual (count, 1/16-dot
 // units), the freshly-latched IRQ-low byte, and the last PC the CPU
 // observed, all before _IRQlow|=FCEU_IQNMI mutates state. Distinct
 // probe name (E1B NMI_LATCH_CALLEE) so the caller-side E1B NMI_LATCH
 // in ppu_rendering.cpp and this CPU-side line can be cross-correlated
 // for the vbl_05/vbl_07 NMI dispatch-latency analysis.
 if (e1_cpu_trace_on())
  fprintf(stderr, "E1B NMI_LATCH_CALLEE abs=%llu (path=VBL) count=%d IRQlow_pre=0x%X lastpc=%04X\n",
   (unsigned long long)e1_cpu_abs(),
   (int)g_cpu.native_layout().count,
   (unsigned)_IRQlow,
   (unsigned)e1_last_pc);
 if (e1_cpu_trace_on())
  fprintf(stderr, "E1 NMI_SET abs=%llu (path=VBL)\n", (unsigned long long)e1_cpu_abs());
 _IRQlow|=FCEU_IQNMI;
 g_e1_nmi_fresh = true;
}

void TriggerNMI2(void)
{
 if (e1_cpu_trace_on())
  fprintf(stderr, "E1 NMI_SET2 abs=%llu (path=W2000-edge)\n", (unsigned long long)e1_cpu_abs());
 _IRQlow|=FCEU_IQNMI2;
}

// Phase 4 closeout: NMI-fresh flag accessors for the IRQ bridge. The
// Rust CPU reads/writes this flag through `kagami_bridge_*` so the C++
// one-instruction NMI deferral is honored at the same boundaries as the
// reference dispatch.
bool x6502_nmi_fresh_get(void) { return g_e1_nmi_fresh; }
void x6502_nmi_fresh_set(bool v) { g_e1_nmi_fresh = v; }

namespace fceu11 {

// v0.3.10 P4.1: definitions live in fceu11:: per plan v3 §5 v0.3.10;
// the global FCEUI_NMI / FCEUI_IRQ symbols are preserved via the inline
// reference aliases declared in core_api.h.
void NMI()
{
 _IRQlow|=FCEU_IQNMI;
}

void IRQ()
{
 _IRQlow|=FCEU_IQTEMP;
}

} // namespace fceu11

void FCEUI_GetIVectors(uint16 *reset, uint16 *irq, uint16 *nmi)
{
 fceuindbg=1;

 // Vector reads go through the same bus path + Lua mem hooks as the
 // deleted C++ dispatch loop's RdMem did (including the _DB latch).
 auto rd = [](unsigned int A) -> uint8 {
  uint8 v = fceu11::g_bus.read(static_cast<uint16_t>(A));
  _DB = v;
  #ifdef _S9XLUA_H
  CallRegisteredLuaMemHook(A, 1, v, LUAMEMHOOK_READ);
  #endif
  return v;
 };
 *reset=rd(0xFFFC);
 *reset|=rd(0xFFFD)<<8;
 *nmi=rd(0xFFFA);
 *nmi|=rd(0xFFFB)<<8;
 *irq=rd(0xFFFE);
 *irq|=rd(0xFFFF)<<8;
 fceuindbg=0;
}

//the opsize table is used to quickly grab the instruction sizes (in bytes)
FCEUX11_CACHE_ALIGN const uint8 opsize[256] = {
#ifdef BRK_3BYTE_HACK
/*0x00*/	3, //BRK
#else
/*0x00*/	1, //BRK
#endif
/*0x01*/      2,0,0,0,2,2,0,1,2,1,0,0,3,3,0,
/*0x10*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x20*/	3,2,0,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0x30*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x40*/	1,2,0,0,0,2,2,0,1,2,1,0,3,3,3,0,
/*0x50*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x60*/	1,2,0,0,0,2,2,0,1,2,1,0,3,3,3,0,
/*0x70*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0x80*/	0,2,0,0,2,2,2,0,1,0,1,0,3,3,3,0,
/*0x90*/	2,2,0,0,2,2,2,0,1,3,1,0,0,3,0,0,
/*0xA0*/	2,2,2,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0xB0*/	2,2,0,0,2,2,2,0,1,3,1,0,3,3,3,0,
/*0xC0*/	2,2,0,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0xD0*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0,
/*0xE0*/	2,2,0,0,2,2,2,0,1,2,1,0,3,3,3,0,
/*0xF0*/	2,2,0,0,0,2,2,0,1,3,0,0,0,3,3,0
};


//the optype table is a quick way to grab the addressing mode for any 6502 opcode
//
//  0 = Implied\Accumulator\Immediate\Branch\NULL
//  1 = (Indirect,X)
//  2 = Zero Page
//  3 = Absolute
//  4 = (Indirect),Y
//  5 = Zero Page,X
//  6 = Absolute,Y
//  7 = Absolute,X
//  8 = Zero Page,Y
//
FCEUX11_CACHE_ALIGN const uint8 optype[256] = {
/*0x00*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x10*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x20*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x30*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x40*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x50*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x60*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x70*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0x80*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0x90*/	0,4,0,3,5,5,8,8,0,6,0,6,7,7,6,6,
/*0xA0*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0xB0*/	0,4,0,3,5,5,8,8,0,6,0,6,7,7,6,6,
/*0xC0*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0xD0*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
/*0xE0*/	0,1,0,1,2,2,2,2,0,0,0,0,3,3,3,3,
/*0xF0*/	0,4,0,3,5,5,5,5,0,6,0,6,7,7,7,7,
};

// the opwrite table aids in predicting the value written for any 6502 opcode
//
//  0 = No value written
//  1 = Write from A
//  2 = Write from X
//  3 = Write from Y
//  4 = Write from P
//  5 = ASL (SLO)
//  6 = LSR (SRE)
//  7 = ROL (RLA)
//  8 = ROR (RRA)
//  9 = INC (ISC)
// 10 = DEC (DCP)
// 11 = (SAX)
// 12 = (AHX)
// 13 = (SHY)
// 14 = (SHX)
// 15 = (TAS)

FCEUX11_CACHE_ALIGN const uint8 opwrite[256] = {
/*0x00*/	 0, 0, 0, 5, 0, 0, 5, 5, 4, 0, 0, 0, 0, 0, 5, 5,
/*0x10*/	 0, 0, 0, 5, 0, 0, 5, 5, 0, 0, 0, 5, 0, 0, 5, 5,
/*0x20*/	 0, 0, 0, 7, 0, 0, 7, 7, 0, 0, 7, 0, 0, 0, 7, 7,
/*0x30*/	 0, 0, 0, 7, 0, 0, 7, 7, 0, 0, 0, 7, 0, 0, 7, 7,
/*0x40*/	 0, 0, 0, 6, 0, 0, 6, 6, 1, 0, 6, 0, 0, 0, 6, 6,
/*0x50*/	 0, 0, 0, 6, 0, 0, 6, 6, 0, 0, 0, 6, 0, 0, 6, 6,
/*0x60*/	 0, 0, 0, 8, 0, 0, 8, 8, 0, 0, 8, 0, 0, 0, 8, 8,
/*0x70*/	 0, 0, 0, 8, 0, 0, 8, 8, 0, 0, 0, 8, 0, 0, 8, 8,
/*0x80*/	 0, 1, 0,11, 3, 1, 2,11, 0, 0, 0, 0, 3, 1, 2,11,
/*0x90*/	 0, 1, 0,12, 3, 1, 2,11, 0, 1, 0,15,13, 1,14,12,
/*0xA0*/	 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
/*0xB0*/	 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
/*0xC0*/	 0, 0, 0,10, 0, 0,10,10, 0, 0, 0, 0, 0, 0,10,10,
/*0xD0*/	 0, 0, 0,10, 0, 0,10,10, 0, 0, 0,10, 0, 0,10,10,
/*0xE0*/	 0, 0, 0, 9, 0, 0, 9, 9, 0, 0, 0, 0, 0, 0, 9, 9,
/*0xF0*/	 0, 0, 0, 9, 0, 0, 9, 9, 0, 0, 0, 9, 0, 0, 9, 9,
};
