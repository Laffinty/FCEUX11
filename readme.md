# FCEUX11

一款面向 Windows 11 的现代化 NES（任天堂娱乐系统）模拟器，基于 [FCEUX](https://fceux.com) 衍生开发。\
*A modernized Nintendo Entertainment System (NES) emulator for Windows 11, derived from [FCEUX](https://fceux.com).*

> **English Overview**: FCEUX11 is a derivative work of the FCEUX emulator, created under the GPLv2 license. It targets exclusively the Windows 11 platform, leveraging modern Microsoft toolchains and UI frameworks for high-performance NES emulation.

---

## 1. 平台定位 Platform Focus

Windows 11 是唯一官方支持的平台。本项目充分利用 Windows 11 的现代 API 与视觉特性（如 Mica 材质、暗色/亮色主题、Segoe UI 可变字体），不提供对 Windows 10 或更早版本的兼容性保证。\
*Windows 11 is the sole officially supported platform. The project leverages modern Windows 11 APIs and visual features; compatibility with Windows 10 or earlier is not guaranteed.*

> **English**: Exclusive Windows 11 support with native dark/light theme integration and modern UI styling.

---

## 2. 开发状态 Development Status

本项目处于活跃开发阶段。v0.2.1 完成了从 MinGW/MSYS2 到 MSVC 2022+ 工具链的迁移，并全面采用 vcpkg 进行依赖管理。\
*The project is in active development. Version 0.2.1 completed the toolchain migration from MinGW/MSYS2 to MSVC 2022+, with vcpkg adopted for dependency management.*

> **English**: v0.2.1 marks the completion of the MSVC migration — MinGW support has been removed in favor of a single MSVC + vcpkg build track.

---

## 3. 技术栈与开发目标 Tech Stack & Roadmap

| 方向 | v0.2.1 现状 | 远期规划 |
|---|---|---|
| **工具链 Toolchain** | MSVC 2022+ (v143)，通过 `vcvars64.bat` 初始化 | — |
| **依赖管理 Dependencies** | vcpkg manifest 模式 (`vcpkg.json`) | — |
| **UI 框架 UI Framework** | Qt6 (Widgets, OpenGL, OpenGLWidgets) | Windows 11 原生样式深度定制 |
| **多媒体 Multimedia** | SDL2 | — |
| **压缩与归档 Archive** | libarchive + zlib + liblzma | — |
| **安全编译 Security Flags** | `/W4 /permissive- /guard:cf /GS /sdl` | 持续强化 |
| **性能组件 Performance** | 少量 Rust 模块 (`x86_64-pc-windows-msvc`) | 逐步扩展 Rust 重构 |
| **C++ 标准 Standard** | C++20 | 跟进最新标准 |

> **English**: The project is locked to the MSVC 2022+ toolchain with vcpkg manifest-mode dependency management. Qt6 provides the UI layer, SDL2 handles multimedia, and Rust is used selectively for performance-critical components.

---

## 4. 工具链要求 Prerequisites

### 4.1 必需工具 Required Tools

- **Visual Studio 2022 BuildTools**（或更高版本）— 需安装 "使用 C++ 的桌面开发" 工作负载\
  *Visual Studio 2022 BuildTools (or later) — "Desktop development with C++" workload required.*
- **CMake** ≥ 3.28\
  *CMake 3.28 or later.*
- **vcpkg** — 建议通过 `VCPKG_ROOT` 环境变量指定路径，CMake 会自动检测集成\
  *vcpkg — the project auto-detects via the `VCPKG_ROOT` environment variable.*
- **Rust** — `stable-x86_64-pc-windows-msvc` 目标（若 Rust 未安装，CMake 会自动禁用 Rust 模块）\
  *Rust with the `stable-x86_64-pc-windows-msvc` target. If Rust is absent, the Rust module is automatically disabled.*

> **English**: MSVC 2022+, CMake ≥ 3.28, vcpkg, and optionally Rust (MSVC target) are required. MinGW and MSYS2 are no longer supported as of v0.2.1.

### 4.2 vcpkg 依赖清单 vcpkg Dependencies

依赖由项目根目录的 `vcpkg.json` 自动管理，无需手动安装：\
*Dependencies are managed automatically by `vcpkg.json` in the project root:*

- `qtbase[opengl,widgets]` ≥ 6.0.0
- `qttools[linguist]` ≥ 6.0.0
- `sdl2` ≥ 2.0.0
- `libarchive` ≥ 3.0.0
- `zlib` ≥ 1.2.0
- `liblzma` ≥ 5.0.0

> **English**: All dependencies are declared in `vcpkg.json` and resolved automatically during the CMake configuration step.

---

## 5. 编译说明 Build Instructions

### 5.1 快速编译 Quick Build

```powershell
# 1. 克隆仓库 Clone the repository
git clone <repo-url>
cd FCEUX11

# 2. 确认 vcpkg 环境变量 Set vcpkg environment variable
$env:VCPKG_ROOT = "D:\vcpkg"   # 根据你的实际路径修改 adjust to your path

# 3. 运行编译脚本 Run the build script
.\do_build.bat
```

> **English**: The `do_build.bat` script handles `vcvars64.bat` initialization, CMake configuration, and the build in a single step.

### 5.2 手动分步编译 Manual Build

```powershell
# 1. 初始化 MSVC 环境 Initialize MSVC environment
& "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 2. 配置 Configure
$env:VCPKG_ROOT = "D:\vcpkg"
cmake -B build -G "NMake Makefiles"

# 3. 编译 Build
cmake --build build --config Release

# 可执行文件位于 build/src/fceux11.exe
# The executable is located at build/src/fceux11.exe
```

> **English**: Use `NMake Makefiles` (single-threaded) or `Visual Studio 17 2022` (multi-threaded with `--parallel`) as the CMake generator.

### 5.3 首次编译注意事项 First-Time Build Notes

- **vcpkg 首次安装时间较长**：所有依赖包会在首次 `cmake configure` 时自动下载并编译，视网络与机器性能可能需要 10~30 分钟。\
  *First-time vcpkg dependency installation may take 10–30 minutes depending on network and machine performance.*
- **避免全量重编译**：`src/CMakeLists.txt` 中的 `BUILD_TS` 时间戳宏每次 configure 都会变化，会导致所有 `.cpp` 被判定为需要重编。日常迭代时建议临时注释掉该宏，或使用 Ninja 生成器加速。\
  *The `BUILD_TS` timestamp macro in `src/CMakeLists.txt` triggers full rebuilds on every reconfigure. Consider disabling it for daily development, or switch to the Ninja generator for parallel builds.*
- **Qt6 LinguistTools**：`qttools` 需显式声明在 `vcpkg.json` 中（`qtbase` 不自带），否则 CMake 会报 `lrelease` 找不到。\
  *`qttools` must be explicitly listed in `vcpkg.json`; `qtbase` does not include LinguistTools.*

> **English**: First builds are slow due to vcpkg bootstrapping. The `BUILD_TS` macro causes unnecessary full rebuilds — disable it for iterative development.

---

## 6. 项目架构 Architecture

| 模块 Module | 职责 Responsibility |
|---|---|
| `fceux11_utils` | 通用工具类（字符串处理、CRC32、MD5、内存管理）*General utilities (strings, CRC32, MD5, memory)* |
| `fceux11_boards` | NES 主板/Mapper 支持库 *NES board and mapper support library* |
| `fceux11_core` | 模拟器核心（CPU、PPU、APU、输入、存档）*Emulator core (CPU, PPU, APU, input, save states)* |
| `fceux11_drivers_common` | 跨平台驱动公共层（视频滤镜、参数解析）*Cross-platform driver commons (video filters, argument parsing)* |
| `fceux11_drivers_qt` | Qt6 前端驱动（主窗口、调试器、TAS 编辑器、配置界面）*Qt6 frontend (main window, debugger, TAS editor, config UI)* |
| `fceux11_rust` | Rust 性能模块（可选编译）*Rust performance modules (optional)* |
| `fceux11` | 最终可执行文件入口 *Final executable entry point* |

> **English**: The codebase is organized into layered static libraries: utilities → boards → core → common drivers → Qt frontend, with optional Rust integration.

---

## 7. 许可证 License

本项目是基于 [FCEUX](https://fceux.com) 模拟器的衍生作品，采用 **GNU 通用公共许可证第 2 版**（GPLv2）发布。\
*This project is a derivative work based on the [FCEUX](https://fceux.com) emulator, distributed under the **GNU General Public License Version 2** (GPLv2).*

完整的许可证文本请参见仓库根目录的 [`COPYING`](COPYING) 文件。\
*For the full license text, see the [`COPYING`](COPYING) file in the repository root.*

> **English**: FCEUX11 is a GPLv2 derivative of FCEUX. Complete source code is maintained and made available alongside any distributed binaries.

---

## 8. 开发者备忘 Developer Notes

### 8.1 已知构建问题 Known Build Issues

| 问题 | 状态 | 说明 |
|---|---|---|
| `asm.cpp.obj` 偶发缺失 | ✅ 已缓解 | NMake 单线程下 `cmake_cl_compile_depends` 偶发静默失败，手动单编该文件或换用 Ninja/VS 生成器可避免 |
| `QWindowsWindowFunctions` | ✅ 已修复 | Qt6 中已移除，已加 `#if QT_VERSION < 6.0.0` 条件编译 |
| `QAction::parentWidget()` | ✅ 已修复 | Qt6 中已废弃，已替换为 `qobject_cast<QWidget*>(parent())` |
| Manifest 重复嵌入 | ✅ 已修复 | `.rc` 文件已包含 manifest，CMake 中加了 `/MANIFEST:NO` |
| `ntdll` 符号缺失 | ✅ 已修复 | Rust std 在 MSVC 下依赖 `ntdll`，已加入链接库 |

> **English**: A series of MSVC/Qt6/vcpkg migration issues have been resolved in v0.2.1. The remaining `asm.cpp.obj` intermittent failure is a NMake-specific quirk best avoided by using Ninja or Visual Studio generators.

### 8.2 推荐日常开发流程 Recommended Dev Workflow

```powershell
# 使用 Ninja 生成器加速迭代（需安装 ninja）
cmake -B build -G Ninja
cmake --build build --parallel

# 或 Visual Studio 生成器（支持 IDE 调试）
cmake -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release --parallel
```

> **English**: For daily development, prefer the Ninja or Visual Studio generator over NMake Makefiles to enable parallel compilation and avoid the `asm.cpp.obj` intermittent issue.

---

*Version 0.2.1 — MSVC + vcpkg Single-Track Build*
