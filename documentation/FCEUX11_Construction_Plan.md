# FCEUX11 项目建设方案

> **文档版本**: 1.1  
> **创建日期**: 2026-05-17  
> **适用范围**: FCEUX11 二次开发项目  
> **许可声明**: 本文档基于 GPLv2 许可证框架制定，所有方案实施须严格遵守许可证合规要求。

---

## 1. 项目概述与演进目标

### 1.1 项目定位

FCEUX11 是基于 FCEUX（版本 2.6.6）的 Windows 11 独占型衍生作品。项目核心目标是在完整保留 NES/Famicom 模拟器核心准确性的前提下，逐步推进 Windows 原生技术栈适配。**近期以保持 msys64/MinGW-w64 构建兼容为主**，在此基线上完成标识层更新、Qt6 迁移与代码现代化；向微软系工具链（MSVC）的迁移作为远期计划推进，最终建立面向 Windows 11 及后续 Windows 版本的高性能、高兼容性、高可维护性的 NES 模拟器发行版。

**平台策略声明**：本项目不再视 Linux/macOS 为支持目标，所有跨平台兼容代码将在后续 Phase 中逐步移除或标记为废弃。

### 1.2 核心原则

| 原则 | 说明 |
|------|------|
| **平台聚焦** | Windows 11 为唯一支持平台，彻底移除 Linux/macOS 兼容代码。工具链向 MSVC 迁移为远期计划，近期保持 msys64/MinGW-w64 兼容。 |
| **合规优先** | 所有修改严格遵循 GPLv2 第 2 条（衍生作品条款），原作者版权声明以最小必要限度保留。 |
| **工具链统一** | 近期以 msys64/MinGW-w64 为主力构建环境，保持现有构建路径可用；远期计划逐步向 MSVC 2022+ 迁移，构建配置随之简化。 |
| **工具链现代** | 远期目标为统一采用微软系工具链（MSVC 2022+、Windows SDK 10.0.22621+、vcpkg）。近期仍维护 MinGW-w64 构建路径，确保现有开发工作流不中断。 |
| **模块聚焦** | 每个 Phase 仅聚焦单一模块，避免跨模块大爆炸式改动，确保每次作业范围可控、可回滚、可独立验证。 |

### 1.3 Phase 总览

```
Phase 0 ── 基线与合规 ─────── 版权审计 / 衍生声明 / Git 标签
Phase 1 ── 标识层模块 ─────── version.h / About / ConsoleWindow / 资源更名
Phase 2 ── 构建系统模块 ───── CMake 升级 / Qt5 移除 / 保留 MinGW 兼容 / 远期 MSVC 统一
Phase 3 ── 包管理模块 ─────── 远期 vcpkg 集成 / 保留 MSYS2 脚本 / DLL 部署优化
Phase 4 ── Qt6 UI 模块 ────── 废弃 API 迁移 / 样式表 / 字体 / 可访问性
Phase 5 ── Win11 平台模块 ─── DPI / 长路径 / 暗色模式 / 文件对话框 / Snap
Phase 6 ── 遗留清理模块 ───── Win32 驱动废弃 / SDL 层废弃 / 跨平台宏清理
Phase 7 ── Rust 重构模块 ──── Corrosion 集成 / FFI 设计 / 构建整合
Phase 8 ── 发布部署模块 ───── CI 管道 / MSIX / 签名 / 自动更新
```

---

## 2. Phase 0：基线与合规

> **聚焦模块**: 法律合规、版本控制基线与项目结构清理  
> **不修改源代码**，但允许删除根目录下已确认过时的非代码文件。

### P0-1 源代码版权审计
- **范围**: 扫描 `src/` 下全部 `.cpp`、`.h`、`.c` 文件的文件头版权声明。
- **输出**: `COPYRIGHT_AUDIT.md`
- **要点**:
  - 不得删除或修改任何现有 `Copyright (C) YYYY Author` 声明。
  - 记录每份文件的版权归属、许可证声明、原始修改历史。

### P0-2 衍生作品声明制定
- **输出**: `DERIVATIVE_WORK_NOTICE.txt`（项目根目录）
- **要点**:
  - 明确说明本项目为 FCEUX 的衍生作品，提供原始项目 URL（https://fceux.com）。
  - 声明本作品遵循 GPLv2 发布。
  - 列出主要原始作者（Xodnizel、zeromus、adelikat、mjbudd77 等），以最小必要限度呈现。

### P0-3 基线冻结与分支策略
- 创建 Git 标签 `legacy/fceux-2.6.6-base`。
- 建立分支模型：`main`（稳定）、`dev`（集成）、`feature/*`（功能）。
- 确立代码审查规则：任何涉及版权文件的修改须经过合规检查。

### P0-4 基线构建与兼容性评估
> **本项已在 Phase 0 执行期间完成。**

