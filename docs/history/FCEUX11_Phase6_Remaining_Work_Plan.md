# FCEUX11 Phase 6 剩余工作执行计划

> **文档版本**: 1.0
> **创建日期**: 2026-05-18
> **适用范围**: Phase 6（遗留代码清理模块）未完成项
> **基线 Commit**: `ba60d30` (Phase 6 part 2)

---

## 1. 前言

Phase 6 的前期跨平台宏清理工作量被显著低估。经过 `part 1` 和 `part 2` 两批次可控推进，**当前代码库已恢复稳定构建**（DEBUG & Release 均通过 MinGW-w64 GCC 16.1.0 + Qt6 验证），但仍有大量收尾工作尚未完成。

本文档将 Phase 6 的全部剩余工作**清单化、优先级化、可验证化**，供后续会话按图索骥、逐条执行。每条任务均标注：
- **风险等级**（低 / 中 / 高）
- **影响范围**（文件、符号、功能）
- **验证方法**（构建验证 / 功能测试 / 代码审查）
- **前置依赖**

---

## 2. 已完成工作摘要（基线状态）

| 批次 | Commit | 内容 | 文件数 |
|------|--------|------|--------|
| Part 1 | `66f78c6` | 废弃 Win32/SDL 驱动、清理 CMake 债务、大规模跨平台宏清理、构建断裂修复 | 21 |
| Part 2 | `ba60d30` | 低风险死代码删除（video.cpp, file.cpp, fceuWrapper.cpp, HelpPages.cpp） | 4 |

**构建状态**: DEBUG 通过（102.7 MB，含完整符号）| Release 通过（6.6 MB）

---

## 3. 剩余工作总览

```
优先级 P0 -- sdl-throttle.cpp 定时精度恢复（已发生退化）
优先级 P1 -- unix-netplay.cpp 重命名（文档/命名不一致）
优先级 P2 -- 中风险跨平台宏清理（5 个文件）
优先级 P3 -- CMake 变量重命名与纯化（无功能影响）
优先级 P4 -- 核心代码 #ifdef WIN32 块评估（需判定去留）
优先级 P5 -- POSIX 到 MSVC 迁移预判清单（Phase 2 远期）
```

---

## 4. 详细任务清单

---

### 4.1 优先级 P0: sdl-throttle.cpp 定时精度恢复

**风险等级**: 高（已造成功能退化）
**任务类型**: 代码恢复 / 重构
**前置依赖**: 无
**预计工时**: 1~2 小时

#### 4.1.1 问题描述

在 Phase 6 part 1 中，`src/drivers/Qt/sdl-throttle.cpp` 的 Linux/Unix 代码分支被整体删除，包括：
- `timerfd_create` / `timerfd_settime` 高精度定时器子系统
- `nanosleep` 高精度睡眠实现
- `setTimingMode()` / `getTimingMode()` 的逻辑被简化

**当前状态**: `highPrecSleep()` 仅剩 `SDL_Delay(ts.toMilliSeconds())`。
**退化影响**: `SDL_Delay` 在 Windows 下的精度约为 1~10 ms。对 60 FPS 模拟器（16.67 ms/帧）通常足够，但在以下场景会出现可见的帧抖动：
- TAS（Tool-Assisted Speedrun）精确帧控
- 高刷新率显示器（120 Hz / 144 Hz / 240 Hz）
- 音频同步要求严格的场景

#### 4.1.2 恢复策略

**方案 A（推荐）: 引入 Windows 高精度睡眠**

在 `highPrecSleep()` 中，为 Windows 平台增加高精度睡眠路径。评估以下实现：

- **方案 1 (`timeBeginPeriod`)**: 实现简单，Win7~Win11 均支持，但会全局提升系统定时器精度，增加功耗。
- **方案 2 (`QWaitCondition` + `QElapsedTimer`)**: 纯 Qt6，无需 Win32 API，精度约 1 ms，实现最干净。
- **最佳实践**: 先用 `QElapsedTimer` 测量已过时间，若剩余时间 > 1 ms 则用 `Sleep(1)` 或 `QThread::msleep()`，剩余 < 1 ms 时用自旋等待（busy-wait）或 `yield`。

**方案 B（保守）: 恢复 nanosleep 为条件编译路径**

不推荐理由：FCEUX11 已声明 Windows 11 独占，`nanosleep` 在 Windows 下需要 MinGW POSIX layer，非原生方案。

#### 4.1.3 验证方法

