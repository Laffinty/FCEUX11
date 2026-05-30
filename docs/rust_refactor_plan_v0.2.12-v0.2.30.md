# FCEUX11 Rust 重构构建计划 v0.2.12 — v0.2.30

> **制定日期**：2026-05-30  
> **版本范围**：v0.2.12 → v0.2.30（最多 19 个迭代版本）  
> **核心决策**：彻底清理已 Rust 化的 C/C++ 回退代码，架构优化优先，向核心模块渐进渗透。  
> **风险等级**：中高（已具备 10 个模块的成功经验与 62 个单元测试保障）

---

## 一、执行摘要

本项目已于 v0.2.11 成功完成 **11 个叶子模块**的 Rust 重构（约 3,714 行 Rust，62 个单元测试全部通过），验证了 CMake/Cargo 混合构建、手动 C ABI、Opaque Pointer、`#[repr(C)]` 结构体等 FFI 范式。当前所有模块仍保留 C/C++ 回退实现，属于"可逆迁移"。

本计划响应"彻底清理回退代码、演进到 v0.2.30"的战略决策，综合现有模块经验、国际前沿学术论文与工业实践，提出一个**分三阶段、共 19 个版本**的构建路线图：

1. **清理与架构重构期**（v0.2.12–v0.2.15）：删除回退代码、单 Crate → Workspace、引入 cbindgen、建立 ROM 回归测试基线。
2. **扩展重构期**（v0.2.16–v0.2.25）：迁移 10 个中等规模子模块（文件格式、解析器、调试辅助、工具类），累计 Rust 代码量突破 15,000 行。
3. **核心渗透与架构定型期**（v0.2.26–v0.2.30）：完成状态序列化、录像系统等大型模块的 Rust 化，设计核心模拟循环（CPU/PPU/APU）的 Rust-first 抽象边界，为 v0.3.x 全面核心迁移奠基。

---

## 二、现状诊断与可借鉴经验

### 2.1 已重构模块全景

| 版本 | 模块 | 功能域 | 代码行数 | FFI 模式 | 测试数 |
|------|------|--------|----------|----------|--------|
| v0.2.1 | CRC32 | 工具/哈希 | 29 | 纯函数 | — |
| v0.2.2 | MD5 | 工具/哈希 | 366 | 纯函数 + thread_local 缓冲 | ~6 |
| v0.2.3 | GUID | 工具/UUID | 200 | 纯函数 + thread_local 缓冲 | ~2 |
| v0.2.4 | General | 工具/数学 | 30 | 纯函数 | ~1 |
| v0.2.5 | Wave | I/O/音频导出 | 225 | 静态 Mutex 全局状态 | ~3 |
| v0.2.6 | OS Utils | I/O/系统调用 | 131 | 纯函数 | ~5 |
| v0.2.7 | ConvertUTF | 算法/编码 | 1004 | 纯函数 | ~20 |
| v0.2.8 | TimeStamp | 工具/计时 | 73 | 纯函数 | ~4 |
| v0.2.9 | Profiler | 性能分析 | 236 | Opaque Pointer + 全局 Mutex | ~4 |
| v0.2.10 | Filter | 音频/数字信号 | 423 | Opaque Pointer + C→Rust→C 回调 | ~6 |
| v0.2.11 | Palette | 图形/调色板 | 385 | 纯计算 + 像素缓冲写入 | ~8 |

**关键经验总结**：

1. **叶子优先策略极度成功**：11 个模块彼此零交叉依赖，仅通过 `lib.rs` 挂载，任何一个模块的构建失败都不会波及其他模块。
2. **手动 C ABI 在小规模下可行**：当前 `fceux11_rust.h` 约 4.5KB、94 行，11 个模块共 23 个 FFI 函数，手工维护成本可控；但当模块数超过 20 个时，头文件同步将成为主要心智负担。
3. **Opaque Pointer 是状态管理的银弹**：`FilterState`、`ProfilerHandle` 等模式被证明是跨语言所有权管理的最佳实践——C++ 不触碰 Rust 内部结构，Rust 不假设 C++ 分配器。
4. **全局状态需显式参数化**：`filter.rs` 和 `palette.rs` 的成功关键在于**拒绝直接读取 C++ 全局变量**（如 `FSettings.SndRate`），全部通过 FFI 参数传入。这降低了耦合，使 Rust 单元测试可在无 C++ 运行时的情况下独立执行。
5. **回调穿越（C→Rust→C）只有一例**：`NeoFilterSound` 的 `GameExpSound.NeoFill` 回调证明函数指针传递是可行的，但需确保 C 侧函数指针生命周期长于 Rust 调用。不建议在 v0.2.x 阶段引入更复杂的闭包或 trait object 回调。

### 2.2 现有 C/C++ 代码基规模

```
src/*.cpp         ~18,000 行（核心模拟器）
src/utils/*.cpp   ~4,800 行（工具库）
src/drivers/      ~12,000 行（前端与视频处理）
src/boards/       ~8,200 行（Mapper 系统）
-------------------------------------------
总计              ~43,000 行（不含 Lua）
```

已 Rust 化占比：**~8.6%**（按行数）。v0.2.30 目标占比：**~35–40%**。

---

## 三、学术研究与工业实践综合分析

### 3.1 增量迁移方法论

#### His2Trans — Skeleton-First + 历史知识复用（arXiv:2603.02617, 2026）

该研究针对 OpenHarmony 工业级代码库提出**"骨架先行"**策略：
- **项目级骨架**：在生成函数体之前，先建立可编译的模块树、类型定义、全局符号和跨模块引用。
- **ABI 保留类型策略**：`#[repr(C)]` 仅在 FFI 边界使用；内部 API 可自由演进到安全 Rust。
- **混合构建验证**：在 5 个工业模块上，混合 C/Rust 构建全部链接成功，无 FFI 接口不匹配，且通过所有既有测试。

