// FCEUX11 — v1.4 Gateway: Memory dispatch + bank-switching bus
//
// Phase 3: Bus class OWNS all the data (ARead[] / BWrite[] / Page[]
// / VPage[] / VPageG[] / MMC5SPRVPage[] / MMC5BGVPage[] / PRGptr[] /
// CHRptr[] / PRGram[] / CHRram[] / PRGsize[] / CHRsize[] / PRGmask*[]
// / CHRmask*[] / PRGIsRAM[] / mirror_hard_). The legacy global names
// are `extern` reference-to-array aliases pointing into g_bus;
// existing call sites continue to compile unchanged.
//
// g_bus is a direct global object (replaces the Phase 2 Meyers
// singleton `bus_instance()`) so the compiler folds every hot-path
// `g_bus.aread_[addr]` / `g_bus.bwrite_[addr]` to a direct
// array-index + indirect-call sequence, identical to v1.3.0's
// `::ARead[addr](addr)` machine code.
//
// v1.4 Roadmap §4.1 inline-forwarder pattern (mirrors v1.3 Cpu/g_cpu).
// The hot-path `read()` / `write()` are `__forceinline` against
// this->aread_ / this->bwrite_; same machine code as the v1.3.0
// legacy global-indexing path.

#ifndef FCEU11_BUS_H
#define FCEU11_BUS_H

#include <cstdint>
#include <cstddef>

#include "types.h"             // readfunc / writefunc typedef
#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN, fceu11::kCacheLineSize

// Forward declarations to keep this header free of cart.h / fceu.h /
// ppu.h (which would pull in CartInfo, FCEUS, PPU, etc. for every
// translation unit that includes bus.h).
struct CartInfo;

namespace fceu11 {
// Forward-declare so Bus::attach_ppu(fceu11::Ppu*) can be declared in
// the public section without pulling in ppu_class.h (which would
// create a circular include).
class Ppu;
} // namespace fceu11

namespace fceu11 {

class FCEUX11_CACHE_ALIGN Bus {
public:
    // -----------------------------------------------------------------
    // Lifecycle. init() does the ResetCartMapping-equivalent reset:
    // zero Page[], init VPage[] with the `nothing[]` open-bus handler,
    // etc. mirror_hard_ resets to 0. Called once at process start
    // (and again on each game load via ResetCartMapping).
    // -----------------------------------------------------------------
    Bus() noexcept;
    void init() noexcept;
    void shutdown() noexcept;
    void reset_mapping() noexcept;   // cart.cpp::ResetCartMapping equivalent

    // -----------------------------------------------------------------
    // v1.5 Prism §3.2 / §3.3: Ppu injection point. The canonical
    // attach site is fceu.cpp::Initialize() right after g_bus.init()
    // (fceu.cpp:1063-1065). After attach_ppu runs, ppu_ is guaranteed
    // non-null for the lifetime of the Bus; the v1.5.1 cleanup
    // removed the pre-attachment fallback branches from setchr* /
    // setmirror* / setntamem (plan §10.6 release-readiness).
    // -----------------------------------------------------------------
    void attach_ppu(fceu11::Ppu* p) noexcept { ppu_ = p; }

    // -----------------------------------------------------------------
    // Hot-path 64K CPU address-space read / write dispatch.
    // `__forceinline` so the compiler emits the same indirect-call
    // machine code as the v1.3.0 `ARead[addr](addr)` path. The
    // aread_/bwrite_ arrays are 64-byte aligned, matching v0.3.12.5
    // cache-line layout (see cart.cpp:254-255 pre-Phase-2).
    // -----------------------------------------------------------------
    __forceinline uint8_t read(uint16_t addr) const noexcept {
        return aread_[addr](addr);
    }
    __forceinline void write(uint16_t addr, uint8_t val) const noexcept {
        bwrite_[addr](addr, val);
    }

    // Direct memory access (debugger / DMR / DMW). Routes through
    // the same Page[] table as the regular read path but skips the
    // aread_[] indirect call.
    uint8_t direct_read(uint32_t addr) const noexcept;
    void    direct_write(uint32_t addr, uint8_t val) noexcept;

    // -----------------------------------------------------------------
    // Handler registration. No Genie wrapping here — that stays in
    // fceu.cpp::SetReadHandler / SetWriteHandler (which write through
    // ::ARead / ::BWrite aliases into this->aread_ / this->bwrite_).
    // -----------------------------------------------------------------
    void set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept;
    void set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept;

    // -----------------------------------------------------------------
    // CPU / PPU address-space page tables. Inline in the class body
    // so the compiler folds `bus_instance().page()` to the known
    // address of the singleton's page_[] member — same machine code
    // as the v1.3.0 `::Page[i]` direct global access.
    // -----------------------------------------------------------------
    __forceinline uint8_t* (& page()  noexcept)[32] { return page_; }
    __forceinline uint8_t* (& vpage() noexcept)[8]  { return vpage_; }

