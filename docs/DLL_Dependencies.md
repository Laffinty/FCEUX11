# FCEUX11 DLL Dependencies (v2.1.2)

> **Toolchain**: MSVC 2022+ + vcpkg (Qt 6.8 LTS / SDL2)
> **Deployment**: Use `cmake --install` or `scripts\copy_dependencies.ps1`
> **Last refreshed from**: build-rust-ppu (Release, v2.1.2, 2026-09-13)

---

## Direct dependencies (v2.1.2, measured)

dumpbin /dependents on the v2.1.2 Release build (build-rust-ppu/src/fceux11.exe) reports only:

- Qt: Qt6Widgets.dll, Qt6OpenGL.dll, Qt6Gui.dll, Qt6Core.dll
- Media and archive: SDL2.dll, z.dll, archive.dll
- MSVC runtime: MSVCP140.dll, VCRUNTIME140.dll, VCRUNTIME140_1.dll (plus the UCRT api-ms-win-crt-* set that ships with Windows 10/11)
- Windows: KERNEL32.dll, USER32.dll, SHELL32.dll, ole32.dll, ADVAPI32.dll, MSVFW32.dll, bcryptprimitives.dll, ntdll.dll

Everything else in the tables below is a **transitive** dependency of those libraries (Qt plugins, ICU, HarfBuzz, Brotli, and the PostgreSQL client libraries pulled in by QtSql) and is collected automatically by cmake --install or scripts/copy_dependencies.ps1.

---

## Quick Deploy

```powershell
# Recommended: CMake handles everything
cmake --install build-rust-ppu --prefix dist

# Or use the PowerShell helper
.\scripts\copy_dependencies.ps1 `
    -ExecutablePath "build-rust-ppu\src\fceux11.exe" `
    -OutputDir "dist"
```

---

## Runtime Dependencies

All runtime DLLs are sourced from `vcpkg_installed\x64-windows\bin` (or your vcpkg root):

### Core / UI

| DLL | Source Package | Notes |
|-----|----------------|-------|
| `Qt6Core.dll` | `qtbase` | Core event loop, I/O, timers |
| `Qt6Gui.dll` | `qtbase[widgets]` | 2D / OpenGL painters, fonts, images |
| `Qt6Widgets.dll` | `qtbase[widgets]` | Main-window / dialog widgets |
| `Qt6OpenGL.dll` | `qtbase[opengl]` | OpenGL rendering backend |
| `Qt6OpenGLWidgets.dll` | `qtbase[opengl,widgets]` | OpenGL-backed widget surface |

### Input / Audio / Archive

| DLL | Source Package | Notes |
|-----|----------------|-------|
| `SDL2.dll` | `sdl2` | Joystick / audio / timing abstraction |
| `archive.dll` | `libarchive` | ROM archive extraction |
| `liblzma.dll` | `liblzma` | `.7z` / `.xz` decompression |

### Text / Font / Unicode

| DLL | Source Package | Notes |
|-----|----------------|-------|
| `freetype.dll` | `freetype` | TrueType font rasterization |
| `harfbuzz.dll` | `harfbuzz` | Complex-script text shaping |
| `pcre2-16.dll` | `pcre2` | RegExp engine used by Qt |
| `md4c.dll` | `md4c` | Markdown parser for Qt help/docs |
| `icudt78.dll` | `icu` | ICU data (collation, codepages) |
| `icuin78.dll` | `icu` | ICU internationalization |
| `icuuc78.dll` | `icu` | ICU common utilities |

### Compression / Crypto

| DLL | Source Package | Notes |
|-----|----------------|-------|
| `z.dll` | `zlib` | DEFLATE / gzip / `.zip` compression |
| `bz2.dll` | `bzip2` | `.bz2` compression |
| `lz4.dll` | `lz4` | Fast LZ compression |
| `zstd.dll` | `zstd` | Zstandard compression |
| `libpng16.dll` | `libpng` | PNG image loading/saving |
| `brotlicommon.dll` | `brotli` | Brotli decoder common data |
| `brotlidec.dll` | `brotli` | Brotli decoder |
| `libcrypto-3-x64.dll` | `openssl` | TLS / hashing used by libarchive |

### Float conversion

| DLL | Source Package | Notes |
|-----|----------------|-------|
| `double-conversion.dll` | `double-conversion` | Fast double↔string conversion |

> **No MinGW runtime DLLs** (`libgcc_s_seh-1.dll`, `libwinpthread-1.dll`, `msys-2.0.dll`) are required or present.

### Qt Plugins (deployed under `plugins\`)

| Relative Path | Source Package | Notes |
|---------------|----------------|-------|
| `platforms\qwindows.dll` | `qtbase` | Windows QPA platform plugin |
| `styles\qmodernwindowsstyle.dll` | `qtbase` | Modern Windows style |
| `imageformats\qgif.dll` | `qtbase` | GIF image format |
| `imageformats\qico.dll` | `qtbase` | ICO image format |

---

## System DLLs (Built into Windows)

| DLL | Purpose |
|-----|---------|
| `KERNEL32.dll` | Windows kernel |
| `USER32.dll` | UI subsystem |
| `GDI32.dll` | GDI rendering |
| `SHELL32.dll` | Shell APIs |
| `OPENGL32.dll` | OpenGL 1.1 fallback |
| `WS2_32.dll` | Winsock |
| `VFW32.dll` | Video for Windows (AVI) |
| `MSVCP140.dll` / `VCRUNTIME140.dll` | Visual C++ runtime redistributables |

> You may need to install the **Microsoft Visual C++ Redistributable** on a clean Windows 11 system if it is not already present.

---

## Rebuild from Scratch

```powershell
Remove-Item -Path build-rust-ppu -Recurse -Force
cmake -S . -B build-rust-ppu -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build-rust-ppu
ctest --test-dir build-rust-ppu --output-on-failure
```
