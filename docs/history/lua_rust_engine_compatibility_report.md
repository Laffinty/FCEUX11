# FCEUX11 Rust Lua 引擎 vs C++ Lua 引擎：兼容性检测报告

> **检测日期**：2026-06-02
> **基线版本**：v0.2.22.8（commit 7801acb）
> **检测范围**：Rust 侧 `fceux11-lua` crate 能否完全替代 C++ 侧 `lua-engine.cpp`，以及替代的前提条件和剩余差距

---

## 一、结论摘要

### Rust 侧能否完全替代 C++ 侧？

**结论：不能完全替代，但可替代约 74% 的 C++ 代码。**

Rust 侧 Lua 引擎已实现 P0–P3 全部 13 个库的函数绑定骨架，核心运行时（协程 frameadvance、回调注册持久化、内存钩子）已从"空壳"升级为"可运行"。但存在以下不可替代的 C++ 残留：

| 类别 | 不可替代原因 | C++ 行数 |
|------|-------------|---------|
| FFI 桥接函数 | Rust → C++ 的接口层，永久需要 | ~515 |
| LuaSaveData 序列化 | 依赖 Lua 5.1 栈内部格式（lstate.h） | ~567 |
| TieredRegion 内存钩子 | 每帧高频调用热路径，FFI 开销不可接受 | ~321 |
| taseditor 库 | Qt/Win 前端 GUI 深度耦合 | ~299 |
| cdlog 库 | Windows 专属 | ~74 |
| 对话框/UI + s_keyToName | 平台耦合（Win32 API / Qt） | ~376 |
| 保存/加载回调 | 依赖 LuaSaveData 序列化 | ~93 |

**总计 C++ 不可替代行数：~2,245 行（占 7,306 行的 31%）**

**可削减 C++ 代码：~5,061 行（69%）**，若采用保守方案仍可削减 ~4,100 行（56%）。

---

## 二、逐库函数级对比与完成度

### 2.1 总览矩阵

| # | Lua 库 | C++ 函数数 | Rust 函数数 | 完成度 | 可替代 C++ | 关键缺失/差异 |
|---|--------|-----------|------------|--------|-----------|-------------|
| 1 | `bit` | 12 | 12 | **100%** | ✅ 完全 | 无 |
| 2 | `emu` | 20 | 15 | **75%** | ⚠️ 部分 | 缺 `loadrom`/`exit`/`getdir`/`setrenderplanes`/`addgamegenie`/`delgamegenie`/`exec_count`/`exec_time`/`debuggerloop`/`debuggerloopstep` |
| 3 | `memory` | 12 | 8 | **67%** | ⚠️ 部分 | 缺 `readbyterange`/`setregister`/`writebyte` 不支持 word |
| 4 | `joypad` | 6 | 2 | **33%** | ⚠️ 部分 | 缺 `getdown`/`getup`/`getimmediate`；`get` 返回裸数值而非 table |
| 5 | `rom` | 6 | 3 | **50%** | ⚠️ 部分 | 缺 `readbytesigned`/`readbyterange`/`getfilename`；`gethash` 返回 CRC32 非 MD5 |
| 6 | `movie` | 17 | 13 | **76%** | ⚠️ 部分 | 缺 `record`/`play`/`replay`/`rerecordcounting` |
| 7 | `savestate` | 9 | 7 | **78%** | ⚠️ 部分 | `persist` 为空壳；缺 `loadscriptdata`/`rerecordcounting` |
| 8 | `gui` | 16 | 11 | **69%** | ⚠️ 部分 | 缺 `gdscreenshot`/`gdoverlay`/`parsecolor`/`fillbox`/`setopacity`(旧名) |
| 9 | `input` | 4 | 4 | **15%** | ❌ 桩函数 | `get()` 返回空表；`popup`/`openfilepopup`/`savefilepopup` 为 TODO 桩 |
| 10 | `ppu` | 2 | 2 | **100%** | ✅ 完全 | 无 |
| 11 | `sound` | 1 | 1 | **95%** | ✅ 基本完全 | 返回结构完整，部分字段映射可能需校准 |
| 12 | `zapper` | 2 | 2 | **100%** | ✅ 完全 | 无 |
| 13 | `debugger` | 6 | 6 | **100%** | ✅ 完全 | 无 |
| — | `taseditor` | 20 | 0 | **0%** | ❌ 不迁移 | Qt/Win 前端耦合 |
| — | `cdlog` | 8 | 0 | **0%** | ❌ 不迁移 | Windows 专属 |

