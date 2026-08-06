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
1. 探测 MSVC 编译器，并定位与 CI 一致的 Ninja 构建工具
2. CMake 配置（configure）
3. 编译全部源码（build）
4. 运行单元测试（test）

**Ninja 探测口径**：`ninja` 不在裸 PowerShell / Git Bash 的 `PATH` 上，**不等于电脑没有安装 Ninja**。Visual Studio 的 **C++ CMake tools for Windows** 组件自带 Ninja，默认位于：

```text
<Visual Studio 安装目录>\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe
```

`do_build.ps1` 的探测顺序是：① `PATH` 上的 `ninja.exe`；② 用 Visual Studio Installer 自带的 `vswhere.exe -latest` 定位首选 VS 安装目录，再检查上述路径；③ 检查备用 VS 安装根目录；④ 全部不存在时才回落到 legacy NMake 并打印醒目告警。因此，**不要仅凭 `Get-Command ninja` / `where ninja` 无输出就判断工具链缺失，也不需要重复安装独立版 Ninja**。

**生成器口径**：本地构建只支持与 CI 一致的 **Ninja + MSVC**。NMake 路径不保证具备与 CI 相同的构建结果；只有脚本按上述顺序仍找不到 Ninja 并明确打印 legacy NMake 回退告警时，才应通过 Visual Studio Installer 确认已勾选 **C++ CMake tools for Windows**。

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

**解决**：用开始菜单搜索打开 **"Developer PowerShell for Visual Studio"**，然后重新执行编译命令。或者直接跑 `.\scripts\do_build.ps1`（它会自动探测并加载 VS 环境）。

### 7.2 `ninja` / `where ninja` 找不到命令

**原因**：Visual Studio 自带的 Ninja 默认不加入普通 shell 的 `PATH`。这是 PATH 可见性问题，不代表 Ninja 或工具链没有安装。

**首选解决**：直接运行 `.\scripts\do_build.ps1`；脚本会通过 `vswhere.exe -latest` 定位首选 VS 安装，并自动使用其中的 Ninja。

如需手工确认本机所有 VS 内置 Ninja，可在 PowerShell 执行：

```powershell
$vswhere = Join-Path ${env:ProgramFiles(x86)} `
    "Microsoft Visual Studio\Installer\vswhere.exe"
$vsRoots = & $vswhere -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    -property installationPath
$ninjaPaths = $vsRoots | ForEach-Object {
    Join-Path $_ "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe"
} | Where-Object { Test-Path $_ }
$ninjaPaths
$ninjaPaths | ForEach-Object { & $_ --version }
```

只有 `vswhere` 已找到完整 VS C++ 安装、但上述列表仍为空时，才应打开 Visual Studio Installer 补装 **C++ CMake tools for Windows**。不要因为裸 `PATH` 查不到就重复安装 Ninja。

### 7.3 cmake 找不到 Qt6 / SDL2

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

### 7.4 编译中途崩溃（编译器内存不足）

**原因**：编译 Qt6 等大型库时内存不足（8 GB 内存容易遇到）。

**解决**：
1. 关闭 Chrome、VS Code 等吃内存的应用
2. 系统设置 → 高级系统设置 → 高级 → 性能 → 虚拟内存 → 增加虚拟内存（建议 8-16 GB）

### 7.5 vcpkg 下载很慢

**原因**：vcpkg 从 GitHub / 各项目官网下载源码，国内网络可能较慢。

**解决**：
- 开启代理后再运行 `.\scripts\setup_vcpkg.ps1`
- 或耐心等待，vcpkg 支持断点续传

### 7.6 Rust crate 编译报错 `linker link.exe not found`

**原因**：Rust 未找到 MSVC 链接器。

**解决**：用 **Developer PowerShell for VS 2022** 执行，或手动设置 Rust 工具链：
```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### 7.7 裸 `cmake --build` 报 `C1083 <cstdio>: No such file` 或 `fatal error C1034: stdafx.h`

**原因**：直接 `cmake --build build`（没有走 `do_build.ps1`）时，vcvars 没有被加载，
`cl.exe` / `Windows SDK` 不在 `PATH` / `INCLUDE` / `LIB` 上，编译器找不到标准头文件。
常见于：① 从裸 Git Bash 跑 `cmake --build`；② 在 PowerShell 里手动 `cmake --build` 但没先
跑过 vcvars。

**解决**：**始终用 `do_build.ps1` 触发构建**——它会探测 `vcvarsall.bat`、把 `cl.exe`、
MSVC include/lib 目录、vcpkg toolchain 一次性注入当前进程的环境，再调 `cmake`。如果
一定要手工分步（仅用于调试），按 §8.4 的步骤 1~2 先把 vcvars 加载完，且后续全部命令
**在同一个 PowerShell 会话里**执行（`vcvarsall.bat` 只修改当前 shell 状态，不会持久化）。

> 这一条是 Stage-2 P2-4 的清扫记录：早期 `do_build.ps1` 没有明确写出"为什么要用它"，导致
> 接手者从 git-bash 复制 `cmake --build` 命令行，触发 C1083。正确做法是任何本地构建都
> 走 `do_build.ps1`，CI 的 `ci.yml` 已经走同一条路径并始终通过。

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