- **构建环境**: MinGW-w64 GCC 16.1.0 + CMake 4.2.3 + Qt5
- **构建结果**: ✅ 成功 (`fceux.exe`, 9.6 MB)
- **总警告数**: ~1,133 条
- **关键发现**:
  - `alloca` / `__forceinline` 宏重定义（与 GCC 16 系统头冲突）。
  - `src/drivers/win/zlib/*.c` 存在 700+ 条旧式 C 函数定义警告。
  - Qt5 弃用 API 已触发编译器警告（`QMutex::Recursive`）。
  - C++20 弃用特性已出现（lambda 隐式 `this` 捕获、`volatile` 自增/自减）。
  - `CMakeLists.txt` 最低版本 3.8 被 CMake 4.x 标记为弃用。
- **输出物**: 原始数据已合并至本文档附录 D（`build_output.txt` 与 `build_warnings.csv` 作为临时产物已清理删除）。
- **作用**: 为后续 Phase 2~6 提供精确的已知问题清单，避免在实施时重复排查。

### P0-5 根目录过时文件清理
> **本项已在 Phase 0 执行期间完成。**

- **清理范围**: 项目根目录下确认过时的非代码文件，不影响源代码编译。
- **已删除文件清单**:
  | 文件 | 删除理由 |
  |------|----------|
  | `NEWS` | 空文件（0 字节） |
  | `README` | FCEUX SDL Linux 说明书（159 行），已被 `readme.md` 替代 |
  | `INSTALL` | 旧 Linux/Windows XP 安装说明（9 行），完全过时 |
  | `LICENSE` | 与 `COPYING` 内容完全重复（339 行 GPLv2） |
  | `ChangeLog` | 空文件（0 字节） |
  | `changelog.txt` | FCEUX 2.2.1 历史变更日志（820 行，2013~），信息已存在于 Git 历史中 |
  | `install_deps.bat` | MSYS2 MinGW/Qt5 依赖脚本（3 行），工具链已废弃 |
  | `install_deps.sh` | 同上 + Linux 不再支持 |
  | `install_qt5.ps1` | Qt5 安装脚本，Phase 4 迁移 Qt6 后过时 |
  | `build_output.txt` | Phase 0 临时编译日志，数据已合并至附录 D |
  | `build_warnings.csv` | Phase 0 临时警告数据，数据已合并至附录 D |
- **保留文件**:
  - `COPYING` — GPLv2 许可证文本（合规必需）。
  - `.gitignore`、`CMakeLists.txt`、`resources.qrc` — 构建与版本控制必需。
  - `COPYRIGHT_AUDIT.md/csv`、`DERIVATIVE_WORK_NOTICE.txt`、`DLL_DEPENDENCIES.md` — Phase 0 产出物。
  - `readme.md` — 待 Phase 1 更新为 FCEUX11 专用说明。
- **合规说明**: 被删除的文件均非源代码文件，不涉及 GPLv2 §2 对修改文件标注的要求。原始 FCEUX 的全部源代码与历史通过 Git 标签 `legacy/fceux-2.6.6-base` 完整保留。

---

## 3. Phase 1：标识层模块

> **聚焦模块**: `src/version.h`、`src/drivers/Qt/AboutWindow.cpp`、`src/drivers/Qt/ConsoleWindow.cpp`、资源文件  
> **目标**: 在不违反 GPLv2 的前提下，将可见标识统一更新为 FCEUX11。

### P1-1 版本体系重构
- **文件**: `src/version.h`
- **修改**:
  ```cpp
  #define FCEU_NAME "FCEUX11"
  #define FCEU_VERSION_MAJOR  0
  #define FCEU_VERSION_MINOR  1
  #define FCEU_VERSION_PATCH  0
  #define FCEU_VERSION_STRING "0.1.0" FCEU_SUBVERSION_STRING ...
  ```
- **合规**: 保留文件头部的原始 `Copyright (C) 2001 Aaron Oneal / 2002 Xodnizel` 声明，在末尾追加 FCEUX11 Contributors 声明。

### P1-2 About 窗口重构
- **文件**: `src/drivers/Qt/AboutWindow.cpp`
- **修改**:
  - 窗口标题: `"About FCEUX11"`（原 `"About fceuX"`）。
  - 产品名称: 增加 `"FCEUX11 - Windows 11 NES Emulator"`。
  - 作者列表: 保留 `Authors[]` 全部内容，上方增加注释：
    ```cpp
    // Original FCEUX authors listed below as required by GPLv2.
    // FCEUX11 is a derivative work based on FCEUX.
    ```
  - 版权行: `"Based on FCEUX by the FCEUX Development Team"` + `"© 2026 FCEUX11 Contributors | License: GPLv2"`。
  - 网站: 保留 https://fceux.com，新增 FCEUX11 仓库链接。

### P1-3 主窗口标题与菜单更新
- **文件**: `src/drivers/Qt/ConsoleWindow.cpp`
- **修改**:
  - 菜单项 `"&About FCEUX"` → `"&About FCEUX11"`。
  - 主窗口标题模板: `"FCEUX11 [ROM_NAME]"`。
  - 启动消息: `FCEUD_Message("Starting FCEUX11...\n");`。