    void set_page(uint32_t idx, uint8_t* ptr) noexcept;
    void set_vpage(uint32_t idx, uint8_t* ptr) noexcept;

    // -----------------------------------------------------------------
    // Bank-switching API. Real implementations (no longer delegating
    // to cart.cpp free functions, which are gone in Phase 2). The
    // 9 functions in v1.4 plan §3.2. The non-listed variants
    // (setprg2, setprg2r, setprg4, setchr2, setchr2r, etc.) stay in
    // cart.cpp as legacy free functions because no board file in the
    // v1.4 batch assignment uses them, and they delegate to the same
    // Page[] / VPage[] tables anyway.
    // -----------------------------------------------------------------
    void setprg8 (uint32_t addr, uint32_t bank) noexcept;
    void setprg16(uint32_t addr, uint32_t bank) noexcept;
    void setprg32(uint32_t addr, uint32_t bank) noexcept;
    void setchr1 (uint32_t addr, uint32_t bank) noexcept;
    void setchr4 (uint32_t addr, uint32_t bank) noexcept;
    void setchr8 (uint32_t bank) noexcept;
    void setmirror (uint32_t m) noexcept;
    void setmirrorw(uint32_t a, uint32_t b, uint32_t c, uint32_t d) noexcept;
    void setntamem(uint8_t* p, int ram, uint32_t addr) noexcept;

    // -----------------------------------------------------------------
    // ROM pointer tables + bank-mask setup. Inline in the class
    // body (see comment on page()/vpage() above).
    // -----------------------------------------------------------------
    __forceinline uint8_t* (& prg_ptr() noexcept)[32] { return prg_ptr_; }
    __forceinline uint8_t* (& chr_ptr() noexcept)[32] { return chr_ptr_; }

    // Mask / size / RAM-flag tables (the cart.cpp globals that board
    // files index directly: PRGsize[chip], CHRmask1[chip], etc.).
    __forceinline uint32_t (& prg_size() noexcept)[32]   { return prg_size_; }
    __forceinline uint32_t (& chr_size() noexcept)[32]   { return chr_size_; }
    __forceinline uint32_t (& prg_mask2() noexcept)[32] { return prg_mask2_;  }
    __forceinline uint32_t (& prg_mask4() noexcept)[32] { return prg_mask4_;  }
    __forceinline uint32_t (& prg_mask8() noexcept)[32] { return prg_mask8_;  }
    __forceinline uint32_t (& prg_mask16() noexcept)[32] { return prg_mask16_; }
    __forceinline uint32_t (& prg_mask32() noexcept)[32] { return prg_mask32_; }
    __forceinline uint32_t (& chr_mask1() noexcept)[32] { return chr_mask1_;  }
    __forceinline uint32_t (& chr_mask2() noexcept)[32] { return chr_mask2_;  }
    __forceinline uint32_t (& chr_mask4() noexcept)[32] { return chr_mask4_;  }
    __forceinline uint32_t (& chr_mask8() noexcept)[32] { return chr_mask8_;  }
    __forceinline uint8_t  (& prg_ram()  noexcept)[32]   { return prg_ram_;  }
    __forceinline uint8_t  (& chr_ram()  noexcept)[32]   { return chr_ram_;  }
    __forceinline uint8_t  (& prg_is_ram() noexcept)[32]  { return prg_is_ram_; }

    // Other special page tables (Genie overlay, MMC5 split banks).
    __forceinline uint8_t* (& vpage_g()         noexcept)[8] { return vpage_g_; }
    __forceinline uint8_t* (& mmc5_spr_vpage()  noexcept)[8] { return mmc5_spr_vpage_; }
    __forceinline uint8_t* (& mmc5_bg_vpage()   noexcept)[8] { return mmc5_bg_vpage_; }

    void setup_prg_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept;
    void setup_chr_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept;
    void setup_mirroring(int m, int hard, uint8_t* extra) noexcept;

    // -----------------------------------------------------------------
    // Internal dispatch-table access. Returns references to the
    // private members; this is what the global `::ARead` / `::BWrite`
    // aliases bind to. Inline in the class body so the compiler can
    // constant-fold `bus_instance().aread_table()` to the singleton's
    // known address — see comment on page()/vpage() above.
    // -----------------------------------------------------------------
    readfunc  (& aread_table()  noexcept)[0x10000]       { return aread_; }
    writefunc (& bwrite_table() noexcept)[0x10000]       { return bwrite_; }
    const readfunc  (& aread_table()  const noexcept)[0x10000] { return aread_; }
    const writefunc (& bwrite_table() const noexcept)[0x10000] { return bwrite_; }

