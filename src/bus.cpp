// FCEUX11 — v1.4 Gateway: Memory dispatch + bank-switching bus
//
// Phase 3 implementation. Bus class OWNS all the dispatch / page /
// ROM pointer / mask / ram-flag storage. Method bodies do the
// work directly on this->state_. The cart.cpp and fceu.cpp
// global arrays for ARead / BWrite / Page / VPage / etc. are gone;
// the legacy `::ARead` / `::Page` / etc. names are `extern`
// reference-to-array aliases defined in bus.h that bind to
// g_bus's internal storage.
//
// `VPageR` is a uint8** pointer alias for `&g_bus.vpage_[0]`; its
// definition lives here so it can take the address of g_bus's
// member at static init (C++ inline pointer variables must have
// constant initializers, and `&g_bus.vpage_[0]` is not constexpr).

#include "bus.h"

#include <cstring>  // memset

#include "fceu.h"   // ::ANull, ::BNull (DECLFR/DECLFW expansion)
#include "ppu.h"    // ::PPUCHRRAM, ::PPUNTARAM, ::vnapage, ::NTARAM
#include "x6502.h"  // g_cpu (for ::ANull's return value)

namespace fceu11 {

// ---------------------------------------------------------------------------
// Class layout assertions.
// ---------------------------------------------------------------------------
static_assert(alignof(Bus) >= 64,
              "Bus must remain 64-byte aligned for cache locality");
static_assert(sizeof(Bus) >= (2 * 0x10000 * sizeof(void*)),
              "Bus must hold ARead[] + BWrite[] tables (>= 128 KB on 64-bit)");

// ---------------------------------------------------------------------------
// Open-bus handler (replaces cart.cpp::nothing[8192] + the page-init
// loop from ResetCartMapping). Each "Page" entry is a pointer into
// the static `nothing` buffer, with a page-base offset so that
// `Page[x][y]` indexes the right 2 KiB "open bus" value.
// ---------------------------------------------------------------------------
namespace {
    uint8_t nothing[8192];