### P1-4 可执行文件与资源文件更名
- **`src/CMakeLists.txt`**: `set( APP_NAME fceux11 )`
- **`icons/fceux.rc`**: 重命名为 `icons/fceux11.rc`，更新内部标识符。
- **`src/drivers/win/fceu_x86.manifest`**: 更新 `<assemblyIdentity name="FCEUX11" ... />`。
- **`scripts/copy_dependencies.ps1`**: 更新生成 `run_fceux11.bat`。
- **`readme.md`**: 全面更新产品名称，增加"基于 FCEUX 的衍生作品"声明。

---

## 4. Phase 2：构建系统模块

> **聚焦模块**: `CMakeLists.txt`、`src/CMakeLists.txt`  
> **目标**: 升级 CMake，全面简化构建配置，移除所有非 Windows 分支（Linux/macOS），同时保留 MinGW-w64/msys64 构建路径。

### P2-1 CMake 升级与项目名更新
- **文件**: `CMakeLists.txt`
- **修改**:
  ```cmake
  cmake_minimum_required(VERSION 3.28)
  project(FCEUX11 VERSION 0.1.0 LANGUAGES CXX C)
  ```
- **理由**: CMake 3.28 提供更佳的 C++20/23 模块支持、Preset 功能。
- **基线发现**: 当前 `cmake_minimum_required(VERSION 3.8)` 在 CMake 4.2.3 下触发 Deprecation Warning。

### P2-2 移除 Qt5 支持代码
- **文件**: `src/CMakeLists.txt`
- **修改**:
  - 删除 `QT6` CMake 选项及 `if( ${QT6} )` 条件分支。
  - 统一为 `find_package( Qt6 REQUIRED COMPONENTS Widgets OpenGL OpenGLWidgets )`。
  - 删除 `Qt5Widgets_DEFINITIONS`、`Qt5Widgets_INCLUDE_DIRS` 等引用。

### P2-3 保留 MinGW/MSYS2 构建分支（近期兼容要求）
- **文件**: `src/CMakeLists.txt`
- **策略**: 近期必须保持 msys64/MinGW-w64 构建兼容，**不得删除 `if(MINGW)` 相关逻辑**。
- **允许操作**:
  - 清理 `MINGW_PREFIX` 等硬编码路径，改用更通用的查找方式。
  - 与 `if(MSVC)` 分支并存，确保两者均可正常构建。
  - `install_deps.bat/sh` 中的 pacman 调用逻辑予以保留并维护。
- **禁止操作**: 在远期 MSVC 迁移计划明确前，不得移除 MinGW-w64 构建支持。

### P2-4 移除 Linux/macOS 构建分支与宏冲突修复
- **文件**: `src/CMakeLists.txt`、`src/types.h`、`src/lua-engine.cpp`
- **修改**:
  - 删除 `else(WIN32)` 块内的全部 Linux/macOS 逻辑：
    - `find_package(PkgConfig REQUIRED)`
    - `find_package(ZLIB REQUIRED)`
    - Unix 编译器标志（`-Wall -Wno-write-strings -fPIC`）
    - `GPROF_ENABLE`、`ASAN_ENABLE` 等仅适用于 Unix 的选项
  - 保留 Windows 块中的 `-D_CRT_SECURE_NO_WARNINGS`、`-D__SDL__`、`-D__QT_DRIVER__` 等定义。
  - **保留 MinGW 分支**: `if(MINGW)` 及其相关逻辑继续保留，确保 msys64 环境可正常构建。
- **基线发现 — 宏重定义冲突**:
  - `src/types.h:65`: `#define alloca __builtin_alloca` 与 MinGW GCC 16 `malloc.h` 冲突（300 次警告）。
    - **修复**: 增加 `#ifndef alloca` 守卫，或完全移除该宏定义。
  - `src/lua-engine.cpp:152`: `#define __forceinline __attribute__((always_inline))` 与 MinGW GCC 16 内置定义冲突。
    - **修复**: 增加 `#ifndef __forceinline` 守卫。
- **MSVC 迁移预判**: MSVC 不支持 `__builtin_alloca`，需改用 `<malloc.h>` 提供的 `alloca`；`__attribute__` 语法需替换为 `__forceinline` 关键字。

### P2-5 编译器标准升级与远期 MSVC 安全选项
- **文件**: `src/CMakeLists.txt`
- **修改**:
  ```cmake
  set(CMAKE_CXX_STANDARD 20)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
  ```
- **MinGW 兼容**: 上述标准升级对 MinGW-w64 GCC 16.x 同样适用，保持现有构建路径可用。
- **MSVC 安全选项（远期实施）**:
  - 待 MSVC 迁移完成后，新增：
    ```cmake
    add_compile_options(/guard:cf /GS /sdl /W4 /permissive-)
    add_link_options(/GUARD:CF /CETCOMPAT)
    ```
  - 现阶段 MinGW 构建继续沿用现有编译器标志。
