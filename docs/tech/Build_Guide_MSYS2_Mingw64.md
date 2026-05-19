# FCEUX11 MSYS2 / MinGW-w64 编译指南

> **文档版本**: 2.0
> **更新日期**: 2026-05-19
> **适用范围**: 当前代码基线（Qt6 + MinGW-w64 构建）
> **目标读者**: 需要在 Windows 上从源码编译 FCEUX11 的开发者 / Agent

---

## 1. 环境要求

| 组件 | 版本 / 路径 | 说明 |
|------|------------|------|
| MSYS2 | 最新版 | 提供 MinGW-w64 工具链与 POSIX 环境 |
| MinGW-w64 | GCC 16.1.0+ | 位于 `D:/msys64/mingw64`（默认安装路径） |
| CMake | 3.28+ | MSYS2 包 `mingw-w64-x86_64-cmake` |
| Qt6 | 6.x | MSYS2 包 `mingw-w64-x86_64-qt6` |
| Rust | stable | 需通过 rustup 安装，`cargo` 必须在 PATH 中；需安装 target `x86_64-pc-windows-gnu` |
| Make | MSYS Makefiles | 使用 MSYS2 自带的 `/usr/bin/make` |
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

# 安装 MinGW GNU target
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

## 2. 已知陷阱

### 2.1 `build.bat` 无法在当前终端捕获输出

项目根目录的 `build.bat` 内容如下：

```batch
D:\msys64\msys2_shell.cmd -mingw64 -c "cd /c/Users/ikrx2/Desktop/project/FCEUX11 && rm -rf build && mkdir -p build && cd build && cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release 2>&1 && make 2>&1"
```

**问题**：直接双击或在 PowerShell 中调用 `build.bat` 时，命令会在独立的 MSYS2 窗口中执行，**当前终端不会收到任何 stdout/stderr**，导致无法判断编译是否成功或失败。

### 2.2 Rust 工具链在 MSYS2 Shell 中不可见

MSYS2 的 `msys2_shell.cmd` 默认会修剪 PATH，仅保留 MSYS2 内部路径。这会导致 CMake 配置阶段报错：

```
CMake Error at src/rust/CMakeLists.txt:19 (message):
  rustup not found.  Install Rust: https://rustup.rs/
```

**解决**：

- **方案 A**（推荐）：启动 MSYS2 Shell 时附加 `-full-path` 参数，使其继承 Windows 的完整 PATH（包含 `C:\Users\<user>\.cargo\bin`）。
- **方案 B**：在 MSYS2 Shell 内手动添加 PATH：
  ```bash
  export PATH="/c/Users/<user>/.cargo/bin:$PATH"
  ```

### 2.3 Rust target `x86_64-pc-windows-gnu` 未安装

若未安装正确的 Rust target，CMake 会报错：

```
CMake Error at src/rust/CMakeLists.txt:31 (message):
  Rust target 'x86_64-pc-windows-gnu' is not installed.
  Please run: rustup target add x86_64-pc-windows-gnu
```

**解决**：运行 `rustup target add x86_64-pc-windows-gnu`。

### 2.4 不能直接使用 `mingw32-make`

CMake 配置使用的是 `-G "MSYS Makefiles"`，生成的 Makefile 依赖 MSYS 特定的路径转换与 `/usr/bin/make`。如果在 PowerShell 中直接调用 `D:\msys64\mingw64\bin\mingw32-make.exe`，会报错：

```
mingw32-make: *** No targets specified and no makefile found. Stop.
```

**解决**：必须在 MSYS2 Shell 内部执行 `make`（即 `/usr/bin/make`）。

### 2.5 并行构建偶发 `file truncated` 错误

在使用 `make -j$(nproc)` 并行构建时，`ar.exe` 有概率在读取 `.obj` 文件时遇到竞争条件，导致：

```
ar.exe: libfceux11_drivers_qt.a: error reading CMakeFiles/fceux11_drivers_qt.dir/.../TasEditorWindow.cpp.obj: file truncated
```

**原因**：多个编译进程同时写入 `.obj` 文件时，`ar` 可能在文件尚未完全写入时尝试读取。

**解决**：

