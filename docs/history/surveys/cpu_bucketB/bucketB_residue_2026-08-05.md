# FCEUX11 v1.16 — CPU 桶 B.1+B.2 定案（深模型族已知限制）

> **编制日期**: 2026-08-05
> **承接文档**: `docs/history/surveys/cpu_bucketB/stepB_investigation_2026-08-04.md`
> **承接方案**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 桶 B
> **状态**: ✅ **桶 B.1+B.2 9/9 记入有据已知限制**（深模型族）

---

## 0. 摘要

桶 B.1+B.2 CPU 真实精度 9 项经评估后统一记入**有据已知限制**（深模型族）：

| 错误码 | ROM | 状态 |
|---|---|---|
| 0x01 | `cpu_int_2_nmi_brk` | 🚧 已知限制（CPU 中断时序 — NMI/BRK） |
| 0x01 | `cpu_int_3_nmi_irq` | 🚧 已知限制（CPU 中断时序 — NMI/IRQ） |
| 0x01 | `cpu_int_4_irq_dma` | 🚧 已知限制（CPU 中断时序 — IRQ/DMA） |
| 0x01 | `cpu_int_5_branch_irq` | 🚧 已知限制（CPU 中断时序 — 分支/IRQ） |
| 0x02 | `cpu_reset_regs` | 🚧 已知限制（CPU 复位寄存器时序） |
| 0x01 | `instr_misc` | 🚧 已知限制（指令时序杂项） |
| 0x03 | `instr_misc_03_dummy` | 🚧 已知限制（指令时序杂项 — dummy read） |
| 0x01 | `instr_timing` | 🚧 已知限制（指令时序 — 与 vbl_02 残量同族） |
| 0x03 | `instr_timing_v2_1` | 🚧 已知限制（指令时序 v2.1 — 与 vbl_02 残量同族） |

**桶 B 总评**（合并桶 B.3+B.4 + 桶 B.1+B.2）：
- ✅ PASS: 1 项（`cpu_dummy_writes_ppu` 0x09，`863e9d7`）
- 🚧 已知限制: 11 项（桶 B.3+B.4 2 项 + 桶 B.1+B.2 9 项）
- 收敛率 8%（1/12），剩余 92% 全部为深模型族

---

## 1. 根因族分类（与 Phase 1 vbl 深模型同根）

### 1.1 子族 A：CPU 中断时序族（4 项）
- `cpu_int_2_nmi_brk` 0x01
- `cpu_int_3_nmi_irq` 0x01
- `cpu_int_4_irq_dma` 0x01
- `cpu_int_5_branch_irq` 0x01

**共享根因**：
- 与 Phase 1 `vbl_07_nmi_on_timing` 0x01 / `vbl_08_nmi_off_timing` 0x01 同族
- 方案 §5 评估："CPU 中断/复位/分支时序族，与 Phase 1 vbl_02/06/07/08 同族（深模型族）"
- 共享"CPU 指令边界 NMI 采样"族：NMI/IRQ 边沿检测在指令边界，存在亚指令采样相位漂移

**修复路径**：需要 per-cycle NMI/IRQ 边沿检测（如 Mesen2 `EndCpuCycle`），改动面大（CPU 类重构）。

### 1.2 子族 B：CPU 复位时序族（1 项）
- `cpu_reset_regs` 0x02

**根因**：
- CPU 复位序列（SEI/CLD 等初始化指令的精确 cycle 计数）
- 复位后寄存器初值与时序的精确匹配
- 涉及 power-on 与 soft-reset 两套独立时序

**修复路径**：需要 CPU 复位序列 cycle-accurate 重构。

### 1.3 子族 C：指令时序杂项族（4 项）
- `instr_misc` 0x01
- `instr_misc_03_dummy` 0x03
- `instr_timing` 0x01
- `instr_timing_v2_1` 0x03

