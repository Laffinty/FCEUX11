# FCEUX11 Lua 引擎 Rust 重构：技术选型、可行性评估与构建计划

> **制定日期**：2026-05-31
> **基线版本**：v0.2.22
> **依赖文档**：`lua_rust_migration_assessment.md`、`rust_refactor_plan_v0.2.12-v0.2.30.md`
> **核心目标**：将 `src/lua-engine.cpp`（6,669 行）的主体逻辑用 Rust 重构，保留 Lua 脚本 100% 向后兼容，无缝融入现有 Workspace 体系

---

## 一、技术选型

### 1.1 候选方案对比

| 维度 | 方案 A：`mlua` 绑定层 | 方案 B：`rhai` 替换引擎 | 方案 C：混合方案（mlua + C++ 兼容层） |
|------|----------------------|------------------------|--------------------------------------|
| **Lua 脚本兼容** | ✅ 100% 兼容（Lua 5.1 VM 不变） | ❌ 不兼容（JS/Rust 语法） | ✅ 100% 兼容 |
| **生态成熟度** | ✅ 1,896+ commits，被多个生产项目使用 | ✅ 4,433+ commits，但不同语言 | ✅ 同 mlua |
| **实现复杂度** | 中（~15 个库需逐个绑定） | 极高（需重写所有用户脚本） | 中低（优先绑定核心库，其余走 C++ 回退） |
| **与现有 Rust Workspace 兼容** | ✅ 直接新增 crate | ✅ 可新增 crate | ✅ 直接新增 crate |
| **与 rust_refactor_plan 冲突** | 无（新增 crate） | 无 | 无 |
| **安全性提升** | 高（Rust 绑定层消除 C++ 裸指针） | 最高（纯 Rust 无 FFI） | 高（关键路径 Rust 化） |
| **长期维护成本** | 低（Lua VM 稳定，绑定层不变） | 高（需维护双脚本引擎或迁移所有脚本） | 低（逐步替换，风险可控） |

### 1.2 选型决策：方案 C — 混合方案（mlua + C++ 兼容层）

**理由**：

1. **用户生态零破坏**：FCEUX 社区积累了大量 Lua 脚本（TAS、调试、自动化），方案 A/C 可保证 100% 兼容，方案 B 会令所有脚本失效。
2. **风险渐进可控**：方案 C 允许按模块逐步绑定，每个 Lua 库（`emu`、`memory`、`joypad` 等）独立迁移，单个失败不阻塞其他。
3. **与 v0.2.12–v0.2.30 计划无冲突**：新增 `fceux11-lua` crate 是纯增量操作，不修改任何已 Rust 化模块的代码或 FFI 接口。
4. **`mlua` 是唯一生产级选择**：Rust 生态中同时支持 Lua 5.1 和高级绑定的 crate 仅 `mlua` 一个。`rlua` 已停止维护；`tealr` 是 mlua 的上层封装，增加复杂度而无收益。

**`rhai` 的定位**：如果未来项目计划引入内置脚本编辑器或配置 DSL，`rhai` 是优秀的候选。但在 v0.2.22.x 阶段，不应将 `rhai` 作为 Lua 替代方案引入。

### 1.3 `mlua` 关键特性确认

| 特性 | 详情 | FCEUX 需求匹配 |
|------|------|----------------|
| Lua 5.1 支持 | `lua51` feature flag | ✅ 必需（现有脚本基于 5.1） |
| LuaJIT 支持 | `luajit` feature flag | ⚠️ 未来可选（性能提升） |
| async/await | `async` feature flag | ❌ 不需要（同步模拟器） |
| userdata 绑定 | `UserData` trait | ✅ 用于暴露 FCEUX 类型 |
| 注册表操作 | `Lua::create_registry_value` | ✅ 用于回调注册 |
| 协程支持 | `Lua::create_thread` | ✅ 用于 `frameadvance` |
| MSVC 工具链 | ✅ 支持 | ✅ 项目已锁定 MSVC |
| 编译方式 | 内嵌 Lua 源码或链接系统库 | ✅ 内嵌（与现有 `src/lua/` 并行） |

### 1.4 `mlua` vs 内嵌 Lua C API 直接绑定

| 维度 | `mlua` | 直接 `lua.h` FFI |
|------|--------|------------------|
| 开发效率 | 高（Rust 风格 API，自动类型转换） | 低（手动管理 `lua_State`，大量 `unsafe`） |
| 安全性 | 高（`UserData` trait 保证类型安全） | 低（裸 `*mut lua_State`，无编译期检查） |
| 维护成本 | 低（mlua 处理 Lua 升级） | 高（每个 Lua 版本需手动更新 FFI） |
| 编译时间 | 略高（mlua 过程宏） | 略低 |
| 控制粒度 | 中（受限 mlua 抽象） | 最高（完全控制） |

**决策**：使用 `mlua`。直接 `lua.h` FFI 仅在 mlua 无法满足的极端场景下考虑（如需访问 `lstate.h` 内部结构），但当前需求不涉及。

---

## 二、可行性评估

### 2.1 技术可行性：✅ 高

#### 2.1.1 架构兼容性

```
当前架构：
  用户 Lua 脚本 → Lua 5.1 VM (src/lua/) → lua-engine.cpp (C++ 绑定) → FCEUX 核心

目标架构：
  用户 Lua 脚本 → Lua 5.1 VM (mlua 内嵌) → Rust 绑定层 (fceux11-lua) → FCEUX 核心 (Rust 已迁移模块)
                                                           ↕ FFI (C ABI)
                                                     兼容层 (lua-engine.cpp 薄壳)
                                                           ↕
                                                     FCEUX 核心 (C++ 未迁移模块)
```

