# FCEUX11

FCEUX11 是基于 [FCEUX](https://fceux.com) 的 NES/Famicom 模拟器衍生项目，面向 Windows 平台持续维护。

FCEUX11 is a derivative of the [FCEUX](https://fceux.com) NES/Famicom emulator, actively maintained for Windows.

---

## 简介

本项目致力于在保留 FCEUX 核心模拟精度的基础上，提供现代化的 Windows 原生体验，包括 Qt6 图形界面、多语言支持以及完善的调试与 TAS 工具集。

This project aims to preserve the core emulation accuracy of FCEUX while providing a modern, native Windows experience, including a Qt6-based user interface, multi-language support, and comprehensive debugging and TAS tooling.

---

## 主要功能

- **精确模拟**：完整支持 NES、Famicom 及各类扩展芯片（Mapper）的游戏运行。
- **调试工具**：内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码数据日志等。
- **TAS 编辑器**：支持逐帧录制与编辑输入，便于制作工具辅助速通（Tool-Assisted Speedrun）。
- **Lua 脚本**：通过 Lua 接口扩展功能，支持自定义 HUD、自动化测试等。
- **录像回放**：录制与播放游戏过程，支持 AVI 导出。
- **金手指**：支持 Game Genie 与原始金手指代码。
- **多语言界面**：提供简体中文、繁体中文及英文界面。
- **自定义调色板**：支持加载外部调色板文件，调整游戏色彩表现。
- **存档管理**：支持即时存档/读档及自动存档记录。

- **Accurate Emulation**: Full support for NES, Famicom, and various expansion chips (Mappers).
- **Debugging Tools**: Built-in CPU/PPU debugger, hex editor, RAM search/watch, code/data logger, and more.
- **TAS Editor**: Frame-by-frame recording and editing of inputs for Tool-Assisted Speedruns.
- **Lua Scripting**: Extend functionality via Lua scripts, including custom HUDs and automated testing.
- **Movie Recording**: Record and playback gameplay, with AVI export support.
- **Cheats**: Support for Game Genie and raw cheat codes.
- **Multi-language UI**: Interface available in Simplified Chinese, Traditional Chinese, and English.
- **Custom Palettes**: Load external palette files to adjust in-game color rendering.
- **Save States**: Instant save/load with automatic state history.

---

## 系统要求

### 运行环境

- **操作系统**：Windows 10 版本 1809（2018年10月更新）或更高版本的 64 位系统。
- **说明**：Windows 7、Windows 8 及 Windows 8.1 不支持。此限制由图形界面框架 Qt 6 的运行时依赖决定，并非人为划定。

- **Operating System**: 64-bit Windows 10 version 1809 (October 2018 Update) or later.
- **Note**: Windows 7, Windows 8, and Windows 8.1 are not supported. This limitation is determined by the runtime requirements of the Qt 6 GUI framework, not an arbitrary policy.

### 构建环境

如需从源码编译，请参阅 `docs/tech/Build_Guide.md`。

For building from source, please refer to `docs/tech/Build_Guide.md`.

---

## 下载与安装

预编译二进制文件可在 [GitHub Releases](https://github.com/Laffinty/FCEUX11/releases) 页面获取。下载后解压至任意目录即可运行，无需安装。

Precompiled binaries are available on the [GitHub Releases](https://github.com/Laffinty/FCEUX11/releases) page. Simply extract the archive to any directory and run—no installation required.

首次运行时，请确保目标目录具有写入权限，以便程序保存配置与存档文件。

Ensure the target directory has write permissions on first run so the program can save configuration and save-state files.

---

## 快速开始

1. 启动 `fceux11.exe`。
2. 通过 **File → Open ROM** 加载游戏文件（支持 `.nes`、`.fds`、`.nsf`、`.unf` 等格式）。
3. 使用键盘或手柄进行游戏；输入映射可在 **Options → Input Config** 中调整。
4. 按 **I** 快速存档，**P** 快速读档。

4. Press **I** for quick save and **P** for quick load.

---

## 支持与反馈

如遇问题或有功能建议，请通过 GitHub Issues 提交。提交前建议搜索现有议题，避免重复。

For issues or feature suggestions, please use GitHub Issues. We recommend searching existing issues before opening a new one to avoid duplicates.

---

## 许可

FCEUX11 基于 FCEUX 开发，采用 GNU GPLv2 许可证发布。完整许可证文本见 [COPYING](COPYING)。

FCEUX11 is based on FCEUX and distributed under the GNU GPLv2 license. The full license text is available in [COPYING](COPYING).
