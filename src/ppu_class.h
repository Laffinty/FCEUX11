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

// ----------------------------------------------------------------------------
// v1.12 Scissors Phase E-A: PPU state-register / sprite-eval structs.
//
// Originally file-scope in src/ppu.cpp (lines 112-292). Moved here so the
// Phase E-A SFORMAT tables (which take `&ppur.field`, `&spr_read.num`,
// etc.) can be compiled in ppu_state.cpp's TU. The struct bodies are
// kept byte-identical to the v1.11 originals to preserve the savestate
// chunk layout (the SFORMAT array in ppu_state.cpp byte-for-byte mirrors
// what state.cpp:614 deserializes).
// ----------------------------------------------------------------------------

struct PPUSTATUS {
    int32_t sl;
    int32_t cycle, end_cycle;
};

struct SPRITE_READ {
    int32_t num;
    int32_t count;
    int32_t fetch;
    int32_t found;
    int32_t found_pos[8];
    int32_t ret;
    int32_t last;
    int32_t mode;

    void reset() {
        num = count = fetch = found = ret = last = mode = 0;
        found_pos[0] = found_pos[1] = found_pos[2] = found_pos[3] = 0;
        found_pos[4] = found_pos[5] = found_pos[6] = found_pos[7] = 0;
    }

    void start_scanline() {
        num = 1;
        found = 0;
        fetch = 1;
        count = 0;
        last = 64;
        mode = 0;
        found_pos[0] = found_pos[1] = found_pos[2] = found_pos[3] = 0;
        found_pos[4] = found_pos[5] = found_pos[6] = found_pos[7] = 0;
    }
};

// PPUREGS — the new-PPU's per-frame state-register file (mirrors the
// 5-scroll-counter daisy chain + latched copies + derived fields).
// Reference docs: http://nesdev.icequake.net/PPU%20addressing.txt
struct PPUREGS {
    uint32_t fv;        //3
    uint32_t v;         //1
    uint32_t h;         //1
    uint32_t vt;        //5
    uint32_t ht;        //5

    uint32_t _fv, _v, _h, _vt, _ht;

    uint32_t fh;        //3 (horz scroll)
    uint32_t s;         //1 ($2000 bit 4)
    uint32_t par;       //8

    PPUSTATUS status;

    void reset() {
        fv = v = h = vt = ht = 0;
        fh = par = s = 0;
        _fv = _v = _h = _vt = _ht = 0;
        status.cycle = 0;
        status.end_cycle = 341;
        status.sl = 241;
    }
    void install_latches() { fv = _fv; v = _v; h = _h; vt = _vt; ht = _ht; }
    void install_h_latches() { ht = _ht; h = _h; }
    void clear_latches() { _fv = _v = _h = _vt = _ht = 0; fh = 0; }
    void increment_hsc() { ht++; h += (ht >> 5); ht &= 31; h &= 1; }
    void increment_vs() {
        fv++;
        int fv_overflow = (fv >> 3);
        vt += fv_overflow;
        vt &= 31;
        if (vt == 30 && fv_overflow == 1) { v++; vt = 0; }
        fv &= 7;
        v &= 1;
    }
    uint32_t get_ntread() { return 0x2000 | (v << 0xB) | (h << 0xA) | (vt << 5) | ht; }
    uint32_t get_2007access() { return ((fv & 3) << 0xC) | (v << 0xB) | (h << 0xA) | (vt << 5) | ht; }
    uint32_t get_atread() { return 0x2000 | (v << 0xB) | (h << 0xA) | 0x3C0 | ((vt & 0x1C) << 1) | ((ht & 0x1C) >> 2); }
    uint32_t get_ptread() { return (s << 0xC) | (par << 0x4) | fv; }
    void increment2007(bool rendering, bool by32) {
        if (rendering) { increment_vs(); return; }
        if (by32) { vt++; }
        else { ht++; vt += (ht >> 5) & 1; }
        h += (vt >> 5);
        v += (h >> 1);
        fv += (v >> 1);
        ht &= 31; vt &= 31; h &= 1; v &= 1; fv &= 7;
    }
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