**关键约束**：
- 当前 `lua-engine.cpp` 使用**单一全局 `lua_State*`**，同一时刻仅运行一个脚本。这与 `mlua` 的 `Lua` 实例一一对应，无并发冲突。
- 协程（`lua_newthread` / `lua_resume`）对应 `mlua` 的 `Lua::create_thread` / `Thread::resume`，天然映射。
- 回调注册表（`LUA_REGISTRYINDEX`）对应 `mlua` 的 `RegistryKey` / `create_registry_value`，语义等价。

#### 2.1.2 各 Lua 库迁移可行性

| Lua 库 | 依赖的 FCEUX 模块 | 已 Rust 化 | 迁移难度 | 优先级 |
|--------|-------------------|-----------|----------|--------|
| `bit` | 无 | N/A | ★☆☆ 极低 | P0（热身） |
| `emu` | fceu, video, debug, cheat, movie, x6502, driver, state, ppu | 部分 | ★★★ 中 | P1 |
| `memory` | x6502 (GetMem, BWrite, _PC/_A/_X/_Y/_S/_P), cheat | 部分 | ★★★ 中 | P1 |
| `joypad` | input, fceu | 否 | ★★☆ 中低 | P1 |
| `movie` | movie | 否 | ★★☆ 中低 | P2 |
| `savestate` | state, file | 否 | ★★★ 中 | P2 |
| `gui` | video (XBuf), fceu, driver | 部分 (palette) | ★★★ 中 | P2 |
| `rom` | fceu (FCEU_ReadRomByte), utils/crc32 | 是 (crc32) | ★★☆ 中低 | P1 |
| `ppu` | ppu (FFCEUX_PPURead) | 否 | ★★☆ 中低 | P2 |
| `sound` | sound (ENVUnits, PSG, etc.) | 否 | ★★☆ 中低 | P3 |
| `input` | fceu, driver | 否 | ★★☆ 中低 | P2 |
| `zapper` | movie, fceu | 否 | ★☆☆ 低 | P3 |
| `debugger` | debug, debugsymboltable | 部分 (asm, conddebug) | ★★☆ 中低 | P3 |
| `taseditor` | drivers/Qt/TasEditor | 否 | ★★★★ 高 | P4（平台耦合） |
| `cdlog` | drivers/win/cdlogger | 否 | ★★★★ 高 | P4（Windows 专属） |

**可行性结论**：
- **P0–P2 共 9 个库**（`bit`, `emu`, `memory`, `joypad`, `rom`, `movie`, `savestate`, `gui`, `input`, `ppu`）可独立迁移，覆盖 ~80% 的日常 TAS 脚本需求。
- **P3–P4 共 5 个库**（`sound`, `zapper`, `debugger`, `taseditor`, `cdlog`）涉及未 Rust 化的深层模块或平台专属代码，初期保留 C++ 回退路径。

#### 2.1.3 编译与构建可行性

| 因素 | 评估 |
|------|------|
| `mlua` + Lua 5.1 编译 | ✅ `mlua` 支持 `lua51` feature，内嵌 Lua 5.1 源码编译，无需外部 Lua 库 |
| MSVC 兼容 | ✅ `mlua` 官方支持 `x86_64-pc-windows-msvc` |
| Workspace 集成 | ✅ 新增 `fceux11-lua` crate，root crate re-export |
| cbindgen 集成 | ✅ `fceux11-lua` 导出的 FFI 函数由 cbindgen 自动生成头文件 |
| CMake 链接 | ✅ 合并进 `fceux11_rust.lib`，无需额外链接步骤 |

#### 2.1.4 不可行场景及应对

| 场景 | 影响 | 应对 |
|------|------|------|
| `mlua` 不支持访问 `lstate.h` 内部 | 无法实现 `LuaStackToBinary` 自定义序列化 | 保留 C++ 侧 `LuaSaveData` 序列化逻辑，不迁移至 Rust |
| `TieredRegion` 内存钩子需每帧调用 | Rust 绑定层增加 FFI 开销 | 将 `TieredRegion` 逻辑保留在 C++，Rust 侧仅注册/查询钩子 |
| `taseditor` / `cdlog` 深度耦合 Qt/Win 前端 | Rust 侧无法访问前端 API | 不迁移这两个库，永久保留 C++ 实现 |

### 2.2 进度可行性：✅ 中高

| 阶段 | 预计版本 | 工作量估计 | 关键路径 |
|------|---------|-----------|---------|
| 基础设施搭建 | v0.2.22.1 | 2–3 天 | Cargo.toml + mlua 集成 + 编译验证 |
| P0：`bit` 库 | v0.2.22.1 | 0.5 天 | 纯计算，零依赖 |
| P1：`emu` + `memory` + `joypad` + `rom` | v0.2.22.2–v0.2.22.3 | 5–7 天 | x6502 FFI 桥接、input 状态传递 |
| P2：`movie` + `savestate` + `gui` + `input` + `ppu` | v0.2.22.4–v0.2.22.5 | 7–10 天 | state 序列化、XBuf 像素缓冲区 |
| P3：`sound` + `zapper` + `debugger` | v0.2.22.6 | 3–5 天 | sound 状态结构映射 |
| P4：平台专属库 | v0.2.22.7+ | 按需 | taseditor/cdlog 需 Qt/Win 专项 |

**总计**：约 18–26 天（1 人全职），覆盖 P0–P3 全部 14 个库。P4 按需推进。

