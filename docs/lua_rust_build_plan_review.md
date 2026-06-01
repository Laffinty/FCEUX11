# FCEUX11 Lua Rust 重构计划复核报告

> **复核日期**：2026-06-01
> **基线版本**：v0.2.22
> **复核范围**：`docs/lua_rust_build_plan.md` 完成度、编译流程正确性、`src/lua-engine.cpp` 瘦身可行性

---

## 一、计划完成度总评

| 阶段 | 计划版本 | 计划状态 | 实际代码状态 | 完成度 |
|------|---------|---------|-------------|--------|
| 基础设施搭建 | v0.2.22.1 | ✅ | ⚠️ 有缺陷 | 70% |
| P0：`bit` 库 | v0.2.22.1 | ✅ | ✅ 完整 + 单元测试 | 100% |
| P1：`emu` + `memory` + `joypad` + `rom` | v0.2.22.2–3 | ✅ | ⚠️ 关键功能缺失 | 65% |
| P2：`movie` + `savestate` + `gui` + `input` + `ppu` | v0.2.22.4–5 | ✅ | ⚠️ 部分桩函数 | 55% |
| P3：`sound` + `zapper` + `debugger` | v0.2.22.6 | ✅ | ✅ FFI 已完成 | 90% |
| P4：平台专属 + C++ 兼容层切换 | v0.2.22.7 | 📋 | ❌ 未启动 | 0% |

**综合完成度：约 55%** — 库绑定的"骨架"基本搭完，但核心运行时逻辑（协程 frameadvance、内存钩子、回调注册）均为空壳，且编译流程存在多个阻断性问题。

---

## 二、编译流程复核 — 阻断性问题

### 🔴 P0-BLOCK-1：双 Lua VM 共存导致链接冲突

**现状**：

- `src/CMakeLists.txt:62–103` 将 `src/lua/*.c`（27 个 C 文件）+ `lua-engine.cpp` 编入 `fceux11_core` 静态库
- `fceux11-lua` crate 通过 `mlua` 的 `vendored` feature 独立编译了另一份 Lua 5.1 源码
- 最终链接时，两个 Lua 5.1 的符号（`lua_newstate`、`lua_pcall` 等）会冲突

**计划描述**：计划 §2.1.3 声称"mlua 使用 vendored feature 独立编译；不与 src/lua/ 同时链接"，风险矩阵 R1 也提到此问题。但实际代码中 **没有实现任何隔离机制**。

**修复方案**：

```
方案 A（推荐）：CMake 条件编译
  当 FCEUX11_LUA_RUST_ENABLED=ON 时：
    1. 从 LUA_ENGINE_SOURCE 中移除 src/lua/*.c
    2. lua-engine.cpp 仅保留 FFI 桥接函数 + 序列化 + taseditor/cdlog
    3. 链接 fceux11_rust.lib（内含 mlua vendored Lua）
  当 FCEUX11_LUA_RUST_ENABLED=OFF 时：
    1. 保持现有编译流程不变

方案 B：mlua 静态重导出前缀
  编译 mlua 时给所有 Lua 符号加前缀（如 mlua_lua_newstate），
  但 mlua 不原生支持此功能，需 fork 维护，不可取。
```

**涉及文件**：`src/CMakeLists.txt:56–104`、`src/rust/CMakeLists.txt`

---

### 🔴 P0-BLOCK-2：根 crate 未 re-export `fceux11-lua`

**现状**：

- `src/rust/Cargo.toml:27` 声明了 `fceux11-lua = { path = "crates/fceux11-lua" }`
- 但 `src/rust/src/lib.rs` 只有 4 行：`pub use fceux11_utils/media/formats/debug;` — **缺少 `pub use fceux11_lua;`**
- 根 crate 类型为 `staticlib`，Rust 静态库只会包含被根 crate 直接或间接引用的符号
- 结果：`fceux11_lua_init`、`fceux11_lua_load_script` 等 FFI 导出函数 **可能不会被链接进 fceux11_rust.lib**

**修复**：

```rust
// src/rust/src/lib.rs — 添加这一行
pub use fceux11_lua;
```

**涉及文件**：`src/rust/src/lib.rs:1–4`

---

### 🔴 P0-BLOCK-3：lua-engine.cpp 全量编译，无切换机制

**现状**：