- **基线发现 — 向 MSVC 迁移的额外风险（远期评估）**:
  - POSIX 函数缺失：`strcasestr`、`strtok_r` 等需替换为 Windows 等效实现。
  - POSIX 类型缺失：`ssize_t` 需映射为 `SSIZE_T`。
  - 内联汇编：若存在 AT&T 语法 `asm` 块，MSVC x64 完全不支持内联汇编，需改为 compiler intrinsics。

---

## 5. Phase 3：包管理模块

> **聚焦模块**: `vcpkg.json`、`scripts/copy_dependencies.ps1`、`install_deps.bat/sh`  
> **目标**: 评估 vcpkg 作为远期依赖管理器；**现阶段保留 MSYS2 生态**，确保 msys64 构建工作流不受影响。

### P3-1 vcpkg 集成（远期计划）
- **状态**: 远期实施项，待 MSVC 迁移时同步启用。
- **输出文件**: `vcpkg.json`（项目根目录）
  ```json
  {
    "name": "fceux11",
    "version": "0.1.0",
    "dependencies": [
      "qtbase", "sdl2", "libarchive", "zlib", "liblzma"
    ]
  }
  ```
- **triplet 策略**: 默认 `x64-windows`；评估 `x64-windows-static` 以减少部署依赖。
- **CMake 集成**:
  ```cmake
  if(DEFINED ENV{VCPKG_ROOT})
    set(CMAKE_TOOLCHAIN_FILE "$ENV{VCPKG_ROOT}/scripts/buildsystems/vcpkg.cmake" CACHE STRING "")
  endif()
  ```
- **说明**: 在 MinGW-w64 仍为主力构建环境的阶段，vcpkg 相关配置以并行方式存在，不强制替代现有 MSYS2 依赖管理。

### P3-2 保留并维护 MSYS2 安装脚本
- **操作**:
  - `install_deps.bat` → **保留并维护**，作为 msys64/MinGW-w64 环境下的一键依赖安装脚本。
  - `install_deps.sh` → 可删除（Linux/macOS 不再支持），但若内含 MinGW-w64 通用逻辑可酌情保留。
  - 新增 `setup_vcpkg.ps1`（可选）：面向未来 MSVC 环境的 PowerShell 脚本，与现有 MSYS2 脚本并存，不互相替代。

### P3-3 DLL 部署方案优化
- **文件**: `scripts/copy_dependencies.ps1`
- **现状**: 当前脚本硬编码 `D:\msys64\mingw64\bin` 路径，在 msys64 环境下工作正常。
- **近期策略**: 优化脚本路径检测逻辑，使其能自动识别常见 msys64 安装位置，降低硬编码依赖。
- **远期重构方案（待 MSVC 迁移时实施）**:
  - 方案 A：使用 `dumpbin /DEPENDENTS` 分析依赖，从 vcpkg `installed/x64-windows/bin` 复制 DLL。
  - 方案 B：利用 CMake 的 `install(TARGETS)` + `install(IMPORTED_RUNTIME_ARTIFACTS)` 自动生成部署目录。
  - 方案 C：使用 vcpkg 的 `applocal` 目标（`vcpkg install --x-install-root`）。
- **输出**: 近期继续维护 `scripts/copy_dependencies.ps1`（兼容 msys64）；远期再引入 `scripts/deploy.ps1`（面向 MSVC）。

---

## 6. Phase 4：Qt6 UI 模块

> **聚焦模块**: `src/drivers/Qt/` 下全部 UI 源文件  
> **目标**: 完成 Qt6 适配，建立 Windows 11 视觉体系。

### P4-1 Qt6 废弃 API 迁移与 C++20 兼容性修复
- **扫描**: 启用 `-DQT_DEPRECATED_WARNINGS_SINCE=0x060000`，使用 `clazy` 识别废弃调用。
- **迁移清单**:
  - `QRegExp` → `QRegularExpression`
  - `QLabel::pixmap()` 返回值类型适配
  - `QWheelEvent::delta()` → `QWheelEvent::angleDelta()`
  - `QAction` 构造函数签名更新
  - `QDesktopServices::storageLocation` → `QStandardPaths`
- **基线发现 — 已确认需修复的 Qt 弃用点**:
  - `src/utils/mutex.cpp:20`: `new QMutex(QMutex::Recursive)` → `new QRecursiveMutex()`（Qt6）。
  - `src/drivers/Qt/ConsoleWindow.cpp:174`: 同上。
- **基线发现 — C++20 弃用特性修复**:
  - `src/drivers/Qt/TasEditor/TasEditorWindow.cpp:6086`: `[=]` lambda 隐式捕获 `this` 已弃用。
    - **修复**: `[=, this]` 或显式列出捕获变量。
  - `src/drivers/common/nes_ntsc.h:115`: 不同枚举类型间的算术运算已弃用。
    - **修复**: 显式 `static_cast<int>()` 转换后再运算。

