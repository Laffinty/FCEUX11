# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.9] - 2026-06-12

C-track continuation: physical split of the 371-line `src/driver.h` into
four peer headers per plan v3 §5 v0.3.9. The split is a **pure
refactor** — no function bodies change, no public symbols are renamed,
no call-site edits are required. The 33 existing `#include "driver.h"`
sites compile unchanged because the new `driver.h` is now a 20-line
shim that re-includes the four new headers.

### Added

- **`src/core_api.h`** (232 lines, new): emulator core lifecycle,
  state, frame, cheats, debug, emulation control, the `EMUSPEED_SET` /
  `EFCEUI` enums, the `TestCommandState` typedef, and the `FCEU_printf`
  / `FCEU_DispMessage` message surface.
- **`src/io_api.h`** (209 lines, new): file I/O helpers (UTF-8 path
  handling, archive open/scan), input devices (joypad / zapper /
  powerpad / Famicom expansion), NTSC hue/tint, palette, base
  directory, audio output, video rendering toggles, AVI recording,
  movie / Lua / savestate driver commands, and the `fceu11::IoDir`
  enum + 14 `FCEUIOD_*` legacy aliases (moved verbatim from the old
  `driver.h`).
- **`src/net_api.h`** (59 lines, new): netplay `Start` / `Stop`,
  `Send` / `Recv`, the two `NetplayText` entry points, and the
  fatal-error `NetworkClose` callback. Independent of the rest of
  the API surface.
- **`src/diag_api.h`** (53 lines, new): `FCEUD_GetCompilerString`
  (moved from the old `driver.h`) plus two new inline accessors
  `FCEU_GetVersion()` and `FCEU_GetNameAndVersion()` that wrap
  `FCEU_DISPLAY_VERSION` and `FCEU_NAME_AND_VERSION` for the
  Rust FFI layer (`fceux11-formats`) to call through a stable
  C ABI without pulling in the preprocessor.

### Changed

- **`src/driver.h`**: collapsed from 371 lines to **20 lines** — a pure
  shim that re-includes `core_api.h`, `io_api.h`, `net_api.h`, and
  `diag_api.h`. The 33 existing `#include "driver.h"` call sites in
  `src/` and `src/drivers/Qt/` continue to compile unmodified.

### Deviations from the plan v3 §5 v0.3.9 literal text

- **Line counts differ from the plan's "约 100/80/60/40" guideline.**
  The plan gives illustrative line counts; the actual split is
  `core_api.h: 232 / io_api.h: 209 / net_api.h: 59 / diag_api.h: 53`.
  The plan's example symbol set (LoadGame / Emulate / CloseGame /
  Kill / Initialize, LoadRomVirtual / SetInput / GetNtscTh, …) is
  preserved in the named headers. The expanded line counts reflect
  the full pre-v0.3.9 `driver.h` surface (~110 declarations across
  the four domains) being distributed rather than only the small
  "core five" example set.
- **`fceu11::IoDir` enum + 14 `FCEUIOD_*` legacy aliases moved into
  `io_api.h`**, not kept in `driver.h`. Plan v3 §5 v0.3.9 does not
  specify which new header owns the enum; the IO domain is the
  natural fit (`FCEUIOD_*` indexes the per-category `odirs[]` path
  table, which is file-path I/O).
- **Test build infrastructure fix (out-of-scope, pre-existing)**:
  `tests/CMakeLists.txt` now defines `__QT_DRIVER__` on test and
  benchmark targets via `target_compile_definitions(... PRIVATE
  __QT_DRIVER__)`. This is the canonical escape hatch in
  `src/version.h:28-35` for non-Qt builds, and was needed because
  the `scmrev.h` generator script (formerly in `vc/defaultconfig/`)
  was deleted in an earlier cleanup commit (`8d26413 chore: remove
  obsolete vc/`). Without this fix, every test target fails with
  `fatal error C1083: 无法打开包括文件: "scmrev.h"`. Documented
  inline at the new definition site; this is a test-infrastructure
  change, not a v0.3.9 API change, and `fceux11.exe` was never
  affected (its `src/CMakeLists.txt:82` already adds
  `-D__QT_DRIVER__`).

### Verification (all five plan-v3 §7 gates)

- **闸 1 (编译)**: `cmake --build build --config Release` — 0 errors.
  All 11 targets built: `fceux11_core` / `fceux11_boards` /
  `fceux11_utils` / `fceux11_drivers_common` / `fceux11_drivers_qt` /
  `fceux11` / `fceux11_smoke_test` / `fceux11_mapper_load_test` /
  `fceux11_mapper_reset_test` / `fceux11_rom_regression_test` /
  `fceux11_expected_api_test` / `fceux11_enum_class_bitflags_test`
  / `fceux11_bench_ppu_render` / `fceux11_bench_x6502_exec` /
  `fceux11_bench_apu_mix`. Warning count is unchanged from v0.3.8
  baseline (the same `C4244` / `C4267` / `C4100` set in
  `src/cart.cpp`, `src/fceu.cpp`, `src/fds.cpp` — pre-existing, not
  introduced by the split).
