# FCEUX11 Internal Build History

> This file consolidates completed build plans and handoff documents from
> v1.4 through v1.11. The original individual files have been extracted,
> merged into this document, and removed to avoid confusion for AI agents
> working on current phases.
>
> **Active documents** (kept in `docs/internal/`):
> - `v1.12_scissors_build_plan.md` — v1.12 Scissors build plan (active)

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

## v1.8 Masonry (2026-06-29 ~ 2026-07-01, tag `v1.8`)

**Scope**: Bulk subclassify 166 unmigrated board files into `fceu11::Mapper`; introduce `MapperEntry` auto-registry; 5 expansion audio mapper subclasses (VRC7/MMC5/N106/69/FDS); split `mapinc.h`; `Cart::save_mapper_state()` API; `FceuMallocPtr` RAII; `mapper_byte_diff_test` body byte-diff.

### 0. Startup Inventory
- Cpu/Bus/Ppu/Apu/Cart/Mapper classes: complete (v1.3-v1.7)
- 4 PoC cart subclasses (NROM/MMC1/MMC3/VRC6): complete
- 165 board files on legacy `currCartInfo` function-pointer path
- 5 expansion audio mappers unsublassed (v1.6 carry-over)
- `MapperEntry` auto-registry: missing
- `mapinc.h` monolithic include (12 headers): pending split
- `FceuMallocPtr` RAII: ~10 files adopted, ~160 still raw malloc/free

### Phase A: Planning & Audit
- `v1.8_masonry_build_plan.md` drafted (deps on Roadmap §8)
- `v1.8_mapper_subclass_audit.md` with full bmap[] audit

**bmap[] census** (src/ines.cpp:320):
- 235 active entries (222 iNES + 13 UNIF-style)
- 36 commented entries (iNES 2.0 placeholders)
- 4 v1.7 PoC already done, leaving ~231 entries across 168 .cpp files

**Mapper P0/P1/P2 priority classification** (by ROM popularity):

| Priority | Mapper range | Count | Complexity | Phase |
|----------|-------------|-------|------------|-------|
| P0 | 0-50 + MMC3 variants | 60 | ≤ 500 lines | D |
| P1 | 5-189 + bootleg | 79 | ≤ 1000 lines | E |
| P2 | 100-255 + UNIF-style | 42 | varies | F |

**Complexity scoring** (3-axis: Register + Bank-switch + ExpansionAudio):
- 3-4 (simple): 47% (NROM, UNROM, CNROM)
- 5-6 (medium): 35% (MMC1, VRC2-4, MMC3 variants)
- 7+ (complex): 18% (MMC3, MMC5, VRC7, Namco163, FDS, COOLGIRL)

**Largest files**: coolgirl.cpp (2355 lines), mmc5.cpp (1092), datalatch.cpp (626), bandai.cpp (612)

### C++11 Magic Statics Verification
- `kMapperRegistry[256]` as Meyers singleton: thread-safe on GCC 7+, Clang 5+, MSVC 19.20+
- `find_mapper()` triggers registry construction on first call
- `kXxxRegister` static instances fill entries lazily

### Phase B: mapinc.h Split
- Split `mapinc.h` (12 monolithic includes) into 5 targeted headers:
  - `mapinc_base.h` — types.h, memory.h, basic macros
  - `mapinc_bus.h` — x6502.h, cart.h (bank-switching)
  - `mapinc_state.h` — state.h (SFORMAT)
  - `mapinc_audio.h` — sound.h (expansion audio)
  - `mapinc_mmc3.h` — mmc3.h shared state
- 171 board files updated to include-only-what-you-use
- Estimated -20% compile time improvement
- Single commit to avoid layout-shift snowball

