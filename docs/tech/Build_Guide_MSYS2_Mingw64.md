# FCEUX11 MSYS2 / MinGW-w64 编译指南

> **文档版本**: 1.0
> **创建日期**: 2026-05-19
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
| Rust | stable | 需通过 rustup 安装，`cargo` 必须在 PATH 中 |
| Make | MSYS Makefiles | 使用 MSYS2 自带的 `/usr/bin/make` |

### 1.1 快速检查命令

在 PowerShell 中执行：

```powershell
# 检查 MSYS2 是否存在
Test-Path D:\msys64\msys2_shell.cmd

# 检查 CMake
D:\msys64\mingw64\bin\cmake.exe --version

# 检查 Rust
where.exe rustup
where.exe cargo
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

**解决**：启动 MSYS2 Shell 时附加 `-full-path` 参数，使其继承 Windows 的完整 PATH（包含 `C:\Users\<user>\.cargo\bin`）。

### 2.3 不能直接使用 `mingw32-make`

CMake 配置使用的是 `-G "MSYS Makefiles"`，生成的 Makefile 依赖 MSYS 特定的路径转换与 `/usr/bin/make`。如果在 PowerShell 中直接调用 `D:\msys64\mingw64\bin\mingw32-make.exe`，会报错：

```
mingw32-make: *** No targets specified and no makefile found. Stop.
```

**解决**：必须在 MSYS2 Shell 内部执行 `make`（即 `/usr/bin/make`）。

---

## 3. 正确编译步骤

### 3.1 一键完整构建（推荐）

在 PowerShell 中执行以下命令，输出会实时重定向到 `build_log.txt`：

```powershell
# 1. 清理旧构建目录
Remove-Item -Recurse -Force build

# 2. 在 MSYS2 Shell 中执行 CMake 配置（带完整 PATH）
D:\msys64\msys2_shell.cmd -mingw64 -no-start -full-path -c `
    "cd /c/Users/ikrx2/Desktop/project/FCEUX11 && `
     rm -rf build && mkdir -p build && cd build && `
     cmake .. -G 'MSYS Makefiles' -DCMAKE_BUILD_TYPE=Release"

# 3. 在 MSYS2 Shell 中执行并行编译（输出追加到日志）
D:\msys64\msys2_shell.cmd -mingw64 -no-start -full-path -c `
    "cd /c/Users/ikrx2/Desktop/project/FCEUX11/build && `
     make -j$(nproc) >> /c/Users/ikrx2/Desktop/project/FCEUX11/build_log.txt 2>&1"
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
# 查看日志尾部
Get-Content build_log.txt -Tail 30

# 确认可执行文件已生成
Test-Path build/src/fceux11.exe
```

成功构建后，输出文件为：

```
build/src/fceux11.exe
```

---

## 4. 常见问题排查

| 现象 | 可能原因 | 解决方案 |
|------|---------|---------|
| `rustup not found` | MSYS2 未继承 Windows PATH | 添加 `-full-path` 参数 |
| `No makefile found` | 使用了 `mingw32-make` 而非 MSYS `make` | 在 MSYS2 Shell 内执行 `make` |
| `file truncated`（静态库） | 上一次并行编译被异常中断，`.obj` 或 `.a` 损坏 | 删除 `build/src/CMakeFiles/<target>.dir` 中对应文件，或重新运行 `make` |
| 编译输出为空 | `build.bat` 在新窗口执行 | 改用本文 3.1 节的 PowerShell + `msys2_shell.cmd` 方式 |

---

## 5. 附录

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

### 附录 C：参考文件

- `build.bat` — 项目提供的旧版构建脚本（存在输出不可见问题）
- `CMakeLists.txt` — 根 CMake 配置
- `src/rust/CMakeLists.txt` — Rust 子项目配置（触发 rustup 检测）
