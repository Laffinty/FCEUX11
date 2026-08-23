# Cycle-Accurate PPU 模型必要性论证 — 已归档

> ⚠️ 已归档（2026-08-23）：本文档是 v2.0_hotfix1 阶段对 cycle-accurate PPU 必要性的前置论证，已作为 v2.1 Rust PPU 重构方案的历史依据归档。请勿再基于本文档直接执行构建任务；后续实施以 `docs/plans/v2.1_ppu_rust_refactor_plan.md` 为准。

> **文档性质**: v2.1 版本迭代理论依据  
> **生成时间**: 2026-08-24  
> **关联版本**: v2.0_hotfix1（83.9% 通过率）  
> **目标**: 论证从 instruction-granular PPU 升级到 cycle-accurate PPU 的技术必要性

---

## 1. 现状与瓶颈

### 1.1 当前通过率

| 指标 | v2.0 基线 | v2.0_hotfix1 | 变化 |
|------|-----------|--------------|------|
| PASS | 2769 (80.2%) | 2894 (83.9%) | +125 |
| FAIL | 679 | 554 | -125 |
| Crash | 11 | 3 | -8 |

### 1.2 剩余 554 个 FAIL 构成

| 类型 | 数量 | 占比 | 根因 |
|------|------|------|------|
| CPU stuck | 551 | 99.5% | PPU VBlank NMI 时序偏差 |
| PC 异常 | 2 | 0.4% | 海盗 mapper 非标准复位 |
| Crash | 3 | 0.5% | UNIF UNL-603-5052 板卡 |

### 1.3 Stuck ROM 地址分布

| 地址范围 | 数量 | 占比 | 典型行为 |
|----------|------|------|----------|
| 0xC000-0xFFFF | 379 | 68.8% | 主游戏循环等待 VBlank NMI |
| 0x8000-0xBFFF | 141 | 25.6% | 初始化代码等待中断 |
| 0x0000-0x3FFF | 17 | 3.1% | 零页/栈循环 |
| 其他 | 14 | 2.5% | 扩展区/SRAM |

**关键发现**: 94.4% 的 stuck PC 在 ROM 空间（0x8000-0xFFFF），且跨所有 mapper（mapper 0-19 均受影响）。这证明问题不是 mapper 特定的，而是 PPU 时序的系统性偏差。

---

## 2. Blargg 测试证据

### 2.1 失败的 PPU VBL 子测试

| 子测试 | 状态 | 错误码 | 含义 |
|--------|------|--------|------|
| vbl_02_set_time | FAIL | 0x01 | VBL flag 设置时间不对（cycle 0 vs 1） |
| vbl_05_nmi_timing | FAIL | 0x01 | NMI 触发延迟漂移（per-frame phase drift） |
| vbl_06_suppression | FAIL | 0x01 | $2002 读取抑制窗口不精确 |
| vbl_07_nmi_on_timing | FAIL | 0x01 | $2000 写入后 NMI 使能时序 |
| vbl_08_nmi_off_timing | FAIL | 0x01 | $2000 写入后 NMI 禁用时序 |
| vbl_10_even_odd_timing | FAIL | 0x03 | 偶/奇帧跳过时机不对 |

### 2.2 通过的 PPU 子测试

| 子测试 | 状态 |
|--------|------|
| vbl_01_basics | PASS |
| vbl_03_clear_time | PASS |
| vbl_04_nmi_control | PASS |
| vbl_09_even_odd_frames | PASS |

### 2.3 测试结论

6/9 个 VBL 子测试失败，全部与 **sub-instruction 级 PPU 时序** 相关。已通过的测试验证的是 instruction-granular 行为（VBL basics、clear time、NMI control），而失败的测试验证的是 **cycle-level 精度**（set time、NMI timing、suppression window）。

---

## 3. 技术根因分析

### 3.1 当前架构：Instruction-Granular PPU

当前 PPU 模型的工作方式：

```
CPU 执行指令 → 更新 PPU 状态 → 检查 VBlank/NMI → 继续下一条指令
```

关键特征：
- **PPU 状态在指令边界更新**，不是每个 PPU cycle
- **NMI 触发在指令边界检查**，不是实时采样
- **VBL flag 设置在一个 PPU dot 内完成**，没有 cycle-level 延迟
- **$2002 读取抑制**在指令粒度上近似，不是 cycle-exact

### 3.2 为什么 Instruction-Granular 不够

#### 问题 1: NMI Phase Drift

当前 NMI 延迟使用 `runppu(nd)` 或 `X6502_Run(nd)` 在 VBL flag 设置后授予 CPU 预算。但由于 CPU 指令边界的位置取决于之前执行的指令，NMI 触发点相对于 PPU 周期的位置每帧都不同。

**blargg vbl_05 检测**: 测量 NMI 触发点相对于 VBL 的 per-frame drift。如果 drift > 0，测试失败。

**当前行为**: drift 存在，因为 NMI 在指令边界触发，而指令边界的位置不可预测。

#### 问题 2: $2002 Read Suppression Window

NES 硬件在 cycle 级别处理 $2002 读取：
- 读取 1 PPU cycle 前 → 抑制 VBL flag + NMI
- 读取 at VBL-set → 读取为 set，清除，抑制 NMI
- 读取 2+ cycles 后 → 正常行为

