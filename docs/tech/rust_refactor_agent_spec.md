# FCEUX11 Rust 模块重构 — Agent 行为约束规范

> **状态**：⚠️ 已停用（v0.2.16 后）  
> **停用原因**：v0.2.16 实证表明，Claude Code 在无法本地验证的 MSVC+vcpkg+Qt6 混合构建环境中，错误率（编译错误、架构级状态分裂、测试逻辑错误）产生的修复开销已超过并行编码的理论收益。后续 v0.2.17–v0.2.30 由 Kimi Code CLI 统一负责编码、构建、测试与版本发布。  
> **历史版本**：v0.2.15+ 曾尝试双 Agent 结对（Claude Code 编码 + Kimi 验证），本文件保留为历史记录。

> ~~**版本**：v0.2.15+~~  
> ~~**适用阶段**：阶段二（v0.2.16 – v0.2.25）与阶段三（v0.2.26 – v0.2.30）~~  
> ~~**约束对象**：Claude Code（及任何具备代码执行能力的 AI Agent）~~  
> ~~**执行与验证负责人**：Kimi Code CLI~~

---

## 一、设计意图

阶段一（v0.2.12 – v0.2.15）已不可逆地清理了 11 个模块的 C++ 回退代码，并完成了 Workspace 拆分、cbindgen 集成与 ROM 回归测试基线。阶段二与阶段三的核心任务是**将 C/C++ 模块手工翻译为 Rust**，目标累计 Rust 代码量 ≥15,000 行，最终 Rust 占比 ≥35%。

本规范的存在是因为：
1. **Claude Code 具备终端执行权限**，可隐式触发 `cargo build` / `ctest`，这与本项目"混合构建验证由 Kimi 统一负责"的决策冲突。
2. **编译环境具有平台特殊性**（MSVC BuildTools + vcpkg + Qt6），Claude Code 在无正确 `INCLUDE` / `LIB` 环境变量时构建必然失败，导致无效的错误反馈循环。
3. **ROM 回归测试是行为级正确性的唯一真理来源**，其执行需要完整的 MSVC 环境与 CMake 构建产物，不应由编码 Agent 在本地尝试。

因此，本文档以**硬性禁令**划定 Agent 的行为边界，确保编码与验证的职责分离。

---

## 二、角色边界（硬性分工）

| 职责 | Claude Code | Kimi Code CLI |
|------|-------------|---------------|
| **需求分析** | 阅读计划文档、分析 C++ 源码、设计 Rust 模块结构 | — |
| **编码实现** | ✅ 唯一负责：编写 `.rs`、修改 `lib.rs`、更新 `Cargo.toml`、生成 `fceux11_rust.h`（通过 `cbindgen` 配置） | — |
| **C++ Wrapper 调整** | ✅ 可修改薄封装层（如删除回退代码、调整 `#include`） | — |
| **单元测试编写** | ✅ 编写 Rust `#[test]`（函数级） | — |
| **代码审查** | — | ✅ 审查 FFI 边界安全性、Rust idiomatic 程度 |
| **构建与链接** | ❌ **绝对禁止** | ✅ 唯一负责：`cmake --build` / `cargo build` |
| **集成测试执行** | ❌ **绝对禁止** | ✅ 唯一负责：`ctest` / ROM 回归测试 |
| **性能回归** | ❌ **绝对禁止** | ✅ 唯一负责：`cargo bench` 或 C++ 侧计时 |
| **Git 操作** | ❌ **绝对禁止** | ✅ 唯一负责：`git commit` / `git tag`（按用户指令） |
| **版本发布** | ❌ **绝对禁止** | ✅ 唯一负责：打 tag、Release Notes |

> **禁令的本质**：Claude Code 的终端权限仅用于**只读文件系统探索**（`cat`、`find`、`grep`），任何可能触发编译器、链接器、测试运行器或包管理器构建动作的命令均属违规。

---

## 三、禁止行为清单（Red Lines）

以下命令/操作在 Claude Code 会话中**不得执行**，无论用户是否直接要求：

### 3.1 编译与链接（全面禁止）

```bash
# 禁止：任何 Rust 编译动作
cargo build
cargo build --release
cargo check
cargo clippy
cargo doc
cargo test          # Rust 单元测试也不得由 Claude 触发
rustc

# 禁止：任何 C++ 编译动作
cmake --build .
ninja
msbuild
cl.exe /link.exe (直接调用 MSVC)
make

# 禁止：CMake 重新配置（会触发生成器重新扫描）
cmake ..
cmake -B build
```

