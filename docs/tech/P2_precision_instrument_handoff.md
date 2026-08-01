# P2 精度收敛剩余项 — 独立 Instrument 专项交接档案

> **用途**：FCEUX11 精度收敛（P2）中，R5（E-1 PPU VBL/NMI）与 R6（E-3 APU 帧计数器）剩余未闭合项的**完整诊断档案**。
> 本文档把散落在 `docs/history/surveys/e1_vbl/`、`docs/history/surveys/e6_apu/` 的 8 份调查文档的关键结论汇总为一份可直接接手攻关的清单。
>
> **性质**：仅记录已实证的根因与排除项，**不含构建计划**（攻关方案须在接手时基于本文档数据重新设计）。
> **适用对象**：后续独立 instrument-first 专项（需 instrument-first 前置硬约束，见 §0）。
> **关联**：`docs/history/reports/FCEUX11-1.16_最终验收报告.md` §十 R5/R6（原始处方，部分已被本文档证伪或修正）。

---

## 0. 接手前置条件（硬约束）

> 以下来自 `docs/history/reports/FCEUX11-1.16_最终验收报告.md` §十 R5/R6 的"🚨 实测校准"块与 `docs/继续任务.txt`：
>
> 1. **instrument-first 是前置硬约束**：任何代码改动前，必须先加 env-gated probe 采集真实时序，用数据证伪/证实假设。
> 2. **每步强制回归**：Oracle A ctest 34/34 + 相关 ROM 全量，任一红即回滚。
> 3. **savestate 兼容**：不得改 `FHCN`/`FCNT`/`IQFM` chunk 名/大小/序（sound.cpp:1303-1307），仅改运行期起始值；改运行期起始值会碎 `golden_savestate_test` / `savestate_regression_test`（MD5 固定参照），需重生 golden 索引（`tests/fixtures/golden/golden_index.json` 等）。
> 4. **不开新分支**：v1.16 系开发直接在 `wip_1.16` 分支进行（用户明确要求）。

---

## 1. 总览：P2 剩余未闭合项

| 项 | 失败 ROM | 诊断文档 | 根因状态 | 已排除假设 |
|---|---|---|---|---|
| **R5-E1** | `vbl_05_nmi_timing`（+ 05/06/07/08/10 组） | `e1_survey/vbl_step{1,2,3}_*.md` | **有据已知限制**（per-row 相位漂移） | NMI 延迟量（runppu(3)/(6)） |
| **R6-缺陷1** | `apu_single_4/5/6`、`apu_reset_4017_timing/written`（+ `apu_test` 剩余） | `e6_survey/r6_step{1,2,3}_*.md` | **暂停**（blargg wait_n 定时器语义未完全反汇编） | fcnt 起始值、IRQ 置位位置、fhcnt 重置 |
| **R6-缺陷2** | `apu_single_3_irq_flag`、`apu_test`（sub-test 3） | `e6_survey/r6_step2_fix_data_*.md` | ✅ **已修复**（`raw & 0x40` 条件清标志） | — |

> **净成果**：R6 缺陷 2 已修复（apu_single_3 PASS）。本文档聚焦 R5-E1 与 R6-缺陷1 两个未闭合项。

---

## 2. R5-E1：vbl_05_nmi_timing（已证伪单参数假设）

### 2.1 核心数据（probe 实测）

**vbl_05 测试机制**（反汇编 `vbl05_disasm_2026-07-30.md`）：
- 10 行重复执行相同 `$E32C` 子程序（A=0..9 循环调用）
- 每行 6 个 dead-store `LDX #0..#5`（各 2 CPU cycle = 6 PPU dots）作"等待时间"
- X 快照 = NMI dispatch 时已执行的 LDX 数
- blargg 期望 X=2（VBL 恰在正确相位可见）

**NMI 延迟量对 X 序列的影响**（三个版本实测）：

| runppu | X 序列 | 全=2? |
|---|---|---|
| 0（原始） | `[2,1,1,1,1,1,1,0,0,0]` | ❌ |
| 3（保留） | `[2,2,2,2,1,1,1,1,1,1]` | ❌（rows 4-9 欠调 1） |
| 6（回滚） | `[3,2,2,2,2,2,2,1,1,1]` | ❌（row 0 超调 3） |

### 2.2 根因（实证）

**单参数 NMI 延迟无法修复 vbl_05**：每次 +3 dots 序列整体上移 1，但 blargg 10 行 per-row 相位漂移（A 循环 `ADC/CMP/BNE`）导致永远有行超调（=3）或欠调（=1）。

