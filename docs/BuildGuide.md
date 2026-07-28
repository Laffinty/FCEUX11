# FCEUX11 编译指南 / Build Guide

> **适用版本**：FCEUX11 v1.16+
> **目标平台**：Windows 11 22H2+（64-bit）
> **预计首次编译时间**：30-60 分钟（取决于网络和 CPU）

---

## 0. 快速开始

如果你已经装了 Visual Studio 2022 和 Rust，直接执行：

```powershell
git clone https://github.com/Laffinty/FCEUX11.git
cd FCEUX11
.\scripts\setup_vcpkg.ps1
$env:VCPKG_ROOT = "$PWD\vcpkg"
.\scripts\do_build.ps1 -Config Release
```

产物：`build\src\fceux11.exe`

**没有装过？下面一步一步来。**

---

## 1. 安装前置工具（只需做一次）

### 1.1 Visual Studio 2022 Community（免费）

1. 访问 https://visualstudio.microsoft.com/zh-hans/downloads/
2. 下载 **Visual Studio 2022 Community**（免费）
3. 运行安装器，在"工作负载"页面勾选 **"使用 C++ 的桌面开发"**
4. 点击"安装"，等待完成（约 15-30 分钟）

> 默认安装路径（C 盘）即可。如果装到 D 盘也完全没问题，构建脚本会自动探测。

### 1.2 Rust

1. 访问 https://rustup.rs/
2. 下载 `rustup-init.exe` 并运行
3. 出现提示时选 **1**（默认安装）
4. 等待完成（约 2 分钟）

### 1.3 验证安装

打开 **PowerShell**，运行以下命令确认：

```powershell
# 应显示 Microsoft (R) C/C++ Optimizing Compiler Version 19.xx
cl /Bv 2>&1 | Select-String "Compiler"

# 应显示 cmake version 4.x.x
cmake --version

# 应显示 rustc 1.78+
rustc --version
```

> 如果 `cl` 或 `cmake` 报错"找不到命令"，请用开始菜单搜索并打开 **"Developer PowerShell for VS 2022"**，它已自动配置好环境变量。

---

## 2. 拉取源码

```powershell
git clone https://github.com/Laffinty/FCEUX11.git
cd FCEUX11
```

这会在当前目录创建 `FCEUX11` 文件夹。

---

## 3. 安装 vcpkg 依赖

运行项目自带的安装脚本：

```powershell
.\scripts\setup_vcpkg.ps1
```

这会自动完成：
- 克隆 vcpkg 包管理器
- 下载并编译所有 C++ 依赖（Qt6、SDL2 等，约 9 个包）
- 耗时约 20-40 分钟，耐心等待

完成后，设置环境变量：

```powershell
$env:VCPKG_ROOT = "$PWD\vcpkg"
```

> 为了方便以后使用，可以在 PowerShell 里执行：
> ```powershell
> [Environment]::SetEnvironmentVariable("VCPKG_ROOT", "$PWD\vcpkg", "User")
> ```

---

## 4. 编译

```powershell
.\scripts\do_build.ps1 -Config Release
```

这个脚本自动完成：
1. 探测 MSVC 编译器和 Ninja/NMake 构建工具
2. CMake 配置（configure）
3. 编译全部源码（build）
4. 运行单元测试（test）

**编译耗时**：首次约 10-20 分钟（Qt6 编译过的前提下），后续增量编译 1-3 分钟。

> 如果需要完全干净重建（清空缓存重来）：
> ```powershell
> .\scripts\do_build.ps1 -Config Release -Clean
> ```

---

## 5. 验证编译结果

```powershell
# 查看版本号
.\build\src\fceux11.exe --version

# 启动模拟器
.\build\src\fceux11.exe
```

如果能启动并显示 GUI 界面，恭喜编译成功！

---

## 6. 部署：打包成可分发的 zip

```powershell
# 复制必需 DLL 到 dist 目录
.\scripts\copy_dependencies.ps1 -ExecutablePath .\build\src\fceux11.exe -OutputDir .\dist

# 打包
Compress-Archive -Path dist\* -DestinationPath FCEUX11-v1.16-win64.zip
```

`dist` 目录可直接运行，复制到任意 Windows 11 电脑都能启动。

---

## 7. 常见问题

### 7.1 `'cl' is not recognized`

**原因**：PowerShell 未加载 VS 环境变量。

**解决**：用开始菜单搜索打开 **"Developer PowerShell for VS 2022"**，然后重新执行编译命令。或者直接跑 `.\scripts\do_build.ps1`（它会自动探测并加载 VS 环境）。

### 7.2 cmake 找不到 Qt6 / SDL2

**原因**：vcpkg 依赖未正确安装或 VCPKG_ROOT 未设置。

**解决**：
```powershell
# 确认 VCPKG_ROOT 已设置
echo $env:VCPKG_ROOT

# 如果为空，手动设置
$env:VCPKG_ROOT = "$PWD\vcpkg"

# 重新编译
.\scripts\do_build.ps1 -Config Release
```

### 7.3 编译中途崩溃（编译器内存不足）

**原因**：编译 Qt6 等大型库时内存不足（8 GB 内存容易遇到）。

**解决**：
1. 关闭 Chrome、VS Code 等吃内存的应用
2. 系统设置 → 高级系统设置 → 高级 → 性能 → 虚拟内存 → 增加虚拟内存（建议 8-16 GB）

### 7.4 vcpkg 下载很慢

**原因**：vcpkg 从 GitHub / 各项目官网下载源码，国内网络可能较慢。

**解决**：
- 开启代理后再运行 `.\scripts\setup_vcpkg.ps1`
- 或耐心等待，vcpkg 支持断点续传

