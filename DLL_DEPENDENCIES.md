# FCEUX11 DLL Dependencies (v0.2.1)

> **Toolchain**: MSVC 2022 + vcpkg
> **Deployment**: Use `cmake --install` or `scripts\copy_dependencies.ps1`

---

## Quick Deploy

```powershell
# Recommended: CMake handles everything
cmake --install build --prefix dist

# Or use the PowerShell helper
.\scripts\copy_dependencies.ps1 `
    -ExecutablePath "build\src\fceux11.exe" `
    -OutputDir "dist"
```

---

## Runtime Dependencies

All runtime DLLs are sourced from `vcpkg_installed\x64-windows\bin` (or your vcpkg root):

| DLL | Source Package |
|-----|---------------|
| Qt6Core.dll | qtbase |
| Qt6Gui.dll | qtbase[widgets] |
| Qt6Widgets.dll | qtbase[widgets] |
| Qt6OpenGL.dll | qtbase[opengl] |
| Qt6OpenGLWidgets.dll | qtbase[opengl,widgets] |
| SDL2.dll | sdl2 |
| archive.dll | libarchive |
| zlib1.dll | zlib |
| liblzma.dll | liblzma |
| ... | transitive deps (ICU, PNG, JPEG, etc.) |

> **No MinGW runtime DLLs** (`libgcc_s_seh-1.dll`, `libwinpthread-1.dll`, `msys-2.0.dll`) are required or present.

---

## System DLLs (Built into Windows)

| DLL | Purpose |
|-----|---------|
| KERNEL32.dll | Windows kernel |
| USER32.dll | UI subsystem |
| SHELL32.dll | Shell APIs |
| OPENGL32.dll | OpenGL 1.1 (fallback) |
| WS2_32.dll | Winsock |
| VFW32.dll | Video for Windows (AVI) |

---

## Rebuild from Scratch

```powershell
Remove-Item -Path build -Recurse -Force
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build
ctest --test-dir build --output-on-failure
```