    // -----------------------------------------------------------------
    // Genie shadow state (read/write wrap of upper 32K). The actual
    // AReadG / BWriteG buffers are 32K each and live on the heap
    // (allocated by AllocGenieRW). Only the pointer / wrap flag is
    // here. Kept on Bus so the state travels with the bus instance
    // even after a future multi-Bus world (v1.7 Cart). For Phase 2,
    // fceu.cpp::SetReadHandler / SetWriteHandler / GetReadHandler /
    // GetWriteHandler still own the Genie-aware code path and call
    // these accessors.
    // -----------------------------------------------------------------
    readfunc*  genie_read_shadow() noexcept       { return genie_a_read_; }
    writefunc* genie_write_shadow() noexcept      { return genie_b_write_; }
    const readfunc*  genie_read_shadow() const noexcept  { return genie_a_read_; }
    const writefunc* genie_write_shadow() const noexcept { return genie_b_write_; }
    int& genie_wrap() noexcept                    { return genie_r_wrap_; }

    // Mirror-mode hard flag (cart.cpp::mirrorhard).
    int& mirror_hard() noexcept { return mirror_hard_; }

private:
    // -----------------------------------------------------------------
    // Storage. All 64-byte-aligned hot arrays (aread_ / bwrite_) lead;
    // the rest are POD arrays sized to match the v1.3.0 global
    // declarations. See bus.cpp for the static_assert on class size
    // and alignment.
    // -----------------------------------------------------------------
    alignas(64) readfunc  aread_[0x10000];
    alignas(64) writefunc bwrite_[0x10000];
    uint8_t*  page_[32];
    uint8_t*  vpage_[8];
    uint8_t*  vpage_g_[8];        // Genie overlay
    uint8_t*  mmc5_spr_vpage_[8];
    uint8_t*  mmc5_bg_vpage_[8];
    uint8_t*  prg_ptr_[32];
    uint8_t*  chr_ptr_[32];
    uint32_t  prg_size_[32];
    uint32_t  chr_size_[32];
    uint32_t  prg_mask2_[32];
    uint32_t  prg_mask4_[32];
    uint32_t  prg_mask8_[32];
    uint32_t  prg_mask16_[32];
    uint32_t  prg_mask32_[32];
    uint32_t  chr_mask1_[32];
    uint32_t  chr_mask2_[32];
    uint32_t  chr_mask4_[32];
    uint32_t  chr_mask8_[32];
    uint8_t   prg_is_ram_[32];
    uint8_t   prg_ram_[32];
    uint8_t   chr_ram_[32];
    int       mirror_hard_ = 0;

    // Genie state (32K upper address space read/write shadow).
    readfunc*  genie_a_read_  = nullptr;
    writefunc* genie_b_write_ = nullptr;
    int        genie_r_wrap_  = 0;

    // v1.5 Prism §3.2: Ppu back-pointer injected by attach_ppu().
    // Null until fceu.cpp::Initialize() calls g_bus.attach_ppu(&g_ppu),
    // then guaranteed non-null for the rest of process lifetime.
    // Bus methods (setchr* / setmirror* / setntamem) dereference
    // ppu_ unconditionally — the v1.5.1 hotfix removed the
    // pre-attachment fallback branches (plan §10.6).
    Ppu* ppu_ = nullptr;
};

// Direct global instance (v1.4 Phase 3 §5.1.1). Replaces the
// Phase 2 Meyers singleton `bus_instance()` because a non-inline
// function-call to resolve the singleton address at every consumer
// TU costs ~5% on bench_full_frame. `g_bus` is a real global
// object defined in bus.cpp; the linker gives it a fixed address,
// so `g_bus.read(addr)` / `g_bus.aread_[addr]` compiles to a
// direct array-index + indirect-call sequence identical to
// v1.3.0's `::ARead[addr](addr)` machine code.
extern Bus g_bus;

} // namespace fceu11

// ---------------------------------------------------------------------------
// Global reference aliases (v1.4 plan §4.1 pattern). Each alias is
// a real `extern` reference-to-array global; the canonical
// initializer (binding to g_bus.member_) lives in bus.cpp
// and runs once during static init (g_bus is declared first in
// bus.cpp, so the references below bind to its already-constructed
// members). Any code that does `ARead[x] = func` writes through to
// g_bus.aread_[x] — the data lives in exactly one place (g_bus),
// with these names as thin references for call-site compat.
//
// Why `extern` (not `inline`): when an inline alias's initializer
// would call a non-inline function (the Phase 2 `bus_instance()`),
// the compiler at every consumer TU emits a real function call to
// resolve the table base — that cost ~5% on bench_full_frame in
// Phase 2 pre-optimization. By moving the initialization to bus.cpp
// (one TU) and using `extern` aliases in the header, every other TU
// sees ARead / BWrite / etc. as a regular global reference — the
// compiler knows the address is fixed at link time and emits a
// direct array-index + indirect-call sequence identical to v1.3.0's
// `::ARead[i](addr)` machine code.
// ---------------------------------------------------------------------------
extern readfunc  (& ARead )[0x10000];
extern writefunc (& BWrite)[0x10000];