### 7.5 Rust crate 编译报错 `linker link.exe not found`

**原因**：Rust 未找到 MSVC 链接器。

**解决**：用 **Developer PowerShell for VS 2022** 执行，或手动设置 Rust 工具链：
```powershell
rustup default stable-x86_64-pc-windows-msvc
```

---

## 8. 高级选项

### 8.1 编译 Debug 版本

```powershell
.\scripts\do_build.ps1 -Config Debug
```

### 8.2 禁用 Rust（纯 C++ 模式）

```powershell
cmake -S . -B build-cpp -G Ninja -DFCEUX11_RUST_ENABLED=OFF
cmake --build build-cpp
```

> Lua 功能需要 Rust crate；禁用 Rust 后 Lua 脚本功能不可用，其余正常。

### 8.3 关闭单元测试

```powershell
cmake -S . -B build -G Ninja -DFCEUX11_BUILD_TESTS=OFF
cmake --build build
```

### 8.4 手动分步编译（不用一键脚本）

```powershell
# 1. 加载 MSVC 环境（Developer PowerShell for VS 2022 已加载则跳过）
$vcvars = & .\scripts\_find_vcvars.bat
cmd /c "`"$vcvars`" && set" | ForEach-Object {
    if ($_ -match "^([^=]+)=(.*)$") {
        Set-Item "Env:$($matches[1])" $matches[2]
    }
}

# 2. 配置
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# 3. 编译
cmake --build build

# 4. 测试
ctest --test-dir build --output-on-failure
```

### 8.5 完整 CMake 选项

| 选项 | 默认值 | 说明 |
|------|--------|------|
| `CMAKE_BUILD_TYPE` | Release | `Debug` / `Release` / `RelWithDebInfo` |
| `FCEUX11_BUILD_TESTS` | ON | 编译单元测试 |
| `FCEUX11_ENABLE_I18N` | ON | 多语言翻译（12 种语言） |
| `FCEUX11_RUST_ENABLED` | ON | Rust crate（Lua 引擎） |
| `FCEUX11_LUA_RUST_ENABLED` | ON | 使用 Rust mlua 作为 Lua 引擎 |
| `FCEUX11_WGI_BACKEND` | OFF | Windows.Gaming.Input 手柄后端 |
| `FCEUX11_ASAN` | OFF | AddressSanitizer（仅 Debug） |
| `FCEUX11_UBSAN` | OFF | MSVC 运行时 UB 检测 |
| `FCEUX11_SHOW_DEPRECATION_WARNINGS` | OFF | 显示废弃 API 警告 |
| `FCEUX11_PGO` | OFF | PGO 插桩编译（第一阶段） |
| `FCEUX11_PGO_USE` | OFF | PGO 优化编译（第二阶段） |

---

## 9. 工具链约束

FCEUX11 强制使用 **MSVC 2022+** 工具链，不支持 MinGW / clang / MSYS2。这是为了保证 ABI 一致性和 ROM savestate 字节级兼容。

---

---

## 10. KagamiQA — 编译与运行

KagamiQA 是 FCEUX11 的双 Oracle 质量保障系统。详见 [`docs/tech/KagamiQA.md`](tech/KagamiQA.md)。

### 10.1 编译 KagamiQA 组件

标准编译（`do_build.ps1`）会自动编译 KagamiQA 相关测试目标：

```powershell
# 完整构建（包含 blargg_runner、lua_runner 等）
.\scripts\do_build.ps1 -Config Release
```

单独（重新）编译 KagamiQA 组件：

```powershell
# blargg $6000 ROM runner (Oracle B 执行器)
cmake --build build --config Release --target fceux11_blargg_runner

# Lua 脚本 runner
cmake --build build --config Release --target fceux11_lua_runner

# In-process direct runner (C ABI 直驱，需 Rust)
cmake --build build --config Release --target kagami_qa_direct_runner
```

### 10.2 编译 Rust kagami-qa-runner

```powershell
cd src/rust
cargo build --release -p kagami-qa
# → target/release/kagami-qa-runner.exe
```

> `--features direct-adapter` 启用 in-process 模式（需链接 fceux11_core）。

### 10.3 下载 blargg 测试 ROM

```powershell
.\scripts\download_blargg_roms.ps1
```

> 从 christopherpow/nes-test-roms GitHub 镜像下载 **180 个** blargg $6000 协议测试 ROM 到 `tests/fixtures/blargg/`。

### 10.4 运行 Oracle A（CTest 回归）

```powershell
ctest --test-dir build --build-config Release --output-on-failure -LE perf
```

### 10.5 运行 Oracle B（blargg 全量批处理）

```powershell
cd tests
..\build\tests\fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json
```

### 10.6 生成迁移矩阵

```powershell
cargo run --release -p kagami-qa -- `
  --manifest tests/tests.json `
  --bin-dir build/tests `
  --output build/kagamiqa_migration_matrix.json `
  --accuracy-table build/kagamiqa_accuracy_table.md `
  --known-fail tests/fixtures/blargg_known_fail.json `
  --save-baseline build/kagamiqa_baseline_next.json
```

### 10.7 CI 自动运行

KagamiQA 在 CI 上自动运行（`.github/workflows/kagami-qa.yml`）：
- 每次 push 到 `main` / `wip_1.16` 触发
- Oracle A + Oracle B 全量运行
- 迁移矩阵 + 精度对照表作为 artifact 上传
- PASS→FAIL 基线漂移自动 PR 评论警报

---

**文档结束** — 如有问题，请提交 [GitHub Issues](https://github.com/Laffinty/FCEUX11/issues)。