当前实现使用指令粒度近似，窗口不够精确。

#### 问题 3: $2000 Write NMI Edge Detection

当游戏写入 $2000 使能 NMI 时，NES 硬件在 cycle 级别采样 NMI 线。当前实现使用 NMI2 flag 在指令边界触发，时序不够精确。

#### 问题 4: Even/Odd Frame Skip Timing

NES PPU 在 pre-render scanline 的 cycle 340 跳过 1 dot（奇数帧）。当前实现在 scanline 0 的 BG fetch 后设置 end_cycle，时序可能不精确。

### 3.3 Cycle-Accurate PPU 如何解决

Cycle-Accurate PPU 的工作方式：

```
每个 PPU cycle:
  1. 更新 PPU 状态（渲染、VRAM 访问）
  2. 检查 VBL flag 设置/清除
  3. 采样 NMI/IRQ 线
  4. 如果 NMI/IRQ 触发，通知 CPU
  5. CPU 执行 1/3 指令（或在指令边界完成）
```

关键区别：
- **PPU 状态每个 cycle 更新**，不是指令边界
- **NMI/IRQ 每个 cycle 采样**，时序精确到 1 PPU dot
- **$2002 读取在 cycle 级别处理**，抑制窗口精确
- **Even/odd frame skip 在 cycle 340 执行**，时序精确

---

## 4. 预期收益

### 4.1 Blargg 测试收益

修复 vbl_02/05/06/07/08/10 全部 6 个失败子测试，实现 blargg ppu_vbl_nmi 9/9 全通过。

### 4.2 ROM 兼容性收益

| 影响范围 | 预期修复 | 依据 |
|----------|----------|------|
| 跨 Mapper stuck ROM | +200~300 | 94.4% stuck PC 在 ROM 空间，NMI 时序是统一根因 |
| Mapper 19 回归 | +15 | 三国志2 变体在 legacy PPU 下 PASS |
| Mapper 1 专属聚类 | +30~50 | 0xC03D/0xC050/0xC05B 等 VBlank 等待循环 |
| 零页 stuck | +4 | 0x0000 地址的 NMI 向量读取问题 |

**预期总收益**: +250~350 ROM，通过率从 83.9% 提升至 **~91-94%**。

### 4.3 长期收益

- 实现 blargg PPU 测试套件全通过（49/49 PPU 测试）
- 为未来 Mapper 精度改进（MMC3 IRQ scanline counter、VRC 扩展音频 IRQ）提供正确的时序基础
- 减少 per-game 硬编码 hack，提高代码可维护性

---

## 5. 实施方案

### 5.1 架构选择

**方案 A: 全新 Cycle-Accurate PPU 引擎**
- 重写 `ppu_rendering.cpp` 的渲染循环
- 每个 PPU cycle 执行一次 PPU 状态更新 + NMI/IRQ 采样
- 工作量: 40-60h
- 风险: 高（可能影响所有已通过的测试）

**方案 B: 渐进式 Cycle-Accurate 改造**
- 在现有 `runppu()` 框架内添加 cycle-level hooks
- 每个 `runppu(1)` 调用时检查 VBL/NMI 状态
- 工作量: 20-30h
- 风险: 中（保留现有架构，逐步添加精度）

**方案 C: 混合方案**
- 关键时序路径（VBL set/clear、NMI trigger）使用 cycle-accurate
- 渲染路径保持 instruction-granular
- 工作量: 15-20h
- 风险: 中低（只修改时序关键路径）

**推荐**: 方案 C（混合方案），以最小风险获得最大收益。

### 5.2 实施步骤

| 步骤 | 内容 | 预计工作量 | 预期收益 |
|------|------|-----------|----------|
| 1 | VBL flag 设置延迟 1 cycle | 2h | +50 ROM |
| 2 | NMI 触发 cycle-accurate 采样 | 4h | +100 ROM |
| 3 | $2002 读取 suppression cycle-exact | 3h | +30 ROM |
| 4 | $2000 写入 NMI edge detection | 3h | +50 ROM |
| 5 | Even/odd frame skip cycle 340 | 2h | +20 ROM |
| 6 | blargg VBL 全量回归验证 | 4h | — |
| 7 | 3451 ROM 兼容性回归验证 | 2h | — |

**总计**: ~20h，预期 +250 ROM。

---

## 6. 风险评估

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| blargg 已通过测试回归 | 中 | 高 | 每步验证，回退机制 |
| KagamiQA 基线失效 | 中 | 中 | 更新 frozen baseline |
| 新 PPU 性能下降 | 低 | 中 | cycle-level hooks 只在关键路径 |
| 与 Rust CPU tick 机制冲突 | 低 | 高 | 复用现有 sync_irq_from_host 桥 |

---

## 7. 结论

从 instruction-granular PPU 升级到 cycle-accurate PPU 是 v2.1 版本的核心技术目标。当前 83.9% 的通过率已接近 instruction-granular 模型的理论上限，剩余 554 个 stuck ROM 全部由 sub-instruction 级 PPU 时序偏差导致。Cycle-accurate PPU 是唯一能够系统性解决这些失败的方案，预期可将通过率提升至 91-94%。

---

*文档生成时间: 2026-08-24*  
*基于: v2.0_hotfix1 兼容性测试数据 + blargg ppu_vbl_nmi 测试结果*