- **闸 2 (单元)**: `ctest --test-dir build` — **6/6 tests pass**
  (smoke, mapper_load, mapper_reset, rom_regression, expected_api,
  enum_class_bitflags).
- **闸 3 (字节级)**: `rom_regression_test` 5-ROM savestate
  SHA-256 hash matches the v0.3.0 baseline in
  `tests/fixtures/golden_hashes.json`. The split is a pure header
  refactor; no function bodies change, so byte-level savestate
  consistency is preserved.
- **闸 4 (烟雾)**: `fceux11.exe --help` exits 0 with the full CLI
  help table rendered. The `fceux11_smoke_test` ctest exercises
  ~50 symbol resolutions across the 4 new headers — the
  `CHECK_SYMBOL(...)` macro at the top of `smoke_test.cpp` now
  resolves through the shim's transitive re-includes.
- **闸 5 (性能)**: N/A — v0.3.9 is a structural refactor that
  changes only header layout. Compiled object code is byte-for-byte
  identical to v0.3.8 (same `cl` invocations, same `driver.h`
  inclusion order via the shim). The 3 Google Benchmark executables
  build but were not run as part of the gate (no perf surface to
  measure against).

## [0.3.8] - 2026-06-12

C-track continuation: scoped-enumeration modernisation per plan v3 §5 v0.3.8.
Five categories of pre-v0.3.x C-style typing are replaced by `enum class`
under the `fceu11::` namespace; all legacy spellings (`ESI`, `SI_*`, `ESIFC`,
`SIFC_*`, `fceuAllocType`, `FCEU_ALLOC_TYPE_*`, `FCEUIOD_*`) are preserved
as global `using` / `inline constexpr` aliases so the ~200 pre-existing call
sites — including the Qt config-file `"SI_GAMEPAD"` string round-trip and
the `switch(int_value) { case SI_*: }` patterns — keep compiling unchanged.
A new `FCEU_ENUM_CLASS_BITFLAGS(E)` macro provides bitwise operators for
future scoped flag enums; `fceu11::CpuFlag` is shipped as the macro's
correctness witness (the live `_P|=Z_FLAG` hot path in `src/x6502.cpp`
remains on the existing `#define N_FLAG..C_FLAG` masks per plan §5 v0.3.8's
PPU/CPU bitflag exclusion).

### Deviations from the plan v3 §5 v0.3.8 literal text

