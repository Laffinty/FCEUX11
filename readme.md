<div align="center">

# FCEUX11

[![Version](https://img.shields.io/badge/version-v1.16-blue)](https://github.com/Laffinty/FCEUX11/releases)
[![License](https://img.shields.io/badge/license-GPL--v2-green)](COPYING)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![Qt](https://img.shields.io/badge/Qt-6.8%20LTS-41CD52?logo=qt)](https://www.qt.io)
[![CMake](https://img.shields.io/badge/CMake-4.0%2B-064F8C?logo=cmake)](https://cmake.org)

[下载 Releases](https://github.com/Laffinty/FCEUX11/releases) · [提交 Issues](https://github.com/Laffinty/FCEUX11/issues) · [查看更新日志](CHANGELOG.md)

</div>

---

## 简介 / Introduction

**FCEUX11** 是基于 [FCEUX](https://fceux.com) 的 NES/Famicom 模拟器衍生项目，针对 **Windows** 平台深度优化。在继承 FCEUX 卓越模拟精度的同时，采用 Qt6 图形界面重塑了现代 Windows 原生体验，并提供 12 种语言的多语言支持，以及面向开发者和速通玩家的调试与 TAS 工具集。项目持续推进内部核心重构与性能优化工作，致力于让模拟器在高负载场景下依旧保持高效、流畅与稳定。无论你是重温经典红白机游戏，还是深入 ROM 修改与研究，FCEUX11 都能以最顺畅的方式运行你的 NES 游戏。

**FCEUX11** is a derivative of the [FCEUX](https://fceux.com) NES/Famicom emulator, optimized for **Windows**. It inherits FCEUX's renowned emulation accuracy while delivering a polished, modern Windows-native experience powered by Qt6. Multi-language support (12 languages) and a full suite of debugging and TAS tools are built in for developers and speedrunners alike. Ongoing internal refactoring and performance optimization work keeps the emulator efficient, smooth, and stable — even under heavy load. Whether you're revisiting childhood classics or diving into ROM hacking, FCEUX11 runs your NES games smoothly and looks great doing it.

---

## 主要功能 / Features

| 中文 | English |
|------|---------|
| **精确模拟**：完整支持 NES、Famicom 及各类 Mapper 扩展芯片，画面与音效高度还原，带来原汁原味的 8 位体验。 | **Accurate Emulation**: Full NES, Famicom, and mapper support with faithful graphics and audio, delivering the authentic 8-bit experience just as you remember it. |
| **调试工具**：内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码/数据日志，满足 ROM 修改与深度研究需求。 | **Debugging Tools**: CPU/PPU debugger, hex editor, RAM search/watch, and code/data logger, covering everything you need for ROM hacking and in-depth analysis. |
| **TAS 编辑器**：逐帧录制并精确编辑按键输入，轻松制作工具辅助速通（TAS）录像，挑战操作极限。 | **TAS Editor**: Frame-by-frame recording and precise input editing to craft perfect Tool-Assisted Speedruns and push gameplay to its limits. |
| **Lua 脚本**：通过 Lua 接口编写脚本，实现自定义屏幕叠加显示、自动化操作、内存数据读取等高级玩法。 | **Lua Scripting**: Write Lua scripts to create custom on-screen displays, automate gameplay, or read memory, extending the emulator however you like. |
| **录像回放**：录制完整游戏过程并支持回放，可导出为 AVI 视频，方便分享你的精彩通关时刻。 | **Movie Recording**: Record full playthroughs, replay them anytime, and export to AVI to share your best runs with the community. |
| **金手指**：支持 Game Genie 与原始金手指代码，轻松修改游戏内容，重温童年「无敌版」的快乐。 | **Cheats**: Game Genie and raw cheat code support to tweak gameplay, unlock hidden content, or just have fun bending the rules. |
| **多语言界面**：支持 **12 种语言** —— 简体中文、繁体中文、英文、日语、韩语、西班牙语、法语、德语、越南语、泰语、印地语（beta）、阿拉伯语（beta）；首启自动按系统区域设置匹配语言（`zh-CN` / `ja-JP` / `ar-SA` 等），切换语言后菜单、对话框即时全部重译；阿拉伯语自动启用从右到左布局。 | **Multi-language UI**: Supports **12 languages** — Simplified Chinese, Traditional Chinese, English, Japanese, Korean, Spanish, French, German, Vietnamese, Thai, Hindi (beta), and Arabic (beta). On first launch the language is auto-detected from your system locale (`zh-CN` / `ja-JP` / `ar-SA`, etc.), and menus/dialogs are fully retranslated the moment you switch. Arabic automatically enables right-to-left layout. |
| **自定义调色板**：加载外部调色板文件，自由调整画面色彩，还原你记忆中最对的那个画面色调。 | **Custom Palettes**: Load custom palette files to fine-tune color rendering and match the exact look you grew up with on your old CRT TV. |
| **即时存档**：随时随地保存/读取进度，支持自动存档历史记录，再难的关卡也能反复挑战、不留遗憾。 | **Save States**: Save and load anywhere, anytime, with automatic state history so you never lose progress on a tough boss fight again. |

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

只需安装 **Visual Studio 2022 Community**（免费）和 **Rust**（免费），然后运行两条脚本即可。详见 [`docs/BuildGuide.md`](docs/BuildGuide.md)。

**三步概览：**

```powershell
# 1. 安装前置工具（一次性）
#    ① Visual Studio 2022 Community → 勾选 "使用 C++ 的桌面开发"
#    ② Rust → https://rustup.rs/ → 默认安装

# 2. 下载源码
git clone https://github.com/Laffinty/FCEUX11.git
cd FCEUX11

# 3. 一键安装依赖 + 编译
.\scripts\setup_vcpkg.ps1
.\scripts\do_build.ps1 -Config Release
# do_build.ps1 会用 vswhere 自动发现 Visual Studio 自带的 Ninja；
# 裸 PATH 查不到 ninja.exe 不代表未安装，也不需要另装一份。
# → 产物：build\src\fceux11.exe
```

仅需两条命令即可完成编译。首次编译约 30-60 分钟（主要耗时在 Qt6 下载和编译），后续增量编译 1-3 分钟。

> 详细说明（含常见错误修复、高级选项），请查阅 [`docs/BuildGuide.md`](docs/BuildGuide.md)。

---

## 下载与安装 / Download & Install

预编译二进制文件可在 **[GitHub Releases](https://github.com/Laffinty/FCEUX11/releases)** 页面获取。下载后解压至任意目录即可运行，**无需安装**。
Precompiled binaries are available on the **[GitHub Releases](https://github.com/Laffinty/FCEUX11/releases)** page. Simply extract the archive to any directory and run — **no installation required**.

> **注意**：首次运行时，请确保目标目录具有**写入权限**，以便程序保存配置与存档文件。
> Ensure the target directory has **write permissions** on first run so the program can save configuration and save-state files.

---

## 快速开始 / Quick Start

**1.** 启动 `fceux11.exe`。
**2.** 通过 **File → Open ROM** 加载游戏（支持 `.nes` / `.fds` / `.nsf` / `.unf`）。
**3.** 键盘或手柄游戏；输入映射在 **Options → Input Config** 调整。
**4.** **I** 快速存档，**P** 快速读档。

Launch `fceux11.exe`, load a game via **File → Open ROM**, play with keyboard or gamepad (remap in **Options → Input Config**). Press **I** to quick-save, **P** to quick-load.

---

## 质量保障 / Quality Assurance — KagamiQA

FCEUX11 内置一套名为 **KagamiQA** 的双 Oracle 自动化质量保障系统，在 CI 上常驻运行：

| 组件 | 说明 |
|------|------|
| **Oracle A（回归测试）** | 34 个 CTest 单元/回归/边界测试（来自 39 条清单条目），每次 push 全量运行 |
| **Oracle B（硬件精度测试）** | 177 个 [blargg](https://github.com/christopherpow/nes-test-roms) $6000 协议 ROM（按 `tests/fixtures/blargg/` 实际落盘数量），覆盖 CPU/PPU/APU/MMC3 全子类，headless 全量批处理 |
| **迁移矩阵** | 每次 CI run 产出 `kagamiqa_migration_matrix.json` 并作为 artifact 上传，追踪 PASS→FAIL 回归与 FAIL→PASS 进展 |
| **基线漂移检测** | PASS→FAIL 自动在 PR 下评论红色警报，防止精度退化 |

> **数字来源 / Where these numbers come from**：`34` CTest 数 + `39` 清单条目 + `177` 落盘 ROM 数均为 **CI 产物**——
> 详见 `docs/tech/KagamiQA.md` §0 「CI 数字回填」与最近一次
> `kagamiqa_migration_matrix.json` 的 `engine.git_rev` 字段。本表数值是
> 本地构建快照（commit `1156ca1` 时点，`engine.git_rev=1156ca1`），**禁止手改；改 README 之前先跑一次
> `kagami-qa-runner --output` 并把数字随 commit 一并刷新**。P1-R4 CI 实跑后已按路径 A 统一刷新（2026-08-01）。

**KagamiQA 的实现细节、原理、独立化运行、跨项目迁移指南，请参阅 [`docs/tech/KagamiQA.md`](docs/tech/KagamiQA.md)。**

FCEUX11 ships **KagamiQA**, a dual-oracle automated quality assurance system that runs continuously in CI:

| Component | Description |
|-----------|-------------|
| **Oracle A (regression)** | 34 CTest registered tests (from 39 manifest entries), full run on every push |
| **Oracle B (hardware accuracy)** | 177 [blargg](https://github.com/christopherpow/nes-test-roms) $6000-protocol ROMs (counted from `tests/fixtures/blargg/` on disk) covering all CPU/PPU/APU/MMC3 sub-categories, headless batch |
| **Migration Matrix** | Every CI run produces `kagamiqa_migration_matrix.json` (uploaded as artifact), tracking PASS→FAIL regressions and FAIL→PASS progress |
| **Baseline Drift Detection** | PASS→FAIL automatically posts a red alert PR comment, preventing accuracy decay |

> **Source of truth / 数字来源**: 34 CTest + 39 manifest entries + 177 on-disk ROMs are **CI-sourced** — see `docs/tech/KagamiQA.md` §0 "CI number back-fill" and the `engine.git_rev` field of the most recent `kagamiqa_migration_matrix.json`. This table is a local-build snapshot at commit `1156ca1` (`engine.git_rev=1156ca1`); **do not hand-edit; re-run `kagami-qa-runner --output` first, then refresh the numbers in the same commit**. Refreshed by Path A after P1-R4 CI run on 2026-08-01.

**For implementation details, principles, standalone operation, and cross-project migration, see [`docs/tech/KagamiQA.md`](docs/tech/KagamiQA.md).**

---

## 版本历史 / Changelog

详见 [CHANGELOG.md](CHANGELOG.md)。当前稳定版为 **v1.16**。

**v1.16 主要亮点**：KagamiQA 双 Oracle 质量防线正式上线 — 177 个 blargg 测试 ROM 覆盖 CPU/PPU/APU 全子类，每次 push 自动运行 CI 产出迁移矩阵；vbl_01_basics 修复（VBL 周期对齐）；Lua 断言级测试判定；in-process runner 框架就绪（C ABI 直驱 core，target `kagami_qa_direct_runner`）。

See [CHANGELOG.md](CHANGELOG.md). Current stable release is **v1.16**.

**v1.16 highlights**: KagamiQA dual-oracle quality defense line deployed — 177 blargg test ROMs covering all CPU/PPU/APU sub-categories, CI auto-runs on every push with migration matrix output; vbl_01_basics fix (VBL cycle alignment); Lua assertion-level test signal; in-process runner pipeline (C ABI direct core drive).

---

## 许可 / License

FCEUX11 基于 FCEUX 开发，采用 **GNU GPLv2** 许可证发布。完整许可证文本见 [COPYING](COPYING)。
FCEUX11 is based on FCEUX and distributed under the **GNU GPLv2** license. The full license text is available in [COPYING](COPYING).
