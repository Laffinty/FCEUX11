# FCEUX11 Build Guide — MSVC 2022 + vcpkg

> **适用范围**: FCEUX11 v0.2.2+
> **工具链**: Microsoft Visual C++ (MSVC) 2022 v143+, CMake 3.28+, Ninja, vcpkg

---

## Prerequisites

| Component | Minimum Version | Installation |
|-----------|-----------------|--------------|
| Visual Studio 2022 | 17.x | [Download](https://visualstudio.microsoft.com/) — Workload: "Desktop development with C++" |
| CMake | 3.28 | Bundled with VS 2022 or [standalone](https://cmake.org/download/) |
| Ninja | 1.11 | Bundled with VS 2022 or `pip install ninja` |
| vcpkg | latest | Clone + bootstrap (see below) |
| PowerShell | 7.x | [Download](https://github.com/PowerShell/PowerShell) |
| Git | 2.40+ | [Download](https://git-scm.com/download/win) |
| Rust (required since v0.2.2) | stable | [rustup.rs](https://rustup.rs/) — target: `x86_64-pc-windows-msvc` |

### 1. Install vcpkg

```powershell
# Clone vcpkg (pick a permanent location)
git clone https://github.com/microsoft/vcpkg.git C:\dev\vcpkg
C:\dev\vcpkg\bootstrap-vcpkg.bat

# Set environment variable (persistent)
[Environment]::SetEnvironmentVariable("VCPKG_ROOT", "C:\dev\vcpkg", "User")
```

### 2. Install Rust target (required since v0.2.2)

```powershell
rustup target add x86_64-pc-windows-msvc
```

### 3. Verify toolchain

```powershell
cmake --version    # >= 3.28
ninja --version    # >= 1.11
cl.exe             # Should print "Microsoft (R) C/C++ Optimizing Compiler"
vcpkg.exe --version
rustup show        # Should list x86_64-pc-windows-msvc (installed)
```

> **Note**: Run these commands from a **Developer PowerShell for VS 2022**, or run `vcvarsall.bat x64` first.

---

## Build Steps

### Quick Build (one-liner)

```powershell
.\do_build.ps1 -Config Release
```

This script handles configure, build, and test automatically. It also auto-detects the environment and loads VS toolchains if needed.

### Manual Build

```powershell
# 1. Install dependencies (first time only; may take 1-2 hours)
.\scripts\setup_vcpkg.ps1

# 2. Configure
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# 3. Build
cmake --build build

# 4. Test
ctest --test-dir build --output-on-failure
```

### Output

- Main executable: `build\src\fceux11.exe`
- Smoke test: `build\src\tests\fceux11_smoke_test.exe`
- Mapper tests: `build\src\tests\fceux11_mapper_load_test.exe`

---

## Deploying Dependencies

### Option A: CMake Install (Recommended)

```powershell
cmake --install build --prefix dist
```

This copies `fceux11.exe` and all required vcpkg DLLs to `dist\`.

### Option B: PowerShell Script

```powershell
.\scripts\copy_dependencies.ps1 `
    -ExecutablePath "build\src\fceux11.exe" `
    -OutputDir "dist"
```

---

## Agent 标准化编译规范

> **目标**：为自动化 Agent（CI/CD、代码助手、回归测试机器人）提供**零歧义、可复现、可回退**的编译指令。
> 
> **示范来源**：Phase 1 (v0.2.2) MD5 Rust 重构验证了以下流程的有效性。

### 1. 环境前提检查清单

Agent 在执行任何编译前，必须确认以下工具可用；若缺失，应立即报错并中止，**不可静默 fallback 到未知工具链**。

```powershell
# 强制检查（PowerShell）
$required = @("cmake","ninja","cl","vcpkg","rustup","cargo")
foreach ($cmd in $required) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        throw "Required tool '$cmd' not found in PATH. Aborting."
    }
}
# 验证 Rust target 已安装
$targets = rustup target list --installed
if ($targets -notmatch "x86_64-pc-windows-msvc") {
    throw "Rust target 'x86_64-pc-windows-msvc' is not installed. Run: rustup target add x86_64-pc-windows-msvc"
}
```

### 2. 编译配置矩阵

| 场景 | Generator | Rust | 额外 CMake 选项 | 用途 |
|------|-----------|------|----------------|------|
| **标准 Release** | Ninja | ON (默认) | `-DCMAKE_BUILD_TYPE=Release` | 日常开发、发布 |
| **标准 Debug** | Ninja | ON (默认) | `-DCMAKE_BUILD_TYPE=Debug` | 本地调试 |
| **Rust OFF 回退验证** | Ninja | OFF | `-DFCEUX11_ENABLE_RUST=OFF` | 验证 C++ fallback 仍可编译 |
| **仅 C++ 侧（无 Rust）** | Ninja | OFF | `-DFCEUX11_ENABLE_RUST=OFF` | 纯 C++ 调试、Rust 工具链缺失环境 |

> **规则**：每个涉及 Rust 重构的 Pull Request，CI 必须同时通过 **Rust=ON** 和 **Rust=OFF** 两种配置。

### 3. 增量编译与目标级构建

Agent 在验证局部修改（如单个 `.cpp` / `.rs` 文件变更）时，**不应**触发全量构建。推荐目标级构建：

```powershell
# 若修改了 Rust 侧代码（如 src/rust/src/md5.rs）
cmake --build build --target fceux11_rust_build

# 若修改了 C++ utils 侧代码（如 src/utils/md5.cpp）
cmake --build build --target fceux11_utils

# 若修改了核心模拟器逻辑
cmake --build build --target fceux11_core

# 最终链接验证（最小全量）
cmake --build build --target fceux11
```

### 4. 回退策略（Feature-Gate 规范）

自 v0.2.2 起，每个 Rust 迁移模块均遵循 **Wrapper-Shim + Feature-Gate** 范式：

- C++ 侧保留同名函数和结构体定义（零侵入调用方）。
- `.cpp` 实现文件内部通过 `#ifdef FCEUX11_RUST_ENABLED` 分支：
  - 定义时 → 调用 Rust FFI (`extern "C"`)。
  - 未定义时 → 编译原 C++ 实现。
- CMake 全局开关：`-DFCEUX11_ENABLE_RUST=ON/OFF`。

**Agent 回退测试模板**：

```powershell
# Step 1: 标准路径（Rust ON）
cmake -S . -B build_rust_on -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build build_rust_on --target fceux11

# Step 2: 回退路径（Rust OFF）
cmake -S . -B build_rust_off -G Ninja -DCMAKE_BUILD_TYPE=Release `
    -DFCEUX11_ENABLE_RUST=OFF
cmake --build build_rust_off --target fceux11

# Step 3: 若 Step 2 失败，说明 C++ fallback 被破坏，立即阻断合并
```

### 5. 目录与产物规范

| 产物 | 路径（相对于构建根） | 说明 |
|------|---------------------|------|
| Rust staticlib | `build/src/rust/x86_64-pc-windows-msvc/release/fceux11_rust.lib` | 由 `cargo` 生成，CMake 自动链接 |
| C++ utils lib | `build/src/fceux11_utils.lib` | 包含 MD5、CRC32 等 wrapper |
| 主程序 | `build/src/fceux11.exe` | 最终可执行文件 |
| 测试程序 | `build/src/tests/fceux11_smoke_test.exe` | 冒烟测试 |

### 6. 常见错误与自动修复

| 错误信息 | 根因 | Agent 自动修复动作 |
|---------|------|-------------------|
| `Rust target 'x86_64-pc-windows-msvc' is not installed` | Rust target 缺失 | 执行 `rustup target add x86_64-pc-windows-msvc` |
| `generator : Ninja Does not match the generator used previously` | 构建目录残留不同 generator 的缓存 | **删除** `build/` 目录后重新配置 |
| `ninja: error: loading 'build.ninja': The system cannot find the file` | 未配置或使用了错误 generator | 重新运行 `cmake -S . -B build -G Ninja` |
| `vcpkg package ... not found` | 本地 vcpkg 缓存未命中 | 运行 `.\scripts\setup_vcpkg.ps1` |
| `Qt6LinguistTools not found` | vcpkg 未安装翻译工具 | 添加 `-DFCEUX11_ENABLE_I18N=OFF` |

### 7. 新增 Rust 模块时的 Agent Checklist

当路线图进入新 Phase（如 Phase 2 GUID、Phase 3 General Utilities）时，Agent 必须执行：

1. [ ] `src/rust/Cargo.toml` 版本号递增（如 `0.2.2` → `0.2.3`）。
2. [ ] 新增 crate 依赖需记录在 `[dependencies]` 或 `[dev-dependencies]`，并附注释说明用途。
3. [ ] `src/rust/fceux11_rust.h` 追加新的 `extern "C"` 声明区块。
4. [ ] C++ wrapper 文件（如 `src/utils/xxx.cpp`）保留 `#ifdef FCEUX11_RUST_ENABLED` 分支。
5. [ ] Rust 侧编写至少一个 `#[test]`，并用成熟 crate（如 `md-5`、`uuid`）做交叉验证。
6. [ ] CI 通过 **Rust=ON** 和 **Rust=OFF** 两种配置的全量构建。
7. [ ] 更新本文档的「适用范围」版本号。

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `cmake` not found | Not in PATH | Launch "Developer PowerShell for VS 2022" |
| `nmake` not found | Wrong generator | Use `-G Ninja` (recommended) or launch from VS Dev Prompt |
| vcpkg packages fail to build | Proxy / network | Set `HTTP_PROXY` / `HTTPS_PROXY` environment variables |
| Qt6 plugins not found at runtime | Missing `QT_PLUGIN_PATH` | Deploy via `cmake --install` or copy `plugins\` from vcpkg |
| `vcpkg` not found | `VCPKG_ROOT` not set | Set the environment variable and restart shell |
| PowerShell execution policy blocked | Default policy is `Restricted` | `Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser` |
| `rustup not found` | Rust not installed | Install from https://rustup.rs/ and ensure `cargo` / `rustup` are on PATH |
| `Rust target 'x86_64-pc-windows-msvc' is not installed` | Target missing | `rustup target add x86_64-pc-windows-msvc` |

---

## Notes

- **MSYS2 / MinGW-w64 support was removed in v0.2.1**. Do not use `MSYS Makefiles`, `mingw32-make`, or any POSIX toolchain.
- The project requires **Windows 11** (or Windows 10 21H2+ with latest updates).
- First vcpkg build may take 1-2 hours; subsequent builds reuse cached artifacts.
- **Rust integration is mandatory since v0.2.2**; however, each module retains a C++ fallback via `-DFCEUX11_ENABLE_RUST=OFF`.
