# FCEUX11 v0.2.1 构建计划 —— 全微软系工具链迁移与 msys64 清理

> **版本**：v0.2.1  
> **制定日期**：2026-05-24  
> **目标发布日期**：待定  
> **核心目标**：彻底弃用 msys64/MinGW-w64 构建路径，全面迁移至微软系原生工具链（MSVC 2022 + vcpkg + Windows SDK），同步完成版本号升级与文档更新。

---

## 目录

1. [概述与关键约束](#1-概述与关键约束)
2. [Phase 0：预研与基线冻结](#2-phase-0预研与基线冻结)
3. [Phase 1：构建系统重构](#3-phase-1构建系统重构)
4. [Phase 2：源码级兼容层清理](#4-phase-2源码级兼容层清理)
5. [Phase 3：外围脚本与文档清理](#5-phase-3外围脚本与文档清理)
6. [Phase 4：全量编译验证](#6-phase-4全量编译验证)
7. [Phase 5：文档与元数据更新](#7-phase-5文档与元数据更新)
8. [版本号同步清单](#8-版本号同步清单)
9. [里程碑与排期](#9-里程碑与排期)
10. [风险登记册与回滚策略](#10-风险登记册与回滚策略)

---

## 1. 概述与关键约束

### 1.1 背景与动机

v0.2.0 阶段完成了大规模死代码清理、品牌标识层重构与 Qt6 i18n 基础设施搭建。当前项目仍处于 **msys64/MinGW-w64** 与 **MSVC** 双轨并存的过渡态：CMake 中保留 `if(MINGW)` 分支，多处硬编码 `D:/msys64/` 路径，构建入口脚本 `do_build.bat` 内部仍调用 MSYS2 bash，Rust 默认 target 为 `x86_64-pc-windows-gnu`。这种混合态导致：

- **维护成本**：DLL 部署、路径映射、符号兼容性需同时维护两套逻辑。
- **性能损失**：MinGW-w64 生成的二进制在 Windows 11 上无法充分利用 MSVC  Profile-Guided Optimization (PGO) 与 Control Flow Guard (CFG)。
- **生态割裂**：vcpkg 已完整支持 Qt6、SDL2、libarchive、zlib，继续维护 MSYS2 pacman 依赖是重复劳动。

v0.2.1 的核心使命是**切断 msys64 依赖，确立 MSVC 2022+ 为唯一官方工具链**。

### 1.2 工具链定义（v0.2.1 及以后唯一官方配置）

| 组件 | 要求版本 | 说明 |
|------|----------|------|
| Visual Studio | 2022 (v17.x) 或更高 | 工作负载："使用 C++ 的桌面开发" |
| MSVC 编译器 | v143 (14.3x) 或更高 | `/permissive-` 强制标准合规 |
| Windows SDK | 10.0.22621.0 或更高 | 支持 Windows 11 长路径、DPI PerMonitorV2 |
| CMake | 3.28+ | 与根目录 `cmake_minimum_required` 一致 |
| Ninja | 1.11+ | 作为 CMake 默认生成器（推荐） |
| vcpkg | 最新 master | 依赖管理唯一来源 |
| PowerShell | 7.x | 构建脚本与部署脚本执行环境 |
| Rust (可选) | stable-x86_64-pc-windows-msvc | Rust 模块默认 target 同步切换 |

### 1.3 关键约束

- **不允许保留任何 msys64 硬编码路径**：`D:/msys64/`、`C:/msys64/`、`/d/msys64/` 等字符串必须从代码库中清零。
- **POSIX 兼容宏必须 MSVC 化**：`alloca`、`__forceinline`、`ssize_t`、`strcasestr`、`strtok_r` 等必须在源码层解决，而非依赖 MinGW 兼容层。
- **构建脚本必须纯微软系**：禁止 `.bat` 文件内部调用 `bash.exe`、`sh.exe` 或任何 POSIX 工具链。
- **验证门禁**：每个 Phase 完成后必须通过 `ctest --output-on-failure` 与手动冒烟测试（启动 → 加载 ROM → 运行 30 秒）。
- **回滚基线**：任何 Phase 失败后，必须能在 5 分钟内回滚至 v0.2.0 构建状态。

---

## 2. Phase 0：预研与基线冻结

> **目标**：确认目标工具链环境就绪，建立可回滚的编译基线，完成版本号原子级更新。  
> **交付物**：环境就绪确认书、编译基线绿灯、版本号全部更新为 v0.2.1。

### P0-1 环境就绪确认

在执行任何源码修改前，必须人工确认以下环境已正确安装：

1. **Visual Studio 2022** 已安装并包含以下组件：
   - MSVC v143 - VS 2022 C++ x64/x86 生成工具
   - Windows 11 SDK (10.0.22621.0)
   - C++ CMake tools for Windows
   - 适用于 Windows 的 C++ Clang 编译器（可选，用于 Clazy 静态分析）

2. **vcpkg** 已克隆并引导完成：
   ```powershell
   git clone https://github.com/microsoft/vcpkg.git C:\dev\vcpkg
   C:\dev\vcpkg\bootstrap-vcpkg.bat
   [Environment]::SetEnvironmentVariable("VCPKG_ROOT", "C:\dev\vcpkg", "User")
   ```

3. **Rust**（如启用 Rust 模块）：
   ```powershell
   rustup default stable-x86_64-pc-windows-msvc
   rustup target add x86_64-pc-windows-msvc
   ```

4. **验证命令**：
   ```powershell
   cmake --version        # >= 3.28
   ninja --version        # >= 1.11
   cl.exe                 # 应输出 "用于 x64 的 Microsoft (R) C/C++ 优化编译器"
   vcpkg.exe --version    # 已正确输出版本号
   ```

### P0-2 基线冻结与分支策略

- 从 `main` 切出特性分支：`feature/v0.2.1-msvc-migration`。
- 在该分支上执行全部 v0.2.1 工作，禁止直接推送 `main`。
- 每个 Phase 至少一个独立 commit，commit message 前缀统一为 `v0.2.1:`。

### P0-3 版本号原子级更新

> **原则**：版本号更新必须作为独立 commit，不与其他逻辑修改混杂，便于 diff 审查。

| 文件路径 | 当前值 | 目标值 |
|----------|--------|--------|
| `CMakeLists.txt` | `VERSION 0.2.0` | `VERSION 0.2.1` |
| `src/version.h` | `FCEU_VERSION_PATCH  0` | `FCEU_VERSION_PATCH  1` |
| `src/version.h` | `"0.2.0"` | `"0.2.1"` |
| `src/version.h` | `"v0.2.0"` | `"v0.2.1"` |
| `vcpkg.json` | `"version": "0.1.0"` | `"version": "0.2.1"` |
| `src/rust/Cargo.toml` | `version = "0.2.0"` | `version = "0.2.1"` |

**验收标准**：
- [ ] 全局搜索 `0.2.0` 仅出现在 Git 历史、旧版计划文档（`FCEUX11_v0.2.0_Construction_Plan.md`）中，源码中清零。
- [ ] 全局搜索 `0.1.0` 在 vcpkg.json 中清零。
- [ ] 编译后启动程序，窗口标题显示 `FCEUX11 v0.2.1`。

---

## 3. Phase 1：构建系统重构

> **目标**：将 CMake 构建系统从 MinGW/MSYS2 双轨制彻底改造为 MSVC + vcpkg 单轨制。  
> **聚焦文件**：`CMakeLists.txt`、`src/CMakeLists.txt`、`src/rust/CMakeLists.txt`、`vcpkg.json`。

### P1-1 根目录 CMakeLists.txt 改造

**操作清单**：

1. **新增 vcpkg 工具链自动检测**（根目录 `CMakeLists.txt` 顶部）：
   ```cmake
   cmake_minimum_required(VERSION 3.28)
   
   # v0.2.1: Auto-detect vcpkg toolchain
   if(DEFINED ENV{VCPKG_ROOT} AND NOT DEFINED CMAKE_TOOLCHAIN_FILE)
       set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake"
           CACHE STRING "")
   endif()
   
   project(FCEUX11 VERSION 0.2.1 LANGUAGES CXX C)
   ```

2. **强制 MSVC 编译器检查**：
   ```cmake
   if(NOT MSVC)
       message(FATAL_ERROR "FCEUX11 v0.2.1+ requires MSVC 2022+ toolchain. "
                           "MinGW-w64 / MSYS2 support has been removed.")
   endif()
   ```

3. **保留并强化 C++20 标准**：
   ```cmake
   set(CMAKE_CXX_STANDARD 20)
   set(CMAKE_CXX_STANDARD_REQUIRED ON)
   set(CMAKE_CXX_EXTENSIONS OFF)  # 禁用 GNU 扩展，强制标准 C++
   ```

4. **新增 MSVC 安全编译选项**（Release / RelWithDebInfo 生效）：
   ```cmake
   add_compile_options(/W4 /permissive- /guard:cf /GS /sdl)
   add_link_options(/GUARD:CF /CETCOMPAT)
   ```

### P1-2 src/CMakeLists.txt 重构

**核心任务**：删除全部 `if(MINGW)` 分支，将 `else()`（原 MSVC 预留分支）提升为唯一路径。

1. **SDL2 / libarchive / zlib 查找方式 vcpkg 化**：
   ```cmake
   find_package(SDL2 CONFIG REQUIRED)
   find_package(LibArchive REQUIRED)
   find_package(ZLIB REQUIRED)
   find_package(Qt6 REQUIRED COMPONENTS Widgets OpenGL OpenGLWidgets ${QtHelpModule})
   ```
   > **说明**：vcpkg 提供的 SDL2 已包含 `SDL2::SDL2` imported target，不再需要手动 `find_path`。

2. **删除 msys64 硬编码路径**：
   - 删除第 40-41 行：
     ```cmake
     find_path(SDL2_INCLUDE_DIR SDL.h PATH_SUFFIXES SDL2 PATHS $ENV{MINGW_PREFIX}/include D:/msys64/mingw64/include C:/msys64/mingw64/include)
     find_path(MINGW_INCLUDE_DIR SDL2/SDL.h PATHS $ENV{MINGW_PREFIX}/include D:/msys64/mingw64/include C:/msys64/mingw64/include)
     ```
   - 删除第 519 行：
     ```cmake
     set(LRELEASE_EXECUTABLE /d/msys64/mingw64/bin/lrelease-qt5.exe)
     ```
     替换为 CMake `find_program` 或 `qt6_add_translations`：
     ```cmake
     find_package(Qt6 COMPONENTS LinguistTools REQUIRED)
     qt6_add_translations(${APP_NAME} TS_FILES ${TS_FILES})
     ```

3. **链接库声明现代化**：
   ```cmake
   target_link_libraries(fceux11_drivers_qt PUBLIC
       fceux11_core
       fceux11_drivers_common
       Qt6::Widgets
       Qt6::OpenGL
       Qt6::OpenGLWidgets
       SDL2::SDL2
       LibArchive::LibArchive
       ZLIB::ZLIB
       ${LUA_LDFLAGS}
       ${LIBAV_LDFLAGS}
       wsock32 ws2_32 vfw32 Htmlhelp
   )
   ```

4. **Rust 模块工具链锁定**：
   ```cmake
   # In src/rust/CMakeLists.txt
   set(FCEUX11_RUST_TARGET "x86_64-pc-windows-msvc" CACHE STRING "Rust target triple")
   set(FCEUX11_RUST_TOOLCHAIN "stable" CACHE STRING "Rust toolchain")
   ```
   删除 `if(MINGW)` 下的 `target_link_libraries(fceux11_utils PUBLIC ntdll)`（MSVC 下由系统 CRT 自动处理）。

### P1-3 vcpkg.json 修正

当前 `vcpkg.json` 存在语法错误（`"version">=6.0.0"` 缺少 `:`），需在升级版本号时一并修复：

```json
{
  "name": "fceux11",
  "version": "0.2.1",
  "description": "FCEUX11 - Windows 11 NES Emulator (derivative work based on FCEUX)",
  "homepage": "https://github.com/fceux11/fceux11",
  "license": "GPL-2.0-or-later",
  "dependencies": [
    {
      "name": "qtbase",
      "version>=": "6.0.0",
      "features": ["opengl", "widgets"]
    },
    {
      "name": "sdl2",
      "version>=": "2.0.0"
    },
    {
      "name": "libarchive",
      "version>=": "3.0.0"
    },
    {
      "name": "zlib",
      "version>=": "1.2.0"
    },
    {
      "name": "liblzma",
      "version>=": "5.0.0"
    }
  ],
  "default-features": [],
  "features": {
    "static": {
      "dependencies": [
        "qtbase[static]",
        "libarchive[libarchivezip]"
      ]
    }
  }
}
```

### P1-4 构建时间戳与 Git 信息生成

当前 `genGitHdr.bat` 调用方式：
```cmake
COMMAND ${CMAKE_SOURCE_DIR}/scripts/genGitHdr.bat ${CMAKE_CURRENT_BINARY_DIR}
```
在纯 MSVC 环境下，该批处理文件（如使用 `git describe`）通常无需修改即可工作，但需验证其在 **cmd.exe / PowerShell** 下行为一致。若原脚本依赖 MSYS2 的 `sed`/`awk`，则需重写为 PowerShell：

```powershell
# scripts/GenGitInfo.ps1 (替代方案)
$gitHash = git rev-parse --short HEAD 2>$null
$gitBranch = git rev-parse --abbrev-ref HEAD 2>$null
if (-not $gitHash) { $gitHash = "unknown" }
if (-not $gitBranch) { $gitBranch = "unknown" }

$outDir = $args[0]
$content = @"
const char *fceuGitBuildString = "$gitHash";
const char *fceuGitBranchString = "$gitBranch";
"@
$content | Out-File -Encoding UTF8 "$outDir\fceux_git_info.cpp"
```

**验收标准**：
- [ ] `cmake -B build -S . -G Ninja` 在纯 PowerShell（非 MSYS2）下配置成功，零错误。
- [ ] `cmake --build build --config Release` 编译通过，零链接错误。
- [ ] 可执行文件 `fceux11.exe` 在 `build/src/` 或 `build/` 下生成。
- [ ] 无 `mingw`、`msys`、`msys64` 相关警告或硬编码路径残留。

---

## 4. Phase 2：源码级兼容层清理

> **目标**：消除所有依赖 MinGW POSIX 兼容层的源码级障碍，使代码能在 MSVC `/permissive-` 下零警告编译。  
> **聚焦文件**：`src/types.h`、`src/lua-engine.cpp`、POSIX 类型/函数使用点。

### P2-1 alloca / __forceinline 宏冲突修复

| 文件 | 行号 | 当前代码 | 修复方案 |
|------|------|----------|----------|
| `src/types.h` | ~65 | `#define alloca __builtin_alloca` | 增加 `#ifndef alloca` 守卫，或完全移除（MSVC 下 `<malloc.h>` 已提供）。 |
| `src/lua-engine.cpp` | ~152 | `#define __forceinline __attribute__((always_inline))` | 增加 `#ifndef __forceinline` 守卫。 |

**推荐统一方案**：
```cpp
// src/types.h
#ifdef _MSC_VER
  #include <malloc.h>  // MSVC 提供 alloca
#else
  #ifndef alloca
    #define alloca __builtin_alloca
  #endif
#endif
```

### P2-2 POSIX 类型缺失映射

| POSIX 类型 | 影响区域 | MSVC 替代方案 |
|------------|----------|---------------|
| `ssize_t` | I/O 调用返回值 | `typedef SSIZE_T ssize_t;` 或直接使用 `ptrdiff_t` |
| `mode_t` | 文件权限 | `typedef int mode_t;`（Windows ACL 模型不同，仅用于接口兼容） |
| `pid_t` | 进程标识 | `typedef DWORD pid_t;` |

搜索范围：
```bash
grep -rn "ssize_t\|mode_t\|pid_t" src/ --include="*.cpp" --include="*.h" --include="*.c"
```

### P2-3 POSIX 函数缺失替换

| 函数 | 影响区域 | 替换方案 |
|------|----------|----------|
| `strcasestr` | 字符串工具 | `StrStrIA` (Shlwapi.h) 或自实现 `case_insensitive_search` |
| `strtok_r` | 解析逻辑 | `strtok_s` (MSVC) 或 `std::stringstream` |
| `strndup` | 字符串复制 | `std::string(src, n)` 或自实现 |
| `gettimeofday` | 时间戳 | `QueryPerformanceCounter` 或 `std::chrono::high_resolution_clock` |

搜索范围：
```bash
grep -rn "strcasestr\|strtok_r\|strndup\|gettimeofday" src/ --include="*.cpp" --include="*.h" --include="*.c"
```

### P2-4 内联汇编与编译器扩展清理

- 搜索 `asm(`、`__asm__`、`__attribute__((`（非 `always_inline`）在全仓库中的使用。
- MSVC x64 **完全不支持内联汇编**，必须替换为 Compiler Intrinsics：
  - `__builtin_bswap32` → `_byteswap_ulong`
  - `__builtin_clz` → `_BitScanReverse` + 位移计算
  - `__builtin_expect` → 直接删除（MSVC 无对应物，现代分支预测器足够智能）

### P2-5 C++20 兼容性加固

在 `/permissive-` 模式下，MSVC 对以下代码模式比 MinGW 更严格：

- **两阶段名称查找**：模板基类中的名称必须显式限定。
- **`volatile` 弃用**：`src/drivers/Qt/sdl-sound.cpp` 中的 `volatile int` 自增/自减需替换为 `std::atomic<int>`。
- **lambda 隐式 `this` 捕获**：`src/drivers/Qt/TasEditor/TasEditorWindow.cpp:6086` 的 `[=]` 改为 `[=, this]`。

**验收标准**：
- [ ] `/W4` 编译下零警告（允许 `/wd4267 /wd4244` 这两个宽度转换警告）。
- [ ] `cl.exe` 编译每个 `.cpp` 时不出现 `"identifier not found"` 的 POSIX 函数错误。
- [ ] 冒烟测试通过（程序启动、加载 ROM、运行 30 秒不崩溃）。

---

## 5. Phase 3：外围脚本与文档清理

> **目标**：删除或重写所有调用 msys64 / bash / POSIX 工具的脚本与文档，统一为 PowerShell / CMD / 原生 Windows 工具。  
> **聚焦文件**：`build.sh`、`do_build.bat`、`scripts/copy_dependencies.ps1`、`DLL_DEPENDENCIES.md`、`docs/tech/Build_Guide_MSYS2_Mingw64.md`、测试文档。

### P3-1 构建入口脚本重写

#### 删除 `build.sh`

该脚本专为 MSYS2 bash 设计，在纯 MSVC 环境下无意义。直接 `git rm build.sh`。

#### 重写 `do_build.bat`

当前脚本调用 `/d/msys64/usr/bin/bash.exe`，彻底违反全微软系约束。重写为纯 PowerShell 脚本 `do_build.ps1`：

```powershell
# do_build.ps1
param(
    [ValidateSet("Debug", "Release", "RelWithDebInfo")]
    [string]$Config = "Release",

    [string]$BuildDir = "build",
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot

if ($Clean -and (Test-Path $BuildDir)) {
    Remove-Item -Recurse -Force $BuildDir
}

# Ensure vcpkg toolchain is discoverable
$vcpkgToolchain = $null
if ($env:VCPKG_ROOT) {
    $vcpkgToolchain = Join-Path $env:VCPKG_ROOT "scripts\buildsystems\vcpkg.cmake"
}

$cmakeArgs = @(
    "-S", $ProjectRoot
    "-B", $BuildDir
    "-G", "Ninja"
    "-DCMAKE_BUILD_TYPE=$Config"
    "-DCMAKE_C_COMPILER=cl"
    "-DCMAKE_CXX_COMPILER=cl"
)
if ($vcpkgToolchain -and (Test-Path $vcpkgToolchain)) {
    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$vcpkgToolchain"
}

Write-Host "[CONFIGURE] cmake $cmakeArgs" -ForegroundColor Cyan
& cmake @cmakeArgs
if ($LASTEXITCODE -ne 0) { throw "CMake configure failed" }

Write-Host "[BUILD] cmake --build $BuildDir --config $Config" -ForegroundColor Cyan
& cmake --build $BuildDir --config $Config
if ($LASTEXITCODE -ne 0) { throw "CMake build failed" }

Write-Host "[TEST] ctest --test-dir $BuildDir --output-on-failure" -ForegroundColor Cyan
& ctest --test-dir $BuildDir --output-on-failure

Write-Host "[SUCCESS] Build complete: $BuildDir" -ForegroundColor Green
```

保留 `do_build.bat` 作为极简调用入口（调用 PowerShell）：
```batch
@echo off
powershell -ExecutionPolicy Bypass -File "%~dp0do_build.ps1" %*
```

### P3-2 依赖复制脚本重构

`scripts/copy_dependencies.ps1` 当前自动搜索 msys64 目录并复制 DLL。改造为 **vcpkg-aware DLL 部署**：

```powershell
# scripts/copy_dependencies.ps1 (v0.2.1 重写)
param(
    [Parameter(Mandatory=$true)]
    [string]$ExecutablePath,

    [string]$OutputDir = (Split-Path $ExecutablePath -Parent),

    [string]$VcpkgRoot = $env:VCPKG_ROOT
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $ExecutablePath)) {
    throw "Executable not found: $ExecutablePath"
}

# Method 1: dumpbin /DEPENDENTS -> search vcpkg installed/bin
if ($VcpkgRoot) {
    $vcpkgBin = Join-Path $VcpkgRoot "installed\x64-windows\bin"
    # ... 解析 dumpbin 输出，从 vcpkgBin 复制所需 DLL ...
}

# Method 2: CMake install() + IMPORTED_RUNTIME_ARTIFACTS (preferred for CI)
# This script is mainly for local ad-hoc deployment.
```

更优的长期方案（v0.2.1 建议实施）：
在 `src/CMakeLists.txt` 末尾增加：
```cmake
install(TARGETS ${APP_NAME} RUNTIME DESTINATION .)
install(IMPORTED_RUNTIME_ARTIFACTS SDL2::SDL2 Qt6::Widgets Qt6::OpenGL ... RUNTIME DESTINATION .)
```
然后使用 `cmake --install build --prefix dist` 一步完成部署。

### P3-3 文档清理

| 文件/目录 | 操作 | 说明 |
|-----------|------|------|
| `docs/tech/Build_Guide_MSYS2_Mingw64.md` | **归档重命名**为 `Build_Guide_MSYS2_Mingw64.md.DEPRECATED` | 保留历史记录，但明确标记为不再维护。 |
| `docs/tech/MSVC_Migration_Guide.md` | **重写**为 `Build_Guide_MSVC_vcpkg.md` | 成为唯一官方编译指南。 |
| `DLL_DEPENDENCIES.md` | **重写** | 移除所有 msys64 PATH 示例，改为 vcpkg `installed/x64-windows/bin` 说明。 |
| `src/tests/AGENTS.md` | **更新** | 移除 `D:/msys64/mingw64/bin` PATH 设置说明，改为 MSVC 环境变量配置。 |
| `pipelines/linux_build.sh` | **删除** | Linux 已非支持目标。 |
| `pipelines/macOS_build.sh` | **删除** | macOS 已非支持目标。 |
| `pipelines/build.pl` | **删除** | Perl 脚本，无人维护。 |
| `pipelines/debpkg.pl` | **删除** | Debian 打包脚本，无人维护。 |
| `pipelines/WinAppveyorBuild.bat` | **删除** | AppVeyor 已弃用。 |
| `pipelines/win32_build.bat` / `win64_build.bat` | **删除** | 遗留 Win32 native 构建。 |
| `pipelines/qwin64_build.bat` | **删除** | 遗留 Qt5 + MSVC 2019 构建。 |
| `scripts/linux_makeIcons.sh` | **删除** | Linux 专用脚本。 |
| `scripts/macosx_makeIcons.sh` | **删除** | macOS 专用脚本。 |
| `scripts/unix_debug_build.sh` | **删除** | Unix 调试脚本。 |
| `scripts/unix_make_docs.sh` | **删除** | Unix 文档生成脚本。 |
| `scripts/genGitHdr.sh` | **删除** | 已使用 `genGitHdr.bat`，不再需要 sh 版本。 |

### P3-4 fceux-server 子目录评估

`fceux-server/` 包含 `cygwin1.dll` 与 Cygwin 编译说明。评估结果：
- 该目录为独立的服务端程序，与主 GUI 构建无关。
- `cygwin1.dll` 为二进制文件，**删除**（Cygwin 已废弃，且 GPL 兼容的 Cygwin DLL 分发需额外声明）。
- `README` 中 "To compile under MS Windows, you should use Cygwin" 句子**删除**或标记为历史信息。

**验收标准**：
- [ ] 仓库中不再存在任何调用 `bash.exe`、`sh.exe`、`msys64`、`mingw64` 的构建入口脚本。
- [ ] 所有 `.ps1` 脚本均能在标准 PowerShell 7 下独立执行，无需额外 POSIX 工具。
- [ ] `docs/tech/Build_Guide_MSVC_vcpkg.md` 包含完整的从零开始编译步骤，一个从未接触过本项目的开发者能在 30 分钟内完成首次编译。

---

## 6. Phase 4：全量编译验证

> **目标**：在干净的 Windows 11 环境（或等效 CI 环境）中，从零开始完成一次完整的配置-编译-测试-部署验证。  
> **交付物**：编译报告、测试报告、部署包。

### P4-1 干净环境构建测试

**环境准备**：
- 一台未安装 MSYS2、未配置 MinGW 的 Windows 11 虚拟机（或 GitHub Actions `windows-latest` runner）。
- 仅安装：VS 2022、CMake、Ninja、vcpkg、PowerShell 7、Git。

**执行步骤**：
```powershell
# 1. 克隆仓库
git clone https://github.com/fceux11/fceux11.git
cd fceux11

# 2. 安装 vcpkg 依赖
.\scripts\setup_vcpkg.ps1

# 3. 配置构建
.\do_build.ps1 -Config Release

# 4. 验证输出
Test-Path .\build\src\fceux11.exe
```

### P4-2 冒烟测试

执行 `src/tests/smoke_test.cpp` 编译的测试程序：
- [ ] 核心符号未丢失（`FCEUI_Init`、`PowerNES`、`FCEU_CreatePalette` 等地址非空）。
- [ ] 返回码 0。

### P4-3 Mapper 回归测试

执行 `src/tests/boards/` 下的回归测试：
- [ ] 8 个代表性 Mapper 加载不失败。
- [ ] `FCEUI_ResetNES()` 不崩溃。

### P4-4 手动功能验证

| 验证项 | 步骤 | 预期结果 |
|--------|------|----------|
| 程序启动 | 双击 `fceux11.exe` | 主窗口出现，标题为 `FCEUX11 v0.2.1` |
| ROM 加载 | File → Open ROM → 选择 `.nes` | ROM 成功加载，画面正常 |
| 基本操作 | 方向键 + A/B/Start/Select | 输入响应正确 |
| 菜单遍历 | 遍历全部一级菜单 | 无崩溃、无异常弹窗 |
| About 窗口 | Help → About | 显示 `FCEUX11 v0.2.1`，无 msys64/MinGW 字样 |
| 语言切换 | Help → Language → 简体中文 | 菜单文字切换为中文 |
| 调试器 | Debug → Debugger | 调试器窗口正常打开 |

### P4-5 部署包验证

使用 `cmake --install` 或重构后的 `copy_dependencies.ps1` 生成发布目录：
- [ ] 目录包含 `fceux11.exe` + 全部必要 DLL（Qt6Core、Qt6Gui、Qt6Widgets、SDL2、archive、zlib 等）。
- [ ] 目录不含任何 `msys-2.0.dll`、`libgcc_s_seh-1.dll`、`libwinpthread-1.dll` 等 MinGW 运行时 DLL。
- [ ] 在另一台未安装开发环境的 Windows 11 机器上，发布包能直接运行。

**验收标准**：
- [ ] 全部测试通过，编译报告零错误、零链接错误。
- [ ] 发布包在裸机 Windows 11 上验证通过。

---

## 7. Phase 5：文档与元数据更新

> **目标**：在编译验证全部通过后，更新面向用户与开发者的文档，同步 `.gitignore`。  
> **原则**：文档修改必须在构建稳定后执行，避免文档与代码状态不一致。

### P5-1 readme.md 重写

**关键变更点**：

1. **工具链声明**：
   ```markdown
   ## Toolchain Requirements
   - **Compiler**: Microsoft Visual C++ (MSVC) 2022 v143 or later
   - **Build System**: CMake 3.28+ with Ninja
   - **Package Manager**: vcpkg
   - **SDK**: Windows SDK 10.0.22621.0+
   > **Note**: MSYS2 / MinGW-w64 support was removed in v0.2.1. The project now builds exclusively with the Microsoft native toolchain.
   ```

2. **编译说明**：删除全部 MSYS2 `pacman` 步骤，替换为：
   ```powershell
   # 1. Install dependencies via vcpkg
   .\scripts\setup_vcpkg.ps1

   # 2. Build
   .\do_build.ps1 -Config Release

   # 3. Run tests
   ctest --test-dir build --output-on-failure
   ```

3. **开发状态**：更新为 "v0.2.1 — Toolchain unified to MSVC 2022+".

### P5-2 .gitignore 更新

新增 MSVC / vcpkg 相关忽略项，清理 MinGW / MSYS2 相关项：

```gitignore
# MSVC / Visual Studio
.vs/
*.user
*.suo
*.sdf
*.opensdf
*.VC.db
*.VC.opendb
ipch/

# vcpkg
vcpkg_installed/

# Build directories (保留原有)
build/
_build/
*.build/
out/

# ... 保留其余现有规则 ...

# 删除或保留（无影响）原有 MinGW 相关项，因为 MinGW 产物扩展名与 MSVC 相同
```

### P5-3 构建指南输出

在 `docs/tech/Build_Guide_MSVC_vcpkg.md` 中提供：
- 环境安装图文/步骤清单。
- vcpkg 依赖安装命令。
- CMake + Ninja 配置命令。
- 常见 MSVC 编译错误速查表（如 `vcpkg` 找不到包、Qt6 插件路径问题）。
- PowerShell 执行策略问题（`Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser`）。

### P5-4 变更日志归档

在 `docs/changes/` 下新增 `FCEUX11_v0.2.1_Changelog.md`，记录：
- 工具链迁移：MinGW-w64 / MSYS2 → MSVC 2022 + vcpkg。
- 构建系统：CMake 单轨制，删除 `if(MINGW)` 分支。
- 源码层：POSIX 兼容宏清理，`alloca` / `__forceinline` / `ssize_t` 修复。
- 脚本层：`do_build.ps1` 取代 `build.sh` + `do_build.bat`。
- 版本号：统一升级至 v0.2.1。

**验收标准**：
- [ ] `readme.md` 中无 "msys64"、"MinGW"、"pacman" 等字样。
- [ ] 新开发者按 `readme.md` 步骤能在 30 分钟内完成首次编译。
- [ ] `.gitignore` 正确忽略 MSVC 中间产物和 `vcpkg_installed/`。

---

## 8. 版本号同步清单

以下文件在 v0.2.1 中必须同步修改：

| # | 文件路径 | 字段/位置 | 目标值 | 负责 Phase |
|---|----------|-----------|--------|-----------|
| 1 | `CMakeLists.txt` | `project(FCEUX11 VERSION 0.2.1 ...)` | `0.2.1` | P0-3 |
| 2 | `src/version.h` | `FCEU_VERSION_PATCH` | `1` | P0-3 |
| 3 | `src/version.h` | `FCEU_VERSION_STRING` | `"0.2.1"` | P0-3 |
| 4 | `src/version.h` | `FCEU_DISPLAY_VERSION` | `"v0.2.1"` | P0-3 |
| 5 | `vcpkg.json` | `"version"` | `"0.2.1"` | P1-3 |
| 6 | `src/rust/Cargo.toml` | `version` | `"0.2.1"` | P0-3 |
| 7 | `src/rust/Cargo.lock` | 多处版本号 | 运行 `cargo update` 自动刷新 | P0-3 |
| 8 | `readme.md` | 版本提及 | 更新为 v0.2.1 | P5-1 |
| 9 | `do_build.ps1` | 脚本头部注释/输出 | 提及 v0.2.1 | P3-1 |

---

## 9. 里程碑与排期

| 里程碑 | 包含 Phase / Task | 预估工期 | 交付物 |
|--------|-------------------|----------|--------|
| **M0：环境就绪** | P0-1 ~ P0-3 | 0.5 人日 | 分支创建、版本号更新、环境确认书 |
| **M1：构建系统重构** | P1-1 ~ P1-4 | 2 人日 | CMake 纯 MSVC 化、vcpkg.json 修正、Rust target 锁定 |
| **M2：源码兼容层清理** | P2-1 ~ P2-5 | 2 人日 | POSIX 宏/函数/类型清零、`/W4` 零警告 |
| **M3：外围脚本清理** | P3-1 ~ P3-4 | 1.5 人日 | `do_build.ps1`、部署脚本、文档归档、遗留脚本删除 |
| **M4：全量验证** | P4-1 ~ P4-5 | 1.5 人日 | 干净环境编译通过、冒烟/回归/手动测试全绿、裸机部署验证 |
| **M5：文档更新** | P5-1 ~ P5-4 | 1 人日 | `readme.md`、`.gitignore`、MSVC 构建指南、变更日志 |
| **Release** | 合并、打标签、发布 | 0.5 人日 | Git 标签 `v0.2.1`、GitHub Release、MSIX / ZIP 包 |

**总预估工期**：约 **9 人日**。

建议推进顺序：

```
Week 1:
  Day 1:  M0 (基线) → M1 (构建系统)
  Day 2:  M1 (收尾) → M2 (源码兼容层)
  Day 3:  M2 (收尾) → M3 (脚本清理)

Week 2:
  Day 4:  M3 (收尾) → M4 (全量验证)
  Day 5:  M4 (收尾) → M5 (文档更新) → Release
```

---

## 10. 风险登记册与回滚策略

| 风险 ID | 描述 | 可能性 | 影响 | 缓解措施 |
|---------|------|--------|------|----------|
| R-01 | vcpkg Qt6 构建时间过长（首次编译 > 2 小时） | 高 | 中 | 启用 vcpkg 二进制缓存 (`VCPKG_BINARY_SOURCES=clear;x-azblob,...`)；CI 缓存 `vcpkg_installed/`；文档中明确说明首次编译耗时。 |
| R-02 | POSIX 函数替换遗漏导致 MSVC 链接错误 | 中 | 高 | Phase 2 中使用 `grep` 建立完整清单，逐条替换后独立编译验证；链接错误比编译错误更容易定位。 |
| R-03 | `alloca` / `__forceinline` 宏清理导致 MinGW 历史分支代码回退困难 | 低 | 低 | 保留 `legacy/fceux-2.6.6-base` 标签；原始 FCEUX upstream 仍在。 |
| R-04 | Qt6 LinguistTools (`lrelease`) 在 MSVC 环境下路径/行为差异 | 中 | 中 | 使用 CMake `qt6_add_translations` 封装，避免直接调用可执行文件；验证 `.qm` 文件生成。 |
| R-05 | Rust MSVC target 与 C++ ABI 不匹配（如 `usize` vs `size_t`） | 低 | 高 | Rust `x86_64-pc-windows-msvc` target 与 MSVC x64 使用相同 C ABI（Itanium-derived）；Phase 1 中通过 `cbindgen` 生成头文件验证。 |
| R-06 | 第三方库通过 vcpkg 安装的版本与 MinGW 时期不兼容（如 SDL2 事件结构变化） | 低 | 高 | vcpkg 锁定 `version>=` 最小版本；冒烟测试覆盖 SDL2 初始化、事件循环、音频打开。 |
| R-07 | 删除 `src/drivers/win/` 后，内部引用的 `zlib` 目录已不存在（v0.2.0 已物理删除） | 低 | 低 | v0.2.0 已删除，v0.2.1 无此风险，仅需确认 `ZLIB::ZLIB` vcpkg target 正确链接。 |
| R-08 | 新构建脚本 `do_build.ps1` 在 Windows PowerShell 5.1 上语法不兼容 | 中 | 低 | 脚本使用 PowerShell 7 语法（如 `Test-Path`、数组展开 `@cmakeArgs`）；文档要求安装 PowerShell 7+。 |

### 回滚策略

- **构建系统回滚**：`src/CMakeLists.txt` 的修改若导致编译失败，优先通过 `git checkout HEAD -- src/CMakeLists.txt` 回退单文件，再逐步重试。
- **源码层回滚**：Phase 2 的宏修改若引发运行时崩溃，使用 `git bisect` 定位具体 commit，revert 后重新评估修复方案。
- **完全回滚**：若整个 v0.2.1 迁移被判定为不可行，删除 `feature/v0.2.1-msvc-migration` 分支，`main` 分支不受影响。
- **文档回滚**：`readme.md` 与 `.gitignore` 的修改始终作为最后几个 commit，回滚不影响代码编译。

---

## 附录 A：POSIX → MSVC 迁移速查表

| POSIX / GCC | MSVC / Windows | 头文件 |
|-------------|----------------|--------|
| `alloca` | `_alloca` | `<malloc.h>` |
| `__attribute__((always_inline))` | `__forceinline` | 内置 |
| `ssize_t` | `SSIZE_T` 或 `ptrdiff_t` | `<BaseTsd.h>` 或 `<stddef.h>` |
| `strcasestr` | `StrStrIA` | `<Shlwapi.h>` + `shlwapi.lib` |
| `strtok_r` | `strtok_s` | `<string.h>` |
| `strndup` | 自实现或 `std::string` | — |
| `gettimeofday` | `GetSystemTimePreciseAsFileTime` | `<windows.h>` |
| `sleep(seconds)` | `Sleep(seconds * 1000)` | `<windows.h>` |
| `usleep(usec)` | `Sleep(usec / 1000)` 或高精度等待 | `<windows.h>` |
| `snprintf` | `_snprintf` 或直接使用（VS 2015+ 已支持标准 `snprintf`） | `<stdio.h>` |
| `__builtin_bswap32` | `_byteswap_ulong` | `<stdlib.h>` |
| `__builtin_clz` | `_BitScanReverse` | `<intrin.h>` |
| `inline asm` (x64) | Compiler Intrinsics | `<intrin.h>` |

---

## 附录 B：v0.2.1 文件删除清单

以下文件/目录将在 v0.2.1 中物理删除：

| 路径 | 删除理由 | 负责 Phase |
|------|----------|-----------|
| `build.sh` | MSYS2 bash 专用，与全微软系冲突 | P3-1 |
| `do_build.bat`（旧版） | 内部调用 `/d/msys64/usr/bin/bash.exe` | P3-1 |
| `scripts/copy_dependencies.ps1`（旧版） | 自动搜索 msys64 目录 | P3-2 |
| `docs/tech/Build_Guide_MSYS2_Mingw64.md` | 工具链已废弃，归档为 `.DEPRECATED` | P3-3 |
| `DLL_DEPENDENCIES.md`（旧版） | 包含 msys64 PATH 示例 | P3-3 |
| `pipelines/linux_build.sh` | Linux 非支持目标 | P3-3 |
| `pipelines/macOS_build.sh` | macOS 非支持目标 | P3-3 |
| `pipelines/build.pl` | Perl 遗留脚本 | P3-3 |
| `pipelines/debpkg.pl` | Debian 遗留脚本 | P3-3 |
| `pipelines/WinAppveyorBuild.bat` | AppVeyor 已废弃 | P3-3 |
| `pipelines/win32_build.bat` | Win32 native 遗留 | P3-3 |
| `pipelines/win64_build.bat` | Win32 native 遗留 | P3-3 |
| `pipelines/qwin64_build.bat` | Qt5 + MSVC 2019 遗留 | P3-3 |
| `scripts/linux_makeIcons.sh` | Linux 专用 | P3-3 |
| `scripts/macosx_makeIcons.sh` | macOS 专用 | P3-3 |
| `scripts/unix_debug_build.sh` | Unix 专用 | P3-3 |
| `scripts/unix_make_docs.sh` | Unix 专用 | P3-3 |
| `scripts/genGitHdr.sh` | 已使用 `genGitHdr.bat` | P3-3 |
| `fceux-server/cygwin1.dll` | Cygwin DLL，分发合规风险 | P3-4 |
| `fceux-server/README` 中 Cygwin 句子 | 编译指南过时 | P3-4 |

---

*本计划由 FCEUX11 开发团队制定，适用于 v0.2.1 版本迭代周期。*
