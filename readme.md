<div align="center">

# FCEUX11

[![Version](https://img.shields.io/badge/version-v2.1.3-blue)](https://github.com/Laffinty/FCEUX11/releases)
[![License](https://img.shields.io/badge/license-GPL--v2-green)](COPYING)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![Qt](https://img.shields.io/badge/Qt-6.8%20LTS-41CD52?logo=qt)](https://www.qt.io)
[![CMake](https://img.shields.io/badge/CMake-4.0%2B-064F8C?logo=cmake)](https://cmake.org)

[下载 Releases](https://github.com/Laffinty/FCEUX11/releases) · [提交 Issues](https://github.com/Laffinty/FCEUX11/issues) · [查看更新日志](docs/ChangeLog.md)

</div>

---

## 简介 / Introduction

FCEUX11 是基于 FCEUX 的 NES/Famicom 模拟器衍生项目，针对 Windows 平台深度优化，提供完整的 mapper 兼容体系、调试工具、TAS 逐帧编辑器与录像回放功能；使用 Rust 全新重构的 CPU/PPU 核心，模块边界更清晰、状态更可控，严格保持硬件级时序与行为还原的同时仍有充足实时余量，全面提升实际运行的流畅度，支持简体中文、繁体中文、英文、日语、韩语、西班牙语、法语、德语、越南语、泰语、印地语（beta）、阿拉伯语（beta）共 12 种语言。

FCEUX11 is a derivative of the FCEUX NES/Famicom emulator, heavily optimized for Windows. It provides a complete mapper compatibility set, debugging tools, a frame-by-frame TAS editor and movie recording/playback. Its CPU and PPU cores have been fully rebuilt in Rust, giving cleaner module boundaries and more controllable state; hardware-level timing and behaviour are still preserved exactly, while plenty of real-time headroom remains, so games run noticeably smoother in practice. The interface is available in 12 languages: Simplified Chinese, Traditional Chinese, English, Japanese, Korean, Spanish, French, German, Vietnamese, Thai, Hindi (beta) and Arabic (beta).

---

## 主要功能 / Features

| 中文 | English |
|------|---------|
| **精确模拟**：完整支持 NES、Famicom 及各类 Mapper 扩展芯片，画面与音效高度还原。 | **Accurate Emulation**: Full NES / Famicom / mapper support with faithful graphics and audio. |
| **独立 Rust CPU 与 PPU**：自 v2.0 起 6502 CPU 由 Rust 完整实现（`fceux11-core` crate），自 v2.1.1 起 PPU 同样由 Rust 完整实现（`fceux11-ppu` crate），二者相互独立、互不共享状态。 | **Independent Rust CPU and PPU**: The 6502 CPU has been fully implemented in Rust since v2.0 (`fceux11-core` crate); the PPU has been fully implemented in Rust since v2.1.1 (`fceux11-ppu` crate). The two engines are independent of each other and share no state. |
| **调试工具**：内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码/数据日志。 | **Debugging Tools**: CPU/PPU debugger, hex editor, RAM search/watch, code/data logger. |
| **TAS 编辑器**：逐帧录制并精确编辑按键输入，轻松制作工具辅助速通（TAS）录像。 | **TAS Editor**: Frame-by-frame recording and precise input editing for Tool-Assisted Speedruns. |
| **Lua 脚本**：通过 Lua 接口编写脚本，实现自定义屏幕叠加显示、自动化操作、内存数据读取等高级玩法。 | **Lua Scripting**: Custom on-screen displays, automation, and memory access via Lua. |
| **录像回放**：录制完整游戏过程并支持回放，可导出为 AVI 视频。 | **Movie Recording**: Record and replay playthroughs, export to AVI. |
| **金手指**：支持 Game Genie 与原始金手指代码，轻松修改游戏内容。 | **Cheats**: Game Genie and raw cheat code support. |
| **多语言界面**：支持 **12 种语言** —— 简体中文、繁体中文、英文、日语、韩语、西班牙语、法语、德语、越南语、泰语、印地语（beta）、阿拉伯语（beta）；首启自动按系统区域设置匹配语言，切换语言后菜单、对话框即时全部重译；阿拉伯语自动启用从右到左布局。 | **Multi-language UI**: **12 languages** — Simplified Chinese, Traditional Chinese, English, Japanese, Korean, Spanish, French, German, Vietnamese, Thai, Hindi (beta), and Arabic (beta). Auto-detected from system locale on first launch, instant retranslate on switch, and automatic right-to-left layout for Arabic. |
| **自定义调色板**：加载外部调色板文件，自由调整画面色彩。 | **Custom Palettes**: Load custom palette files to fine-tune color rendering. |
| **即时存档**：随时随地保存 / 读取进度，支持自动存档历史记录。 | **Save States**: Save / load anywhere with automatic state history. |

---

## 系统要求 / System Requirements

### 运行环境 / Runtime

| 项目 / Item | 要求 / Requirement |
|-------------|-------------------|
| **操作系统 / OS** | Windows 11 22H2+ (64-bit) |
| **说明 / Note** | Windows 7/8/8.1/10 不支持，由 Qt 6.8 LTS 运行时依赖及 Windows 平台深度优化策略决定。Windows 7, 8, 8.1, and 10 are not supported, determined by Qt 6.8 LTS runtime requirements and the Windows platform optimization strategy. |

### 自行编译 / Build from Source

需要 **Visual Studio 2022 Community**（勾选「使用 C++ 的桌面开发」）与 **Rust**（[rustup](https://rustup.rs) 默认安装）。首次编译约 30-60 分钟，后续增量编译 1-3 分钟；详细说明（含常见错误修复与高级选项）见 [`docs/BuildGuide.md`](docs/BuildGuide.md)。

```powershell
git clone https://github.com/Laffinty/FCEUX11.git
cd FCEUX11
.\scripts\setup_vcpkg.ps1
$env:VCPKG_ROOT = "$PWD\vcpkg"          # 必设，do_build.ps1 据此定位 vcpkg
.\scripts\do_build.ps1 -Config Release -BuildDir build-rust-ppu
# 产物：build-rust-ppu\src\fceux11.exe
```

> 自 v2.1.1 起，Rust CPU 与 Rust PPU 是唯一实现，不存在 C++ 引擎可回退；自 v2.1.1.7 Step C（C++ PPU 退役）起，`-DFCEUX11_RUST_PPU=OFF` 是配置期错误。`build-rust-ppu/` 是当前在维护的验证目录；历史遗留的 `build/`（缓存为 OFF，已无法 configure）须先删除或重新配置。

---

## 下载与安装 / Download & Install

预编译二进制文件可在 **[GitHub Releases](https://github.com/Laffinty/FCEUX11/releases)** 页面获取。下载后解压至任意目录即可运行，**无需安装**。
Precompiled binaries are available on the **[GitHub Releases](https://github.com/Laffinty/FCEUX11/releases)** page. Simply extract the archive to any directory and run — **no installation required**.

> **注意**：首次运行时，请确保目标目录具有**写入权限**，以便程序保存配置与存档文件。
> Ensure the target directory has **write permissions** on first run so the program can save configuration and save-state files.

---

## 快速开始 / Quick Start

1. 启动 `fceux11.exe`。
2. 通过 **File → Open ROM** 加载游戏（支持 `.nes` / `.fds` / `.nsf` / `.unf`）。
3. 键盘或手柄游戏；输入映射在 **Options → Input Config** 调整。
4. **I** 快速存档，**P** 快速读档。

Launch `fceux11.exe`, load a game via **File → Open ROM**, play with keyboard or gamepad (remap in **Options → Input Config**). Press **I** to quick-save, **P** to quick-load.

---

## 版本历史 / Changelog

详见 [ChangeLog.md](docs/ChangeLog.md)。当前稳定版为 **v2.1.3**。
See [ChangeLog.md](docs/ChangeLog.md). Current stable release is **v2.1.3**.

---

## 许可 / License

FCEUX11 基于 FCEUX 开发，采用 **GNU GPLv2** 许可证发布。完整许可证文本见 [COPYING](COPYING)。
FCEUX11 is based on FCEUX and distributed under the **GNU GPLv2** license. The full license text is available in [COPYING](COPYING).