### 2.2 致命功能差异详析

#### 🔴 差异 1：`joypad.get` 返回类型不兼容

| 维度 | C++ | Rust |
|------|-----|------|
| 返回值 | Lua table `{A=true, B=false, ...}` | 裸整数（位掩码） |
| 调用方式 | `local btn = joypad.get(1); if btn.A then ...` | `local state = joypad.get(1); if state & 0x01 then ...` |
| 影响 | **破坏所有使用 `joypad.get` 的社区脚本** | — |

**修复**：`joypad.get` 必须返回与 C++ 相同格式的 table。

#### 🔴 差异 2：`rom.gethash` 哈希算法不一致

| 维度 | C++ | Rust |
|------|-----|------|
| 算法 | MD5 | CRC32 |
| 返回值 | 32 字符十六进制字符串 | 8 字符十六进制字符串 |
| 影响 | **依赖 MD5 哈希匹配的脚本会失败** | — |

**修复**：使用 `fceux11-utils` 中已有的 MD5 实现替换 CRC32。

#### 🟡 差异 3：`input.get` 为空壳

C++ 的 `input.get()` 返回完整的键盘/鼠标状态表（含 `shift`/`control`/`alt`/鼠标坐标等）。Rust 侧返回空表。这是平台耦合功能，需要 FFI 桥接。

#### 🟡 差异 4：`emu.frameadvance` 协程语义

Rust 侧使用 `coroutine.yield()` 实现 frameadvance，这比之前 review 中的 `CoroutineUnresumable` 错误已有重大改进。但需要验证以下关键行为：

| 行为 | C++ | Rust | 验证状态 |
|------|-----|------|---------|
| yield 后协程状态 | `LUA_YIELD` | `ThreadStatus::Resumable` | ✅ 等价 |
| resume 后栈清洁度 | 栈已清空 | 需验证 | ⚠️ 待测 |
| 多次 frameadvance | 每次正确 yield/resume | 需验证 | ⚠️ 待测 |
| 脚本错误传播 | `lua_pcall` 捕获 | `mlua::Error` 捕获 | ⚠️ 待测 |
| 脚本结束检测 | `LUA_OK` / `LUA_ERRRUN` | `ThreadStatus::Finished/Error` | ✅ 等价 |

#### 🟡 差异 5：`gui_data` 像素格式

| 维度 | C++ | Rust |
|------|-----|------|
| 内部缓冲格式 | 索引色（8 位，NES palette 索引） | ARGB8888（32 位） |
| 绘制方式 | `gui_prepare()` 将索引色转换为 RGB | 直接写入 RGBA 值 |
| 叠加方式 | `blend32()` 做半透明混合 | 简单 alpha 判断（>=250 则覆盖） |
| 影响 | gui_overlay 混合效果可能不一致 | — |

---

## 三、编译与构建检测

### 3.1 Rust 侧编译检测结果

| 检测项 | 结果 | 说明 |
|--------|------|------|
| `cargo check --workspace` | ✅ 通过 | 1 个 warning（`speed_mode` 字段未读取） |
| `cargo build --workspace` | ✅ 通过 | — |
| `fceux11-lua` crate 编译 | ✅ 通过 | `mlua` vendored Lua 5.1 编译成功 |
| 根 crate re-export | ✅ 已修复 | `src/rust/src/lib.rs` 包含 `pub use fceux11_lua;` |

### 3.2 FFI 链接检测

| 检测项 | 结果 | 说明 |
|--------|------|------|
| `fceux11_rust.h` 包含 Lua FFI 声明 | ✅ | 行 896–1065，共 50+ 个 FFI 函数 |
| C++ 侧 FFI 桥接实现 | ✅ | `lua-engine.cpp:6866–7306`，`extern "C"` 块 |
| `cargo test -p fceux11-lua` | ❌ 链接失败 | 71 个未解析外部符号（C++ FFI 函数不在 Rust 测试链接中） |
| 非 Lua crate 单元测试 | ✅ | `fceux11-utils/media/formats/debug` 共 46 个测试通过 |

### 3.3 CMake 构建检测

