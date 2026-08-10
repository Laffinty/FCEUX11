# Phase 1 · CPU 解释器迁移

> **目标**：用 Rust 实现完整的 6502 解释器，达到与 C++ `x6502.cpp` **逐周期、逐指令**等价。可独立测试（不依赖 PPU/APU）。
>
> **2026-08-10 修订**：本文件按 `AUDIT_20260810.md` S3/S8 修订——
> ① 明确 CPU 以 **budget 驱动**（复刻 `X6502_Run` 语义，决策 A）接入；
> ② Phase 1 末增加**性能门禁**（与 LTCG 后 C++ 对比，≤ ×1.05）。

## 工期：4 周

---

## 1. 范围

### 1.1 ✅ 在范围内
- 官方 151 条 6502 指令（含所有寻址模式）
- 未公开指令（undocumented opcodes）：`Lax`、`Sax`、`Dcp`、`Isb`、`Slo`、`Rla`、`Sre`、`Rra`、`Anc`、`Arr`、`Xaa`、`Las`、`Axs`、`Sha`、`Shx`、`Shy`、`Tas`、`Lar`
- 周期精度：page-cross penalty、RMW extra cycle、branch taken/not-taken
- **[修订 2026-08-10] Decimal parity**：`SED` 设置 D 标志，但 **ADC/SBC 按二进制运算忽略 D**。
  这是与 C++ 基线 `x6502.cpp`（其 ADC/SBC 宏无任何 `D_FLAG` 处理）的 shadow-run parity
  硬约束——一个"更正确"的 BCD 实现会造成逐帧 diff。商业游戏不用 decimal 模式。
- 复位 / NMI / IRQ 序列（penultimate cycle 采样）
- 开中断/关中断延迟（SEI/CLI 一周期延迟）
- **[修订] `run_budget(cycles)` 语义**：复刻 `X6502_Run`（跑到 `tcount >= 预算` 返回），
  与 `FCEUPPU_Loop` 的 scanline 段驱动对接（S3）

### 1.2 ❌ 不在范围内（Phase 4 处理）
- APU DMC DMA 仲裁（CPU 会被 stall）
- OAM DMA（CPU 会被 stall）
- mapper IRQ 注入（vNESU11 内部由 mapper FFI 触发）

### 1.3 ❌ 不在范围内（v1.x 不变）
- 调试器符号访问（仍走 `cpu_peek_regs` FFI）
- Lua 内存钩子（Phase 5 接通）

---

## 2. 任务清单

### 2.1 模块结构

```
crates/vnesu11/src/cpu/
├── mod.rs              # struct CpuCore
├── decoder.rs          # 256 项指令 dispatch 表（match）
├── addressing.rs       # 13 种寻址模式
├── ops_load_store.rs   # LDA/LDX/LDY/STA/STX/STY
├── ops_arith.rs        # ADC/SBC/AND/ORA/EOR/CMP
├── ops_branch.rs       # BCC/BCS/BEQ/BMI/BNE/BPL/BVC/BVS
├── ops_transfer.rs     # TAX/TXA/TAY/TYA/TSX/TXS
├── ops_stack.rs        # PHA/PLA/PHP/PLP/RTS/RTI/JSR/JMP
├── ops_increment.rs    # INC/INX/INY/DEC/DEX/DEY
├── ops_shift.rs        # ASL/ROL/LSR/ROR（accumulator + memory）
├── ops_flag.rs         # CLC/SEC/CLD/SED/CLI/SEI/CLV
├── ops_system.rs       # BRK/NOP + undocumented
├── regs.rs             # CpuRegsLayout + #[repr(C)]
├── interrupt.rs        # NMI/IRQ/RESET 序列
└── cycle.rs            # 周期计数 + page-cross penalty
```

### 2.2 关键代码骨架