- `src/CMakeLists.txt:464` 将 `fceux11_core` 静态库链接到 `fceux11_drivers_common` → `fceux11_drivers_qt` → 最终可执行文件
- `lua-engine.cpp`（7186 行）全量编译，包含旧的完整 C++ Lua 引擎 + 新增的 FFI 桥接函数
- 即使 Rust 引擎初始化成功，旧引擎的全局状态（`L`、`luaRunning`、`gui_data` 等）仍会存在并可能被其他 C++ 代码引用
- **无 `FCEUX11_LUA_RUST_ENABLED` 宏守卫** — 计划 §4.9.3 描述的 `#ifdef` 切换机制完全未实现

**修复**：

在 `src/CMakeLists.txt` 中添加选项，在 `lua-engine.cpp` 中添加条件编译：

```cmake
# src/CMakeLists.txt
option(FCEUX11_LUA_RUST_ENABLED "Use Rust Lua engine instead of C++" OFF)
if(FCEUX11_LUA_RUST_ENABLED)
    add_definitions(-DFCEUX11_LUA_RUST_ENABLED)
    # 仅编译 FFI 桥接部分，排除 src/lua/*.c
    set(LUA_ENGINE_SOURCE ${CMAKE_CURRENT_SOURCE_DIR}/lua-engine.cpp)
else()
    # 原有逻辑：包含 src/lua/*.c + lua-engine.cpp
    ...
endif()
```

```cpp
// lua-engine.cpp 顶部
#ifdef FCEUX11_LUA_RUST_ENABLED
// 仅保留：FFI 桥接函数(6672-7186) + LuaSaveData 序列化(762-1328)
//         + TieredRegion(2042-2362) + taseditor(4937-5235) + cdlog(5237-5310)
// 移除：旧引擎的所有全局变量、库注册、frameboundary 逻辑
#else
// 原有完整 C++ Lua 引擎
#endif
```

**涉及文件**：`src/CMakeLists.txt:56–104`、`src/lua-engine.cpp`

---

### 🟡 P1-WARN-1：缺少 `.cargo/config.toml`

**现状**：项目无 `.cargo/config.toml`，无法锁定 linker 为 MSVC 链接器。当 `cargo build --target x86_64-pc-windows-msvc` 执行时，Rust 会使用默认 MSVC 链接器，但无法传递额外的链接器标志（如 Windows SDK lib 路径）。

**修复**：

