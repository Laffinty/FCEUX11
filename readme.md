<div align="center">

# FCEUX11

[![Version](https://img.shields.io/badge/version-v2.0-blue)](https://github.com/Laffinty/FCEUX11/releases)
[![License](https://img.shields.io/badge/license-GPL--v2-green)](COPYING)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![Qt](https://img.shields.io/badge/Qt-6.8%20LTS-41CD52?logo=qt)](https://www.qt.io)
[![CMake](https://img.shields.io/badge/CMake-4.0%2B-064F8C?logo=cmake)](https://cmake.org)

[下载 Releases](https://github.com/Laffinty/FCEUX11/releases) · [提交 Issues](https://github.com/Laffinty/FCEUX11/issues) · [查看更新日志](docs/ChangeLog.md)

</div>

---

## 简介 / Introduction

**FCEUX11** 是基于 [FCEUX](https://fceux.com) 的 NES/Famicom 模拟器衍生项目，针对 **Windows** 平台深度优化。在继承 FCEUX 卓越模拟精度的同时，采用 Qt6 图形界面重塑了现代 Windows 原生体验，并提供 12 种语言的多语言支持，以及面向开发者和速通玩家的调试与 TAS 工具集。项目持续推进内部核心重构与性能优化工作，致力于让模拟器在高负载场景下依旧保持高效、流畅与稳定。v2.0 已完成核心 CPU 的 Rust 化重构（Phase 1-7）：C++ X6502 已被删除，`fceux11-core` 的 Rust 6502 成为唯一实现（`FCEUX11_RUST_CPU` 默认开启）。

**FCEUX11** is a derivative of the [FCEUX](https://fceux.com) NES/Famicom emulator, optimized for **Windows**. It inherits FCEUX's renowned emulation accuracy while delivering a polished, modern Windows-native experience powered by Qt6, with 12-language localization and a full suite of debugging and TAS tools for developers and speedrunners. Ongoing internal refactoring and performance optimization keeps the emulator efficient, smooth, and stable even under heavy load. v2.0 completes the Rust-first CPU migration (Phase 1-7): the C++ X6502 has been deleted and the `fceux11-core` Rust 6502 is now the only CPU implementation (`FCEUX11_RUST_CPU` is ON by default).

---

## 主要功能 / Features

| 中文 | English |
|------|---------|
| **精确模拟**：完整支持 NES、Famicom 及各类 Mapper 扩展芯片，画面与音效高度还原。 | **Accurate Emulation**: Full NES / Famicom / mapper support with faithful graphics and audio. |
| **Rust 6502 核心**：v2.0 起由 `fceux11-core` 的 Rust 6502 驱动 CPU（Phase 1-7 迁移完成，C++ X6502 已删除）。| **Rust 6502 Core**: since v2.0 the CPU is driven by the `fceux11-core` Rust 6502 (Phase 1-7 migration complete; the C++ X6502 has been deleted). |
| **调试工具**：内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码/数据日志。 | **Debugging Tools**: CPU/PPU debugger, hex editor, RAM search/watch, code/data logger. |
| **TAS 编辑器**：逐帧录制并精确编辑按键输入，轻松制作工具辅助速通（TAS）录像。 | **TAS Editor**: Frame-by-frame recording and precise input editing for Tool-Assisted Speedruns. |
| **Lua 脚本**：通过 Lua 接口编写脚本，实现自定义屏幕叠加显示、自动化操作、内存数据读取等高级玩法。 | **Lua Scripting**: Custom on-screen displays, automation, and memory access via Lua. |
| **录像回放**：录制完整游戏过程并支持回放，可导出为 AVI 视频。 | **Movie Recording**: Record and replay playthroughs, export to AVI. |
| **金手指**：支持 Game Genie 与原始金手指代码，轻松修改游戏内容。 | **Cheats**: Game Genie and raw cheat code support. |
| **多语言界面**：支持 **12 种语言** —— 简体中文、繁体中文、英文、日语、韩语、西班牙语、法语、德语、越南语、泰语、印地语（beta）、阿拉伯语（beta）；首启自动按系统区域设置匹配语言，切换语言后菜单、对话框即时全部重译；阿拉伯语自动启用从右到左布局。 | **Multi-language UI**: **12 languages** — Simplified Chinese, Traditional Chinese, English, Japanese, Korean, Spanish, French, German, Vietnamese, Thai, Hindi (beta), and Arabic (beta). Auto-detected from system locale on first launch, instant retranslate on switch, and automatic right-to-left layout for Arabic. |
| **自定义调色板**：加载外部调色板文件，自由调整画面色彩。 | **Custom Palettes**: Load custom palette files to fine-tune color rendering. |
| **即时存档**：随时随地保存 / 读取进度，支持自动存档历史记录。 | **Save States**: Save / load anywhere with automatic state history. |

---

> **已知移除 / Known Removal**：自 v1.15 (hotfix4) 起，**NetPlay（联机对战）** 正式移除——上游 FCEUX 的该功能本已不可用（`config.cpp` 原注 "netplay is broken"），本版本清理了其 CLI 选项与不可达代码，核心 `netplay.cpp` 保留以维持存档兼容。详见 [ChangeLog.md](docs/ChangeLog.md)。
> Since v1.15 (hotfix4), **NetPlay** has been formally removed — it was already broken upstream. Related CLI options and unreachable code were cleaned up; core `netplay.cpp` is kept for savestate compatibility. See [ChangeLog.md](docs/ChangeLog.md).

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

> **v2.0 说明**：Phase 7 起 Rust 6502 CPU 为唯一实现，`FCEUX11_RUST_CPU` 默认 ON（OFF 为配置期错误）；需要 Rust toolchain（rustup 默认安装即可）。
> **v2.0 note**: Since Phase 7 the Rust 6502 CPU is the only implementation; `FCEUX11_RUST_CPU` defaults to ON (OFF is a configure-time error); a Rust toolchain is required (default `rustup` install).

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

## 质量保障 / Quality Assurance — KagamiQA

FCEUX11 内置一套名为 **KagamiQA** 的双 Oracle 自动化质量保障系统，在 CI 上常驻运行：

| 组件 | 说明 |
|------|------|
| **Oracle A（回归测试）** | 27 个 Oracle A 清单条目（外加 6 个 CTest-only 基础设施测试），每次 push 全量运行 |
| **Oracle B（硬件精度测试）** | 20 个 Oracle B 清单条目代表桶 + 177 个 [blargg](https://github.com/christopherpow/nes-test-roms) `$6000` 协议 ROM 全量批处理，覆盖 CPU/PPU/APU/MMC3 全子类 |
| **迁移矩阵** | 每次 CI run 产出 `kagamiqa_migration_matrix.json` 并作为 artifact 上传，追踪 PASS→FAIL 回归与 FAIL→PASS 进展 |
| **基线漂移检测** | PASS→FAIL 自动在 PR 下评论红色警报，防止精度退化 |

> **v2.0 发布评审矩阵**（2026-08-22，commit `3511aa0` 的 kagami-qa / build-windows 两次 CI 审计）：KagamiQA **Grade D（阻断）**——47 项 37P/10F，2 条 PASS→FAIL 回归（`blargg_cpu_instrs`、`rom_regression_test`），R4 门禁失败。根因与处置见 [`docs/plans/v2.0-release-optimization.md`](docs/plans/v2.0-release-optimization.md)：`blargg_cpu_instrs`（0xAB LAX #imm 立即数误读）已修复转 PASS；`rom_regression_test`（nestest 第 4 帧）为已记录的 PPU 渲染时序残差，按方案修复或治理冻结。修复后本地矩阵 38P/9F，唯一回归即 `rom_regression_test`；目标评级 **B（发布标准）**，A 级需继续消解 7 个 blargg known-limit 与全量 blargg 套件。

**实现细节、原理、独立化运行、跨项目迁移**请参阅 [`docs/tech/KagamiQA.md`](docs/tech/KagamiQA.md)。

FCEUX11 ships **KagamiQA**, a dual-oracle automated quality assurance system that runs continuously in CI:

| Component | Description |
|-----------|-------------|
| **Oracle A (regression)** | 27 Oracle A manifest entries (plus 6 CTest-only infrastructure tests), full run on every push |
| **Oracle B (hardware accuracy)** | 20 Oracle B manifest entries as bucket representatives + 177 [blargg](https://github.com/christopherpow/nes-test-roms) `$6000`-protocol ROMs (full batch) covering all CPU/PPU/APU/MMC3 sub-categories |
| **Migration Matrix** | Every CI run produces `kagamiqa_migration_matrix.json` (uploaded as artifact), tracking PASS→FAIL regressions and FAIL→PASS progress |
| **Baseline Drift Detection** | PASS→FAIL automatically posts a red alert PR comment, preventing accuracy decay |

> **v2.0 release-audit matrix** (2026-08-22, kagami-qa + build-windows CI runs on commit `3511aa0`): KagamiQA **Grade D (blocked)** — 47 entries 37P/10F, 2 PASS→FAIL regressions (`blargg_cpu_instrs`, `rom_regression_test`), R4 gate failed. Root causes and disposition in [`docs/plans/v2.0-release-optimization.md`](docs/plans/v2.0-release-optimization.md): `blargg_cpu_instrs` (0xAB LAX #imm immediate-operand bug) fixed and back to PASS; `rom_regression_test` (nestest frame 4) is the documented PPU render-timing residual — to be fixed or governed-baseline-frozen per the plan. Post-fix local matrix: 38P/9F with the only remaining regression being `rom_regression_test`; target grade **B (release standard)**; A requires clearing the 7 blargg known-limits plus the full blargg suite.

**For implementation details, principles, standalone operation, and cross-project migration**, see [`docs/tech/KagamiQA.md`](docs/tech/KagamiQA.md).

---

## 版本历史 / Changelog

详见 [ChangeLog.md](docs/ChangeLog.md)。当前稳定版为 **v2.0**。
See [ChangeLog.md](docs/ChangeLog.md). Current stable release is **v2.0**.

---

## 许可 / License

FCEUX11 基于 FCEUX 开发，采用 **GNU GPLv2** 许可证发布。完整许可证文本见 [COPYING](COPYING)。
FCEUX11 is based on FCEUX and distributed under the **GNU GPLv2** license. The full license text is available in [COPYING](COPYING).
