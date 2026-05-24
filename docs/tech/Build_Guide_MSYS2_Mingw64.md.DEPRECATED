# FCEUX11 MSYS2 / MinGW-w64 编译指南

> **文档版本**: 3.0
> **更新日期**: 2026-05-19
> **适用范围**: 当前代码基线（Qt6 + MinGW-w64 构建）
> **目标读者**: 需要在 Windows 上从源码编译 FCEUX11 的开发者 / Agent

---

## 0. Agent 一键编译（TL;DR）

如果你是 Agent（opencode / Claude / Codex 等），在 Windows 终端中直接执行以下命令即可完成完整编译：

```bash
/d/msys64/usr/bin/bash.exe -l -c "export PATH='/c/Users/ikrx2/.cargo/bin:$PATH' && cd /c/Users/ikrx2/Desktop/project/FCEUX11 && rm -rf build && mkdir -p build && cd build && cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release 2>&1 && make -j4 2>&1"
```

验证编译结果：

```bash
ls -la /c/Users/ikrx2/Desktop/project/FCEUX11/build/src/fceux11.exe
```

运行测试：

```bash
/d/msys64/usr/bin/bash.exe -l -c "cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && ctest --output-on-failure 2>&1"
```

> **如果以上命令成功生成 `fceux11.exe` 且 ctest 全部通过，编译验证即完成。**

---

## 1. 环境要求

| 组件 | 版本 / 路径 | 说明 |
|------|------------|------|
| MSYS2 | 最新版 | 提供 MinGW-w64 工具链与 POSIX 环境，安装在 `D:/msys64` |
| MinGW-w64 | GCC 16.1.0+ | 位于 `D:/msys64/mingw64` |
| CMake | 3.28+ | MSYS2 包 `mingw-w64-x86_64-cmake` |
| Qt6 | 6.x | MSYS2 包 `mingw-w64-x86_64-qt6` |
| Rust | stable | 需通过 rustup 安装，`cargo` 必须在 PATH 中；需安装 target `x86_64-pc-windows-gnu` |
| Make | MSYS Makefiles | 使用 MSYS2 自带的 `/usr/bin/make`（**不是** `mingw32-make`） |
| libarchive | - | MSYS2 包 `mingw-w64-x86_64-libarchive` |
| SDL2 | - | MSYS2 包 `mingw-w64-x86_64-SDL2` |
| zlib | - | MSYS2 包 `mingw-w64-x86_64-zlib`（通常作为依赖自动安装） |

### 1.1 MSYS2 依赖安装

在 MSYS2 MinGW64 终端中一次性安装所有必需包：

```bash
pacman -S --needed mingw-w64-x86_64-gcc \
    mingw-w64-x86_64-cmake \
    mingw-w64-x86_64-qt6 \
    mingw-w64-x86_64-SDL2 \
    mingw-w64-x86_64-libarchive \
    mingw-w64-x86_64-zlib \
    make
```

### 1.2 Rust 环境准备

在 PowerShell / CMD 中（非 MSYS2 终端）：

```powershell
# 安装 rustup（如未安装）
winget install Rustlang.Rustup

# 安装 MinGW GNU target（必须！）
rustup target add x86_64-pc-windows-gnu

# 验证
rustup target list --installed
# 应包含 x86_64-pc-windows-gnu
```

### 1.3 快速检查命令

在 PowerShell 中执行：

```powershell
# 检查 MSYS2 是否存在
Test-Path D:\msys64\msys2_shell.cmd

# 检查 CMake
D:\msys64\mingw64\bin\cmake.exe --version

# 检查 Rust
where.exe rustup
where.exe cargo
rustup target list --installed
```

---

## 2. Agent 编译核心要点

> **本节是 Agent 编译失败的最常见原因汇总。如果编译失败，请逐条检查。**

### 2.1 必须通过 MSYS2 bash 执行编译

编译**不能**在 Windows CMD / PowerShell / Git Bash 中直接运行 `cmake` 或 `make`。必须通过 MSYS2 的 bash 来执行。