### 2.3 与 v0.2.12–v0.2.30 计划的兼容性

| 维度 | 评估 |
|------|------|
| FFI 接口冲突 | ❌ 无冲突。`fceux11-lua` 新增独立 FFI 函数，不修改现有 `fceux11_rust.h` 中任何签名 |
| Workspace 结构冲突 | ❌ 无冲突。新增 `crates/fceux11-lua/` 目录，不移动任何现有 crate |
| 依赖冲突 | ❌ 无冲突。`mlua` 仅被 `fceux11-lua` 引用，不传播到其他 crate |
| 构建顺序冲突 | ❌ 无冲突。`fceux11-lua` 依赖 `fceux11-utils`（CRC32 等），但无循环依赖 |
| 版本号冲突 | ⚠️ 需注意。Lua 重构在 v0.2.22.x 版本线推进，与 v0.2.23–v0.2.30 并行开发 |

**时序说明**：Lua Rust 重构与 v0.2.23–v0.2.30 的模块迁移可并行推进。理由：
1. Workspace 拆分和 cbindgen 集成在 v0.2.13–v0.2.14 已完成，`fceux11-lua` 可直接利用。
2. `fceux11-lua` 是纯增量 crate，不修改任何现有模块的 FFI 接口，不存在冲突。
3. 部分 Lua 库依赖的模块（`state`、`movie`、`input`）尚未 Rust 化，初期通过 FFI 直接调用 C++ 函数，后续模块 Rust 化后可逐步缩短调用链。

---

## 三、安全性评估

### 3.1 FFI 边界安全

| 风险点 | 严重度 | 缓解措施 |
|--------|--------|----------|
| **Lua → Rust 回调中的 `panic`** | 🔴 高 | `mlua` 默认将 Rust panic 转为 Lua error（`mlua::Error::externalError`），不会跨 FFI 边界 unwind。所有 `#[lua_module]` 导出函数必须使用 `Result<T, mlua::Error>` 返回值，禁止隐式 panic。 |
| **裸指针穿过 FFI 边界** | 🟡 中 | `memory.readbyte` 等函数接收 CPU 地址（`u16`），Rust 侧通过 FFI 调用 C++ `GetMem()` 获取值，不直接解引用模拟器内存指针。`gui.pixel` 等函数接收 `XBuf` 指针 + 尺寸参数，使用 `FceuSliceMut` + `slice::from_raw_parts_mut` 做边界检查。 |
| **`lua_State` 生命周期** | 🟡 中 | `mlua` 的 `Lua` 类型持有 `lua_State*` 的独占所有权。通过 `Lua::create_registry_value` 管理回调引用，避免手动操作 `LUA_REGISTRYINDEX`。Rust 侧不缓存 `Lua` 引用，每次通过 C++ 传入的 `Lua&` 操作。 |
| **缓冲区溢出** | 🟡 中 | 所有向 Lua 返回的字符串使用 `mlua` 的 `String` 类型（带长度），不使用 `const char*` + 隐含 `\0`。所有接收缓冲区的 FFI 函数使用 `FceuSliceMut` 携带长度。 |
| **整数溢出** | 🟢 低 | NES 地址空间固定 16 位（`u16`），不可能溢出。Lua 侧传入的整数通过 `mlua` 的 `i64` → `u16` 转换，Rust 侧做范围检查（`try_into()`）。 |

### 3.2 内存安全

| 风险点 | 严重度 | 缓解措施 |
|--------|--------|----------|
| **Rust ownership vs Lua GC** | 🟡 中 | `mlua` 通过 `UserData` trait 管理 Rust 对象在 Lua 侧的生命周期：Lua GC 触发 `__gc` 元方法时，`mlua` 自动 `Drop` Rust 对象。关键规则：**Rust 侧不得持有 Lua 对象的引用超过一次 `lua_resume`/`lua_call` 调用**——所有长生命周期引用必须存入 `RegistryKey`。 |
| **`gui_data` 像素缓冲区** | 🟡 中 | `gui_data` 是 256×240×4 = 245,760 字节的 ARGB8888 缓冲区。Rust 侧通过 `FceuSliceMut` 接收，每次 `FCEU_LuaGui` 调用时传入，不缓存指针。Lua 脚本在 `gui.pixel` 等函数中传递坐标，Rust 侧做 `x < 256 && y < 240` 边界检查。 |
| **`TieredRegion` 内存钩子** | 🟢 低 | `TieredRegion` 结构纯数据（三个 `std::map<uint, ...>`），不涉及指针。若迁移至 Rust，使用 `BTreeMap<u32, Vec<RegistryKey>>` 替代，生命周期由 `Lua` 实例管理。 |
| **`LuaSaveData` 自定义序列化** | 🔴 高 | `LuaStackToBinary` / `BinaryToLuaStack` 直接操作 Lua 栈内部结构（`LUA_TNIL`、`LUA_TNUMBER`、`LUA_TSTRING` 等），依赖 Lua 5.1 的内部类型标签。**建议不迁移此逻辑**，保留在 C++ 侧，避免跨版本兼容风险。 |

### 3.3 供应链安全