### 3.2 测试执行（全面禁止）

```bash
# 禁止：任何测试运行
cargo test --workspace
cargo test -p fceux11-utils
ctest
./fceux11_smoke_test.exe
./fceux11_rom_regression_test.exe
python fixtures/generate_test_roms.py   # 若涉及测试数据生成需 Kimi 确认
```

### 3.3 包管理器副作用（全面禁止）

```bash
# 禁止：修改依赖树
cargo add
cargo remove
cargo update
vcpkg install

# 例外：若需新增 crate 依赖，Claude Code 应在代码注释中标记 TODO，
# 由 Kimi 在构建阶段统一评估并执行 cargo add。
```

### 3.4 Git 变异操作（全面禁止）

```bash
git commit
git push
git merge
git rebase
git tag
git reset --hard
```

> **只读例外**：`git log`、`git show`、`git diff`、`git status` 可用于理解代码历史，但不得作为写入操作的铺垫。

### 3.5 环境修改（全面禁止）

```bash
# 禁止：修改系统或项目级环境
set INCLUDE=...           # Windows
export LIB=...            # Linux/macOS（本项目不使用，但列入禁止）
vcvarsall.bat             # 即使为了"正确编译"也不得执行
```

---

## 四、允许的只读探索行为

Claude Code 可自由执行以下操作以理解代码基：

```bash
# 文件系统探索
ls、find、cat、head、tail、grep、rg

# Rust 源码分析（不触发编译）
cargo tree --no-dedupe    # 查看依赖树（只读）
cargo metadata --no-deps  # 查看元数据（只读）

# 理解已有 FFI 边界
cat src/rust/fceux11_rust.h
cat src/rust/cbindgen.toml
```

---

## 五、编码规范（Claude Code 必须遵守）

### 5.1 模块认领与范围

每个重构版本（v0.2.16 – v0.2.30）对应单一模块或模块对。Claude Code 在开始编码前必须：

1. 阅读 `docs/rust_refactor_plan_v0.2.12-v0.2.30.md` 中对应版本的描述。
2. 阅读 C++ 原文件的完整内容（包括 `.h` 头文件）。
3. 在 Rust 侧确定目标 sub-crate：
   - `fceux11-utils`：工具类（md5、guid、crc32 等）
   - `fceux11-formats`：格式解析（ines、unif、cart、nsf、fds、wave、emufile）
   - `fceux11-media`：音视频（filter、palette、video helpers）
   - `fceux11-debug`：调试辅助（profiler、cheat、conddebug、debug、debugsym、asm）
   - `fceux11-core`：未来核心（state、movie、input、ppu、apu、cpu、boards）
4. **不得跨版本预迁后续模块**。v0.2.16 只处理 EmuFile，即使发现 Cheat 的代码可以顺手复用，也必须在注释中标记 `// TODO(v0.2.24): extract shared logic`，等待对应版本实施。

### 5.2 FFI 边界设计模式

所有 Rust 模块必须遵循已验证的 11 个模块的 FFI 范式：

#### 模式 A：纯函数（无状态）
适用于 crc32、md5、guid、general、timestamp、os_utils、ConvertUTF。

```rust
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_crc32(crc: u32, buf: *const u8, len: u32) -> u32 {
    // 1. null/len 检查
    // 2. unsafe { slice::from_raw_parts(...) }
    // 3. 纯计算
    // 4. 返回结果
}
```

#### 模式 B：Opaque Pointer（状态由 Rust 拥有）
适用于 FilterState、ProfilerHandle。

```rust
pub struct FilterState { /* ... */ }

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_filter_create(rate: u32) -> *mut FilterState {
    Box::into_raw(Box::new(FilterState::new(rate)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_filter_destroy(state: *mut FilterState) {
    if !state.is_null() {
        drop(Box::from_raw(state));
    }
}
```

**硬性规则**：
- C++ 侧不触碰 Rust 内部结构字段。
- Rust 侧不假设 C++ 分配器。
- Opaque Pointer 的创建/销毁必须成对出现，并在 Rust 文档注释中标注生命周期责任方。

#### 模式 C：Slice 类型（缓冲区传递）