    // ---- Batch 1: PPU control register mirror state (plan §2.1) ----
    // The v1.0 PPU loop maintains a handful of static / global scratch
    // variables that mirror derived state of the PPU register file
    // (the current VRAM address, the toggle bit, fine X scroll, etc.).
    // These are now class members with reference-returning accessors
    // (no `const` qualifier) so the v1.0 global names can bind as
    // reference aliases. Callers that want read-only semantics should
    // capture-by-value or use static_cast — typical call sites do
    // `vtoggle ^= 1` or `XOffset = V & 7` which are write-through.
    __forceinline uint8_t&  vtoggle()        noexcept { return vtoggle_; }
    __forceinline uint8_t&  fine_x_scroll()  noexcept { return fine_x_scroll_; }
    __forceinline uint32_t& vaddr()          noexcept { return vaddr_; }
    __forceinline uint32_t& vaddr_latch()    noexcept { return vaddr_latch_; }
    __forceinline uint32_t& nt_refresh_addr() noexcept { return nt_refresh_addr_; }
    __forceinline uint32_t& dummy_read()     noexcept { return dummy_read_; }

    // ---- Batch 2 (plan §2.2): rendering line-buffer + BG pixel latches.
    // v1.0 used `static uint8 sprlinebuf[256+8]` (file-static) plus
    // `static uint32 pshift[2]` and `static uint32 atlatch` (static
    // inside RefreshLine). They migrate here so the line buffer sits
    // in a known-cache-aligned address and the BG latches sit next to
    // the other hot per-tile state in the class.
    //
    // line_buffer is `alignas(64)` and 264 bytes — per plan §2.2 risk
    // analysis, this 264-byte line buffer spans 5 cache lines on a
    // 64-byte L1. The original v1.0 storage was a BSS global which
    // landed on a random cache line; v1.5 forces alignment so MSVC
    // doesn't pack the buffer against adjacent cold data.
    //
    // `bg_latch()` and `bg_latch_h()` return references so callers
    // can write through (`pshift[0] |= C[0]` etc.). The aliases
    // `pshift` and `atlatch` declared at file scope in ppu.cpp are
    // local aliases pointing into g_ppu.bg_latch_[] / g_ppu.bg_latch_h_,
    // so pputile.inc's `pshift[0]`, `pshift[1]`, `atlatch` references
    // resolve unchanged.
    __forceinline uint8_t (& line_buffer() noexcept)[264] { return line_buffer_; }
    __forceinline uint32_t (& bg_latch() noexcept)[2]    { return bg_latch_; }
    __forceinline uint32_t & bg_latch_h() noexcept       { return bg_latch_h_; }

    // ---- Batch 3 (plan §2.3): OAM (256 bytes) ----
    // v1.0 used `extern uint8 SPRAM[0x100]` — file-scope BSS global
    // referenced from ppu.cpp, debug.cpp, and Qt UI tools (HexEditor,
    // PPU viewer). Migrates here as oam_[256] (alias name retained
    // for the public-facing UI / debugger surface; v1.0 savestate
    // chunk "SPRA" continues to memcpy these 256 bytes — see
    // state.cpp SFORMAT descriptor which references `SPRAM`).
    __forceinline uint8_t (& oam() noexcept)[256] { return oam_; }

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
    // Storage layout (revised after Phase C bench regression — see
    // git log). Hot per-cycle control state is grouped at the start of
    // the struct (after the NTARAM block) so the PPU loop's frequent
    // vtoggle_ / vaddr_ / vaddr_latch_ / XOffset_ accesses land in
    // the same ~64-byte cache line as regs_ / vnapage_, instead of
    // being scattered across 2 KB of NTARAM BSS. Phase B had these
    // fields AFTER phase_; that layout forced a 2 KB cache-line walk
    // before reaching them, which showed up as +15% on bench_ppu_frame
    // vs the Phase B baseline.
    alignas(64) uint8_t ntaram_[0x800];    // name-table RAM (2 KB)
    uint8_t* vnapage_[4];                  // 4-entry pointer table (32 B)
    uint8_t  regs_[4] = {};                // PPU[0..3] register file (4 B)
    uint8_t  chr_ram_mask_ = 0;            // PPUCHRRAM flag (1 B)
    uint8_t  nt_ram_mask_  = 0;            // PPUNTARAM flag (1 B)
    // ---- Batch 1 hot control state (plan §2.1) ----
    // v1.0 used file-scope `static` / global variables here (`vtoggle`,
    // `XOffset`, `TempAddr`, `RefreshAddr`, `NTRefreshAddr`,
    // `DummyRead`); they migrate into the class so the link-time
    // layout can fold the accessor calls. v1.0 global names are
    // reference aliases in the extern block below. Types preserved:
    // vtoggle stays uint8_t (v1.0's `vtoggle ^= 1` toggle idiom works
    // on both); TempAddr / RefreshAddr / NTRefreshAddr stay uint32_t
    // (v1.0 stores flags in the upper bits alongside the 14-bit
    // address; narrowing to 16 bits would break the bit extracts at
    // ppu.cpp:584-587 / 770 / 790 / 810 / 830 / 856).
    uint8_t  vtoggle_         = 0;   // PPU read-toggle (was ppu.cpp:368)
    uint8_t  fine_x_scroll_   = 0;   // $2005 fine X (was ppu.cpp:369 XOffset)
    uint32_t vaddr_           = 0;   // current VRAM address (was TempAddr)
    uint32_t vaddr_latch_     = 0;   // reload value (was RefreshAddr)
    uint32_t nt_refresh_addr_ = 0;   // MMC5 sync (was NTRefreshAddr)
    uint32_t dummy_read_      = 0;   // $2007 dummy read state (was DummyRead)
    // ---- Cold / phase state ----
    PPUPHASE phase_ = PPUPHASE_VBL;        // frame phase (cold, touched once per scanline)

