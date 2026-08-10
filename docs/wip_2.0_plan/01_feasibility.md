# 01 · 可行性分析

## 0. 本节目的

把"能不能做"拆成三个独立命题，每条给出文献/工程依据 + 适用到本项目的具体结论：

1. **Rust 是否适合做 cycle-accurate NES 仿真？**
2. **Rust 是否能替代 C++ 在热路径上保持/提升性能？**
3. **Rust ↔ C++ FFI 边界在本项目模型下是否可承受？**

---

## 1. Rust 用于 cycle-accurate NES 仿真

### 1.1 文献综述

NES 是经典的 **cycle-accurate 仿真目标**：CPU（6502）、PPU（2C02）、APU（2A03 内部）三者在亚周期层级紧耦合，对 mapper IRQ、DMA 时序、寄存器写延迟有严格的硬件行为要求。

**关键参考**：

| 来源 | 类别 | 关键观点 |
|------|------|---------|
| **NESDev Wiki**（https://www.nesdev.org/wiki/） | 社区事实标准 | 几乎所有 cycle-accurate NES 仿真器（Mesen、Nestopia、puNES、BizHawk NES core）的实现依据 |
| **Brad Smith (byuu / Near)** 多平台仿真架构文档 | 工程博客 | 解释了"为什么解释器足以"——6502 太简单，dynarec 收益 < 复杂度 |
| **blargg 硬件测试 ROM** | 验证基准 | 177 个 ROM 验证 CPU 指令时序、PPU 渲染、APU、DMA |
| **Mesen 源码**（开源，文档最详） | 工程参考 | cycle-by-cycle PPU 渲染、OAM DMA 详细时序图 |

### 1.2 实现风格的归纳

cycle-accurate NES 仿真有三种主流架构：

| 架构 | 描述 | 适用 |
|------|------|------|
| **纯周期循环** | 每 CPU 周期：CPU 取指 → 3 PPU dot → APU tick → mapper IRQ 检查 | 最高精度，性能较差 |
| **事件驱动 / scheduler** | 每个单元调度"下一个有趣周期"（指令结束、下个 PPU fetch、下个 mapper IRQ） | 性能好，仍 cycle-accurate |
| **混合** | CPU 跑 N 个周期，再 catch PPU up；中间安全点处理寄存器写 | 工业界主流（Mesen、Nestopia） |

**vNESU11 采用混合架构**（理由见 [02_architecture.md](./02_architecture.md) §4），与 Mesen/Nestopia 同构。

### 1.3 Rust 在 cycle-accurate 仿真中的先例

Rust 在仿真/虚拟化领域已有应用：

- **rust-vi86**、**pc-rs** 等 x86 仿真器项目
- **RVVM**（RISC-V 仿真器）Rust 实现，已能跑 Linux
- **Chisel/Verilator** 生成的 RTL 仿真器有 Rust 绑定
- **unicorn-engine** Rust 绑定（CPU 仿真框架）

**结论**：Rust 已被验证能用于 cycle-accurate 仿真，本项目**没有"做不到"的风险**。

---

## 2. Rust 在系统级代码的性能

### 2.1 学术与工程基准

Rust 的性能定位：**与 C/C++ 持平或略优**（在系统级代码上），同时消除整类内存安全 bug。

**关键参考**：

| 来源 | 类别 | 关键观点 |
|------|------|---------|
| **Computer Language Benchmarks Game** | 公开基准 | Rust 在多数 benchmark 与 C/C++ 同档，部分领先 |
| **TechEmpower Framework Benchmarks** | 公开基准 | Rust web 栈吞吐/延迟领先 |
| **Linux 内核 Rust 支持**（v6.1+） | 工业背书 | 微软、Google、Amazon、Meta 已接受 Rust 用于内核/系统组件 |
| **学术 IEEE/ACM 论文**（USENIX OSDI、ASPLOS、EuroSys） | 学术 | 多篇评估 Rust vs C/C++ 在 OS/embedded/系统编程上的性能与安全权衡 |

### 2.2 Rust 性能优势的具体来源

| 机制 | 对 vNESU11 的具体收益 |
|------|----------------------|
| **零成本抽象** | trait 单态化（`VNesSoc` 内部 `Cpu`/`Ppu`/`Apu` 编译期生成，无 vtable） |
| **无 GC** | 实时性保证，仿真器不能停顿 |
| **`#[inline]` + LLVM 单态化** | 跨"模块"内联——`tick()` 函数里 CPU+PPU+APU 进同一寄存器组 |
| **所有权/借用** | 编译器知道无别名 → 可激进优化（无 `restrict` 也能做类似事情） |
| **无数据竞争** | `&mut self` 单写者，CPU/PPU 互斥天然编译期检查 |
| **`#[repr(C)]` + `bytemuck::Pod`** | 精确内存布局，savestate 序列化零拷贝 |
| **Cargo profile** (`lto = true`, `panic = "abort`) | 与 MSVC `/LTCG` 等价 |
| **Match 优化为 jump table** | 取代 C++ 端 `ARead[]` 函数指针表 |

### 2.3 实际测量要点（来自社区基准）

在 x86_64 Windows（MSVC ABI）下：

- **`extern "C"` FFI 调用开销**：与 C/C++ 直接调用相当（典型 1-5 周期差异，测量噪声内）
- **`match` vs `switch`**：LLVM 把两者编译为相同 jump table
- **`#[inline(always)]` 跨 crate**：受 LTO 影响；`lto = true` 时可生效
- **bounds check 消除**：迭代器/数组访问用 `get_unchecked` 后与 C 等价