**真根因方向**：VBL flag **可见相位** vs blargg 测试循环的 per-row 漂移——不是 NMI dispatch 延迟量。印证 E-1 §-1 "1-cycle 线性偏移不工作，每个 ROM 对应独立时序参数"。

### 2.3 关键代码位置

- `src/ppu_rendering.cpp:1580`：`if (VBlankON) { runppu(3); TriggerNMI(); }`（当前保留的 partial fix）
- VBL flag 置位 `PPU_status |= 0x80`（working config，`ppu_rendering.cpp` 同函数内）
- ppudead 路径 `ppu_rendering.cpp:1542`（**不得动**，P4-bridge SDK 修复依赖）

### 2.4 未来攻关建议方向（非构建计划）

1. **修 VBL flag 置位相位本身**（而非 NMI 延迟）：`PPU_status |= 0x80` 从 cycle 0 移到 cycle 1（真实硬件 sl 241 dot 1），观察 vbl_05 10 行是否同时收敛
2. **instrument-first 验证**：在 `$2002` 读时记录 sl/cycle（vbl_05 每行读点），对齐 blargg 期望相位
3. 每改一处强制回归 `vbl_01/04/09`（PASS 基线）+ Oracle A

---

## 3. R6-缺陷1：APU 帧计数器相位（3 假设全部证伪）

### 3.1 核心数据（probe 实测）

**上电后 IRQ 置位时机**（对比 PASS/FAIL ROM）：

| ROM | 上电后写 $4017? | 首个 FSU fcnt | IRQ 置位时点 | 结果 |
|---|---|---|---|---|
| apu_01 (PASS) | ✅ | 1 | 第 4 个 quarter (~29828 cyc) | ✅ |
| apu_single_4 (FAIL) | ❌ | 0 | **第 1 个 quarter (~7457 cyc)** | ❌ 太早 |

**blargg 测试机制**（反汇编 `apu_single_4_jitter.nes`）：
```
STA $4017=$40 (inhibit) → STA $4017=$00 (enable, 计时起点)
→ wait_n 定时器 → LDA $4015 → AND #$40
→ BEQ (IRQ 未置 = PASS) / JMP (IRQ 已置 = FAIL "too soon")
```

**R4015 probe 决定性时序**：
```
FSU fcnt=3  sirq=0x0      ← 第 3 个 FSU
FSU fcnt=0  fhcnt=0 sirq=0x0  ← 第 4 个 FSU（IRQ 内置）
R4015 fcnt=1  fhcnt=357960  sirq=0x40  ← blargg 读时 IRQ 已置 → FAIL
```

### 3.2 三次假设验证结果（全部证伪）

| # | 假设 | 改动 | apu_single_4 | golden savestate |
|---|---|---|---|---|
| 1 | fcnt 起始值（0→1） | `FCEUSND_Reset` | 仍 FAIL 0x02 | ❌ 碎 |
| 2 | IRQ 置位位置（`if(!fcnt)`→`if(fcnt==3)`） | `FrameSoundUpdate` | 仍 FAIL 0x02 | ✅ 未碎 |
| 3 | fhcnt 重置（去掉 `fhcnt=fhinc`） | `Write_IRQFM` | 仍 FAIL 0x02 | ❌ 碎 |

**fhcnt 漂移分析**（R6 Step3 probe）：HOOK_TRIG 的 `fhcnt_before` range [24,336], mean=82 → quarter 边界**稳定，无累积漂移**（否定 fhcnt 漂移假设）。

### 3.3 根因状态

**未闭合**。IRQ 在 enable 写后 3-4 quarters 置位（无论 fcnt 起始值、IRQ 位置、fhcnt 重置），blargg 期望**更晚**。需精确反汇编 blargg `wait_n` 定时器（E342/E358/E440 循环）确定 blargg 期望的 enable→IRQ 时延。

### 3.4 关键代码位置

- `FCEUSND_Reset`：`fcnt=0`（sound.cpp:1168），`fhcnt=fhinc`（sound.cpp:1200）
- `FrameSoundUpdate` IRQ 置位：`if(!fcnt && !(IRQFrameMode&0x3))`（sound.cpp:478）
- `Write_IRQFM`：`fcnt=1` + `fhcnt=fhinc`（sound.cpp:1044-1045）
- `FCEU_SoundCPUHook`：`fhcnt-=cycles*48`（sound.cpp:540）
- savestate chunks：`FHCN`/`FCNT`/`IQFM`（sound.cpp:1303-1307，不得改 chunk 结构）

