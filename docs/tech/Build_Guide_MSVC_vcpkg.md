# FCEUX11 Build Guide — MSVC 2022 + vcpkg

> **适用范围**: FCEUX11 v0.2.1+
> **工具链**: Microsoft Visual C++ (MSVC) 2022 v143+, CMake 3.28+, Ninja, vcpkg

---

## Prerequisites

| Component | Minimum Version | Installation |
|-----------|-----------------|--------------|
| Visual Studio 2022 | 17.x | [Download](https://visualstudio.microsoft.com/) — Workload: "Desktop development with C++" |
| CMake | 3.28 | Bundled with VS 2022 or [standalone](https://cmake.org/download/) |
| Ninja | 1.11 | Bundled with VS 2022 or `pip install ninja` |
| vcpkg | latest | Clone + bootstrap (see below) |
| PowerShell | 7.x | [Download](https://github.com/PowerShell/PowerShell) |
| Git | 2.40+ | [Download](https://git-scm.com/download/win) |

### 1. Install vcpkg

```powershell
# Clone vcpkg (pick a permanent location)
git clone https://github.com/microsoft/vcpkg.git C:\dev\vcpkg
C:\dev\vcpkg\bootstrap-vcpkg.bat

# Set environment variable (persistent)
[Environment]::SetEnvironmentVariable("VCPKG_ROOT", "C:\dev\vcpkg", "User")
```

### 2. Verify toolchain

```powershell
cmake --version    # >= 3.28
ninja --version    # >= 1.11
cl.exe             # Should print "Microsoft (R) C/C++ Optimizing Compiler"
vcpkg.exe --version
```

> **Note**: Run these commands from a **Developer PowerShell for VS 2022**, or run `vcvarsall.bat x64` first.

---

## Build Steps

### Quick Build (one-liner)

```powershell
.\do_build.ps1 -Config Release
```

This script handles configure, build, and test automatically.

### Manual Build

```powershell
# 1. Install dependencies (first time only; may take 1-2 hours)
.\scripts\setup_vcpkg.ps1

# 2. Configure
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# 3. Build
cmake --build build

# 4. Test
ctest --test-dir build --output-on-failure
```

### Output

- Main executable: `build\src\fceux11.exe`
- Smoke test: `build\src\tests\fceux11_smoke_test.exe`
- Mapper tests: `build\src\tests\fceux11_mapper_load_test.exe`

---

## Deploying Dependencies

### Option A: CMake Install (Recommended)

```powershell
cmake --install build --prefix dist
```

This copies `fceux11.exe` and all required vcpkg DLLs to `dist\`.

### Option B: PowerShell Script

```powershell
.\scripts\copy_dependencies.ps1 `
    -ExecutablePath "build\src\fceux11.exe" `
    -OutputDir "dist"
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `cmake` not found | Not in PATH | Launch "Developer PowerShell for VS 2022" |
| `nmake` not found | Wrong generator | Use `-G Ninja` (recommended) or launch from VS Dev Prompt |
| vcpkg packages fail to build | Proxy / network | Set `HTTP_PROXY` / `HTTPS_PROXY` environment variables |
| Qt6 plugins not found at runtime | Missing `QT_PLUGIN_PATH` | Deploy via `cmake --install` or copy `plugins\` from vcpkg |
| `vcpkg` not found | `VCPKG_ROOT` not set | Set the environment variable and restart shell |
| PowerShell execution policy blocked | Default policy is `Restricted` | `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser` |

---

## Notes

- **MSYS2 / MinGW-w64 support was removed in v0.2.1**. Do not use `MSYS Makefiles`, `mingw32-make`, or any POSIX toolchain.
- The project requires **Windows 11** (or Windows 10 21H2+ with latest updates).
- First vcpkg build may take 1-2 hours; subsequent builds reuse cached artifacts.