1. **构建验证**: DEBUG + Release 均 0 错误
2. **精度测试**: 在 `TimingConf.cpp` 或日志中输出每帧实际睡眠时间与目标睡眠时间的偏差（单位: us）
3. **功能测试**:
   - 启动模拟器，不加载 ROM，观察空闲循环 CPU 占用是否正常下降
   - 加载 ROM，运行 60 FPS 游戏，肉眼观察是否有规律性微卡顿
   - 若条件允许，用 FCEUX11 的 TAS 编辑器录制 10 秒输入，回放时对比帧号是否漂移

#### 4.1.4 回滚策略

若引入的高精度睡眠导致 CPU 占用飙升或兼容性问题，立即回滚到 `SDL_Delay` 路径，并增加编译选项控制。

---

### 4.2 优先级 P1: unix-netplay.cpp 重命名

**风险等级**: 中（涉及构建系统）
**任务类型**: 文件重命名 + CMake 更新 + 头文件引用更新
**前置依赖**: 无
**预计工时**: 30 分钟

#### 4.2.1 问题描述

`src/drivers/Qt/unix-netplay.cpp` 文件名带有 "unix" 前缀，与 Phase 6 "Windows 11 独占"定位冲突。Construction Plan P6-3 明确指出该文件名暴露 Unix 起源，应在清理范围内。

**当前状态**: 该文件仍在构建中（`src/CMakeLists.txt` 线 442），且是 Qt 驱动的**唯一 netplay 实现**（内部含 `#ifdef WIN32` Winsock 分支）。`src/drivers/sdl/README.DEPRECATED` 已修正为说明该文件仍在 Qt 驱动中使用。

#### 4.2.2 执行步骤

1. **重命名文件**
   - `src/drivers/Qt/unix-netplay.cpp` -> `src/drivers/Qt/QtNetplay.cpp`
   - `src/drivers/Qt/unix-netplay.h` -> `src/drivers/Qt/QtNetplay.h`
2. **更新 CMakeLists.txt**: `src/CMakeLists.txt` 线 442: `unix-netplay.cpp` -> `QtNetplay.cpp`
3. **更新头文件引用**: `unix-netplay.cpp` 自身包含 `#include "Qt/unix-netplay.h"`，改为 `#include "Qt/QtNetplay.h"`
4. **更新 Construction Plan**: 将 `unix-netplay.cpp` 的清理项标记为"已重命名"

#### 4.2.3 验证方法

1. DEBUG + Release 均 0 错误
2. netplay 功能快速验证（可选）: 启动模拟器 -> Network -> 检查菜单项是否正常出现

---

### 4.3 优先级 P2: 中风险跨平台宏清理（5 个文件）

**风险等级**: 中（可能造成功能缺失或构建断裂）
**任务类型**: 条件编译分支清理
**前置依赖**: P0 完成后（避免同时修改多个模块导致难以定位的断裂）
**预计工时**: 3~4 小时（含双重构建验证）

#### 4.3.1 文件清单与清理策略

| # | 文件 | 当前宏状态 | 清理策略 | 风险点 |
|---|------|-----------|---------|--------|
| 1 | `src/drivers/Qt/ConsoleWindow.cpp` | 线 172: `#ifndef WIN32` 块内调用已删除的 `setNicePriority` / `setSchedParam` | **直接删除**该 `#ifndef WIN32` 块（共约 10 行）。这些代码在 Windows 下本就跳过，且调用的方法已在头文件中移除。 | 低 |
| 2 | `src/drivers/Qt/LuaControl.cpp` | 线 307: `#ifndef WIN32` 设置 `/usr/share/fceux/luaScripts` 路径 | **替换为 `#error`** 或删除整个 `#else` 分支，保留 Windows 路径。 | 低 |
| 3 | `src/drivers/Qt/config.cpp` | 线 413: `#if defined(WIN32) || defined(NEED_MINGW_HACKS)` 目录创建逻辑；线 635: `#if defined(WIN32)` AVI 视频格式默认值 | **简化条件**: 移除 `NEED_MINGW_HACKS`（MinGW 即 Windows），保留 Windows 路径为无条件路径。 | 中 |
| 4 | `src/drivers/common/configSys.cpp` | 线 7, 582: `#ifndef WIN32` 包含 `<dirent.h>` 和 `cfg.d/` 目录读取 | **删除非 Windows 分支**。`cfg.d/` 是 Linux 的配置目录机制，Windows 下不使用。 | 中 |
| 5 | `src/drivers/common/os_utils.cpp` | 线 5, 20, 71, 94: `#if defined(WIN32)` 平台变体（`mkdir`, `Sleep`, `fopen`） | **保留 `#ifdef WIN32` 块**但删除 `#else` 分支。该文件是平台抽象层，Windows 路径已完整。 | 中 |

