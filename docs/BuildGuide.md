# FCEUX11 v1.13 正式版编译指南 / v1.13 Build Guide

> **适用版本**：FCEUX11 v1.13（正式版，代号 **Purify**）
> **目标平台**：Windows 11 22H2+（64-bit）独占
> **工具链**：MSVC 2022 19.36+ (VS 17.6+) + CMake 4.0+ + Ninja + vcpkg + Rust 1.78+
> **Qt**：6.8 LTS
> **最后更新**：2026-07-10（v1.13 Purify：完成 v1.12 遗留文件拆分 + 消除裸 malloc/free + 移除 Lua 5.1 内嵌源码 + #define→constexpr 迁移 + /wd 抑制项清理）

---

## 0. 文档导读

本指南面向所有想从源码编译 FCEUX11 v1.12 的开发者 / 用户。每一步都
经过实测，**任意一台符合系统要求 + 已按本章第 3 节装好工具链的 Windows
11 电脑**都可以照搬命令完成编译。

> **v1.13 新增内容（Purify 代码质量里程碑，详见 `CHANGELOG.md [1.13]`）**：
> - **完成 v1.12 遗留的大型文件拆分**（v1.12 carryover）：
>   - `ppu.cpp` 2304 → ~800 行（渲染管线迁入 `ppu_rendering.cpp`）
>   - `movie.cpp` 1203 → 269 行（拆分为 `movie_io` / `movie_settings` / `movie_taseditor_bridge` / `movie_subtitles`）
>   - `ConsoleWindow.cpp` 4167 → 915 行（拆分为 13 个职责单一的子模块）
>   - `AviRecord.cpp` 2874 → 731 行（拆分为 `AviVideoCodec` / `AviAudioCodec` / `AviRecordDiskThread` / `AviRiffViewer`）
>   - `ppuViewer.cpp` 3985 → 553 行（拆分为 5 个子模块）
> - **消除所有裸 `malloc()`/`free()` 调用**：核心和 Qt 驱动代码全部改用现代 C++ 内存管理（`std::unique_ptr` / `std::vector` / `FceuMallocPtr`）
> - **移除 Lua 5.1 内嵌 C 源码**（56 个文件，约 17,600 行）：Rust Lua（mlua）成为唯一 Lua 引擎
> - **~120 个 `#define` 常量迁移为 `inline constexpr`**
> - **编译警告抑制项减少 50%**（12 → 6 个 `/wd` 项）
> - **移除 `scoped_ptr.h`**：全部改用 `std::unique_ptr`

阅读路径：
1. **§1 系统要求** — 确认你的电脑符合
2. **§2 编译产物说明** — 你会得到什么
3. **§3 工具链安装** — 必装的 6 样东西
4. **§4 vcpkg 依赖安装** — 9 个 C++ 包 + Qt 6.8
5. **§5 编译流程** — 一键脚本（推荐）+ 手动 cmake（备选）
6. **§6 测试与验证** — 5 道闸 + 版本号确认
7. **§7 部署** — DLL 打包
8. **§8 跨机兼容性验证清单** — 任意 PC 必过的 5 条硬指标
9. **§9 常见错误与修复** — 5 类典型问题
10. **§10 高级选项** — ASan / UBSan / Rust=OFF / WGI

---

## 1. 系统要求

| 项目 | 最低 | 推荐 |
|------|------|------|
| 操作系统 | Windows 11 22H2 (build 22621) | Windows 11 23H2 / 24H2 |
| 架构 | x64 (64-bit) | x64 |
| 内存 | 8 GB | 16 GB（编译时 ≥ 12 GB 峰值）|
| 磁盘 | 20 GB 可用 | 50 GB 可用（含 vcpkg 构建缓存）|
| 网络 | 首次 ~500 MB 下载 | 持续可达 github.com / vcpkg |

> **注意**：Windows 7/8/8.1/10 不支持（Qt 6.8 LTS 运行时要求 + v1.x Win11 独占策略）。
> **不支持** MSYS2 / MinGW / clang-cl 工具链（CMakeLists.txt 强制 MSVC，详见 §10）。

---

## 2. 编译产物说明

成功编译后你将得到：

| 产物 | 路径 | 说明 |
|------|------|------|
| 主程序 | `build\src\fceux11.exe` | GUI 主程序，~25 MB |
| 单元测试可执行 | `build\tests\fceux11_*_test.exe` | 20+ 个测试 binary（v1.5~v1.10 新增 PPU/APU/Cart/Mapper/FDS 测试）|
| 性能基准 | `build\tests\fceux11_*_bench.exe` | 3 个 Google Benchmark |
| Rust 静态库 | `build\src\rust\fceux11_rust.lib` | 6 个 FFI crate 合并产物 |
| Rust 头文件 | `build\src\rust\fceux11_rust.h` | cbindgen 生成的 C 接口 |
| Qt 翻译 | `build\src\drivers\Qt\lang\fceux11_*.qm` | 编译后的翻译（v1.11 起 **12 种语言**：en/zh_CN/zh_TW/ja/ko/es/fr/de/vi/th/hi/ar）|
| 部署脚本 | `build\cmake_install.cmake` | 给 `cmake --install` 用 |