### Phase C: MapperEntry Registry
- `src/boards/registry.h` / `registry.cpp` introduced
- `struct MapperEntry { mapper_number, name, legacy_init, factory }`
- `find_mapper(number)` → lookup in static array
- `find_mapper_by_name(name)` → O(N) linear scan for UNIF
- 4 PoC boards register first; `create_cart_for_mapper()` switch replaced
- `bmap[]` retained for UNIF loader (v1.9 to remove)
- Bench verification: single-batch commit, 7x bench median before/after

### Phase D: Board Batch 1 + save_mapper_state API

**Cart::save_mapper_state() API**:
```cpp
virtual std::vector<uint8_t> save_mapper_state() const noexcept;
virtual bool load_mapper_state(const std::vector<uint8_t>& state) noexcept;
```

**Binary format** (little-endian):
```
Offset  Size  Field
0       8     magic = "FMAP\0\0\0"
8       4     version = 1 (uint32 LE)
12      4     mapper_number (uint32 LE)
16      4     body_size (uint32 LE, excludes 16B header)
20      ...   body bytes (Cart::save_mapper_state() output)
```

**Pack helpers** (in `cart_class.h`, all `inline noexcept`):
- `pack_u8/u16/u32/u64` — scalar LE packing
- `pack_i8/i32` — signed LE
- `pack_array<T,N>` — fixed-size array
- `pack_bytes(data, len)` — variable-length byte array

**Per-mapper body sizes**: NROM 32B, UNROM 32B, CNROM 24B, MMC1 128B, MMC3 256B, VRC6 96B, VRC7 192B, MMC5 384B, Namco163 256B, FDS 320B, Sunsoft-5B 192B, coolgirl ~512B

**Golden files**: 169 .bin files at `tests/fixtures/golden_mapper/`, shared-body deduplication (e.g., 19 MMC3 variants share `mmc3.bin`)

**API redesign**: v1.7 used `size_t(void*, size_t)`; v1.8 changed to `std::vector<uint8_t>` for automatic buffer management and zero-caller-burden semantics.

**Unified on_close() release** (`_cart_helpers.h`):
- `release_mapper_resources(mapper_no)`: dispatch table for 256 entries
- `GameHBIRQHook = nullptr` always executed (leak-proof)
- Fixes v1.7 PoC WRAM/CHRRAM leak (8 KiB per game load)

**4 golden files**: nrom, mmc1, mmc3, vrc6

### Phase E: Board Batch 2 + MMC3 Variants
- Mmc3BaseCart derived class for 19 MMC3 variants (Mappers 12/37/44/45/47/49/52/74/114/115/116/118/119/165/205/245/249/250/254/406)
- Each variant: 1-line .h + dispatch through `kMmc3PowerFuncs[256]`
- cart_class_test expanded to 85+ assertions

### Phase F: Board Batch 3 (remaining 66 + UNIF-style)
- coolgirl.cpp (2355 lines) as final commit, separate bench verification
- UNIF-style entries (13) co-migrated with P2

### Phase G: ExpansionAudio 5 Subclasses
- Vrc7Cart, Mmc5Cart, N106Cart, Mapper69Cart, FdsCart
- `install_expansion_audio` overrides inject `ExpansionAudio*` into `g_apu`
- `apu_wav_diff_test` expanded: 4 → 8 baselines (nrom/mmc1/vrc6/mmc5/vrc7/namco163/sunsoft5b/fds)
- Golden WAV files: ~500 KiB each, byte-exact comparison

### Phase H: FceuMallocPtr RAII + Doc Refresh + Tag
- 160+ board files: mechanical `malloc()/free()` → `FceuMallocPtr` / `FceuMallocPtr_array`
- Zero raw `malloc()/free()` in `src/boards/`
- Docs: readme.md, BuildGuide.md, Roadmap §8 all updated
- Tag `v1.8` annotated, CHANGELOG entry