#### 4.3.2 逐文件详细说明

**4.3.2.1 `src/drivers/Qt/ConsoleWindow.cpp`（线 172~182）**

当前代码（Windows 下被跳过）:
```cpp
#ifndef WIN32
    int policy, prio, nice;
    g_config->getOption("SDL.GuiSchedPolicy", &policy);
    g_config->getOption("SDL.GuiSchedPrioRt", &prio);
    g_config->getOption("SDL.GuiSchedNice", &nice);
    setNicePriority(nice);
    setSchedParam(policy, prio);
#endif
```

- `setNicePriority` 和 `setSchedParam` 方法已在 `ConsoleWindow.h` 中删除。
- 该代码块在 Windows 下被 `#ifndef WIN32` 跳过，属于**死代码**。
- **操作**: 整段删除。

**4.3.2.2 `src/drivers/Qt/LuaControl.cpp`（线 307 附近）**

```cpp
#ifdef WIN32
    luaScriptPath = "luaScripts";
#else
    luaScriptPath = "/usr/share/fceux/luaScripts";
#endif
```

- **操作**: 删除 `#else` 分支，保留 Windows 路径。或改为 `#error "Platform not supported"`。

**4.3.2.3 `src/drivers/Qt/config.cpp`（线 413, 635）**

- 线 413: `#if defined(WIN32) || defined(NEED_MINGW_HACKS)` -> 由于 FCEUX11 仅在 Windows 运行，可简化为无条件编译。
- 线 635: `#if defined(WIN32)` -> 保留，但删除 `#else` 分支（Linux 视频格式默认值）。

**4.3.2.4 `src/drivers/common/configSys.cpp`（线 7, 582）**

- 线 7: `#ifndef WIN32 #include <dirent.h>` -> 删除。Windows 下不需要 dirent.h。
- 线 582: `#ifndef WIN32` 的 `cfg.d/` 扫描循环 -> 删除。`cfg.d/` 是 Linux 配置片段目录机制。

**4.3.2.5 `src/drivers/common/os_utils.cpp`（线 5, 20, 71, 94）**

- 该文件是平台抽象层，当前结构为 `#if defined(WIN32)` ... `#else` ... `#endif`。
- **操作**: 删除所有 `#else` ... `#endif` 块，保留 Windows 实现。在文件顶部加 `#ifndef WIN32 #error "Platform not supported" #endif` 作为编译期防护。

#### 4.3.3 验证方法

每修改一个文件后：
1. `cmake --build build --config Debug -j4`（0 错误）
2. `cmake --build build --config Release -j4`（0 错误）
3. 启动可执行文件，验证主窗口正常
4. 加载 ROM，验证基本运行 10 秒无异常

**建议分 2~3 个 commit 提交**，不要一次性全部修改：
- Commit 1: ConsoleWindow.cpp + LuaControl.cpp（Qt 驱动层）
- Commit 2: config.cpp（Qt 配置层）
- Commit 3: configSys.cpp + os_utils.cpp（通用驱动层）

---

### 4.4 优先级 P3: CMake 变量重命名与纯化

**风险等级**: 低（无功能影响）
**任务类型**: CMake 重构
**前置依赖**: 无
**预计工时**: 30 分钟

#### 4.4.1 任务清单

| # | 任务 | 文件/位置 | 说明 |
|---|------|----------|------|
| 1 | 重命名 `SRC_DRIVERS_SDL` | `src/CMakeLists.txt` 线 392 | 当前变量包含的全是 `drivers/Qt/*.cpp`，无任何 SDL 代码。应重命名为 `SRC_DRIVERS_QT`。 |
| 2 | 移除 `include(GNUInstallDirs)` | 根 `CMakeLists.txt` 线 1 | Windows 专用项目不需要 GNU 安装目录模块。 |
| 3 | 评估 `MINIZIP_LDFLAGS` | `src/CMakeLists.txt` 线 496 | 当前为空字符串。若 `utils/ioapi.cpp` + `utils/unzip.cpp` 始终提供符号，可删除该变量引用。 |
| 4 | 评估 `OPENGL_LDFLAGS` | `src/CMakeLists.txt` 线 494 | 仅在 `MINGW` 分支定义。MSVC 下为空，但 Qt6::OpenGLWidgets 会自动链接 OpenGL。 |