| 检测项 | 结果 | 说明 |
|--------|------|------|
| `FCEUX11_LUA_RUST_ENABLED` 选项 | ✅ 存在 | `src/CMakeLists.txt:56`，默认 OFF |
| ON 时排除 `src/lua/*.c` | ✅ | `CMakeLists.txt:58–61` 条件编译 |
| ON 时编译 lua-engine.cpp | ✅ | 仅 FFI 桥接 + 序列化 + taseditor/cdlog |
| 双 Lua VM 链接冲突 | ✅ 已解决 | ON 时不编译内嵌 Lua C 源码 |
| `#ifdef FCEUX11_LUA_RUST_ENABLED` 守卫 | ✅ | `lua-engine.cpp:86` / `:225` / `:6851` |

---

## 四、测试方案与执行结果

### 4.1 测试分层设计

| 层级 | 覆盖范围 | 工具 | 执行状态 | 结果 |
|------|---------|------|---------|------|
| **L1**：Rust 单元测试 | `bit` 库纯计算逻辑 | `cargo test` | ⚠️ 受 FFI 链接限制无法独立运行 | 仅 cargo check 通过 |
| **L2**：Lua 脚本集成测试 | 各库 API 兼容性 | `tests/lua_scripts/*.lua`（12 个脚本） | 📋 已创建，需端到端运行 | 待执行 |
| **L3**：C++ 冒烟测试 | 混合构建运行 | CMake + 启动模拟器 | 📋 需手动执行 | 待执行 |
| **L4**：社区脚本回归 | 20+ 真实 TAS 脚本 | 手动 | 📋 待安排 | 待执行 |

### 4.2 L2 测试脚本清单

已创建 12 个测试脚本，覆盖全部 13 个 Lua 库 + 3 个集成场景：

| 脚本 | 覆盖库 | 测试点数 | 预期问题 |
|------|--------|---------|---------|
| `test_bit.lua` | bit | 22 | 无 |
| `test_emu.lua` | emu | 16 | 无 |
| `test_memory.lua` | memory | 15 | 无 |
| `test_joypad.lua` | joypad | 5 | `get` 返回类型不兼容 |
| `test_rom.lua` | rom | 5 | `gethash` 算法不一致 |
| `test_gui.lua` | gui | 16 | 无 |
| `test_movie.lua` | movie | 13 | 无 |
| `test_savestate.lua` | savestate | 8 | `persist` 为空壳 |
| `test_input.lua` | input | 4 | `get()` 返回空表 |
| `test_ppu.lua` | ppu | 4 | 无 |
| `test_sound.lua` | sound | 16 | 无 |
| `test_zapper.lua` | zapper | 2 | 无 |
| `test_debugger.lua` | debugger | 6 | 无 |
| `test_frameadvance.lua` | emu (协程) | 1 | 核心测试 |
| `test_callbacks.lua` | emu (回调) | 1 | 核心测试 |
| `test_tas_simulation.lua` | 多库集成 | 1 | 核心测试 |

### 4.3 自动化测试执行结果

| 测试 | 命令 | 结果 |
|------|------|------|
| `cargo check --workspace` | 编译检查 | ✅ 通过（1 warning） |
| `cargo test -p fceux11-utils -p fceux11-media -p fceux11-formats -p fceux11-debug` | 非 Lua 单元测试 | ✅ 46 passed |
| `cargo test -p fceux11-lua` | Lua crate 单元测试 | ❌ 71 个链接错误（FFI 符号缺失） |
| CMake 构建（RUST_ENABLED=OFF） | 传统 C++ 引擎 | 📋 未在本次检测中执行 |
| CMake 构建（RUST_ENABLED=ON, LUA_RUST_ENABLED=ON） | Rust 引擎 | 📋 未在本次检测中执行 |

---

## 五、安全性评估

| 风险点 | 严重度 | 当前状态 | 评估 |
|--------|--------|---------|------|
| `LUA_ENGINE_PTR` 全局裸指针 | 🔴 高 | `static mut LUA_ENGINE_PTR: *mut c_void` | 单线程场景可接受，但 `get_engine()` 返回 `&mut` 无运行时检查 |
| FFI 边界 panic 传播 | 🟡 中 | `mlua` 默认将 Rust panic 转 Lua error | 可接受 |
| `gui_data` 像素写入越界 | 🟢 低 | `set_pixel` 有边界检查 `x < 256 && y < 240` | 安全 |
| `RegistryKey` 生命周期 | 🟡 中 | 已从"立即 Drop"修复为"存入 `CallbackEntry`" | 需验证 GC 时机 |
| `speed_mode` 字段未读取 | 🟢 低 | 编译器 warning | 功能缺失：`FCEU_LuaSpeed()` 未实现 |

