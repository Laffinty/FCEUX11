# FCEUX11 Internal Build History

> This file consolidates completed build plans and handoff documents from
> v1.4 through v1.8. The original files have been removed to avoid
> confusion for AI agents working on current phases.
>
> **Active documents** (kept in `docs/internal/`):
> - `v1.8_masonry_build_plan.md` — v1.8 Masonry master plan (archived)
> - `v1.8_mapper_subclass_audit.md` — mapper subclass audit
> - `v1.8_mapper_byte_diff_design.md` — byte-diff test design

---

## v1.4 Gateway (2026-06-22)

**Scope**: Bus class extraction, PPU/APU decoupling.

### Key Decisions
- Extracted `fceu11::Bus` from global state
- PPU/APU reference-alias pattern (`extern T (&NAME) = g_ppu.X`)
- Call site audit: 1,200+ global references catalogued

### Post-Release Optimization
- Identified layout shift risks from new class introductions
- Established bench_tolerance_test baseline (+2.5% max regression)

---

## v1.5 Prism (2026-06-24)

**Scope**: PPU class extraction, rendering pipeline modernization.

### Key Decisions
- `fceu11::Ppu` class with cache-line alignment
- Sprite pipeline refactoring (deferred sprite priority to v1.14)
- Savestate layout audit for PPU chunk

---

## v1.6 Resonance (2026-06-27)

**Scope**: APU class extraction, ExpansionAudio interface, VRC6 PoC.

### Key Decisions
- `fceu11::Apu` class with `ExpansionAudio*` slot
- `EXPSOUND` adapter struct for legacy function pointers
- VRC6 PoC: first `install_expansion_audio` implementation
- `g_vrc6_audio` static instance as ExpansionAudio subclass

### Known Issues (carried to v1.7/v1.8)
- bench_full_frame +3.63% from VRC6 unreachable assignment
- 5 expansion audio mappers deferred to v1.8 Phase G

---

## v1.7 Cartograph (2026-06-28)

**Scope**: Cart/Mapper class extraction, CartInfo migration, 4 PoC subclasses.

### Key Decisions
- `fceu11::Cart` abstract base with `on_power/on_reset/on_close`
- `fceu11::Mapper : Cart` thin base with `attach_bus(Bus&)`
- `MirrorMode` enum class replacing MI_H/MI_V/MI_0/MI_1 macros
- 4 PoC subclasses: NromCart, Mmc1Cart, Mmc3Cart, Vrc6Cart
- CartInfo dual-write for backward compatibility

### Phase D Handoff
- Cart class lifecycle verified (on_save_pre/on_load_post)
- Savestate hooks wired through CartInfo forwarders

### Phase F Handoff
- Vrc6Cart install_expansion_audio verified
- bench_tolerance_test +4.37% (CPU frame, carryover from Phase B/C)

### Phase G Handoff
- 166 un-migrated boards identified
- PoC on_close() WRAM/CHRRAM leak documented
- MMC3 variant list (19 mappers) catalogued
- save_mapper_state() API design started

---

## v1.8 Masonry Phase D (2026-06-29)

**Scope**: save_mapper_state() API, 4 golden files, pack helpers.

### Key Decisions
- `std::vector<uint8_t> save_mapper_state() const noexcept` API
- Binary format: FMAP magic + u32 version + u32 body_size + body
- Pack helpers: pack_u8/u16/u32 + array variants
- 4 golden files: nrom, mmc1, mmc3, vrc6

### Phase D Handoff
- Mmc3BaseCart derived for 23 MMC3 variants
- cart_class_test expanded to 85 assertions

---

## v1.8 Masonry Phase E.1 (2026-06-29)

**Scope**: Strategy A lifecycle fix, volatile keepalive for static registration.

### Key Decisions
- Meyers-singleton registry_storage() for MapperEntry array
- volatile g_keepalive[] to prevent DCE of static initializers
- Mapper 406 special fallback in find_mapper()

### Phase E.1 Handoff
- 21/21 ctest PASS after lifecycle fix
- 4 SEGFAULTs eliminated from static initialization order

---

## Refactor Plans (archived)

### R1–R5 Completion (2026-06-27)
- R1: string utility O(n²)→O(n) optimization
- R2: mass_replace deduplication
- R3: memory.h RAII cleanup
- R4: driver.h interface consolidation
- R5: state.h SFORMAT modernization

### Global State Audit (2026-06-18)
- Catalogued all global variables across 171 board files
- Identified WRAM/CHRRAM ownership patterns
- Mapped IRQ hook lifecycle

### Savestate Layout Audit (2026-06-23)
- Documented SFORMAT chunk structure
- Identified endianness and alignment issues
- Mapped mapper-specific state formats
