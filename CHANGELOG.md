# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