---

## 六、C++ 代码瘦身路线图

### 6.1 当前 C++ 代码构成（7,306 行）

```
lua-engine.cpp (7,306 lines)
├── #ifdef FCEUX11_LUA_RUST_ENABLED → Rust 桥接薄壳 (~225 lines)
├── #else → 完整 C++ Lua 引擎 (~6,594 lines)
│   ├── 可迁移至 Rust 的库代码: ~4,100 lines
│   ├── 不可迁移的 LuaSaveData 序列化: ~567 lines
│   ├── 不可迁移的 TieredRegion 钩子: ~321 lines
│   ├── 不可迁移的 taseditor/cdlog: ~373 lines
│   ├── 不可迁移的对话框/UI/平台代码: ~376 lines
│   ├── 不可迁移的保存/加载回调: ~93 lines
│   └── 其他不可迁移: ~764 lines
└── #endif → FFI 桥接函数 (~484 lines)
```

### 6.2 瘦身阶段建议

#### Phase A：修复阻塞性问题后立即切换（预计削减 ~4,100 行，56%）

**前置条件**：
1. 修复 `joypad.get` 返回类型（table vs 裸数值）
2. 修复 `rom.gethash` 返回 MD5（非 CRC32）
3. 实现 `input.get` FFI 桥接
4. 端到端测试 L2 全部通过

**操作**：将 `FCEUX11_LUA_RUST_ENABLED` 设为 ON，C++ 侧裁剪掉全部库注册函数、全局变量、FrameBoundary 逻辑。

#### Phase B：渐进补全后进一步瘦身（预计削减至 ~2,245 行，69%）

**前置条件**：
1. 实现 `movie.record`/`play`/`replay`
2. 实现 `gui.gdscreenshot`/`gdoverlay`/`parsecolor`
3. 实现 `savestate.persist` FFI 桥接
4. L4 社区脚本回归全部通过

#### Phase C：终极状态（~2,245 行 C++ 永久残留，69% 削减）

以下 C++ 代码因技术限制**永久不可迁移**：

| 模块 | 行数 | 原因 |
|------|------|------|
| FFI 桥接函数 | ~515 | Rust → C++ 接口层，本质需求 |
| LuaSaveData 序列化 | ~567 | 依赖 Lua 栈内部格式（lstate.h），mlua 不暴露 |
| TieredRegion 钩子 | ~321 | 每帧数千次调用，FFI 开销不可接受 |
| taseditor 库 | ~299 | Qt/Win 前端 GUI 深度耦合 |
| cdlog + 对话框/UI | ~450 | 平台耦合 |
| 保存/加载回调 | ~93 | 依赖 LuaSaveData 序列化 |

---

## 七、最终判定

| 判定维度 | 结论 | 说明 |
|---------|------|------|
| **能否完全替代？** | ❌ 不能 | C++ 永久残留 ~2,245 行（31%），主要是 FFI 桥接 + 序列化 + 平台耦合 |
| **能否替代大部分？** | ✅ 可以 | 69% 的 C++ 代码可被 Rust 替代 |
| **当前能否切换？** | ⚠️ 不能 | 有 3 个阻塞性兼容性问题需先修复 |
| **切换后性能影响** | ⚠️ 待测 | gui_overlay 混合逻辑简化，可能影响视觉效果；FFI 调用链加长 |

### 阻塞性兼容问题（切换前必须修复）

| # | 问题 | 影响 | 修复工作量 |
|---|------|------|-----------|
| B1 | `joystick.get` 返回裸数值而非 table | 破坏所有使用 joypad.get 的脚本 | 1–2 小时 |
| B2 | `rom.gethash` 返回 CRC32 非 MD5 | 破坏依赖 MD5 的脚本 | 1 小时 |
| B3 | `input.get` 返回空表 | 输入相关脚本失效 | 2–3 天（需 C++ FFI 桥接） |

### 建议的下一步行动