**对本项目的启示**：
- v0.2.12–v0.2.15 的架构重构本质上就是"骨架先行"：先拆分 Workspace、稳定 FFI 边界，再填充后续模块。
- 历史知识复用：本项目已有的 11 个翻译对（C ↔ Rust）已形成内部知识库，后续模块可直接复用 `Opaque Pointer`、`thread_local` 返回缓冲区、`#[repr(C)]` 结构体等模式。

#### C2SaferRust — 神经符号混合翻译（arXiv:2501.14257, 2025）

该工作以 C2Rust 为桥梁，将 C 代码先转为等价的 unsafe Rust，再用 LLM 分块翻译为更安全、更地道的 Rust：
- **原始指针声明减少 38%**，**解引用减少 27%**，**unsafe 代码行减少 28%**。
- **关键发现**：函数长度与翻译成功率成反比；将长函数拆分为 ≤150 行的翻译单元可显著提升成功率。

**对本项目的启示**：
- 对于 `movie.cpp`（2041 行）、`state.cpp`（1543 行）等大型模块，**必须在 Rust 侧先做架构拆分**（如将 state 序列化拆分为 `state_serialize`、`state_deserialize`、`state_compress` 等子模块），而非直接 1:1 翻译。
- 本项目已有 C++ 代码是手工翻译的，质量远高于 C2Rust 输出，但 C2SaferRust 的"分块翻译"思想可直接借鉴。

#### Rustine — 大规模仓库级翻译（arXiv:2511.20617, 2025）

Rustine 在 23 个真实 C 项目上达到 **100% 编译通过率**、**87% 断言通过率**，且通过人工调试可在平均 4.5 小时内将断言通过率提升至 99.3%。其输出相比 C2Rust：
- 原始指针声明从 4,904 降至 198（-96%）
- unsafe 代码行从 13,127 降至 114（-99.1%）

**对本项目的启示**：
- 自动化工具目前更适合"语法翻译"，但语义正确性（尤其是模拟器的行为等价性）仍需人工验证。
- 本项目应建立**自动化差异测试框架**（ROM 级别），而非仅依赖单元测试。

#### C-to-Rust 代码质量分析（arXiv:2602.00840, 2025）

该研究比较了 C2Rust、C2SaferRust、TranslationGym 和人类翻译，发现：
- 自动化工具在提升内部质量（地道性）时，常以外部质量为代价：引入兼容性风险、运行时 panic、线程安全问题。
- **即使是人类翻译，也在所有 18 个质量类别中存在问题**，只是数量更少。

**对本项目的启示**：
- 不要追求"一次写出完美 Rust"；应接受迭代修复，每个 Phase 预留 20% 时间用于 Clippy 修复、边界情况补充和性能回归测试。
- 对于模拟器这种**行为敏感型系统**，兼容性风险比语法不地道更致命。宁可多写一点 `unsafe` 或保留 C ABI，也要确保时序和数值输出完全一致。

### 3.2 FFI 边界安全

#### R3 — Rewrite it in Rust with Refinements（UCSD HotOS, 2024）

该论文指出，真实世界的 Rust/C++ 重写引入了一类**新的 FFI 边界 bug**：
- C 侧分配追踪器缺失导致 Rust 侧无法验证指针安全不变量。
- Rust 的 `unsafe` 块集中在 FFI 边界，但缺乏 refinement type 来关联指针与其边界。

**对本项目的启示**：
- 当前项目的 FFI 函数已普遍检查 `null` 指针，但**缺乏长度验证**（如 `*mut u8` + `width * height` 的像素缓冲区）。
- v0.2.12 起，所有接收缓冲区的 FFI 函数应增加 `size: usize` 参数，并在 Rust 侧用 `slice::from_raw_parts` 进行边界检查。
- 建议引入 `#[repr(C)] struct Slice { ptr: *mut u8, len: usize }` 类型，替代裸指针+隐含长度的 C 习惯。

### 3.3 工业级迁移案例

#### Meta — 十亿用户代码库的 C→Rust 迁移（2025）

- **库级替换**：仅替换中心消息库，而非全应用栈，范围可控。
- **双构建支持**：过渡期同时支持 C 和 Rust，运行时特性标志切换。
- **组件化重写**：优先替换内存不安全的高风险例程，低风险模块延后。

#### Microsoft — 2030 年全面替换 C/C++（2025–2030）

- 目标：1 名工程师 + 1 个月 + 100 万行代码（AI 辅助）。
- 策略：算法基础设施建立源代码可扩展图 + AI Agent 引导修改。

**对本项目的启示**：
- FCEUX11 规模（~43K 行）远小于 Microsoft 目标，**完全在手工可控范围内**，无需等待 AI 翻译工具成熟。
- 但可借鉴"图化理解"思想：在 v0.2.15 前建立 C++ 模块依赖图，识别"调用扇出低、被调用扇入低"的模块作为下一批目标。

### 3.4 模拟器领域的 Rust 实践

#### NES Emulator in Rust 社区经验（TetaNES, Lochnes, nescore, 2019–2024）

综合多个 Rust NES 模拟器项目的公开技术博客与源码，总结出以下与 C→Rust 迁移高度相关的经验：

1. **Bus 解码天然适合 Rust match**：`read_u8` / `write_u8` 中的地址范围解码用 `match addr { 0x0000..=0x1FFF => ... }` 比 C switch 更清晰，且编译器可优化为跳转表。
2. **Ownership 对共享状态的挑战**：CPU、PPU、APU、Mapper 需要共享总线状态。Rust 社区方案包括：
   - `Rc<RefCell<T>>`（单线程，简单但运行时检查）
   - 通道/消息传递（严格但可能引入延迟）
   -  unsafe 裸指针（高性能，需人工保证别名安全）
   - **建议本项目在 v0.2.30 阶段设计核心抽象时，采用 "System 结构体 owning 所有组件" 模式**：`struct Nes { cpu: Cpu, ppu: Ppu, apu: Apu, cart: Cart }`，通过 `&mut self` 统一调度，避免分布式共享状态。