    // Inline handler for reads on unmapped ranges: returns the CPU
    // data-bus (X6502::DB) — matches the v1.3.0 fceu.cpp::ANull.
    uint8_t ANullImpl(uint32_t) {
        return g_cpu.native_layout().DB;
    }
    void BNullImpl(uint32_t, uint8_t) {}
} // namespace

// ---------------------------------------------------------------------------
// ctor: zero-init everything except the Genie shadow pointers (those
// stay nullptr until AllocGenieRW mallocs them).
// ---------------------------------------------------------------------------
Bus::Bus() noexcept {
    std::memset(aread_,  0, sizeof(aread_));
    std::memset(bwrite_, 0, sizeof(bwrite_));
    std::memset(page_,          0, sizeof(page_));
    std::memset(vpage_,         0, sizeof(vpage_));
    std::memset(vpage_g_,       0, sizeof(vpage_g_));
    std::memset(mmc5_spr_vpage_, 0, sizeof(mmc5_spr_vpage_));
    std::memset(mmc5_bg_vpage_,  0, sizeof(mmc5_bg_vpage_));
    std::memset(prg_ptr_,       0, sizeof(prg_ptr_));
    std::memset(chr_ptr_,       0, sizeof(chr_ptr_));
    std::memset(prg_size_,      0, sizeof(prg_size_));
    std::memset(chr_size_,      0, sizeof(chr_size_));
    std::memset(prg_mask2_,     0, sizeof(prg_mask2_));
    std::memset(prg_mask4_,     0, sizeof(prg_mask4_));
    std::memset(prg_mask8_,     0, sizeof(prg_mask8_));
    std::memset(prg_mask16_,    0, sizeof(prg_mask16_));
    std::memset(prg_mask32_,    0, sizeof(prg_mask32_));
    std::memset(chr_mask1_,     0, sizeof(chr_mask1_));
    std::memset(chr_mask2_,     0, sizeof(chr_mask2_));
    std::memset(chr_mask4_,     0, sizeof(chr_mask4_));
    std::memset(chr_mask8_,     0, sizeof(chr_mask8_));
    std::memset(prg_is_ram_,    0, sizeof(prg_is_ram_));
    std::memset(prg_ram_,       0, sizeof(prg_ram_));
    std::memset(chr_ram_,       0, sizeof(chr_ram_));
    mirror_hard_ = 0;
}

// ---------------------------------------------------------------------------
// init: do the v1.3.0 cart.cpp::ResetCartMapping equivalent.
// Called once at process start; also re-callable on game load.
// ---------------------------------------------------------------------------
void Bus::init() noexcept {
    // Install default open-bus handler on the entire 64K CPU read
    // and write dispatch. cart.cpp::ResetCartMapping only zeroed
    // Page[] (no ARead/BWrite init), because the original Initialize
    // path in fceu.cpp did ARead[x] = ANull manually. Both are
    // idempotent and equivalent — installing ANull/BNull here keeps
    // the bus in a known state even if a caller never runs the
    // legacy Initialize path.
    for (uint32_t x = 0; x < 0x10000; x++) {
        aread_[x]  = ANullImpl;
        bwrite_[x] = BNullImpl;
    }
    reset_mapping();
}

void Bus::shutdown() noexcept {
    // The Genie shadow buffers (genie_a_read_ / genie_b_write_) are
    // owned by fceu.cpp (via AllocGenieRW / FlushGenieRW); Bus does
    // not free them here.
}

// ---------------------------------------------------------------------------
// reset_mapping: cart.cpp::ResetCartMapping equivalent. Called by
// fceu.cpp::ResetNES / PowerNES on each game load.
// ---------------------------------------------------------------------------
void Bus::reset_mapping() noexcept {
    for (int x = 0; x < 32; x++) {
        page_[x] = nothing - x * 2048;
        prg_ptr_[x] = nullptr;
        chr_ptr_[x] = nullptr;
        prg_size_[x] = 0;
        chr_size_[x] = 0;
    }
    for (int x = 0; x < 8; x++) {
        mmc5_spr_vpage_[x] = mmc5_bg_vpage_[x] = vpage_[x] = nothing - 0x400 * x;
    }
}

// ---------------------------------------------------------------------------
// Accessors (page / vpage / prg_ptr / chr_ptr / aread_table /
// bwrite_table) are defined INLINE in the class body in bus.h so
// the compiler can constant-fold `g_bus.page()` etc. to the
// global's known address. Out-of-line definitions would
// force a function call at every use site, costing ~5% on
// bench_full_frame (measured in Phase 2 pre-optimization).
// ---------------------------------------------------------------------------

void Bus::set_page(uint32_t idx, uint8_t* ptr) noexcept  { page_[idx]  = ptr; }
void Bus::set_vpage(uint32_t idx, uint8_t* ptr) noexcept { vpage_[idx] = ptr; }

// ---------------------------------------------------------------------------
// Direct memory access (debugger / DMR / DMW). Routes through the
// same Page[] table as the read path but skips the aread_[] indirect
// call. Mirrors the cart.cpp::CartBR / CartBW behaviour with no
// prefetch hint (this is the debugger path, not the hot loop).
// ---------------------------------------------------------------------------
uint8_t Bus::direct_read(uint32_t addr) const noexcept {
    return page_[addr >> 11][addr];
}

void Bus::direct_write(uint32_t addr, uint8_t val) noexcept {
    if (prg_is_ram_[addr >> 11] && page_[addr >> 11])
        page_[addr >> 11][addr] = val;
}

// ---------------------------------------------------------------------------
// Handler registration. Bus is the no-wrap path. The Genie-aware
// path stays in fceu.cpp::SetReadHandler / SetWriteHandler (which
// call these on the non-wrap branch).
// ---------------------------------------------------------------------------
void Bus::set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept {
    if (!fn) fn = ANullImpl;
    for (uint32_t x = end; ; x--) {
        aread_[x] = fn;
        if (x == start) break;
    }
}

void Bus::set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept {
    if (!fn) fn = BNullImpl;
    for (uint32_t x = end; ; x--) {
        bwrite_[x] = fn;
        if (x == start) break;
    }
}

// ---------------------------------------------------------------------------
// Bank-switching — setprg8/16/32. Each writes through this->page_
// via set_page(). The legacy setprg*r variants (setprg2r,
// setprg4r, setprg8r, setprg16r, setprg32r) stay in cart.cpp
// because the plan's Bus class only lists the non-r versions.
// ---------------------------------------------------------------------------
void Bus::setprg8(uint32_t A, uint32_t V) noexcept {
    uint32_t bank = V;
    if (prg_size_[0] >= 8192) {
        bank &= prg_mask8_[0];
        uint8_t* base = prg_ptr_[0] ? &prg_ptr_[0][bank << 13] : nullptr;
        uint32_t AB = A >> 11;
        for (int x = 3; x >= 0; x--) {
            prg_is_ram_[AB + x] = prg_ram_[0];
            set_page(AB + x, base ? (base - A) : nullptr);
        }
    } else {
        uint32_t VA = V << 2;
        for (int x = 0; x < 4; x++) {
            uint8_t* base = prg_ptr_[0] ? &prg_ptr_[0][((VA + x) & prg_mask2_[0]) << 11] : nullptr;
            uint32_t AB = (A + (x << 11)) >> 11;
            prg_is_ram_[AB] = prg_ram_[0];
            set_page(AB, base ? (base - (A + (x << 11))) : nullptr);
        }
    }
}

void Bus::setprg16(uint32_t A, uint32_t V) noexcept {
    uint32_t bank = V;
    if (prg_size_[0] >= 16384) {
        bank &= prg_mask16_[0];
        uint8_t* base = prg_ptr_[0] ? &prg_ptr_[0][bank << 14] : nullptr;
        uint32_t AB = A >> 11;
        for (int x = 7; x >= 0; x--) {
            prg_is_ram_[AB + x] = prg_ram_[0];
            set_page(AB + x, base ? (base - A) : nullptr);
        }
    } else {
        uint32_t VA = V << 3;
        for (int x = 0; x < 8; x++) {
            uint8_t* base = prg_ptr_[0] ? &prg_ptr_[0][((VA + x) & prg_mask2_[0]) << 11] : nullptr;
            uint32_t AB = (A + (x << 11)) >> 11;
            prg_is_ram_[AB] = prg_ram_[0];
            set_page(AB, base ? (base - (A + (x << 11))) : nullptr);
        }
    }
}

void Bus::setprg32(uint32_t A, uint32_t V) noexcept {
    uint32_t bank = V;
    if (prg_size_[0] >= 32768) {
        bank &= prg_mask32_[0];
        uint8_t* base = prg_ptr_[0] ? &prg_ptr_[0][bank << 15] : nullptr;
        uint32_t AB = A >> 11;
        for (int x = 15; x >= 0; x--) {
            prg_is_ram_[AB + x] = prg_ram_[0];
            set_page(AB + x, base ? (base - A) : nullptr);
        }
    } else {
        uint32_t VA = V << 4;
        for (int x = 0; x < 16; x++) {
            uint8_t* base = prg_ptr_[0] ? &prg_ptr_[0][((VA + x) & prg_mask2_[0]) << 11] : nullptr;
            uint32_t AB = (A + (x << 11)) >> 11;
            prg_is_ram_[AB] = prg_ram_[0];
            set_page(AB, base ? (base - (A + (x << 11))) : nullptr);
        }
    }
}

// ---------------------------------------------------------------------------
// Bank-switching — setchr1/4/8. Each writes through this->vpage_ via
// set_vpage(). The PPUCHRRAM and PPUNTARAM flags live in ppu.cpp
// and are accessed via the ppu.h declarations (extern).
// ---------------------------------------------------------------------------
void Bus::setchr1(uint32_t A, uint32_t V) noexcept {
    if (!chr_ptr_[0]) return;
    FCEUPPU_LineUpdate();
    uint32_t bank = V & chr_mask1_[0];
    if (chr_ram_[0]) PPUCHRRAM |= (1u << (A >> 10));
    else             PPUCHRRAM &= ~(1u << (A >> 10));
    set_vpage(A >> 10, &chr_ptr_[0][bank << 10] - A);
}

void Bus::setchr4(uint32_t A, uint32_t V) noexcept {
    if (!chr_ptr_[0]) return;
    FCEUPPU_LineUpdate();
    uint32_t bank = V & chr_mask4_[0];
    set_vpage((A) >> 10,        &chr_ptr_[0][bank << 12] - A);
    set_vpage(((A) >> 10) + 1,  &chr_ptr_[0][bank << 12] - A);
    set_vpage(((A) >> 10) + 2,  &chr_ptr_[0][bank << 12] - A);
    set_vpage(((A) >> 10) + 3,  &chr_ptr_[0][bank << 12] - A);
    if (chr_ram_[0]) PPUCHRRAM |= (15u << (A >> 10));
    else             PPUCHRRAM &= ~(15u << (A >> 10));
}

void Bus::setchr8(uint32_t V) noexcept {
    if (!chr_ptr_[0]) return;
    FCEUPPU_LineUpdate();
    uint32_t bank = V & chr_mask8_[0];
    for (int x = 7; x >= 0; x--) {
        set_vpage(x, &chr_ptr_[0][bank << 13]);
    }
    if (chr_ram_[0]) PPUCHRRAM = 0xFF;
    else             PPUCHRRAM = 0;
}

// ---------------------------------------------------------------------------
// Mirroring + NTAMEM. PPU side (PPUNTARAM, vnapage[], NTARAM) lives
// in ppu.cpp and is accessed via ppu.h.
// ---------------------------------------------------------------------------
void Bus::setmirror(uint32_t m) noexcept {
    FCEUPPU_LineUpdate();
    if (!mirror_hard_) {
        switch (m) {
        case 0:  // MI_H
            vnapage[0] = vnapage[1] = NTARAM;
            vnapage[2] = vnapage[3] = NTARAM + 0x400;
            break;
        case 1:  // MI_V
            vnapage[0] = vnapage[2] = NTARAM;
            vnapage[1] = vnapage[3] = NTARAM + 0x400;
            break;
        case 2:  // MI_0
            vnapage[0] = vnapage[1] = vnapage[2] = vnapage[3] = NTARAM;
            break;
        case 3:  // MI_1
            vnapage[0] = vnapage[1] = vnapage[2] = vnapage[3] = NTARAM + 0x400;
            break;
        }
        PPUNTARAM = 0xF;
    }
}

void Bus::setmirrorw(uint32_t a, uint32_t b, uint32_t c, uint32_t d) noexcept {
    FCEUPPU_LineUpdate();
    vnapage[0] = NTARAM + a * 0x400;
    vnapage[1] = NTARAM + b * 0x400;
    vnapage[2] = NTARAM + c * 0x400;
    vnapage[3] = NTARAM + d * 0x400;
}

void Bus::setntamem(uint8_t* p, int ram, uint32_t b) noexcept {
    FCEUPPU_LineUpdate();
    vnapage[b] = p;
    PPUNTARAM &= ~(1u << b);
    if (ram) PPUNTARAM |= (1u << b);
}

// ---------------------------------------------------------------------------
// ROM pointer setup. Replaces cart.cpp::SetupCartPRGMapping /
// SetupCartCHRMapping. Also exposes setup_mirroring for the
// hard-mirroring case (was SetupCartMirroring in v1.3.0).
// ---------------------------------------------------------------------------
void Bus::setup_prg_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept {
    prg_ptr_[chip]  = p;
    prg_size_[chip] = size;
    prg_mask2_[chip]  = (size >> 11) - 1;
    prg_mask4_[chip]  = (size >> 12) - 1;
    prg_mask8_[chip]  = (size >> 13) - 1;
    prg_mask16_[chip] = (size >> 14) - 1;
    prg_mask32_[chip] = (size >> 15) - 1;
    prg_ram_[chip]    = ram ? 1 : 0;
}

void Bus::setup_chr_mapping(uint32_t chip, uint8_t* p, uint32_t size, int ram) noexcept {
    chr_ptr_[chip]  = p;
    chr_size_[chip] = size;
    chr_mask1_[chip] = (size >> 10) - 1;
    chr_mask2_[chip] = (size >> 11) - 1;
    chr_mask4_[chip] = (size >> 12) - 1;
    chr_mask8_[chip] = (size >> 13) - 1;
    if (chr_mask1_[chip] >= static_cast<uint32_t>(-1)) chr_mask1_[chip] = 0;
    if (chr_mask2_[chip] >= static_cast<uint32_t>(-1)) chr_mask2_[chip] = 0;
    if (chr_mask4_[chip] >= static_cast<uint32_t>(-1)) chr_mask4_[chip] = 0;
    if (chr_mask8_[chip] >= static_cast<uint32_t>(-1)) chr_mask8_[chip] = 0;
    chr_ram_[chip] = static_cast<uint8_t>(ram);
}

void Bus::setup_mirroring(int m, int hard, uint8_t* extra) noexcept {
    if (m < 4) {
        mirror_hard_ = 0;
        setmirror(static_cast<uint32_t>(m));
    } else {
        vnapage[0] = NTARAM;
        vnapage[1] = NTARAM + 0x400;
        vnapage[2] = extra;
        vnapage[3] = extra + 0x400;
        PPUNTARAM = 0xF;
    }
    mirror_hard_ = hard;
}

// ---------------------------------------------------------------------------
// Direct global instance (v1.4 Phase 3 §5.1.1). Replaces the Phase 2
// Meyers singleton `bus_instance()` because a non-inline function
// call to resolve the singleton address costs ~5% on bench_full_frame.
// `g_bus` is constructed during static init (zero-init at program
// load, then Bus::Bus() runs). Declared `extern` in bus.h.
//
// IMPORTANT: g_bus must be declared BEFORE the reference-alias
// definitions below so the static-init order in this TU is well
// defined (g_bus is fully constructed by the time the aliases
// bind to its members).
// ---------------------------------------------------------------------------
Bus g_bus;

} // namespace fceu11