| 风险点 | 严重度 | 缓解措施 |
|--------|--------|----------|
| **`mlua` crate 供应链** | 🟡 中 | `mlua` 是 Rust 生态中 Lua 绑定的事实标准，1,896+ commits，活跃维护者（khvzak）。使用 `cargo audit` 持续监控漏洞。建议锁定 `mlua = "0.10"` 具体小版本。 |
| **内嵌 Lua 5.1 源码** | 🟢 低 | `mlua` 的 `lua51` feature 内嵌标准 Lua 5.1.5 源码（与 `src/lua/` 中的 5.1.4 仅差补丁版本）。Lua 5.1 已冻结，无安全更新。 |
| **可选依赖（IUP、CD、luasocket）** | 🟡 中 | 当前 `lua-engine.cpp` 在 Windows 上加载 `lua51.dll` 中的 IUP/CD/luasocket 扩展。`mlua` 绑定层**不加载这些扩展**，它们通过 C++ 兼容层的 `luaL_requiref` 单独注册。如需在 Rust 侧支持，需额外评估这些 C 库的安全性。 |

### 3.4 安全性综合评级

| 模块 | 安全性评级 | 说明 |
|------|-----------|------|
| `bit` 库 | 🟢 安全 | 纯计算，零 FFI |
| `emu` 库 | 🟡 中等 | 需 FFI 调用多个 C++ 全局函数，但通过参数化避免直接全局变量访问 |
| `memory` 库 | 🟡 中等 | CPU 地址空间读取需 FFI 回调 C++ `GetMem()`，写入需边界检查 |
| `joypad` 库 | 🟡 中等 | 输入覆盖需原子性保证（Rust 侧用 `Mutex<JoypadState>`） |
| `gui` 库 | 🟡 中等 | 像素缓冲区写入需边界检查；ARGB 颜色解析需校验 |
| `savestate` 库 | 🔴 需关注 | 序列化逻辑保留 C++；Rust 侧仅做文件 I/O 和压缩 |
| `movie` 库 | 🔴 需关注 | 录像同步是致命 bug，需 round-trip 测试 |
| `taseditor` / `cdlog` | 🔴 不迁移 | 平台耦合 + 前端 GUI 依赖 |

---

## 四、详细构建计划

### 4.1 版本规划总览

```
v0.2.22 ─── v0.2.22.1 ─── v0.2.22.2 ─── v0.2.22.3 ─── ... ─── v0.2.22.7
   │             │              │              │                     │
  当前      基础设施+bit    emu/memory    gui/savestate          平台专属
             + 编译验证      joypad/rom   movie/input
```

**前置条件**：v0.2.22 已发布，Workspace 拆分和 cbindgen 集成已就绪。

### 4.2 v0.2.22.1 — 基础设施搭建（预计 2–3 天）

#### 4.2.1 创建 `fceux11-lua` crate

```
src/rust/crates/fceux11-lua/
├── Cargo.toml
├── cbindgen.toml
└── src/
    ├── lib.rs           # crate 入口 + FFI 导出
    ├── engine.rs        # Lua 引擎生命周期管理
    ├── slice_ext.rs     # FceuSlice 在 Lua 上下文中的扩展
    └── bindings/
        ├── mod.rs
        ├── bit.rs       # P0
        ├── emu.rs       # P1
        ├── memory.rs    # P1
        ├── joypad.rs    # P1
        ├── rom.rs       # P1
        ├── movie.rs     # P2
        ├── savestate.rs # P2
        ├── gui.rs       # P2
        ├── input.rs     # P2
        ├── ppu.rs       # P2
        ├── sound.rs     # P3
        ├── zapper.rs    # P3
        └── debugger.rs  # P3
```

#### 4.2.2 Cargo.toml 配置

```toml
[package]
name = "fceux11-lua"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["rlib"]

[dependencies]
fceux11-utils = { path = "../fceux11-utils" }
mlua = { version = "0.10", features = ["lua51", "vendored", "module"] }

[dev-dependencies]
mlua = { version = "0.10", features = ["lua51", "vendored"] }
```

**feature 说明**：
- `lua51`：使用 Lua 5.1（与现有脚本兼容）
- `vendored`：内嵌编译 Lua 5.1 源码（无需系统 Lua 库）
- `module`：支持 `#[lua_module]` 宏自动生成 `luaopen_*` 入口

#### 4.2.3 根 crate 集成

修改 `src/rust/Cargo.toml`：

```toml
[dependencies]
fceux11-utils = { path = "crates/fceux11-utils" }
fceux11-media = { path = "crates/fceux11-media" }
fceux11-formats = { path = "crates/fceux11-formats" }
fceux11-debug = { path = "crates/fceux11-debug" }
fceux11-lua = { path = "crates/fceux11-lua" }  # 新增
```

#### 4.2.4 引擎生命周期管理（`engine.rs`）

```rust
use mlua::Lua;

pub struct LuaEngine {
    lua: Lua,
    thread: mlua::Thread,
    gui_data: Vec<u8>,  // 256 * 240 * 4 ARGB
    joypad_state: JoypadOverride,
    speed_mode: SpeedMode,
    transparency_modifier: u8,
}

pub enum SpeedMode {
    Normal,
    NoThrottle,
    Turbo,
    Maximum,
}

pub struct JoypadOverride {
    mask1: [u32; 4],  // pass-through bits
    mask2: [u32; 4],  // override bits
}

impl LuaEngine {
    pub fn new() -> Result<Self, mlua::Error> {
        let lua = Lua::new_with(mlua::StdLib::ALL, mlua::LuaOptions::empty())?;
        let thread = lua.create_thread(lua.create_function(|_, ()| Ok(()))?)?;
        // 注册所有 FCEUX 绑定库...
        Ok(Self {
            lua,
            thread,
            gui_data: vec![0u8; 256 * 240 * 4],
            joypad_state: JoypadOverride::default(),
            speed_mode: SpeedMode::Normal,
            transparency_modifier: 255,
        })
    }

    pub fn load_script(&mut self, path: &str, arg: Option<&str>) -> Result<(), mlua::Error> { ... }
    pub fn frame_boundary(&mut self) -> Result<(), mlua::Error> { ... }
    pub fn stop(&mut self) { ... }
    pub fn call_registered(&mut self, call_id: LuaCallID) -> Result<(), mlua::Error> { ... }
    pub fn call_mem_hook(&mut self, addr: u32, size: i32, value: u32, hook_type: LuaMemHookType) { ... }
    pub fn gui_overlay(&self, xbuf: &mut [u8], width: i32, height: i32) { ... }
    pub fn read_joypad(&self, controller: i32, original: u8) -> u8 { ... }
}
```

