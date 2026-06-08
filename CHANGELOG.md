# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