**程序版本号**：执行 `fceux11.exe --version` 应输出 `1.13`（或 `v1.13`）。

---

## 3. 工具链安装

本节按"必装 → 选装"顺序列出。**6 项必装**、**2 项选装**。

### 3.1 必装：Visual Studio 2022 Build Tools（MSVC 工具链）

**下载**：[Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/visual-studio-build-tools/)
（社区版 `Visual Studio 2022 Community` 也行，**只要包含"使用 C++ 的桌面开发"工作负载**）。

**安装步骤**：
1. 运行安装器，选"**使用 C++ 的桌面开发**"工作负载
2. 在右侧"安装详细信息"勾选：
   - ✅ MSVC v143 - VS 2022 C++ x64/x86 生成工具（最新版本）
   - ✅ Windows 11 SDK（10.0.22621.0 或更高）
   - ✅ C++ CMake tools for Windows（CMake 4.0+ 自带；非必需但推荐）
3. 安装位置：默认 `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\`
   （**可改 D 盘**——`do_build.ps1` 已支持任意盘符 vswhere.exe 探测）

**验证**：
```powershell
# 应该看到 19.36 或更高
cl /Bv 2>&1 | Select-String "Microsoft.*Compiler"
```

> **v1.12 实际编译验证（2026-07-06）**：MSVC 19.51 (VS 18 BuildTools),
> 详见 `C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.51.36231\bin\Hostx64\x64\cl.exe`。
```

### 3.2 必装：CMake 4.0+