#### 4.2.5 FFI 桥接函数

```rust
// src/rust/crates/fceux11-lua/src/lib.rs

use std::ffi::{c_char, c_int, c_void};
use std::os::raw::c_uint;

static mut LUA_ENGINE: Option<Box<LuaEngine>> = None;

#[no_mangle]
pub extern "C" fn fceux11_lua_init() -> c_int {
    // 初始化 LuaEngine
}

#[no_mangle]
pub extern "C" fn fceux11_lua_load_script(path: *const c_char, arg: *const c_char) -> c_int {
    // 加载脚本
}

#[no_mangle]
pub extern "C" fn fceux11_lua_frame_boundary() {
    // 帧边界回调
}

#[no_mangle]
pub extern "C" fn fceux11_lua_stop() {
    // 停止引擎
}

#[no_mangle]
pub extern "C" fn fceux11_lua_running() -> c_int {
    // 查询运行状态
}

#[no_mangle]
pub extern "C" fn fceux11_lua_gui(xbuf: *mut u8, width: c_int, height: c_int) {
    // GUI 叠加
}

#[no_mangle]
pub extern "C" fn fceux11_lua_read_joypad(controller: c_int, original: u8) -> u8 {
    // 读取 joypad 覆盖
}
// ... 其他 fceulua.h 中声明的所有函数
```

#### 4.2.6 验收标准

| 验收项 | 标准 |
|--------|------|
| 编译通过 | `cargo build --workspace` 零错误零警告 |
| CMake 集成 | `cmake --build` 包含 `fceux11-lua` 编译步骤 |
| cbindgen | `fceux11_rust.h` 新增 `fceux11_lua_*` 函数声明 |
| 基础运行 | `fceux11_lua_init()` + `fceux11_lua_stop()` 可成功调用（无脚本加载） |

---

### 4.3 v0.2.22.1 — P0：`bit` 库 + P1 启动（预计 2 天）

#### 4.3.1 `bit` 库绑定

`bit` 库是纯计算模块，零 FCEUX 依赖，是验证 `mlua` 绑定流程的理想起点。

```rust
// src/rust/crates/fceux11-lua/src/bindings/bit.rs
use mlua::{Lua, Table, Result};

pub fn register(lua: &Lua) -> Result<Table> {
    let bit = lua.create_table()?;

    bit.set("tobit", lua.create_function(|_, x: i64| Ok(x as i32))?)?;
    bit.set("bnot", lua.create_function(|_, x: i32| Ok(!x))?)?;
    bit.set("band", lua.create_function(|_, (a, b): (i32, i32)| Ok(a & b))?)?;
    bit.set("bor", lua.create_function(|_, (a, b): (i32, i32)| Ok(a | b))?)?;
    bit.set("bxor", lua.create_function(|_, (a, b): (i32, i32)| Ok(a ^ b))?)?;
    bit.set("lshift", lua.create_function(|_, (x, n): (i32, i32)| Ok(x.wrapping_shl(n as u32)))?)?;
    bit.set("rshift", lua.create_function(|_, (x, n): (i32, i32)| Ok(x.wrapping_shr(n as u32)))?)?;
    bit.set("arshift", lua.create_function(|_, (x, n): (i32, i32)| Ok((x as i64 >> n) as i32))?)?;
    bit.set("rol", lua.create_function(|_, (x, n): (i32, i32)| Ok(x.rotate_left(n as u32)))?)?;
    bit.set("ror", lua.create_function(|_, (x, n): (i32, i32)| Ok(x.rotate_right(n as u32)))?)?;
    bit.set("bswap", lua.create_function(|_, x: i32| Ok(x.swap_bytes()))?)?;
    bit.set("tohex", lua.create_function(|_, (x, n): (i32, Option<i32>)| {
        let digits = n.unwrap_or(8);
        Ok(format!("{:0>width$X}", x as u32, width = digits as usize))
    })?)?;

    Ok(bit)
}
```

#### 4.3.2 `emu` 库绑定（部分）

`emu` 库是最大的 Lua 库，分两步：
- **v0.2.22.1**：实现无 FFI 依赖的函数（`framecount`、`lagcount`、`paused`、`message`、`print`）
- **v0.2.22.2**：实现需要 FFI 回调的函数（`poweron`、`softreset`、`frameadvance`、`speedmode`、`registerbefore`/`registerafter`）

#### 4.3.3 验收标准

| 验收项 | 标准 |
|--------|------|
| `bit` 库功能测试 | Lua 脚本 `bit.bor(0xFF, 0x00) == 0xFF` 等全部运算正确 |
| `emu.framecount` | 返回值与 C++ 实现一致 |
| 无回归 | 现有 114+ 单元测试全部通过 |
| 无链接警告 | CMake 构建零新增链接警告 |

---

### 4.4 v0.2.22.2 — P1 核心库（预计 3–4 天）