### Layout-Shift Mitigation (v1.6/v1.7 lessons)
- MapperEntry introduced as single batch (Phase C), verified before use
- Board subclassification in 3 batched commits (D/E/F)
- mapinc.h split as standalone Phase B (before board migration)
- FceuMallocPtr RAII deferred to Phase H (final, avoids struct-layout churn)
- Bench verification: 3x median per phase, +1.5% threshold → revert
- v1.8 cumulative target: ≤ +5% (vs v1.7 +4.37% advisory)

### Test Expansion Targets
- cart_class_test: 12 → 200+ tests (166 mapper factory/dispatch + round-trip)
- apu_wav_diff_test: 4 → 8 baselines
- mapper_byte_diff_test: 3 → 169 baselines (body byte-diff enabled)
- 166 per-mapper load+save+reset trinity tests

### Phase D Handoff
- Mmc3BaseCart derived for 23 MMC3 variants
- cart_class_test expanded to 85 assertions

### Phase E.1 Handoff
- Strategy A lifecycle fix, volatile keepalive for static registration
- Meyers-singleton registry_storage() for MapperEntry array
- volatile g_keepalive[] to prevent DCE of static initializers
- Mapper 406 special fallback in find_mapper()
- 21/21 ctest PASS after lifecycle fix, 4 SEGFAULTs eliminated

### Total Estimate
~9-11 weeks for full v1.8 delivery

---

## v1.9 Chronicle (2026-07-01 ~ 2026-07-03, tag `v1.9`)

**Scope**: Savestate V2 format (Roadmap §9). Interfaces delivered to v1.10: `Cart::save_mapper_state()` / `load_mapper_state()` for sub-class-granular serialization; `MapperEntry::legacy_init` retained for backward compat with v1.0 savestates. `bmap[]` marked `[[deprecated]]` for v2.0 removal.

---

## v1.10 Cryptex (2026-07-03 ~ 2026-07-04, tag `v1.10`, commit `07f0126`)

**Scope**: ROM parsing in Rust (`dr/` workspace), FDS BIOS `.dr/` isolation. Bench protocol established (best-of-3, sequential execution, shared vcpkg link). Baselines recorded:
- bench_cpu_frame: 65.680 ms (nestest.nes)
- bench_ppu_frame: 66.634 ms (mapper_nrom.nes)
- bench_full_frame: 68.685 ms (mapper_mmc3.nes)

---

## v1.11 Bridge (2026-07-04 ~ 2026-07-05, tag `v1.11`)

**Scope**: Eliminate core-to-driver reverse dependencies; establish formal `fceu11::DriverCallbacks` interface; migrate `FCEUD_*` free-function callbacks to interface members; remove 86 `#ifdef` blocks from 12 core source files; split `src/drivers/Qt/fceuWrapper.cpp` (1907 lines) into 5 single-responsibility files; pImpl-isolate Qt types from `src/utils/mutex.*`.

### 0. Startup Inventory
- Cpu/Bus/Ppu/Apu/Cart/Mapper classes: complete (v1.3-v1.8)
- `src/driver.h` shim: 32 `#include "driver.h"` sites in core
- 4 peer API headers (core_api/io_api/net_api/diag_api.h): 41 live `FCEUD_*` declarations
- 5 dead `FCEUD_*` declarations pending deletion
- `fceuWrapper.cpp`: 1907 lines (had grown from Roadmap estimate of 1637)
- Core source files: 86 driver `#ifdef` blocks across 12 .cpp + 1 .h + 1 test
- `src/utils/mutex.h`: Qt type leakage via `QRecursiveMutex` member
- `__WIN_DRIVER__` macro: dead (no build config defines it, 59 dead-code blocks)
- `__SDL__`: redundant (always co-defined with `__QT_DRIVER__`)