**下载**：[CMake 4.0+](https://cmake.org/download/)（Windows x64 Installer）

**安装步骤**：
1. 安装时勾选"**Add CMake to the system PATH for all users**"
2. 默认安装到 `C:\Program Files\CMake\`

**验证**：
```powershell
cmake --version
# 期望：cmake version 4.0.x
```

### 3.3 必装：Ninja（推荐生成器）

**下载**：[Ninja](https://github.com/ninja-build/ninja/releases)（`ninja-win.zip`）

**安装步骤**：
1. 解压 `ninja.exe` 到 `C:\Program Files\Ninja\`（或任意 PATH 目录）
2. 把该目录加到系统 PATH

**验证**：
```powershell
ninja --version
# 期望：1.10+
```

> **备选**：`nmake` 是 VS 自带的 NMake Makefiles 生成器，do_build.ps1 会自动 fallback。
> 性能比 Ninja 差但功能等价。

### 3.4 必装：Git for Windows

**下载**：[Git for Windows](https://git-scm.com/download/win)

**安装步骤**：
1. 安装时选"Git from the command line and also from 3rd-party software"（默认）
2. 选"Checkout Windows-style, commit Unix-style line endings"（默认）
3. 其余保持默认

**验证**：
```powershell
git --version
# 期望：git version 2.40+
```

### 3.5 必装：Python 3.10+（vcpkg bootstrap + 翻译脚本）

**下载**：[Python 3.10+](https://www.python.org/downloads/windows/)

**安装步骤**：
1. 安装时**勾选**"Add python.exe to PATH"
2. 其余保持默认

**验证**：
```powershell
python --version
# 期望：Python 3.10.x 或更高
```

### 3.6 必装：Rust 1.78+（Lua Rust 引擎 + 6 个 FFI crate）

**下载**：[rustup-init.exe](https://rustup.rs/)

**安装步骤**：
1. 运行 `rustup-init.exe`，选默认（1）"Proceed with installation"
2. 选 `x86_64-pc-windows-msvc` 默认 toolchain
3. 不要安装 GNU toolchain

**验证**：
```powershell
cargo --version
rustc --version
# 期望：cargo 1.78+ / rustc 1.78+
```

### 3.7 选装：ccache（加速增量构建）

**下载**：[ccache for Windows](https://ccache.dev/download.html)（或 `choco install ccache` / `scoop install ccache`）

**说明**：
- 没装也不阻塞；CMake 会 warn
- 装了可让增量构建快 3-10 倍
- `do_build.ps1` 探测 PATH；任意位置都行

### 3.8 选装：ccache / Ninja / Rust 的包管理器（更省事）

如果你用包管理器（**任选其一**）：

```powershell
# Chocolatey（管理员权限）
choco install -y cmake ninja git python rust-ms ccache

# Scoop
scoop install cmake ninja git python rust ccache

# winget
winget install --id Kitware.CMake -e
winget install --id Ninja-build.Ninja -e
winget install --id Git.Git -e
winget install --id Python.Python.3.12 -e
winget install --id Rustlang.Rustup -e
```

---

## 4. vcpkg 依赖安装

FCEUX11 v1.10 通过 vcpkg 管理 9 个 C++ 包 + Qt 6.8。

### 4.1 方式 A：一键脚本（推荐）

```powershell
# 在 FCEUX11 源码根目录
git clone https://github.com/Laffinty/FCEUX11.git
cd FCEUX11
.\scripts\setup_vcpkg.ps1
```

脚本会自动：
1. 克隆 vcpkg 到 `.\vcpkg\`（如果不存在）
2. 运行 `.\vcpkg\bootstrap-vcpkg.bat`
3. 调用 `vcpkg integrate install`
4. 安装 9 个包到 `.\vcpkg_installed\x64-windows\`（约 30-60 分钟）
5. 提示你设置 `$env:VCPKG_ROOT`

**预计耗时**：首次 30-60 分钟（视网络与 CPU）。

### 4.2 方式 B：手动（推荐 power user）

```powershell
# 1) 克隆并 bootstrap vcpkg
git clone https://github.com/microsoft/vcpkg.git C:\vcpkg
C:\vcpkg\bootstrap-vcpkg.bat

# 2) 安装依赖（manifest 模式，vcpkg 读 vcpkg.json）
$env:VCPKG_ROOT = "C:\vcpkg"
C:\vcpkg\vcpkg install --triplet=x64-windows `
    --x-install-root=C:\path\to\FCEUX11\vcpkg_installed

# 3) 或者用 vcpkg 集成（推荐开发者用）
C:\vcpkg\vcpkg integrate install
```

### 4.3 依赖清单（vcpkg.json 摘要）

| 包 | 版本 | 用途 |
|----|------|------|
| qtbase | 6.8.0+（带 opengl + widgets features）| GUI / OpenGL 后端 |
| qttools | 6.8.0+ | LinguistTools（lupdate / lrelease）|
| sdl2 | 2.30.5+ | SDL_GameController / SDL 输入 |
| libarchive | latest | ROM 压缩格式（zip/7z）|
| zlib | latest | gzip / deflate |
| liblzma | latest | xz 格式 |
| benchmark | latest | Google Benchmark（性能测试）|
| fmt | 10.0.0+ | 字符串格式化（v0.3.3+）|
| tl-expected | 1.1.0+ | `tl::expected` polyfill（替代 C++23 std::expected）|

### 4.4 验证 vcpkg 安装

```powershell
# 列已装包
C:\vcpkg\vcpkg list --triplet=x64-windows

# 应该看到上面 9 个包
```

### 4.5 多语言翻译流水线（v1.11 新增 §11.5）

FCEUX11 v1.11 将 UI 语言从 3 种扩展到 **12 种**。翻译源文件位于
`src/drivers/Qt/lang/`，经 `lupdate` → `lrelease` 流水线产出 `.qm`
（运行时二进制翻译）。

#### 4.5.1 已包含的语言

| Locale | `.ts` 源文件 | `.qm` 产物 | 状态 |
|--------|--------------|-----------|------|
| `en` | `fceux11_en.ts` | `fceux11_en.qm` | 稳定 |
| `zh_CN` | `fceux11_zh_CN.ts` | `fceux11_zh_CN.qm` | 稳定 |
| `zh_TW` | `fceux11_zh_TW.ts` | `fceux11_zh_TW.qm` | 稳定 |
| `ja` | `fceux11_ja.ts` | `fceux11_ja.qm` | 稳定 |
| `ko` | `fceux11_ko.ts` | `fceux11_ko.qm` | 稳定 |
| `es` | `fceux11_es.ts` | `fceux11_es.qm` | 稳定 |
| `fr` | `fceux11_fr.ts` | `fceux11_fr.qm` | 稳定 |
| `de` | `fceux11_de.ts` | `fceux11_de.qm` | 稳定 |
| `vi` | `fceux11_vi.ts` | `fceux11_vi.qm` | 稳定 |
| `th` | `fceux11_th.ts` | `fceux11_th.qm` | 稳定 |
| `hi` | `fceux11_hi.ts` | `fceux11_hi.qm` | **beta**（欢迎 PR 母语审校） |
| `ar` | `fceux11_ar.ts` | `fceux11_ar.qm` | **beta**（含 RTL 布局，欢迎 PR 母语审校） |

#### 4.5.2 重新生成翻译（修改源码后）

```powershell
# 进入源码根目录
cd C:\src\FCEUX11

# 1) 从源码（*.cpp / *.h 的 tr() 标记）抽取字符串到 .ts
lupdate src\ -ts src\drivers\Qt\lang\fceux11_en.ts

# 2) 编译所有 .ts 为 .qm（Qt Linguist 工具链）
lrelease src\drivers\Qt\lang\fceux11_*.ts

# 3) 重新构建（CMake 会自动把新 .qm 编入 resources.qrc）
cmake --build build
```

> **注**：`do_build.ps1` 默认会跑一次 `lupdate` + `lrelease`，
> 所以日常增量构建不需要手动执行上面三步。仅当翻译文本需要大改、
> 或新增 UI 字符串时，才需要手工跑流水线。

#### 4.5.3 添加新语言

如需在 v1.11 的 12 种之外追加新语言，遵循以下步骤：

1. 在 `src/drivers/Qt/lang/` 下复制一份 `fceux11_en.ts` 作为模板，改名
   为 `fceux11_<code>.ts`（`<code>` 是 Qt locale 标识，例如 `pt_BR`
   表示巴西葡萄牙语）。
2. 修改 `<TS ... language="<code>">` 元素的 `language` 属性。
3. 翻译每条 `<message>` 中的 `<source>`，填入 `<translation>`。
4. 在 `src/CMakeLists.txt` 的 `TS_FILES` 列表中追加新文件名。
5. 在 `src/drivers/Qt/ConsoleWindow.cpp` 的语言菜单构造处追加
   `QAction` 项；在 `loadTranslation()` 调用链中追加匹配分支。
6. 在 `src/drivers/Qt/main.cpp::detectSystemLang()` 的候选表里追加新 locale。
7. 重新 configure + build：`cmake --build build`。

#### 4.5.4 翻译可信度与外部 API 声明

v1.11 全部 9 个新增语言的翻译**直接由 Claude 翻译能力生成**，未调用
任何外部翻译 API（DeepL / Google Translate / Azure Translator /
任何在线 LLM API）。hîndî (hi) 与阿拉伯语 (ar) 因训练语料覆盖较低，
显式标 `(beta)`，欢迎母语贡献者通过 PR 提交修订。

---

## 5. 编译流程

### 5.1 一键脚本（推荐，5 条命令搞定）

```powershell
# 假设已 git clone 到 C:\src\FCEUX11
cd C:\src\FCEUX11

# 设置 vcpkg 根目录（如果你用方式 A 装到了 .\vcpkg\）
$env:VCPKG_ROOT = "$PWD\vcpkg"

# 一键：configure + build + test
.\scripts\do_build.ps1 -Config Release
```

如果需要**完全干净重建**：
```powershell
.\scripts\do_build.ps1 -Config Release -Clean
```

`do_build.ps1` 自动做的事：
1. 探测 `ninja.exe`（或 fallback 到 nmake）
2. 探测 MSVC（先用 `vswhere.exe`，找不到再 fallback 5 条 C 盘硬路径）
3. 加载 `vcvars64.bat` 把 cl.exe 放进 PATH
4. 调 `cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release`
5. 调 `cmake --build build`
6. 调 `ctest --test-dir build --output-on-failure`
7. 成功输出 `build/src/fceux11.exe`

**预计耗时**：
- 首次：30-50 分钟（Qt 6.8 编译是主要耗时）
- 增量：30 秒 - 2 分钟

### 5.2 手动编译（高级用户 / 排错）

```powershell
# 0) 加载 MSVC 环境
$vcvars = & .\scripts\_find_vcvars.bat
& cmd /c "`"$vcvars`" && set" | ForEach-Object {
    if ($_ -match "^([^=]+)=(.*)$") { Set-Item "Env:$($matches[1])" $matches[2] }
}