自 v0.2.13 起，所有涉及缓冲区的 FFI 函数必须使用 `FceuSlice` / `FceuSliceMut`：

```rust
use fceux11_utils::slice::{FceuSlice, FceuSliceMut};

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_draw_control_bars(buf: FceuSliceMut, w: i32, h: i32) {
    let slice = unsafe { buf.as_mut_slice() };
    // 边界检查：slice.len() >= (w * h) as usize
}
```

### 5.3 `unsafe` 使用规范

| 场景 | 要求 |
|------|------|
| `unsafe` 块 | 必须附注释说明不变量由谁保证 |
| `unsafe fn` | FFI 入口函数可标记为 `unsafe extern "C"`，内部 Rust API 应优先使用 safe wrapper |
| `from_raw_parts` | 必须前置 null 与长度检查 |
| 原始指针解引用 | 仅在 FFI 边界处允许；内部逻辑优先使用 `&[u8]` / `&mut [u8]` |
| `std::mem::transmute` | **禁止**；如需类型转换，显式使用 `as` 或 `bytemuck`（需 Kimi 评估引入） |

### 5.4 C++ Wrapper 清理规范

当 Rust 实现取代 C++ 回退代码时：

1. **保留 C++ 文件**，但将其内容替换为对 Rust FFI 的薄封装（thin wrapper），**不要直接删除 `.cpp` 文件**（避免破坏其他文件的 `#include` 链）。
2. 在文件顶部添加注释：
   ```cpp
   // [v0.2.XX] Rust-backed wrapper. Original C++ fallback archived at
   // docs/legacy_fallback_v0.2.11/xxx.cpp
   ```
3. 如果原 C++ 文件是 `.c`（如 `ConvertUTF.c`），改为 `.cpp` 以统一编译单元，或在 CMake 中移除该源文件并链接 Rust staticlib。
4. **不修改调用方代码的函数签名**。如果原 C++ 函数被 20 个文件调用，Rust 化后应通过同名 wrapper 保持 ABI 兼容。

### 5.5 单元测试要求

每个新 Rust 模块必须包含 ≥5 个 `#[test]`：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // 覆盖 happy path
    }

    #[test]
    fn test_null_safety() {
        // 覆盖 null 指针输入（模拟 C 侧误用）
    }

    #[test]
    fn test_boundary_condition() {
        // 覆盖空切片、零长度、最大值
    }

    #[test]
    fn test_determinism() {
        // 相同输入必须产生相同输出（用于帧哈希稳定性）
    }

    #[test]
    fn test_roundtrip_or_inverse() {
        // 若适用：serialize → deserialize、encode → decode
    }
}
```

> **Claude Code 只负责编写测试代码，不执行 `cargo test`**。测试能否通过由 Kimi 在构建阶段验证。

### 5.6 cbindgen 头文件同步

如果新增或修改了 `extern "C"` 函数：

1. 确保 `cbindgen.toml` 的 `prefix` 与命名约定一致。
2. **不要手动编辑 `src/rust/fceux11_rust.h`**。
3. 在 `Cargo.toml` 的 `[package.metadata.cbindgen]` 或对应 sub-crate 中更新导出配置。
4. 在代码交付注释中提醒 Kimi："需重新运行 cbindgen 生成头文件"。

---

## 六、与 Kimi 的协作交接协议

### 6.1 编码完成信号

Claude Code 完成一个模块的编码后，必须以**结构化注释**的形式在对话末尾输出交付清单：

```markdown
## 交付清单（v0.2.XX — 模块名）

### 新增/修改文件
- `src/rust/crates/xxx/src/xxx.rs` （新增，NNN 行）
- `src/rust/crates/xxx/src/lib.rs` （修改，新增 pub mod）
- `src/rust/crates/xxx/Cargo.toml` （修改，新增依赖）
- `src/xxx.cpp` （修改，改为 Rust FFI wrapper）
- `src/xxx.h` （修改，更新 extern 声明或删除）

### 单元测试
- `test_basic_functionality`
- `test_null_safety`
- `test_boundary_condition`
- `test_determinism`
- `test_roundtrip`

### 待 Kimi 执行
- [ ] `cargo build --workspace`（验证编译）
- [ ] `cargo test --workspace`（验证 Rust 单元测试）
- [ ] `cmake --build .`（验证 C++ 混合链接）
- [ ] `ctest --output-on-failure`（验证 ROM 回归测试无退化）
- [ ] `cbindgen` 重新生成头文件（若 FFI 签名变更）
- [ ] 版本号 bump（由 Kimi 统一操作）

