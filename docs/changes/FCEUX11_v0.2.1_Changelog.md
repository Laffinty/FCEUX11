# FCEUX11 v0.2.1 变更日志

> **发布日期**: 待定  
> **全名**: FCEUX11 v0.2.1 — MSVC + vcpkg Single-Track Build

---

## 1. 工具链迁移

- **彻底弃用 MinGW-w64 / MSYS2 构建路径**，确立 MSVC 2022+ 为唯一官方工具链。
- 所有依赖统一通过 **vcpkg manifest 模式** 管理 (`vcpkg.json`)。
- Rust 模块默认 target 切换为 `x86_64-pc-windows-msvc`。

## 2. 构建系统重构

- `CMakeLists.txt` 强制 MSVC 编译器检查（`if(NOT MSVC)` → `FATAL_ERROR`）。
- 删除 `src/CMakeLists.txt` 中全部 `if(MINGW)` 分支与 msys64 硬编码路径。
- 启用 MSVC 安全编译选项：`/W4 /permissive- /guard:cf /GS /sdl`。
- 链接选项：`/GUARD:CF /CETCOMPAT`。
- `vcpkg.json` 修正语法并升级版本号至 `0.2.1`。

## 3. 源码兼容层清理

- `alloca` / `__forceinline` 宏冲突修复（增加 `#ifndef` 守卫或改用 MSVC 原生提供）。
- POSIX 类型缺失映射：`ssize_t` → `SSIZE_T` / `ptrdiff_t`。
- POSIX 函数替换：`strcasestr` / `strtok_r` / `strndup` / `gettimeofday` 等改用 Windows API 或标准 C++。
- 内联汇编与 GCC 扩展清理：替换为 MSVC Compiler Intrinsics。
- C++20 `/permissive-` 兼容性加固（`volatile` → `std::atomic`，lambda `this` 捕获显式化等）。

## 4. 脚本与文档清理

- 新增 `do_build.ps1`（纯 PowerShell，零 POSIX 依赖）。
- `do_build.bat` 改为极简调用入口（转发至 `do_build.ps1`）。
- 删除 `build.sh` 及所有 Unix/Linux/macOS 遗留脚本。
- `scripts/copy_dependencies.ps1` 重构为 vcpkg-aware DLL 部署。
- `docs/tech/Build_Guide_MSYS2_Mingw64.md` 归档为 `.DEPRECATED`。
- 新增官方构建指南 `docs/tech/Build_Guide_MSVC_vcpkg.md`。
- `DLL_DEPENDENCIES.md` 重写为 vcpkg 版本。
- `fceux-server/cygwin1.dll` 删除。

## 5. 版本号同步

以下文件统一升级至 **v0.2.1**：

| 文件 | 字段 |
|------|------|
| `CMakeLists.txt` | `project(FCEUX11 VERSION 0.2.1 ...)` |
| `src/version.h` | `FCEU_VERSION_PATCH` / `FCEU_VERSION_STRING` / `FCEU_DISPLAY_VERSION` |
| `vcpkg.json` | `"version": "0.2.1"` |
| `src/rust/Cargo.toml` | `version = "0.2.1"` |
| `readme.md` | 版本提及与工具链声明 |

## 6. 其他

- `.gitignore` 更新：新增 `vcpkg_installed/`、`build_test/`、MSVC 中间产物（`*.sdf` / `*.VC.db` / `ipch/` 等）。
- 移除已误跟踪的 `vcpkg_installed/vcpkg/vcpkg-running.lock`。