### P4-2 样式表系统建立
- **输出文件**:
  - `src/drivers/Qt/styles/dark.qss`
  - `src/drivers/Qt/styles/light.qss`
- **设计规范**:
  - 暗色：背景 `#202020`、控件 `#2D2D2D`、强调色 `#0078D4`。
  - 亮色：标准 Windows 11 浅色配色。
- **动态切换**: 菜单增加 `View -> Theme -> Dark / Light / System`。

### P4-3 Windows 11 字体渲染优化
- **实施**:
  ```cpp
  QFont font("Segoe UI Variable", 9);
  font.setStyleHint(QFont::SansSerif);
  QApplication::setFont(font);
  ```
- **高 DPI 字体**: 确保 `Qt::AA_EnableHighDpiScaling` 已启用时字体点大小正确缩放。

### P4-5 `volatile` 表达式弃用修复
- **聚焦文件**: `src/drivers/Qt/sdl-sound.cpp`
- **基线发现**: GCC 16 触发 16 次 `-Wvolatile` 警告，C++20 已弃用对 `volatile` 标量的 `++`/`--` 和复合赋值。
- **修复方案**:
  - 将 `volatile int s_BufferIn` 等变量改为 `std::atomic<int>`。
  - 若仅需编译器屏障，改用 `std::atomic_thread_fence`。
  - 此修复与 Phase 4 音频模块改动聚集，不扩散到其他文件。

### P4-4 可访问性改进
- 为所有 `QAction` 添加 `setShortcutVisibleInContextMenu(true)`。
- 为自定义控件（TAS 编辑器帧列表等）评估 `QAccessibleInterface` 实现。
- 确保暗色/亮色主题下的颜色对比度符合 WCAG 2.1 AA 标准。

---

## 7. Phase 5：Windows 11 平台模块

> **聚焦模块**: Windows 平台适配层（Manifest、Registry、COM、Win32 API）  
> **目标**: 使应用程序在 Windows 11 上获得原生级体验。

### P5-1 高 DPI 感知全面适配
- **Manifest** (`icons/fceux11.rc` 嵌入):
  ```xml
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  ```
- **Qt 属性** (`src/drivers/Qt/main.cpp`):
  ```cpp
  QApplication::setAttribute(Qt::AA_EnableHighDpiScaling);
  QApplication::setAttribute(Qt::AA_UseHighDpiPixmaps);
  QApplication::setHighDpiScaleFactorRoundingPolicy(Qt::HighDpiScaleFactorRoundingPolicy::PassThrough);
  ```
- **图标资源**: 准备多分辨率 `.ico`（16x16, 32x32, 48x48, 256x256）。

### P5-2 长路径支持
- **Manifest**:
  ```xml
  <windowsSettings>
    <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
  </windowsSettings>
  ```
- **编译定义**: 增加 `-D_UNICODE -DUNICODE`。
- **代码审查**: 确保文件 I/O 使用 `std::filesystem::path` 或 `QString::toStdWString()`。

### P5-3 暗色模式检测与响应
- **检测**:
  ```cpp
  QSettings settings("HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                     QSettings::NativeFormat);
  bool isDarkMode = settings.value("AppsUseLightTheme", 1).toInt() == 0;
  ```
- **响应**: 加载对应 Qt 样式表；若使用 `IFileDialog`，传递 `FOS_FORCEFILESYSTEM` 与暗色模式扩展。

### P5-4 现代文件对话框
- **步骤 1**: 确保 `QFileDialog::setOption(QFileDialog::DontUseNativeDialog, false)`。
- **步骤 2（可选增强）**: 评估直接调用 Windows COM `IFileDialog` 接口，获取完全原生的 Windows 11 文件对话框（含暗色模式）。
- **模块隔离**: 将文件对话框逻辑封装为 `Win32FileDialog` 辅助类，便于后续从 Qt 完全迁移时替换。

### P5-5 Snap Layouts 支持
- 确保主窗口 `Qt::WindowMaximizeButtonHint` 正确设置。
- 若未来实现自定义标题栏，需调用 `DwmExtendFrameIntoClientArea` 并响应 `WM_NCHITTEST` 以支持 Snap Layouts 悬停菜单。

---

## 8. Phase 6：遗留代码清理模块

> **聚焦模块**: `src/drivers/win/`、`src/drivers/sdl/`、核心代码中的跨平台宏  
> **目标**: 移除不再维护的代码路径，减少维护面。

### P6-1 标记 Win32 驱动为废弃
- **范围**: `src/drivers/win/` 下 legacy Win32 GUI 代码（`taseditor`、`directx`、`res.rc` 等）。
- **操作**:
  - 在 CMake 中移除对 `src/drivers/win/` 的引用。
  - 保留目录但不再参与构建。
  - 目录内添加 `README.DEPRECATED` 说明废弃原因与保留期限。