### Key Design: DriverCallbacks Interface
```cpp
struct DriverCallbacks {
    // All fields are function pointers (POD, no vtable, memset-zeroable)
    // g_driver() accessor: __forceinline DriverCallbacks&
    // Unregistered state: all-nullptr static instance
    
    // Messages & lifecycle (Batch 1, 7 callbacks)
    void (*print_error)(const char*) = nullptr;
    void (*message)(const char*) = nullptr;
    // ... 41 total fields covering all live FCEUD_* callbacks
    
    // UI refresh / dialogs (new abstractions)
    void (*set_main_window_text)(const char*) = nullptr;
    void (*update_ram_search)() = nullptr;
    void (*update_cheat_list)() = nullptr;
    int  (*message_box)(const char*, const char*, int) = nullptr;
    void (*emu_command)(int) = nullptr;
    
    // Phase A additions (from audit)
    void (*get_keyboard_state)(void*) = nullptr;
    void (*taseditor_disable_run_function)() = nullptr;
    const char* (*get_thread_name)() = nullptr;
};

static_assert(std::is_trivially_copyable_v<DriverCallbacks>);
```

### Phase A Audit Results (v1.11_driver_callbacks_audit.md)

**FCEUD_* census**: 57 unique symbols
- 40 unique live (41 with function-signature counting for OpenArchive/OpenArchiveIndex overloads)
- 5 dead (CmdOpen, OnCloseGame, LuaRunFrom, BlitScreen, BlitScreenDummy)
- 12 Qt-private (Update, UpdateInput, NetworkConnect, SoundIsMuted, MuteSoundOutput/Window, TraceLogger×4, AviGet×2) — out of scope

**Driver #ifdef block map** (83 macro blocks + 1 test include):

| File | Blocks | Dominant macro | Phase |
|------|--------|---------------|-------|
| src/input.cpp | 30 | __WIN_DRIVER__ | F |
| src/fceu.cpp | 20 | 17 WIN + 3 QT | C |
| src/movie.cpp | 11 | WIN/QT | D |
| src/lua-engine.cpp | 5 | WIN/SDL/QT | E |
| src/state.cpp | 3 | WIN | D |
| src/utils/mutex.cpp | 3 | QT | H |
| src/video.cpp | 2 | WIN | E |
| src/wave.cpp | 2 | WIN | E |
| src/profiler.cpp | 2 | QT | E |
| src/utils/mutex.h | 2 | QT | H |
| src/debugsymboltable.cpp | 1 | QT | E |
| src/version.h | 1 | !__QT_DRIVER__ | E |
| src/utils/xstring.cpp | 1 | WIN | E |

**Key finding**: `__WIN_DRIVER__` was never defined in any build configuration — 59 blocks (30 input + 17 fceu + 8 movie + 3 state + 1 video) are dead code. Safe to delete with `#else` branch preservation.

**v1.10 tag**: Confirmed annotated tag at commit `07f0126`. No re-tagging needed.

### Migration Batches

| Batch | Phase | Functions | Scope |
|-------|-------|-----------|-------|
| 1 | C | 7 | PrintError, Message, GetCompilerString, ShowStatusIcon, ToggleStatusIcon, HideMenuToggle, PauseAfterPlayback |
| 7 | C | 4 | SetEmulationSpeed, TurboOn, TurboOff, TurboToggle |
| 2 | D | 6 | UTF8fopen, UTF8_fstream, ScanArchive, OpenArchive(×2), OpenArchiveIndex(×2) |
| 8 | D | 6 | AviRecordTo, AviStop, SaveStateAs, LoadStateFrom, MovieRecordTo, MovieReplayFrom |
| 3 | E | 6 | SetPalette, GetPalette, VideoChanged, ShouldDrawInputAids, GetTime, GetTimeFreq |
| 4 | E | 2 | SoundToggle, SoundVolumeAdjust |
| 5 | E | 1 | SetInput |
| 6 | E | 5 | DebugBreakpoint, TraceInstruction, FlushTrace, UpdateNTView, UpdatePPUView |
| 9 | F | 4 | SendData, RecvData, NetplayText, NetworkClose |
| — | B | 5 | (deletion) CmdOpen, OnCloseGame, LuaRunFrom, BlitScreen, BlitScreenDummy |