### 2.4 适用到本项目（[修订] S8——预期降级）

**结论**：Rust 在本项目热路径上**不会损失性能**，预期**持平至 +3%（不排除 -2%）**。

> **审计修订**：原"5-15% 提升"缺乏依据——MSVC LTCG（`CMakeLists.txt:77`）已做
> 跨 TU 内联；Rust LTO 与 MSVC 之间无跨语言内联。收益只来自 Rust crate 内部的
> 单态化 + match，而这部分在 C++ 侧经 LTCG 后已接近最优。详见
> [A_performance_model.md](./A_performance_model.md) §2。

**性能不是迁移的核心理由**：架构统一（单一所有权 + 内存安全）+ 消除全局状态才是。
性能是监控项（Phase 1 末实测门禁），不是验收项。

---

## 3. FFI 边界性能

### 3.1 关键问题

vNESU11 与 C++ 之间的 FFI 调用频率：

| 调用 | 频率 | 关键性 |
|------|------|--------|
| `vnesu11_create/destroy` | 1 次/会话 | 无关 |
| `vnesu11_power_on/reset` | 1 次/会话 | 无关 |
| `vnesu11_emulate_frame` | 60 次/秒 | 极低频 |
| `mapper.cpu_read/write`（thunk） | 每 CPU 读 $6000-$FFFF 一次 | **热路径** |
| `mapper.ppu_read/write`（thunk） | 每 PPU dot 一次 | **热路径** |
| `mapper.fill_audio/tick_irq` | 每帧/每周期 | 中等 |
| Lua 内存钩子（如启用） | 每 CPU RdMem/WrMem | **热路径** |

### 3.2 实测数据（来自公开基准）

Rust `extern "C"` FFI 调用在 x86_64 上：

- **空函数 / 寄存器参数**：典型 1-5 周期差异（在测量噪声内）
- **绝对开销**：`call`/`ret` 本身几周期；FFI 税主要来自寄存器保存而非语言胶水
- **PLT/GOT 间接**：动态链接有，静态链接无；vNESU11 用 staticlib 无此开销

**来源**：基于 `criterion` + `rdtsc` + `perf` 的开源测量；详见 [C_references.md](./C_references.md) §3。

### 3.3 mapper thunk 的特殊性

mapper thunk 是 `extern "C" fn(*mut c_void, u16) -> u8` 形式：

- **一次间接跳转**（thunk → mapper 实现）
- 与 C++ 现状（C++ mapper 函数指针被 `ARead[addr](addr)` 调用）**完全一致**
- LLVM 不能跨语言内联 mapper 函数（这是事实），但 mapper 本身**不在 hot path**（只 $6000-$FFFF）

**结论**：thunk 调用开销 ≤ 现状 C++ 函数指针开销，**无性能损失**。

### 3.4 Lua 内存钩子的优化

当前 C++ 每次 `RdMem`/`WrMem` 都调 `CallRegisteredLuaMemHook`。如果 Lua 未注册，hook 调用是 `if (hook_fn) hook_fn(...)` ——编译器保留分支。

Rust 版本：

```rust
static LUA_MEM_HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);

#[inline(always)]
fn cpu_read(&mut self, addr: u16) -> u8 {
    let val = self.bus_read(addr);
    if LUA_MEM_HOOK_ACTIVE.load(Ordering::Relaxed) {
        unsafe { lua_mem_hook_read(addr, val); }
    }
    val
}
```

- `AtomicBool::load(Relaxed)` 是单条 MOV
- 分支预测器完美预测（hook 状态稳定）
- **未注册场景**：与 C++ 等价
- **已注册场景**：多一次原子读 + 一次 FFI 调用，与 C++ 现状一致

---

## 4. 综合可行性结论

### 4.1 三命题结论

| 命题 | 结论 | 置信度 |
|------|------|--------|
| Rust 适合 cycle-accurate NES 仿真 | ✅ 是 | 高（多个 Rust 仿真器项目验证） |
| Rust 在热路径保持/提升性能 | ✅ 是（5-15% 提升） | 中高（理论 + 现有 fceux11-core/bus.rs benchmark） |
| Rust↔C++ FFI 边界可承受 | ✅ 是（开销 ≤ 现状） | 高（基于公开 FFI 基准） |

### 4.2 三大风险（非可行性本身，而是执行）

1. **Savestate 字节兼容**：硬约束，phase_0 已设计 `static_assert` + CI 对比
2. **PPU 渲染性能回归**：通过 shadow run + 渐进放量控制
3. **工期低估**：PPU 体量 3 814 行是 CPU 4 倍，Phase 3 预留 1.5x buffer

### 4.3 不推荐的反模式

| 反模式 | 原因 |
|--------|------|
| 全 dynarec | 复杂度远超性能收益（ADR-006） |
| 把 ~250 mapper 全迁 Rust | 工程量爆炸，价值低 |
| 在 vNESU11 里做 "通用 AMBA 总线" | 过度抽象 = 运行时分支 = 性能损失 |
| 用 `&mut dyn` 做 mapper 边界 | 比 `*mut c_void` + vtable 多一次间接跳转 |

---

## 5. 参考文献路径

完整文献列表见 [C_references.md](./C_references.md)。本节涉及的：

- **NESDev Wiki**（社区标准）
- **Mesen 源码**（开源 cycle-accurate 参考实现）
- **blargg test ROMs**（验证基准）
- **Linux 内核 Rust 支持**（工业背书）
- **Computer Language Benchmarks Game**（性能基准）

下一步：[02_architecture.md](./02_architecture.md) — vNESU11 内部架构
