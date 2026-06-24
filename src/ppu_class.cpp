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
// ---------------------------------------------------------------------------

void Ppu::set_chr_ram(uint8_t /*mask*/) noexcept {}
void Ppu::set_nt_ram (uint8_t /*mask*/) noexcept {}
void Ppu::set_mirror_page(uint32_t /*idx*/, uint8_t* /*ptr*/) noexcept {}
void Ppu::set_mirror_mode(uint32_t /*mode*/) noexcept {}
void Ppu::set_mirror_pages(uint8_t /*a*/, uint8_t /*b*/,
                           uint8_t /*c*/, uint8_t /*d*/) noexcept {}
void Ppu::notify_line_update() noexcept {}

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