### 3.5 未来攻关建议方向（非构建计划）

1. **完整反汇编 blargg `wait_n` 定时器**（E342/E358/E440 + E458/E46A 分支）确定 enable→IRQ 期望时延
2. **R4015 probe 已就位**：`StatusRead` 记录 (fcnt, fhcnt, sirq)，可直接复用
3. 重点核对 `FCEU_SoundCPUHook` 的 `cycles*48` 因子与硬件 quarter 周期（7457.5 cyc）的精确对齐

---

## 4. 已就位的 instrument 探针（复用清单）

以下 probe 均已实现并在 `wip_1.16` 上（env-gated，`FCEUX11_E3_TRACE=1` 时输出，未设时完全静默，ctest 34/34 PASS）：

| Probe | 位置 | 输出 | 用途 |
|---|---|---|---|
| `E3 FSU` | `FrameSoundUpdate` 入口 | fcnt/mode/fhcnt/sirq | 帧计数器状态机 |
| `E3 HOOK` | `FCEU_SoundCPUHook` 触发分支 | cycles/fhcnt_before/fhcnt_after | quarter 边界 |
| `E3 HOOK_SAMPLE` | `FCEU_SoundCPUHook` 每 4096 次 | call/fhcnt | 长期漂移 |
| `E3 HOOK_TRIG` | `FCEU_SoundCPUHook` 每 quarter | call/cycles/fhcnt_before/after/fhinc | 边界稳定 |
| `E3 R4015` | `StatusRead`（$4015 读） | fcnt/mode/fhcnt/sirq | blargg 读时刻 |
| `E3 W4017_IN/OUT` | `Write_IRQFM` | V/pre/post 状态 | $4017 写行为 |

> **注意**：E-1（PPU）的 probe（`E1 VBL_SET/NMI_ENTRY/P2002` 等）在 R5 收尾时已从代码移除（保留数据文档）。若未来重开 R5-E1 攻关，需重新插桩（数据模式见 `e1_survey/vbl_step{1,2}_instrument_data_*.md`）。

---

## 5. 数据文档索引（完整证据链）

### R5-E1（PPU VBL/NMI）

| 文档 | 内容 |
|---|---|
| `e1_survey/vbl_baseline_2026-07-30.txt` | 10 vbl ROM 基线 $6000 |
| `e1_survey/vbl05_disasm_2026-07-30.md` | vbl_05 反汇编 + 测试机制 |
| `e1_survey/vbl_step1_instrument_data_2026-08-01.md` | PPU 侧 probe 数据（PASS/FAIL ROM 一致） |
| `e1_survey/vbl_step2_instrument_data_2026-08-01.md` | CPU 侧 probe（NMI_ENTRY X 序列） |
| `e1_survey/vbl_step3_fix_data_2026-08-01.md` | runppu(3) fix + runppu(6) 实验（§9） |

### R6-E3（APU 帧计数器）

| 文档 | 内容 |
|---|---|
| `e6_survey/r6_step1_instrument_data_2026-08-01.md` | 缺陷 1/2 实证 |
| `e6_survey/r6_step2_fix_data_2026-08-01.md` | 缺陷 2 修复（已闭合） |
| `e6_survey/r6_step3_fix_data_2026-08-01.md` | 缺陷 1 三次假设失败 + R4015 时序 |

### 相关主线文档

- `docs/history/reports/FCEUX11-1.16_最终验收报告.md` §十 R5/R6（原始处方 + 🚨 校准块）
- `docs/history/plans/FCEUX11-1.16_Stage2-构建计划.md`（bucket C 分桶）

---

## 6. 状态摘要（写档时点）

- **wip_1.16 HEAD**：`f50573a`（已 push 至 `origin/wip_1.16`）
- **R6 缺陷 2**：✅ 已修复（apu_single_3 PASS）
- **R5 vbl_05**：⏸ 有据已知限制（runppu(3) partial fix 保留，per-row 相位漂移根因）
- **R6 缺陷 1**：⏸ 暂停（3 假设证伪，blargg wait_n 语义未完全反汇编）
- **Oracle A**：✅ 34/34 无回归
- **golden savestate**：✅ 未碎（所有失败实验均已回滚）

---

*交接档案完。后续独立 instrument 专项可直接基于本文档 §2/§3 的根因状态与 §4 的探针复用清单开始，无需重读全部 8 份 survey。*