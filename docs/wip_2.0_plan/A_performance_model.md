# A · 性能模型与预期收益

> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S8 重写——原 5-15% 预期
> 缺乏依据（MSVC LTCG/PGO 已做跨 TU 内联；Rust LTO 与 MSVC 之间无跨语言内联）。
> **新结论：性能预期为「持平至 +3%，不排除 -2%」；性能不是迁移的核心理由。**

## 1. 概述

本附录量化 vNESU11 在不同路径上的预期性能变化。**修订后的核心论点**：

> 迁移的核心理由是**架构统一（单一所有权 + 内存安全）+ 消除全局状态**，
> 性能是**次要且不确定**的收益。若性能是硬目标，Phase 1 末必须实测再决定。

---

## 2. 为什么原 5-15% 预期不成立（S8）

### 2.1 MSVC LTCG 已做跨 TU 内联

`CMakeLists.txt:73-78`：

```cmake
# v1.14 Anvil §14.3 — Link-Time Code Generation for Release builds.
# /LTCG enables cross-TU inlining and whole-program optimization.
if(NOT FCEUX11_ASAN AND NOT FCEUX11_UBSAN)
    set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON)
endif()
```

**结论**：C++ Release 构建的跨 TU 内联**已经生效**（PGO 可选进一步强化）。
"消除跨 TU 边界 → 5-15%"的论据在 LTCG 面前不成立——C++ 侧早已不是
"每跨一次 TU 就一次 ABI 调用"。

### 2.2 Rust LTO 与 MSVC 之间无跨语言内联

- `Cargo.toml:11` 的 `lto = true` 是 **rustc 内部 LLVM 链接**（Rust crate 之间）
- `vnesu11` 与 C++ 的链接是 **MSVC `link.exe`**，两者之间**不存在跨语言 LTO**
- 因此 `vnesu11_*` FFI 调用、mapper handler thunk 调用**都是真实 ABI 边界**，
  Rust 编译器无法跨过去内联

**结论**：Rust 迁移的收益只来自"**Rust crate 内部**的单态化 + match + 所有权优化"。
而这部分在 C++ 侧经 LTCG 后**已接近最优**——`ARead[]` 表的间接调用被
`__forceinline`（`bus.h:74-79`）包裹，机器码与 Rust `match` 等价。

### 2.3 现有 benchmark 证据不足

`crates/fceux11-core/src/bus.rs` 的 benchmark 只证明了：
- **"Rust `match` 不比 C++ `switch` 慢"**（jump table 等价）

**没有证明**：
- "Rust 比 LTCG 后的 C++ 快"
- "跨 TU 消除带来实际收益"

---

## 3. 修订后的性能预期

### 3.1 分路径预期

| 路径 | 预期变化 | 依据 |
|------|---------|------|
| CPU dispatch | 持平（±1%） | 两边都是 jump table + 内联指令 |
| PPU 渲染 | **+0% 至 +5%**（不排除 -2%） | Rust 单态化可能比 C++ LTCG 略优，但无实证 |
| APU | 持平 | 周期频率低 |
| Mapper FFI | 持平 | thunk 与函数指针等价 |
| **综合帧时间** | **持平至 +3%，不排除 -2%** | 各路径加总 |

**注意**：这个预期是"迁移不改模型"（决策 A，budget 复刻）下的数字。
如果未来做 dot 级重写（决策 B），性能可能 +5-15%，但那是**行为重写**的收益，
不是迁移本身的收益，且风险大。

### 3.2 什么情况下可能反而变慢

| 场景 | 原因 |
|------|------|
| Rust 端过度抽象（`dyn` / 泛型滥用） | 虚表 / 单态化膨胀 |
| 区间表线性扫描退化 | mapper 注册超预期 |
| FFI 调用点遗漏（Lua hook 等） | 额外 ABI 边界 |
| Rust 端 debug 构建 | 无优化（Release 无此问题） |

---

## 4. 验证方法（修订：以实测为准）

### 4.1 Phase 1 末强制门禁（新增）

**Phase 1 DoD 增加**：
```
cargo bench -p vnesu11 cpu_dispatch   vs   Release C++ X6502 同 ROM 对比
门禁：Rust CPU dispatch 时间 ≤ C++ × 1.05
不达标 → 冻结 Phase 3，先优化
```

这是审计修订的核心新增——**性能预期不确定，就必须在最早阶段实测**。

### 4.2 Microbenchmark

```rust
// crates/vnesu11/benches/cpu_dispatch.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_cpu_dispatch(c: &mut Criterion) {
    let mut cpu = CpuCore::new();
    let mut bus = MockBus::new_with_prg(vec![0xEA; 1024]);  // NOP sled
    c.bench_function("cpu_dispatch", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                cpu.run_budget(1000);
            }
        });
    });
}
```

### 4.3 Frame-level benchmark

```bash
# Release + PGO + LTO（Phase 7 用）
build-release/src/fceux11.exe -rom smb1.nes -bench 60 -headless
# 输出：平均帧时间、最坏帧时间

# 对比基线（v1.17 Release，同机器同参数）
git checkout v1.17 && build-release/src/fceux11.exe -rom smb1.nes -bench 60
```

### 4.4 Profiling

```bash
# Rust 端
cargo flamegraph -p vnesu11 --release --bench ppu_segment

# C++ 端残留（Phase 7 末对比）
# VTune / perf 分析，报告 top-10 热点
```

### 4.5 基准控制（关键）

对比必须控制：
- **同机器、同 CPU 频率**（关 turbo / 锁频）
- **同构建配置**（Release + LTCG + PGO 对 Release + LTO）
- **同 ROM、同输入序列**
- 多轮取中位数（criterion 或 ≥ 10 次取中位）

---

## 5. 性能不达预期时的备选方案

### 5.1 备选 1：针对性优化（首选）

- Profile 找出 top-5 热点
- 每个热点针对性：`#[inline(always)]`、位运算重写、消除 bounds check
- 重新 benchmark

### 5.2 备选 2：SIMD tile fetch（中等侵入）

AVX2 一次处理 8 个 PPU dot：

```rust
#[cfg(target_arch = "x86_64")]
unsafe fn fetch_8_dots_avx2(...) {
    use std::arch::x86_64::*;
    // 一次 256-bit 操作处理 8 个 byte
}
```

**风险**：仅 Release 构建启用，调试时退化为标量路径。

### 5.3 备选 3：dot 级重写（决策 B）

如果迁移后性能**仍不达标且经过 5.1/5.2 后仍不达标**，才考虑把 budget 模型
重写为 dot 紧交错（+5-15%）。代价：
- 工作量大 2-3 倍
- 风险高（时序模型重写）
- **必须**单独立项评审

**默认不做**（ADR-008/ADR-006）。

---

## 6. 性能 DoD（修订）

| 指标 | 目标（修订） | 测量 |
|------|------------|------|
| blargg `cpu_instrs.nes` 总时间 | ≤ v1.17 × 1.05 | frame-level bench |
| SMB1 平均帧时间 | ≤ v1.17 × 1.05 | frame-level bench |
| PPU 段渲染（`ppu_segment` bench） | ≤ C++ LTCG 对照 × 1.05 | microbench |
| mapper 区间表 read | ≤ 20ns | microbench |
| 跨 FFI 总开销 | ≤ 5% CPU cycles | callgrind |

**说明**：修订后的 DoD 允许 ±5% 浮动（持平定义），把"性能收益"从验收项
降级为"监控项"。真正的验收项是**正确性等价**（savestate round-trip、
blargg 全量、shadow run 三级 diff）。
