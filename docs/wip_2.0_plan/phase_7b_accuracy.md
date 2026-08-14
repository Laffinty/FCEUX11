# Phase 7b · 精度攻关（T1 80.8% → 85%）

> **目标**：在 Phase 7a 发布 v2.0.0（精度 80.8% 冻结）的基础上，独立排期推进精度 oracle 至 v2.0.1 / v2.1 的 **T1 ≥ 85%** 硬门槛（150/177）。
>
> **2026-08-14 路径 B 决议**：精度攻关与发布解耦。Phase 7b 不是 Phase 7a 的前置——v2.0.0 可以先发，85% 门槛留给 hotfix / minor release。
>
> **依赖**：Phase 7a 发 v2.0.0；本阶段产物为 v2.0.1（hotfix）或 v2.1（minor）。

---

## 工期：4-5 周

---

## 1. 范围

### 1.1 ✅ 在范围内（推 7 PASS，达到 150/177 = 85%）

按 Phase 6 §9.1.0c 的剩余路径优先级修：

| 优先级 | 目标 | 期望 PASS | 性质 |
|---|---|---|---|
| P1.1 | MMC3 5 个 0x02 具体失败（mmc3_2_details / mmc3_5_MMC3 / mmc3_6_MMC6 / mmc3_v2_2_details / mmc3_v2_6_MMC3_alt） | +5 | C++ mapper 侧 IRQ 计数器 reload 逻辑（`mapper_meta_hblank_irq` 时序或 MapperMetaVtable::tick_irq 边缘） |
| P1.2 | CPU 5 个中断时序（cpu_int_2_nmi_brk / cpu_int_3_nmi_irq / cpu_int_4_irq_dma / cpu_int_5_branch_irq / cpu_dummy_writes_ppu） | +2~3 | Rust CPU IRQ/BRK/DMA 交互；需对照 blargg test ROM 源逐指令对照 |

P1 即可达成 150/177 = 85%（143 + 7 = 150）。

**溢出范围**（P2/P3，超过 150 后可继续推 90%）：

| 优先级 | 目标 | 期望 PASS | 性质 |
|---|---|---|---|
| P2.1 | sprdma×2（sprdma_dmc_dma / sprdma_dmc_dma_512） | +2 | per-cycle DMA 仲裁（513/514 stall 边界与 DMC 冲突） |
| P2.2 | MMC3 7 个 0x80 失败（mmc3_*_clocking + A12_clocking） | +5~7 | **A12 时钟 deep model**（PPU 地址 0x1000/0x1FF0 上升沿过滤） |
| P2.3 | PPU VBL 8 个（vbl_02..10） | +4~6 | **PPU dot 粒度时序**（sl 240 cycle 340 sub-scanline） |
| P2.4 | oam_stress + ppu_open_bus + ppu_read_buffer | +1~2 | PPU open-bus decay + OAM 时序 |

### 1.2 ❌ 不在范围内（明确划出）
- T1 ≥ 90%（v2.1+ 候选，Phase 7b 完成后再独立评估）
- MapperMetaVtable::fill_audio 真实实现（VRC6/FDS/N163 扩展音频）
- 完整 PPU/APU 5 通道 state mirror 扩展（仅在 Phase 7b 末验证需要时纳入）
- 真实游戏 smoke ROM 投放（owner 责任，Phase 7b 不阻塞精度攻关）

---

## 2. 任务清单

### 2.1 MMC3 IRQ 计数器 reload 深度（高优先级）

#### 2.1.1 根因诊断

Phase 6 §9.1.0 已知：
- `mmc3_2_details` 失败诊断 "Counter isn't working when reloaded with 255"
- `mmc3_v2_6_MMC3_alt` 失败诊断 "IRQ shouldn't be set when reloading to 0 due to counter naturally reaching 0 previously"
- C++ mapper4 (`src/boards/mmc3.cpp`) 已实现 IRQ 计数器，**问题在驱动时序**