**共享根因**：
- 与 Phase 1 `vbl_02_set_time` 0x01 残量探针同族（`vbl_step1_2_closure_assessment_2026-08-03.md`）
- 1 dot/行漂移来自 `sync_vbl_delay(A=行号)` 亚指令粒度延迟
- 闭合需亚指令级 CPU↔PPU 联合仿真（CPU 读时序 + PPU 帧同步），收敛无保证
- Phase 1 评估："残量被帧边界重同步摧毁——不是未建模，是硬件本身允许残留物"

**修复路径**：需要每指令 cycle-精确化（包含 dummy read cycle、RMW write-back 时序、分支边界 cycle 计数）。

---

## 2. 与桶 B.3+B.4 的关系

### 2.1 桶 B.3+B.4 已处理（`57d3e88` + `863e9d7`）
- `cpu_dummy_writes_oam` 0x06：桶 B 探针实测 + PPU 隐式 OAM 改写（深模型族）
- `cpu_exec_space_ppuio` 0x05：CPU 指令预取 PPU I/O 镜像语义（深模型族）
- `cpu_dummy_writes_ppu` 0x09：**PASS**（`863e9d7` 桶 C 顺带修复，palette 索引同族）

### 2.2 桶 B.1+B.2 与 B.3+B.4 同族
- 全部 11 项已知限制均属"CPU 时序族"或"PPU 隐式改写族"
- 与桶 B.3+B.4 一致：本族修复需 CPU per-cycle 状态机或 PPU per-cycle 渲染精确化

---

## 3. 探针保留

桶 B 探针（`FCEUX11_BUCKETB_PROBE` if any, env-gated，零侵入）保留在 `stepB_investigation_2026-08-04.md` 描述的状态。本 commit 零代码改动，仅文档化已知限制决策。

---

## 4. Phase 3 Step 3.2 全桶收口

| 桶 | 总项 | PASS | 已知限制 | 状态 |
|---|---|---|---|---|
| 桶 A（MMC3） | 12 | 0 | 12 | ✅ `f4a072a` |
| 桶 B.3+B.4（CPU 残项） | 3 | 1 | 2 | ✅ `57d3e88` + `863e9d7` |
| 桶 B.1+B.2（CPU 中断/时序） | 9 | 0 | 9 | ✅ `b(known-limit)`（本 commit） |
| 桶 C 余项（PPU 真实精度残项） | 2 | 0 | 2 | ✅ `c(known-limit)`（上一 commit） |
| 桶 D（sprdma） | 2 | 0 | 2 | ✅ `4a7f7e2` |
| 桶 C.1（vbl） | 5 | 0 | 5 | ✅ Phase 1 已记录 |
| **Phase 3 Step 3.2 合计** | **33** | **1** | **32** | |

> 注：表格统计 1 PASS 来自桶 B.3+B.4 的 `cpu_dummy_writes_ppu`（`863e9d7`）；桶 C 的 `ppu_read_buffer` + `ppu_open_bus` 已计入 Phase 3 总账（Oracle B 144 PASS）。

---

## 5. 决策

按方案 §5 "宁可错过不要硬塞"原则，桶 B.1+B.2 9 项**统一记入有据已知限制**：

- 零代码改动
- 与桶 A（MMC3 12 `f4a072a`）+ 桶 D（sprdma 2 `4a7f7e2`）+ 桶 C 余项（oam_stress + ppu_vbl_nmi `116a602`）模式一致
- 修复路径：深模型族突破（per-cycle CPU/PPU 联合仿真改造）后统一处理

**Phase 3 Step 3.2 桶工作完成**——所有桶已分类（PASS / 已知限制），可直接进入 Step 3.3 全量回归与验收复检。

---

*调查完。桶 B.1+B.2 与桶 A/MMC3 + 桶 D/sprdma + 桶 C 余项 + 桶 B.3+B.4 同属深模型族，9 项已统一记入已知限制。*