### 已知风险
- 该模块调用了 C++ 侧的 `XXX` 回调函数，需验证生命周期。
- 使用了 `std::io::Write` trait，需确认 release profile 性能。
```

### 6.2 失败回退路径

若 Kimi 在构建/测试阶段发现失败：

1. Kimi 收集错误日志（编译错误、链接错误、测试失败、ROM 哈希不匹配）。
2. Kimi 将日志 + 失败文件路径 回传给 Claude Code。
3. **Claude Code 仅修改代码，不尝试本地复现构建**。其修复应基于错误日志的静态分析。
4. 修复后再次进入 6.1 的交付流程。

---

## 七、版本迭代节奏

| 版本 | 模块 | Claude Code 编码预估 | Kimi 构建验证预估 |
|------|------|---------------------|-------------------|
| v0.2.16 | EmuFile | 1 轮 | 1 轮 |
| v0.2.17 | VS UniSystem | 1 轮 | 1 轮 |
| v0.2.18 | UNIF 解析 | 1–2 轮 | 1–2 轮 |
| v0.2.19 | iNES 解析 | 2 轮 | 2 轮 |
| v0.2.20 | Cart 卡带管理 | 2 轮 | 2 轮 |
| v0.2.21 | NSF 播放器 | 1–2 轮 | 1–2 轮 |
| v0.2.22 | conddebug + asm | 2 轮 | 1–2 轮 |
| v0.2.23 | Drawing | 1–2 轮 | 1–2 轮 |
| v0.2.24 | Cheat 引擎 | 2 轮 | 2 轮 |
| v0.2.25 | debug + debugsym | 2 轮 | 2 轮 |
| v0.2.26 | FDS 磁盘系统 | 2–3 轮 | 2–3 轮 |
| v0.2.27 | Video 后处理 | 2 轮 | 2 轮 |
| v0.2.28 | Movie 录像系统 | 3 轮 | 3 轮 |
| v0.2.29 | State 状态序列化 | 3 轮 | 3 轮 |
| v0.2.30 | 核心边界设计 | 1 轮（文档+原型） | 1 轮 |

> **每轮 = Claude Code 编码 → Kimi 构建测试 → 反馈 → Claude Code 修复**。一轮的理想周期为 5–15 分钟对话交互。

---

## 八、违规处理

若 Claude Code 执行了本规范禁止的操作（如隐式运行 `cargo build`）：

1. **立即中止**该次会话的编码输出。
2. Kimi 有权丢弃该次会话产生的所有文件修改。
3. 在 `docs/tech/rust_refactor_agent_spec.md` 的修订记录中登记该次违规，作为后续 Prompt 工程的负例。

---

## 九、附录： Claude Code 特性利用指南

### 9.1 应充分利用的能力

- **大范围上下文窗口**：一次性传入整个 C++ 源文件（≤1,500 行）进行 1:1 翻译，减少分段错误。
- **批量编辑**：使用多文件编辑工具同时修改 `.rs`、`.cpp`、`.h`、`.toml`。
- **类型推断辅助**：让 Claude 先写出 Rust 的 `struct` / `enum` 定义，再填充函数体，利用类型系统捕获 C 风格错误。
- **文档生成**：自动生成 `///` 文档注释，特别是 FFI 函数的 Safety 契约。

### 9.2 应警惕的陷阱

- **过度优化**：Claude 可能在未验证的情况下使用 `unsafe` 进行微优化。规范要求：先写出 safe Rust，仅在 Kimi 确认性能退化 >110% 时才引入 `unsafe`。
- **闭包穿越 FFI**：Claude 可能尝试将 Rust 闭包传递给 C。本阶段**禁止**闭包穿越 FFI，仅允许裸函数指针（如 `NeoFilterSound` 模式）。
- **隐式 `cargo check`**：某些编辑操作（如保存 `Cargo.toml`）可能触发 IDE 后台的 `cargo check`。Claude Code 必须关闭或忽略此类诊断，不得基于其输出修改代码。

---

## 十、修订记录

| 日期 | 版本 | 修订内容 |
|------|------|----------|
| 2026-05-30 | v1.0 | 初始发布，对应阶段二/三启动 |