- **方案 A**（推荐）：重新运行 `make -j$(nproc)`，Make 会检测到损坏的文件并重新编译。通常 1 次重试即可成功。
- **方案 B**：删除 `build/src/CMakeFiles/fceux11_drivers_qt.dir/` 目录后重试。
- **方案 C**：使用 `make -j1` 单线程构建（较慢，但不会出现此问题）。

项目提供的 `build.sh` 已内置自动重试机制（最多 2 次），可直接使用。

---

## 3. 正确编译步骤

### 3.0 调用方式说明

编译需要在 MSYS2 MinGW64 环境中执行。有以下几种调用方式：

| 方式 | 命令 | 优点 | 缺点 |
|------|------|------|------|
| **PowerShell + msys2_shell.cmd** | `D:\msys64\msys2_shell.cmd -mingw64 -no-start -full-path -c "..."` | 可在 PowerShell 中获取退出码 | 命令较长 |
| **MSYS2 bash -l** | `/d/msys64/usr/bin/bash.exe -l -c "export PATH=... && ..."` | 脚本友好，适合 CI | 需手动设置 PATH |
| **MSYS2 终端** | 直接打开 `D:\msys64\msys2.exe` | 交互方便 | 需手动切换到 MinGW64 环境 |
| **build.sh** | 在 MSYS2 终端中运行 `bash build.sh` | 自动重试，错误处理 | 需先进入 MSYS2 环境 |

### 3.1 一键完整构建（推荐）

#### 方式一：使用 build.sh（带自动重试）

在 MSYS2 MinGW64 终端中：

```bash
cd /c/Users/ikrx2/Desktop/project/FCEUX11
bash build.sh
```

#### 方式二：PowerShell 调用

在 PowerShell 中执行以下命令：

```powershell
# 1. 清理旧构建目录
Remove-Item -Recurse -Force build

# 2. 在 MSYS2 Shell 中执行 CMake 配置（带完整 PATH）
D:\msys64\msys2_shell.cmd -mingw64 -no-start -full-path -c `
    "cd /c/Users/ikrx2/Desktop/project/FCEUX11 && `
     rm -rf build && mkdir -p build && cd build && `
     cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release"

# 3. 在 MSYS2 Shell 中执行并行编译
D:\msys64\msys2_shell.cmd -mingw64 -no-start -full-path -c `
    "cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && `
     make -j$(nproc)"
```

#### 方式三：直接使用 MSYS2 bash（适合脚本/CI）

```bash
/d/msys64/usr/bin/bash.exe -l -c `
    "export PATH='/c/Users/ikrx2/.cargo/bin:$PATH' && `
     cd /c/Users/ikrx2/Desktop/project/FCEUX11 && `
     rm -rf build && mkdir -p build && cd build && `
     cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release && `
     make -j$(nproc)"
```

> **注意**：`-no-start` 确保 `msys2_shell.cmd` 在当前控制台中等待命令完成并返回退出码；`-full-path` 解决 Rust 不可见问题。如果 MSYS2 安装路径非默认 `D:\msys64`，请相应调整。

### 3.2 仅重新编译（增量构建）

若 `build` 目录已存在且 CMake 配置完好，直接执行：

```powershell
D:\msys64\msys2_shell.cmd -mingw64 -no-start -full-path -c `
    "cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && make -j$(nproc)"
```

### 3.3 检查构建结果

```powershell
# 确认可执行文件已生成
Test-Path build/src/fceux11.exe
```

成功构建后，输出文件为：

```
build/src/fceux11.exe
```

### 3.4 运行测试（可选）

项目启用了 `FCEUX11_BUILD_TESTS` 选项（默认 ON），构建完成后可在 MSYS2 终端中运行：

```bash
cd /c/Users/ikrx2/Desktop/project/FCEUX11/build
ctest --output-on-failure
```

---

## 4. 编译警告说明

### 4.1 `-Wstringop-overflow` 警告（GCC 16.x 误报）

**现象**：编译 `emufile.h` 相关代码时，GCC 16.1.0 会产生大量类似警告：

```
warning: 'void* __builtin_memset(...)' writing 1 or more bytes into a region of size 0
    overflows the destination [-Wstringop-overflow=]