- **基线发现 — 内嵌 zlib 警告**: `src/drivers/win/zlib/*.c` 触发 700+ 次 `-Wold-style-definition`（K&R 风格函数定义）。
  - **处置**: 本 Phase 不修复这些 C 文件，而是在 P3-1 中通过 vcpkg 引入现代 zlib，随后在 P6-1 中彻底移除内嵌 zlib 目录。

### P6-2 标记 SDL 驱动为废弃
- **范围**: `src/drivers/sdl/` 下 Linux/SDL 适配层代码。
- **操作**:
  - 在 CMake 中移除引用。
  - 添加 `README.DEPRECATED`。

### P6-3 清理核心代码中的跨平台宏
- **范围**: 散布在 `src/` 核心模拟器代码中的 `#ifdef __linux__`、`#ifdef __APPLE__`、`#ifdef __unix__`。
- **策略**:
  - 低风险分支：直接删除非 Windows 代码路径。
  - 复杂分支：以 `#ifdef _WIN32` 替代通用宏；无法直接移除的非 Windows 分支改为 `#error "Platform not supported"` 编译期防护。
  - 保留 `src/drivers/Qt/` 中必要的 `#ifdef WIN32` 代码（如 `AttachConsole`、`QWindowsWindowFunctions`）。
- **基线发现补充**: `src/drivers/Qt/unix-netplay.cpp` 文件名即暴露其 Unix 起源，应在清理范围内。

---

## 9. Phase 7：Rust 重构模块

> **聚焦模块**: `src/rust/`（新建目录）  
> **目标**: 为性能关键组件引入 Rust，建立 FFI 边界。

### P7-1 Rust 工具链集成
- **文件**: `src/rust/CMakeLists.txt`、`src/rust/Cargo.toml`
- **实施**:
  - 引入 Corrosion 或手动调用 `cargo`。
  - Rust Edition 2024，**默认工具链锁定为 `stable-x86_64-pc-windows-msvc`**。
  - 目录结构:
    ```
    src/rust/
    ├── Cargo.toml
    ├── CMakeLists.txt
    └── src/
        └── lib.rs
    ```
- **环境要求**: Visual Studio 2022 C++ 工具链 + `rustup`。
- **兼容性说明**: Rust 模块集成属于远期计划，待 MSVC 迁移完成后实施。在 MinGW-w64 仍为主力构建环境的阶段，暂不强制引入 Rust 构建依赖。

### P7-2 FFI 边界设计与试点模块
- **原则**: Rust 以 `staticlib` 形式暴露 C ABI，C++ 侧通过 `extern "C"` 调用。
- **试点模块**（低风险、独立边界）:
  - **CRC32/Hash 计算**（ROM 校验）。
  - **图像滤镜/Scaler**（hq2x、Scale2x 等）。
  - **FM2 文件解析器**（纯文本格式，逻辑独立）。
- **禁止试点**: PPU 渲染循环、CPU 指令译码器（过于复杂，风险过高）。
- **工具**: 使用 `cbindgen` 自动生成 C 头文件。

### P7-3 构建流程整合
- **CMake 集成**:
  ```cmake
  find_package(Corrosion REQUIRED)
  corrosion_import_crate(MANIFEST_PATH rust/Cargo.toml)
  target_link_libraries(fceux11 PRIVATE fceux11_rust)
  ```
- **CI 要求**: GitHub Actions 安装 `rustup` + `stable-x86_64-pc-windows-msvc` target。

---

## 10. Phase 8：发布部署模块

> **聚焦模块**: `.github/workflows/`、`packaging/`  
> **目标**: 建立自动化的构建、测试、打包、发布流程。

### P8-1 GitHub Actions CI 管道
- **文件**: `.github/workflows/build.yml`
- **矩阵配置**:
  - **近期主配置**: `windows-latest` + MinGW-w64 (msys64) + Qt6
  - **远期配置**: `windows-latest` + MSVC 2022 + vcpkg + Qt6（待迁移完成后启用）
- **流水线阶段**:
  1. Checkout
  2. 设置构建环境（msys64 或 MSVC）
  3. 依赖缓存恢复（pacman 或 vcpkg）
  4. CMake 配置 + 构建
  5. 单元测试（如有）
  6. DLL 部署（`copy_dependencies.ps1` 或 `deploy.ps1`）
  7. Artifact 上传

### P8-2 MSIX 安装程序
- **方案**: 以 MSIX 为主分发格式，WiX v4 为高级用户备选。
- **输出**: `packaging/msix/Package.appxmanifest`
  ```xml
  <Identity Name="FCEUX11" Publisher="CN=FCEUX11 Contributors" Version="0.1.0.0" />
  ```

### P8-3 数字签名策略
- **开发阶段**: 自签名证书 + `signtool` 测试签名。
- **发布阶段**: 建议获取 OV/EV 代码签名证书，消除 SmartScreen 警告。

