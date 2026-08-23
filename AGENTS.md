# FCEUX11 — Agent Guide

NES/Famicom emulator derived from FCEUX. Windows 11 only, MSVC 2022+ only. Qt6 GUI, Rust CPU rewrite (since v2.0), C++20.

## Build

Always use the build script — it loads vcvars, finds Ninja, and injects vcpkg. Running bare `cmake --build` from a normal shell fails with `C1083 <cstdio>`.

```powershell
.\scripts\setup_vcpkg.ps1          # one-time: clone vcpkg + install deps (~30 min)
$env:VCPKG_ROOT = "$PWD\vcpkg"     # must be set before build
.\scripts\do_build.ps1 -Config Release       # → build\src\fceux11.exe
.\scripts\do_build.ps1 -Config Debug
.\scripts\do_build.ps1 -Config Release -Clean  # full clean rebuild
```

Incremental builds: `cmake --build build` (within Developer PowerShell) or re-run `do_build.ps1`.

## Test

```powershell
cd build
ctest --output-on-failure               # all tests
ctest --output-on-failure -LE perf      # skip perf benchmarks
ctest --test-dir build --build-config Release --output-on-failure
```

Single test target:
```powershell
cmake --build build --target fceux11_smoke_test
ctest --test-dir build -R smoke_test --output-on-failure
```

Tests live in two locations:
- `src/tests/` — headless smoke/mapper tests (Qt GUI globals are NULL; see `src/tests/AGENTS.md` for known traps)
- `tests/` — KagamiQA integration, blargg regression, benchmarks, Lua scripts

Test executables must include `git_info_stub.cpp` to satisfy `fceu_get_git_url`/`fceu_get_git_rev` symbols.

## CMake Options (key ones)

| Option | Default | Notes |
|--------|---------|-------|
| `FCEUX11_BUILD_TESTS` | ON | Build test suite |
| `FCEUX11_ENABLE_RUST` | ON | Rust crate (CPU + Lua + FFI). OFF disables Lua too |
| `FCEUX11_RUST_CPU` | ON | Rust 6502 CPU is the only implementation (C++ CPU deleted in Phase 7). OFF is a configure error |
| `FCEUX11_ENABLE_I18N` | ON | Qt Linguist i18n (12 languages) |
| `FCEUX11_ASAN` | OFF | MSVC AddressSanitizer |
| `FCEUX11_UBSAN` | OFF | MSVC runtime UB checks (/RTC1) |

Pass via: `cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release -DOPTION=VALUE`

## Toolchain Constraints

- **MSVC-only.** MinGW, clang-cl, MSYS2 all rejected at configure time. This is an ABI/savestate compatibility decision, not a preference.
- C++20 (`CMAKE_CXX_STANDARD 20`), `/W4 /WX` (warnings as errors), `/permissive-`, `/utf-8`.
- Pre-existing `/wd` suppressions for legacy code (4100, 4267, 4200, 4244, 4820, 4996). New code must compile clean under `/WX`.
- Ninja is the only supported generator. It's bundled with VS but not on PATH — `do_build.ps1` finds it via `vswhere`. Don't assume missing Ninja because `where ninja` returns nothing.
- ccache auto-detected if installed (scoop, chocolatey, winget paths).

## Architecture

```
src/
├── *.cpp/h              # Core emulation (CPU facade, PPU, APU, mappers, bus, cart)
├── boards/              # Mapper implementations (NES cartridge bank-switching)
├── drivers/
│   ├── Qt/              # Qt6 GUI frontend (main app, dialogs, video/audio/input)
│   ├── common/          # Shared driver code (config, input, video helpers)
│   └── null/            # Null driver for headless/test builds
├── rust/                # Rust workspace root
│   ├── Cargo.toml       # Workspace: root staticlib + crates/*
│   ├── crates/
│   │   ├── fceux11-core/   # Rust 6502 CPU (the only CPU impl since Phase 7)
│   │   ├── fceux11-lua/    # Rust mlua Lua engine (FFI stubs for cargo test)
│   │   ├── fceux11-utils/
│   │   ├── fceux11-media/
│   │   ├── fceux11-formats/
│   │   ├── fceux11-debug/
│   │   └── kagami-qa/      # QA dual-oracle system
│   └── fceux11_rust.h      # cbindgen-generated (DO NOT commit, in .gitignore)
├── tests/               # Headless regression tests (smoke, mapper)
└── utils/               # C++ utility code

tests/                   # Top-level test dir (KagamiQA, blargg, benchmarks)
├── fixtures/            # Test ROMs (blargg ROMs downloaded separately)
├── kagami/              # KagamiQA test infrastructure
└── lua_scripts/         # Lua-based test scripts

scripts/                 # PowerShell build/CI helpers
tools/                   # Developer analysis/disassembly Python scripts
```