3. **整数溢出检查**：Rust debug 构建默认启用溢出检查，但 release 默认关闭。NES 模拟器中大量依赖 `u8` 回绕行为（如 CPU 标志位、PPU 地址寄存器），必须在 Rust 侧显式使用 `wrapping_add`、`wrapping_sub` 或 `u8::overflowing_add`，否则 release 行为正确但 debug 行为可能 panic。
4. **Cycle-accurate 模拟与生成器**：Lochnes 使用 Rust 生成器（Generator）实现 CPU/PPU 的协同调度，使 `clock()` 可逐周期交错执行。该模式对 FCEUX11 未来核心迁移有参考价值，但 Generator 目前仍为 nightly 特性，v0.2.x 阶段不引入。

---

## 四、战略决策：不可逆化的可行性与风险分析

### 4.1 决策评估

| 维度 | 评估 | 说明 |
|------|------|------|
| **技术可行性** | ✅ 高 | 11 个模块全部通过独立单元测试，FFI 接口稳定，C++ wrapper 仅做转发，删除后不影响调用方。 |
| **回退能力** | ✅ 由 Git 保障 | `git checkout v0.2.11` 可在 30 秒内回退到完整回退代码状态；`git bisect` 可精确定位问题版本。 |
| **构建风险** | ⚠️ 中低 | 删除 wrapper 后，`#ifdef FCEUX11_RUST_ENABLED` 分支消失，CMake 逻辑简化，但需确保所有 CI 构建矩阵默认开启 Rust。 |
| **心智负担** | ✅ 降低 | 不再维护两套实现，开发者无需在修改时同步更新 C++ 和 Rust；`cargo test` 成为唯一 truth source。 |
| **社区贡献门槛** | ⚠️ 略升 | 新贡献者若只懂 C++，无法修改已 Rust 化模块；但可通过 `git blame` 追溯到 C++ 原始实现作为参考。 |

### 4.2 风险缓解措施

1. **版本标签锁定**：在删除任何 C++ 代码前，先在 Git 上打 `v0.2.11-legacy-fallback` 标签，并写入 `git tag -a` 附注说明。
2. **CI 矩阵强化**：确保 CI 在 `FCEUX11_ENABLE_RUST=ON` 和 `OFF`（仅保留未 Rust 化模块）两种配置下构建通过；删除回退代码后，保留 `OFF` 路径用于无 Rust 工具链的极端场景，但仅编译未迁移模块。
3. **文档化接口契约**：每个 FFI 函数在 `fceux11_rust.h` 和对应 Rust `lib.rs` 文档注释中，明确写明：
   - 指针是否可为 null
   - 缓冲区长度责任方
   - 返回值生命周期（立即复制 / 由 Rust 管理 / 由 C++ 释放）
4. **分批次删除**：不按"大爆炸"方式一次性删除所有 11 个 wrapper，而是每个版本删除 2–3 个，便于 `git bisect` 定位。

---

## 五、架构优化路线图（为 Rust 重构铺路）

当前单 Crate `staticlib` 模式在 v0.2.11 阶段运行良好，但继续扩展到 30+ 模块时将遇到以下瓶颈：

- **编译时串行化**：任何 `.rs` 文件修改触发全 crate 重编译。
- **头文件同步负担**：手动维护 `fceux11_rust.h`，模块数 >20 后极易出现"函数签名已改、头文件未更新"的链接错误。
- **依赖混杂**：工具模块（`md5`、`guid`）与核心模拟模块将共享同一个 `Cargo.toml`，不利于后续独立发布 `fceux11-core` crate。
- **测试隔离性差**：`cargo test` 编译所有模块，即使只修改了 `palette.rs`。

### 5.1 Workspace 拆分（v0.2.13–v0.2.14）

```
src/rust/
├── Cargo.toml                    # Workspace 根
├── fceux11_rust.h                # 仍保留为统一头文件，但由 cbindgen 生成
├── crates/
│   ├── fceux11-utils/            # 工具类：md5, guid, crc32, general, timestamp, os_utils
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── fceux11-formats/          # 格式解析：ines, unif, cart, nsf, fds, wave, emufile
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── fceux11-media/            # 音视频：filter, palette, video helpers
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── fceux11-debug/            # 调试辅助：profiler, cheat, conddebug, debug, debugsym, asm
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── fceux11-core/             # 未来核心：state, movie, input, ppu, apu, cpu, boards
│       ├── Cargo.toml
│       └── src/lib.rs
└── cbindgen.toml                 # cbindgen 配置
```

**拆分原则**：
- 每个 sub-crate 保持 `crate-type = ["staticlib", "rlib"]`，既供 C++ 链接，也供 Rust 内部依赖。
- crate 间依赖关系必须为 DAG，禁止循环依赖。
- `fceux11-core` 在 v0.2.30 阶段可为空壳或仅含接口 trait，为 v0.3.x 填充。

### 5.2 cbindgen 自动化头文件生成（v0.2.14）

