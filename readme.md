<div align="center">

# FCEUX11

[![Version](https://img.shields.io/badge/version-v1.8-blue)](https://github.com/Laffinty/FCEUX11/releases)
[![License](https://img.shields.io/badge/license-GPL--v2-green)](COPYING)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![Qt](https://img.shields.io/badge/Qt-6.8%20LTS-41CD52?logo=qt)](https://www.qt.io)
[![CMake](https://img.shields.io/badge/CMake-4.0%2B-064F8C?logo=cmake)](https://cmake.org)

[下载 Releases](https://github.com/Laffinty/FCEUX11/releases) · [提交 Issues](https://github.com/Laffinty/FCEUX11/issues) · [查看更新日志](CHANGELOG.md)

</div>

---

## 简介 / Introduction

**FCEUX11** 是基于 [FCEUX](https://fceux.com) 的 NES/Famicom 模拟器衍生项目，针对 **Windows** 平台深度优化。在继承 FCEUX 卓越模拟精度的同时，采用 Qt6 图形界面重塑了现代 Windows 原生体验，并提供 12 种语言的多语言支持，以及面向开发者和速通玩家的调试与 TAS 工具集。项目持续推进内部核心重构与性能优化工作，致力于让模拟器在高负载场景下依旧保持高效、流畅与稳定。

**FCEUX11** is a derivative of the [FCEUX](https://fceux.com) NES/Famicom emulator, optimized for **Windows**. It inherits FCEUX's renowned emulation accuracy while delivering a polished, modern Windows-native experience powered by Qt6, with 12-language localization and a full suite of debugging and TAS tools for developers and speedrunners. Ongoing internal refactoring and performance optimization keeps the emulator efficient, smooth, and stable even under heavy load.

---

## 主要功能 / Features

| 中文 | English |
|------|---------|
| **精确模拟**：完整支持 NES、Famicom 及各类 Mapper 扩展芯片，画面与音效高度还原。 | **Accurate Emulation**: Full NES / Famicom / mapper support with faithful graphics and audio. |
| **调试工具**：内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码/数据日志。 | **Debugging Tools**: CPU/PPU debugger, hex editor, RAM search/watch, code/data logger. |
| **TAS 编辑器**：逐帧录制并精确编辑按键输入，轻松制作工具辅助速通（TAS）录像。 | **TAS Editor**: Frame-by-frame recording and precise input editing for Tool-Assisted Speedruns. |
| **Lua 脚本**：通过 Lua 接口编写脚本，实现自定义屏幕叠加显示、自动化操作、内存数据读取等高级玩法。 | **Lua Scripting**: Custom on-screen displays, automation, and memory access via Lua. |
| **录像回放**：录制完整游戏过程并支持回放，可导出为 AVI 视频。 | **Movie Recording**: Record and replay playthroughs, export to AVI. |
| **金手指**：支持 Game Genie 与原始金手指代码，轻松修改游戏内容。 | **Cheats**: Game Genie and raw cheat code support. |
| **多语言界面**：支持 **12 种语言** —— 简体中文、繁体中文、英文、日语、韩语、西班牙语、法语、德语、越南语、泰语、印地语（beta）、阿拉伯语（beta）；首启自动按系统区域设置匹配语言，切换语言后菜单、对话框即时全部重译；阿拉伯语自动启用从右到左布局。 | **Multi-language UI**: **12 languages** — Simplified Chinese, Traditional Chinese, English, Japanese, Korean, Spanish, French, German, Vietnamese, Thai, Hindi (beta), and Arabic (beta). Auto-detected from system locale on first launch, instant retranslate on switch, and automatic right-to-left layout for Arabic. |
| **自定义调色板**：加载外部调色板文件，自由调整画面色彩。 | **Custom Palettes**: Load custom palette files to fine-tune color rendering. |
| **即时存档**：随时随地保存 / 读取进度，支持自动存档历史记录。 | **Save States**: Save / load anywhere with automatic state history. |

---

> **已知移除 / Known Removal**：自 v1.15 (hotfix4) 起，**NetPlay（联机对战）** 正式移除——上游 FCEUX 的该功能本已不可用（`config.cpp` 原注 "netplay is broken"），本版本清理了其 CLI 选项与不可达代码，核心 `netplay.cpp` 保留以维持存档兼容。详见 [CHANGELOG.md](CHANGELOG.md)。
> Since v1.15 (hotfix4), **NetPlay** has been formally removed — it was already broken upstream. Related CLI options and unreachable code were cleaned up; core `netplay.cpp` is kept for savestate compatibility. See [CHANGELOG.md](CHANGELOG.md).

## 系统要求 / System Requirements

### 运行环境 / Runtime

| 项目 / Item | 要求 / Requirement |
|-------------|-------------------|
| **操作系统 / OS** | Windows 11 22H2+ (64-bit) |
| **说明 / Note** | Windows 7/8/8.1/10 不支持，由 Qt 6.8 LTS 运行时依赖及 Windows 平台深度优化策略决定。Windows 7, 8, 8.1, and 10 are not supported, determined by Qt 6.8 LTS runtime requirements and the Windows platform optimization strategy. |

### 自行编译 / Build from Source

需要 **Visual Studio 2022 Community**（勾选「使用 C++ 的桌面开发」）与 **Rust**（[rustup](https://rustup.rs) 默认安装）。详见 [`docs/BuildGuide.md`](docs/BuildGuide.md)。

```powershell
git clone https://github.com/Laffinty/FCEUX11.git
cd FCEUX11
.\scripts\setup_vcpkg.ps1
$env:VCPKG_ROOT = "$PWD\vcpkg"          # 必设，do_build.ps1 据此定位 vcpkg
.\scripts\do_build.ps1 -Config Release  # 产物：build\src\fceux11.exe
```

`do_build.ps1` 会自动通过 `vswhere` 发现 Visual Studio 自带的 Ninja——裸 PATH 查不到 `ninja.exe` 不代表未安装，也无需另装一份。首次编译约 30-60 分钟（主要为 Qt6 下载与编译），后续增量编译 1-3 分钟。

> 详细说明（含常见错误修复、高级选项）见 [`docs/BuildGuide.md`](docs/BuildGuide.md)。

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

## 质量保障 / Quality Assurance — F11QA

FCEUX11 内置一套名为 **F11QA**（原 KagamiQA）的双 Oracle 自动化质量保障系统，在 CI 上常驻运行：

| 组件 | 说明 |
|------|------|
| **Oracle A（回归测试）** | CTest 注册测试 + 120 项扁平清单中的 unit/harness 条目，每次 push 全量运行 |
| **Oracle B（硬件精度测试）** | 78 项 rom-suite 代表（vendor_state 三态）+ 177 个 [blargg](https://github.com/christopherpow/nes-test-roms) `$6000` 协议 ROM 全量批处理，覆盖 CPU/PPU/APU/MMC3 |
| **迁移矩阵** | 每次 CI 产出 `f11qa_migration_matrix.json`（artifact），追踪 PASS→FAIL 回归与 FAIL→PASS 进展 |
| **R4 门禁** | `total==120`、`fail_to_pass==0`、vendor_state 三态、advisory≤15%、grade∉{D,E} 机器校验 |

> **当前 CI 矩阵**（`f11qa.yml`，`engine.git_rev=f19fa7d`，R4 gate passed，grade **B**）：
> **120 项 / 106 PASS / 14 FAIL**；Oracle A **42P/0F**，Oracle B **64P/14F**
> （blargg 全量 **145P/32F**）；advisory known-limit 14/120 = 11.7%（cap 15%）；
> **0 条 PASS→FAIL 漂移**。数字以 CI artifact 的 `engine.git_rev` 为准；
> 详见 [`docs/tech/F11QA.md`](docs/tech/F11QA.md) 与
> [`docs/history/plans/FCEUX11-v1.8_收口验收.md`](docs/history/plans/FCEUX11-v1.8_收口验收.md)。

**实现细节、原理、独立化运行**请参阅 [`docs/tech/F11QA.md`](docs/tech/F11QA.md)。

FCEUX11 ships **F11QA** (formerly KagamiQA), a dual-oracle automated quality assurance system that runs continuously in CI:

| Component | Description |
|-----------|-------------|
| **Oracle A (regression)** | CTest suite + unit/harness entries of the 120-case flat manifest, full run on every push |
| **Oracle B (hardware accuracy)** | 78 rom-suite representatives (vendor_state tri-state) + 177 [blargg](https://github.com/christopherpow/nes-test-roms) `$6000`-protocol ROMs covering CPU/PPU/APU/MMC3 |
| **Migration Matrix** | `f11qa_migration_matrix.json` per CI run, tracking PASS→FAIL regressions and FAIL→PASS progress |
| **R4 Gate** | Machine-checked: `total==120`, `fail_to_pass==0`, vendor_state, advisory≤15%, grade∉{D,E} |

> **Current CI matrix** (`f11qa.yml`, `engine.git_rev=f19fa7d`, R4 gate passed, grade **B**):
> **120 cases / 106 PASS / 14 FAIL**; Oracle A **42P/0F**, Oracle B **64P/14F**
> (blargg batch **145P/32F**); advisory known-limits 14/120 = 11.7% (cap 15%);
> **0 PASS→FAIL drifts**. Source of truth is `engine.git_rev` in the CI artifact;
> see [`docs/tech/F11QA.md`](docs/tech/F11QA.md) and the
> [v1.8 closeout sheet](docs/history/plans/FCEUX11-v1.8_收口验收.md).

**For implementation details, principles, and standalone operation**, see [`docs/tech/F11QA.md`](docs/tech/F11QA.md).

---

## 版本历史 / Changelog

详见 [CHANGELOG.md](CHANGELOG.md)。当前主线为 **v1.8**（上一稳定发布 **v1.17**）。
See [CHANGELOG.md](CHANGELOG.md). Mainline is **v1.8** (previous stable **v1.17**).

---

## 许可 / License

FCEUX11 基于 FCEUX 开发，采用 **GNU GPLv2** 许可证发布。完整许可证文本见 [COPYING](COPYING)。
FCEUX11 is based on FCEUX and distributed under the **GNU GPLv2** license. The full license text is available in [COPYING](COPYING).