#### 4.4.2 验证方法

修改后重新执行 `cmake -S . -B build`，DEBUG + Release 构建均通过。

---

### 4.5 优先级 P4: 核心代码 `#ifdef WIN32` 块评估（判定去留）

**风险等级**: 中（误判会导致功能删除）
**任务类型**: 代码审查 / 判定
**前置依赖**: P2 完成后
**预计工时**: 2~3 小时（纯审查，不一定修改）

#### 4.5.1 评估原则

Construction Plan P6-3 的策略：
> "低风险分支：直接删除非 Windows 代码路径。复杂分支：以 `#ifdef _WIN32` 替代通用宏；无法直接移除的非 Windows 分支改为 `#error "Platform not supported"` 编译期防护。"

**关键区分**:
- **非 Windows 兼容代码**（目标：删除或改为 `#error`）
- **Windows 专有功能代码**（目标：保留，这是 Windows 功能增强而非废弃兼容层）

#### 4.5.2 待评估文件清单

| 文件 | 当前宏 | 初步判定 | 建议 |
|------|--------|---------|------|
| `src/sound.cpp` | `#ifdef WIN32` (线 151, 170, 1172) | **Windows 功能增强** | 保留。CD logger / DPCM debug 是 Windows Qt 驱动提供的调试功能，非兼容层。 |
| `src/sound.h` | `#ifdef WIN32` (线 54) | **Windows 功能增强** | 保留。 |
| `src/movie.cpp` | `#ifdef WIN32` (线 30) | **Windows 功能增强** | 保留。包含 `windows.h` 和 `__WIN_DRIVER__` 相关代码，是 Windows 视频录制功能。 |
| `src/input.cpp` | `#if defined(WIN32) && !defined(__QT_DRIVER__)` (线 39) | **历史耦合** | 需审查。`__WIN_DRIVER__` 是 legacy Win32 驱动标记，若 Qt 驱动为唯一驱动，可简化条件。 |
| `src/fceu.h` | `#ifdef WIN32` (线 155) | **Windows 功能增强** | 保留。`UpdateCheckedMenuItems()` 是 Windows UI 功能。 |
| `src/types.h` | `#if defined(WIN32) && !defined(__QT_DRIVER__) && !defined(__WIN_DRIVER__)` (线 141) | **历史耦合** | 需审查。`__WIN_DRIVER__` 已废弃，可简化或移除该守卫。 |
| `src/file.cpp` | 已清理 `#ifndef WIN32` | -- | Part 2 已完成。 |
| `src/video.cpp` | 已清理 `#ifndef WIN32` | -- | Part 2 已完成。 |

#### 4.5.3 详细审查要点

**`src/input.cpp` 线 39**
```cpp
#if defined(WIN32) && !defined(__QT_DRIVER__)
#include "drivers/win/directinput.h"
#endif
```
- `__QT_DRIVER__` 在当前构建中始终定义。
- 该 `#include` 在 Qt 驱动下被跳过。
- **建议**: 由于 `drivers/win/` 已废弃，该 `#include` 永不被触发。可删除整个条件块，或保留但改为 `#error` 防护。

**`src/types.h` 线 141**
```cpp
#if defined(WIN32) && !defined(__QT_DRIVER__) && !defined(__WIN_DRIVER__)
#define __WIN_DRIVER__
#endif
```
- `__WIN_DRIVER__` 是 legacy Win32 GUI 驱动的标记。
- 当前构建使用 Qt6 驱动（`__QT_DRIVER__` 已定义），所以该块被跳过。
- **建议**: 删除。`__WIN_DRIVER__` 不再被任何活跃代码使用。

#### 4.5.4 验证方法

- 纯代码审查，无需额外构建验证（若不修改）
- 若决定修改，按 P2 的验证流程执行

---

### 4.6 优先级 P5: POSIX 到 MSVC 迁移预判清单（Phase 2 远期）

**风险等级**: 信息（不立即执行）
**任务类型**: 文档 / 预判
**前置依赖**: Phase 6 全部完成后
**预计工时**: 1 小时（纯文档工作）

#### 4.6.1 背景

Construction Plan Phase 2 提到远期向 MSVC 2022+ 迁移。Phase 6 清理跨平台代码时，应同步记录以下 POSIX 依赖，为后续迁移提供精确清单。

#### 4.6.2 已识别的 POSIX 依赖（当前代码库中）