### Phase C: Messages/Lifecycle + fceu.cpp
- 10 callbacks migrated to DriverCallbacks
- Qt driver registers implementations via `register_driver()`
- `FCEUD_*` free functions become forwarding shims
- fceu.cpp 20 blocks: driver includes deleted, UI refresh → `g_driver()->fn()`, MessageBox → callback
- `/FAcs` assembly verification: `g_driver()` __forceinline must produce direct call, not indirect chain

### Phase D: File I/O + movie.cpp/state.cpp
- 12 callbacks: UTF8fopen (17 core call sites), archive (9 sites), file dialogs
- movie.cpp 11 blocks: SetMainWindowText abstraction, MessageBox, TASEDITOR unified path
- state.cpp 3 blocks: driver includes, Update_RAM_Search, MessageBox

### Phase E: Video/Audio/Input/Debug + Small Files
- 14 callbacks + 8 small files
- lua-engine.cpp: `taseditor_lua` type mismatch (Win value vs Qt pointer) → unified callback
- profiler.cpp: QThread::currentThread() → `g_driver()->get_thread_name()`
- FCEUD_GetTime/GetTimeFreq stray forward-decls moved from video.cpp to driver_callbacks.h

### Phase F: Netplay + input.cpp
- 4 netplay callbacks
- input.cpp 30 blocks: all `__WIN_DRIVER__` dead code; retained-command paths → `g_driver()->emu_command()`

### Phase G: fceuWrapper.cpp Split
- 1907 lines → 5 files + ≤100-line compat shell:

| New File | Lines | Responsibility |
|----------|-------|---------------|
| fceu_bridge.cpp | ~480 | Core lifecycle bridge (LoadGame/CloseGame/reset/pause/mutex) + DriverInitialize/DriverKill |
| fceu_globals.cpp | ~70 | Core global variable definitions (dendy/eoptions/isloaded/pal_emulation/gametype/...) |
| driver_callbacks.cpp | ~420 | DriverCallbacks registration + FCEUD_* tool callbacks + forwarding |
| fceu_config.cpp | ~520 | Config sync (fceuWrapperInit 420-line body + ShowUsage + DriverUsage) |
| fceu_archive.cpp | ~440 | Archive subsystem (minizip + libarchive backends) |
| fceuWrapper.cpp | ≤100 | Include + forward-declare stub (compat entry) |

- Total new files = 1907 ±5% (anti-fake-migration gate: §0.6)
- fceu_config.cpp 520 lines accepted as deviation (fceuWrapperInit single-function scale)

### Phase H: mutex pImpl + driver.h Removal + Tag
- `src/utils/mutex.h`: `QRecursiveMutex` member → `void* impl_` opaque pointer
- `mutex.cpp` internal Qt includes isolated to single TU
- `src/driver.h` shim deleted; 32 core includes changed to direct peer headers
- `__WIN_DRIVER__`/`__SDL__` removed from CMakeLists.txt/.vcxproj
- Core boundary test: grep-asserts zero driver `#ifdef`, zero driver-specific includes
- Tag `v1.11` annotated

### Performance Budget
- v1.10 baseline: bench_full_frame 68.685 ms
- Layout-shift risks: new TU additions + indirect call (mitigated by `__forceinline`)
- Per-phase gate: 3x bench median, +1.5% threshold → revert
- Cumulative target: < +5% (advisory if exceeded)
- dry-run for Phase G (fceuWrapper split) to validate layout-shift magnitude

### Total Estimate
~6-7 weeks for full v1.11 delivery

---

## DLL Decoupling Analysis (v1.11, 2026-07-05)

**Context**: Post-v1.11 release packaging audit. Independent of source code changes — operates on the `发布打包/` distribution directory only.

### Method
- `dumpbin /DEPENDENTS` import-table tracing on all .dll/.exe in package directory
- Source-code audit of `LoadLibrary`/`QLibrary`/`QPluginLoader` + CMakeLists.txt linkage

