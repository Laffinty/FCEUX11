<div align="center">

# FCEUX11

[![Version](https://img.shields.io/badge/version-v2.0.4-blue)](https://github.com/Laffinty/FCEUX11/releases)
[![License](https://img.shields.io/badge/license-GPL--v2-green)](COPYING)
[![Platform](https://img.shields.io/badge/platform-Windows%2011-0078D4?logo=windows)](https://www.microsoft.com/windows/windows-11)
[![Qt](https://img.shields.io/badge/Qt-6.8%20LTS-41CD52?logo=qt)](https://www.qt.io)
[![CMake](https://img.shields.io/badge/CMake-4.0%2B-064F8C?logo=cmake)](https://cmake.org)

[下载 Releases](https://github.com/Laffinty/FCEUX11/releases) · [提交 Issues](https://github.com/Laffinty/FCEUX11/issues) · [查看更新日志](CHANGELOG.md)

</div>

---

## 简介 / Introduction

**FCEUX11** 是基于 [FCEUX](https://fceux.com) 的 NES/Famicom 模拟器衍生项目，针对 **Windows** 平台深度优化。在继承 FCEUX 卓越模拟精度的同时，采用 Qt6 图形界面重塑了现代 Windows 原生体验，并提供 12 种语言的多语言支持，以及面向开发者和速通玩家的调试与 TAS 工具集。项目持续推进内部核心重构与性能优化工作，致力于让模拟器在高负载场景下依旧保持高效、流畅与稳定。新增实验性 Game Boy Advance（GBA） 模拟支持，复用现有 UI、存档系统、热键与调试基础设施；GBA 功能处于 Beta 阶段，自带画面水印，部分硬件特性尚未完整实现。

**FCEUX11** is a derivative of the [FCEUX](https://fceux.com) NES/Famicom emulator, optimized for **Windows**. It inherits FCEUX's renowned emulation accuracy while delivering a polished, modern Windows-native experience powered by Qt6, with 12-language localization and a full suite of debugging and TAS tools for developers and speedrunners. Ongoing internal refactoring and performance optimization keeps the emulator efficient, smooth, and stable even under heavy load. Experimental Game Boy Advance (GBA) emulation support has been added, reusing the existing UI, save-state system, hotkeys and debugging infrastructure; GBA features are in Beta, the picture carries a watermark, and some hardware features are not yet fully implemented.

---

## 主要功能 / Features

| 中文 | English |
|------|---------|
| **精确模拟**：完整支持 NES、Famicom 及各类 Mapper 扩展芯片，画面与音效高度还原。 | **Accurate Emulation**: Full NES / Famicom / mapper support with faithful graphics and audio. |
| **稳定性与内存安全**：修复 mapper 218 与 CHR-RAM < 8K 卡带的越界读写（可致崩溃或内存损坏）、畸形 `.fm2` 的 GUID 字段导致进程中止、VRC7 跨进程读档崩溃，以及 AVI 录制与手柄状态的跨线程数据竞争。 | **Stability & Memory Safety**: Fixed out-of-bounds access on mapper 218 and sub-8K CHR-RAM cartridges (crashes / memory corruption), a malformed `.fm2` GUID aborting the process, a VRC7 cross-process save-state crash, and cross-thread races in AVI recording and controller state. |
| **实验性 GBA 模拟 (BETA)**：支持 ROM 加载、BIOS SWI 系统调用、音频输出、电池存档（`.srm`）、即时存档、手柄输入、整数倍画面缩放；内置诊断探针用于硬件行为定位，Beta 水印不可关闭。 | **Experimental GBA Emulation (BETA)**: ROM loading, BIOS SWI system calls, audio output, battery saves (`.srm`), save states, controller input, and integer-scaled display; a built-in diagnostic probe helps pinpoint hardware behaviour, and the Beta watermark cannot be disabled. |
| **调试工具**：内置 CPU/PPU 调试器、十六进制编辑器、内存搜索与监视、代码/数据日志。 | **Debugging Tools**: CPU/PPU debugger, hex editor, RAM search/watch, code/data logger. |
| **Function Profiler 报告**（仅 profiler 构建）：`Advanced → Misc Tools → Profiler Report` 弹出只读统计对话框（sum / %total / avg / min / max / calls / func），此前只能在退出时从 stdout 查看。 | **Function Profiler Report** (profiler builds only): a read-only report dialog under `Advanced → Misc Tools → Profiler Report` (sum / %total / avg / min / max / calls / func); previously the report was only visible on stdout at exit. |
| **TAS 编辑器**：逐帧录制并精确编辑按键输入，轻松制作工具辅助速通（TAS）录像。 | **TAS Editor**: Frame-by-frame recording and precise input editing for Tool-Assisted Speedruns. |
| **Famicom 麦克风**：`$4016` bit2 改为**电平直读**（按住时每次读均为 1），替换了 1996 年遗留的逐读翻转实现（按住时读数 1,0,1,0 交替）。 | **Famicom Microphone**: `$4016` bit2 is now read as a plain level (always 1 while held), replacing the 1996-era alternating-read hack (1,0,1,0 while held). |
| **Lua 脚本**：通过 Lua 接口编写脚本，实现自定义屏幕叠加显示、自动化操作、内存数据读取等高级玩法。 | **Lua Scripting**: Custom on-screen displays, automation, and memory access via Lua. |
| **Lua 接口修正**：`input.get()` 的 96 个键名与实际按键对齐（此前整体错位，如 `capslock` 读到 Q）、三个 toggle 键不再恒为 false；鼠标 `click` 改为位掩码并补上中键，`input.get().click` 恒为数值字段。 | **Lua API Fixes**: all 96 `input.get()` key names now match the physical keys (they were globally misaligned, e.g. `capslock` reported Q), the three toggle keys are no longer stuck false, the mouse `click` value is now a bit mask with middle-button support, and `input.get().click` is always present as a numeric field. |
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

需要 **Visual Studio 2022 Community**（勾选「使用 C++ 的桌面开发」）与 **Rust**（[rustup](https://rustup.rs) 默认安装）。CI 锁定 **Rust 1.100.0**，本地建议同版本，以免「本机绿、CI 红」。详见 [`docs/BuildGuide.md`](docs/BuildGuide.md)。

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
2. 通过 **File → Open ROM** 加载游戏（支持 `.nes` / `.fds` / `.nsf` / `.unf`）；实验性 GBA（Beta）可直接打开 `.gba` 卡带，画面带 `BETA` 水印，个别游戏存在已知问题。
3. 键盘或手柄游戏；输入映射在 **Options → Input Config** 调整。
4. **I** 快速存档，**P** 快速读档。

Launch `fceux11.exe`, load a game via **File → Open ROM** (NES: `.nes` / `.fds` / `.nsf` / `.unf`; experimental GBA (Beta): `.gba` cartridges open from the same dialog, frames carry a `BETA` watermark, and a few games have known issues), play with keyboard or gamepad (remap in **Options → Input Config**). Press **I** to quick-save, **P** to quick-load.

---

## 版本历史 / Changelog

详见 [CHANGELOG.md](CHANGELOG.md)。当前主线为 **v2.0.4**（残留精度与调试待办收口，**20/22 项落地**；上一稳定发布 **v2.0.3**）。
See [CHANGELOG.md](CHANGELOG.md). Mainline is **v2.0.4** (accuracy & debugging backlog closed out, **20 of 22 items landed**; previous stable **v2.0.3**).

### v2.0.4 行为变更 / Behaviour changes in v2.0.4

> 旧存档与录像仍可正常读取，但下列读数会变：
> - 非 FDS 卡带的 `.cdl` 输出改变（与上游 FCEUX 2.7.0 一致化）；
> - 按住 Famicom 麦克风时 `$4016` bit2 恒为 1（旧为 1,0,1,0 交替）；
> - CHR-RAM < 8K 的卡带画面会变、其既有 movie 会漂移（**≥8K 卡逐字节不变**）；
> - Lua `input.get()` 恒含数值 `click` 字段（遍历键的脚本会多见到一个 `click=0`）。
>
> Existing save states and movies still load, but four observable readouts change: the `.cdl` output for non-FDS cartridges, the Famicom microphone level on `$4016` bit2, the picture (and movie replay) of sub-8K CHR-RAM cartridges, and the always-present numeric `click` field in `input.get()`.

### 已知限制 / Known limitations

> - **GUI 下运行会 yield 的 Lua 脚本可能崩溃**（无头 runner 不受影响）—— v2.0.4 未修，如实登记；
> - **GBA 仍是 Alpha/Beta**：BIOS SWI 声音家族 12 号与 `0x2A` 走兜底实现，GPIO/RTC 未做；沿用 v2.0.3 的已知限制；
> - **F11QA 矩阵以 CI 复核为准**（`pass_to_fail = 0`、grade ≥ B）。
>
> Known limitations: yielding Lua scripts may crash under the GUI (headless runners are unaffected); GBA remains Alpha/Beta with the v2.0.3 limitations still in force; the F11QA matrix is judged by CI (`pass_to_fail = 0`, grade ≥ B).

---

## 许可 / License

FCEUX11 基于 FCEUX 开发，采用 **GNU GPLv2** 许可证发布。完整许可证文本见 [COPYING](COPYING)。
FCEUX11 is based on FCEUX and distributed under the **GNU GPLv2** license. The full license text is available in [COPYING](COPYING).
