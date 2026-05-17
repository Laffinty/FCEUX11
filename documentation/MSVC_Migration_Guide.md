# MSVC 2022+ 迁移预判清单

> **文档版本**: 1.0
> **创建日期**: 2026-05-18
> **适用范围**: Phase 6 完成后的代码基线（Commit: Phase 6 part 3）
> **目标读者**: Phase 2 MSVC 迁移执行者

---

## 1. 说明

本文档是 Phase 6 跨平台代码清理的副产品，记录了当前代码库中仍存在的 POSIX / GCC / MinGW 特定依赖。当项目从 MinGW-w64 向 MSVC 2022+ 迁移时，以下符号和惯用法需要被替换或审查。

**注意**: 此清单基于 Phase 6 清理后的代码状态。已删除的代码（如 `nanosleep`、`timerfd_create`、`realpath`、`fork` 等）不再出现在当前代码库中，故不列入。

---

## 2. 待处理符号清单

| 符号/函数 | 当前使用位置 | MSVC 替代方案 | 优先级 | 备注 |
|-----------|-------------|--------------|--------|------|
| `ssize_t` | `src/drivers/Qt/QtNetplay.cpp`（原 `unix-netplay.cpp`）及核心 I/O | `SSIZE_T` (`<Windows.h>`) 或 `ptrdiff_t` | 中 | 需全局搜索确认所有出现位置 |
| `strcasestr` | 待排查 | `StrStrIA` (`<Shlwapi.h>`) 或自实现 | 低 | 可能在 Lua 脚本或旧代码中 |
| `strtok_r` | 待排查 | `strtok_s` (MSVC) | 低 | 线程安全版本 |
| `alloca` (`__builtin_alloca`) | `src/types.h` 线 66 | `<malloc.h>` 中的 `alloca` | 中 | MSVC 支持 `alloca`，但头文件不同 |
| `__attribute__((always_inline))` | `src/lua-engine.cpp` 线 140 | `__forceinline` 关键字 | 低 | 仅 GCC 属性 |
| AT&T 内联汇编 | 待排查 | Compiler intrinsics | 高（若存在） | 需在代码库中彻底排查 |
| `dirent.h` / `DIR*` | ~~`configSys.cpp`~~（已清理） | `FindFirstFile` / `QDir` | 中 | Phase 6 已移除 `dirent.h` 依赖 |
| `setpriority` / `getpriority` | ~~`ConsoleWindow.cpp`~~（已删除） | `SetThreadPriority` / `GetThreadPriority` | 低 | Phase 6 已移除 |
| `sched_get_priority_min/max` | ~~`ConsoleWindow.cpp`~~（已删除） | `THREAD_PRIORITY_*` 常量 | 低 | Phase 6 已移除 |
| `pthread_t` | ~~`ConsoleWindow.h`~~（已删除） | `HANDLE` (Win32 thread) | 低 | Phase 6 已移除 |
| `timerfd_create` | ~~`sdl-throttle.cpp`~~（已删除） | `CreateWaitableTimer` | 低 | Phase 6 已移除；P0 改用 `QElapsedTimer` |

---

## 3. 已完成的清理（Phase 6）

以下 POSIX 依赖在 Phase 6 中已被移除，无需在 MSVC 迁移中处理：

| 符号/函数 | 清理方式 |
|-----------|---------|
| `nanosleep` | `sdl-throttle.cpp` 已重写为 `QElapsedTimer` + 自旋等待 |
| `realpath` | `fceuWrapper.cpp` 注释块已删除 |
| `fork` / `execl` | `HelpPages.cpp` 注释块已删除 |
| `dirent.h` / `DIR*` | `configSys.cpp` 非 Windows 分支已删除 |
| `setpriority` / `sched_*` | `ConsoleWindow.cpp` 调度代码已删除 |
| `pthread_t` | `ConsoleWindow.h` 线程类型已删除 |
| `timerfd_create` / `timerfd_settime` | `sdl-throttle.cpp` Linux 定时器代码已删除 |

---

## 4. 构建系统注意事项

### 4.1 CMake 差异

| MinGW (当前) | MSVC (目标) |
|-------------|-------------|
| `find_path(SDL2_INCLUDE_DIR ...)` | 使用 vcpkg 或 Conan 管理依赖；或 `find_package(SDL2 REQUIRED)` |
| `set(OPENGL_LDFLAGS -lopengl32)` | 移除；Qt6::OpenGLWidgets 自动链接 |
| `set(SDL2_LDFLAGS -lSDL2)` | `find_package(SDL2 REQUIRED)` + `target_link_libraries(... SDL2::SDL2)` |
| `set(LIBARCHIVE_LDFLAGS -larchive)` | `find_package(LibArchive REQUIRED)` |
| `add_definitions(-D_CRT_SECURE_NO_WARNINGS)` | 保留 |
| `if(MINGW)` 分支 | 改为 `if(MSVC)` 或统一处理 |

### 4.2 编译器特定定义

- `__builtin_alloca` -> `_alloca` 或直接包含 `<malloc.h>`
- `__attribute__((...))` -> 使用条件编译：`#ifdef _MSC_VER` ... `#else` ... `#endif`
- GCC 内建函数（如 `__builtin_expect`、`__builtin_clz` 等）-> 需排查并替换为 MSVC 等价物或标准 C++

---

## 5. 建议迁移顺序

1. **准备环境**: 安装 MSVC 2022+、Qt6 MSVC 二进制包、vcpkg
2. **CMake 适配**: 修改 `src/CMakeLists.txt`，处理 MSVC 与 MinGW 的差异分支
3. **符号替换**: 按上表优先级，逐一替换 POSIX / GCC 特定符号
4. **构建验证**: DEBUG + Release 均 0 错误
5. **回归测试**: 运行核心功能测试（ROM 加载、TAS、网络对战、AVI 录制等）

---

## 6. 附录

### 附录 A: 快速排查命令

```powershell
# 搜索 POSIX 特定符号
grep -rn "ssize_t\|strcasestr\|strtok_r\|alloca\|__attribute__\|__builtin_" src/

# 搜索 MinGW 特定宏
grep -rn "MINGW\|__MINGW" src/ CMakeLists.txt

# 搜索 GCC 内联汇编
grep -rn "__asm__\|asm(" src/
```

### 附录 B: 参考文档

- `documentation/FCEUX11_Construction_Plan.md` -- 原始 Construction Plan
- `documentation/FCEUX11_Phase6_Remaining_Work_Plan.md` -- Phase 6 执行计划