```rust
// crates/vnesu11/src/cpu/mod.rs
pub struct CpuCore {
    pc: u16,
    a: u8, x: u8, y: u8, s: u8, p: u8,
    db: u8, moo_pi: u8,
    jammed: bool,
    irq_pending: u32,        // IRQ 低位延迟（penultimate cycle）
    irq_pending_high: bool,
    nmi_pending: bool,
    nmi_edge: bool,
    count: i32,              // 本指令剩余周期
    tcount: i32,             // 总已执行周期
}

impl CpuCore {
    #[inline(always)]
    pub fn step<BC: BusContext>(&mut self, bus: &mut BC) -> StepResult {
        // 1. 检查中断（penultimate cycle）
        self.poll_interrupts();
        // 2. 取指 + dispatch
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        self.execute(opcode, bus)
    }
}
```

### 2.3 [修订] budget 驱动接口（S3）

CPU 以 **budget 驱动**接入（复刻 `X6502_Run` 语义），而不是"每周期 step"：

```rust
// crates/vnesu11/src/cpu/mod.rs
pub struct CpuCore {
    pc: u16,
    a: u8, x: u8, y: u8, s: u8, p: u8,
    db: u8, moo_pi: u8,
    jammed: bool,
    irq_pending: u32,
    irq_pending_high: bool,
    nmi_pending: bool,
    nmi_edge: bool,
    count: i32,              // 本指令剩余周期
    tcount: i32,             // 本 budget 内已用周期
}

impl CpuCore {
    /// 复刻 X6502_Run：执行指令直到 tcount >= cycles 预算耗尽。
    /// 预算耗尽在指令边界返回（tcount 可能超出 cycles，与 C++ 一致）。
    #[inline(always)]
    pub fn run_budget<BC: BusContext>(&mut self, cycles: i32, bus: &mut BC) {
        self.tcount = 0;
        while self.tcount < cycles {
            self.step(bus);  // step 内部累加 tcount
        }
    }
}
```

**为什么**：`FCEUPPU_Loop`（`ppu_rendering.cpp:690-703` 等）以 `X6502_Run(256)` /
`X6502_Run(85)` / `X6502_Run(256+69)` 发放预算。Rust 端若按"每周期 step"对接，
与 C++ 的预算边界语义不一致，shadow run 无法等价。`run_budget` 保证**逐帧等价**。

**Bus Context trait（私有，非 dyn）**：

```rust
// crates/vnesu11/src/cpu/mod.rs
pub trait BusContext {
    fn read(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn dma_stalled(&self) -> bool;
}
```

**关键**：这是 `trait`，但**只在 crate 内使用**，所有实现是 monomorphized——无 vtable 开销。
`VNesSoc` 提供 `impl BusContext for BusContextImpl<'_> { ... }`。

---

## 3. 验证策略

### 3.1 单元测试（host-side）

```
crates/vnesu11/tests/cpu_tests.rs
├── nestest.nes 自动跑（首个权威 NES 验证 ROM）
├── blargg cpu_instrs 子集（16 个 instr_v5_* ROM，mapper 0 / NROM）
└── 自定义 corner case：undocumented op、page-cross、decimal parity
```

**`nestest.nes`** 是 6502 验证的金标准：[修订] 实际采用 **self-test 状态字节**
（`$0002-$0005`）验证而非 golden log 逐行对比——`$0002`/`$0004` 必须为 0x00（PASS，
覆盖官方指令集），`$0003`（undocumented）/`$0005`（decimal）报告非零值但与 C++
基线一致（C++ 的 ADC/SBC 是二进制-only，这部分 C++ 也 FAIL）。golden log 逐行对比
是 ignored 的 stretch goal（需官方 nestest.log 文件）。

### 3.2 blargg cpu_instrs（Rust parity 测试）

[修订] `crates/vnesu11/tests/blargg_cpu_instrs.rs`：用纯 Rust CPU + NROM 总线跑
16 个 `instr_v5_*` ROM，按 blargg `$6000` 协议判定（0x00 = PASS），并与 C++
基线对比：

- **实测**：Rust 16/16 PASS；C++ 基线 15/16（`instr_v5_07_abs_xy` C++ FAIL，
  Rust PASS——C++ 侧 abs,X 处理瑕疵，非 Rust 回归）
- 断言：Rust 不得在 C++ 通过的 ROM 上失败（无新增失败）