当前手动维护 `fceux11_rust.h` 的模式在规模扩大后不可持续。引入 [cbindgen](https://github.com/mozilla/cbindgen) 自动生成：

```toml
# cbindgen.toml
language = "C"
cpp_compat = true
include_version = true
header = "/* Auto-generated by cbindgen. Do not edit. */"

[parse]
parse_deps = true

[export]
prefix = "fceux11_rust_"
```

**关键配置**：
- `prefix` 确保所有导出符号带统一前缀，避免与 C++ 符号冲突。
- `cpp_compat = true` 生成 `extern "C"` 块，兼容 C++ 编译器。
- 在 CMake 中增加 `add_custom_command`，在 `cargo build` 之前调用 `cbindgen` 生成头文件。

### 5.3 引入 Slice 类型统一缓冲区传递（v0.2.13）

基于 R3 论文的启示，定义统一缓冲区描述符：

```rust
#[repr(C)]
pub struct FceuSlice {
    pub ptr: *mut u8,
    pub len: usize,
}

#[repr(C)]
pub struct FceuSliceMut {
    pub ptr: *mut u8,
    pub len: usize,
}
```

所有涉及缓冲区读写的 FFI 函数（如 `palette.rs` 的 `draw_control_bars`）逐步从 `(ptr: *mut u8, w: i32, h: i32)` 迁移到 `(buf: FceuSliceMut, w: i32, h: i32)`，并在 Rust 侧用 `std::slice::from_raw_parts_mut` 做边界检查。

### 5.4 ROM 回归测试框架（v0.2.15）

单元测试无法覆盖模拟器的行为等价性。必须建立基于真实 ROM 的自动化回归测试：

1. **nestest 自动化**：像 nescore 项目一样，将 `nestest.nes` 作为 CI fixture，运行到地址 `$C66E`，验证 `$02`（官方指令结果）和 `$03`（非官方指令结果）均为 `0x00`。
2. **帧哈希比对**：对一批"金标准 ROM"（如 `donkey_kong.nes`、`super_mario_bros.nes`），在 C++ 和 Rust 混合构建下运行 60 帧，计算每帧像素缓冲区的 CRC32，与 v0.2.11 基线比对。
3. **录像往返测试**：对 `movie.cpp` 重构，需验证录制的 `.fm2` 文件与 C++ 版逐字节一致。

---

## 六、模块重构计划（v0.2.12 – v0.2.30）

### 6.1 阶段一：清理与架构重构（v0.2.12 – v0.2.15）

#### v0.2.12 — 不可逆化清理（Deletion of Legacy Fallbacks）

**目标**：删除已 Rust 化 11 个模块的 C++ 回退实现，彻底清理条件编译分支。

| 被删除/简化的 C++ 文件 | 说明 |
|------------------------|------|
| `src/utils/crc32.cpp` | 删除回退实现，保留 `#include "rust/fceux11_rust.h"` 的薄封装或直接内联到调用方。 |
| `src/utils/md5.cpp` | 同上。 |
| `src/utils/guid.cpp` | 同上。 |
| `src/utils/general.cpp` | 同上。 |
| `src/wave.cpp` | 同上。 |
| `src/drivers/common/os_utils.cpp` | 同上。 |
| `src/utils/ConvertUTF.c` | 同上。 |
| `src/utils/timeStamp.cpp` | 同上。 |
| `src/profiler.cpp` | 同上。 |
| `src/filter.cpp` | 同上。 |
| `src/palette.cpp` | 同上。 |

**风险控制**：
- 每个文件删除在独立 commit 中进行，commit message 标注 `"[irreversible] remove C++ fallback for XXX"`。
- 保留 `docs/legacy_fallback_v0.2.11/` 目录（可选），将删除的 C++ 文件原样归档，便于离线查阅。
- **不修改任何 FFI 接口签名**，确保 Rust 侧零变更。

#### v0.2.13 — Workspace 拆分与 Slice 类型引入

**目标**：将单 crate 拆分为 Workspace；引入 `FceuSlice` 类型。

**任务清单**：
1. 创建 `src/rust/crates/` 目录结构。
2. 将现有 11 个模块按功能域拆入 `fceux11-utils`、`fceux11-media`。
3. 在 `fceux11-utils` 中新增 `slice.rs`，定义 `FceuSlice` / `FceuSliceMut`。
4. 修改顶层 `CMakeLists.txt`，链接多个 `.lib`（或先保持单 `.lib`，由 Workspace 的 root crate 统一 re-export）。
5. 所有 CI 构建验证通过。

#### v0.2.14 — cbindgen 集成与头文件自动化

**目标**：消除手动维护头文件的负担。

**任务清单**：
1. 在 `Cargo.toml` `[dependencies]` 中引入 `cbindgen`（build dependency）。
2. 编写 `cbindgen.toml`，配置前缀、C++ 兼容、导出白名单。
3. 在每个 sub-crate 的 `build.rs` 中调用 `cbindgen::generate()`，输出到 `src/rust/fceux11_rust.h`。
4. 删除所有手动编写的 `extern "C"` 声明，验证生成的头文件与原有手动文件功能等价。
5. 文档更新：《Rust FFI 开发指南》新增"如何添加新模块"流程。

#### v0.2.15 — ROM 回归测试基线建立

**目标**：建立行为级正确性验证能力。

**任务清单**：
1. 在 `src/tests/` 新增 `rom_regression_test.cpp`，调用现有模拟器初始化逻辑，加载 `tests/fixtures/nestest.nes`。
2. 在 `src/rust/` 新增 `tests/rom_tests/`（Rust 侧 ROM 测试 runner，未来用于纯 Rust 核心）。
3. 生成"金标准帧哈希表"：对 5 个无版权争议 ROM 运行 60 帧，记录每帧 CRC32。
4. 将该基线数据提交到 `tests/fixtures/golden_hashes.json`。
5. CI 中新增 `rom_regression` job，混合构建后自动跑 ROM 测试。

### 6.2 阶段二：扩展重构（v0.2.16 – v0.2.25）

本阶段选择模块的核心标准：
- **代码量 ≤ 1,500 行**（可分块翻译）
- **非每帧高频路径**（允许初期性能波动）
- **调用扇出低**（不依赖复杂 C++ 模板/宏）
- **有明确数据格式或状态机**（Rust 的 `match` 和类型系统优势大）

#### v0.2.16 — EmuFile（`src/emufile.cpp`, 283 行）✅ 已完成

| 属性 | 详情 |
|------|------|
| **功能** | 模拟器文件抽象层，支持普通文件与内存文件的统一读写。 |
| **Rust 优势** | `std::io::Read` / `std::io::Write` trait 天然适合此抽象；可消除 `EMUFILE_MEMORY` / `EMUFILE_FILE` 的虚函数开销。 |
| **风险** | 低。被 `state.cpp`、`movie.cpp` 等调用，但接口清晰（`fread` / `fwrite` / `fseek` 风格）。 |
| **FFI 模式** | Opaque Pointer：返回 `*mut c_void` 作为文件 handle。 |

> **v0.2.16 执行记录**：
> - `fceux11-formats` crate 已创建，含 `EmuFileMem` + 23 个 FFI 函数 + 7 个单元测试（全部通过）。
> - C++ `EMUFILE_MEMORY` 因 `buf()`/`get_vec()` 被 `state.cpp`/`file.cpp`/`lua-engine.cpp` 等 10 余处直接调用，**头文件级迁移暂缓**；C++ 侧保持原实现以确保稳定性。
> - **决策变更**：v0.2.16 实证表明双 Agent 结对编程（Claude Code 编码 + Kimi 验证）在 MSVC 混合构建环境中修复开销大于收益，后续 v0.2.17–v0.2.30 统一由 Kimi Code CLI 负责编码、构建、测试与版本发布。详见 `docs/tech/rust_refactor_agent_spec.md`（已归档）。

#### v0.2.17 — VS UniSystem（`src/vsuni.cpp`, 430 行）✅ 已完成

| 属性 | 详情 |
|------|------|
| **功能** | VS System（街机版 NES）的 DIP 开关、调色板、保护芯片模拟。 |
| **Rust 优势** | DIP 开关状态机用 Rust `enum` + `match` 表达更安全；VS 调色板数据表用 `const` 数组。 |
| **风险** | 低。仅在加载 VS 游戏时激活，常规 NES 游戏不触发。 |

> **v0.2.17 执行记录**：
> - `VSUniGames` 数据库（37 个游戏条目）迁移至 Rust `const` 数组，含 MD5 匹配、mapper/mirroring/PPU 类型信息。
> - `FCEU_VSUniCheck` ROM 匹配逻辑通过 `fceux11_rust_vsuni_lookup` FFI 完成。
> - `FCEU_VSUniDraw` DIP 状态绘制迁移至 Rust。
> - `FCEU_VSUniToggleDIP`/`Coin`/`Service` 核心位操作迁移至 Rust，C++ wrapper 保留消息显示与全局变量更新。
> - 全局状态（`vsdip`, `coinon`, `coinon2`, `service`）、`SFORMAT` 序列化、`SetReadHandler` 回调注册、保护芯片模拟保留在 C++。
> - 新增 16 个单元测试（lookup_found/not_found/RBI/TKO, toggle_dip, coin, service, draw）。
> - `cargo test --workspace` 81 测试通过；`cmake --build` 377/377 成功；`ctest` 4/4 通过（ROM 回归无退化）。

#### v0.2.18 — UNIF 解析（`src/unif.cpp`, 642 行）

| 属性 | 详情 |
|------|------|
| **功能** | UNIF ROM 格式解析（较 iNES 更复杂的替代格式）。 |
| **Rust 优势** | 可用 `nom` 或手动 `match` 实现解析器组合子，消除 C 风格指针偏移算术；`Result<T, E>` 替代错误码。 |
| **风险** | 中低。UNIF 文件较罕见，但解析错误会导致 ROM 加载失败。 |

#### v0.2.19 — iNES 解析（`src/ines.cpp`, 1,208 行）✅ 已完成

| 属性 | 详情 |
|------|------|
| **功能** | 最主流的 NES ROM 格式解析，含 iNES 1.0/2.0、NES 2.0 扩展头。 |
| **Rust 优势** | 结构体布局用 `#[repr(C)]` 映射头文件 16 字节，剩余逻辑纯 safe Rust；`nom` 解析 iNES 2.0 的变长字段。 |
| **风险** | 中。这是 ROM 加载的第一入口，任何头文件解析错误都会导致游戏无法运行。但已有大量 ROM 可用于测试。 |

> **v0.2.19 执行记录**：
> - `fceux11-formats` crate 新增 `ines.rs` + `ines/ines_data.rs`：
>   - `FceuInesHeader` — `#[repr(C)]` 16 字节头结构体，含 `cleanup()` 方法（DiskDude/demiforce/Ni03 垃圾签名清除）。
>   - 静态数据库迁移：`bmap` 名称表（168 条）、`not_power2`（4 条）、`SetInput` CRC→控制器表（70 条）、`SetInputNes20` expansion→控制器表（25 条）、`BadROMImages`（40 条）、`sMasterRomInfo`（9 条）、`savie` 电池白名单（33 条）、`ines-correct.h` ROM 修正表（256 条）。
> - C++ `ines.cpp` 中以下逻辑替换为 Rust FFI 调用：
>   - `head.cleanup()` → `fceux11_rust_ines_header_cleanup`
>   - `SetInput()` → `fceux11_rust_ines_lookup_input_crc`
>   - `SetInputNes20()` → `fceux11_rust_ines_lookup_input_nes20`
>   - `CheckBad()` → `fceux11_rust_ines_check_bad`
>   - `CheckHInfo()` → `fceux11_rust_ines_check_hinfo`（返回 `FceuInesHInfoResult`，C++ 应用修正）
>   - `not_power2` 循环 → `fceux11_rust_ines_not_power2`
>   - Mapper 名称查询 → `fceux11_rust_ines_mapper_name`
> - 删除 C++ 侧未使用的 `INPSEL`、`INPSEL_NES20`、`BADINF`、`BadROMImages`、`savie`、`CHINF moo` 定义。
> - 新增 16 个单元测试（header cleanup ×3, mapper name ×3, not_power2, input CRC ×2, input NES20 ×2, check bad ×2, check hinfo ×2, databases nonempty）。
> - `cargo test --workspace` 114 测试全部通过；C++ 侧因环境限制未执行混合构建，但头文件已由 cbindgen 自动生成并验证签名匹配。

#### v0.2.20 — Cart 卡带管理（`src/cart.cpp`, 608 行）

| 属性 | 详情 |
|------|------|
| **功能** | 卡带（Cartridge）高层管理：PRG/CHR ROM 分配、Trainer、Battery RAM。 |
| **Rust 优势** | 内存分配可用 `Vec<u8>` 管理 PRG/CHR 数据，消除 `FCEU_malloc` / `FCEU_gmalloc`；Rust 所有权确保卡带生命周期与模拟器实例绑定。 |
| **风险** | 中。`cart.cpp` 是 `ines.cpp` 和 `unif.cpp` 的消费者，也是 Mapper 的初始化者。需确保 FFI 边界设计允许 C++ Mapper 代码继续运行。 |
| **里程碑意义** | **v0.2.20 是 ROM 加载管道的 Rust 化里程碑**：`ines` → `unif` → `cart` 全链路 Rust 化，C++ 侧仅剩 Mapper 绑定。 |

#### v0.2.21 — NSF 播放器（`src/nsf.cpp`, 657 行）

| 属性 | 详情 |
|------|------|
| **功能** | NSF（NES Sound Format）音乐文件播放。 |
| **Rust 优势** | 独立的播放状态机，与游戏模拟主循环解耦，适合 Rust 化。 |
| **风险** | 中低。NSF 文件格式固定，播放逻辑自包含。 |

#### v0.2.22 — 调试辅助双模块（`src/conddebug.cpp`, 506 行 + `src/asm.cpp`, 529 行）

| 属性 | 详情 |
|------|------|
| **功能** | 条件断点调试器、内联汇编器。 |
| **Rust 优势** | `asm.cpp` 的指令编码表天然适合 Rust `match` + `const` 数组；`conddebug.cpp` 的条件表达式求值可用 Rust `enum Expr` 表示 AST。 |
| **风险** | 低。仅在调试构建或用户主动开启调试器时激活。 |

#### v0.2.23 — Drawing（`src/drawing.cpp`, 525 行）

| 属性 | 详情 |
|------|------|
| **功能** | 模拟器内部绘图辅助：消息框、状态显示、图标绘制。 |
| **Rust 优势** | 像素操作可用 `&mut [u8]` slice 安全写入；字体数据表用 Rust `const` 数组。 |
| **风险** | 中低。涉及向 `XBuf` 像素缓冲区写入，需验证 stride 和边界。 |

#### v0.2.24 — Cheat 引擎（`src/cheat.cpp`, 937 行）

| 属性 | 详情 |
|------|------|
| **功能** | Game Genie / Pro Action Replay 作弊码解析与应用。 |
| **Rust 优势** | 作弊码编码/解码算法纯计算；作弊列表管理用 `Vec<CheatEntry>` 替代动态链表。 |
| **风险** | 中。涉及内存补丁（`write_byte` 到 CPU 地址空间），需确保 FFI 回调到 C++ 内存写入函数的行为一致。 |

#### v0.2.25 — 调试系统双模块（`src/debug.cpp`, 993 行 + `src/debugsymboltable.cpp`, 1,002 行）

| 属性 | 详情 |
|------|------|
| **功能** | 调试器核心、符号表管理（.dbg 文件解析）。 |
| **Rust 优势** | 符号表可用 `HashMap<String, Symbol>` 管理；`.dbg` 文件解析可用 `nom`。 |
| **风险** | 中。调试器与前端 GUI（Qt）耦合较多，需保留 C++ 侧 GUI 壳层，仅将后端逻辑移入 Rust。 |

### 6.3 阶段三：核心渗透与架构定型（v0.2.26 – v0.2.30）

本阶段模块代码量大、耦合深，**不追求完整 Rust 化**，而是"Rust 主导 + C++ 薄壳"或"完成 Rust 化但保留扩展接口"。

#### v0.2.26 — FDS 磁盘系统（`src/fds.cpp`, 948 行）

| 属性 | 详情 |
|------|------|
| **功能** | Famicom Disk System 模拟，含磁盘读写、IRQ 计时、BIOS 加载。 |
| **Rust 优势** | 磁盘镜像（`.fds`）解析是 I/O 密集型，Rust 的 `std::fs` + `Vec<u8>` 更安全；IRQ 状态机可用 `enum IrqState` 表达。 |
| **风险** | 中高。FDS 涉及与 CPU/APU 的 IRQ 交互，时序敏感。策略：Rust 化磁盘镜像加载与 IRQ 标志计算，但 IRQ 触发时机仍由 C++ 主循环控制（通过 FFI 读取 Rust 计算的 `irq_pending` 标志）。 |

#### v0.2.27 — Video 后处理（`src/video.cpp`, 804 行）

| 属性 | 详情 |
|------|------|
| **功能** | 视频输出后处理：分辨率缩放、全屏切换、截图。 |
| **Rust 优势** | 截图（PNG/BMP 写入）可用 `image` crate；视频缩放参数计算纯数学。 |
| **风险** | 中。与 Qt/SDL 前端窗口系统耦合，保留 C++ 侧窗口管理，Rust 负责像素缓冲区转换。 |

#### v0.2.28 — Movie 录像系统（`src/movie.cpp`, 2,041 行）

| 属性 | 详情 |
|------|------|
| **功能** | FM2 录像格式录制、回放、帧级精度控制。 |
| **Rust 优势** | FM2 文件格式是文本/二进制混合格式，适合 Rust 解析器；帧数据用 `Vec<FrameInput>` 管理，消除手动 `realloc`。 |
| **风险** | 高。Movie 系统与 `input.cpp`、`fceu.cpp`、`state.cpp` 深度耦合；录像不同步是致命 bug。策略：分两步——v0.2.28 先迁移文件 I/O 与格式解析，v0.2.29 再迁移帧级逻辑。 |

#### v0.2.29 — State 状态序列化（`src/state.cpp`, 1,543 行）

| 属性 | 详情 |
|------|------|
| **功能** | 即时存档（Save State）序列化与反序列化，使用 `SFORMAT` 宏系统描述内存布局。 |
| **Rust 优势** | `SFORMAT` 宏本质是反射式内存序列化，Rust 可用 `serde` + `bincode` 实现更安全的版本化序列化；但需保持与旧版存档格式兼容。 |
| **风险** | 高。State 系统被全代码库调用，且需精确序列化 CPU/PPU/APU 的内部寄存器。策略：v0.2.29 先实现"增量式 state"——Rust 管理序列化缓冲区与压缩（`zstd`/`lz4`），但具体字段的读写仍通过 FFI 回调到 C++ 侧注册函数；v0.3.x 再将各组件状态移入 Rust。 |

#### v0.2.30 — 核心边界设计与架构定型

**目标**：不新增大量 Rust 代码，而是完成 v0.2.x 系列的架构收尾，为 v0.3.x 的核心迁移奠基。

**任务清单**：
1. **核心抽象设计**：在 `fceux11-core` crate 中定义 `trait Cpu`、`trait Ppu`、`trait Apu`、`trait Mapper`，明确各组件的时钟接口、总线接口、中断接口。
2. **总线模拟器原型**：用 Rust 实现一个极简的 `struct Bus { wram: [u8; 0x800], prg_rom: Vec<u8> }`，验证 `read_u8` / `write_u8` 的 `match` 解码性能不低于 C++ switch。
3. **FFI 边界冻结**：从 v0.2.30 开始，所有已定义的 FFI 函数进入"冻结期"——签名变更需经过 RFC 式评审，确保 C++ 调用方稳定。
4. **文档与知识沉淀**：
   - 《FCEUX11 Rust 重构手册 v1.0》：涵盖 FFI 设计模式、Workspace 结构、测试策略、性能调优。
   - 《Mapper Rust 化可行性研究报告》：分析 `src/boards/` 下 50+ Mapper 的宏系统，提出替代方案（如 proc-macro 或 DSL）。
5. **版本发布**：`v0.2.30` 作为 LTS 里程碑，打 tag 并发布 Release Notes。

---

## 七、测试与质量保障策略

### 7.1 三层测试金字塔

```
        /\
       /  \     层3: ROM 回归测试（行为级）
      /____\        nestest, golden frame hashes, movie round-trip
     /      \   
    /________\   层2: C++ 集成测试（模块级）
   /          \      fceux11_smoke_test, mapper_load_test
  /____________\ 层1: Rust 单元测试（函数级）
 /              \    cargo test, #[test], property-based test
/________________\
```

### 7.2 各阶段测试要求

| 阶段 | 层1 Rust 单元测试 | 层2 C++ 集成测试 | 层3 ROM 回归测试 |
|------|-------------------|------------------|------------------|
| v0.2.12–v0.2.15 | 新增 Workspace 拆分后的编译测试 | CI 全矩阵构建 | 建立金标准基线 |
| v0.2.16–v0.2.25 | 每个新模块 ≥5 个 `#[test]` | 模块加载/功能冒烟 | 每版本跑 golden hashes，偏差即失败 |
| v0.2.26–v0.2.30 | 核心 trait 的 mock 测试 | State/Movie 往返测试 | nestest + 10 个商业 ROM 的 60 帧比对 |

### 7.3 性能不退化指标

对于音频（Filter）、图形（Palette）、文件 I/O（EmuFile）等模块：
- 使用 `cargo bench` 或 C++ 侧 `std::chrono` 计时，对比重构前后 1000 次调用的平均耗时。
- **阈值**：Rust 实现耗时不得超过 C++ 原实现的 **110%**；若超过，需在 Release Profile 中启用 `lto = "fat"`、`codegen-units = 1` 进行优化。

---

## 八、风险缓解与回退方案

### 8.1 已知高风险点

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **FFI 边界 ABI 漂移** | 链接错误或运行时崩溃 | cbindgen 自动生成头文件；CI 中增加 `abi-checker` 或至少链接验证。 |
| **Rust 整数溢出 panic** | Debug 构建下运行时崩溃 | 所有模拟器逻辑使用 `wrapping_*` 或 `overflowing_*`；Clippy lint 强制检查。 |
| **Movie/State 序列化不兼容** | 旧存档/录像无法加载 | 格式版本号 + 向后兼容解析路径；自动化 round-trip 测试。 |
| **C++ Mapper 宏系统无法对接** | v0.3.x 核心迁移受阻 | v0.2.30 完成 Mapper DSL 预研，不急于在 v0.2.x 实施。 |
| **编译时间剧增** | 开发体验下降 | Workspace 拆分并行编译；`sccache` 缓存；仅修改的 crate 重编译。 |

### 8.2 回退方案

尽管本项目已决定不可逆化，但仍需为"灾难性失败"预留路径：

1. **Git 标签回退**：任何严重 bug 可通过 `git revert` 或 `git checkout v0.2.11` 在数分钟内回退到全 C++ 实现。
2. **分支隔离**：每个 Phase 在独立 feature branch 开发（如 `feat/v0.2.16-emufile`），通过 PR 合并；`main` 分支始终可发布。
3. **特性开关保留**：即使删除 C++ 实现，仍在 CMake 中保留 `FCEUX11_ENABLE_RUST` 选项。当设置为 OFF 时，仅编译未 Rust 化的 C++ 模块，生成一个"功能受限但可运行"的构建（适用于无 Rust 工具链的极端环境）。

---

## 九、里程碑与验收标准

| 里程碑 | 版本 | 核心交付物 | 验收标准 |
|--------|------|-----------|----------|
| M1 | v0.2.12 | 11 个模块不可逆化 | C++ wrapper 文件删除；CI 构建时间缩短 ≥10%；零链接警告。 |
| M2 | v0.2.15 | Workspace + cbindgen + ROM 测试 | `cargo test --workspace` 通过；头文件自动生成与手动版 diff 为空；nestest 通过。 |
| M3 | v0.2.20 | ROM 加载管道 Rust 化 | `ines` + `unif` + `cart` 全在 Rust；加载 100 个 ROM 无失败；加载速度不退化。 |
| M4 | v0.2.25 | 调试/工具层 Rust 化 | `cheat` + `debug` + `debugsym` + `drawing` 在 Rust；调试器功能等价性人工验证通过。 |
| M5 | v0.2.28 | Movie 系统 Rust 化（文件 I/O 部分） | FM2 录制/回放文件与 C++ 版逐字节一致；录像加载通过 golden test。 |
| M6 | v0.2.30 | 架构定型与 v0.2.x 收官 | 核心 trait 设计文档合并；Rust 代码占比 ≥35%；所有 FFI 函数冻结；发布 v0.2.30 Release Notes。 |

---

## 十、参考文献

### 学术论文

1. **His2Trans**: "Build-Aware Incremental C-to-Rust Migration via Skeleton-First Translation and Historical Knowledge Reuse." *arXiv:2603.02617*, 2026. https://arxiv.org/abs/2603.02617
2. **C2SaferRust**: Nitin et al. "C2SaferRust: Transforming C Projects into Safer Rust with NeuroSymbolic Techniques." *arXiv:2501.14257*, 2025. https://arxiv.org/abs/2501.14257
3. **Rustine**: "Translating Large-Scale C Repositories to Idiomatic Rust." *arXiv:2511.20617*, 2025. https://arxiv.org/abs/2511.20617
4. **Code Quality Analysis**: "Code Quality Analysis of Translations from C to Rust." *arXiv:2602.00840*, 2025. https://arxiv.org/abs/2602.00840
5. **R3 (Rewrite it in Rust with Refinements)**: UCSD HotOS, 2024. https://goto.ucsd.edu/~rjhala/hotos-ffi.pdf
6. **CROWN**: Zhang et al. "Ownership-Guided C to Rust Translation." *CCS 2023*. （被 Rustine 引用）
7. **RustMap**: "Towards Project-Scale C-to-Rust Migration via Program Analysis and LLM." *arXiv:2503.17741*, 2025. https://arxiv.org/abs/2503.17741

### 工业实践与博客

8. **Meta C→Rust Migration**: "How Meta's Engineers Shifted a Billion-User Codebase from C to Rust." *ARTIBA*, 2025. https://www.artiba.org/intelligent-engineering-at-scale/how-metas-engineers-shifted-a-billion-user-codebase-from-c-to-rust
9. **Microsoft 2030 Rust Goal**: "Microsoft's Bold Goal: Replace 1B Lines of C/C++ With Rust." *The New Stack*, 2025. https://thenewstack.io/microsofts-bold-goal-replace-1b-lines-of-c-c-with-rust/
10. **Safe C++ Rust Interop**: "Safe C++ Rust Interop: FFI Boundaries That Do Not Rot." *stofu.io*, 2026. https://stofu.io/blog/safe-cplusplus-rust-interop-ffi-boundaries-that-do-not-rot.html
11. **KDAB Hybrid Rust/C++**: "Best Practices for Hybrid Rust/C++ Apps." *KDAB*, 2024. https://www.kdab.com/publications/bestpractices/best-practices-hybrid-rust-cpp-apps.html
12. **Linux Kernel Rust**: Weinan Li. "Rust and Linux Kernel ABI Stability: A Technical Deep Dive." 2026. https://weinan.io/2026/02/16/rust-kernel-abi-stability-analysis.html

### 模拟器与 Rust 技术博客

13. **TetaNES**: "NES Emulation in Rust: Designs and Frustrations." *Luke Works*, 2020. https://lukeworks.tech/tetanes-part-2
14. **Lochnes**: "I made a NES emulator in Rust using generators." *Kyle Lacy*, 2019. https://kyle.space/posts/i-made-a-nes-emulator/
15. **nescore**: "nescore — NES Emulator written in Rust." *Natesh's Dev Blog*, 2020. http://nnarain.github.io/2020/04/15/nescore-NES-Emulator-written-in-Rust.html
16. **Dave's NES Emulator**: "NES Emulator in Rust." *David Tyler's Blog*, 2020. https://blog.davetcode.co.uk/post/nes-emulator-rust/
17. **Rust Embedded Book**: "A little Rust with your C." *Rust Embedded Working Group*. https://docs.rust-embedded.org/book/interoperability/rust-with-c.html

### 工具文档

18. **cbindgen**: https://github.com/mozilla/cbindgen
19. **C2Rust**: https://c2rust.com/
20. **Corrosion** (CMake + Cargo): https://github.com/corrosion-rs/corrosion