1. **立即修复 B1/B2**（预计 3 小时）
2. **实现 `input.get` FFI 桥接**（B3，预计 2–3 天）
3. **执行 L2 端到端测试**：使用 `FCEUX11_LUA_RUST_ENABLED=ON` 构建，运行全部 12 个测试脚本
4. **执行 L4 社区脚本回归**：收集 10+ 社区 TAS 脚本，逐一验证
5. **性能基准测试**：对比 C++ / Rust 两种模式的帧率
6. **内存泄漏检测**：运行 1 小时以上，监控内存占用
7. **全部通过后，正式切换并裁剪 C++ 代码**

---

## 附录 A：测试脚本位置

所有 L2 测试脚本位于 `tests/lua_scripts/` 目录：

```
tests/lua_scripts/
├── test_bit.lua          -- bit 库完整测试（22 项）
├── test_emu.lua          -- emu 库 API 测试（16 项）
├── test_memory.lua       -- memory 库读写+寄存器测试（15 项）
├── test_joypad.lua       -- joypad 库测试（5 项）
├── test_rom.lua          -- rom 库测试（5 项）
├── test_gui.lua          -- gui 库绘图测试（16 项）
├── test_movie.lua        -- movie 库查询测试（13 项）
├── test_savestate.lua    -- savestate 库测试（8 项）
├── test_input.lua        -- input 库桩函数测试（4 项）
├── test_ppu.lua          -- ppu 库测试（4 项）
├── test_sound.lua        -- sound 库结构测试（16 项）
├── test_zapper.lua       -- zapper 库测试（2 项）
├── test_debugger.lua     -- debugger 库测试（6 项）
├── test_frameadvance.lua -- 协程 yield/resume 核心测试
├── test_callbacks.lua    -- 回调持久化集成测试
└── test_tas_simulation.lua -- TAS 风格多库集成测试
```

## 附录 B：函数级完整对照表

### B.1 `emu` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `framecount` | ✅ | ✅ | 兼容 |
| `lagcount` | ✅ | ✅ | 兼容 |
| `lagged` | ✅ | ✅ | 兼容 |
| `emulating` | ✅ | ✅ | 兼容 |
| `paused` | ✅ | ✅ | 兼容 |
| `message` | ✅ | ✅ | 兼容 |
| `print` | ✅ | ✅ | 兼容 |
| `poweron` | ✅ | ✅ | 兼容 |
| `softreset` | ✅ | ✅ | 兼容 |
| `frameadvance` | ✅ | ✅ | ✅ 已修复（coroutine.yield） |
| `speedmode` | ✅ | ✅ | 兼容 |
| `pause` | ✅ | ✅ | 兼容 |
| `unpause` | ✅ | ✅ | 兼容 |
| `registerbefore` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `registerafter` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `registerexit` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `loadrom` | ✅ | ❌ | 缺失 |
| `exit` | ✅ | ❌ | 缺失 |
| `getdir` | ✅ | ❌ | 缺失 |
| `setrenderplanes` | ✅ | ❌ | 缺失 |
| `addgamegenie` | ✅ | ❌ | 缺失 |
| `delgamegenie` | ✅ | ❌ | 缺失 |
| `exec_count` | ✅ | ❌ | 缺失 |
| `exec_time` | ✅ | ❌ | 缺失 |
| `debuggerloop` | ✅ | ❌ | 缺失（Win 专属） |
| `debuggerloopstep` | ✅ | ❌ | 缺失（Win 专属） |

### B.2 `memory` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `readbyte` | ✅ | ✅ | 兼容 |
| `readbytesigned` | ✅ | ✅ | 兼容 |
| `readword` | ✅ | ✅ | 兼容 |
| `readwordsigned` | ✅ | ✅ | 兼容 |
| `writebyte` | ✅ | ✅ | 兼容 |
| `getregister` | ✅ | ✅ | 兼容 |
| `registerwrite` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `registerread` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `registerexec` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `readbyterange` | ✅ | ❌ | 缺失 |
| `setregister` | ✅ | ❌ | 缺失 |

### B.3 `joypad` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `get` | ✅ 返回 table | ✅ 返回 number | ❌ **不兼容** |
| `set` | ✅ | ✅ | 兼容 |
| `getdown` | ✅ | ❌ | 缺失 |
| `getup` | ✅ | ❌ | 缺失 |
| `getimmediate` | ✅ | ❌ | 缺失 |

