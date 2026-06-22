// FCEUX11 — v1.4 Gateway: Memory dispatch + bank-switching bus skeleton
//
// Phase 1 introduces fceu11::Bus, the single owner (in later phases) of
// the CPU/PPU address-space dispatch tables (ARead[] / BWrite[] /
// Page[] / VPage[] / PRGptr[] / CHRptr[]) and the bank-switching API
// (setprg*/setchr*/setmirror*/setntamem / SetReadHandler /
// SetWriteHandler). v1.4 Roadmap §4; build plan §3.
//
// In Phase 1 the class is a *facade*: every method delegates to the
// existing free function in cart.cpp / fceu.cpp, or returns a
// reference to the existing global array. The data is still owned by
// the legacy globals. Phase 2 (per plan §4) will rewire the legacy
// globals to be inline reference aliases into `bus_instance()`'s
// internal arrays, and the method bodies will switch to writing
// `this->page_` etc. directly. Until then, the build is
// byte-equivalent to v1.3.0.
//
// The hot-path `read()` / `write()` methods (per plan §3.2) are
// declared but stubbed in Phase 1 — they will be wired against
// `this->aread_` / `this->bwrite_` in Phase 2 when those arrays are
// adopted. Calling them in Phase 1 is a no-op that returns 0 / does
// nothing, with a comment so it doesn't get confused with the
// eventual implementation.

#ifndef FCEU11_BUS_H
#define FCEU11_BUS_H

#include <cstdint>

#include "types.h"             // readfunc / writefunc typedef
#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN

// Forward declarations to match the surface in core_state.h / fceu.h /
// cart.h. The Bus methods either return references to these globals
// (Phase 1) or to its own private storage (Phase 2+).
struct CartInfo;
struct iNES_HEADER;

namespace fceu11 {

class FCEUX11_CACHE_ALIGN Bus {
public:
    // -----------------------------------------------------------------
    // Lifecycle (Phase 2+ will move real init logic here; Phase 1
    // is a no-op so cart.cpp / fceu.cpp keep owning the boot path).
    // -----------------------------------------------------------------
    Bus() noexcept;
    void init() noexcept;
    void shutdown() noexcept;

    // -----------------------------------------------------------------
    // 64K CPU address-space read / write dispatch (HOT PATH).
    //
    // Phase 1 STUB: returns 0 / does nothing. The legacy call sites
    // still go through the global ARead[] / BWrite[] arrays directly.
    // Phase 2 will route these to this->aread_[addr](addr) /
    // this->bwrite_[addr](addr, val) once the arrays are owned by
    // Bus. The __forceinline attribute is kept now so Phase 2's
    // implementation is inlinable in the same call sites without a
    // signature change.
    // -----------------------------------------------------------------
    __forceinline uint8_t read(uint16_t addr) const noexcept;
    __forceinline void    write(uint16_t addr, uint8_t val) const noexcept;

    // Direct memory access (debugger / DMR / DMW). Phase 1 STUB.
    uint8_t direct_read(uint32_t addr) const noexcept;
    void    direct_write(uint32_t addr, uint8_t val) noexcept;

    // -----------------------------------------------------------------
    // Handler registration (Phase 1: delegates to ::SetReadHandler /
    // ::SetWriteHandler in fceu.cpp).
    // -----------------------------------------------------------------
    void set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept;
    void set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept;

    // -----------------------------------------------------------------
    // CPU / PPU address-space page tables.
    // Phase 1: returns a reference to the existing global array.
    // Definitions are out-of-line in bus.cpp so this header does not
    // need to include cart.h (which would otherwise pull in CartInfo,
    // iNES_HEADER, and the full setprg*/setchr* surface for every
    // translation unit that includes bus.h).
    // -----------------------------------------------------------------
    uint8_t* (& page()  noexcept)[32];
    uint8_t* (& vpage() noexcept)[8];

    void set_page(uint32_t idx, uint8_t* ptr) noexcept;
    void set_vpage(uint32_t idx, uint8_t* ptr) noexcept;

    // -----------------------------------------------------------------
    // Bank-switching API (Phase 1: delegates to the free functions in
    // cart.cpp; signatures match the existing decls in cart.h).
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
    // ROM pointer tables. Phase 1: returns reference to the existing
    // global arrays in cart.cpp. Out-of-line definitions in bus.cpp.
    // -----------------------------------------------------------------
    uint8_t* (& prg_ptr() noexcept)[32];
    uint8_t* (& chr_ptr() noexcept)[32];

    // Phase 1: delegates to cart.cpp::SetupCartPRGMapping /
    // cart.cpp::SetupCartCHRMapping. Phase 2+ will own the mask
    // tables and the bank-swap paths internally.
    void setup_prg_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept;
    void setup_chr_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept;

    // -----------------------------------------------------------------
    // Internal dispatch-table access. Phase 1: returns a reference to
    // the existing global arrays in fceu.cpp. Phase 2 will return
    // references to this->aread_ / this->bwrite_ once the arrays
    // migrate. Out-of-line definitions in bus.cpp.
    // -----------------------------------------------------------------
    readfunc  (& aread_table()  noexcept)[0x10000];
    writefunc (& bwrite_table() noexcept)[0x10000];
    const readfunc  (& aread_table()  const noexcept)[0x10000];
    const writefunc (& bwrite_table() const noexcept)[0x10000];

private:
    // Reserved storage for Phase 2. Empty for now — the class size
    // assertion in bus.cpp confirms the layout is stable as the data
    // arrays are adopted.
    //
    // Phase 2 will add:
    //   alignas(64) readfunc  aread_[0x10000];
    //   alignas(64) writefunc bwrite_[0x10000];
    //   uint8_t* page_[32];
    //   uint8_t* vpage_[8];
    //   uint8_t* prg_ptr_[32];
    //   uint8_t* chr_ptr_[32];
    //   uint32_t prg_size_[32];
    //   uint32_t chr_size_[32];
    //   uint32_t prg_mask*_[32];
    //   uint32_t chr_mask*_[32];
    uint8_t  reserved_[1] = {0};
};

// Meyers singleton: thread-safe lazy init per C++11 [stmt.dcl] p4.
// Returns the process-wide Bus instance.
Bus& bus_instance() noexcept;

} // namespace fceu11

// Convenience alias for new call sites (mirrors g_cpu pattern from
// v1.3 Legion, plan §3.2 last code block).
inline auto& g_bus = fceu11::bus_instance();

#endif // FCEU11_BUS_H