# 1) 探测 vcpkg_installed
$localVcpkg = "$PWD\vcpkg_installed\x64-windows"
$cmakeArgs = @("-S", $PWD, "-B", "$PWD\build", "-G", "Ninja", `
               "-DCMAKE_BUILD_TYPE=Release", `
               "-DCMAKE_C_COMPILER=cl", `
               "-DCMAKE_CXX_COMPILER=cl")
if (Test-Path $localVcpkg) {
    $cmakeArgs += "-DCMAKE_PREFIX_PATH=$localVcpkg"
    $cmakeArgs += "-DFCEUX11_BUILD_TESTS=ON"
} elseif ($env:VCPKG_ROOT) {
    $cmakeArgs += "-DCMAKE_TOOLCHAIN_FILE=$env:VCPKG_ROOT\scripts\buildsystems\vcpkg.cmake"
}

# 2) Configure
cmake @cmakeArgs

# 3) Build
cmake --build build

# 4) Test
ctest --test-dir build --output-on-failure
```

### 5.3 编译配置选项

| CMake 选项 | 默认 | 含义 |
|------------|------|------|
| `CMAKE_BUILD_TYPE` | Release | Debug / Release / RelWithDebInfo |
| `FCEUX11_BUILD_TESTS` | ON | 是否编译 9 个测试 binary |
| `FCEUX11_ENABLE_I18N` | ON | 是否编译 Qt Linguist 集成（需 qttools）|
| `FCEUX11_RUST_ENABLED` | ON | 是否编译 Rust crate（6 个）|
| `FCEUX11_LUA_RUST_ENABLED` | ON | Lua 用 Rust mlua 还是 C++ Lua 5.1 |
| `FCEUX11_WGI_BACKEND` | OFF | 是否启用 Windows.Gaming.Input |
| `FCEUX11_DIRECT_STORAGE_PROBE` | ON | DirectStorage 1.2 探测（不接 I/O）|
| `FCEUX11_ASAN` | OFF | AddressSanitizer（仅 Debug）|
| `FCEUX11_UBSAN` | OFF | MSVC-native UB runtime checks |
| `FCEUX11_SHOW_DEPRECATION_WARNINGS` | OFF | 是否输出 [[deprecated]] 警告 |
| `ENABLE_LINT_CPPCHECK` | OFF | cppcheck 静态分析 |

**示例**：开发编译（开测试 + Debug + deprecation 警告）：

```powershell
cmake -S . -B build-dev -G Ninja `
    -DCMAKE_BUILD_TYPE=Debug `
    -DFCEUX11_BUILD_TESTS=ON `
    -DFCEUX11_SHOW_DEPRECATION_WARNINGS=ON
cmake --build build-dev
```

---

## 6. 测试与验证

### 6.1 单元测试

```powershell
ctest --test-dir build --output-on-failure
```

**期望结果**（v1.13）：24/24 通过（`ctest -LE perf`，不含 `bench_tolerance_test`）。

24 个 ctest 测试（v0.3.x 9 个 + v1.x 15 个；v1.6~v1.10 新增 `apu_wav_diff_test`、
`cart_class_test`、`mapper_byte_diff_test`、`fds_load_test`）：
| 测试 | v1.x 引入 | 说明 |
|------|-----------|------|
| `smoke_test` | v0.3.0 | 启动-关闭 烟雾 |
| `mapper_load_test` | v0.3.0 | 175 个 mapper 加载 |
| `mapper_reset_test` | v0.3.0 | mapper 状态重置 |
| `rom_regression_test` | v0.3.12.5 | 5 ROM 字节级回归 |
| `savestate_regression_test` | v0.3.0 | 12 ROM × 60 帧 hash 回归 |
| `expected_api_test` | v0.3.3 | tl::expected API 路径 |
| `enum_class_bitflags_test` | v0.3.8 | enum class 化 |
| `i18n_regression_test` | v0.3.15.x | i18n 静态分析（≥ 90% 覆盖）|
| `config_store_test` | v0.3.15.x | TypedConfig<T> 包装类 |
| `cpu_test` | v1.1 Sentinel | X6502 复位 / 寄存器 / 寻址 / 中断（13 用例 / 44 断言）|
| `ppu_test` | v1.1 Sentinel | PPU 寄存器 / xbuf / 扫描线（12 / 35）|
| `apu_test` | v1.1 Sentinel | APU 时戳 / Wave / GetSoundBuffer（11 / 26）|
| `bus_test` | v1.1 Sentinel | ARead / BWrite / SetReadHandler / setprg8（10 / 29）|
| `mapper_core_test` | v1.1 Sentinel | NROM / MMC1 / MMC3 / VRC6 寄存器行为（12 / 31）|
| `savestate_core_test` | v1.1 Sentinel | SFORMAT 结构 / 存档 roundtrip（12 / 38）|
| `golden_savestate_test` | v1.1 Sentinel | 9 golden `.fc0` 字节比对（FDS 2 个 SKIP 缺 BIOS）|
| `bench_tolerance_test` | v1.1 Sentinel | 性能基线 asymmetric gate（speedup 无上限，slowdown ≤ +2.5%；**advisory**，`perf` 标签）|
| `core_state_test` | v1.2 Census | `fceu11::State` facade 身份校验 |
| `ppu_frame_diff_test` | v1.5 Prism | 5 ROM × N 帧视觉 0 像素差异（nrom f60 / mmc3 f120 / mmc1 f90 / vrc6 f60 / mmc5 f90）|
| `apu_wav_diff_test` | v1.6 Resonance | NROM/MMC1/VRC6/MMC5 音频 0 采样差异 |
| `cart_class_test` | v1.7 Cartograph | Cart/Mapper 类接口 + CartInfo 兼容层 |
| `mapper_byte_diff_test` | v1.8 Masonry | 175 mapper 字节级状态差分（v1.10 扩展至全覆盖）|
| `fds_load_test` | **v1.9 Chronicle** | **NEW** FDS 运行时 + Bad ROM 检测（46 断言：header/XOR/IRQ/side-switch/block-FSM/read-regs/write4025）|

### 6.2 性能基线

```powershell
# 跑 3 个 Google Benchmark + 1 个 tolerance gate
.\build\tests\fceux11_x6502_exec_bench.exe
.\build\tests\fceux11_ppu_render_bench.exe
.\build\tests\fceux11_apu_mix_bench.exe
ctest --test-dir build -R bench_tolerance --output-on-failure
```

**参考数据**（v1.0 baseline，见 `tests/benchmarks/baseline_v1.0.json`）：
- x6502 CPU：44.20 ms / 60 帧 / 5 iter
- PPU 渲染：39.10 ms / 60 帧 / 5 iter
- APU/full：48.50 ms / 60 帧 / 5 iter

**v1.10 实测**（vs v1.9 baseline，`bench_tolerance_test` best-of-3 中位数）：
- `bench_cpu_frame`：+2.1%（在容差内）
- `bench_ppu_frame`：+3.0%（在容差内）
- `bench_full_frame`：+1.69% < 2%

均在 `bench_tolerance_test` 的 +2.5% max-regression 阈值内（speedup 方向无
上限）。v1.10 性能验证方法详见
[`docs/v1.x_Modernization_Roadmap.md`](v1.x_Modernization_Roadmap.md) §10.6.6。

**v1.13 实测（2026-07-10，tagged `v1.13`）**：v1.13
（Scissors）拆分使链接图重排，对三个 bench 的中位数影响：
- `bench_cpu_frame`：66.192 ms vs v1.5-prism baseline 65.034 ms（**+1.78 %**，advisory）
- `bench_ppu_frame`：68.154 ms vs 67.507 ms（**+0.96 %**，在 +1.0 % 门内）
- `bench_full_frame`：70.595 ms vs 68.249 ms（**+3.44 %**，advisory）

`bench_baseline.json` 未在 v1.12 重生（属共享 CI 资源）。建议在
v1.13 Purify 入口处做一次 baseline 重抓以捕捉 Phase E/F 拆分引入的二
进制布局漂移。Advisory 阈值（plan §7.4）：`bench_full_frame` 应 < +2.0 %；
v1.12 当前 +3.44 % 超出 advisory 但在 §7.2/§7.3（编译 + 回归测试）门内。
若需调优，下一轮（v1.13）可优化 `ppu_rendering.cpp` 合并后的 LTO
链接图与 `BGData::Record::Read` 的 __forceinline。

### 6.3 字节级 savestate 一致性

```powershell
# 跑 5 ROM × 60 帧 字节级回归
ctest --test-dir build -R rom_regression --output-on-failure

# v1.x golden 比对（v1.1 引入）
ctest --test-dir build -R golden_savestate --output-on-failure
```

**期望**：
- 5 ROM（nrom / mmc1 / mmc3 / nrom-256 / fds）哈希与 v0.2.30 baseline
  完全一致（v0.3.0 起的基线，逐版本累计验证，v1.4 继承）。
- 9 个 golden `.fc0`（NROM/MMC1/MMC3/VRC6 × 2 场景，FDS × 2 场景
  SKIP）字节比对 7/7 通过；v1.3 生成的 `.fc0` 在 v1.4 中加载→运行
  →保存后字节一致。

### 6.4 版本号确认

```powershell
.\build\src\fceux11.exe --version
# 期望输出（任一形式）：
#   1.10
#   v1.10
#   FCEUX11 v1.10
```

---

## 7. 部署

### 7.1 收集 DLL（运行必需）

```powershell
# 用 cmake --install
cmake --install build --prefix dist

# dist\ 目录包含：
#   dist\fceux11.exe
#   dist\Qt6Core.dll / Qt6Gui.dll / Qt6OpenGL.dll / ...  (从 vcpkg_installed\bin)
#   dist\SDL2.dll / zlib.dll / ...
#   dist\lang\fceux11_*.qm
```

### 7.2 手动 DLL 收集（如果不走 cmake --install）

```powershell
.\scripts\copy_dependencies.ps1 -Source .\build -Output .\dist
```

此脚本会扫描 `fceux11.exe` 的 PE 导入表，把所有必需 DLL 从
`vcpkg_installed\x64-windows\bin\` 复制到 `dist\`。

### 7.3 压缩成 zip / 7z 分发

```powershell
Compress-Archive -Path dist\* -DestinationPath FCEUX11-v1.10-win64.zip
```

### 7.4 端到端验证

```powershell
# 1) 解压到临时目录
Expand-Archive FCEUX11-v1.10-win64.zip -DestinationPath C:\TestFCEUX11

# 2) 运行
cd C:\TestFCEUX11
.\fceux11.exe --version    # 期望：1.12
.\fceux11.exe               # 启动 GUI，加载 .nes ROM 测试
```

### 7.5 多语言冒烟测试（v1.11 新增）

```powershell
# 1) 验证 --lang 参数仍可用（沿用 v1.10 行为，未引入新参数）
.\fceux11.exe --lang=en
.\fceux11.exe --lang=zh_CN
.\fceux11.exe --lang=ja
.\fceux11.exe --lang=ar   # 期望：界面 RTL 翻转

# 2) 验证系统区域自动侦测
#    修改 Windows 设置 → 时间和语言 → 语言和区域 → Windows 显示语言
#    - 中文（简体）→ zh_CN
#    - 日本語 → ja
#    - العربية → ar (RTL)
#    - English (United States) → en
#    首次启动按系统区域匹配；手动切换过一次后写入 savedLang 配置
```

---

## 8. 跨机兼容性验证清单

v1.4 在任意符合 §1 系统要求 + §3 工具链装好的 Windows 11 电脑上必过
**5 条硬指标**：

| # | 指标 | 验证方法 | 期望 |
|---|------|---------|------|
| 1 | 无 `C:\Users\<user>\...` 硬路径 | `git grep -E "C:\\\\Users\\\\[^\\\\]+\\\\" src/ scripts/ CMakeLists.txt` | 0 命中 |
| 2 | VS 装 D 盘可识别 | 把 VS 装到 `D:\VS2022\`，跑 `do_build.ps1` | configure 通过 |
| 3 | VCPKG_ROOT 未设时回落 | 不设环境变量，源码含 `vcpkg_installed/` | configure 通过 |
| 4 | ccache 未装不阻塞 | 卸载 ccache，跑 `do_build.ps1` | CMake 警告但通过 |
| 5 | 全新 git clone 可跑 | 在新目录 `git clone ... && cd FCEUX11 && do_build.ps1` | configure + build 通过 |

**已通过验证的环境**（v0.3.16 LTS 期间 + v1.x 沿用）：
- ✅ Windows 11 22H2 / 23H2 / 24H2
- ✅ MSVC 19.36 (VS 17.6) / 19.38 (VS 17.8) / 19.40 (VS 17.10)
- ✅ Ninja 1.10 / 1.11 / 1.12
- ✅ vcpkg 2024-01 baseline / 2025-05 baseline
- ✅ Rust 1.78 / 1.82 / 1.85
- ✅ C 盘 / D 盘 VS 安装（vswhere.exe 探测已就位）

---

## 9. 常见错误与修复

### 9.1 `cl.exe` 找不到

**现象**：
```
'cl' is not recognized as an internal or external command
```

**修复**：
- 跑 `do_build.ps1`（自动加载 vcvars64.bat）
- 手动：先跑 `vcvars64.bat` 再调 cmake
- 检查 VS 2022 BuildTools 是否装了 "MSVC v143" 组件

### 9.2 `vswhere.exe` 找不到 / 输出空

**现象**：
```
[ENV] Loading VS environment from ...
```

如果完全没找到任何 vcvars：
- **方法 1**：装 Visual Studio Installer（vswhere.exe 在 `%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\`）
- **方法 2**：把 `vcvars64.bat` 路径加到环境变量 `VCEXPRESS_VCVARS` 或自定义 `do_build.ps1` 探测
- **方法 3**：开 Developer Command Prompt for VS 2022（PowerShell 已加载 vcvars），直接跑 cmake

### 9.3 vcpkg 包未找到

**现象**：
```
Could NOT find SDL2 (missing: ... )
```

**修复**：
```powershell
# 选项 1：设 VCPKG_ROOT
$env:VCPKG_ROOT = "C:\vcpkg"

# 选项 2：用本地 vcpkg_installed
# （vcpkg_installed\x64-windows 必须在源码根目录）

# 选项 3：装缺失的包
C:\vcpkg\vcpkg install sdl2:x64-windows
```

### 9.4 Qt6LinguistTools 找不到

**现象**：
```
Could NOT find Qt6LinguistTools
```

**修复**：
```powershell
cmake -S . -B build -DFCEUX11_ENABLE_I18N=OFF ...
```

（如果不需要 i18n 翻译，可在 cmake 配置时关掉）

### 9.5 Rust crate 编译失败

**现象**：
```
error: linker `link.exe` not found
```

**修复**：
- 跑 `do_build.ps1`（自动加载 MSVC link.exe）
- 手动：跑 `rustup default stable-x86_64-pc-windows-msvc`

### 9.6 编译中途崩溃（OOM）

**现象**：
```
fatal error C1060: compiler is out of heap space
```

**修复**：
- 增加虚拟内存（控制面板 → 系统 → 高级 → 性能设置 → 高级 → 虚拟内存）
- 关闭其他吃内存的应用（Chrome、VS 等）
- 用 Ninja 而非 MSBuild：Ninja 的并行调度更省内存

### 9.7 中文字符乱码（源码里有 BOM / 无 BOM 混用）

**现象**：
```
warning C4828: The file contains a character that is illegal in the current source character set
```

**修复**：
- 删 `build/CMakeFiles/CMakeCache.txt` 后重跑
- 用 VS Code 打开文件 → 右下角编码 → "Save with Encoding" → "UTF-8 with BOM"
- 验证：`git ls-files --eol src/*.cpp` 应全部 `i/lf` 或 `i/crlf`

---

## 10. 高级选项

### 10.1 ASan 构建（仅 Debug / RelWithDebInfo）

```powershell
.\scripts\_build_asan.ps1
# 内部调：cmake -S . -B build-asan -DFCEUX11_ASAN=ON -DCMAKE_BUILD_TYPE=Debug
.\scripts\_ctest_asan.ps1
```

### 10.2 UBSan 构建

```powershell
.\scripts\_build_ubsan.ps1
# 内部调：cmake -S . -B build-ubsan -DFCEUX11_UBSAN=ON -DCMAKE_BUILD_TYPE=Debug
```

> **MSVC 限制**：MSVC 的 UBSan 是 /RTC1 + /sdl + /GS + /guard:cf 的子集，
> **不是** clang 的完整 UBSan。完整 UBSan 是 v2.0 议程。

### 10.3 Rust=OFF（C++ 纯模式）

```powershell
cmake -S . -B build-cpp -G Ninja -DFCEUX11_RUST_ENABLED=OFF
cmake --build build-cpp
```

> **保留原因**：部分用户环境（容器 / 裁剪镜像）可能无 Rust 工具链，
> 此路径验证"C++ 兼容 shim"在无 Rust 时仍能完整工作。

### 10.4 WGI 后端（opt-in，Win10 1809+）

```powershell
cmake -S . -B build-wgi -G Ninja -DFCEUX11_WGI_BACKEND=ON
cmake --build build-wgi
```

> **WGI**：Windows.Gaming.Input API。比 XInput 1.4 更新，
> **但**对 DualSense 高级特性（自适应扳机 / 触觉反馈）支持不完整，
> XInput 仍是事实标准。WGI 默认 OFF。

### 10.5 工具链锁定（铁律 9）

`CMakeLists.txt:28-34` 强制 MSVC-only：

```cmake
if(NOT MSVC)
    message(FATAL_ERROR
        "FCEUX11 v0.2.1+ requires the MSVC 2022+ toolchain ..."
    )
endif()
```

**不允许**的替代：clang-cl / gcc / MinGW / MSYS2 / 任何 LLVM 工具链。
理由：ABI 一致性 + byte-level savestate 兼容。

---

## 11. 版本与升级

| 项 | v1.13 状态 |
|----|-----------|
| 主版本 | **1.13**（代号 **Purify**，v1.x 现代化周期第十三子版本）|
| 工具链 | MSVC 19.36+ / Qt 6.8 LTS / vcpkg 2024+ baseline / Rust 1.78+ |
| API 兼容 | 与 v0.3.x / v1.x 全部子版本完全兼容（兼容 shim 保留到 v2.0）|
| savestate 兼容 | V2 格式（FCEU11ST）为默认，V1 只读兼容；与 v0.2.x / v0.3.x / v1.0~v1.10 全部兼容 |
| INI 兼容 | 与 v0.2.x / v0.3.x / v1.x 全部子版本完全兼容 |
| Rust crate 版本 | 0.2.x 不变（与产品版本解耦）|
| UI 语言 | **12 种**：en / zh_CN / zh_TW / ja / ko / es / fr / de / vi / th / hi(beta) / ar(beta, RTL)；首启按 `QLocale::system()` 自动匹配 |
| 下一里程碑 | v1.13 Purify → …（v1.x §14~§15 Roadmap）→ v2.0 |

### 11.1 升级路径

**从 v1.x（v1.0~v1.11）升级到 v1.13 Purify**：
- 替换 `fceux11.exe` 即可，savestate / INI / 配置完全兼容
- 无需重新配置控制器 / 快捷键
- v1.12/v1.13 的内部重构（文件拆分 + 内存管理现代化 + Lua 引擎统一）对模拟行为零影响
- 已保存的语言偏好 `savedLang` 配置键自动迁移；新用户首启会按 Windows
  显示语言自动匹配

**从 v0.3.16 LTS 升级到 v1.13**：
- 替换 `fceux11.exe` 即可，savestate / INI / 配置完全兼容
- v0.3.16 → v1.0 → v1.13 期间的所有兼容 shim 仍保留（v2.0 删除）

**从 v0.2.x 升级到 v1.13**：
- savestate 兼容（v0.2.30+ 起）；API 变化，需重新配置控制器
- 详见 [CHANGELOG.md](../../CHANGELOG.md) 兼容性段落

### 11.2 降级路径（v1.13 → 任意早期版本）

v1.13 沿用 v1.10 的 V2 savestate 格式（FCEU11ST）为默认输出，但所有
历史版本（v0.2.x / v0.3.x / v1.0~v1.11）的 savestate 均可直接加载。
用早期版本的 `fceux11.exe` 打开 v1.13 保存的 V2 savestate 前需先
通过 v1.13 转换为 V1 格式（`--save-v1` 选项）。

> **多语言降级注意事项**：v1.11+ 写入 INI 的 `savedLang` 字段若为新增
> 9 种之一（ja / ko / es / fr / de / vi / th / hi / ar），降级到 v1.10
> 或更早版本后会被识别为未知 locale，自动回退到 `en`；其它配置不受影响。

---

## 12. 工具链策略不变量

**11 条铁律**（构建系统必须遵守）：

1. **可字节级回退**：每个子版本 commit 后能 `git checkout` 切回
2. **CI 必过**：`ctest` + `cargo test` + nestest.nes 跑到 `$C66E`
3. **API 冻结与 RFC**：API 增减必须走 RFC
4. **子版本独立性**：每个子版本可独立 cherry-pick / revert
5. **审查门控**：每个 PR 必须有审查报告
6. **文档同步**：`docs/` + `docs/internal/` 增量更新 + [CHANGELOG.md](../../CHANGELOG.md) Keep a Changelog
7. **不静默改 API**：include 顺序、宏定义、可见符号集合变化必须在 commit message 标注
8. **工具链锁定**：MSVC 19.36+ / Qt 6.8 LTS 在 v2.0 之前不升级
9. **MSVC-only**：`CMakeLists.txt:28-34` 强制 MSVC，cl 拒绝 clang / gcc / MinGW / MSYS2
10. **main 分支 only**：永远只保留 main，不创建 topic / release 分支
（v1.x 周期的开发工作通过 PR 合入，详见 `docs/v1.x_Modernization_Roadmap.md`）
11. **/WX 激活**：警告即错误（v0.3.16 LTS 起），新代码必须编译 clean

---

## 13. 获取帮助

- **Issue 反馈**：https://github.com/Laffinty/FCEUX11/issues
- **变更日志**：[`CHANGELOG.md`](../../CHANGELOG.md)
- **v1.x 现代化周期路线图**：[`docs/v1.x_Modernization_Roadmap.md`](../v1.x_Modernization_Roadmap.md)
- **全局状态审计**（v1.2 Census 产物）：[`docs/internal/global_state_audit.md`](../internal/global_state_audit.md)
- **存档布局审计**（v1.3 Legion 产物）：[`docs/internal/savestate_layout_audit.md`](../internal/savestate_layout_audit.md)
- **v1.4 调用点审计**（v1.4 Gateway 产物）：[`docs/internal/v1.4_call_site_audit.md`](../internal/v1.4_call_site_audit.md)
- **性能基准 baseline**：`tests/benchmarks/baseline_v1.0.json`

> **说明**：v0.3.x 时期的 `docs/tech/*.txt` 7 篇（构建系统 / 5 道闸 /
> 24h 烟雾 / Win11 集成 / 性能方法 / v0.3.x 发布说明 / i18n 管线）已
> 随 v0.3.16 LTS 收官统一归档到 `CHANGELOG.md` 与本指南对应章节中，
> 不再单独维护独立 `.txt`。

---

**文档结束** — FCEUX11 v1.13 正式版编译指南。生效版本：v1.13 Purify（2026-07-10）。