// ---------------------------------------------------------------------------
// VPageR — uint8** pointer alias for &VPage[0]. Declared `extern` in
// bus.h because its initializer is not constexpr (g_bus is a global
// object, not a constant). Used by datalatch.cpp, mmc5.cpp, and the
// legacy setchr*r code paths in cart.cpp.
// ---------------------------------------------------------------------------
uint8_t** VPageR = &fceu11::g_bus.vpage()[0];

// ---------------------------------------------------------------------------
// Global reference aliases (definitions matching the `extern`
// declarations in bus.h). Each alias is a reference to a g_bus
// member array; initialized once at static init. Every consumer TU
// sees these as ordinary global references — the compiler treats
// ARead[i] as a direct array-index + indirect-call, identical to
// v1.3.0's `::ARead[i](addr)` machine code (no per-use g_bus
// indirection — the link-time address is fixed). This restores
// the bench_full_frame performance that was lost when the
// Phase 2 inline-alias version forced a runtime function call
// at every dispatch.
// ---------------------------------------------------------------------------
readfunc  (& ARead )[0x10000] = fceu11::g_bus.aread_table();
writefunc (& BWrite)[0x10000] = fceu11::g_bus.bwrite_table();

uint8_t* (& Page        )[32] = fceu11::g_bus.page();
uint8_t* (& VPage       )[8]  = fceu11::g_bus.vpage();
uint8_t* (& VPageG      )[8]  = fceu11::g_bus.vpage_g();
uint8_t* (& MMC5SPRVPage)[8]  = fceu11::g_bus.mmc5_spr_vpage();
uint8_t* (& MMC5BGVPage )[8]  = fceu11::g_bus.mmc5_bg_vpage();