### Dependency Graph
```
fceux11.exe directly imports:
├── Qt6Core.dll ──→ icuin78.dll → icuuc78.dll → icudt78.dll
│                → pcre2-16.dll, double-conversion.dll, z.dll, zstd.dll
├── Qt6Gui.dll ──→ freetype.dll → z.dll, bz2.dll, libpng16.dll, brotlidec.dll
│               → harfbuzz.dll → freetype.dll
│               → libpng16.dll → z.dll, md4c.dll, z.dll
├── Qt6Widgets.dll ──→ Qt6Gui.dll, Qt6Core.dll
├── Qt6OpenGL.dll ──→ Qt6Gui.dll, Qt6Core.dll
├── Qt6OpenGLWidgets.dll → Qt6OpenGL.dll, Qt6Widgets.dll, Qt6Gui.dll, Qt6Core.dll
├── SDL2.dll ──→ [system DLLs only]
├── archive.dll ──→ z.dll, bz2.dll, liblzma.dll, libcrypto-3-x64.dll, lz4.dll, zstd.dll
└── z.dll ──→ [system DLLs only]

=== Orphaned sub-graph (no path from main chain) ===

Qt6Network.dll ──→ Qt6Core.dll, brotlidec.dll, zstd.dll, z.dll, libcrypto-3-x64.dll
    ↑
qschannelbackend.dll ──→ Qt6Network.dll, Qt6Core.dll
qopensslbackend.dll ──→ Qt6Network.dll, Qt6Core.dll, libssl-3-x64.dll, libcrypto-3-x64.dll
qcertonlybackend.dll ──→ Qt6Network.dll, Qt6Core.dll

pcre2-8.dll ──→ [system DLLs only]   ← ZERO importers in entire directory
```

### DLL Classification

**Safe to delete (orphaned graph, 6 files, ~3.5 MB)**:
| DLL | Size | Reason |
|-----|------|--------|
| Qt6Network.dll | 1,587 KB | Not imported by fceux11.exe or any Qt module in main chain |
| plugins/tls/qschannelbackend.dll | 228 KB | TLS plugin, no HTTPS usage |
| plugins/tls/qopensslbackend.dll | 230 KB | TLS plugin + libssl |
| plugins/tls/qcertonlybackend.dll | 84 KB | TLS plugin |
| libssl-3-x64.dll | 858 KB | Only imported by qopensslbackend |
| pcre2-8.dll | 599 KB | Absolute orphan — zero importers |

**Needs vcpkg rebuild to remove (3 items, ~1.1 MB)**:
- bz2.dll: imported by archive.dll + freetype.dll
- brotlidec.dll + brotlicommon.dll: imported by freetype.dll (WOFF2 font support)
- lz4.dll: imported by archive.dll

**Not removaable**:
- zstd.dll (Qt6Core hard dependency)
- libcrypto-3-x64.dll (archive.dll imports for hash verification)

**Source-code audit confirmation**: Zero references to `Qt6Network`, `QNetwork`, `QNetworkAccessManager`, `QSslSocket`, `pcre2-8`, `libssl`, `openssl`, or `Qt6::Network` in CMakeLists.txt anywhere in the codebase.

### Execution Protocol
1. Backup all 6 files to `_dll_backup/` directory
2. Move (not delete) to `_removed_test/` for testing
3. Smoke test: start → open .nes → open .zip → menu → sound → input → OpenGL
4. Confirm with Process Explorer: Qt6Network.dll not loaded
5. After verification: delete backup and temp directories
6. Rollback: copy files back from backup

### Interface Contract
- Delivers to v1.14 Anvil §14.4 ("build artifact slim-down"): 6 files/3.5 MB removed; bz2/brotli/lz4 candidates for vcpkg overlay rebuild
- Orthogonal to v1.12 Scissors (source file split, no packaging change)

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