**正确调用方式**（从 Windows 终端发起）：

```bash
/d/msys64/usr/bin/bash.exe -l -c "你的编译命令"
```

**错误调用方式**：

| 错误写法 | 原因 |
|----------|------|
| `D:\msys64\usr\bin\bash.exe -l -c "..."` | Windows 反斜杠路径在 bash 中会被解析错误，必须用正斜杠 `/d/msys64/...` |
| `cmake .. -G "MinGW Makefiles"` | 本项目使用 `-G "MSYS Makefiles"`，不是 MinGW Makefiles |
| `mingw32-make` | 必须使用 MSYS2 的 `/usr/bin/make`，不是 `mingw32-make` |
| `make`（在 Git Bash 中） | Git Bash 的 `make` 不是 MSYS2 的 make，会找不到正确的编译器 |

### 2.2 必须在 MSYS2 bash 中导出 Rust PATH

MSYS2 bash 默认不继承 Windows 的 PATH，导致 `rustup` / `cargo` 不可见。**每次调用 MSYS2 bash 时都必须手动 export**：

```bash
export PATH='/c/Users/ikrx2/.cargo/bin:$PATH'
```

如果不做这一步，CMake 配置阶段会报错：

```
CMake Error at src/rust/CMakeLists.txt:19 (message):
  rustup not found.  Install Rust: https://rustup.rs/
```

### 2.3 增量编译的前提条件

增量编译（仅运行 `make`，不重新 `cmake`）**仅在 `build/` 目录已有完整 CMake 配置时可用**。如果：

- `build/` 目录不存在
- `build/` 目录中只有 `CMakeCache.txt` 和 `CMakeFiles/`（不完整配置）
- 修改了 `CMakeLists.txt` 或新增/删除了源文件

则**必须先执行完整构建流程**（清理 build 目录 + cmake configure + make）。

安全的增量编译方式：

```bash
/d/msys64/usr/bin/bash.exe -l -c "export PATH='/c/Users/ikrx2/.cargo/bin:$PATH' && cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && make -j4 2>&1"
```

如果增量编译报错 `No makefile found` 或其他配置错误，请改用完整构建（见第 3 节）。

### 2.4 编译警告 ≠ 编译失败

编译过程中会产生大量 `-Wstringop-overflow` 警告（见第 5 节），这些是 **GCC 16.x 的已知误报**，不影响编译结果。判断编译是否成功的唯一标准是：

1. `make` 命令退出码为 0
2. `build/src/fceux11.exe` 文件存在

**不要因为有大量 warning 输出就判定编译失败。**

---

## 3. 完整编译步骤

### 3.1 方式一：Agent 推荐方式（Windows 终端直接调用 MSYS2 bash）

从 Windows 终端（opencode Bash / PowerShell / CMD）执行：

```bash
/d/msys64/usr/bin/bash.exe -l -c "export PATH='/c/Users/ikrx2/.cargo/bin:$PATH' && cd /c/Users/ikrx2/Desktop/project/FCEUX11 && rm -rf build && mkdir -p build && cd build && cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release 2>&1 && make -j4 2>&1"
```

此命令一次性完成：清理 → cmake 配置 → 编译。`2>&1` 确保所有输出都可在当前终端看到。

### 3.2 方式二：使用 do_build.bat

```batch
do_build.bat
```

`do_build.bat` 内部调用 MSYS2 bash 执行完整构建，输出重定向到当前终端。

### 3.3 方式三：使用 build.sh（在 MSYS2 终端中）

在 MSYS2 MinGW64 终端中：

```bash
cd /c/Users/ikrx2/Desktop/project/FCEUX11
export PATH="/c/Users/ikrx2/.cargo/bin:$PATH"
bash build.sh
```

### 3.4 增量编译

仅当 `build/` 目录已存在完整 CMake 配置时：

```bash
/d/msys64/usr/bin/bash.exe -l -c "export PATH='/c/Users/ikrx2/.cargo/bin:$PATH' && cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && make -j4 2>&1"
```