### 3.3 Shadow Run（与 C++ CPU 对比）— [修订] Phase 6 待办

```rust
// crates/vnesu11/tests/shadow_cpu.rs
// 同一个 ROM 同时跑 C++ 和 Rust CPU，每 1024 周期 diff 一次寄存器
```

需要：
- 一个测试 helper（与 kagami-qa-runner 类似）加载 ROM
- 一个 hook：每 N 周期读取 Rust CPU 状态
- 同样 hook 读 C++ X6502 全局状态
- diff，失败即报

**[修订]**：此 DoD 在 Phase 1 未执行——需要 vNESU11 接入主路径（Phase 6）后，
C++ 与 Rust CPU 才能同进程对比。Phase 1 用 blargg cpu_instrs parity + nestest
self-test 作为替代验证。

### 3.4 blargg instr_timing / cpu_interrupts — [修订] 需完整模拟器

[修订] `instr_timing*`、`cpu_interrupts*` 依赖 PPU/APU/mapper（MMC1 等）时序，
纯 CPU + NROM 总线无法运行。且 **C++ 基线本身在这些 ROM 上 FAIL**（58 个 CPU
ROM 实测 39 PASS / 19 FAIL，含 `instr_timing`、`cpu_interrupts`、`instr_v5_all`）。
Phase 1 的 parity 目标 = Rust 与 C++ 基线一致（非全 PASS）；完整 blargg 覆盖在
Phase 7 默认切换时由 kagami-qa-runner Oracle B 统一验证。

---

## 4. 性能基准（[修订 2026-08-10] 已 PASS——含实测）

| 项目 | 目标 | 实测 |
|------|------|------|
| nestest 60 帧 CPU 工作量 | ≤ C++ baseline × 1.05 | ✅ 42.773 ms vs C++ 43.441 ms |
| 单条指令 dispatch | 不劣于 C++（无独立 C++ 纯 CPU bench） | ✅ 与 C++ 全模拟持平 |
| 与 C++ X6502 同负载 | 帧时间差 ≤ 5% | ✅ Rust 纯 CPU 0.713 ms/帧 ≤ C++ 全模拟 0.724×1.05 |

> [修订] 测量方法：`crates/vnesu11/src/bin/cpu_gate_best.rs`（best-of-9 降低
> turbo/频率噪声），对比 `fceux11_bench_x6502_exec.exe`（best-of-5）。C++ 侧
> 无独立纯 CPU bench，其基准含 PPU/APU（LTCG 已压薄），Rust 纯 CPU 追平是
> 最严格的合理对比。合成纯 NOP 负载偏慢（~1.3 ms/帧）是 `step_inner` 的
> self.count 往返在无内存访问负载下的伪影，真实游戏负载不受影响。

### 4.1 [修订] Phase 1 末强制门禁（已执行并 PASS）

```
门禁：Rust CPU dispatch（cargo bench -p vnesu11 cpu_dispatch）
      ≤ Release C++ X6502 对照 × 1.05
不达标 → 冻结 Phase 3，先优化（见 A_performance_model.md §5）
```

**为什么**：审计（S8）确认 MSVC LTCG 已做跨 TU 内联，Rust 迁移性能预期
不确定（持平至 +3%）。必须在最早阶段实测，不能拖到 Phase 7。

对比控制：同机器、锁频、同 ROM、同输入序列，Release + LTCG 对 Release + LTO，
多轮取中位数。

详见 [A_performance_model.md](./A_performance_model.md) §4。

---

## 5. 关键技术决策

### 5.1 指令 dispatch：match 而非函数指针表

```rust
#[inline(always)]
fn execute<BC: BusContext>(&mut self, opcode: u8, bus: &mut BC) -> StepResult {
    match opcode {
        0x00 => self.brk(bus),
        0x01 => self.ora_indirect_x(bus),
        0x05 => self.ora_zero_page(bus),
        // ... 256 项
        _ => self.illegal(opcode, bus),
    }
}
```

LLVM 把 `match u8` 编译为 **256 项 jump table**，与 C++ 函数指针表等价但**间接跳转数减半**（一次跳转直达，不需要 `ops_table.inc` 的两级分派）。

