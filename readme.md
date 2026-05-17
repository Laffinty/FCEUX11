# FCEUX11

A modernized Nintendo Entertainment System (NES) emulator, derived from [FCEUX](https://fceux.com) 2.6.6, featuring a Qt6-based UI, high-DPI support, and native Windows integration.

## About This Project

FCEUX11 is a derivative work of the FCEUX emulator, created under the terms of the GNU General Public License Version 2 (GPLv2). This project focuses exclusively on the Windows 11 platform, leveraging modern Microsoft toolchains and UI frameworks to deliver a high-performance NES emulation experience.

## Development Status

This project is currently in active development. Key areas of focus include platform consolidation, codebase modernization, and dependency updates.

## Development Goals

- **Platform Focus**: Windows 11 as the sole supported platform
- **Toolchain Strategy**: MinGW-w64 via msys64 as the current primary build environment; MSVC 2022+ migration planned for the long term
- **UI Migration**: Qt6 with Windows 11 native styling (dark/light themes, Segoe UI)
- **Code Modernization**: Gradual refactoring using Rust for performance-critical components (long-term)
- **General Improvements**: Code cleanup, performance optimizations, and quality-of-life enhancements

## Toolchain Requirements

### Required Tools (Current - msys64/MinGW-w64)
- **msys2/msys64**: Primary build environment
- **MinGW-w64 GCC**: 13.x or later (available via msys2 pacman)
- **CMake**: 3.20 or later
- **Qt6**: UI framework (Widgets, OpenGL, OpenGLWidgets)
- **SDL2**: Multimedia library

### Future Tools (Long-term - MSVC migration)
- **Visual Studio 2022** (or later) with C++ workload
- **CMake**: 3.28 or later
- **vcpkg**: Dependency management

> **Note**: The project currently builds and runs on msys64/MinGW-w64. MSVC support is a long-term goal and not yet ready for daily development.

### Environment Setup

```powershell
# Ensure vcpkg is installed and integrated (for long-term MSVC migration)
$env:VCPKG_ROOT = "C:\path\to\vcpkg"
```

## Building Instructions

### Windows (msys64/MinGW-w64) — Current Primary

```bash
# In MSYS2 MinGW64 shell
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-cmake \
  mingw-w64-x86_64-qt6-base mingw-w64-x86_64-SDL2 \
  mingw-w64-x86_64-libarchive mingw-w64-x86_64-zlib

# Create build directory
rm -rf build
mkdir build
cd build

# Configure with CMake
cmake .. -G "MinGW Makefiles" \
  -DCMAKE_BUILD_TYPE=Release

# Build
mingw32-make -j$(nproc)

# The executable will be located at:
# ./src/fceux11.exe
```

### Windows (MSVC) — Long-term Plan (Not Yet Ready)

```powershell
# Create build directory
Remove-Item -Path build -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path build | Out-Null
cd build

# Configure with CMake
cmake .. -G "Visual Studio 17 2022" -A x64 `
  -DCMAKE_TOOLCHAIN_FILE="$env:VCPKG_ROOT\scripts\buildsystems\vcpkg.cmake"

# Build
cmake --build . --config Release

# The executable will be located at:
# .\src\Release\fceux11.exe
```

## License and Legal Statement

This project is a derivative work based on the [FCEUX](https://fceux.com) emulator, distributed under the **GNU General Public License Version 2** (GPLv2).

### Legality

This project is developed in compliance with the GPLv2 license. The right to create derivative works, modify, and distribute software under GPLv2 is explicitly granted by the license terms. All original code and modifications remain subject to the GPLv2 license.

### Developer Commitments

- **License Compliance**: This project will remain licensed under GPLv2, ensuring all derivative works maintain the same licensing terms.
- **Source Availability**: Complete source code will be maintained and made available alongside any distributed binaries.
- **Attribution**: Proper attribution to original FCEUX authors and contributors is preserved in all distributions.
- **Transparency**: Changes and modifications are clearly documented in the project's version control history.

For full license details, see the [`COPYING`](COPYING) file.

---

# FCEUX11

一款现代化的任天堂娱乐系统（NES）模拟器，基于 [FCEUX](https://fceux.com) 2.6.6 衍生开发，采用 Qt6 界面框架，支持高 DPI 缩放与原生 Windows 集成。

## 关于本项目

FCEUX11 是 FCEUX 模拟器的衍生作品，依据 GNU 通用公共许可证第 2 版（GPLv2）条款创建。本项目专注于 Windows 11 平台，利用现代微软工具链和 UI 框架，提供高性能的 NES 模拟体验。

## 开发状态

本项目目前处于活跃开发阶段。主要关注点包括平台整合、代码库现代化和依赖更新。

## 开发方向

- **平台聚焦**: Windows 11 为唯一支持平台
- **工具链策略**: 近期以 msys64/MinGW-w64 为主力构建环境；MSVC 2022+ 迁移为远期计划
- **UI 迁移**: Qt6 配合 Windows 11 原生样式（暗色/亮色主题、Segoe UI 字体）
- **代码现代化**: 逐步使用 Rust 重构性能关键组件（远期计划）
- **整体改进**: 代码清理、性能优化和用户体验增强

## 工具链要求

### 必需工具（当前 - msys64/MinGW-w64）
- **msys2/msys64**: 主力构建环境
- **MinGW-w64 GCC**: 13.x 或更高版本（通过 msys2 pacman 安装）
- **CMake**: 3.20 或更高版本
- **Qt6**: UI 框架（Widgets、OpenGL、OpenGLWidgets）
- **SDL2**: 多媒体库

### 未来工具（远期 - MSVC 迁移）
- **Visual Studio 2022**（或更高版本）及 C++ 工作负载
- **CMake**: 3.28 或更高版本
- **vcpkg**: 依赖管理

> **注意**: 本项目当前基于 msys64/MinGW-w64 构建和运行。MSVC 支持是远期目标，尚不适合日常开发使用。

### 环境配置

```powershell
# 确保 vcpkg 已安装并完成集成（远期 MSVC 迁移时使用）
$env:VCPKG_ROOT = "C:\path\to\vcpkg"
```

## 编译说明

### Windows（msys64/MinGW-w64）—— 当前主力

```bash
# 在 MSYS2 MinGW64 终端中执行
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-cmake \
  mingw-w64-x86_64-qt6-base mingw-w64-x86_64-SDL2 \
  mingw-w64-x86_64-libarchive mingw-w64-x86_64-zlib

# 创建构建目录
rm -rf build
mkdir build
cd build

# 使用 CMake 配置
cmake .. -G "MinGW Makefiles" \
  -DCMAKE_BUILD_TYPE=Release

# 编译
mingw32-make -j$(nproc)

# 可执行文件位于：
# ./src/fceux11.exe
```

### Windows（MSVC）—— 远期计划（尚未就绪）

```powershell
# 创建构建目录
Remove-Item -Path build -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path build | Out-Null
cd build

# 使用 CMake 配置
cmake .. -G "Visual Studio 17 2022" -A x64 `
  -DCMAKE_TOOLCHAIN_FILE="$env:VCPKG_ROOT\scripts\buildsystems\vcpkg.cmake"

# 编译
cmake --build . --config Release

# 可执行文件位于：
# .\src\Release\fceux11.exe
```

## 许可证与法律声明

本项目是基于 [FCEUX](https://fceux.com) 模拟器的衍生作品，采用 **GNU 通用公共许可证第 2 版**（GPLv2）发布。

### 合法性声明

本项目的开发符合 GPLv2 许可证条款。根据 GPLv2 许可，创建衍生作品、修改和分发软件的权利已被明确授予。所有原始代码和修改均受 GPLv2 许可证约束。

### 开发者承诺

- **许可证合规**: 本项目将始终采用 GPLv2 许可证，确保所有衍生作品遵循相同的许可条款。
- **源代码可用性**: 完整源代码将与任何分发的二进制文件一起提供。
- **署名尊重**: 在所有分发版本中保留对原始 FCEUX 作者和贡献者的适当署名。
- **透明性**: 变更和修改将在项目的版本控制历史中清晰记录。

有关完整许可证详情，请参阅 [`COPYING`](COPYING) 文件。
