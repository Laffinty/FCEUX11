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
- Decimal mode（商业游戏不用，但 NES 硬件支持）
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
├── blargg cpu_instrs 子集（11 个 ROM：01-basics, 02-implied, ...）
└── 自定义 corner case：undocumented op、page-cross、decimal mode
```

**`nestest.nes`** 是 6502 验证的金标准：跑完指令后输出 256 字节状态（PC/A/X/Y/S/P/周期/指令码序列），与已知 golden 二进制对比。

### 3.2 Shadow Run（与 C++ CPU 对比）

```rust
// crates/vnesu11/tests/shadow_cpu.rs
// 同一个 ROM 同时跑 C++ 和 Rust CPU，每 1024 周期 diff 一次寄存器
```

需要：
- 一个测试 helper（与 kagami-qa-runner 类似）加载 ROM
- 一个 hook：每 N 周期读取 Rust CPU 状态
- 同样 hook 读 C++ X6502 全局状态
- diff，失败即报

**DoD**：nestest.nes golden 完全一致，shadow run 100 帧零 diff。

### 3.3 blargg 完整覆盖（177 ROM）

由 kagami-qa-runner 的 Oracle B 统一跑——Phase 1 只关心 CPU 子集（~30 ROM），完整覆盖推迟到 Phase 7 默认切换时。

---

## 4. 性能基准（[修订] S8——含强制门禁）

| 项目 | 目标 |
|------|------|
| blargg `cpu_instrs.nes` 跑完时间 | ≤ v1.17 baseline × 1.05 |
| `criterion::Bencher` 单条指令 dispatch | ≤ 20ns |
| 与 C++ X6502 同 ROM 跑 100 帧 | 帧时间差 ≤ 5% |

### 4.1 [修订] Phase 1 末强制门禁（新增）

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

### 5.3 Decimal mode：表查找而非计算

```rust
fn adc_decimal(&mut self, val: u8) {
    let lo = (self.a & 0x0F) + (val & 0x0F) + self.carry();
    let hi = (self.a >> 4) + (val >> 4) + if lo > 9 { 1 } else { 0 };
    self.set_flag(C, hi > 9);
    self.set_flag(V, ((self.a ^ val) & 0x80 == 0) && ((self.a ^ (lo + hi * 16)) & 0x80) != 0);
    self.a = ((lo & 0x0F) + ((hi & 0x0F) << 4)) & 0xFF;
}
```

或者查 200 项表（更慢但代码简单）——先实现正确性，再优化。

---

## 6. 风险

| 风险 | 严重度 | 缓解 |
|------|--------|------|
| Undocumented opcode 行为不一致 | 🟠 中 | nestest.nes + 自定义 corner case 测试 |
| 周期计数偏差 1-2 周期 | 🟠 中 | blargg instr_timing 子集 |
| IRQ 采样时序差 1 周期 | 🟠 中 | blargg interrupt_test |
| Decimal mode bug | 🟡 低 | 商业游戏不用；decimal test ROM 可选 |
| 性能不达标 | 🟠 中 | Phase 1 末 benchmark；不达标则加 dynarec（ADR-006 默认不做） |

---

## 7. DoD

- [ ] 151 条官方指令全部实现 + 测试
- [ ] 21 条 undocumented 指令全部实现
- [ ] `nestest.nes` golden 字节一致
- [ ] blargg `cpu_instrs` 11 ROM 全 PASS
- [ ] blargg `instr_timing` 5 ROM 全 PASS
- [ ] blargg `cpu_interrupts` 3 ROM 全 PASS
- [ ] shadow run 与 C++ X6502 100 帧零 diff
- [ ] `cargo test -p vnesu11` 全绿
- [ ] benchmark 与 v1.17 baseline 持平

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