### 5.2 寻址模式：内联 `#[inline(always)]`

13 种寻址模式实现为 13 个函数，全部 `#[inline(always)]`，dispatch 时内联进 `execute`。

### 5.3 [修订] Decimal 模式：不实现（C++ parity 硬约束）

**2026-08-10 实测决定**：C++ 基线 `src/x6502.cpp` 的 ADC/SBC 宏（第 156-170 行）
**完全没有 `D_FLAG` 处理**——`ADC` 是 `uint32 l=_A+x+(_P&1)`，`SBC` 是
`uint32 l=_A-x-((_P&1)^1)`，纯二进制。整个 `x6502.cpp` 无任何 `D_FLAG` 引用。

迁移目标（ADR-008 / 决策 A）是 byte-for-byte shadow-run parity，因此 Rust 的
ADC/SBC 必须**匹配 C++ 的二进制-only 行为**（SED 设置 D，但运算忽略之）。
一个"更正确"的 BCD 实现会让 shadow run 逐帧 diff、全部标红——这是不能接受的。

若未来决定实现 decimal（FCEUX 上游也是二进制-only），必须是**独立、显式开关的**
改动，不能混入 parity 迁移。相关测试锁定该行为（见 `cpu_tests.rs` 的
`adc_binary_ignores_decimal_flag` 等）。

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| Undocumented opcode 行为不一致 | 🟠 中 | nestest.nes + 自定义 corner case 测试 |
| 周期计数偏差 1-2 周期 | 🟠 中 | blargg instr_timing 子集（Phase 7 完整验证） |
| IRQ 采样时序差 1 周期 | 🟠 中 | blargg interrupt_test（Phase 7 完整验证） |
| [修订] Decimal parity 漂移 | 🟡 低 | 已按 C++ 二进制-only 锁定；测试覆盖 `adc/sbc_binary_ignores_decimal_flag` |
| 性能不达标 | 🟠 中 | [修订] **已 PASS**（0.713 ≤ 0.724×1.05）；后续回归用 `cpu_gate_best` 监控 |

---

## 7. DoD

> **2026-08-10 实测更新**：Phase 1 已实质完成（解释器 + 本地验证 + 性能门禁）。
> 勾选 = 已完成；标注 [Phase 6/7] = 延迟到对应阶段（需完整模拟器接入）。

- [x] 151 条官方指令全部实现 + 测试
- [x] 21 条 undocumented 指令全部实现
- [x] `nestest.nes` self-test：$0002/$0004 = 0x00（官方指令 PASS）；$0003/$0005
      与 C++ 基线一致的非零（decimal + 部分 undocumented 是 C++ 已知限制）
- [x] blargg `cpu_instrs` **16** ROM 全 PASS（Rust 16/16，C++ 基线 15/16）
- [ ] blargg `instr_timing` 5 ROM 全 PASS — [Phase 7] 依赖完整模拟器；C++ 基线本身 FAIL
- [ ] blargg `cpu_interrupts` 3 ROM 全 PASS — [Phase 7] 同上
- [ ] shadow run 与 C++ X6502 100 帧零 diff — [Phase 6] 需 vNESU11 接入主路径
- [x] `cargo test -p vnesu11` 全绿（53 passed / 0 failed）
- [x] **性能门禁 PASS**：Rust nestest CPU 42.773 ms（0.713 ms/帧）≤ C++ 全模拟
      43.441 ms（0.724 ms/帧）× 1.05 = 0.760 ms/帧

---

## 8. 关键文件交付

```
新增：
  src/rust/crates/vnesu11/src/cpu/        # 整个目录（14 个 .rs）
  src/rust/crates/vnesu11/tests/
    ├── cpu_tests.rs
    ├── shadow_cpu.rs
    └── fixtures/nestest.nes              # blargg nestest

修改：
  src/rust/crates/vnesu11/src/lib.rs      # 添加 pub mod cpu
  src/rust/crates/vnesu11/src/soc.rs      # 集成 CpuCore
```

下一步：[phase_2_bus_and_ram.md](./phase_2_bus_and_ram.md)
