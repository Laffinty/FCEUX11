# FCEUX11 Lua 引擎 Rust 重构可行性评估报告

> **评估日期**：2026-05-31
> **版本**：v0.2.22 完成之后
> **目标**：在不与 rust_refactor_plan_v0.2.12-v0.2.30.md 冲突的前提下，将 src/lua 臃肿模块用 Rust 重构

---

## 一、核心问题：保留兼容层 + Rust 重构

用户意图是：
- **保留**一部分 C/C++ API 作为兼容层（对接现有 Lua 脚本）
- **主体**用 Rust 重构
- **无缝融入**现有 Rust 体系

这意味着不替换 Lua 脚本语言，只替换底层实现。

---

## 二、相关技术调研

### 2.1 方案 A：使用 `mlua`（推荐）

**项目**：`mlua-rs/mlua` (GitHub 1,896 commits, 活跃)

**特点**：
- 支持 Lua 5.1/5.2/5.3/5.4/5.5 + LuaJIT + Luau
- 高级 Rust 绑定，async/await 支持
- 保留 Lua 脚本**完全兼容**
- 成熟稳定，被多个生产项目使用

**架构**：
```
Rust 绑定层 (mlua) → Lua VM (系统库或内嵌) → Lua 脚本
```

**FCEUX 改造后**：
```
Rust 绑定层 (mlua) → Lua 5.1 VM → 用户 Lua 脚本
     ↑
FCEUX 特定绑定（Rust）→ emu, memory, joypad, gui 等库
```

### 2.2 方案 B：使用 `rhai`（不兼容 Lua）

**项目**：`rhai` (GitHub 4,433+ commits)

**特点**：
- 纯 Rust 实现的脚本语言
- 语法类似 JavaScript + Rust
- **Lua 脚本不兼容**
- 轻量级，适合配置/逻辑脚本

### 2.3 方案 C：混合方案（最务实）

```
┌─────────────────────────────────────────┐
│           Rust 重构层 (新增)             │
│  fceux11_lua_engine (Rust crate)        │
│  - FCEUX 特定绑定 (emu, memory, gui...) │
│  - 帧推进控制、状态管理               │
│  - 输入拦截、GUI 叠加                  │
└────────────────┬──────────────────────┘
                 │
                 │ FFI (原有 C++ API)
                 ↓
┌─────────────────────────────────────┐
│      兼容层 (保留)                     │
│  lua-engine.cpp 薄封装                 │
│  - 仅做 mlua → Lua_State 适配          │
│  - 暴露原有 C++ API 给 mlua          │
└─────────────────────────────────────┘
```

---

## 三、与 rust_refactor_plan_v0.2.12-v0.2.30.md 的兼容性

### 3.1 文档分析

计划聚焦**叶子模块**迁移，特征为：
- 零交叉依赖
- 仅通过 `lib.rs` 挂载
- 一个模块失败不波及其他

**冲突点**：Lua 引擎深度耦合核心模块（x6502.cpp, input.cpp, video.cpp 等）

### 3.2 解决方案：**不迁移 Lua 引擎本身，而是创建新 Rust crate 绑定到现有 Lua**

```rust
// src/rust/crates/fceux11-lua/src/lib.rs
pub mod bindings {
    pub mod emu;      // FCEUX 特定绑定
    pub mod memory;
    pub mod joypad;
    pub mod gui;
    // ...
}
```

**优点**：
1. 不删除任何现有 C++ 代码
2. Lua 脚本完全兼容
3. 新增 Rust crate 融入现有体系
4. 与计划**无冲突**（计划针对的是"替换"不是"新增绑定"）

---

## 四、可行性评估

### 4.1 技术可行性：高

| 因素 | 评估 |
|------|------|
| mlua 成熟度 | ✅ 1,896 commits，生产可用 |
| Lua 脚本兼容 | ✅ 保留 |
| Rust 绑定复杂度 | 中（~15 个库需要绑定） |
| 工作量 | 低-中（绑定层 vs 完整重写） |
| 耦合冲突 | 无（新增而非替换） |

### 4.2 风险：低

| 风险 | 缓解 |
|------|------|
| mlua 内存管理 | Rust ownership 模型与 Lua GC 需协调 |
| 异步集成 | mlua 支持 async/await |
| 第三方库（IUP, CD 等） | 可选，初期跳过 |

---

## 五、建议实施方案

### 5.1 架构设计

```
用户 Lua 脚本
    ↓
lua-engine.cpp (薄封装)
    ↓
mlua (Lua VM bindings)
    ↓
Rust FFI 绑定层 (fceux11_lua crate)
    ↓
FCEUX 核心 (Rust 已迁移模块)
```

### 5.2 实施步骤

**v0.3.1**（如果计划允许）：
1. 创建 `src/rust/crates/fceux11-lua/Cargo.toml`
2. 添加 `mlua = "1.4"` 依赖
3. 实现 `emu` 绑定（最基础）
4. 编译测试

**v0.3.2+**：
5. 实现 `memory` 绑定
6. 实现 `joypad` 绑定
7. 实现 `gui` 绑定
8. ... 逐步完成所有库

### 5.3 不改动的地方

- `src/lua-engine.cpp` - 保留（只是调用 mlua）
- `src/lua/src/` - 保留（Lua 5.1 VM）
- `src/fceulua.h` - 保留（API 契约不变）

---

## 六、结论

**可行性**：✅ 高

**方案**：使用 `mlua` 创建 Rust 绑定层，不替换 Lua 引擎本身

**与计划关系**：无冲突（新增 crate，非替换现有模块）

**推荐程度**：★★★★☆

**理由**：
1. 完全保留 Lua 脚本向后兼容
2. 工作量可控（绑定 vs 重写）
3. 融入现有 Rust 体系
4. 技术成熟（mlua 活跃度高）