可能的根因方向：
1. `mapper_meta_hblank_irq` 调用时序与 C++ `GameHBIRQHook` 在 PPU sl 边界差几条 cycle
2. PPU 地址 A12 上升沿检测——C++ mapper4 在每 cycle 比较 PPU 地址，Rust 仅在 segment 边界触发 hblank_irq
3. C++ mapper4 在 cycle 260+ 还做精细的 MMC3 IRQ scanline 计数

#### 2.1.2 实施步骤

1. **建立诊断 harness**（基于 Phase 6 的 `kagami_qa_shadow_run_runner`）：
   - 单跑 `mmc3_2_details.nes` 600 帧，按 PC 流对照 C++ mapper4 IRQ counter
   - 探针：`g_mmc3_irq_counter` 在每帧末快照，对比两端
2. **如确认是 hblank_irq 时序**：
   - 在 PPU 段边界多触发一次 `mapper_meta_hblank_irq`（Nesdev: 每 scanline 触发，而非每帧一次）
   - 参考 `src/ppu.cpp:GameHBIRQHook` 调用栈
3. **如确认是 A12 检测**：
   - 在 PPU 每条 cycle 比较 v.addr() bit 12（A12），rising edge 时通知 mapper
   - 新增 `vnesu11_mapper_a12_rising_edge_bridge` FFI 函数
4. **验证**：单 ROM PASS → 5 个 0x02 全 PASS

#### 2.1.3 期望产出

- 修 5 个 MMC3 0x02 失败
- T1: 143 → 148（**+5 PASS**，达 83.6%）

### 2.2 CPU 中断时序（高优先级）

#### 2.2.1 根因诊断

失败的 5 个测试：
- `cpu_int_2_nmi_brk`：NMI 在 BRK handler 期间触发
- `cpu_int_3_nmi_irq`：NMI 在 IRQ handler 期间触发
- `cpu_int_4_irq_dma`：IRQ 与 DMA 交互
- `cpu_int_5_branch_irq`：分支延迟 IRQ（CLI/SEI 的一周期延迟）
- `cpu_dummy_writes_ppu`：RMW 假写到 $2007 的时序

C++ 行为（`src/x6502.cpp`）：
- `_PI = _P` 在 RESET 服务后立即生效
- IRQ 服务前检查 `_PI & I_FLAG`（不是 `_P`）——给 CLI/SEI 一周期延迟
- BRK 与 IRQ 共享同一 vector 入口路径，但 BRK 设置 pushed P 的 B 标志
- NMI edge 在每条指令后采样（penultimate cycle）

#### 2.2.2 实施步骤

1. **对照 blargg test ROM 源**（在 GitHub `https://github.com/christopherpow/nes-test-roms` 或 blargg 原站）逐子测检查：
   - `cpu_int_2_nmi_brk`：是否 BRK 执行后立即采样 NMI？还是 RTI 后？
   - `cpu_int_5_branch_irq`：分支成功转移后 IRQ 是下一条指令前还是同一条指令后？
2. **如 Rust 端 moo_pi 时序有 bug**：
   - 增强 `cpu/ops_branch.rs` 在分支成功后立即同步 moo_pi = P
   - 增强 `cpu/interrupt.rs` 的 IRQ 服务路径同步 moo_pi
3. **如 dummy_writes_ppu 是 PPU $2007 写路径问题**：
   - 检查 RMW 中间假写是否触发 PPU $2007 副作用（应该 v 增量两次）
   - 与 C++ `B2007` 对照

#### 2.2.3 期望产出

- 修 2~3 个 CPU 测试
- T1: 148 → 150~151（达 84.7%~85.3%，**达到 85% 门槛**）

### 2.3 MMC3 A12 时钟 deep model（P2 溢出）

如 P1 完成后 T1 已 ≥ 150%，可跳过。如 < 150%：

