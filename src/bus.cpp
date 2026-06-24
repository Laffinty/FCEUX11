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

// v1.5 Prism §5.x target: redirect PPU side-effects through a Ppu&
// reference (Bus would hold a Ppu* member). The current direct
// PPUCHRRAM / PPUNTARAM / vnapage[] writes in setchr1 / setchr4 /
// setchr8 / setmirror / setmirrorw / setntamem are the natural
// seam where v1.5 work will pull — see
// docs/v1.x_Modernization_Roadmap.md §5.2 "渲染内部状态迁入".

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
    // v1.4 Post-Release Optimization Plan §2.2: use the new typed
    // accessor (Cpu::db()) instead of reaching through native_layout().
    uint8_t ANullImpl(uint32_t) {
        return g_cpu.db();
    }
    void BNullImpl(uint32_t, uint8_t) {}
} // namespace

// ---------------------------------------------------------------------------
// ctor: zero-init everything via a single whole-object memset
// (v1.4 Post-Release Optimization Plan §2.3 — replaces 22 per-array
// memsets, which were easy to forget when a new Bus member array is
// added in v1.5+). The Genie shadow pointers / mirror_hard_ all
// default-init to 0 / nullptr already, and the wholesale memset
// keeps them at 0 / nullptr which is what AllocGenieRW / PowerNES
// expect at first contact.
// ---------------------------------------------------------------------------
Bus::Bus() noexcept {
    std::memset(this, 0, sizeof(*this));
}

