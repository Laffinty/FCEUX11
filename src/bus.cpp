// FCEUX11 — v1.4 Gateway: Memory dispatch + bank-switching bus skeleton
//
// Phase 1 implementation. Every method is either a no-op stub (for
// the hot-path read/write that will be wired in Phase 2 against
// this->aread_ / this->bwrite_) or a thin delegation to the existing
// free functions in cart.cpp / fceu.cpp. The result is byte-equivalent
// to v1.3.0 behaviour: every legacy call site still resolves to the
// same code through the same globals.

#include "bus.h"

#include "fceu.h"   // ::ARead, ::BWrite, ::SetReadHandler, ::SetWriteHandler
#include "cart.h"   // ::Page, ::VPage, ::PRGptr, ::CHRptr, ::setprg*, ::setchr*,
                    // ::setmirror, ::setmirrorw, ::setntamem, ::SetupCart*Mapping

namespace fceu11 {

// ---------------------------------------------------------------------------
// Class layout — reserved storage. The class currently holds a single
// padding byte so the alignment of the class is at least 1; once
// Phase 2 migrates the global arrays in, those become private members
// and the static_assert below continues to compile.
//
// alignas(64) on the class is what allows Bus to live in a single
// cache line and for the future aread_[0x10000] / bwrite_[0x10000]
// to start on a fresh cache line. The 64-byte alignment must NOT
// regress; if a future change drops it, this static_assert fires.
// ---------------------------------------------------------------------------
static_assert(alignof(Bus) >= 64,
              "Bus must remain 64-byte aligned for cache locality");

// ---------------------------------------------------------------------------
// Lifecycle — Phase 1: no-op. Phase 2 will move ResetCartMapping-style
// initialisation here.
// ---------------------------------------------------------------------------
Bus::Bus() noexcept = default;
void Bus::init() noexcept {}
void Bus::shutdown() noexcept {}

// ---------------------------------------------------------------------------
// Hot-path read / write (PHASE 1 STUB).
//
// The real implementation per plan §3.2 is `return aread_[addr](addr);`
// and `bwrite_[addr](addr, val);`. The internal arrays `aread_` /
// `bwrite_` will be added in Phase 2 when the data migrates. Until
// then, calling Bus::read / Bus::write returns 0 / does nothing —
// no existing call site invokes these methods, so this is harmless.
// ---------------------------------------------------------------------------
__forceinline uint8_t Bus::read(uint16_t addr) const noexcept {
    (void)addr;
    return 0;  // Phase 2: return aread_[addr](addr);
}

__forceinline void Bus::write(uint16_t addr, uint8_t val) const noexcept {
    (void)addr;
    (void)val;
    // Phase 2: bwrite_[addr](addr, val);
}

// ---------------------------------------------------------------------------
// Direct memory access (debugger / DMR / DMW). Phase 1: stub. Phase 2:
// routes through the cart.cpp::CartBR / CartBW / CartBROB path using
// the Bus-owned Page[] table.
// ---------------------------------------------------------------------------
uint8_t Bus::direct_read(uint32_t addr) const noexcept {
    (void)addr;
    return 0;  // Phase 2: Page[addr >> 11][addr]
}

void Bus::direct_write(uint32_t addr, uint8_t val) noexcept {
    (void)addr;
    (void)val;
    // Phase 2: Page[addr >> 11][addr] = val
}

// ---------------------------------------------------------------------------
// Handler registration — Phase 1: delegate to the existing
// fceu.cpp::SetReadHandler / SetWriteHandler (which manipulate the
// global ARead[] / BWrite[] arrays).
// ---------------------------------------------------------------------------
void Bus::set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept {
    ::SetReadHandler(static_cast<int32_t>(start),
                     static_cast<int32_t>(end),
                     fn);
}

void Bus::set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept {
    ::SetWriteHandler(static_cast<int32_t>(start),
                      static_cast<int32_t>(end),
                      fn);
}

// ---------------------------------------------------------------------------
// Page-table setters — Phase 1: write to the existing global arrays
// in cart.cpp. Same semantics as direct access; the wrapping is here
// so Phase 2 can flip the body to write to this->page_ / this->vpage_
// without changing any call site.
// ---------------------------------------------------------------------------

// Out-of-line definitions for the page-table / ROM-pointer / dispatch-
// table accessors declared in bus.h. They are out-of-line so bus.h
// does not have to include cart.h / fceu.h (which would pull in the
// full CartInfo / setprg* / setchr* / FCEUS surface for every
// translation unit that includes bus.h).
uint8_t* (& Bus::page()  noexcept)[32] { return ::Page; }
uint8_t* (& Bus::vpage() noexcept)[8]  { return ::VPage; }

uint8_t* (& Bus::prg_ptr() noexcept)[32] { return ::PRGptr; }
uint8_t* (& Bus::chr_ptr() noexcept)[32] { return ::CHRptr; }

readfunc  (& Bus::aread_table()  noexcept)[0x10000]       { return ::ARead; }
writefunc (& Bus::bwrite_table() noexcept)[0x10000]       { return ::BWrite; }
const readfunc  (& Bus::aread_table()  const noexcept)[0x10000] { return ::ARead; }
const writefunc (& Bus::bwrite_table() const noexcept)[0x10000] { return ::BWrite; }

void Bus::set_page(uint32_t idx, uint8_t* ptr) noexcept {
    ::Page[idx] = ptr;
}

void Bus::set_vpage(uint32_t idx, uint8_t* ptr) noexcept {
    ::VPage[idx] = ptr;
}

// ---------------------------------------------------------------------------
// Bank-switching API — Phase 1: delegate to the free functions in
// cart.cpp (setprg8/16/32, setchr1/4/8, setmirror, setmirrorw,
// setntamem). Signatures match the cart.h declarations exactly.
// ---------------------------------------------------------------------------
void Bus::setprg8(uint32_t addr, uint32_t bank) noexcept {
    ::setprg8(addr, bank);
}

void Bus::setprg16(uint32_t addr, uint32_t bank) noexcept {
    ::setprg16(addr, bank);
}

void Bus::setprg32(uint32_t addr, uint32_t bank) noexcept {
    ::setprg32(addr, bank);
}

void Bus::setchr1(uint32_t addr, uint32_t bank) noexcept {
    ::setchr1(addr, bank);
}

void Bus::setchr4(uint32_t addr, uint32_t bank) noexcept {
    ::setchr4(addr, bank);
}

void Bus::setchr8(uint32_t bank) noexcept {
    ::setchr8(bank);
}

void Bus::setmirror(uint32_t m) noexcept {
    ::setmirror(static_cast<int>(m));
}

void Bus::setmirrorw(uint32_t a, uint32_t b, uint32_t c, uint32_t d) noexcept {
    ::setmirrorw(static_cast<int>(a),
                 static_cast<int>(b),
                 static_cast<int>(c),
                 static_cast<int>(d));
}

void Bus::setntamem(uint8_t* p, int ram, uint32_t addr) noexcept {
    ::setntamem(p, ram, addr);
}

// ---------------------------------------------------------------------------
// ROM pointer-table setup — Phase 1: delegate to cart.cpp.
// ---------------------------------------------------------------------------
void Bus::setup_prg_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept {
    ::SetupCartPRGMapping(static_cast<int>(chip), p, size, ram);
}

void Bus::setup_chr_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept {
    ::SetupCartCHRMapping(static_cast<int>(chip), p, size, ram);
}

// ---------------------------------------------------------------------------
// Singleton — Meyers pattern. The single Bus instance lives in a
// function-local static; first call initializes, subsequent calls
// return the same reference. C++11 guarantees thread-safe init.
// ---------------------------------------------------------------------------
Bus& bus_instance() noexcept {
    static Bus instance;
    return instance;
}

} // namespace fceu11
