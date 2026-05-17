# FCEUX11 v0.1.0 Release Notes

**Version:** 0.1.0  
**Date:** 2026-05-18  
**Platform:** Windows 10/11 (x64)  
**Toolchain:** MinGW-w64 GCC 16.1.0 + Qt6 + Rust (optional)

---

## Overview

FCEUX11 is a Windows 11-focused derivative of the FCEUX NES/Famicom emulator (based on FCEUX 2.6.6). This is the first public release, bringing together the foundational refactoring work from Phase 0 through Phase 7.

## What's Included

This is a **portable archive** — no installer required. Simply extract and run `fceux11.exe`.

| File | Description |
|------|-------------|
| `fceux11.exe` | Main executable (includes Qt6 UI + Rust CRC32 module) |
| `*.dll` | Runtime dependencies (Qt6, SDL2, zlib, libarchive, compiler runtime, etc.) |

## System Requirements

- **OS:** Windows 10 64-bit or Windows 11
- **Architecture:** x86-64 (amd64)
- **Dependencies:** All required DLLs are included in this archive; no additional runtime installation is needed.

## Key Changes Since Upstream (FCEUX 2.6.6)

### Phase 1 — Branding & Identity
- Rebranded visible identifiers to **FCEUX11**
- Updated `version.h` to 0.1.0
- Refreshed About window and main window titles

### Phase 2 — Build System
- Upgraded CMake minimum to 3.28
- Removed Qt5 support; unified on **Qt6**
- Removed Linux/macOS build branches
- Fixed `alloca` / `__forceinline` macro conflicts with GCC 16

### Phase 4 — Qt6 UI
- Migrated deprecated Qt5 APIs to Qt6 equivalents
- Fixed C++20 deprecation warnings (`volatile`, lambda `this` capture, enum arithmetic)

### Phase 6 — Legacy Cleanup
- Marked Win32 and SDL drivers as deprecated (removed from build)
- Cleaned up cross-platform macros in core code

### Phase 7 — Rust Integration
- Introduced `src/rust/` module with CMake integration
- First FFI pilot: **CRC32/Hash computation** via `crc32fast` crate
- Rust target auto-detected: `x86_64-pc-windows-msvc` (MSVC) or `x86_64-pc-windows-gnu` (MinGW)
- Falls back to C++ `zlib::crc32` when Rust is disabled

### Build Fixes
- Resolved `-Wstringop-overflow=` false positives in `EMUFILE_MEMORY`
- Resolved CMake configuration issues under MinGW-w64 GCC 16.1.0

## Known Issues

- `stringop-overflow` warnings may still appear during compilation from source (GCC 16 false positives in `EMUFILE_MEMORY`); they do not affect runtime behavior.
- MSVC build path is planned but not yet the primary target; MinGW-w64 remains the recommended toolchain for source builds.

## License

FCEUX11 is a derivative work of [FCEUX](https://fceux.com) and is licensed under the **GNU General Public License v2** (GPLv2). The original FCEUX copyright notices and author credits are preserved as required by the license.

---

**Download:** `fceux11-0.1.0-windows-amd64.zip`