### P8-4 自动更新机制
- **方案**: 集成 `winsparkle`（Windows 专用）或自建 HTTP 检查。
- **数据**: 服务器托管 `updates.json`。

---

## 11. 模块依赖关系图

```
Phase 0 (基线合规)
    │
    ▼
Phase 1 (标识层) ─────────────────────────┐
    │                                       │
    ▼                                       │
Phase 2 (构建系统) ──► Phase 3 (包管理) ──► Phase 4 (Qt6 UI)
    │                    │                    │
    └────────────────────┴────────────────────┘
                         │
                         ▼
              Phase 5 (Win11 平台)
                         │
                         ▼
              Phase 6 (遗留清理)
                         │
                         ▼
              Phase 7 (Rust 重构)
                         │
                         ▼
              Phase 8 (发布部署)
```

> **说明**: Phase 1~3 之间无强依赖，可并行启动；Phase 4 依赖 Phase 2（Qt6 构建通过）；Phase 5 依赖 Phase 4（UI 基础稳定）；Phase 6 可在 Phase 4 之后随时进行；Phase 7 依赖 Phase 2（构建系统支持 Rust）；Phase 8 为最后阶段。

---

## 12. 风险登记册

| 风险 ID | 描述 | 可能性 | 影响 | 缓解措施 |
|---------|------|--------|------|----------|
| R-01 | Qt6 废弃 API 导致编译错误 | 高 | 中 | Phase 4 聚焦单一模块，利用 MSVC `/W4` 快速定位；逐文件修复，不跨模块扩散。 |
| R-02 | 品牌重塑引发合规质疑 | 中 | 高 | Phase 0 审计结果作为强制 check-in gate；所有标识修改在 Phase 1 内闭环。 |
| R-03 | vcpkg 依赖构建时间过长 | 中 | 中 | 启用 vcpkg 二进制缓存（`VCPKG_BINARY_SOURCES`）；CI 中缓存 `installed/` 目录。 |
| R-04 | Rust/MSVC FFI ABI 不匹配 | 中 | 高 | Phase 7 仅试点低风险模块；使用 `cbindgen`；编写完整边界测试。 |
| R-05 | 移除跨平台代码后无法回溯 | 低 | 低 | 原始 FCEUX 代码库仍作为 upstream；`legacy/fceux-2.6.6-base` 标签保留完整历史。 |
| R-06 | `alloca` / `__forceinline` / `volatile` 等旧兼容宏导致 MSVC 编译失败 | 中 | 高 | 已在 Phase 0 基线评估中精确定位，Phase 2 和 Phase 4 中按清单逐一修复；修复前使用 CI 复现验证。 |
| R-07 | `src/drivers/win/zlib/` 旧式 C 函数定义拖慢编译且警告噪音大 | 低 | 低 | 不直接修复：Phase 3 通过 vcpkg 引入外部 zlib，Phase 6 废弃该目录。 |
| R-08 | POSIX 类型/函数缺失导致 MSVC 构建失败 | 中 | 高 | Phase 2 转型即执行一次完整 MSVC 编译；将缺失函数/类型清单化，优先替换为 Win32 API 等效实现。

---

## 13. 附录

### 附录 A：GPLv2 合规检查清单

- [ ] 所有修改过的源代码文件保留了原始版权声明，并追加了新的版权声明。
- [ ] 项目根目录包含完整的 GPLv2 许可证文本（`COPYING` 或 `LICENSE`）。
- [ ] About 窗口中可见地展示了原始作者名单和"基于 FCEUX"的声明。
- [ ] 提供与二进制文件对应的完整源代码（Git 仓库公开可访问）。
- [ ] 未添加任何限制性许可证条款。

### 附录 B：术语表

| 术语 | 说明 |
|------|------|
| FCEUX | 原始 upstream NES 模拟器项目，网址 https://fceux.com |
| FCEUX11 | 本衍生项目，Windows 11 独占型发行版 |
| Phase | 聚焦单一模块的阶段性任务组 |
| FFI | Foreign Function Interface，跨语言调用接口 |
| MSIX | Windows 10/11 现代应用程序包格式 |

### 附录 C：参考文档

- `documentation/tech_docs/fm2.txt` — FM2 电影文件格式规范
- `documentation/tech_docs/fcs.txt` — FCS 存档状态格式规范
- `documentation/tech_docs/ppu/` — PPU 2C02 技术文档
- `documentation/tech_docs/cpu/` — APU/声音硬件文档
- `documentation/tech_docs/exp/` — 扩展 Mapper 芯片文档
- `LICENSE` / `COPYING` — GNU General Public License Version 2

### 附录 D：Phase 0 基线编译兼容性发现

本附录记录 Phase 0 基线构建（MinGW GCC 16.1.0 + Qt5）期间收集到的全部关键编译器警告和平台兼容性问题，作为后续 Phase 的已知问题清单。

#### D.1 警告统计概览

