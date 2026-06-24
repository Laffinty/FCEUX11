// FCEUX11 — v1.5 Prism §1: fceu11::Ppu class declaration.
//
// Goal: encapsulate PPU register file, name-table RAM, and rendering
// state inside a single class, mirroring the v1.4 Bus pattern
// (Bus g_bus direct global + reference-to-array aliases for v1.0
// compat). Phase B scaffolding — the class compiles, the 5 compat
// aliases wire to canonical storage, and the methods have empty
// bodies. Phase C/D/E will fill the method bodies in three batches:
//   - Batch 1 (§2.1): vtoggle, vaddr, vaddr_latch, temp_vaddr,
//                     fine_x_scroll, ppucontrol_a, ppumask_*
//   - Batch 2 (§2.2): line_buffer, bg_latch, sprite_latch,
//                     priority_latch
//   - Batch 3 (§2.3): SPRAM, Spr_Pri, Spr_Count, Spr_Index,
//                     Sprite0Hit, MaxSprites
//
// Storage layout (per plan §1.1):
//   * alignas(64) ntaram_[] first — hot path (PPU reads NTARAM every
//     tile fetch; cache-line aligned to match the v1.0 BSS alignment)
//   * vnapage_[4] next — 32 bytes of pointer table
//   * regs_[4] next — 4-byte register file (PPU[0..3])
//   * chr_ram_mask_, nt_ram_mask_ — single-byte PPU flags
//   * phase_ — small control field
//
// All 5 alias targets now live inside the class (NTARAM, vnapage,
// PPU[4], PPUCHRRAM, PPUNTARAM). The plan §1.3 wording "PPU[4] stays
// as v1.0 global" was aspirational for savestate compatibility — in
// practice a reference-to-storage alias cannot coexist in the same
// TU as a variable definition of the same name, so PPU[4] migrates
// into the class. The v1.0 savestate chunk format is preserved by
// pointing the SFORMAT descriptor at g_ppu.regs_ instead of PPU[] in
// Phase E (Batch 3); for Phase B (visual diff) the byte-level
// semantics are unchanged.
//
// Compat aliases (plan §1.1): the v1.0 global names PPU, NTARAM,
// vnapage, PPUCHRRAM, PPUNTARAM are bound as `extern` reference-to-
// storage aliases; ppu_class.cpp provides the initializer pointing at
// g_ppu members. Every existing call site that does
// `NTARAM[x] = v`, `vnapage[i] = p`, `PPU[2] |= 0x80`,
// `PPUCHRRAM |= mask`, or `PPUNTARAM = 0xF` writes through to the
// g_ppu storage without code change.

#ifndef FCEU11_PPU_CLASS_H
#define FCEU11_PPU_CLASS_H

#include <cstdint>
#include <cstddef>

#include "types.h"
#include "utils/cache.h"       // FCEUX11_CACHE_ALIGN, kCacheLineSize

// PPUPHASE — the three documented PPU frame phases. Defined here
// (not ppu.h) to break the circular include: ppu.h includes this
// header, and the class body uses PPUPHASE for the phase_ default
// member initializer. boards/mmc5.cpp and core_state.cpp get
// PPUPHASE transitively via ppu.h -> ppu_class.h.
enum PPUPHASE {
    PPUPHASE_VBL, PPUPHASE_BG, PPUPHASE_OBJ
};

namespace fceu11 {

class FCEUX11_CACHE_ALIGN Ppu {
public:
    // ---- Lifecycle (Phase B: no-op; Phase C/E: real impls) ----
    Ppu() noexcept;
    void init() noexcept;
    void shutdown() noexcept;
    void power() noexcept;
    void reset() noexcept;
    int  loop(int skip) noexcept;

    // ---- Register file access (Phase B: read returns 0, write is dropped;
    // regs_[4] is the new home of the v1.0 PPU[0..3] storage).
    uint8_t reg(uint32_t idx) const noexcept;
    void    set_reg(uint32_t idx, uint8_t v) noexcept;

    // ---- Name table RAM + pointer table ----
    // `ntaram()` returns a reference to the 0x800-byte internal array.
    // Return type is `uint8_t (&)[0x800]` (array reference), NOT a
    // pointer — this preserves v1.0 `uint8 NTARAM[0x800]` semantics so
    // every call site that does `NTARAM[idx]`, `NTARAM + 0x400`,
    // `&NTARAM[0x400]`, or `sizeof(NTARAM)` keeps working unchanged.
    // The alias `extern uint8_t (& NTARAM)[0x800]` in this header
    // binds to the same array.
    __forceinline uint8_t (& ntaram() noexcept)[0x800] { return ntaram_; }

    // `vnapage()` returns a reference to the 4-entry pointer table.
    // The alias `extern uint8_t* (& vnapage)[4]` binds to the same
    // array. Type preserved as `uint8_t*[4]` (array of pointers) so
    // `vnapage[i] = p` keeps working without change.
    __forceinline uint8_t* (& vnapage() noexcept)[4] { return vnapage_; }