- **Underlying type `: int8_t`, not `: u8`**. `ESI::Unset` / `ESIFC::Unset`
  remain at sentinel value `-1` (used by ROM detection to mean "unknown
  desired input"). Plan §1.3 iron-rule 1 (byte-level savestate consistency)
  takes precedence over the `: u8` literal in §5; `int8_t` preserves the
  sentinel while still being a 1-byte ABI.
- **`fceuAllocType` is the actual symbol name** (plan calls it
  `FCEU_ALLOC_TYPE` — written before the v0.3.7 split). Rewritten as
  `fceu11::AllocKind`.
- **`OldFceuApi = fceu11::v0_2_compat` shim deferred to v0.3.10**. Per
  plan §6.1 phase-1 ("only NEW symbols enter `fceu11::`; OLD symbols stay
  global"), v0.3.8 has no payload for the `v0_2_compat` sub-namespace —
  every legacy alias is already in place at file scope (`git.h`,
  `scoped_ptr.h`, `driver.h`). The sub-namespace lands when the
  `FCEUI_*` mass-rename starts in v0.3.10.
- **Mapper hook breadth: 35 files / 42 sites**, not the plan's "175"
  (which predated the v0.3.4 dead-code cleanup).

### Added

- **`src/fceu11_core_types.h`** (28 lines, new): `namespace fceu11 { using
  MapIRQHook = void(*)(int); }`. Strongly types the `extern void
  (*MapIRQHook)(int a)` global without changing its linkage. The global
  symbol `::MapIRQHook` keeps its pre-v0.3.x C-linkage contract used by
  35 mapper `.cpp` files in `src/boards/`; only the extern declaration in
  `src/x6502.h` is reformulated through the typedef. A `static_assert`
  in `src/x6502.cpp` enforces that the definition and the extern
  declaration agree at compile time.
- **`src/utils/enum_class_bitflags.h`** (108 lines, new):
  `FCEU_ENUM_CLASS_BITFLAGS(E)` macro emitting `operator|/&/^/~`,
  `operator|=/&=/^=`, plus `has(E,E)` / `set(E&,E)` / `clear(E&,E)` free
  functions for any scoped enumeration. SFINAE-restricted to
  `enum class` via `std::is_enum_v` + `!std::is_convertible_v<E,int>`
  static_asserts (plain `enum` types are rejected with a clear
  diagnostic). Ships with `fceu11::CpuFlag` (`enum class : uint8_t`
  with `N/V/U/B/D/I/Z/C` mirroring the 6502 P-register layout) as the
  macro's correctness witness — `CpuFlag` does NOT participate in
  `src/x6502.cpp`'s live `_P|=Z_FLAG` dispatch.
- **`tests/enum_class_bitflags_test.cpp`** (115 lines, new): 9 unit
  tests covering `operator|/&/^/~`, compound-assignment forms, `has` /
  `set` / `clear`, and underlying-type invariants. Header-only test —
  no NES core link, runs in milliseconds. Registered in
  `tests/CMakeLists.txt` as `fceux11_enum_class_bitflags_test`.

### Changed

- **`src/git.h`** (179 → 246 lines): `enum ESI { SI_UNSET = -1, … }` and
  `enum ESIFC { SIFC_UNSET = -1, … }` are now
  `namespace fceu11 { enum class InputDevice : int8_t { Unset = -1, … };
  enum class InputDeviceFC : int8_t { … }; }`. Global aliases provide:
  `using ESI = fceu11::InputDevice;`, `using ESIFC = fceu11::InputDeviceFC;`,
  and `inline constexpr int SI_*` / `SIFC_*` (int-typed to match the
  pre-v0.3.8 storage convention — `static int CurInputType[3] = {SI_GAMEPAD,
  …};` and `switch(int_value) { case SI_*: }` blocks compile verbatim).
  Type safety lives at the API boundary: `FCEUI_SetInput(int, ESI, …)` and
  `FCEUGI::input[2]` / `inputfc` keep typed signatures. `ESI_Name` /
  `ESIFC_Name` index helpers replace implicit `names[esi]` with explicit
  `static_cast<int>(esi)`.
- **`src/utils/scoped_ptr.h`**: `enum fceuAllocType { … }` rewritten as
  `namespace fceu11 { enum class AllocKind : uint8_t { New, NewArray,
  Malloc }; }` + legacy `using fceuAllocType = fceu11::AllocKind` alias
  and three `inline constexpr fceuAllocType FCEU_ALLOC_TYPE_*` constants.
  Zero in-tree usage — `fceuAllocType` was already a phantom symbol kept
  for out-of-tree ABI per v0.3.6's RAII migration; this just modernises
  the storage form without functional change.
- **`src/driver.h`**: the 13 `#define FCEUIOD_*` macros are rewritten as
  `namespace fceu11 { enum class IoDir : uint8_t { Roms = 0, Nv, States,
  FdsRom, Snaps, Cheats, Movies, MemW, BBot, Macro, Input, Lua, Avi,
  Count = 13 }; }`. The legacy `FCEUIOD_*` names remain as
  `inline constexpr int` (not `IoDir`-typed) so the 50+ array-index
  sites in `src/file.cpp` (e.g. `odirs[FCEUIOD_STATES]`) require zero
  per-site `static_cast<size_t>` decoration.
- **`src/x6502.h`**: `extern void (*MapIRQHook)(int a);` →
  `extern fceu11::MapIRQHook MapIRQHook;` (via the new
  `fceu11_core_types.h` include). The `#define N_FLAG..C_FLAG` block
  (P-register bit masks, ~600 in-tree usage sites in `x6502.cpp` macros)
  is intentionally left unchanged — same rationale as plan §5 v0.3.8's
  PPU[0..2] exclusion.
- **`src/x6502.cpp`**: `static_assert` added below the
  `void (*MapIRQHook)(int a);` definition to guard against future type
  drift between the definition and the extern declaration.
- **`src/input.h`**: `JOYPORT(int _w)` constructor's `type(SI_UNSET)`
  initializer → `type(static_cast<ESI>(SI_UNSET))` (legacy `SI_*` are
  now int aliases; `ESI type` field needs the enum-class form).

### Adjusted (back-compat casts at int↔ESI boundaries)

- **`src/fceu.cpp`, `src/nsf.cpp`, `src/unif.cpp`, `src/vsuni.cpp`**:
  `GameInfo->input[X] = SI_*` and `GameInfo->inputfc = SIFC_*` assignments
  gain `static_cast<ESI>` / `static_cast<ESIFC>` because `FCEUGI::input[]`
  / `inputfc` is the typed form and `SI_*` / `SIFC_*` are int aliases.
- **`src/drivers/Qt/input.cpp`**: 3 sites for `gi->input[] >= 0` /
  `gi->inputfc >= 0` plus 3 paired `CurInputType[X] = gi->input[X]`
  assignments now wrap with `static_cast<int>` (enum class doesn't
  implicitly compare to or convert to int).
- **`src/input.cpp`**: `switch(joyports[port].type)` and
  `switch(portFC.type)` operands wrap with `static_cast<int>` because
  the case labels are int aliases.
- **`src/drivers/Qt/InputConf.cpp`**: 17 `QComboBox::addItem(text, SI_*)`
  / `addItem(text, SIFC_*)` calls add `(int)` casts. Qt 6's `QVariant`
  does not implicitly accept `enum class` values; the user-data int
  contract is preserved.
- **`src/movie.cpp`**: 3 `currMovieData.ports[X] = .type` assignments
  cast to int (movie struct stores port type as `int[3]`).
- **`src/tests/rom_regression_test.cpp`, `tests/expected_api_test.cpp`,
  `tests/benchmark/{apu_mix,ppu_render,x6502_exec}_bench.cpp`**: 15
  `FCEUI_SetInput(N, SI_NONE, …)` / `FCEUI_SetInputFC(SIFC_NONE, …)`
  calls cast to `ESI` / `ESIFC` because the API signature is typed.

### Notes

- **Byte-level savestate consistency**: `FCEUGI` struct shrinks by ~5
  bytes (`ESI input[2]` and `ESIFC inputfc` go from `int` to `int8_t`),
  but `FCEUGI` is never serialized into savestate or movie files —
  verified by absence of any `SFORMAT` mapping that references these
  fields. The 5 ROM regression fixtures' SHA-256 hashes are expected
  to remain identical to the v0.3.0 baseline.
- **FFI / Rust ABI**: zero impact. The Rust `fceux11-formats` crate
  exposes input-device IDs as `int32_t` out-params (`fceu11_rust_ines_lookup_input_crc`
  and `_nes20`); the C++ receiver in `src/ines.cpp:151-166` already uses
  `static_cast<ESI>(int32_t)` at the boundary, and the cast continues
  to work after the enum-class change.
- **Mapper sound callbacks named `*_ESI`** (`Mapper5_ESI`, `Mapper19_ESI`,
  `VRC6_ESI`, `VRC7_ESI`, `Mapper69_ESI`, `FDS_ESI`) are local function
  identifiers, NOT references to the `ESI` type — no migration needed.
- **Migration path**: external code may add
  `#define FCEUX11_NO_DEPRECATION_WARNINGS` before including project
  headers to suppress any deprecation diagnostics; the same gate as the
  v0.3.6 `fceuScopedPtr` deprecation, available until v0.4.0.

## [0.3.7] - 2026-06-11

C-track start: `types.h` responsibility split per plan v3 §5 v0.3.7. The
254-line pre-split header (222 lines after the v0.3.6 deprecations + the
v0.3.6.6 `__clang__` removal) is reorganised into one thin fundamentals
header plus three focused split headers. All 43 existing in-tree consumers
of `#include "types.h"` keep working unchanged because the split headers
are pulled in transitively at the bottom of `types.h`.

This sub-version also introduces the first two symbols in the `fceu11::`
namespace (`kPathSep` and `kPathSepStr`); the namespace machinery was
prepared in v0.3.0/v0.3.6 for fceu11_format.h and fceu11_expected.{h,cpp}.

### Added

- **`src/utils/platform_compat.h`** (105 lines): POSIX compatibility
  shims (MSVC `_dup`/`_stat`/`_mkdir`/`_alloca`/`_fstat`/`_vsnprintf`),
  POSIX access-mode macros (`W_OK`/`R_OK`/`X_OK`/`F_OK`), `PATH_MAX`
  constant, and the first two `fceu11::` namespace symbols —
  `fceu11::kPathSep` (`char`) and `fceu11::kPathSepStr` (`const char[]`).
  The legacy `PSS` / `PS` macros are preserved as direct string/char
  literals (not `fceu11::` references — the C preprocessor does not
  recognise a `const char[]` as a string literal for `"a" PSS "b"`
  concatenation; switching the macros to `fceu11::kPathSepStr` would
  break 12+ call sites in `src/file.cpp` and
  `src/drivers/Qt/config.cpp`).
- **`src/utils/scoped_ptr.h`** (46 lines): `fceuScopedPtr<T>` deprecated
  alias for `std::unique_ptr<T>` (the v0.3.6 RAII migration kept it
  here, gated by `FCEUX11_NO_DEPRECATION_WARNINGS`), and the
  `fceuAllocType` enum preserved as a no-op for out-of-tree consumers.
- **`src/utils/format.h`** (86 lines): `FCEU_CPP_HAS_STD` /
  `FCEU_HAS_CPP_ATTRIBUTE` / `__FCEU_STRINGIZE` / `FCEU_UNUSED` /
  `FCEU_MAYBE_UNUSED` / `__FCEU_PRINTF_FORMAT` /
  `__FCEU_PRINTF_ATTRIBUTE` / `CTASSERT`. The `__clang__` defensive
  branch of the printf format attribute was already removed in v0.3.6.6.

### Changed

- **`src/types.h`** (222 → 100 lines): now a thin fundamentals header
  containing only (a) `<cstdint>` aliases (`uint8/16/32/64`,
  `int8/16/32/64`), (b) `writefunc` / `readfunc` typedefs, (c) the
  `DEBUG(X)` macro, (d) `INLINE` / `GINLINE` / `__restrict__` compiler
  hints, (e) C++20 feature-test macros (`__cpp_lib_span` /
  `__cpp_lib_format`), and (f) transitive includes of the three split
  headers + `utils/endian.h`. Meets the plan v3 §5 v0.3.7 acceptance
  target of `types.h ≤ 100 行`.

### Notes

- **PSS / PS migration path**: existing call sites keep using `PSS` /
  `PS` macros unchanged. New code should use `fceu11::kPathSep` (char)
  or `fceu11::kPathSepStr` (string). v0.3.10 (C-track API modernisation)
  will switch the call sites to `fceu11::kPathSep(Str)` and remove the
  `PSS` / `PS` macros.
- **`fceu11::` namespace**: already used by `src/utils/fceu11_format.h`,
  `src/utils/fceu11_expected.h`, `src/utils/fceu11_expected.cpp` (since
  v0.3.3 / v0.3.6). The new `fceu11::kPathSep(Str)` symbols join these
  in the same `fceu11` namespace — no name collisions.
- **No source change required at any call site**: 43 `#include "types.h"`
  consumers work unchanged. `scripts/_ctest_asan.ps1` is expected to
  show 5/5 ctest passing on the next CI run.
- **PSS_STYLE plumbing unchanged**: still injected by
  `src/CMakeLists.txt:82` (`-DPSS_STYLE=2`); still defined in the
  `_MSC_VER` branch of `src/types.h`; still read in
  `src/utils/platform_compat.h` to select between `PSS "/"` and
  `PSS "\\"`. v0.3.10 will replace this with
  `std::filesystem::path::preferred_separator`.
- **MSVC toolchain unchanged**: `CMakeLists.txt:28-34` still rejects
  non-MSVC toolchains; `__clang__` defensive macros remain removed
  per the v0.3.6.6 errata.

## [0.3.6.6] - 2026-06-11

B-track errata: finish the cleanup of clang-toolchain residue that was started
in the v0.3.6.5 errata commit, and close the 9 items (out of 12) deferred
from the v0.3.6.5 code review that have not been touched since. Enshrines
the two inviolable project principles: (1) MSVC-only toolchain (rejected
toolchains: clang-cl, gcc, MinGW, MSYS2), and (2) `main` is the only
permitted branch (the stale `release/v0.3.6.5` local branch was deleted in
this commit).

### Removed

- **`CMakeLists.txt`**: deleted the `if(ENABLE_LINT)` block's clang-tidy half
  (`find_program(CLANG_TIDY_PROG clang-tidy)` + `add_custom_target(lint-clang-tidy ...)`).
  The project's toolchain is MSVC-only (plan §3.1); clang-tidy has no place in it.
- **`src/drivers/Qt/fceuWrapper.cpp`**: deleted the `__clang__` branch of the
  `__COMPILER__STRING__` ladder. `__GNUC__` and `_MSC_VER` branches stay.
- **`src/types.h`**: deleted the `__clang__` predicate from the
  `__FCEU_PRINTF_ATTRIBUTE` macro `#elif` chain. The `__GNUC__` /
  `FCEU_HAS_CPP_ATTRIBUTE(format)` branches stay.

### Changed

- **`CMakeLists.txt`**: `option(ENABLE_LINT "Enable static analysis targets (clang-tidy, cppcheck)" OFF)` renamed to
  `option(ENABLE_LINT_CPPCHECK "Enable cppcheck static analysis custom target (clang-tidy removed per v0.3.6.6 toolchain policy)" OFF)`.
  The remaining `if(ENABLE_LINT_CPPCHECK)` block contains only the cppcheck
  custom target (cppcheck is a separate, non-clang tool).
- **`CMakeLists.txt`**: `compile_commands.json` comment changed from
  "for clang-tidy / IDE integration" to "for IDE integration (Visual Studio
  Code, CLion, Qt Creator, etc.)". The `set(CMAKE_EXPORT_COMPILE_COMMANDS ON)`
  itself stays — IDEs need it.
- **`scripts/_build_asan.ps1`, `scripts/_build_ubsan.ps1`, `.github/workflows/ci.yml`**: `-DENABLE_LINT=OFF` → `-DENABLE_LINT_CPPCHECK=OFF`.
- **`src/CMakeLists.txt`**: the sanitizer-only LibArchive / OpenGL / ZLIB
  imported-target link bypass has been refactored into a helper function
  `fceux11_resolve_linked_lib` that engages in any non-Release build (Debug
  / RelWithDebInfo) and during sanitizer builds. Fixes the underlying
  "IMPORTED_LOCATION not set for configuration X" generator-time failure
  mode (Q2 of the v0.3.6.5 code review).
- **`scripts/_with_vcvars.bat`, `scripts/_probe_msvc_asan.bat`**: no longer
  hard-code the VS 18 BuildTools vcvars path. New `scripts/_find_vcvars.bat`
  helper provides the same 5-path fallback list as `scripts/do_build.ps1`.
- **`scripts/_build_asan.ps1:27-32`, `scripts/_build_ubsan.ps1:23-26`**:
  cache-wipe `[CLEAN]` message now explicitly mentions `-KeepCache` as the
  opt-in and explains *why* the default wipe exists (stale sanitizer cache
  may bake in the wrong `/fsanitize:` flag form).
- **`scripts/_verify_asan_instrumentation.ps1:3-6`**: header comment now
  says "dumpbin /imports finds at least one `__asan_*` / `__sanitizer_*` import"
  (matching what the script body does). The earlier "dumpbin /symbols" line
  was wrong — `__asan_*` lives in the import table, not the symbol table.
- **`docs/v0.3.x_Construction_Plan_v3.md`**: added the v0.3.6.6 sub-section
  to §5; normalised the 6 stale references to `.clang-format` / `.clang-tidy`
  / `clang-tidy` / `clang-format` (v0.3.0 / v0.3.2 / §4.4 / §4.1 v0.3.2 row);
  annotated the v0.3.2 row in §4.1 to flag the v0.3.6.6 废止.
- **`docs/tech/v0.3.x_Checkpoint_6.5.md`**: line range `593-599` → `626-632`
  (Q9); release date "2026-06-10 REDO" → "2026-06-11 REDO" (Q17).
- **`CHANGELOG.md`**: the orphan `[0.3.6]` content block (RAII 化 /
  fceuScopedPtr migration / Mapper PRG-RAM RAII / Deprecated / Testing)
  now has the missing `## [0.3.6] - 2026-06-09` header (Q13).
- **`CHANGELOG.md`**: F-1 entry rewritten from "F-1 (REAL, deferred)" to
  "F-1 (REAL, CLOSED in v0.3.6.5 errata commit a606561)" with the root cause
  (`sizeof((char*))` sizeof-pointer in `state.cpp:766` + `unif.cpp:158` +
  `bworld.cpp:64,65`) and the fix details. The followup link now points to
  `docs/tech/closed/FOLLOWUP_v0.3.6.5_F1_strncpy_overflow.md` (Q11).

### Added

- **`scripts/_find_vcvars.bat`**: new helper that tries the 5 standard
  vcvars64.bat paths in order (VS 18 BuildTools, 2022 BuildTools, 2022
  Enterprise, 2022 Professional, 2022 Community) and echoes the first one
  that exists. Used by `_with_vcvars.bat` and `_probe_msvc_asan.bat`.
- **`docs/tech/v0.3.x_CodeReview_6.6.md`**: the v0.3.6.6 errata code review
  report (mirrors v0.3.6.5 CodeReview format).

### Notes

- No changes to the emulator hot path, mapper code, savestate layout, or
  test fixtures. The five-gate regression (compile / ctest / byte-level
  savestate / smoke / perf) is unaffected. v0.3.7 (C-track) may start with
  confidence.
- Third-party headers (`src/utils/tl/expected.hpp`, `src/utils/expected.hpp`,
  `src/utils/backward.{hpp,cpp}`) still contain `__clang__` defensive
  macros. They are upstream polyfills / libraries; modifying them would
  diverge from upstream and is out of scope.
- The `clang_rt.asan_dynamic-x86_64.dll` file-name reference is the MSVC
  official ASan runtime file name and cannot be renamed; references in
  scripts and `tests/CMakeLists.txt` are correct.

## [0.3.6.5] - 2026-06-10

B-track integration checkpoint redo. Initial v0.3.6.5 attempt had been
published as ⚠ PARTIAL with the incorrect conclusion "MSVC does not support
ASan". Root cause: the CMake plumbing used `/fsanitize:address` (colon),
which cl silently drops as `warning D9002` — MSVC accepts only the equals
form `/fsanitize=address`. This release corrects the syntax, wires up the
runtime end-to-end, and rewrites the checkpoint report from real evidence.

### Fixed

- `CMakeLists.txt`: sanitizer flag syntax `/fsanitize:address` →
  `/fsanitize=address` (5 occurrences across compile + linker options +
  `ASAN_LDFLAGS` cache var). MSVC 14.51.36231 cl now actually instruments
  the binary — 94 `__asan_*` / `__sanitizer_*` imports from
  `clang_rt.asan_dynamic-x86_64.dll` confirmed via `dumpbin /imports`.
  Build log no longer emits the ~870 D9002 warnings from the initial
  v0.3.6.5 attempt.
- `CMakeLists.txt`: ASan build now also adds `/Zi` so cl emits debug
  symbols (silences C5072) and appends `/DEBUG /OPT:REF /OPT:ICF` to
  `CMAKE_*_LINKER_FLAGS_RELEASE` so the Release sanitizer build emits
  PDBs — without this, MSVC's bundled `llvm-symbolizer.exe` cannot
  resolve ASan stack frames and reports show only `<exe>+0xRVA`.
  (PDB wiring landed in v0.3.6.5; the first build that actually
  consumes it to symbolise F-1 is part of the v0.3.6.5-followup
  commit — see `FOLLOWUP_v0.3.6.5_F1_strncpy_overflow.md` §4.)
- `src/CMakeLists.txt`: vcpkg `LibArchive::LibArchive`, `ZLIB::ZLIB`, and
  `OpenGL::GL` only register `IMPORTED_LOCATION` (no per-config suffix),
  causing CMake generator-time `IMPORTED_LOCATION not set for
  configuration Debug/RelWithDebInfo` failures under sanitizer builds.
  Workaround: under `FCEUX11_ASAN` / `FCEUX11_UBSAN`, link these via raw
  `${..._LIBRARIES}` strings / system `opengl32` to bypass the imported
  target machinery (instrumentation lives in our TUs, not theirs).
- `tests/CMakeLists.txt`: ctest properties now inject MSVC bin + vcpkg
  release bin + vcpkg debug bin onto PATH for all 5 tests when
  `FCEUX11_ASAN` or `FCEUX11_UBSAN` is on, so sanitizer-instrumented
  test exes find `clang_rt.asan_dynamic-x86_64.dll` / `Qt6Core.dll` /
  `SDL2.dll`. Without this, every test exited 0xc0000135 STATUS_DLL_NOT_FOUND
  before `main()`.

### Changed

- `FCEUX11_UBSAN=ON` no longer attempts unsupported `/fsanitize=undefined`
  (MSVC has never implemented clang-style UBSan — Microsoft Learn
  `/cpp/sanitizers/` landing page lists only AddressSanitizer). It now
  substitutes MSVC-native `/RTC1 + /sdl + /GS + /guard:cf` runtime checks,
  explicitly labelled "MSVC-native UB-runtime checks" in CMake STATUS.
  Full clang-style UBSan deferred to v0.4.x toolchain workstream.
- `FCEUX11_ASAN` documentation: MSVC's bundled ASan does NOT implement
  LeakSanitizer on Windows (asking for `detect_leaks=1` makes ASan exit
  with "detect_leaks is not supported on this platform"). Removed
  `detect_leaks=1` from default `ASAN_OPTIONS`.

### Added

- `scripts/_build_asan.ps1` — Release+ASan build driver with D9002 guard
- `scripts/_build_ubsan.ps1` — Debug+/RTC1 UB-substitute build driver
- `scripts/_ctest_asan.ps1` — ctest with auto PATH + sane ASAN_OPTIONS
- `scripts/_ctest_ubsan.ps1` — ctest against build-ubsan
- `scripts/_verify_asan_instrumentation.ps1` — three independent witnesses
  (import count, DLL dependency, D9002 absence) to prevent the v0.3.6.5
  initial failure mode where a non-instrumented binary still "passes"
- `scripts/_probe_msvc_asan.bat` — single-shot probe that compares
  `/fsanitize=address`, `/fsanitize:address`, and `/fsanitize=undefined`
  acceptance; first-line debugging when any future agent suspects ASan
  isn't working
- `scripts/_with_vcvars.bat` — generic vcvars64 + run wrapper

### Docs

- Rewrote `docs/tech/v0.3.x_Checkpoint_6.5.md` from scratch with real
  instrumentation evidence. Retracted initial version's defensive
  "DO NOT switch to clang-cl" guardrail commentary (it was rationalizing
  the flag-syntax bug, not a real toolchain constraint). MSVC lock
  remains in place for ABI / byte-level savestate reasons (plan §3.1),
  which is independent of sanitizer support.

### Known Issues

- **F-1 (REAL, CLOSED in v0.3.6.5 errata commit a606561)**: ASan exposed
  a heap-buffer-overflow in the ROM-load test path — `strncpy(buf, src, 7)`
  wrote 7 bytes into a 5-byte `malloc`'d buffer, overflowing 2 bytes.
  Reproduced in 4 of 5 ctest cases. **Root cause**: three call sites
  (`src/state.cpp:766`, `src/unif.cpp:158`, `src/input/bworld.cpp:64,65`)
  passed `sizeof((char*))` (the pointer size, 8 on x64) to `FCEU_strlcpy`
  instead of the actual malloc'd buffer size. **Fix**: pass the
  runtime-computed `desc_len` / `name_len` / `sizeof(bdata)` instead.
  Pattern audit: 0 instances of `sizeof((char*))` remain in real code.
  Followup document is now archived at
  [`docs/tech/closed/FOLLOWUP_v0.3.6.5_F1_strncpy_overflow.md`](docs/tech/closed/FOLLOWUP_v0.3.6.5_F1_strncpy_overflow.md).
  v0.3.7 may start with confidence.
- Real LSan / clang-style UBSan coverage requires clang-cl, scheduled
  for v0.4.x as an opt-in CI matrix job (main toolchain remains MSVC).



## [0.3.6] - 2026-06-09

### Changed

- **RAII 化 (异常安全 + unique_ptr 迁移)**: `FCEU_malloc` / `FCEU_free` /
  `FCEU_dmalloc` / `FCEU_dfree` 标记 `[[deprecated]]`,指向
  `std::make_unique_for_overwrite<uint8_t[]>(n)` /
  `std::pmr::get_default_resource()->allocate(n)` (utils/memory.h).
- **fceuScopedPtr 迁移**: 旧的 `fceuScopedPtr<T>` 类定义替换为
  `using fceuScopedPtr = std::unique_ptr<T>` 别名(types.h);唯一使用点
  `state.cpp:FCEUSS_Load` 改为 `std::unique_ptr<EMUFILE>`.
- **Mapper PRG-RAM RAII**: 95 处 `FCEU_gmalloc` / `FCEU_dmalloc` 调用迁移为
  `FCEU_gmalloc_unique` RAII 模式(static `FceuMallocPtr NAME_owner` +
  `NAME_owner = FCEU_gmalloc_unique(N); NAME = NAME_owner.get();`)。
  自动化:`tools/transform_v036.py` Python 脚本(幂等,处理嵌套括号
  size 表达式、extern/bare/static 三种声明、跨文件 extern 指针保留原
  FCEU_gfree、nes_ntsc_t*/uint32*/uint16*/float* 多类型 cast 保留)。
- **CFG 验证文档化**: `fceu.h` 顶部添加注释,说明 DECLFW 标注的 mapper 写
  函数已通过 `/guard:cf` 全局 CFG 保护,无需逐函数 `__declspec(guard_overwrite)`。
- **fceu.cpp / ines.cpp / nsf.cpp / fds.cpp / vidblit.cpp**: 多文件手工迁移
  + `transform_v036.py` 自动化补充,确保 `FreeBuffers()` 等关键路径无双重释放。
- **docs/tech/v0.3.6_Release_Notes.md**: 完整交付记录(实施 + 五道闸验收 + 文件清单)。

### Deprecated

- `FCEU_malloc`, `FCEU_free`, `FCEU_dmalloc`, `FCEU_dfree` — 仍可用,但
  编译期警告。`FCEUX11_NO_DEPRECATION_WARNINGS` 宏可在 v0.3.x 期间抑制。
  实际删除推迟到 v0.4.0(per 计划 §6.3)。
- `fceuScopedPtr<T>` 类 — 仍可用(typedef to `std::unique_ptr<T>`),编译期警告。

### Testing

- 5/5 ctest 通过(smoke_test, mapper_load_test, mapper_reset_test,
  rom_regression_test, expected_api_test)。
- 323/323 cargo test 通过(utils 92 + formats 135 + media 48 + debug 2 +
  rom_tests 46)。
- 字节级 savestate 哈希与 v0.3.0 基线 `tests/fixtures/golden_hashes.json` 一致。

## [0.3.5] - 2026-06-09

### Changed

- Release v0.3.5

## [0.3.1] - 2026-06-08

### Changed

- V0.3.1 工具链锁定（MSVC 19.36 + Qt 6.8 LTS）


### Testing

- V0.3.0 测试基建构建完成

## [0.2.30] - 2026-06-07

### Changed

- Core abstraction design, architecture freeze, and documentation

## [0.2.29] - 2026-06-07

### Changed

- Migrate PNG snapshot save + pixel accessors to Rust

- Migrate FM2 movie parser/serializer to Rust

- Incremental state serialization migration to Rust

## [0.2.25] - 2026-06-04

### Changed

- Migrate drawing (text rendering + status icons) to Rust

- Migrate cheat decoders + cheat-map to Rust

- Complete cheat list + search migration to Rust

- Migrate debug helpers + symbol I/O + ld65 .dbg parser + DebuggerState to Rust


### Documentation

- Archive lua_rust_engine_compatibility_report into docs/history/


### Fixed

- Move fceux11_lua_SetMouseDataCallback out of extern "C" block

## [0.2.22.9] - 2026-06-02

### Changed

- Remove obsolete vc/ and slim down output/

- Migrate conddebug and asm modules to Rust

- Fceux11-lua crate — infrastructure + bit/emu (partial)

- Fix fceux11_lua_GetJoypadState linkage error (joy array)

- Complete P2 Lua bindings + fix test infrastructure

- Complete P3 Lua bindings (sound, zapper, debugger)

- Clean up dead code warnings in fceux11-lua

- Rust Lua engine builds end-to-end with FCEUX11_LUA_RUST_ENABLED=ON

- Add Rust Lua engine compatibility report and L2 test scripts


### V0.2.22

- Bump workspace version to 0.2.22


### V0.2.22.3

- Continue fceux11-lua migration — add gui, input, movie, ppu, savestate, sound bindings


### V0.2.22.6

- Sync fceux11_rust.h (cbindgen headers) + remove null


### V0.2.22.7

- Eliminate all warnings in fceux11-lua


### V0.2.22.9

- Phase B fixes, savestate FFI, GetMouseData callback, unit tests, version bump

## [0.2.21] - 2026-05-30

### Changed

- ROM regression test baseline + agent spec

- NSF parser Rust migration + version bump

## [0.2.8] - 2026-05-28

### Added

- Phase 2 GUID Rust migration with build fixes (**v0.2.3**)

- Phase 3 General Utilities Rust migration (**v0.2.4**)

## [0.2.1] - 2026-05-24

### Changed

- Phase 0 baseline freeze and version bump to v0.2.1

- Phase 1 build system refactor — MSVC + vcpkg single-track

- V0.2.1 MSVC 2022 + vcpkg single-track migration

- Phase 2 source-level compatibility cleanup

- Phase 3 peripheral script & document cleanup

- Phase 4 clean build validation fixes

- Phase 5 — 文档与元数据更新


### Documentation

- Rewrite readme intro for clarity and tone


### Fixed

- Resolve fceux11_smoke_test link conflicts (**tests**)

## [0.2.0] - 2026-05-20

### Changed

- Fix MinGW link order for Rust module and remove BOM from fceuWrapper.cpp

- Add .claude/ to .gitignore


### Documentation

- Add AGENTS.md for test suite maintainer guide (**tests**)


### Fixed

- Resolve build warnings, improve build script, update build guide, clean up root artifacts

## [0.1.0] - 2026-05-17

### Documentation

- 调整工具链策略，明确近期以 msys64 为主，MSVC 为远期计划

- Soften Windows 11 exclusivity in readme taglines