> **⚠️ 推荐**：直接使用 [`.\scripts\do_build.ps1 -Config Release`](#4-编译) —— 它已经按顺序完成 vcvars 加载、vcpkg toolchain 注入、Ninja 探测、CMake 配置、编译、测试六步，并自动选择受支持的 Ninja + MSVC 工具链。**不要**在裸 Git Bash 或未加载 vcvars 的 PowerShell 里直接 `cmake --build`——会触发 C1083 `<cstdio>` 等标准头文件找不到的错（见 §7.7）。
>
> 若确有分步需求（例如调试 CMake configure 阶段），按下方手动加载 MSVC 环境后，直接用对应 cmake 命令：

```powershell
# 1. 打开 Developer PowerShell for Visual Studio，确保 cl.exe 环境已加载。
#    旧版 .\scripts\_find_vcvars.bat 已归档，不再维护。

# 2. 若 ninja.exe 不在 PATH，用 vswhere 定位 VS 自带版本并临时加入 PATH。
$vswhere = Join-Path ${env:ProgramFiles(x86)} `
    "Microsoft Visual Studio\Installer\vswhere.exe"
$vsRoot = & $vswhere -latest -products * `
    -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
    -property installationPath
$ninja = Join-Path $vsRoot `
    "Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja\ninja.exe"
if (-not (Test-Path $ninja)) { throw "Visual Studio bundled Ninja not found" }
$env:PATH = "$(Split-Path -Parent $ninja);$env:PATH"
& $ninja --version

# 3. 配置
cmake -S . -B build -G Ninja -DCMAKE_BUILD_TYPE=Release

# 4. 编译
cmake --build build

# 5. 测试
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
# → target/x86_64-pc-windows-msvc/release/kagami-qa-runner.exe
```

> **注意产物路径带 target 三元组**：`src/rust/.cargo/config.toml` 设了
> `build.target = "x86_64-pc-windows-msvc"`，所以 cargo 输出到
> `target/x86_64-pc-windows-msvc/release/`，**不是** `target/release/`。
> 本文档此前写的是后者（v1.16 R4-1 已更正）。若你的 `target/release/` 下
> 也有一个 `kagami-qa-runner.exe`，那是历史遗留的**陈旧副本** —— 按旧路径跑
> 会用到过期二进制，产出的矩阵无法反映当前代码。以 `git ls-files` 无法察觉，
> 请以修改时间/大小核对，或直接 `cargo clean`。

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
- **`R4 Gate` 步**：矩阵缺失 / `engine.git_rev` 为 `unknown` / `summary.total < 39` /
  `transition_matrix.fail_to_pass != 0` 任一成立即让作业失败。此前所有实质步骤都带
  `continue-on-error: true`，矩阵静默缺失时无人察觉；该 gate 把验收报告 §十 R4 的证伪判据变成门禁

### 10.8 CI 的 vcpkg 缓存机制（v1.16 R4-0）

> **本节只影响 CI，本地开发流程完全不受影响**，但了解它有助于看懂 CI 日志。

GitHub runner 每次都是干净机器。整改前两个 workflow 的缓存路径写错（缓存了空目录、且
`${{ env.LOCALAPPDATA }}` 在 workflow 级上下文展开为空串），导致缓存从未生效、**每一轮 CI 都在从源码
冷编 Qt 6.8.0**——`kagami-qa.yml` 因此在配置阶段撞 45 分钟超时被取消（run `82956632293`）。

现在的机制：

| 机制 | 说明 |
|------|------|
| `-DVCPKG_INSTALLED_DIR=<workspace>/vcpkg_installed` | 让 vcpkg 装到**仓库根**而非 `build/` 下。这样 (a) 缓存步能真的缓存到它，(b) 下一轮 `CMakeLists.txt:7` 的 prefer-local 分支命中后**完全跳过 vcpkg**，(c) `tests/CMakeLists.txt:431` 的测试 DLL PATH 注入指向真实目录 |
| `cmake/triplets/x64-windows.cmake` | overlay triplet，`VCPKG_BUILD_TYPE release` —— CI 只构 Release，不编 debug 半边。安装树体积从 2.4 GB 降到约 1.2 GB，冷编时间对半砍。**故意与内置 triplet 同名**，因为安装目录名必须保持 `x64-windows` 才能被上面三条命中 |
| 缓存 `path` 收窄 | 只存 `vcpkg_installed/x64-windows` + `vcpkg/{status,info}` + `vcpkg_bincache`；数 GB 的 `vcpkg/blds` buildtrees 刻意排除 |
| `timeout-minutes: 180` | 只为容纳**一轮**冷预热跑（预计 60-90 分钟）。预热后稳态每轮约 15 分钟 |

**看 CI 日志时**：若顶部出现 `::warning::vcpkg cache miss`，说明本轮是冷跑，配置步会慢很多，属正常。

**本地为什么不受影响**：overlay triplet 只在传 `-DVCPKG_OVERLAY_TRIPLETS` 时才被读取，
而 `scripts/do_build.ps1` 不传该参数——它走的是自己的 `-DVCPKG_MANIFEST_MODE=OFF` +
复用现成 `vcpkg_installed/x64-windows` 的路径（`do_build.ps1:120-135`），与 CI 互不干扰。

完整背景见 `docs/history/reports/FCEUX11-1.16_CI-R4-实跑诊断.md`。

---

**文档结束** — 如有问题，请提交 [GitHub Issues](https://github.com/Laffinty/FCEUX11/issues)。