### B.4 `rom` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `gethash` | ✅ MD5 | ❌ CRC32 | ❌ **不兼容** |
| `readbyte` | ✅ | ✅ | 兼容 |
| `writebyte` | ✅ | ✅ | 兼容 |
| `readbytesigned` | ✅ | ❌ | 缺失 |
| `readbyterange` | ✅ | ❌ | 缺失 |
| `getfilename` | ✅ | ❌ | 缺失 |

### B.5 `movie` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `mode` | ✅ | ✅ | 兼容 |
| `rerecordcount` | ✅ | ✅ | 兼容 |
| `length` | ✅ | ✅ | 兼容 |
| `stop` | ✅ | ✅ | 兼容 |
| `getfilename` | ✅ | ✅ | 兼容 |
| `getname` | ✅ | ✅ | 兼容 |
| `getreadonly` | ✅ | ✅ | 兼容 |
| `setreadonly` | ✅ | ✅ | 兼容 |
| `ispoweron` | ✅ | ✅ | 兼容 |
| `isfromsavestate` | ✅ | ✅ | 兼容 |
| `active` | ✅ | ✅ | 兼容 |
| `recording` | ✅ | ✅ | 兼容 |
| `playing` | ✅ | ✅ | 兼容 |
| `record` | ✅ | ❌ | 缺失 |
| `play`/`playback` | ✅ | ❌ | 缺失 |
| `replay` | ✅ | ❌ | 缺失 |
| `rerecordcounting` | ✅ | ❌ | 缺失 |

### B.6 `savestate` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `save` | ✅ | ✅ | 兼容 |
| `load` | ✅ | ✅ | 兼容 |
| `create` | ✅ | ✅ | 兼容 |
| `object` | ✅ | ✅ | 兼容 |
| `registersave` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `registerload` | ✅ | ✅ | ✅ 已修复（RegistryKey 持久化） |
| `persist` | ✅ | ❌ | 空壳 |
| `loadscriptdata` | ✅ | ❌ | 缺失 |

### B.7 `gui` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `pixel` | ✅ | ✅ | 兼容 |
| `getpixel` | ✅ | ✅ | 兼容 |
| `line` | ✅ | ✅ | 兼容 |
| `box` | ✅ | ✅ | 兼容 |
| `text` | ✅ | ✅ | 兼容（5x7 自定义字体 vs C++ 内嵌字体） |
| `savescreenshot` | ✅ | ✅ | 兼容 |
| `opacity` | ✅ | ✅ | 兼容 |
| `transparency` | ✅ | ✅ | 兼容 |
| `popup` | ✅ | ✅ | 兼容 |
| `register` | ✅ | ✅ | 兼容 |
| `clear` | ✅ | ✅ | 兼容 |
| `gdscreenshot` | ✅ | ❌ | 缺失 |
| `gdoverlay` | ✅ | ❌ | 缺失 |
| `parsecolor` | ✅ | ❌ | 缺失 |
| `fillbox` | ✅ | ❌ | 缺失 |

### B.8 `input` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `get` | ✅ | ❌ | 空壳（返回空表） |
| `popup` | ✅ | ❌ | 桩函数（仅 print） |
| `openfilepopup` | ✅ | ❌ | 桩函数（返回空串） |
| `savefilepopup` | ✅ | ❌ | 桩函数（返回空串） |

### B.9 `ppu` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `readbyte` | ✅ | ✅ | 兼容 |
| `readbyterange` | ✅ | ✅ | 兼容 |

### B.10 `sound` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `get` | ✅ | ✅ | 基本兼容（结构完整） |

### B.11 `zapper` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `read` | ✅ | ✅ | 兼容 |
| `set` | ✅ | ✅ | 兼容 |

### B.12 `debugger` 库

| 函数 | C++ | Rust | 状态 |
|------|-----|------|------|
| `hitbreakpoint` | ✅ | ✅ | 兼容 |
| `getcyclescount` | ✅ | ✅ | 兼容 |
| `getinstructionscount` | ✅ | ✅ | 兼容 |
| `resetcyclescount` | ✅ | ✅ | 兼容 |
| `resetinstructionscount` | ✅ | ✅ | 兼容 |
| `getsymboloffset` | ✅ | ✅ | 兼容 |

### B.13 `taseditor` 库（不迁移）

全部 20 个函数保留在 C++ 侧。

### B.14 `cdlog` 库（不迁移）

全部 8 个函数保留在 C++ 侧。
