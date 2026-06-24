// FCEUX11 — v1.5 Prism §1: fceu11::Ppu class implementation.
//
// Phase B scaffolding (plan §10.2 + §6.1): the class compiles, all
// 5 compat aliases wire to canonical storage, method bodies are
// no-ops. Phase C/D/E will fill in the bodies (vtoggle/vaddr,
// line_buffer, OAM, etc.) in three independently-testable batches.
//
// The 5 reference aliases at the bottom of this file are the seam
// between v1.0 PPU globals and the new Ppu-owned storage. Binding
// pattern matches v1.4 Bus (bus.cpp:406-428): a single TU provides
// the initializer; every consumer TU sees the alias as a fixed-
// address global reference at link time.

#include "ppu_class.h"

// Forward decl — ppu.h can't be included here (circular: ppu.h
// already includes ppu_class.h). The actual definition is in ppu.cpp;
// notify_line_update() below calls it.
void FCEUPPU_LineUpdate();

namespace fceu11 {

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

Ppu::Ppu() noexcept {
    // All member arrays are zero-initialized by the compiler
    // (alignas(64) ntaram_[0x800], vnapage_[4], regs_[4]) or have
    // default member initializers (chr_ram_mask_, nt_ram_mask_,
    // phase_). This matches v1.0 BSS defaults for the migrated
    // globals (PPU[4]=zero, PPUCHRRAM=0, PPUNTARAM=0, NTARAM=zero,
    // vnapage=nullptr).
}

void Ppu::init() noexcept    { /* Phase C/E: real impl */ }
void Ppu::shutdown() noexcept { /* Phase C/E: real impl */ }
void Ppu::power() noexcept    { /* Phase C/E: real impl */ }
void Ppu::reset() noexcept    { /* Phase C/E: real impl */ }
int  Ppu::loop(int /*skip*/) noexcept { return 0; /* Phase D: real impl */ }

// ---------------------------------------------------------------------------
// Register file (Phase B: read returns 0, write is dropped; the
// real PPU[0..3] write semantics come in Phase C / Batch 1, when
// ppu.cpp's existing write paths migrate to g_ppu.set_reg(idx, v)).
// ---------------------------------------------------------------------------

uint8_t Ppu::reg(uint32_t /*idx*/) const noexcept { return 0; }
void    Ppu::set_reg(uint32_t /*idx*/, uint8_t /*v*/) noexcept {}

// ---------------------------------------------------------------------------
// Frame phase
// ---------------------------------------------------------------------------

PPUPHASE Ppu::phase() const noexcept { return phase_; }
void     Ppu::set_phase(PPUPHASE p) noexcept { phase_ = p; }

// ---------------------------------------------------------------------------
// Debug (Phase B: stub returning 0; Phase C reads from scanline /
// newppu_get_scanline() etc.).
// ---------------------------------------------------------------------------

int Ppu::scanline() const noexcept { return 0; }
int Ppu::dot()      const noexcept { return 0; }

// ---------------------------------------------------------------------------
// Bank-switching entry points (Phase B: no-op; Phase F replaces the
// direct PPUCHRRAM / PPUNTARAM / vnapage[] writes in Bus::setchr* /
// setmirror* / setntamem with calls through these methods).
//
// Deliberately empty in Phase B — leaving the writes to the existing
// ppu.cpp / cart.cpp code paths guarantees the visual-diff baseline
// stays 0-pixel identical to the v1.4 + WIP pre-class state.
//
// Phase F fills these in so Bus::setchr* / setmirror* / setntamem
// can route through ppu_->method() instead of touching the v1.0
// globals directly. Each method writes through g_ppu's own member
// storage; the compat aliases (PPUCHRRAM / PPUNTARAM / vnapage[] /
// NTARAM) bind to the same members so ppu.cpp's read-side access
// (which still uses the v1.0 names through the reference aliases)
// sees the same bytes.
// ---------------------------------------------------------------------------

// Phase F: bus-switching entry points. Bus::setchr* / setmirror* /
// setntamem call these to update PPU-side mirror / CHR-RAM state.
// Per plan §3.5 these are warm-path (mapper register write), so the
// ~3-cycle indirect-call cost is acceptable. They MUST NOT be called
// from inside Ppu::loop() / RefreshLine — that would be hot-path and
// blow up bench_ppu_frame. Bus::setchr1/4/8 already call them only
// on mapper register writes (not per-cycle).
void Ppu::set_chr_ram(uint8_t mask) noexcept {
    chr_ram_mask_ = mask;
}
void Ppu::set_nt_ram(uint8_t mask) noexcept {
    nt_ram_mask_ = mask;
}
void Ppu::set_mirror_page(uint32_t idx, uint8_t* ptr) noexcept {
    vnapage_[idx & 3] = ptr;
}
void Ppu::set_mirror_mode(uint32_t mode) noexcept {
    // Same switch as v1.0 Bus::setmirror (bus.cpp:288-309).
    uint8_t* nt = ntaram_;
    switch (mode) {
    case 0:  // MI_H (horizontal)
        vnapage_[0] = vnapage_[1] = nt;
        vnapage_[2] = vnapage_[3] = nt + 0x400;
        break;
    case 1:  // MI_V (vertical)
        vnapage_[0] = vnapage_[2] = nt;
        vnapage_[1] = vnapage_[3] = nt + 0x400;
        break;
    case 2:  // MI_0
        vnapage_[0] = vnapage_[1] = vnapage_[2] = vnapage_[3] = nt;
        break;
    case 3:  // MI_1
        vnapage_[0] = vnapage_[1] = vnapage_[2] = vnapage_[3] = nt + 0x400;
        break;
    }
    nt_ram_mask_ = 0xF;
}
void Ppu::set_mirror_pages(uint8_t a, uint8_t b, uint8_t c, uint8_t d) noexcept {
    uint8_t* nt = ntaram_;
    vnapage_[0] = nt + a * 0x400;
    vnapage_[1] = nt + b * 0x400;
    vnapage_[2] = nt + c * 0x400;
    vnapage_[3] = nt + d * 0x400;
}
void Ppu::notify_line_update() noexcept {
    // FCEUPPU_LineUpdate is defined in ppu.cpp. We can't include
    // ppu.h here (circular: ppu.h includes ppu_class.h), so the
    // declaration is provided as a forward decl at the top of this
    // file.
    FCEUPPU_LineUpdate();
}

// ---------------------------------------------------------------------------
// Direct global instance (plan §1.2). Real global object — same pattern
// as Bus g_bus. The linker gives g_ppu a fixed address; every
// `g_ppu.ntaram_[i]` / `g_ppu.vnapage()[j]` call site compiles to a
// direct array-index through a known address. No Meyers singleton:
// v1.14 LTO needs the singleton address link-time visible.
// ---------------------------------------------------------------------------

Ppu g_ppu;

} // namespace fceu11