// ---------------------------------------------------------------------------
// init: install the open-bus handlers (ANullImpl / BNullImpl) on the
// entire 64K CPU read/write dispatch.
//
// v1.4 Post-Release Optimization Plan §1.1: this replaces the legacy
// `SetReadHandler(0, 0xFFFF, ANull); SetWriteHandler(0, 0xFFFF, BNull);`
// pair in fceu.cpp::PowerNES. Intentionally does NOT call
// reset_mapping() — SetupCartPRGMapping / SetupCartCHRMapping
// (called by FCEUXLoad BEFORE PowerNES) populate prg_ptr_[] /
// chr_ptr_[], and reset_mapping() would wipe those before the
// mapper's Power handler can call setprg* / setchr*. Callers that
// want the v1.3.0 cart.cpp::ResetCartMapping effect should call
// reset_mapping() separately.
//
// Called once at process start (and again on each game load by
// PowerNES) — idempotent.
// ---------------------------------------------------------------------------
void Bus::init() noexcept {
    for (uint32_t x = 0; x < 0x10000; x++) {
        aread_[x]  = ANullImpl;
        bwrite_[x] = BNullImpl;
    }
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
//
// v1.4 Post-Release Optimization Plan §1.2 — ascending loop with
// explicit end < start early-out. The previous descending loop
// relied on a quirk: a caller passing end < start would cause `x` to
// underflow to UINT32_MAX and the loop would run ~4G times. All
// current callers (v1.4 grep) pass end >= start, but the API did
// not enforce it; the new form is both safer and clearer.
// ---------------------------------------------------------------------------
void Bus::set_read_handler(uint32_t start, uint32_t end, readfunc fn) noexcept {
    // Defensive guard (v1.4 Post-Release Optimization Plan §1.2):
    // a caller passing end < start would underflow the previous
    // descending loop. Silently no-op rather than spin ~4G times.
    if (end < start) return;
    if (!fn) fn = ANullImpl;
    for (uint32_t x = start; x <= end; x++) {
        aread_[x] = fn;
    }
}

void Bus::set_write_handler(uint32_t start, uint32_t end, writefunc fn) noexcept {
    if (end < start) return;
    if (!fn) fn = BNullImpl;
    for (uint32_t x = start; x <= end; x++) {
        bwrite_[x] = fn;
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
// set_vpage(). The PPU side (chr_ram_mask_, nt_ram_mask_, vnapage_[],
// ntaram_) lives in fceu11::g_ppu; we route the writes through
// ppu_->method() per plan §3 (Bus → Ppu decoupling). The compat
// aliases PPUCHRRAM / PPUNTARAM / vnapage[] / NTARAM bind to those
// same g_ppu members so ppu.cpp's read-side access (which still uses
// the v1.0 names through the reference aliases) sees the same bytes.
// ---------------------------------------------------------------------------
void Bus::setchr1(uint32_t A, uint32_t V) noexcept {
    if (!chr_ptr_[0]) return;
    if (ppu_) ppu_->notify_line_update();
    else      FCEUPPU_LineUpdate();   // plan §3.3 fallback
    uint32_t bank = V & chr_mask1_[0];
    if (ppu_) {
        uint8_t mask = ppu_->chr_ram_mask();
        if (chr_ram_[0]) mask |= (1u << (A >> 10));
        else             mask &= ~(1u << (A >> 10));
        ppu_->set_chr_ram(mask);
    } else {
        if (chr_ram_[0]) PPUCHRRAM |= (1u << (A >> 10));
        else             PPUCHRRAM &= ~(1u << (A >> 10));
    }
    set_vpage(A >> 10, &chr_ptr_[0][bank << 10] - A);
}

void Bus::setchr4(uint32_t A, uint32_t V) noexcept {
    if (!chr_ptr_[0]) return;
    if (ppu_) ppu_->notify_line_update();
    else      FCEUPPU_LineUpdate();
    uint32_t bank = V & chr_mask4_[0];
    set_vpage((A) >> 10,        &chr_ptr_[0][bank << 12] - A);
    set_vpage(((A) >> 10) + 1,  &chr_ptr_[0][bank << 12] - A);
    set_vpage(((A) >> 10) + 2,  &chr_ptr_[0][bank << 12] - A);
    set_vpage(((A) >> 10) + 3,  &chr_ptr_[0][bank << 12] - A);
    if (ppu_) {
        uint8_t mask = ppu_->chr_ram_mask();
        if (chr_ram_[0]) mask |= (15u << (A >> 10));
        else             mask &= ~(15u << (A >> 10));
        ppu_->set_chr_ram(mask);
    } else {
        if (chr_ram_[0]) PPUCHRRAM |= (15u << (A >> 10));
        else             PPUCHRRAM &= ~(15u << (A >> 10));
    }
}

void Bus::setchr8(uint32_t V) noexcept {
    if (!chr_ptr_[0]) return;
    if (ppu_) ppu_->notify_line_update();
    else      FCEUPPU_LineUpdate();
    uint32_t bank = V & chr_mask8_[0];
    for (int x = 7; x >= 0; x--) {
        set_vpage(x, &chr_ptr_[0][bank << 13]);
    }
    if (ppu_) {
        ppu_->set_chr_ram(chr_ram_[0] ? 0xFF : 0x00);
    } else {
        if (chr_ram_[0]) PPUCHRRAM = 0xFF;
        else             PPUCHRRAM = 0;
    }
}

// ---------------------------------------------------------------------------
// Mirroring + NTAMEM. vnapage[] / PPUNTARAM / NTARAM live in g_ppu;
// we route through ppu_->set_mirror_mode / set_mirror_pages /
// set_mirror_page per plan §3. The fallback (ppu_ == nullptr) writes
// through the v1.0 reference aliases, which are bound to the same
// g_ppu members — so the rendered output is identical either way.
// ---------------------------------------------------------------------------
void Bus::setmirror(uint32_t m) noexcept {
    if (ppu_) ppu_->notify_line_update();
    else      FCEUPPU_LineUpdate();
    if (!mirror_hard_) {
        if (ppu_) ppu_->set_mirror_mode(m);
        else {
            // Fallback: same switch as Ppu::set_mirror_mode.
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
}

void Bus::setmirrorw(uint32_t a, uint32_t b, uint32_t c, uint32_t d) noexcept {
    if (ppu_) ppu_->notify_line_update();
    else      FCEUPPU_LineUpdate();
    if (ppu_) ppu_->set_mirror_pages(a, b, c, d);
    else {
        vnapage[0] = NTARAM + a * 0x400;
        vnapage[1] = NTARAM + b * 0x400;
        vnapage[2] = NTARAM + c * 0x400;
        vnapage[3] = NTARAM + d * 0x400;
    }
}

void Bus::setntamem(uint8_t* p, int ram, uint32_t b) noexcept {
    if (ppu_) ppu_->notify_line_update();
    else      FCEUPPU_LineUpdate();
    if (ppu_) {
        ppu_->set_mirror_page(b, p);
        uint8_t mask = ppu_->nt_ram_mask() & ~(1u << b);
        if (ram) mask |= (1u << b);
        ppu_->set_nt_ram(mask);
    } else {
        vnapage[b] = p;
        PPUNTARAM &= ~(1u << b);
        if (ram) PPUNTARAM |= (1u << b);
    }
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