    // ---- Frame phase ----
    PPUPHASE phase() const noexcept;
    void     set_phase(PPUPHASE p) noexcept;

    // ---- Debug ----
    int scanline() const noexcept;
    int dot() const noexcept;

    // ---- Bank-switching entry points (Phase B: no-op; Phase F: real) ----
    // These are the seams Bus::setchr*/setmirror*/setntamem call into.
    // For Phase B they are pure stubs that do NOT touch PPUCHRRAM /
    // PPUNTARAM / vnapage / NTARAM — leaving those writes to the
    // existing ppu.cpp / cart.cpp code paths so we get a clean 0-diff
    // visual baseline at the v1.4 + WIP + empty-Ppu state.
    void set_chr_ram(uint8_t mask) noexcept;
    void set_nt_ram(uint8_t mask) noexcept;
    void set_mirror_page(uint32_t idx, uint8_t* ptr) noexcept;
    void set_mirror_mode(uint32_t mode) noexcept;
    void set_mirror_pages(uint8_t a, uint8_t b, uint8_t c, uint8_t d) noexcept;
    void notify_line_update() noexcept;

    // ---- Savestate compat (Phase B: raw_ntaram() returns ntaram_) ----
    // Per plan §1.1: NTARAM migration uses this raw pointer so the v1.0
    // savestate chunk (which serializes NTARAM as a contiguous 0x800
    // block) keeps memcpy-ing the same bytes.
    __forceinline uint8_t* raw_ntaram() noexcept { return ntaram_; }
    static constexpr std::size_t raw_ntaram_size() noexcept { return 0x800; }

    // ---- Internal-storage accessors (used to bind compat aliases) ----
    // All 5 alias targets are members of this class; the accessors
    // return references so the alias `extern ... (& NAME) ... = g_ppu.X()`
    // bindings in ppu_class.cpp can bind to the same storage.
    __forceinline uint8_t (& regs_alias() noexcept)[4] { return regs_; }
    __forceinline uint8_t (& chr_ram_mask() noexcept)   { return chr_ram_mask_; }
    __forceinline uint8_t (& nt_ram_mask() noexcept)    { return nt_ram_mask_; }

private:
    // Storage. See file-top comment for layout rationale.
    alignas(64) uint8_t ntaram_[0x800];    // name-table RAM (2 KB)
    uint8_t* vnapage_[4];                  // 4-entry pointer table (32 B)
    uint8_t  regs_[4] = {};                // PPU[0..3] register file (4 B)
    uint8_t  chr_ram_mask_ = 0;            // PPUCHRRAM flag (1 B)
    uint8_t  nt_ram_mask_  = 0;            // PPUNTARAM flag (1 B)
    PPUPHASE phase_ = PPUPHASE_VBL;        // frame phase
};

// Direct global instance (plan §1.2). Same pattern as Bus g_bus —
// a real global object so the linker gives it a fixed address and the
// compiler folds `g_ppu.ntaram_[i]` / `g_ppu.vnapage()[j]` call site
// accesses to direct array-index through a known address. No Meyers
// singleton: v1.14 LTO needs the singleton address link-time visible.
extern Ppu g_ppu;

} // namespace fceu11

// ---------------------------------------------------------------------------
// Compat aliases (plan §1.1). Reference-to-storage globals, defined in
// ppu_class.cpp. The binding pattern matches v1.4 Bus (bus.h lines 244-269
// + bus.cpp lines 406-428) — a single TU provides the initializer and
// every consumer TU sees the alias as a regular global reference.
//
// Why `extern` and not `inline`:
//
//   If these were `inline` references whose initializer called a
//   function, every consumer TU would emit a real function call at
//   every use site to resolve the table base. With `extern` + a
//   single TU's initializer, the alias becomes a fixed-address global
//   reference at link time, and `NTARAM[x] = v` compiles to a direct
//   store through a known address. Same machine code as the v1.0
//   `::NTARAM[x] = v` direct-global path.
// ---------------------------------------------------------------------------

// PPU[4] — 4-byte register array (v1.0 PPU[0]=CTRL, PPU[1]=MASK,
// PPU[2]=STATUS, PPU[3]=OAMADDR). Bound to g_ppu.regs_.
extern uint8_t (& PPU)[4];

// NTARAM — name-table RAM (0x800 bytes). Array-reference alias (NOT a
// pointer — see the g_ppu.ntaram() doc above for the type-choice
// rationale). Bound to g_ppu.ntaram_.
extern uint8_t (& NTARAM)[0x800];

// vnapage[4] — pointer table. Array-of-pointers reference. Bound to
// g_ppu.vnapage_.
extern uint8_t* (& vnapage)[4];

// PPUCHRRAM, PPUNTARAM — single-byte masks. Bound to g_ppu.chr_ram_mask_
// / g_ppu.nt_ram_mask_.
extern uint8_t (& PPUCHRRAM);
extern uint8_t (& PPUNTARAM);

#endif // FCEU11_PPU_CLASS_H