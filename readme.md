<div align="center">

# FCEUX11

[![Version](https://img.shields.io/badge/version-v1.0.0-blue)](https://github.com/Laffinty/FCEUX11/releases)
[![License](https://img.shields.io/badge/license-GPL--v2-green)](COPYING)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![Qt](https://img.shields.io/badge/Qt-6.8%20LTS-41CD52?logo=qt)](https://www.qt.io)
[![CMake](https://img.shields.io/badge/CMake-4.0%2B-064F8C?logo=cmake)](https://cmake.org)

[下载 Releases](https://github.com/Laffinty/FCEUX11/releases) · [提交 Issues](https://github.com/Laffinty/FCEUX11/issues) · [查看更新日志](CHANGELOG.md)

</div>

---

## 简介 / Introduction

**FCEUX11** 是基于 [FCEUX](https://fceux.com) 的 NES/Famicom 模拟器衍生项目，面向 **Windows 11** 平台持续维护。在保留 FCEUX 核心模拟精度的基础上，提供现代化的 Windows 原生体验，包括 Qt6 图形界面、多语言支持以及完善的调试与 TAS 工具集。

**FCEUX11** is a derivative of the [FCEUX](https://fceux.com) NES/Famicom emulator, actively maintained for **Windows 11**. It preserves the core emulation accuracy of FCEUX while providing a modern, native Windows experience, including a Qt6-based user interface, multi-language support, and comprehensive debugging and TAS tooling.

---

## 主要功能 / Features

| 中文 | English |
|------|---------|
| **精确模拟** — 完整支持 NES、Famicom 及各类扩展芯片（Mapper）的游戏运行。 | **Accurate Emulation** — Full support for NES, Famicom, and various expansion chips (Mappers). |
| **调试工具** — 内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码数据日志等。 | **Debugging Tools** — Built-in CPU/PPU debugger, hex editor, RAM search/watch, code/data logger, and more. |
| **TAS 编辑器** — 支持逐帧录制与编辑输入，便于制作工具辅助速通。 | **TAS Editor** — Frame-by-frame recording and editing of inputs for Tool-Assisted Speedruns. |
| **Lua 脚本** — 通过 Lua 接口扩展功能，支持自定义 HUD、自动化测试等。 | **Lua Scripting** — Extend functionality via Lua scripts, including custom HUDs and automated testing. |
| **录像回放** — 录制与播放游戏过程，支持 AVI 导出。 | **Movie Recording** — Record and playback gameplay, with AVI export support. |
| **金手指** — 支持 Game Genie 与原始金手指代码。 | **Cheats** — Support for Game Genie and raw cheat codes. |
| **多语言界面** — 提供简体中文、繁体中文及英文界面。 | **Multi-language UI** — Interface available in Simplified Chinese, Traditional Chinese, and English. |
| **自定义调色板** — 支持加载外部调色板文件，调整游戏色彩表现。 | **Custom Palettes** — Load external palette files to adjust in-game color rendering. |
| **存档管理** — 支持即时存档/读档及自动存档记录。 | **Save States** — Instant save/load with automatic state history. |

---

## 系统要求 / System Requirements

### 运行环境 / Runtime

| 项目 / Item | 要求 / Requirement |
|-------------|-------------------|
| **操作系统 / OS** | Windows 11 22H2+ (64-bit) |
| **说明 / Note** | Windows 7/8/8.1/10 不支持，由 Qt 6.8 LTS 运行时依赖及 v0.3.x 的 Win11 独占优化策略决定。 Windows 7, 8, 8.1, and 10 are not supported, determined by Qt 6.8 LTS runtime requirements and the v0.3.x Win11-exclusive optimization strategy. |

### 构建环境 / Build Environment

| 项目 / Item | 要求 / Requirement |
|-------------|-------------------|
| **编译器 / Compiler** | MSVC 2022 19.36 (VS 17.6) 或更高 |
| **CMake** | 4.0 或更高 / 4.0+ |
| **生成器 / Generator** | Ninja（推荐）或 Visual Studio 2022 / Ninja (recommended) or Visual Studio 2022 |
| **包管理器 / Package Manager** | vcpkg（manifest 模式）/ vcpkg (manifest mode) |
| **Qt** | 6.8 LTS |
| **C++ 标准 / Standard** | C++20 |

> 如需从源码编译，请参阅 [`docs/tech/13_构建系统与CI矩阵.txt`](docs/tech/13_构建系统与CI矩阵.txt) 与 [`docs/tech/00_NES架构与FCEUX11项目导览.txt`](docs/tech/00_NES架构与FCEUX11项目导览.txt)。
> For building from source, please refer to [`docs/tech/13_构建系统与CI矩阵.txt`](docs/tech/13_构建系统与CI矩阵.txt) and [`docs/tech/00_NES架构与FCEUX11项目导览.txt`](docs/tech/00_NES架构与FCEUX11项目导览.txt).

---

## 下载与安装 / Download & Install

预编译二进制文件可在 **[GitHub Releases](https://github.com/Laffinty/FCEUX11/releases)** 页面获取。下载后解压至任意目录即可运行，**无需安装**。

Precompiled binaries are available on the **[GitHub Releases](https://github.com/Laffinty/FCEUX11/releases)** page. Simply extract the archive to any directory and run—**no installation required**.

> **注意**：首次运行时，请确保目标目录具有**写入权限**，以便程序保存配置与存档文件。
> Ensure the target directory has **write permissions** on first run so the program can save configuration and save-state files.

---

## 快速开始 / Quick Start

1. 启动 `fceux11.exe`。
2. 通过 **File → Open ROM** 加载游戏文件（支持 `.nes`、`.fds`、`.nsf`、`.unf` 等格式）。
3. 使用键盘或手柄进行游戏；输入映射可在 **Options → Input Config** 中调整。
4. 按 **I** 快速存档，**P** 快速读档。

1. Launch `fceux11.exe`.
2. Load a game via **File → Open ROM** (supports `.nes`, `.fds`, `.nsf`, `.unf`, etc.).
3. Play with keyboard or gamepad; remap inputs in **Options → Input Config**.
4. Press **I** for quick save and **P** for quick load.

---

## 版本历史 / Changelog

详见 [CHANGELOG.md](CHANGELOG.md)。

v1.0 是首个正式版（基于 v0.3.16 LTS 收官），整合 v0.3.0 ~ v0.3.16 全部
17 个子版本 + 2 个整合检查点。完整 Release Notes 见
[`docs/tech/22_v0_3_x发布说明.txt`](docs/tech/22_v0_3_x发布说明.txt)。
编译指南见 [`docs/v1.0_BuildGuide.md`](docs/v1.0_BuildGuide.md)。

See [CHANGELOG.md](CHANGELOG.md) for version history.

v1.0 is the first official stable release (based on the v0.3.16 LTS
closure), integrating all 17 sub-versions and 2 integration checkpoints
from v0.3.0 to v0.3.16. Full release notes:
[`docs/tech/22_v0_3_x发布说明.txt`](docs/tech/22_v0_3_x发布说明.txt).
Build guide: [`docs/v1.0_BuildGuide.md`](docs/v1.0_BuildGuide.md).

---

## 许可 / License

FCEUX11 基于 FCEUX 开发，采用 **GNU GPLv2** 许可证发布。完整许可证文本见 [COPYING](COPYING)。

FCEUX11 is based on FCEUX and distributed under the **GNU GPLv2** license. The full license text is available in [COPYING](COPYING).