#### 4.4.1 `memory` 库绑定

```
Lua memory.readbyte(address) → Rust fceux11_lua_memory_readbyte(addr: u32) → FFI → C++ GetMem(addr)
Lua memory.writebyte(address, value) → Rust fceux11_lua_memory_writebyte(addr: u32, val: u8) → FFI → C++ BWrite[addr](addr, val)
Lua memory.getregister(name) → Rust fceux11_lua_memory_getregister(reg_id: i32) → FFI → C++ X [_PC/_A/_X/_Y/_S/_P]
Lua memory.registerwrite(addr, callback) → Rust 存入 RegistryKey → TieredRegion (C++ 侧)
```

**关键设计**：
- `GetMem` / `BWrite` 是 C++ 全局函数指针数组，Rust 侧不直接访问，通过 FFI 桥接函数调用。
- `memory.registerwrite` / `registerread` / `registerexec` 注册的回调存储在 Rust 侧的 `RegistryKey` 中，但 `TieredRegion` 的查找逻辑保留在 C++（因为它是每帧高频调用的热路径）。

#### 4.4.2 `joypad` 库绑定

```rust
pub fn register(lua: &Lua) -> Result<Table> {
    let joypad = lua.create_table()?;

    joypad.set("get", lua.create_function(|lua, (port, frame): (i32, Option<i32>)| {
        // 通过 FFI 获取 joypad 状态
        let state = unsafe { fceux11_lua_get_joypad_state(port) };
        Ok(state)
    })?)?;

    joypad.set("set", lua.create_function(|lua, (port, table): (i32, Table)| {
        // 解析 Lua table {A=true, B=false, ...} 为位掩码
        // 存入 JoypadOverride
        Ok(())
    })?)?;

    Ok(joypad)
}
```

#### 4.4.3 `rom` 库绑定

`rom` 库依赖 CRC32（已 Rust 化）和 `FCEU_ReadRomByte` / `FCEU_WriteRomByte`（C++ 全局函数），通过 FFI 桥接。

#### 4.4.4 验收标准

| 验收项 | 标准 |
|--------|------|
| `memory.readbyte(0x0000)` | 返回值与 C++ 版一致 |
| `memory.writebyte(0x0000, 0xFF)` | 写入成功，读取验证 |
| `joypad.set(1, {A=true})` | joypad 覆盖生效 |
| `rom.gethash()` | MD5 哈希与 C++ 版一致 |
| TAS 脚本冒烟测试 | 加载一个简单 TAS Lua 脚本，`emu.frameadvance` 正常工作 |

---

### 4.5 v0.2.22.3 — P1 完成 + P2 启动（预计 3–4 天）

#### 4.5.1 `emu` 库完整绑定

完成 `emu` 库剩余函数：
- `poweron` / `softreset`：FFI 调用 `FCEUI_ResetNES()` / `FCEUI_PowerNES()`
- `frameadvance`：`lua_yield` 当前协程（对应 `mlua` 的 `Thread::yield`）
- `speedmode`：设置 `SpeedMode` 枚举
- `registerbefore` / `registerafter` / `registerexit`：注册 `RegistryKey` 回调

#### 4.5.2 `movie` 库绑定

`movie` 库主要查询函数（`mode`、`recording`、`playing`、`length` 等），无状态修改，FFI 开销低。

#### 4.5.3 验收标准

| 验收项 | 标准 |
|--------|------|
| `emu.frameadvance()` | 帧推进正常，协程 yield/resume 正确 |
| `emu.speedmode("turbo")` | 速度模式切换生效 |
| `emu.registerbefore(callback)` | 回调在每帧前触发 |
| `movie.mode()` | 返回 "record"/"playback"/"finished" 正确值 |

---

### 4.6 v0.2.22.4 — P2 核心（预计 3–4 天）

#### 4.6.1 `savestate` 库绑定

**关键挑战**：`LuaSaveData` 自定义序列化（`LuaStackToBinary` / `BinaryToLuaStack`）依赖 Lua 5.1 栈内部格式。

**策略**：
- `savestate.save` / `savestate.load`：FFI 调用 C++ `FCEUSS_SaveMS` / `FCEUSS_LoadFP`
- `savestate.persist` / `registersave` / `registerload`：Rust 侧管理回调注册，但序列化逻辑保留 C++ `LuaSaveData`
- `savestate.create` / `savestate.object`：Rust 侧管理 `LuaSaveState` 生命周期

#### 4.6.2 `gui` 库绑定

```rust
pub fn register(lua: &Lua, gui_data: &[u8]) -> Result<Table> {
    let gui = lua.create_table()?;

    gui.set("pixel", lua.create_function(|_, (x, y, color): (i32, i32, Option<u32>)| {
        // 写入 gui_data[(y * 256 + x) * 4 .. (y * 256 + x) * 4 + 4]
        // 边界检查：0 <= x < 256, 0 <= y < 240
        Ok(())
    })?)?;

    gui.set("line", lua.create_function(|_, (x1, y1, x2, y2, color): (i32, i32, i32, i32, Option<u32>)| {
        // Bresenham 线段算法
        Ok(())
    })?)?;

    gui.set("text", lua.create_function(|_, (x, y, msg, color): (i32, i32, String, Option<u32>)| {
        // 像素字体渲染到 gui_data
        Ok(())
    })?)?;

    Ok(gui)
}
```

#### 4.6.3 验收标准