- 在 PPU 加 `A12 rising edge` 检测：`self.ppu_v.bit(12)` 在每条 cycle 切换时比较
- 新增 FFI：`vnesu11_ppu_a12_rising_edge_bridge()`
- mapper4 侧接收通知，递减 IRQ 计数器

### 2.4 PPU dot 粒度时序（P2 溢出）

最复杂的 deep model 改动：
- PPU 必须从"segment-budget" 改为"cycle-by-cycle"（per-cycle）
- vbl_02..10 全部需要 dot 粒度：sl 240 cycle 340、sl 241 cycle 0、sl 20 cycle 0 等子扫描线相位
- 预计 3 周工作量

### 2.5 DMA 仲裁（P2 溢出）

- `sprdma_dmc_dma` 与 `sprdma_dmc_dma_512` 测试 DMC DMA 与 sprite DMA 同时请求时的优先级
- C++ `src/sound.cpp` 与 `src/ppu.cpp` 共享 cycle 仲裁——Rust 端需要协调 `DmaCore` 与 `DmcCore`

---

## 3. 验证策略

### 3.1 增量精度跟踪

每修一批跑：
```bash
cd tests
$env:VNESU11_RUST_PRIMARY="1"
..\build\tests\kagami_qa_blargg_runner.exe --manifest fixtures\blargg_manifest.json

# 期望趋势：143 → 148 → 150 → ... → 152 → ...（每修一个分类 +5 PASS）
```

### 3.2 shadow cpu_match 不退化

```bash
..\build\tests\kagami_qa_shadow_run_runner.exe fixtures\blargg\cpu\cpu_dummy_reads.nes --frames 60
# 期望 cpu_match ≥ 46/59（Phase 7a 基线）
```

### 3.3 回归基线不破

```bash
..\build\kagami-qa-runner.exe --baseline build/kagamiqa_baseline_v2.0.json --filter "tag=blargg"
# 期望：与 v2.0 frozen 一致 + 新 PASS
```

---

## 4. 关键技术决策

### 4.1 精度 oracle 维持 KagamiQA 5 层（ADR-011）

- T1 blargg 是**唯一**进度指标（不是 byte-level shadow match）
- 偏差登记 `deviations.yaml`（已 6 条 D-B）继续累积
- 任何"按 FCEUX C++ 行为" vs "按 chip 规范"的偏离必须登记

### 4.2 A12 时钟 deep model 改动需谨慎

如真的需要做 A12 rising edge 检测：
- 影响所有 IRQ-driven mapper（MMC3/MMC5/mapper 90 等）
- 改动 PPU 主循环时序（segment → cycle）
- 必须保留 Phase 7a shadow cpu_match ≥ 46/59
- 提交前必须跑全量 manifest + shadow 验证 0 退化

### 4.3 PPU dot 粒度改动代价评估

如必须改：
- 工作量：3-4 周
- 风险：segment-budget 模型到 cycle-by-cycle 是结构性变化
- 备选：**在 v2.1 推迟到 dot 粒度**（路径 B' = 仅 A12 + 中断时序，本阶段不碰 dot 粒度）
- 决策点：4.1 + 4.2 完成后 T1 是否 ≥ 85%？是 → 不需要 dot 粒度；否 → 必须做

### 4.4 不重写 mapper

MMC3 行为差异**只通过 FFI 适配解决**，不重写 mapper 为 Rust（250 mapper 全部在 C++）。改的是：
- `mapper_meta_hblank_irq` 调用频率/时序
- `vnesu11_ppu_a12_rising_edge_bridge` 新通知（仅 P2 需要时）

---

## 5. 风险

