# FCEUX11 Rust 重构手册 v1.0

> **版本**：v1.0  
> **对应代码版本**：v0.2.30  
> **适用范围**：所有参与 FCEUX11 Rust 重构的开发者与代码贡献者

---

## 目录

1. [项目背景与目标](#1-项目背景与目标)
2. [Workspace 结构](#2-workspace-结构)
3. [FFI 设计模式](#3-ffi-设计模式)
4. [测试策略](#4-测试策略)
5. [性能调优](#5-性能调优)
6. [版本发布流程](#6-版本发布流程)
7. [v0.3.x 迁移指南](#7-v03x-迁移指南)

---

## 1. 项目背景与目标

FCEUX11 从 v0.2.1 开始逐步将 C++ 模块迁移至 Rust。截至 v0.2.30，已完成 **16 个模块** 的 Rust 化，累计约 **6,500+ 行 Rust 代码**、**200+ 个单元测试**。

### 1.1 已完成模块清单

| Crate | 模块 | 功能域 | 代码行数(约) | 测试数 |
|-------|------|--------|-------------|--------|
| fceux11-utils | CRC32 | 哈希 | 29 | — |
| fceux11-utils | MD5 | 哈希 | 366 | ~6 |
| fceux11-utils | GUID | UUID | 200 | ~2 |
| fceux11-utils | General | 数学 | 30 | ~1 |
| fceux11-utils | TimeStamp | 计时 | 73 | ~4 |
| fceux11-utils | OS Utils | 系统调用 | 131 | ~5 |
| fceux11-utils | ConvertUTF | 编码转换 | 1004 | ~20 |
| fceux11-media | Wave | 音频导出 | 225 | ~3 |
| fceux11-media | Filter | 音频滤波 | 423 | ~6 |
| fceux11-media | Palette | 调色板 | 385 | ~8 |
| fceux11-debug | Profiler | 性能分析 | 236 | ~4 |
| fceux11-formats | EmuFile | 文件 I/O | 614 | ~7 |
| fceux11-formats | VS UniSystem | VS 街机 | 430 | 16 |
| fceux11-formats | iNES | ROM 解析 | 1208 | 16 |
| fceux11-formats | Movie | FM2 格式 | 1576 | — |
| fceux11-debug | Cheat | 作弊引擎 | 1099 | 20 |
| fceux11-core | StateFile | 存档格式 | 600 | 7 |

### 1.2 核心原则

1. **叶子优先**：优先迁移无交叉依赖的工具/格式模块。
2. **不可逆化**：v0.2.12 起，每个模块 Rust 化后删除 C++ 回退代码。
3. **C++ 薄壳**：保留 C++ 侧调用封装，核心逻辑在 Rust。
4. **行为等价**：模拟器输出必须逐位一致，性能退化不得超过 110%。

---

## 2. Workspace 结构

```
src/rust/
├── Cargo.toml                    # Workspace 根
├── cbindgen.toml                 # cbindgen 配置
├── build.rs                      # 头文件自动生成
├── fceux11_rust.h                # 合并后的 C 头文件 (Auto-generated)
├── crates/
│   ├── fceux11-utils/            # 工具类
│   ├── fceux11-formats/          # 格式解析 (iNES, UNIF, FM2, EmuFile)
│   ├── fceux11-media/            # 音视频 (Filter, Palette)
│   ├── fceux11-debug/            # 调试辅助 (Profiler, Cheat, ASM)
│   ├── fceux11-lua/              # Lua 绑定
│   └── fceux11-core/             # 核心模拟器抽象 (v0.3.x 主场)
└── tests/                        # Rust 侧集成测试 (预留)
```

### 2.1 Crate 依赖规则

- **必须构成 DAG**，禁止循环依赖。
- `fceux11-core` 在 v0.2.30 阶段仅含 trait + state_file，v0.3.x 逐步填充 CPU/PPU/APU/Mapper。
- 每个 sub-crate 保持 `crate-type = ["rlib"]`，由 root crate 统一 re-export 为 `staticlib`。

### 2.2 添加新模块的流程

1. 在对应 crate 的 `src/` 下创建 `.rs` 文件。
2. 在 `lib.rs` 中 `pub mod` 声明。
3. 编写 `#[unsafe(no_mangle)] pub extern "C"` FFI 函数。
4. 运行 `cargo build` 触发 cbindgen，自动更新 `fceux11_rust.h`。
5. 在 C++ 侧包含 `#include "rust/fceux11_rust.h"` 并调用。
6. 编写 `#[test]` 单元测试。
7. 验证 `cargo test --workspace` + CMake 构建 + ROM 回归测试。

---

## 3. FFI 设计模式

截至 v0.2.30，项目中已形成 5 种稳定的 FFI 模式。

### 3.1 纯函数模式

**适用**：无状态、无 side-effect 的计算（CRC32、MD5、编码转换）。

```rust
#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_crc32(data: *const u8, len: usize) -> u32
```

**规则**：
- 所有指针参数必须检查 `is_null()`。
- 用 `std::slice::from_raw_parts` 构造 slice，长度由调用方显式传入。

### 3.2 Thread-Local 返回缓冲区模式

**适用**：返回可变长度字符串（GUID、文件名、错误信息）。

```rust
fn with_thread_local_buf<F: FnOnce(&mut [u8; 512]) -> *const c_char>(s: &str, f: F) -> *const c_char {
    thread_local! {
        static BUF: std::cell::RefCell<[u8; 512]> = std::cell::RefCell::new([0u8; 512]);
    }
    BUF.with(|buf| { ... })
}
```

**规则**：
- 缓冲区大小固定（通常 512 字节），超过部分截断。
- C++ 调用方应在返回后立即复制，不得长期持有指针。

### 3.3 Opaque Pointer + Handle 模式

**适用**：有状态的组件（Profiler、Filter、EmuFileMem、MovieData）。

```rust
pub struct ProfilerState { ... }
pub type ProfilerHandle = *mut ProfilerState;

#[unsafe(no_mangle)]
pub extern "C" fn fceux11_rust_profiler_create() -> ProfilerHandle { ... }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn fceux11_rust_profiler_destroy(h: ProfilerHandle) { ... }
```

**规则**：
- C++ 不得解引用 handle，只能回传。
- `create` 用 `Box::into_raw`，`destroy` 用 `Box::from_raw` + `drop`。

### 3.4 Slice 结构体模式 (v0.2.13 引入)

**适用**：传递大块缓冲区（像素、音频采样、ROM 数据）。

```rust
#[repr(C)]
pub struct FceuSlice {
    pub ptr: *const u8,
    pub len: usize,
}
```

**规则**：
- Rust 侧用 `std::slice::from_raw_parts` / `from_raw_parts_mut` 构造 slice。
- 长度必须与实际缓冲区严格匹配，禁止隐含长度约定。

### 3.5 批量回调 / Chunk 编排模式 (v0.2.29 引入)

**适用**：复杂数据结构的序列化（Savestate 文件格式）。

```rust
#[repr(C)]
pub struct FceuStateChunkInput {
    pub chunk_type: u8,
    pub data: *const u8,
    pub len: usize,
}
```

**规则**：
- Rust 负责文件外壳（header + chunk 编排 + 压缩）。
- C++ 负责每个 chunk 的 payload 生成/解析。
- 内存所有权：Rust 分配 → C++ 读取 → Rust 释放。

---

## 4. 测试策略

### 4.1 三层测试金字塔

```
        /\          层3: ROM 回归测试 (行为级)
       /  \            nestest, golden frame hashes, movie round-trip
      /____\
     /      \     层2: C++ 集成测试 (模块级)
    /________\        fceux11_smoke_test, mapper_load_test
   /          \
  /____________\  层1: Rust 单元测试 (函数级)
 /              \     cargo test, #[test], property-based test
/________________\
```

### 4.2 各层要求

| 层级 | 工具 | 触发条件 | 失败策略 |
|------|------|----------|----------|
| 层1 | `cargo test --workspace` | 每次 commit | 阻止合并 |
| 层2 | `ctest -C Release` | CI / 本地构建 | 阻止合并 |
| 层3 | ROM 回归 runner | CI nightly / 版本发布前 | 阻止发布 |

### 4.3 新模块测试 checklist

- [ ] `#[test]` 覆盖正常路径、边界条件、错误输入。
- [ ] 纯计算模块：≥5 个 `#[test]`。
- [ ] 涉及缓冲区的模块：null 指针安全测试、零长度测试、越界测试。
- [ ] 涉及文件格式的模块：round-trip 测试（序列化 → 反序列化 → 逐字节比对）。

---

## 5. 性能调优

### 5.1 基准原则

- 对比重构前后 1000 次调用的平均耗时。
- **阈值**：Rust 实现耗时不得超过 C++ 原实现的 **110%**。
- 测量方式：`cargo bench` 或 C++ 侧 `std::chrono`。

### 5.2 Release Profile 优化

`Cargo.toml` 已配置：

```toml
[profile.release]
panic = "abort"
lto = true
```

若仍超标，可尝试：
- `lto = "fat"`
- `codegen-units = 1`
- 对热点函数加 `#[inline(always)]`

### 5.3 Bus 解码性能 (v0.2.30 新增)

Rust `match addr { 0x0000..=0x1FFF => ... }` 在 release 模式下可被 LLVM 优化为跳转表。

`fceux11-core::bus::bench_simple_bus` 提供了可重复的微基准：

```bash
cd src/rust
cargo test -p fceux11-core --lib bus::tests::bench_runs_without_panic -- --nocapture
```

---

## 6. 版本发布流程

### 6.1 每次迭代 (v0.2.x)

1. 完成代码修改，确保 `cargo test --workspace` 通过。
2. 运行 cbindgen：`cargo build`（自动触发）。
3. 验证 C++ 侧编译通过。
4. 运行 `ctest -C Release`。
5. 运行 ROM 回归测试。
6. `git add -A && git commit -m "v0.2.xx: ..."`
7. `git tag -a v0.2.xx -m "..."`

### 6.2 禁止操作

- **不要** 在未经 ROM 回归测试的情况下修改核心模拟循环。
- **不要** 在 v0.2.x 阶段引入 nightly Rust 特性。
- **不要** 修改已冻结的 FFI 函数签名（见第 7 节）。

---

## 7. v0.3.x 迁移指南

### 7.1 目标架构

```rust
pub struct NesSystem<C: Cpu, P: Ppu, A: Apu, M: Mapper> {
    pub cpu: C,
    pub ppu: P,
    pub apu: A,
    pub mapper: M,
    pub wram: [u8; 0x800],
    pub prg_rom: Vec<u8>,
    pub chr_rom: Vec<u8>,
    pub cycle_count: u64,
}
```

### 7.2 迁移优先级 (建议)

1. **Bus + WRAM**（最小、零行为风险）
2. **CPU**（指令集固定， nestest 可验证）
3. **PPU**（渲染管线复杂，但帧哈希可验证）
4. **APU**（音频输出可波形比对）
5. **Mapper**（最后，数量多，需预研 DSL 方案）

### 7.3 FFI 冻结期 (v0.2.30+)

从 v0.2.30 开始，**所有已定义的 FFI 函数进入冻结期**。签名变更需满足：

- 在 `docs/tech/` 下提交 RFC 文档。
- 说明变更理由、影响范围、回退方案。
- 获得至少一次代码审查批准。
- 同步更新 C++ 所有调用方。

---

## 附录：快速参考

| 问题 | 解决方案 |
|------|----------|
| cbindgen 头文件未更新 | `cd src/rust && cargo build` |
| 新增 crate 未出现在头文件中 | 在 `build.rs` 中添加 cbindgen 调用 |
| C++ 编译报 undefined reference | 确认 `lib.rs` 中 `pub use` 了该 crate |
| 测试覆盖率不足 | 参考 `state_file.rs` 的 `tests` 模块 |
| 不确定用哪种 FFI 模式 | 参考第 3 节的决策树 |

---

*本文档随 v0.2.30 发布，后续版本如需更新，请在 `docs/tech/` 目录下创建 `rust_refactor_handbook_v1.1.md` 并链接本文档。*