| 验收项 | 标准 |
|--------|------|
| `gui.pixel(0, 0, 0xFF0000)` | 左上角像素为红色 |
| `gui.text(10, 10, "hello")` | 文字正确渲染 |
| `savestate.save(1)` | 存档文件与 C++ 版格式兼容 |
| `savestate.load(1)` | 读档后游戏状态正确恢复 |

---

### 4.7 v0.2.22.5 — P2 完成（预计 3–4 天）

#### 4.7.1 `input` 库绑定

`input.get()` 返回当前按键状态，`input.popup()` 显示对话框。

#### 4.7.2 `ppu` 库绑定

`ppu.readbyte(address)` 和 `ppu.readbyterange(address, length)` 通过 FFI 调用 `FFCEUX_PPURead`。

#### 4.7.3 验收标准

| 验收项 | 标准 |
|--------|------|
| `input.get()` | 返回正确按键状态表 |
| `ppu.readbyte(0x2000)` | 返回 PPU 寄存器值 |
| 完整 TAS 脚本测试 | 加载 3 个真实 TAS Lua 脚本，全部功能正常 |

---

### 4.8 v0.2.22.6 — P3 库（预计 3–5 天）

#### 4.8.1 `sound` 库绑定

`sound.get()` 返回 APU 状态表。依赖 C++ `sound.cpp` 中的全局状态（`ENVUnits`、`PSG`、`curfreq` 等），全部通过 FFI 读取。

#### 4.8.2 `zapper` 库绑定

`zapper.read()` / `zapper.set()` 读取/设置光枪位置。通过 FFI 访问 `currMovieData` 中的 zapper 状态。

#### 4.8.3 `debugger` 库绑定

`debugger.hitbreakpoint()` / `debugger.getcyclescount()` 等函数通过 FFI 调用 C++ `debug.cpp` / `debugsymboltable.cpp`。

#### 4.8.4 验收标准

| 验收项 | 标准 |
|--------|------|
| `sound.get()` | 返回 APU 状态表，字段完整 |
| `zapper.read()` | 返回光枪坐标 |
| `debugger.getcyclescount()` | 返回周期计数 |
| 全部 14 个库可用 | P0–P3 所有 Lua 库功能等价 |

---

### 4.9 v0.2.22.7 — 平台专属库 + C++ 兼容层切换（按需）

#### 4.9.1 `taseditor` 库（仅 Qt/Win）

**策略**：不迁移。`taseditor` 的 20+ 个函数全部回调到 Qt/Win 前端，Rust 侧无法安全访问前端 API。保留 C++ 实现，通过 `lua-engine.cpp` 兼容层注册。

#### 4.9.2 `cdlog` 库（仅 Windows）

**策略**：不迁移。`cdlog` 依赖 `drivers/win/cdlogger.h`，是 Windows 专属功能。保留 C++ 实现。

#### 4.9.3 C++ 兼容层切换

在 `lua-engine.cpp` 中引入特性开关：

```cpp
#ifdef FCEUX11_LUA_RUST_ENABLED
    // Rust 绑定层已实现所有库，此处仅做薄封装
    // 调用 fceux11_lua_* FFI 函数
#else
    // 完整 C++ Lua 引擎（原有实现）
#endif
```

**切换策略**：
- 初期（v0.2.22.1–v0.2.22.6）：`FCEUX11_LUA_RUST_ENABLED=OFF`，Rust 绑定层与 C++ 引擎并存，Rust 侧仅用于测试。
- 成熟期（v0.2.22.7+）：`FCEUX11_LUA_RUST_ENABLED=ON`，Rust 绑定层接管主流程，C++ 侧仅保留 `taseditor`/`cdlog`/`LuaSaveData` 序列化。

#### 4.9.4 验收标准

| 验收项 | 标准 |
|--------|------|
| 特性开关 | `ON`/`OFF` 两种配置均编译通过 |
| Lua 脚本兼容 | 20 个社区 Lua 脚本全部运行正常 |
| 性能不退化 | 帧率不低于 C++ 版 95% |
| 内存不泄漏 | 长时间运行（1 小时）内存占用稳定 |

---

## 五、测试策略

### 5.1 测试分层

| 层级 | 覆盖范围 | 工具 | 频率 |
|------|---------|------|------|
| L1：Rust 单元测试 | 每个 Lua 库的绑定逻辑 | `cargo test` + `mlua` 内嵌 Lua | 每次提交 |
| L2：Lua 脚本集成测试 | 端到端 Lua 脚本执行 | `tests/lua_scripts/` | 每个版本 |
| L3：C++ 冒烟测试 | 混合构建运行 | CTest | 每个版本 |
| L4：社区脚本回归 | 20+ 真实 TAS/调试脚本 | 手动 + 自动化 | 每个里程碑 |