> **注意**：增量编译失败时，请回退到完整编译（3.1 方式）。

### 3.5 检查构建结果

```bash
ls -la /c/Users/ikrx2/Desktop/project/FCEUX11/build/src/fceux11.exe
```

或在 PowerShell 中：

```powershell
Test-Path build/src/fceux11.exe
```

### 3.6 运行测试

```bash
/d/msys64/usr/bin/bash.exe -l -c "cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && ctest --output-on-failure 2>&1"
```

预期输出：

```
1/3 Test #1: smoke_test .......................   Passed
2/3 Test #2: mapper_load_test .................   Passed
3/3 Test #3: mapper_reset_test .................   Passed

100% tests passed, 0 tests failed out of 3
```

---

## 4. 已知陷阱与排错

### 4.1 Rust 工具链在 MSYS2 Shell 中不可见

**现象**：

```
CMake Error at src/rust/CMakeLists.txt:19 (message):
  rustup not found.  Install Rust: https://rustup.rs/
```

**原因**：MSYS2 bash 默认不继承 Windows PATH，`cargo` / `rustup` 不可见。

**解决**：在 MSYS2 bash 命令开头添加 `export PATH='/c/Users/ikrx2/.cargo/bin:$PATH'`。

### 4.2 Rust target `x86_64-pc-windows-gnu` 未安装

**现象**：

```
CMake Error at src/rust/CMakeLists.txt:31 (message):
  Rust target 'x86_64-pc-windows-gnu' is not installed.
```

**解决**：在 PowerShell 中运行 `rustup target add x86_64-pc-windows-gnu`。

### 4.3 `mingw32-make` 报错 No makefile found

**现象**：

```
mingw32-make: *** No targets specified and no makefile found. Stop.
```

**原因**：本项目使用 `-G "MSYS Makefiles"` 生成的 Makefile，必须在 MSYS2 bash 中用 `/usr/bin/make` 执行。

**解决**：不要使用 `mingw32-make`，改用 MSYS2 bash 内的 `make`。

### 4.4 并行构建偶发 `file truncated` 错误

**现象**：

```
ar.exe: libfceux11_drivers_qt.a: error reading CMakeFiles/fceux11_drivers_qt.dir/.../TasEditorWindow.cpp.obj: file truncated
```

**原因**：`make -j4` 并行编译时 `ar.exe` 读取 `.obj` 文件的竞争条件。

**解决**：重新运行相同的 `make -j4` 命令，通常 1 次重试即可成功。Make 会自动检测损坏的 `.obj` 并重新编译。

### 4.5 MSYS2 bash 路径必须用正斜杠

**现象**：从 Windows 终端调用 `D:\msys64\usr\bin\bash.exe` 时报 `command not found` 或路径解析错误。

**原因**：在 bash / opencode 终端中，反斜杠 `\` 会被当作转义符处理。

**解决**：始终使用 MSYS2 风格正斜杠路径：

| Windows 路径 | MSYS2 / bash 路径 |
|-------------|-------------------|
| `D:\msys64\usr\bin\bash.exe` | `/d/msys64/usr/bin/bash.exe` |
| `C:\Users\ikrx2\.cargo\bin\cargo.exe` | `/c/Users/ikrx2/.cargo/bin/cargo.exe` |
| `C:\Users\ikrx2\Desktop\project\FCEUX11` | `/c/Users/ikrx2/Desktop/project/FCEUX11` |

### 4.6 `msys2_shell.cmd` 输出丢失

直接使用 `msys2_shell.cmd -mingw64 -no-start -full-path -c "..."` 时，部分环境下输出可能丢失（"No output"）。

**解决**：改用 `/d/msys64/usr/bin/bash.exe -l -c "..."` 直接调用 bash。

---

## 5. 编译警告说明

### 5.1 `-Wstringop-overflow` 警告（GCC 16.x 误报，可安全忽略）

**现象**：编译时产生大量类似警告：

```
warning: 'void* __builtin_memset(...)' writing 1 or more bytes into a region of size 0
    overflows the destination [-Wstringop-overflow=]
