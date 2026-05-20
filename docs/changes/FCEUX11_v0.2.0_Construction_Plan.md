# FCEUX11 v0.2.0 构建计划

> 版本：v0.2.0  
> 制定日期：2026-05-18  
> 目标发布日期：待定  

---

## 目录

1. [概述](#1-概述)
2. [任务一：死代码清理（Dead Code Elimination）](#2-任务一死代码清理dead-code-elimination)
3. [任务二：主界面标题规范与 About 窗口精简](#3-任务二主界面标题规范与-about-窗口精简)
4. [任务三：多语言国际化（i18n）支持](#4-任务三多语言国际化i18n支持)
5. [版本号同步清单](#5-版本号同步清单)
6. [里程碑与排期](#6-里程碑与排期)
7. [风险与回滚策略](#7-风险与回滚策略)

---

## 1. 概述

本计划针对 FCEUX11 v0.2.0 版本的三大建设目标进行任务拆解、技术方案设计与质量保障规划。

| 任务 | 目标 | 复杂度 | 风险等级 |
|------|------|--------|----------|
| 死代码清理 | 移除约 150,000+ 行废弃/无效代码，提升可维护性 | 高 | 🔴 高 |
| 标题与 About 规范 | 统一品牌展示，精简 About 对话框 | 低 | 🟢 低 |
| 多语言支持 | 建立 i18n 框架，支持 EN/ZH-CN/ZH-TW | 中 | 🟡 中 |

**关键约束**：
- 死代码清理必须拆分为**原子级子任务**，每个子任务配备独立的编译验证与行为回归测试。
- 所有修改必须通过 **CI 编译检查**后方可合并。
- 严禁在清理过程中修改任何活跃业务逻辑；仅做删除或归档操作。
- 编译操作请参考docs\tech\Build_Guide_MSYS2_Mingw64.md

---

## 2. 任务一：死代码清理（Dead Code Elimination）

### 2.1 现状评估

经代码审计，项目内死代码总量约 **150,400 行**，分为两大类：

| 类别 | 位置 | 规模 | 处置方式 |
|------|------|------|----------|
| **废弃目录** | `src/drivers/win/`, `src/drivers/sdl/`, `src/attic/`, `src/drivers/videolog/` | ~150,000 行 | 物理删除或归档至 `attic/` |
| **活跃文件内死代码** | `#if 0` 块、大段注释代码、未使用头文件 | ~400 行 | 就地清理 |

> **现状**：项目无现有单元测试框架，无 `add_test()` / `enable_testing()` 配置。因此本任务需**先建立最小防护体系**，再进行清理。

### 2.2 质量防护体系（先行任务）

#### 2.2.1 建立编译验证基线（Task 1.0）

**目标**：确保任何死代码清理都能通过"能否编译、能否链接、能否启动"的自动化验证。

**具体工作**：
1. 在 `src/tests/` 下新建目录结构：
   ```
   src/tests/
   ├── CMakeLists.txt          # 引入 enable_testing() + add_test()
   ├── smoke_test.cpp          # 最小可执行文件冒烟测试
   └── compilation_baseline/   # 编译基线脚本
   ```
2. `smoke_test.cpp` 职责：
   - 链接核心静态库（`fceux-core`、`boards`、`utils`、`drivers/Qt`）
   - 验证核心符号未被误删（如 `FCEUI_Init()`、`PowerNES()`、`FCEU_CreatePalette()` 等至少 20 个关键符号的地址非空）
   - 执行最小初始化序列后正常退出（返回码 0）
3. 根目录 `CMakeLists.txt` 追加：
   ```cmake
   option(FCEUX11_BUILD_TESTS "Build smoke and regression tests" ON)
   if(FCEUX11_BUILD_TESTS)
       enable_testing()
       add_subdirectory(src/tests)
   endif()
   ```
4. **CI 集成**：确保每次 PR 触发 `cmake --build .` + `ctest --output-on-failure`。

**验收标准**：
- [ ] `ctest` 在清理前全量通过（基线绿灯）。
- [ ] 所有后续子任务在修改后必须保持 `ctest` 通过。

#### 2.2.2 建立 Mapper 行为回归测试（Task 1.1）

**目标**：`src/boards/` 目录下部分死代码（如 `vrc5.cpp` 备用实现、`__dummy_mapper.cpp` 模板注释）涉及 Mapper 逻辑，误删可能影响 ROM 兼容性。

**具体工作**：
1. 在 `src/tests/boards/` 下新增测试程序：
   - `mapper_load_test.cpp`：加载一组代表性 ROM（覆盖 NROM/MMC1/MMC3/MMC5/VRC6/VRC7/AxROM/UxROM 等 8 个主要 Mapper），验证 `iNESLoad()` 返回成功且 Mapper 号匹配。
   - `mapper_reset_test.cpp`：对每个加载的 ROM 执行一次 `FCEUI_ResetNES()`，确认不崩溃。
2. 测试 ROM 使用开源 homebrew 或最小合法测试镜像（每个 < 8KB），存放于 `src/tests/fixtures/`。
3. 测试不验证图形/音频输出，仅验证**加载与重置不崩溃**。

**验收标准**：
- [ ] 8 个代表性 Mapper 的加载测试全部通过。
- [ ] `FCEUI_ResetNES()` 在加载后不崩溃。

---

### 2.3 死代码清理子任务（原子级拆分）

以下子任务**必须顺序执行**（从低风险到高风险），每个子任务独立提交（独立 commit），独立通过 CI。

---

#### Task 1.2：删除 `src/drivers/videolog/` 废弃目录

- **范围**：`src/drivers/videolog/` 共 6 个文件，~3,026 行。
- **死代码证据**：
  - 未被 `src/drivers/CMakeLists.txt` 或 `src/CMakeLists.txt` 引用。
  - 活跃代码中仅剩字符串残留（Qt 代码中 "videolog" 字样），无实际函数调用。
- **操作**：
  1. `git rm -r src/drivers/videolog/`
  2. 搜索并删除 Qt 代码中残留的 "videolog" 字符串常量（仅清理字符串，不修改逻辑）。
- **测试**：`smoke_test.cpp` 编译通过 + `ctest` 通过。
- **预估工时**：0.5 人日

---

#### Task 1.3：物理删除 `src/drivers/win/` 与 `src/drivers/sdl/`

- **范围**：
  - `src/drivers/win/`：189 文件，~100,665 行（含 `taseditor/`、`directx/`、`res.rc`、嵌入式 `zlib`）
  - `src/drivers/sdl/`：35 文件，~19,242 行
- **死代码证据**：两目录均含 `README.DEPRECATED`（2026-05-17 正式废弃），已从 CMake 中排除。
- **操作**：
  1. `git rm -r src/drivers/win/ src/drivers/sdl/`
  2. 更新 `src/drivers/` 下任何引用这两个目录路径的文档或注释。
- **测试**：`smoke_test.cpp` + Mapper 回归测试。
- **预估工时**：0.5 人日

---

#### Task 1.4：物理删除 `src/attic/`

- **范围**：49 文件，~27,340 行（DOS/PC/SDL 旧驱动、`sexyal` 音频库含 20,387 行的 `convert.inc`、旧文档压缩包）。
- **死代码证据**：目录名即表明为 "attic"（阁楼/杂物间），无 CMake 引用。
- **操作**：
  1. `git rm -r src/attic/`
- **测试**：`smoke_test.cpp` + Mapper 回归测试。
- **预估工时**：0.5 人日

---

#### Task 1.5：清理活跃文件中的 `#if 0` 死代码块

- **范围**：5 处共 146 行，精确位置如下：

| 文件 | 行号 | 行数 | 内容 |
|------|------|------|------|
| `src/utils/backward.hpp` | 2105–2160 | 56 | `crit_err_hdlr` 信号处理函数 |
| `src/utils/backward.hpp` | 996–1038 | 43 | 内联调试信息回溯回退逻辑 |
| `src/state.cpp` | 998–1021 | 24 | 联机存档临时文件逻辑（`FCEUnetplay`） |
| `src/lua-engine.cpp` | 4599–4615 | 17 | Lua GUI 绘制旧实现 |
| `src/fceu.cpp` | 1107–1112 | 6 | 文件 stdout 调试输出 |

- **操作**：逐块删除 `#if 0` … `#endif` 包裹的代码，保留其上方注释说明（如有）。
- **测试**：`smoke_test.cpp` + Mapper 回归测试。特别注意 `backward.hpp` 是堆栈回溯库，删除后需确认 Qt 的崩溃报告对话框仍能正常工作（手动验证一次即可）。
- **预估工时**：1 人日

---

#### Task 1.6：清理大段 `/* */` 注释代码块

- **范围**：活跃文件中的代码型注释（含分号、return、大括号特征），精确位置：

| 文件 | 行号 | 行数 | 内容 |
|------|------|------|------|
| `src/boards/vrc5.cpp` | 62–129 | 68 | Mapper VRC5 备用实现 |
| `src/movie.cpp` | 1479–1521 | 43 | 旧电影文件处理逻辑 |
| `src/lua-engine.cpp` | 4359–4395 | 37 | Lua 回调/注册函数旧代码 |
| `src/lua-engine.cpp` | 1613–1635 | 23 | Lua 钩子区域计算旧逻辑 |
| `src/movie.cpp` | 196–217 | 22 | 电影元数据处理旧代码 |
| `src/conddebug.cpp` | 21–42 | 22 | 条件调试表达式解析备用代码 |
| `src/drivers/Qt/input.cpp` | 1199–1218 | 20 | Qt 输入配置旧逻辑 |
| `src/drivers/Qt/avi/gwavi.cpp` | 43–66 | 24 | AVI 写入器旧代码块 |
| `src/boards/__dummy_mapper.cpp` | 多处 | ~25 | 模板 mapper 注释掉的 WRAM/CHRRAM 分配代码 |

- **操作**：逐块删除，保留文件头部版权注释和功能性说明注释。
- **测试**：`smoke_test.cpp` + Mapper 回归测试。对 `vrc5.cpp` 的修改需额外运行 VRC5 Mapper ROM 冒烟测试。
- **预估工时**：1.5 人日

---

#### Task 1.7：重构 `src/oldmovie.cpp`

- **范围**：692 行中活跃代码仅 ~53 行，其余为 639 行注释/死代码（含 214 行最大连续注释块）。
- **操作**：
  1. 保留 `convert_metadata()` 和 `movie_readchar()` 两个活跃函数。
  2. 删除所有死代码和纯历史注释。
  3. 为保留的 53 行代码补充最小文档注释，说明文件用途（旧电影格式兼容层）。
- **测试**：`smoke_test.cpp` + 若存在旧 `.fcm` 格式测试文件则运行兼容性测试。
- **预估工时**：1 人日

---

#### Task 1.8：清理未使用的 TasEditor 头文件

- **范围**：`src/drivers/Qt/TasEditor/` 下以下头文件未被任何 `.cpp` 文件 `#include`：
  - `bookmark.h`
  - `branches.h`
  - `greenzone.h`
  - `markers_manager.h`
  - `snapshot.h`
  - `splicer.h`
  - `taseditor_lua.h`
- **操作**：
  1. 二次确认：使用 `grep -r "bookmark.h" src/drivers/Qt/` 等交叉验证，确保无间接包含。
  2. `git rm` 删除上述 7 个文件。
  3. 若对应的 `.cpp` 文件已不存在或为空，一并删除。
- **测试**：`smoke_test.cpp`。TasEditor 为高级功能，手动启动程序验证 TasEditor 菜单项仍可打开（Qt 编译器会在缺失必需头文件时报错）。
- **预估工时**：0.5 人日

---

#### Task 1.9：清理零散 `//` 注释旧代码块

- **范围**：`src/drivers/Qt/` 下多段连续 `//` 注释的旧代码：

| 文件 | 行号范围 | 行数 | 内容 |
|------|----------|------|------|
| `src/drivers/Qt/config.cpp` | 985–1020 | 36 | 配置系统旧代码 |
| `src/drivers/Qt/fceuWrapper.cpp` | 1163–1267 | ~76 | 包装器旧逻辑 |
| `src/drivers/Qt/NameTableViewer.cpp` | 1602–1639 | 38 | PPU 查看器旧绘制逻辑 |
| `src/drivers/Qt/GuiConf.cpp` | 690–713 | 24 | GUI 配置旧代码 |

- **操作**：逐块删除。
- **测试**：`smoke_test.cpp` + 手动启动程序验证对应对话框/配置页面能正常打开。
- **预估工时**：1 人日

---

#### Task 1.10：清理其他零散死代码

- **范围**：
  - `src/types-des.h:449`：未使用静态数组 `hexValid[23]`
  - `src/fceu.h:103`：注释掉的 `#include "driver.h"`
  - `src/boards/164.cpp`、`boards/ax5705.cpp`、`boards/coolboy.cpp`、`boards/et-100.cpp`、`boards/sb-2000.cpp`、`netplay.cpp`、`sound.cpp`、`nsf.h` 中的 6–18 行小块死代码
- **操作**：逐个删除。
- **测试**：`smoke_test.cpp` + Mapper 回归测试。
- **预估工时**：1 人日

---

### 2.4 死代码清理任务汇总

| 子任务 | 内容 | 死代码行数 | 测试要求 | 工时 |
|--------|------|-----------|----------|------|
| 1.0 | 建立编译验证基线 | — | `smoke_test.cpp` 通过 | 1 人日 |
| 1.1 | Mapper 行为回归测试 | — | 8 个 Mapper 加载+重置通过 | 1 人日 |
| 1.2 | 删除 `videolog/` | ~3,026 | 编译+冒烟 | 0.5 人日 |
| 1.3 | 删除 `win/` + `sdl/` | ~119,907 | 编译+冒烟 | 0.5 人日 |
| 1.4 | 删除 `attic/` | ~27,340 | 编译+冒烟 | 0.5 人日 |
| 1.5 | 清理 `#if 0` 块 | ~146 | 编译+冒烟+崩溃报告手动验证 | 1 人日 |
| 1.6 | 清理 `/* */` 代码块 | ~260 | 编译+冒烟+Mapper 回归 | 1.5 人日 |
| 1.7 | 重构 `oldmovie.cpp` | ~639 | 编译+冒烟+旧格式兼容 | 1 人日 |
| 1.8 | 删除未使用 TasEditor 头文件 | ~7 文件 | 编译+冒烟+TasEditor 手动验证 | 0.5 人日 |
| 1.9 | 清理 `//` 注释旧代码 | ~174 | 编译+冒烟+对应页面手动验证 | 1 人日 |
| 1.10 | 零散死代码清理 | ~60 | 编译+冒烟+Mapper 回归 | 1 人日 |
| **合计** | | **~150,400 行** | | **9.5 人日** |

---

## 3. 任务二：主界面标题规范与 About 窗口精简

### 3.1 现状

| 位置 | 当前内容 | 问题 |
|------|----------|------|
| 主窗口标题 | `FCEU_NAME_AND_VERSION` → `FCEUX11 0.1.0-interim git...` | 格式不规范，无 "v" 前缀 |
| About 窗口标题 | `About FCEUX11` | 正确，无需改动 |
| About 窗口内容 | 包含 git URL、git Revision、原版 FCEUX 网站链接、完整作者名单、完整依赖库版本列表 | 信息冗余，品牌展示不聚焦 |

### 3.2 目标规范

- **主窗口标题**：`FCEUX11 v0.2.0`
- **About 窗口**：在合规（保留 GPLv2 要求的原作者署名和版权声明）前提下最精简化。

### 3.3 子任务拆分

#### Task 2.1：版本号同步更新

- **修改文件**：
  - `src/version.h`：
    ```cpp
    #define FCEU_VERSION_MINOR  2   // 从 1 改为 2
    #define FCEU_VERSION_STRING "0.2.0"  FCEU_SUBVERSION_STRING ...
    ```
  - `CMakeLists.txt`（根目录）：`project(FCEUX11 VERSION 0.2.0 LANGUAGES CXX C)`
  - `src/rust/Cargo.toml`：`version = "0.2.0"`
- **测试**：编译后启动程序，验证窗口标题包含 `v0.2.0`。
- **预估工时**：0.5 人日

#### Task 2.2：主窗口标题格式规范化

- **修改文件**：`src/drivers/Qt/ConsoleWindow.cpp`
- **修改内容**：
  - 当前：`setWindowTitle( tr(FCEU_NAME_AND_VERSION) );`
  - 目标：标题格式固定为 `"FCEUX11 v" + 版本号`，移除 `-interim git` 等后缀。
  - 在 `src/version.h` 中新增宏（可选方案）：
    ```cpp
    #define FCEU_DISPLAY_VERSION "v" FCEU_VERSION_STRING
    #define FCEU_NAME_AND_VERSION FCEU_NAME " " FCEU_DISPLAY_VERSION
    ```
  - 或直接修改 `ConsoleWindow.cpp`：
    ```cpp
    setWindowTitle(QString("FCEUX11 v%1").arg(FCEU_VERSION_STRING));
    ```
- **测试**：启动程序，验证标题栏精确显示 `FCEUX11 v0.2.0`（或含 debug 后缀的 `FCEUX11 v0.2.0 debug`）。
- **预估工时**：0.5 人日

#### Task 2.3：About 窗口精简化

**合规约束**（GPLv2 要求）：
- 必须保留版权声明。
- 必须提供获取完整许可证的方式。
- 建议保留原作者署名（Derivative work based on FCEUX）。

**精简方案**：

- **保留内容**：
  1. FCEUX11 Logo（128×128）
  2. 软件名称 + 版本号：`FCEUX11 v0.2.0`
  3. 单行版权声明：`© 2026 FCEUX11 Contributors | Based on FCEUX | License: GPLv2`
  4. 一个超链接按钮：`View License` → 打开 `COPYING` 文件或 GitHub 上的 GPL 文本
  5. `OK` 关闭按钮

- **删除内容**：
  - git URL / git Revision 显示（开发者信息移至 `Help -> System Info` 或完全移除）
  - 原版 FCEUX 网站独立链接（已隐含在 "Based on FCEUX" 中）
  - 完整的 `Authors[]` 名单（过长，且 GPLv2 不要求必须在 About 窗口中展示完整作者列表，只需保留声明即可）
  - `FCEUI_GetAboutString()` 输出的依赖库版本清单（移至 `Help -> System Info` 或完全移除）
  - 所有 `Compiled with Qt/SDL/zlib/ffmpeg...` 技术细节

- **修改文件**：`src/drivers/Qt/AboutWindow.cpp`
  - 删除 `Authors[]` 数组。
  - 删除 `credits` QTextEdit 及其所有内容填充逻辑。
  - 删除条件编译块（`#ifdef _USE_LIBAV`、`_USE_X264`、`_USE_LIBARCHIVE`、`_S9XLUA_H`、`ZLIB_VERSION`）。
  - 窗口尺寸从 `512×600` 缩小为 `400×280` 左右。
  - 布局简化为：Logo → 版本标签 → 版权标签 → View License 按钮 → OK 按钮。

- **测试**：
  - 编译通过。
  - 手动验证 About 窗口能正常打开、超链接可点击、`OK` 按钮可关闭。
  - 确认 `COPYING` 文件存在于发布包中。
- **预估工时**：1 人日

---

## 4. 任务三：多语言国际化（i18n）支持

### 4.1 需求

- 建立独立的语言包机制。
- 初期支持：**英文（en）**、**简体中文（zh_CN）**、**繁体中文（zh_TW）**。
- 运行时切换语言，无需重启（Qt 支持动态翻译重载）。

### 4.2 技术方案：Qt Linguist + QTranslator（推荐）

FCEUX11 基于 Qt 构建，代码中已广泛使用 `tr()` 宏包裹 UI 字符串，天然具备 i18n 基础。最佳方案是**Qt 原生国际化框架**。

#### 4.2.1 架构设计

```
assets/i18n/                    # 语言包目录（建议新建）
├── fceux11_en.ts               # 英文源文件（提取的源字符串）
├── fceux11_zh_CN.ts            # 简体中文翻译
├── fceux11_zh_TW.ts            # 繁体中文翻译
├── fceux11_en.qm               # 编译后的二进制翻译包
├── fceux11_zh_CN.qm
└── fceux11_zh_TW.qm
```

#### 4.2.2 核心实现步骤

| 步骤 | 工作 | 文件 |
|------|------|------|
| 1 | 在 `CMakeLists.txt` 中集成 `lupdate` / `lrelease` | 根目录 `CMakeLists.txt` |
| 2 | 创建 `assets/i18n/` 目录和初始 `.ts` 文件 | 新建 |
| 3 | 运行 `lupdate` 扫描 `src/drivers/Qt/` 下所有 `tr()` 字符串 | 构建时自动生成 |
| 4 | 翻译 `zh_CN.ts` 和 `zh_TW.ts` | 人工翻译 |
| 5 | 运行 `lrelease` 生成 `.qm` 二进制文件 | 构建时自动生成 |
| 6 | 在 `main.cpp` / `ConsoleWindow` 中初始化 `QTranslator` 并加载 `.qm` | `src/drivers/Qt/main.cpp` |
| 7 | 在 `Preferences` 或 `Help` 菜单中增加语言切换选项 | `ConsoleWindow.cpp` |
| 8 | 实现动态语言切换（无需重启） | `ConsoleWindow.cpp` |

#### 4.2.3 CMake 集成示例

```cmake
find_package(Qt6 COMPONENTS LinguistTools)  # 或 Qt5

set(TS_FILES
    assets/i18n/fceux11_en.ts
    assets/i18n/fceux11_zh_CN.ts
    assets/i18n/fceux11_zh_TW.ts
)

qt_add_translations(${APP_NAME}
    TS_FILES ${TS_FILES}
    QM_FILES_OUTPUT_VARIABLE QM_FILES
)

# 安装时复制 .qm 到输出目录
install(FILES ${QM_FILES} DESTINATION ${CMAKE_INSTALL_PREFIX}/assets/i18n)
```

#### 4.2.4 C++ 运行时加载示例

```cpp
// ConsoleWindow 或 main.cpp
#include <QTranslator>
#include <QLocale>
#include <QApplication>

static QTranslator *appTranslator = nullptr;

void consoleWin_t::loadTranslation(const QString &langCode)
{
    if (!appTranslator) {
        appTranslator = new QTranslator(qApp);
    }
    qApp->removeTranslator(appTranslator);
    
    QString tsPath = QString(":/i18n/fceux11_%1.qm").arg(langCode);
    if (appTranslator->load(tsPath)) {
        qApp->installTranslator(appTranslator);
    } else {
        // 加载失败回退到英文（源字符串）
    }
    
    // 触发所有已创建窗口的 retranslateUi()
    // 对于代码中手动创建的 UI，需遍历重设所有 setText/tr()
    updateWindowTitle();
    // ... 其他需要重载的 UI 元素
}
```

> **注意**：由于 FCEUX11 大量使用代码动态创建 UI（非 `.ui` 文件），没有自动生成的 `retranslateUi()`。需在 `ConsoleWindow` 和各对话框中手动实现字符串刷新函数，或在切换语言时重启界面。

#### 4.2.5 语言切换菜单设计

在 `Help` 菜单或新增 `Tools -> Language` 子菜单中：

```
Language (语言)
├── English
├── 简体中文 (Simplified Chinese)
└── 繁體中文 (Traditional Chinese)
```

- 选择后立即调用 `loadTranslation()` + UI 刷新。
- 当前语言偏好保存到 Qt `QSettings`（`fceux11.ini`），下次启动自动加载。

---

### 4.3 Rust 实现可行性评估

#### 4.3.1 评估结论

| 评估项 | 结论 | 说明 |
|--------|------|------|
| **技术可行性** | ⚠️ 可行但极不推荐 | Rust 可管理 `.qm` 文件加载逻辑，但无法与 Qt `tr()` 系统集成 |
| **工程合理性** | ❌ 不合理 | Qt i18n 是完整闭环：`tr()` → `lupdate` → `.ts` → `lrelease` → `.qm` → `QTranslator`。引入 Rust 会打破此闭环 |
| **维护成本** | ❌ 显著增加 | 需要 Rust ↔ C++ 双向 FFI 封装 `QTranslator`、`QLocale`、`QEvent`（LanguageChange），复杂度远超收益 |
| **性能收益** | ❌ 无 | 翻译加载是一次性 I/O 操作，无性能瓶颈 |

#### 4.3.2 详细分析

1. **Qt `tr()` 宏与 `QTranslator` 的耦合**：
   - Qt 的 `tr()` 宏在运行时会查询当前安装在 `QCoreApplication` 上的 `QTranslator` 实例。
   - `QTranslator` 是 Qt 对象，继承自 `QObject`，支持信号/槽和事件系统（`QEvent::LanguageChange`）。
   - Rust 没有原生 Qt 绑定，若要在 Rust 中实现同等功能，需要：
     - 用 `cxx` 或 `bindgen` 生成 Qt 的 FFI 封装（数十万行 C++ 头文件）。
     - 手动桥接 `QTranslator::translate()` 虚函数。
     - 处理 `QEvent::LanguageChange` 的分发和接收。

2. **`.qm` 文件格式**：
   - `.qm` 是 Qt 私有二进制格式，由 `lrelease` 生成。
   - Rust 社区没有成熟的 `.qm` 解析库（只有一些不完整的逆向工程实现）。
   - 若用 Rust 自行实现翻译查找，必须脱离 Qt 生态，重写整个字符串替换机制。

3. **正确的 Rust 使用边界**：
   - Rust 在当前项目中用于**计算密集型、无 Qt 依赖**的任务（如 `crc32fast`）。
   - 多语言属于**UI 框架层**功能，应完全由 Qt C++ 实现。

#### 4.3.3 建议

**否决**使用 Rust 实现多语言模块。多语言功能应作为 Qt 原生模块，在 `src/drivers/Qt/i18n/` 或 `assets/i18n/` 下维护。

---

### 4.4 子任务拆分

#### Task 3.1：搭建 Qt i18n 基础设施

- **工作**：
  1. 新建 `assets/i18n/` 目录。
  2. 在根目录 `CMakeLists.txt` 中集成 `Qt6LinguistTools`（或 Qt5 对应模块）。
  3. 创建初始 `fceux11_en.ts`（空翻译，仅提取源字符串）。
  4. 编写 CMake 规则，使构建时自动运行 `lupdate` + `lrelease`。
  5. 在 `resources.qrc` 中注册 `.qm` 文件路径，确保内嵌到可执行文件。
- **测试**：构建成功，输出目录生成 `.qm` 文件。
- **预估工时**：1 人日

#### Task 3.2：实现翻译加载与语言切换

- **工作**：
  1. 在 `src/drivers/Qt/ConsoleWindow.h` 中声明 `loadTranslation(QString)` 和 `retranslateUi()`。
  2. 在 `ConsoleWindow.cpp` 中实现 `QTranslator` 的创建、加载、安装。
  3. 在 `main.cpp` 中根据 `QSettings` 或系统默认语言初始加载翻译。
  4. 在 `Help` 菜单新增 `Language` 子菜单，含三个 QAction（EN / ZH_CN / ZH_TW）。
  5. 连接 QAction 的 `triggered` 信号到 `loadTranslation()`。
  6. 实现 `retranslateUi()`：重设主窗口标题、菜单栏所有 `setTitle()` / `setText()`。这是最大工作量，需遍历所有菜单和动作。
- **测试**：
  - 启动程序，验证默认语言正确。
  - 切换语言，验证菜单文字即时变更。
  - 重启程序，验证 `QSettings` 持久化生效。
- **预估工时**：2 人日

#### Task 3.3：提取与翻译字符串

- **工作**：
  1. 运行 `lupdate` 扫描 `src/drivers/Qt/` 下所有 `.cpp` / `.h` 文件，提取 `tr()` 字符串到 `.ts`。
  2. 使用 Qt Linguist 工具翻译 `zh_CN.ts` 和 `zh_TW.ts`。
  3. 优先翻译高频界面：主菜单（File / Emulation / Config / Debug / Tools / Help）、常用对话框（Open ROM、Preferences、About）、状态栏提示。
  4. 专业术语对照表（示例）：
     | 英文 | 简体中文 | 繁體中文 |
     |------|----------|----------|
     | Open ROM | 打开 ROM | 開啟 ROM |
     | Emulator | 模拟器 | 模擬器 |
     | Frame | 帧 | 幀 |
     | Scanline | 扫描线 | 掃描線 |
     | Save State | 即时存档 | 即時存檔 |
     | Load State | 即时读档 | 即時讀檔 |
     | Cheat | 金手指 | 金手指 |
     | PPU Viewer | PPU 查看器 | PPU 檢視器 |
     | Hex Editor | 十六进制编辑器 | 十六進位編輯器 |
     | Lua Script | Lua 脚本 | Lua 腳本 |
  5. 运行 `lrelease` 生成 `.qm`。
- **测试**：
  - 在简体中文 Windows / 系统环境下启动，验证界面显示中文。
  - 在繁体中文环境下启动，验证界面显示繁体中文。
  - 确认英文环境下无异常（源字符串fallback）。
- **预估工时**：3 人日（翻译工作量占主要部分）

#### Task 3.4：对话框级翻译刷新支持

- **工作**：
  - FCEUX11 有大量独立对话框类（`AboutWindow`、`GamePadConf`、`CheatsDialog`、`PPUViewer`、`HexEditor` 等）。
  - 这些对话框在语言切换时若已打开，不会自动更新文字。
  - 方案 A（推荐）：语言切换时关闭所有对话框，弹出提示 "Language changed. Please reopen dialogs."
  - 方案 B（完整）：为每个对话框实现 `changeEvent(QEvent *event)` 监听 `LanguageChange`，重设所有标签文字。
  - **v0.2.0 建议采用方案 A**，降低复杂度；方案 B 作为 v0.3.0 技术债务。
- **测试**：语言切换时，已打开的 About 窗口文字不崩溃；方案 A 下关闭后重新打开显示新语言。
- **预估工时**：0.5 人日

---

## 5. 版本号同步清单

升级至 v0.2.0 时，以下文件必须同步修改：

| 文件路径 | 当前值 | 目标值 | 备注 |
|----------|--------|--------|------|
| `src/version.h` | `FCEU_VERSION_MINOR 1` | `FCEU_VERSION_MINOR 2` | 驱动 C++ 代码版本 |
| `src/version.h` | `"0.1.0"` | `"0.2.0"` | 版本字符串 |
| `CMakeLists.txt` | `VERSION 0.1.0` | `VERSION 0.2.0` | CMake 项目版本 |
| `src/rust/Cargo.toml` | `version = "0.1.0"` | `version = "0.2.0"` | Rust crate 版本 |
| `src/rust/Cargo.lock` | 多处 `0.1.0` | 更新为 `0.2.0` | 运行 `cargo update` 自动刷新 |
| `vcpkg.json`（如有版本字段） | 检查并更新 | `0.2.0` | 包管理版本 |

---

## 6. 里程碑与排期

| 里程碑 | 包含任务 | 预估工期 | 交付物 |
|--------|----------|----------|--------|
| **M1：质量基线** | Task 1.0 + 1.1 | 2 人日 | `src/tests/` 编译基线 + Mapper 回归测试 |
| **M2：死代码清理** | Task 1.2 ~ 1.10 | 7.5 人日 | 150,000+ 行死代码移除，全部 CI 绿灯 |
| **M3：品牌规范** | Task 2.1 ~ 2.3 | 2 人日 | 统一标题 + 精简 About |
| **M4：i18n 基础设施** | Task 3.1 | 1 人日 | 构建时自动生成 `.qm`，QTranslator 可加载 |
| **M5：语言切换与翻译** | Task 3.2 ~ 3.4 | 5.5 人日 | 三语言完整支持，菜单级动态切换 |
| **Release** | 全量回归 + 打包 | 2 人日 | v0.2.0 发布包 |

**总预估工期**：约 **20 人日**。

建议按以下顺序推进：

```
Week 1: M1（测试基线） → M2（死代码清理，分批提交）
Week 2: M2（收尾） → M3（品牌规范）
Week 3: M4（i18n 基建） → M5（翻译填充）
Week 4: M5（收尾） → Release（回归测试 + 打包）
```

---

## 7. 风险与回滚策略

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 死代码误删导致编译失败 | 🔴 高 | 每个子任务独立 commit，独立 CI；编译失败立即 revert |
| 误删活跃代码（如 `oldmovie.cpp` 中实际被调用的函数） | 🔴 高 | 清理前先用 `grep -r` 全仓库交叉验证符号引用；Mapper 回归测试覆盖 |
| 翻译字符串遗漏导致英文混杂 | 🟡 中 | `lupdate` 对比 diff，建立翻译完成度检查清单 |
| 动态语言切换导致已打开对话框崩溃 | 🟡 中 | v0.2.0 采用方案 A（关闭对话框 + 提示），避免运行时状态不一致 |
| `tr()` 宏包裹不完整（部分硬编码中文/英文） | 🟢 低 | 代码审查时专门检查新增/修改的 UI 字符串是否使用 `tr()` |

### 回滚策略

- **死代码清理**：每个子任务一个独立分支 + PR，禁止多个子任务合并提交。任何 CI 失败直接 `git revert <commit>`。
- **版本号与 About**：修改量小，直接 revert 即可。
- **多语言**：`.ts` 文件纳入版本控制，翻译错误可随时修正；`QTranslator` 加载失败自动 fallback 到源字符串，无功能损失。

---

*本计划由 FCEUX11 开发团队制定，适用于 v0.2.0 版本迭代周期。*