extern uint8_t* (& Page        )[32];
extern uint8_t* (& VPage       )[8];
extern uint8_t* (& VPageG      )[8];
extern uint8_t* (& MMC5SPRVPage)[8];
extern uint8_t* (& MMC5BGVPage )[8];

extern uint8_t* (& PRGptr)[32];
extern uint8_t* (& CHRptr)[32];

extern uint32_t (& PRGsize  )[32];
extern uint32_t (& CHRsize  )[32];
extern uint32_t (& PRGmask2 )[32];
extern uint32_t (& PRGmask4 )[32];
extern uint32_t (& PRGmask8 )[32];
extern uint32_t (& PRGmask16)[32];
extern uint32_t (& PRGmask32)[32];
extern uint32_t (& CHRmask1 )[32];
extern uint32_t (& CHRmask2 )[32];
extern uint32_t (& CHRmask4 )[32];
extern uint32_t (& CHRmask8 )[32];
extern uint8_t  (& PRGram   )[32];
extern uint8_t  (& CHRram   )[32];
extern uint8_t  (& PRGIsRAM )[32];

// ---------------------------------------------------------------------------
// Inline forwarders for the 9 free functions in v1.4 plan §3.2 / §4.1.
// All call sites that used `setprg8(...)` etc. now route through
// g_bus.method_ without an extra function-call frame — the
// compiler folds the member-function call into the same machine
// code as the v1.3.0 free-function path (g_bus is a real global,
// so its address is known at link time).
// ---------------------------------------------------------------------------
inline void setprg8 (uint32_t A, uint32_t V) noexcept { fceu11::g_bus.setprg8 (A, V); }
inline void setprg16(uint32_t A, uint32_t V) noexcept { fceu11::g_bus.setprg16(A, V); }
inline void setprg32(uint32_t A, uint32_t V) noexcept { fceu11::g_bus.setprg32(A, V); }
inline void setchr1 (uint32_t A, uint32_t V) noexcept { fceu11::g_bus.setchr1 (A, V); }
inline void setchr4 (uint32_t A, uint32_t V) noexcept { fceu11::g_bus.setchr4 (A, V); }
inline void setchr8 (uint32_t V)          noexcept { fceu11::g_bus.setchr8 (V); }
// v1.4 Post-Release Optimization Plan §2.4 — forwarder params
// match Bus::setmirror(uint32_t) / Bus::setmirrorw(uint32_t, ...).
// A grep of src/boards/ for setmirror( / setmirrorw( shows all
// callers pass either MI_0..MI_3 macros (uint32 literals) or
// uint8 / int local variables — uint8→uint32_t and int→uint32_t
// implicit conversions are well-defined, so no cast is needed
// at the call site.
inline void setmirror (uint32_t t)              noexcept { fceu11::g_bus.setmirror (t); }
inline void setmirrorw(uint32_t a, uint32_t b, uint32_t c, uint32_t d) noexcept {
    fceu11::g_bus.setmirrorw(a, b, c, d);
}
inline void setntamem(uint8_t* p, int ram, uint32_t b) noexcept {
    fceu11::g_bus.setntamem(p, ram, b);
}

// Cart setup / reset (replaces the cart.cpp::SetupCart* and
// ResetCartMapping free functions). All board files that called
// SetupCartPRGMapping(...) now route through g_bus.
inline void SetupCartPRGMapping(int chip, uint8_t* p, uint32_t size, int ram) noexcept {
    fceu11::g_bus.setup_prg_mapping(static_cast<uint32_t>(chip), p, size, ram);
}
inline void SetupCartCHRMapping(int chip, uint8_t* p, uint32_t size, int ram) noexcept {
    fceu11::g_bus.setup_chr_mapping(static_cast<uint32_t>(chip), p, size, ram);
}
inline void SetupCartMirroring(int m, int hard, uint8_t* extra) noexcept {
    fceu11::g_bus.setup_mirroring(m, hard, extra);
}
inline void ResetCartMapping() noexcept {
    fceu11::g_bus.reset_mapping();
}

// VPageR (uint8** pointer alias for VPage[0]) is declared separately
// in bus.cpp because inline pointer variables need their initializer
// to be a constant expression, and `&bus_instance().vpage_[0]` is
// only constant after bus_instance() is constructed. bus.cpp does:
//   uint8_t** VPageR = bus_instance().vpage_r_ptr();
// and the header just forward-declares it (VPageR is used widely in
// datalatch.cpp, mmc5.cpp, cart.cpp, so it must remain a global).
extern uint8_t** VPageR;

#endif // FCEU11_BUS_H
