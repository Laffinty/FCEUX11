# FCEUX11

A Nintendo Entertainment System (NES) emulator project.

## Development Status

This project is currently in active development. Key areas of focus include platform consolidation, codebase modernization, and dependency updates.

## Development Goals

- **Platform Focus**: Targeting Windows 11 as the primary supported platform
- **Code Modernization**: Gradual refactoring using Rust for performance-critical components
- **Dependency Update**: Transitioning from Qt to modern UI frameworks
- **General Improvements**: Code cleanup, performance optimizations, and quality-of-life enhancements

## Toolchain Requirements

### Required Tools
- **MSYS2**: MinGW-w64 development environment
- **CMake**: Build system (version 3.10 or later recommended)
- **Qt5**: UI framework (for current implementation)
- **SDL2**: Multimedia library

### Environment Setup

```powershell
# Set environment variables
$env:PATH = "D:\msys64\mingw64\bin;D:\msys64\usr\bin;$env:PATH"
$env:MINGW_PREFIX = "D:\msys64\mingw64"
```

## Building Instructions

### Windows (MinGW)

```powershell
# Create build directory
Remove-Item -Path build -Recurse -Force
New-Item -ItemType Directory -Force -Path build
cd build

# Configure with CMake
cmake .. -G "MinGW Makefiles" -DCMAKE_CXX_FLAGS="-DPSS_STYLE=2" -DCMAKE_C_FLAGS="-DPSS_STYLE=2"

# Build
mingw32-make -j4

# Copy dependencies
cd ..
& .\scripts\copy_dependencies.ps1 -Executable "build\src\fceux.exe" -OutputDir "build\src\dependencies"

# Run
& .\build\src\dependencies\run_fceux.bat
```

## License and Legal Statement

This project is a derivative work based on the FCEUX emulator, distributed under the **GNU General Public License Version 2** (GPLv2).

### Legality

This project is developed in compliance with the GPLv2 license. The right to create derivative works, modify, and distribute software under GPLv2 is explicitly granted by the license terms. All original code and modifications remain subject to the GPLv2 license.

### Developer Commitments

- **License Compliance**: This project will remain licensed under GPLv2, ensuring all derivative works maintain the same licensing terms.
- **Source Availability**: Complete source code will be maintained and made available alongside any distributed binaries.
- **Attribution**: Proper attribution to original authors and contributors will be preserved in all distributions.
- **Transparency**: Changes and modifications will be clearly documented in the project's version control history.

For full license details, see the [LICENSE](LICENSE) file.

---

# FCEUX11

一个任天堂娱乐系统（NES）模拟器项目。

## 开发状态

本项目目前处于活跃开发阶段。主要关注点包括平台整合、代码库现代化和依赖更新。

## 开发方向

- **平台聚焦**: 以 Windows 11 作为主要支持平台
- **代码现代化**: 逐步使用 Rust 重构性能关键组件
- **依赖更新**: 从 Qt 过渡到现代 UI 框架
- **整体改进**: 代码清理、性能优化和用户体验增强

## 工具链要求

### 必需工具
- **MSYS2**: MinGW-w64 开发环境
- **CMake**: 构建系统（建议版本 3.10 或更高）
- **Qt5**: UI 框架（当前实现使用）
- **SDL2**: 多媒体库

### 环境配置

```powershell
# 设置环境变量
$env:PATH = "D:\msys64\mingw64\bin;D:\msys64\usr\bin;$env:PATH"
$env:MINGW_PREFIX = "D:\msys64\mingw64"
```

## 编译说明

### Windows (MinGW)

```powershell
# 创建构建目录
Remove-Item -Path build -Recurse -Force
New-Item -ItemType Directory -Force -Path build
cd build

# 使用 CMake 配置
cmake .. -G "MinGW Makefiles" -DCMAKE_CXX_FLAGS="-DPSS_STYLE=2" -DCMAKE_C_FLAGS="-DPSS_STYLE=2"

# 编译
mingw32-make -j4

# 复制依赖项
cd ..
& .\scripts\copy_dependencies.ps1 -Executable "build\src\fceux.exe" -OutputDir "build\src\dependencies"

# 运行
& .\build\src\dependencies\run_fceux.bat
```

## 许可证与法律声明

本项目是基于 FCEUX 模拟器的衍生作品，采用 **GNU 通用公共许可证第 2 版**（GPLv2）发布。

### 合法性声明

本项目的开发符合 GPLv2 许可证条款。根据 GPLv2 许可，创建衍生作品、修改和分发软件的权利已被明确授予。所有原始代码和修改均受 GPLv2 许可证约束。

### 开发者承诺

- **许可证合规**: 本项目将始终采用 GPLv2 许可证，确保所有衍生作品遵循相同的许可条款。
- **源代码可用性**: 完整源代码将与任何分发的二进制文件一起提供。
- **署名尊重**: 在所有分发版本中保留对原始作者和贡献者的适当署名。
- **透明性**: 变更和修改将在项目的版本控制历史中清晰记录。

有关完整许可证详情，请参阅 [LICENSE](LICENSE) 文件。