| 警告类别 | 数量 | 关键文件/区域 | 归属 Phase |
|----------|------|---------------|-----------|
| `-Wbuiltin-macro-redefined` (`alloca`) | ~300 | `src/types.h:65` | Phase 2 |
| `-Wbuiltin-macro-redefined` (`__forceinline`) | 1 | `src/lua-engine.cpp:152` | Phase 2 |
| `-Wold-style-definition` | ~700+ | `src/drivers/win/zlib/*.c` | Phase 6（间接解决） |
| `-Wdeprecated-declarations` (Qt API) | 2 | `mutex.cpp:20`, `ConsoleWindow.cpp:174` | Phase 4 |
| `-Wdeprecated-enum-enum-conversion` | 1 | `nes_ntsc.h:115` | Phase 4 |
| `-Wdeprecated` (lambda `this`) | 1 | `TasEditorWindow.cpp:6086` | Phase 4 |
| `-Wvolatile` | 16 | `sdl-sound.cpp` | Phase 4 |
| `-Wdeprecated` (CMake) | 1 | `CMakeLists.txt` (`min 3.8`) | Phase 2 |

#### D.2 `alloca` / `__forceinline` 宏冲突（Phase 2）

- **问题**: `src/types.h:65` 的 `#define alloca __builtin_alloca` 与 MinGW GCC 16 的 `<malloc.h>` 内置定义冲突。`src/lua-engine.cpp:152` 的 `#define __forceinline __attribute__((always_inline))` 同样冲突。
- **影响**: 目前为警告，但在 MSVC 上该定义无效且导致 `<malloc.h>` 中的 `alloca` 无法正确解析。
- **修复方向**: 增加 `#ifndef` 守卫，或完全移除宏定义，改用平台原生方案（MinGW 保留 `<malloc.h>`；MSVC 保留 `malloc.h` 中的 `alloca`）。

#### D.3 旧式 C 函数定义（Phase 6 / P3-1）

- **问题**: `src/drivers/win/zlib/*.c` 中存在大量 K&R 风格函数定义（参数类型在参数名之后声明），GCC 16 已将其标记为弃用。
- **处置**: 不修改这些 `.c` 文件。Phase 3 通过 vcpkg 引入外部 zlib 库；Phase 6 废弃 `src/drivers/win/zlib/` 目录。

#### D.4 Qt5 弃用 API（Phase 4）

- **问题**: `QMutex(RecursionMode)` 构造函数在 Qt6 中已删除，需替换为 `QRecursiveMutex`。
- **文件**: `src/utils/mutex.cpp:20`、`src/drivers/Qt/ConsoleWindow.cpp:174`。
- **修复**: 删除 Qt5 分支，统一使用 `QRecursiveMutex`。

#### D.5 C++20 标准弃用特性（Phase 4）

- **Lambda `this` 隐式捕获**: `src/drivers/Qt/TasEditor/TasEditorWindow.cpp:6086` 使用 `[=]` 捕获，`this` 被隐式包含。C++20 已弃用此行为。
  - **修复**: 改为 `[=, this]` 或显式列出捕获变量。
- **不同枚举间算术**: `src/drivers/common/nes_ntsc.h:115` 对两个不同枚举类型进行加减运算。
  - **修复**: 显式 `static_cast<int>()` 转换。
- **`volatile` 操作弃用**: `src/drivers/Qt/sdl-sound.cpp` 中对 `volatile int` 使用 `++`/`--` 和复合赋值。C++20 已弃用此行为（16 次警告）。
  - **修复**: 改为 `std::atomic<int>` 和原子操作，或改用显式内存序屏障。

#### D.6 CMake 最低版本弃用（Phase 2）

- **问题**: `CMakeLists.txt` 中 `cmake_minimum_required(VERSION 3.8)` 在 CMake 4.2.3 下触发弃用警告。
- **修复**: 升级为 `cmake_minimum_required(VERSION 3.28)`。

#### D.7 POSIX 到 MSVC 的迁移预判（Phase 2）

在基线评估期间，虽未实际使用 MSVC 编译，但已识别出以下可能的移植障碍：

| 障碍 | 影响区域 | 建议修复 |
|------|----------|----------|
| `ssize_t` | POSIX I/O 调用 | 映射为 `SSIZE_T` 或 `ptrdiff_t` |
| `strcasestr` | 字符串工具函数 | 替换为 `StrStrIA` (Shlwapi.h) 或自实现 |
| `strtok_r` | 解析逻辑 | 替换为 `strtok_s` (MSVC) |
| AT&T 内联汇编 | 任何 `.cpp` 中的 `asm` 块 | 替换为 compiler intrinsics（MSVC x64 不支持内联汇编） |

#### D.8 遗留可修复警告

- `-Wunused-variable`: 全局/局部变量定义但未使用。可在各 Phase 清理时顺带修复。
- `-Wshadow`: 变量遮蔽。同样可在修改对应文件时修复。