### 5.2 L1：单元测试示例

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn test_bit_bor() {
        let lua = Lua::new();
        let bit = register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: i32 = lua.load("bit.bor(0xFF, 0x00)").eval().unwrap();
        assert_eq!(result, 0xFF);
    }

    #[test]
    fn test_bit_lshift() {
        let lua = Lua::new();
        let bit = register(&lua).unwrap();
        lua.globals().set("bit", bit).unwrap();
        let result: i32 = lua.load("bit.lshift(1, 8)").eval().unwrap();
        assert_eq!(result, 256);
    }
}
```

### 5.3 L2：Lua 脚本集成测试

在 `tests/lua_scripts/` 下创建标准测试脚本：

```lua
-- test_bit.lua
assert(bit.bor(0xFF, 0x00) == 0xFF)
assert(bit.band(0xFF, 0x0F) == 0x0F)
assert(bit.lshift(1, 8) == 256)
assert(bit.rshift(256, 8) == 1)
assert(bit.bxor(0xFF, 0x0F) == 0xF0)
print("bit library: ALL PASSED")
```

```lua
-- test_memory.lua
local val = memory.readbyte(0x0000)
memory.writebyte(0x0000, 0x42)
assert(memory.readbyte(0x0000) == 0x42)
memory.writebyte(0x0000, val)  -- restore
print("memory library: ALL PASSED")
```

### 5.4 L4：社区脚本回归测试清单

| 脚本类别 | 最少覆盖数 | 关键验证点 |
|---------|-----------|-----------|
| TAS 自动化 | 5 | `emu.frameadvance`、`joypad.set`、`movie` 操作 |
| 内存监视 | 5 | `memory.registerwrite`、`memory.readbyte` |
| GUI 叠加 | 3 | `gui.pixel`、`gui.text`、`gui.line` |
| 调试辅助 | 3 | `debugger.hitbreakpoint`、`memory.getregister` |
| 存档管理 | 2 | `savestate.save`/`load`、`savestate.persist` |
| 音乐播放 | 2 | `sound.get()`（NSF 文件） |

---

## 六、风险缓解矩阵

| # | 风险 | 概率 | 影响 | 缓解措施 | 负责版本 |
|---|------|------|------|----------|---------|
| R1 | `mlua` Lua 5.1 feature 与内嵌 `src/lua/` 编译冲突 | 低 | 高 | `mlua` 使用 `vendored` feature 独立编译；不与 `src/lua/` 同时链接 | v0.2.22.1 |
| R2 | 协程 yield/resume 语义差异 | 中 | 高 | 逐帧对比 C++ 和 Rust 的 `lua_resume` 行为，编写专门测试 | v0.2.22.3 |
| R3 | `LuaSaveData` 序列化不兼容 | 高 | 高 | 不迁移此逻辑，永久保留 C++ | v0.2.22.4 |
| R4 | 回调注册表迁移丢失引用 | 中 | 中 | 双重验证：Rust `RegistryKey` 和 C++ `LUA_REGISTRYINDEX` 并行运行 | v0.2.22.3 |
| R5 | `TieredRegion` 内存钩子性能退化 | 低 | 中 | 保持 C++ 侧实现不变，Rust 侧仅注册/查询 | v0.2.22.2 |
| R6 | `mlua` 编译时间剧增 | 低 | 低 | Workspace 拆分隔离；`sccache` 缓存 | v0.2.22.1 |
| R7 | IUP/CD/luasocket 扩展在 Rust 侧不可用 | 中 | 低 | 保留 C++ 侧动态加载路径 | v0.2.22.7 |
| R8 | `taseditor`/`cdlog` 用户脚本因 Rust 切换失效 | 低 | 高 | 特性开关：Rust 切换后仍通过 C++ 兼容层注册这两个库 | v0.2.22.7 |

---

## 七、与 v0.2.12–v0.2.30 计划的集成时间线

```
v0.2.22 ─── v0.2.22.1 ─── v0.2.22.2 ─── ... ─── v0.2.22.7 ─── v0.2.23 ─── ... ─── v0.2.30
   │             │              │                     │            │
  当前      Lua 基础设施    emu/memory           平台专属      继续 Rust
             + bit绑定      joypad/rom          兼容层切换     模块迁移
```

**关键依赖**：
- v0.2.22.1 依赖已有的 Workspace 架构和 cbindgen 基础设施（v0.2.13–v0.2.14 已完成）
- v0.2.22.2 的 `memory` 库通过 FFI 直接调用 C++ `GetMem()`/`BWrite[]`，不依赖 `state.cpp` Rust 化
- v0.2.22.4 的 `savestate` 库通过 FFI 调用 C++ `FCEUSS_SaveMS`/`FCEUSS_LoadFP`，序列化逻辑保留 C++
- v0.2.22.5 的 `movie` 库通过 FFI 调用 C++ `FCEUMOV_*` 函数，不依赖 `movie.cpp` Rust 化

**如果 v0.2.23–v0.2.30 并行推进**：
- v0.2.22.1–v0.2.22.3 不受影响（仅依赖 Workspace + cbindgen，已完成）
- v0.2.22.4+ 的 `savestate`/`movie` 库始终通过 FFI 调用 C++ 函数，后续对应模块 Rust 化后可逐步缩短调用链

---

## 八、决策记录

| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|
| Lua 绑定 crate | `mlua` / `rlua` / 直接 FFI | `mlua` | 唯一活跃维护的 Lua 5.1 绑定 crate |
| Lua 版本 | 5.1 / 5.4 / LuaJIT | Lua 5.1 | 现有脚本 100% 兼容 |
| 编译方式 | vendored / 系统 Lua | vendored | 避免与 `src/lua/` 链接冲突 |
| `LuaSaveData` 序列化 | 迁移 Rust / 保留 C++ | 保留 C++ | 依赖 Lua 栈内部格式，迁移风险极高 |
| `TieredRegion` 内存钩子 | 迁移 Rust / 保留 C++ | 保留 C++ | 每帧高频调用，FFI 开销不可接受 |
| `taseditor`/`cdlog` | 迁移 / 不迁移 | 不迁移 | 平台专属 + 前端 GUI 耦合 |
| 启动时间 | v0.2.x / v0.2.22.x | v0.2.22.x | 与 v0.2.23–v0.2.30 并行推进，无前置依赖 |
| async/await | 启用 / 禁用 | 禁用 | 同步模拟器，无收益 |