| 风险 | 严重度 | 缓解 |
|---|---|---|
| A12 deep model 改动破坏 shadow cpu_match | 🔴 高 | 提交前全量验证；任一 PASS 退 FAIL 即回滚 |
| 改 PPU 时序破坏其他 mapper | 🟠 中 | mapper_byte_diff 175 case 必须 0 退化 |
| 真实游戏 smoke ROM 不达，无法 UX 验证 | 🟡 低 | 不阻塞 Phase 7b 收口（仅 KagamiQA 5 层即可） |
| CPU 中断时序改动引新 bug | 🟠 中 | 单元测试覆盖 `cpu/branch` / `cpu/interrupt`；逐步修每子测 |
| T1 推到 85% 但发现新 FAIL | 🟡 低 | kagami_qa_full + shadow run 双验证 |

---

## 6. DoD

- [ ] T1 blargg (Rust-primary) ≥ **150/177 = 85%**
- [ ] kagami-qa-runner Oracle A 27 项 + Oracle B 21 项：新 FAIL = 0
- [ ] shadow run cpu_match ≥ 46/59（保持 Phase 7a 基线）
- [ ] mapper_byte_diff 175 case：新 FAIL = 0
- [ ] cargo test -p vnesu11 ≥ 203/0/0（+ 新增 deep model 测试）
- [ ] vn_perf_bench 帧时间 ≤ v1.17×1.05
- [ ] 文档更新：`docs/tech/KagamiQA.md` §3.3 + §3.3a 标注 85% 达成
- [ ] 新增 `deviations.yaml` D-B 条目（按 §4.2 原则）
- [ ] **Release tag：`v2.0.1`（hotfix）或 `v2.1`（minor）**

---

## 7. 关键文件交付

```
修改：
  src/rust/crates/vnesu11/src/cpu/ops_branch.rs      # 分支 moo_pi 同步
  src/rust/crates/vnesu11/src/cpu/ops_increment.rs   # RMW dummy write 行为
  src/rust/crates/vnesu11/src/cpu/interrupt.rs        # IRQ/BRK 服务时序
  src/rust/crates/vnesu11/src/bus.rs                 # $2007 RMW 时序（如需要）
  src/rust/crates/vnesu11/src/ppu/                   # A12 检测（如 P2 需要）
  src/rust/crates/vnesu11/src/dma/                   # DMA 仲裁（如 P2 需要）
  src/vnesu11_mapper_adapter.cpp                      # hblank_irq 调用频率
  src/kagami_bridge.cpp                              # PPU A12 通知桥（如 P2 需要）
  docs/tech/KagamiQA.md                              # 精度进度更新
  docs/wip_2.0_plan/deviations.yaml                  # 偏离登记
```

新交付：
```
  docs/wip_2.0_plan/phase_7b_accuracy.md             # 本文件
  src/rust/crates/vnesu11/tests/                     # 新增 deep model 回归测试
```

---

## 8. 工期与排期建议

| 周 | 任务 |
|---|---|
| 第 1 周 | MMC3 0x02 根因诊断 + hblank_irq 时序修复 + 5 PASS 验证 |
| 第 2 周 | CPU 中断时序逐子测对照 blargg 源 + moo_pi 同步 + RMW dummy 修复 |
| 第 3 周 | T1 ≥ 85% 验收（如 P1 全部完成可提前结束；如未达则进入 P2） |
| 第 4 周（可选） | A12 rising edge 检测（如 P1 未达 150%） |
| 第 5 周（可选） | dot 粒度 deep model（如 A12 仍不够） |

最短 3 周（仅 P1）。最长 5 周（含 P2 部分溢出）。

## 9. 退出条件

**Phase 7b 完成**：T1 ≥ 85%（150/177），shadow cpu_match ≥ 46/59，发布 v2.0.1。

**Phase 7b 不完成**：如 5 周后 T1 < 85%，发布 v2.0.0+epsilon（精度仍 80.8%），把 P2 部分纳入 v2.1 路线图。这是**接受现状**的选项——路径 B 的弹性正在于此。

下一步：Phase 7a 发布后启动 Phase 7b。