Entry points:
- `src/drivers/Qt/main.cpp` → GUI executable
- `src/cpu.cpp` / `src/cpu.h` → C++ CPU facade routing to Rust FFI (`fceux11_cpu_*`)
- `src/rust/crates/fceux11-core/src/` → actual 6502 CPU implementation

## Rust Integration

Rust builds as a staticlib (`fceux11_rust.lib`) linked into C++ via FFI. CMake drives `cargo build --release` via a custom command.

- **cbindgen** generates `src/rust/fceux11_rust.h` on every Cargo build. Never edit or commit this file.
- **Target triple** is pinned in `src/rust/.cargo/config.toml`: `x86_64-pc-windows-msvc` with static CRT.
- **Cargo target dir**: CMake sets `CARGO_TARGET_DIR` to `build/src/rust/target/` (not `src/rust/target/`).
- **`cargo test` in isolation**: `cargo test --workspace --exclude fceux11-lua` (fceux11-lua needs C++ FFI symbols for most tests; pure-Rust tests work without).
- **KagamiQA direct runner** requires `--features direct-adapter` (always on in CMake builds).
- **Cargo output path**: `target/x86_64-pc-windows-msvc/release/` (NOT `target/release/`) due to the pinned target triple.

## KagamiQA

Dual-oracle QA system (Oracle A = CTest regression, Oracle B = blargg ROM accuracy).

```powershell
# Download blargg test ROMs (one-time)
.\scripts\download_blargg_roms.ps1

# Build KagamiQA targets
cmake --build build --config Release --target fceux11_blargg_runner
cmake --build build --config Release --target fceux11_lua_runner
cmake --build build --config Release --target kagami_qa_direct_runner

# Rust KagamiQA runner
cd src/rust; cargo build --release -p kagami-qa
# Output: src/rust/target/x86_64-pc-windows-msvc/release/kagami-qa-runner.exe
```

Details: `docs/tech/KagamiQA.md`

## i18n

12 languages via Qt Linguist. Translation sources: `src/drivers/Qt/lang/*.ts`. Build artifacts (`*.qm`) go to `assets/i18n/` (gitignored).

```powershell
# Update translation files from source
python scripts/lupdate_run.py
# or
.\scripts\i18n_update.ps1
```

## Code Style

- **C++**: tabs (width 4), UTF-8, CRLF line endings. See `.editorconfig`.
- **CMake**: tabs (width 4). Format config in `.cmake-format`.
- **Rust**: standard rustfmt (edition 2024).
- **Commit messages**: conventional commits (`feat:`, `fix:`, `refactor:`, `test:`, etc.). git-cliff generates changelogs from these.
- Source files are `/utf-8` encoded (MSVC flag in CMakeLists.txt).

## Gotchas

1. **Never run bare `cmake --build`** from Git Bash or unloaded PowerShell. Always use `do_build.ps1` or a Developer PowerShell with vcvars loaded.
2. **`git_info_stub.cpp`** must be in every test executable's source list or linking fails with undefined `fceu_get_git_rev`.
3. **Headless test traps**: Qt driver globals (`g_config`, `consoleWindow`, `msgLog`) are NULL without QApplication. Any `FCEUD_*` callback can crash. See `src/tests/AGENTS.md` for the full trap list and null-guard pattern.
4. **vcpkg_installed/** is gitignored. First build requires `setup_vcpkg.ps1` + `$env:VCPKG_ROOT`.
5. **Blargg test ROMs** are NOT in git (`.gitignore *.nes`). Download with `scripts/download_blargg_roms.ps1`.
6. **`FCEUX11_RUST_CPU=OFF`** is a configure-time error — the C++ CPU was deleted. Don't try to disable it.
7. **ASan + LTCG conflict**: `FCEUX11_ASAN` or `FCEUX11_UBSAN` disables Link-Time Code Generation.
8. **`fceux11-lua` crate tests**: most require C++ FFI symbols. Use `cargo test --workspace --exclude fceux11-lua` for pure-Rust testing.
9. **KagamiQA Cargo output** is at `target/x86_64-pc-windows-msvc/release/` not `target/release/`. A stale binary at the wrong path will silently produce outdated results.
10. **MSVC lock** is intentional for ABI/savestate byte-level compatibility. Do not attempt to add clang/gcc support.