```toml
# src/rust/.cargo/config.toml
[build]
target = "x86_64-pc-windows-msvc"

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

---

### 🟡 P1-WARN-2：vcpkg 中无 Lua 依赖

**现状**：`vcpkg.json` 依赖列表中无 Lua。项目当前使用 `src/lua/` 内嵌源码。当切换到 Rust 引擎后，`mlua` 的 `vendored` feature 自带 Lua 5.1 源码编译，但 **两份 Lua 5.1 会同时存在于编译流程中**（见 P0-BLOCK-1）。

**建议**：切换到 Rust 引擎后，从 `SRC_CORE` 中移除 `src/lua/*.c`，无需在 vcpkg 中添加 Lua。

---

### 🟡 P1-WARN-3：cbindgen 对 `fceux11-lua` 的 `extern "C"` 声明处理

**现状**：`fceux11_rust.h` 中同时包含：
- Rust 导出的 FFI 函数（如 `fceux11_lua_init`、`fceux11_lua_load_script`）
- C++ 实现的 `extern` 桥接函数（如 `fceux11_lua_GetMem`、`fceux11_lua_BWrite`）

cbindgen 对 `fceux11-lua` crate 的 `parse_deps = false` 意味着它只导出 `#[no_mangle]` 函数。但 `unsafe extern "C"` 块中的声明也会被 cbindgen 输出为 `extern` 声明，这些在 C++ 侧已有实现，会导致 **重复声明**（虽然不是链接错误，但某些编译器会发出警告）。

**修复**：在 `fceux11-lua/cbindgen.toml` 中排除 FFI 桥接声明：

```toml
[export.exclude]
# 这些由 C++ 侧实现，不需要 cbindgen 生成声明
fceux11_lua_GetMem = true
fceux11_lua_BWrite = true
# ... 所有 extern "C" 桥接函数
```

或在 `lib.rs` 中将 FFI 声明放入非 `pub` 模块，让 cbindgen 忽略。

---

## 三、工具链目录复核

### 3.1 Windows Kits（D:\Windows Kits）

| 项目 | 值 | 状态 |
|------|-----|------|
| SDK 版本 | 10.0.26100.0 | ✅ 当前最新 |
| Include 目录 | `D:\Windows Kits\10\Include\10.0.26100.0\` | ✅ 存在 |
| Lib 目录 | `D:\Windows Kits\10\Lib\10.0.26100.0\` | ✅ 存在（含 um/ucrt/ucrt_enclave） |
| UCRT | ✅ 存在 | ✅ MSVC CRT 依赖满足 |

**结论**：Windows SDK 10.0.26100.0 完整可用，与 MSVC 2022+ 工具链兼容。

### 3.2 vcpkg（D:\vcpkg）

| 项目 | 值 | 状态 |
|------|-----|------|
| vcpkg 根目录 | `D:\vcpkg\` | ✅ 存在 |
| vcpkg.exe | ✅ 存在 | ✅ |
| 已安装 triplet | `x64-windows` | ✅ |
| Qt6 (qtbase) | ✅ 已安装 | ✅ |
| SDL2 | ✅ 已安装 | ✅ |
| libarchive | ✅ 已安装 | ✅ |
| zlib | ✅ 已安装 | ✅ |
| Lua | ❌ 未安装 | ⚠️ 使用内嵌源码，见 P1-WARN-2 |
| 项目本地 vcpkg_installed | ✅ `vcpkg_installed/x64-windows/` | ✅ |

**vcpkg.json 依赖完整性**：

| 依赖 | 版本 | CMake 查找方式 | 状态 |
|------|------|---------------|------|
| qtbase | 6.x | `find_package(Qt6 ...)` | ✅ |
| qttools | 6.x | `find_package(Qt6 LinguistTools)` | ✅ |
| sdl2 | 2.x | `find_package(SDL2 CONFIG)` | ✅ |
| libarchive | x.x | `find_package(LibArchive)` | ✅ |
| zlib | x.x | `find_package(ZLIB)` | ✅ |
| liblzma | x.x | — | ✅ (libarchive 传递依赖) |

**结论**：vcpkg 工具链完整，项目依赖已全部安装。CMake 的 vcpkg 集成逻辑正确（优先使用 `vcpkg_installed/x64-windows`，回退到 `$VCPKG_ROOT` toolchain）。

---

## 四、各 Lua 库实现完成度详解

### 4.1 完成度矩阵

| 库 | 计划优先级 | C++ 行数 | Rust 行数 | 函数级完成度 | 关键缺失 |
|----|-----------|---------|----------|-------------|---------|
| `bit` | P0 | 200 | 150 | **100%** | 无 |
| `emu` | P1 | 450 | 172 | **75%** | frameadvance 未实现协程 yield/resume |
| `memory` | P1 | 225 | 90 | **55%** | registerwrite/read/exec 为空壳（无 TieredRegion 集成） |
| `joypad` | P1 | 124 | 83 | **90%** | getdown/getup/getimmediate 未实现 |
| `rom` | P1 | 58 | 82 | **80%** | gethash 返回 CRC32 非 MD5；readbytebig/little 未实现 |
| `movie` | P2 | 233 | 151 | **85%** | record/play/replay 未实现 |
| `savestate` | P2 | 275 | 125 | **60%** | persist 为空壳；registersave/load 的 RegistryKey 被立即丢弃 |
| `gui` | P2 | 1305 | 245 | **80%** | register/gdscreenshot/gdoverlay/parsecolor/gdpixel 未实现 |
| `input` | P2 | 188 | 52 | **15%** | 全部为 TODO 桩，无实际键盘/鼠标状态读取 |
| `ppu` | P2 | 20 | 34 | **90%** | 仅缺写操作（C++ 原版也没有） |
| `sound` | P3 | 148 | 90 | **95%** | 基本完整 |
| `zapper` | P3 | 70 | 47 | **100%** | 无 |
| `debugger` | P3 | 63 | 68 | **100%** | 无 |

### 4.2 致命功能缺失

#### 🔴 `emu.frameadvance` — 协程 yield/resume 未实现

这是 TAS 脚本的核心函数，占 99% 脚本的使用频率。

**C++ 实现**（`lua-engine.cpp:480–493`）：
```cpp
static int emu_frameadvance(lua_State *L) {
    frameAdvanceWaiting = TRUE;
    return lua_yield(L, 0);  // 挂起协程
}
```

**C++ 恢复**（`lua-engine.cpp:6187–6255`）：
```cpp
void FCEU_LuaFrameBoundary() {
    lua_getfield(L, LUA_REGISTRYINDEX, frameAdvanceThread);
    lua_State *thread = lua_tothread(L, 1);
    int result = lua_resume(thread, 0);  // 恢复协程
    if (result == LUA_YIELD) → 脚本暂停在 frameadvance，正常
    else → 脚本结束或出错
}
```

**Rust 当前实现**（`emu.rs:97–101`）：
```rust
emu.set("frameadvance", lua.create_function(|_, ()| {
    Err::<(), _>(mlua::Error::CoroutineUnresumable)  // 错误！
})?);
```

**正确实现方案**：使用 `mlua` 的 `Thread::yield` 机制。`LuaEngine` 需要持有主协程的引用，在 `frameadvance` 中 yield，在 `frame_boundary()` 中 resume。

```rust
pub struct LuaEngine {
    lua: Lua,
    main_thread: mlua::Thread,       // 主协程
    running: bool,
    // ...
}

// emu.frameadvance 实现
lua.create_function(|lua, ()| {
    lua.create_function(|_, ()| mlua::Thread::yield::<()>(&lua, Vec::new()))
})?;

// frame_boundary 实现
pub fn frame_boundary(&mut self) -> mlua::Result<()> {
    let result = self.main_thread.resume::<()>(())?;
    // 处理结果...
}
```

#### 🔴 `memory.registerwrite/read/exec` — TieredRegion 未集成

C++ 的 `TieredRegion`（`lua-engine.cpp:2042–2362`）是性能关键路径，每帧被调用数千次。计划正确地决定将其保留在 C++ 侧，但 Rust 侧需要：
1. 通过 FFI 调用 C++ 的 `CalculateMemHookRegions` 和注册函数
2. 在 Rust 侧存储 `RegistryKey`，在 C++ 触发钩子时回调 Rust

**当前状态**：Rust 的 `registerwrite/read/exec` 是空壳（接受函数但不存储）。

**修复方案**：在 C++ 侧添加 FFI 桥接：

```cpp
// lua-engine.cpp FFI bridge 新增
extern "C" void fceux11_lua_register_mem_hook(
    uint32_t addr, int size, int hook_type,
    void (*callback)(uint32_t, int, uint32_t, int));
```

Rust 侧将 Lua callback 存入 `RegistryKey`，并将一个 Rust 函数指针传给 C++ 作为回调。

#### 🔴 `savestate.registersave/load` — RegistryKey 生命周期管理错误

**当前实现**（`savestate.rs:104–113`）：
```rust
savestate.set("registersave", lua.create_function(|lua, cb: Function| {
    let _key = lua.create_registry_value(cb)?;  // _key 立即被 Drop！
    Ok(())
})?);
```

`RegistryKey` 在函数返回时被 Drop，Lua GC 会回收该回调引用。需要将 `RegistryKey` 存入 `LuaEngine` 结构体中持久保存。

---

## 五、`src/lua-engine.cpp` 瘦身分析

### 5.1 当前代码构成（7186 行）

| 分类 | 行范围 | 行数 | 可迁移 Rust | 必须留 C++ |
|------|--------|------|-----------|-----------|
| 包含/平台守卫 | 1–78 | 78 | ✅ | — |
| CheckLua/DemandLua | 80–107 | 28 | — | ✅ (Win DLL 检测) |
| LuaSaveState 结构体 | 159–191 | 33 | — | ✅ |
| 全局状态变量 | 193–273 | 81 | ✅ (移至 LuaEngine) | — |
| FCEU_LuaOnStop | 316–342 | 27 | ✅ | — |
| 速度模式查询 | 350–386 | 37 | ✅ | — |
| emu 库函数 | 393–637 | 245 | ✅ | — |
| **LuaSaveData 序列化** | 762–1328 | 567 | — | ✅ 不可迁移 |
| 保存/加载回调 | 1335–1427 | 93 | — | ✅ (依赖序列化) |
| rom 库 | 1430–1487 | 58 | ✅ | — |
| memory 基础操作 | 1489–1552 | 64 | ✅ | — |
| ppu 库 | 1554–1573 | 20 | ✅ | — |
| 调试打印基础设施 | 1575–1824 | 250 | ✅ | — |
| 工具函数 | 1829–1889 | 61 | ✅ | — |
| CPU 寄存器访问 | 1891–1985 | 95 | ✅ | — |
| 错误处理 | 1988–2031 | 44 | ✅ | — |
| **TieredRegion 钩子** | 2042–2362 | 321 | — | ✅ 性能关键 |
| s_keyToName | 2366–2466 | 101 | — | ✅ Win 专属 |
| input 库 | 2477–2664 | 188 | ✅ | — |
| zapper 库 | 2666–2735 | 70 | ✅ | — |
| joypad 库 | 2737–2860 | 124 | ✅ | — |
| savestate 库 | 2862–3136 | 275 | ✅ (部分) | ✅ 序列化留 C++ |
| emu 查询函数 | 3139–3180 | 42 | ✅ | — |
| movie 库 | 3182–3414 | 233 | ✅ | — |
| gui 库 | 3417–4722 | 1306 | ✅ | — |
| sound 库 | 4724–4871 | 148 | ✅ | — |
| debugger 库 | 4873–4935 | 63 | ✅ | — |
| **taseditor 库** | 4937–5235 | 299 | — | ✅ 平台耦合 |
| **cdlog 库** | 5237–5310 | 74 | — | ✅ Win 专属 |
| 对话框/UI | 5314–5588 | 275 | — | ✅ 平台耦合 |
| bit 库 (LuaBitOp) | 5590–5790 | 201 | ✅ | — |
| Hook 函数 | 5792–5839 | 48 | ✅ | — |
| 执行限制 | 5841–5910 | 70 | ✅ | — |
| 库注册表 | 5912–6165 | 254 | ✅ (Rust 自动注册) | — |
| CallExitFunction | 6167–6185 | 19 | ✅ | — |
| FrameBoundary | 6187–6255 | 69 | ✅ (需重写) | — |
| LoadLuaCode | 6264–6434 | 171 | ✅ (部分) | — |
| 其他入口 | 6436–6670 | 235 | ✅ (部分) | — |
| **FFI 桥接函数** | 6672–7186 | 515 | — | ✅ 必须留 C++ |

### 5.2 瘦身目标

| 场景 | C++ 保留行数 | 削减比例 |
|------|-------------|---------|
| 全部 Rust 化后 | ~1,850 行 | **74%** 削减 |
| 仅移除已 Rust 化的库 | ~2,400 行 | **67%** 削减 |
| 保守方案（保留更多回退） | ~3,200 行 | **55%** 削减 |

**1,850 行 C++ 残留构成**：

| 残留模块 | 行数 | 原因 |
|---------|------|------|
| FFI 桥接函数 | 515 | Rust 调用 C++ 的接口层 |
| LuaSaveData 序列化 | 567 | 依赖 Lua 栈内部格式，不可迁移 |
| TieredRegion 钩子 | 321 | 性能关键路径 |
| taseditor 库 | 299 | Qt/Win 前端耦合 |
| 对话框/UI + cdlog | 349 | 平台耦合 |
| 保存/加载回调 | 93 | 依赖序列化 |
| CheckLua + 杂项 | ~100 | Win DLL 检测等 |

---

## 六、可操作性修复清单（按优先级排序）

### Phase 1：修复阻断性问题（预计 1–2 天）

| # | 任务 | 涉及文件 | 类型 |
|---|------|---------|------|
| 1.1 | 根 crate 添加 `pub use fceux11_lua;` | `src/rust/src/lib.rs` | 编译修复 |
| 1.2 | 添加 `FCEUX11_LUA_RUST_ENABLED` CMake 选项 + `#ifdef` 守卫 | `src/CMakeLists.txt`、`src/lua-engine.cpp` | 编译修复 |
| 1.3 | 当 `LUA_RUST_ENABLED=ON` 时，排除 `src/lua/*.c` 从编译 | `src/CMakeLists.txt` | 链接修复 |
| 1.4 | 创建 `.cargo/config.toml`，锁定 MSVC 目标 | `src/rust/.cargo/config.toml` | 构建稳定 |
| 1.5 | 验证 `cargo build --workspace` 和 `cmake --build` 均可通过 | — | 集成验证 |

### Phase 2：实现核心运行时（预计 3–5 天）

| # | 任务 | 涉及文件 | 类型 |
|---|------|---------|------|
| 2.1 | **实现 `emu.frameadvance` 协程 yield/resume** | `lib.rs`、`bindings/emu.rs` | 核心功能 |
| 2.2 | **实现 `LuaEngine.frame_boundary()` 协程恢复** | `lib.rs` | 核心功能 |
| 2.3 | **修复 `savestate.registersave/load` 的 RegistryKey 生命周期** | `bindings/savestate.rs`、`lib.rs` | 功能修复 |
| 2.4 | **修复 `emu.registerbefore/after/exit` 的 RegistryKey 生命周期** | `bindings/emu.rs`、`lib.rs` | 功能修复 |
| 2.5 | 添加 C++ FFI：`fceux11_lua_register_mem_hook` | `lua-engine.cpp` | FFI 桥接 |
| 2.6 | 实现 `memory.registerwrite/read/exec` 通过 FFI 桥接 | `bindings/memory.rs` | 功能实现 |
| 2.7 | 实现 `LuaEngine.call_registered()` 回调执行 | `lib.rs` | 功能实现 |
| 2.8 | 实现 `LuaEngine.call_mem_hook()` 内存钩子回调 | `lib.rs` | 功能实现 |

### Phase 3：补全库函数（预计 3–5 天）

| # | 任务 | 涉及文件 | 类型 |
|---|------|---------|------|
| 3.1 | 实现 `input.get()` — 添加 `fceux11_lua_get_input_state` FFI | `bindings/input.rs`、`lua-engine.cpp` | 功能实现 |
| 3.2 | 实现 `input.popup()` / `openfilepopup()` / `savefilepopup()` | `bindings/input.rs`、`lua-engine.cpp` | 功能实现 |
| 3.3 | 实现 `joypad.getdown/getup/getimmediate` | `bindings/joypad.rs`、`lua-engine.cpp` | 功能补全 |
| 3.4 | 实现 `gui.register` / `gdscreenshot` / `gdoverlay` | `bindings/gui.rs` | 功能补全 |
| 3.5 | 实现 `rom.gethash` 返回 MD5（与 C++ 兼容） | `bindings/rom.rs` | 兼容性 |
| 3.6 | 实现 `movie.record/play/replay` | `bindings/movie.rs`、`lua-engine.cpp` | 功能补全 |
| 3.7 | 实现 `emu.exec_count/time` 执行限制 | `lib.rs` | 功能实现 |

### Phase 4：集成测试与切换（预计 2–3 天）

| # | 任务 | 涉及文件 | 类型 |
|---|------|---------|------|
| 4.1 | lua-engine.cpp 中添加 `#ifdef FCEUX11_LUA_RUST_ENABLED` 分区裁剪 | `lua-engine.cpp` | 代码瘦身 |
| 4.2 | 测试 10+ 社区 TAS Lua 脚本 | `tests/lua_scripts/` | 回归验证 |
| 4.3 | 性能基准：帧率对比 C++ 版 | — | 性能验证 |
| 4.4 | 内存泄漏检测（1 小时运行） | — | 稳定性验证 |

---

## 七、风险提示

| 风险 | 严重度 | 说明 |
|------|--------|------|
| `mlua` 协程语义与原生 Lua 5.1 不一致 | 🔴 高 | `mlua` 的 `Thread::yield` / `resume` 行为需逐帧对比 C++ 的 `lua_yield` / `lua_resume`，特别关注错误传播和栈状态 |
| `gui_data` 像素格式不匹配 | 🟡 中 | C++ 使用索引色 XBuf（8 位），Rust 使用 ARGB8888（32 位）。`gui_overlay` 需做格式转换，当前实现直接 `copy_from_slice` 是错误的 |
| `savestate.persist` 依赖 `LuaSaveData` | 🟡 中 | Rust 侧的 `persist` 是空壳，但 C++ 实现依赖 `LuaStackToBinary`，需要 FFI 桥接 |
| IUP/CD/luasocket 扩展 | 🟡 中 | Windows 上 `lua-engine.cpp:6296–6311` 动态加载这些 C 扩展，Rust 侧无法处理，需通过 C++ 兼容层 |

---

## 八、结论

1. **计划完成度约 55%**：骨架已搭好，但核心运行时（协程、钩子、回调）为空壳，不具备实际运行 Lua 脚本的能力。
2. **编译流程有 3 个阻断性问题**：双 Lua VM 链接冲突、根 crate 未 re-export、无切换机制。必须先解决才能继续。
3. **工具链完整**：Windows SDK 10.0.26100.0 + vcpkg（含 Qt6/SDL2/libarchive/zlib）均正常。
4. **lua-engine.cpp 瘦身潜力**：从 7186 行可削减至 ~1,850 行（74% 削减），残留部分为 FFI 桥接、序列化、TieredRegion、taseditor/cdlog。
5. **建议优先级**：Phase 1（编译修复）→ Phase 2（frameadvance + 回调）→ Phase 3（库补全）→ Phase 4（切换 + 瘦身）。