uint8_t* (& PRGptr)[32] = fceu11::g_bus.prg_ptr();
uint8_t* (& CHRptr)[32] = fceu11::g_bus.chr_ptr();

uint32_t (& PRGsize  )[32] = fceu11::g_bus.prg_size();
uint32_t (& CHRsize  )[32] = fceu11::g_bus.chr_size();
uint32_t (& PRGmask2 )[32] = fceu11::g_bus.prg_mask2();
uint32_t (& PRGmask4 )[32] = fceu11::g_bus.prg_mask4();
uint32_t (& PRGmask8 )[32] = fceu11::g_bus.prg_mask8();
uint32_t (& PRGmask16)[32] = fceu11::g_bus.prg_mask16();
uint32_t (& PRGmask32)[32] = fceu11::g_bus.prg_mask32();
uint32_t (& CHRmask1 )[32] = fceu11::g_bus.chr_mask1();
uint32_t (& CHRmask2 )[32] = fceu11::g_bus.chr_mask2();
uint32_t (& CHRmask4 )[32] = fceu11::g_bus.chr_mask4();
uint32_t (& CHRmask8 )[32] = fceu11::g_bus.chr_mask8();
uint8_t  (& PRGram   )[32] = fceu11::g_bus.prg_ram();
uint8_t  (& CHRram   )[32] = fceu11::g_bus.chr_ram();
uint8_t  (& PRGIsRAM )[32] = fceu11::g_bus.prg_is_ram();
