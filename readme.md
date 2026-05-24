# FCEUX11

基于 [FCEUX](https://fceux.com) 的 NES 模拟器衍生项目，Windows 11 专属维护版本。

**许可证**: GPLv2 | **工具链**: MSVC 2022+ + vcpkg | **状态**: 活跃开发

---

## 项目概述

FCEUX11 是 FCEUX 的衍生作品，专注于 Windows 11 平台。v0.2.1 完成了工具链统一，全面迁移至 MSVC 2022+，所有依赖通过 vcpkg 管理，不再支持旧版 Windows。

### 与上游 FCEUX 的区别

- **仅支持 Windows 11**：不保证 Windows 10/8/7 的兼容性
- **工具链统一**：MSVC 2022+ 单一构建路径，旧版跨平台工具链支持已移除
- **品牌独立**：自有版本号体系与发布周期

---

## 环境要求

### 必需工具

| 工具 | 版本要求 | 说明 |
|------|----------|------|
| Visual Studio 2022 | Build Tools 或更高 | 需要 "使用 C++ 的桌面开发" 工作负载 |
| CMake | ≥ 3.28 | |
| vcpkg | 最新 stable | 通过 `VCPKG_ROOT` 环境变量自动检测 |
| Rust | stable-x86_64-pc-windows-msvc | 可选，未安装时自动跳过 Rust 模块 |

> **注意**：旧版跨平台工具链支持已在 v0.2.1 中移除，项目仅通过 MSVC 编译。

### vcpkg 依赖

依赖声明在项目根目录 `vcpkg.json`，CMake 配置时自动解析：

```
qtbase[opengl,widgets]  ≥ 6.0.0    # Qt6 UI + OpenGL
qttools[linguist]       ≥ 6.0.0    # 翻译工具
sdl2                    ≥ 2.0.0    # 多媒体
libarchive              ≥ 3.0.0    # 存档/压缩
zlib                    ≥ 1.2.0
liblzma                 ≥ 5.0.0
```

---

## 快速开始

### 一键编译

```powershell
# 设置 vcpkg 路径（根据实际安装位置调整）
$env:VCPKG_ROOT = "D:\vcpkg"

# 运行构建脚本
.\do_build.bat
```

构建产物：`build/src/fceux11.exe`

### 手动编译

```powershell
# 1. 初始化 MSVC 环境
& "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 2. 配置（推荐 Ninja 生成器，支持并行编译）
cmake -B build -G Ninja

# 3. 编译
cmake --build build --config Release --parallel
```

### 首次编译说明

- **vcpkg 首次安装较慢**：首次 `cmake configure` 会下载并编译所有依赖，约 10~30 分钟
- **BUILD_TS 时间戳**：每次 configure 都会变化，可能触发全量重编译，日常开发可临时注释 `src/CMakeLists.txt` 中的该宏
- **lrelease 路径**：Qt6 LinguistTools 必须显式声明在 `vcpkg.json` 中（`qtbase` 不自带）

---

## 项目架构

```
fceux11_utils        通用工具（字符串、CRC32/MD5、内存管理）
fceux11_boards       NES 主板/Mapper 支持库
fceux11_core         模拟器核心（CPU、PPU、APU、输入、存档）
fceux11_drivers_common   跨平台驱动公共层（视频滤镜、参数解析）
fceux11_drivers_qt   Qt6 前端（主窗口、调试器、TAS 编辑器、配置）
fceux11_rust         Rust 性能模块（可选）
fceux11              最终可执行文件
```

---

## 技术栈

| 组件 | 现状 | 说明 |
|------|------|------|
| **编译器** | MSVC 2022+ (v143) | `/permissive-` 强制标准合规 |
| **C++ 标准** | C++20 | |
| **UI 框架** | Qt6 (Widgets, OpenGL) | |
| **多媒体** | SDL2 | |
| **依赖管理** | vcpkg manifest 模式 | |
| **安全编译** | `/W4 /guard:cf /GS /sdl` | |
| **Rust 模块** | x86_64-pc-windows-msvc | 可选，未安装时自动禁用 |

---

## 推荐开发流程

```powershell
# Ninja 生成器（推荐，日常开发）
cmake -B build -G Ninja
cmake --build build --parallel

# Visual Studio 生成器（支持 IDE 调试）
cmake -B build -G "Visual Studio 17 2022" -A x64
cmake --build build --config Release --parallel
```

---

## 已知问题

| 问题 | 状态 | 说明 |
|------|------|------|
| `asm.cpp.obj` 偶发缺失 | 已缓解 | NMake 单线程下 `cmake_cl_compile_depends` 偶发失败，使用 Ninja/VS 生成器可避免 |
| `QWindowsWindowFunctions` | 已修复 | Qt6 已移除，添加 `#if QT_VERSION < 6.0.0` 条件编译 |
| `QAction::parentWidget()` | 已修复 | 替换为 `qobject_cast<QWidget*>(parent())` |
| Manifest 重复嵌入 | 已修复 | `.rc` 文件已包含 manifest，CMake 加了 `/MANIFEST:NO` |
| `ntdll` 符号缺失 | 已修复 | Rust std 在 MSVC 下依赖 `ntdll`，已加入链接库 |

---

## 许可证

本项目基于 [FCEUX](https://fceux.com) 模拟器，采用 **GNU GPLv2** 发布。

完整许可证文本参见 [`COPYING`](COPYING)。

---

**v0.2.1 — MSVC + vcpkg Single-Track Build**