    // ---- Batch 2 (plan §2.2) render-buffer + BG latch state. ----
    // line_buffer_ is the per-scanline final pixel buffer the PPU
    // writes during RefreshLine and the consumers (XBuf writeback,
    // sprite 0 hit check, InputScanlineHook) read. The struct
    // overall is alignas(64) (via FCEUX11_CACHE_ALIGN on the class),
    // so line_buffer_ sits at some offset within g_ppu whose
    // absolute address is a multiple of 64 only if the cumulative
    // preceding-member size is a multiple of 64 — currently not, but
    // perf impact is small vs the original v1.0 BSS-global layout
    // (where the buffer landed at an arbitrary 64-byte boundary).
    // Plan §2.2 risk: 264 bytes span 5 cache lines. If perf regresses
    // we can reorganize / add explicit padding here.
    uint8_t  line_buffer_[264] = {};             // was ppu.cpp:1064 sprlinebuf[256+8]
    uint32_t bg_latch_[2]      = {0, 0};         // was RefreshLine static pshift[2]
    uint32_t bg_latch_h_       = 0;              // was RefreshLine static atlatch

    // ---- Batch 3 (plan §2.3) OAM. ----
    // Plan §2.3 also lists Spr_Pri[8] / Spr_Index[8] / sprite_0_hit_
    // / max_sprites_; none of these exist as separate globals in v1.0
    // (sprite priority rides inside SPRBUF during eval, sprite 0 hit
    // is encoded in PPU[2] bit 6, max sprites is the constant 8).
    // We migrate only SPRAM (the on-chip OAM that the CPU reads at
    // $2004 / writes at $2004 / DMAs via $4014) and leave the rest
    // for a hypothetical v1.5+ sprite-pipeline rewrite.
    uint8_t  oam_[256]         = {};             // was ppu.cpp SPRAM[0x100]
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

// ---------------------------------------------------------------------------
// Batch 1 (plan §2.1) compat aliases — control-register mirror state.
//
// These v1.0 global names (vtoggle, XOffset, TempAddr, RefreshAddr,
// NTRefreshAddr, DummyRead) are migrated into fceu11::g_ppu. External
// consumers (debug.cpp uses XOffset; mmc5.cpp uses NTRefreshAddr;
// state.cpp savestate uses &vtoggle / &XOffset / &TempAddr / &
// RefreshAddr / &NTRefreshAddr / &DummyRead via the SFORMAT descriptor)
// still see the v1.0 names bound as references to g_ppu's storage —
// so `&vtoggle` and `&NTRefreshAddr` resolve to the same address
// before and after the migration, and savestate chunks stay
// byte-exact identical.
//
// Pure-internal-to-ppu.cpp names (TempAddr, RefreshAddr, DummyRead)
// are also aliased because pputile.inc (included into ppu.cpp) reads
// RefreshAddr; the alias lets the include-file stay unchanged.
// ---------------------------------------------------------------------------

extern uint8_t  (& vtoggle);
extern uint8_t  (& XOffset);
extern uint32_t (& TempAddr);
extern uint32_t (& RefreshAddr);
extern uint32_t (& NTRefreshAddr);
extern uint32_t (& DummyRead);

// ---------------------------------------------------------------------------
// Batch 3 (plan §2.3) compat alias — OAM.
//
// SPRAM is the on-chip OAM accessed by the CPU at $2004 / DMAd via
// $4014. Migrated into fceu11::g_ppu.oam_. External consumers (Qt
// debugger HexEditor / PPU viewer / debugger.cpp) keep referencing
// the v1.0 `SPRAM` name through this reference alias; the v1.0
// savestate chunk 'SPRA' continues to memcpy the same 256 bytes via
// the existing state.cpp SFORMAT descriptor (which points at &SPRAM
// — now resolves to g_ppu.oam_'s address).
// ---------------------------------------------------------------------------

extern uint8_t (& SPRAM)[0x100];

#endif // FCEU11_PPU_CLASS_H