| 符号/函数 | 当前使用位置 | MSVC 替代方案 | 优先级 |
|-----------|-------------|--------------|--------|
| `ssize_t` | 待排查（可能在 `unix-netplay.cpp` 或核心 I/O 中） | `SSIZE_T` (Windows.h) 或 `ptrdiff_t` | 中 |
| `strcasestr` | 待排查 | `StrStrIA` (Shlwapi.h) 或自实现 | 低 |
| `strtok_r` | 待排查 | `strtok_s` (MSVC) | 低 |
| `alloca` (`__builtin_alloca`) | `src/types.h` 线 66 | `<malloc.h>` 中的 `alloca` | 中 |
| `__attribute__((always_inline))` | `src/lua-engine.cpp` 线 140 | `__forceinline` 关键字 | 低 |
| AT&T 内联汇编 | 待排查 | Compiler intrinsics | 高（若存在） |
| `nanosleep` | `sdl-throttle.cpp`（已删除） | 见 P0 恢复方案 | 高 |
| `realpath` | `fceuWrapper.cpp`（已删除注释块） | `GetFullPathNameA` / `QFileInfo::canonicalFilePath()` | 低 |
| `fork` / `execl` | `HelpPages.cpp`（已删除注释块） | `CreateProcess` | 低 |
| `dirent.h` / `DIR*` | `configSys.cpp`（待清理） | `FindFirstFile` / `QDir` | 中 |
| `setpriority` / `getpriority` | `ConsoleWindow.cpp`（已删除） | `SetThreadPriority` / `GetThreadPriority` | 低 |
| `sched_get_priority_min/max` | `ConsoleWindow.cpp`（已删除） | `THREAD_PRIORITY_*` 常量 | 低 |
| `pthread_t` | `ConsoleWindow.h`（已删除） | `HANDLE` (Win32 thread) | 低 |
| `timerfd_create` | `sdl-throttle.cpp`（已删除） | `CreateWaitableTimer` | 低 |

#### 4.6.3 建议

在 Phase 6 完全结束后，将上表整理为 `documentation/MSVC_Migration_Guide.md`，作为 Phase 2 远期 MSVC 迁移的输入物。

---

## 5. 建议执行顺序与里程碑

```
Milestone 1: P0 完成 -- sdl-throttle 定时精度恢复
    |
    v
Milestone 2: P1 完成 -- unix-netplay.cpp 重命名
    |
    v
Milestone 3: P2 完成 -- 中风险宏清理（分 2~3 commit）
    |
    v
Milestone 4: P3 完成 -- CMake 纯化
    |
    v
Milestone 5: P4 完成 -- 核心代码 #ifdef WIN32 评估报告
    |
    v
Milestone 6: P5 完成 -- MSVC 迁移预判清单文档
    |
    v
Phase 6 全面结束
```

---

## 6. 回滚与风险控制

| 风险场景 | 应对措施 |
|---------|---------|
| 修改后构建断裂 | 立即 `git revert` 最近一次 commit，回到 `ba60d30` 基线 |
| sdl-throttle 恢复后 CPU 占用飙升 | 在 `highPrecSleep()` 中增加编译开关（`#ifdef USE_HIGH_PREC_SLEEP`），默认关闭 |
| 误删 Windows 功能代码 | 代码审查时严格区分"Windows 功能增强"与"非 Windows 兼容层" |
| unix-netplay 重命名后头文件找不到 | 全局搜索 `#include "*unix-netplay*"`，确保所有引用已更新 |

---

## 7. 附录

### 附录 A: 快速构建验证命令

```powershell
# 配置 DEBUG
$env:PATH = "D:/msys64/mingw64/bin;$env:PATH"; cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug

# 构建 DEBUG
$env:PATH = "D:/msys64/mingw64/bin;$env:PATH"; cmake --build build --config Debug -j4

# 配置 Release
$env:PATH = "D:/msys64/mingw64/bin;$env:PATH"; cmake -S . -B build -DCMAKE_BUILD_TYPE=Release

# 构建 Release
$env:PATH = "D:/msys64/mingw64/bin;$env:PATH"; cmake --build build --config Release -j4
```

### 附录 B: 当前基线 Commit

```
commit ba60d30
Author: FCEUX11 Contributors
Date:   2026-05-18

    Phase 6: Legacy code cleanup -- part 2 (low-risk macro removal)
```

### 附录 C: 参考资料

- `documentation/FCEUX11_Construction_Plan.md` -- 原始 Construction Plan
- `src/drivers/win/README.DEPRECATED` -- Win32 驱动废弃声明
- `src/drivers/sdl/README.DEPRECATED` -- SDL 驱动废弃声明