// ---------------------------------------------------------------------------
// Compat-alias initializers (plan §1.1).
//
// All five reference-to-storage aliases bind to g_ppu members:
//   PPU[4]      -> g_ppu.regs_alias()    = g_ppu.regs_[4]
//   NTARAM      -> g_ppu.ntaram()        = g_ppu.ntaram_[0x800]
//   vnapage[4]  -> g_ppu.vnapage()       = g_ppu.vnapage_[4]
//   PPUCHRRAM   -> g_ppu.chr_ram_mask()  = g_ppu.chr_ram_mask_
//   PPUNTARAM   -> g_ppu.nt_ram_mask()   = g_ppu.nt_ram_mask_
//
// Initialization order: g_ppu is constructed first (declared before
// these aliases in this TU); the alias initializers run next during
// dynamic init. By the time main() runs, every alias is bound to
// canonical storage.
// ---------------------------------------------------------------------------

uint8_t   (& PPU    )[4]    = fceu11::g_ppu.regs_alias();
uint8_t   (& NTARAM )[0x800] = fceu11::g_ppu.ntaram();
uint8_t*  (& vnapage)[4]    = fceu11::g_ppu.vnapage();
uint8_t   (& PPUCHRRAM)     = fceu11::g_ppu.chr_ram_mask();
uint8_t   (& PPUNTARAM)     = fceu11::g_ppu.nt_ram_mask();

// Batch 1 (plan §2.1) alias bindings — control-register mirror state.
// Bind to g_ppu's batch-1 member fields so existing ppu.cpp / pputile.inc /
// debug.cpp / mmc5.cpp / state.cpp call sites (which use the v1.0 names)
// transparently read/write through the class.
uint8_t  (& vtoggle)        = fceu11::g_ppu.vtoggle();
uint8_t  (& XOffset)        = fceu11::g_ppu.fine_x_scroll();
uint32_t (& TempAddr)       = fceu11::g_ppu.vaddr();
uint32_t (& RefreshAddr)    = fceu11::g_ppu.vaddr_latch();
uint32_t (& NTRefreshAddr)  = fceu11::g_ppu.nt_refresh_addr();
uint32_t (& DummyRead)      = fceu11::g_ppu.dummy_read();

// Batch 3 (plan §2.3) alias binding — OAM.
uint8_t (& SPRAM)[0x100]    = fceu11::g_ppu.oam();