```

**来源**：`src/emufile.h:131` 的 `EMUFILE_MEMORY::reserve()` 调用 `std::vector<uint8_t>::resize()` 时，GCC 内联优化器产生的误报。

**影响**：纯编译器误报，不影响运行时正确性。

**处理**：可安全忽略。如需消除，可在 `src/CMakeLists.txt` 中添加：

```cmake
if(MINGW)
    add_compile_options(-Wno-stringop-overflow)
endif()
```

---

## 6. 常见问题排查

| 现象 | 可能原因 | 解决方案 |
|------|---------|---------|
| `rustup not found` | MSYS2 未继承 Windows PATH | 在 MSYS2 bash 中 `export PATH='/c/Users/ikrx2/.cargo/bin:$PATH'` |
| `Rust target 'x86_64-pc-windows-gnu' is not installed` | 未安装 MinGW Rust target | `rustup target add x86_64-pc-windows-gnu` |
| `No makefile found` | 使用了 `mingw32-make` 或不在 MSYS2 环境中 | 在 MSYS2 bash 中用 `make` |
| `file truncated`（静态库） | 并行编译竞争条件 | 重新运行 `make -j4`（通常 1 次即可） |
| 编译输出为空 | 直接调用 `msys2_shell.cmd` | 改用 `/d/msys64/usr/bin/bash.exe -l -c "..."` |
| CMake 找不到 Qt6 | 未安装 Qt6 包 | `pacman -S mingw-w64-x86_64-qt6` |
| 链接时找不到 `-larchive` | 未安装 libarchive | `pacman -S mingw-w64-x86_64-libarchive` |
| 链接时找不到 `-lSDL2` | 未安装 SDL2 | `pacman -S mingw-w64-x86_64-SDL2` |
| `bash.exe` not found | 使用了反斜杠路径 | 改用正斜杠 `/d/msys64/usr/bin/bash.exe` |
| 增量编译失败 | build 目录配置不完整 | 执行完整构建（先 `rm -rf build`） |
| LSP 报错 Qt 头文件 not found | IDE 无 MSYS2 Qt 路径 | 忽略，不影响实际编译 |

---

## 7. 附录

### 附录 A：MSYS2 路径映射速查

| Windows 路径 | MSYS2 路径 |
|-------------|-----------|
| `C:\Users\ikrx2\Desktop\project\FCEUX11` | `/c/Users/ikrx2/Desktop/project/FCEUX11` |
| `D:\msys64\mingw64\bin\cmake.exe` | `/d/msys64/mingw64/bin/cmake.exe` |
| `D:\msys64\usr\bin\bash.exe` | `/d/msys64/usr/bin/bash.exe` |
| `C:\Users\ikrx2\.cargo\bin\cargo.exe` | `/c/Users/ikrx2/.cargo/bin/cargo.exe` |

### 附录 B：参考文件

- `build.sh` — Bash 构建脚本（在 MSYS2 终端中运行）
- `do_build.bat` — Windows 构建脚本（从 CMD/PowerShell 运行）
- `CMakeLists.txt` — 根 CMake 配置
- `src/rust/CMakeLists.txt` — Rust 子项目配置

### 附录 C：变更记录

| 版本 | 日期 | 变更 |
|------|------|------|
| 3.0 | 2026-05-19 | 重构文档：新增 Agent 一键编译（第 0 节）；新增 Agent 编译核心要点（第 2 节），涵盖 MSYS2 bash 调用方式、Rust PATH、增量编译前提、警告≠失败等关键问题；明确正斜杠路径要求；精简冗余内容 |
| 2.1 | 2026-05-19 | 修订构建方式说明，移除在独立窗口执行的方式；新增 `do_build.bat` 作为 Windows 推荐脚本 |
| 2.0 | 2026-05-19 | 新增 MSYS2 依赖安装命令；新增 Rust target 安装说明；新增 `file truncated` 重试方案；新增 `-Wstringop-overflow` 警告说明 |
| 1.0 | 2026-05-19 | 初始版本 |