```

**来源**：`src/emufile.h:131` 的 `EMUFILE_MEMORY::reserve()` 调用 `std::vector<uint8_t>::resize()` 时，GCC 内联优化器在分析 `std::fill_n` + `memset` 时产生了误报。

**影响**：纯编译器误报，不影响运行时正确性。这是 GCC 在高优化级别下对 `vector::resize` 的静态分析已知缺陷。

**处理**：可安全忽略。如需消除，可在 `src/CMakeLists.txt` 中对 MinGW 添加：

```cmake
if(MINGW)
    add_compile_options(-Wno-stringop-overflow)
endif()
```

### 4.2 `-Wwrite-strings` 警告（已修复）

**历史现象**：`coolgirl.cpp` 中的 `SET_BITS`/`get_bits`/`set_bits` 宏使用 `char*` 参数接收字符串字面量（`const char*`），违反 C++ 标准。

**修复**：v2.0 已将 `string_to_bits`、`get_bits`、`set_bits` 的参数类型从 `char*` 改为 `const char*`，此警告不再出现。

---

## 5. 常见问题排查

| 现象 | 可能原因 | 解决方案 |
|------|---------|---------|
| `rustup not found` | MSYS2 未继承 Windows PATH | 添加 `-full-path` 参数，或手动 `export PATH="/c/Users/<user>/.cargo/bin:$PATH"` |
| `Rust target 'x86_64-pc-windows-gnu' is not installed` | 未安装 MinGW Rust target | 运行 `rustup target add x86_64-pc-windows-gnu` |
| `No makefile found` | 使用了 `mingw32-make` 而非 MSYS `make` | 在 MSYS2 Shell 内执行 `make` |
| `file truncated`（静态库） | 并行编译竞争条件，`.obj` 文件损坏 | 重新运行 `make`（通常 1 次即可），或使用 `build.sh`（已内置重试） |
| 编译输出为空 | `build.bat` 在新窗口执行 | 改用本文 3.1 节的方式 |
| CMake 找不到 Qt6 | 未安装 Qt6 包 | 运行 `pacman -S mingw-w64-x86_64-qt6` |
| 链接时找不到 `-larchive` | 未安装 libarchive | 运行 `pacman -S mingw-w64-x86_64-libarchive` |
| 链接时找不到 `-lSDL2` | 未安装 SDL2 | 运行 `pacman -S mingw-w64-x86_64-SDL2` |

---

## 6. 附录

### 附录 A：关键参数说明

| 参数 | 作用 |
|------|------|
| `-mingw64` | 启动 MinGW-w64 环境（而非 MSYS2 原生环境） |
| `-no-start` | 不使用 `start` 命令弹出新窗口，而是阻塞等待并返回退出码 |
| `-full-path` | 继承 Windows 的完整 PATH（使 rustup、vcpkg 等外部工具可见） |
| `-c "..."` | 作为登录 shell 的命令参数直接执行 |

### 附录 B：MSYS2 路径映射速查

| Windows 路径 | MSYS2 路径 |
|-------------|-----------|
| `C:\Users\ikrx2\Desktop\project\FCEUX11` | `/c/Users/ikrx2/Desktop/project/FCEUX11` |
| `D:\msys64\mingw64\bin\cmake.exe` | `/d/msys64/mingw64/bin/cmake.exe` |
| `C:\Users\ikrx2\.cargo\bin\cargo.exe` | `/c/Users/ikrx2/.cargo/bin/cargo.exe` |

### 附录 C：参考文件

- `build.sh` — 推荐的构建脚本（带自动重试和错误处理）
- `build.bat` — 旧版构建脚本（存在输出不可见问题，不推荐）
- `CMakeLists.txt` — 根 CMake 配置
- `src/rust/CMakeLists.txt` — Rust 子项目配置（触发 rustup 检测和 target 验证）

### 附录 D：变更记录

| 版本 | 日期 | 变更 |
|------|------|------|
| 2.0 | 2026-05-19 | 新增 MSYS2 依赖安装命令；新增 Rust target 安装说明；修复 `coolgirl.cpp` 的 `-Wwrite-strings` 警告；新增 `file truncated` 并行构建陷阱及重试方案；新增 `-Wstringop-overflow` 警告说明；新增 `build.sh` 自动重试脚本；新增多种调用方式；完善常见问题排查表 |
| 1.0 | 2026-05-19 | 初始版本 |
