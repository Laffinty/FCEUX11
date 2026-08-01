# E-3 APU R6 Step 2 — Defect 2 Fix Empirical Result (2026-08-01)

> **目的**：在 R6 Step 1 probe 实证缺陷 2（$4017 写无条件清 IRQ 标志）后，实施最小 fix 并做全 APU 回归验证。
>
> **分支**：`wip_1.16`（HEAD `5434e7a` + 本 fix）
> **Commit**：（见 git log）
> **作者**：独立接管（ZCode agent）

---

## 1. 改动

**文件**：`src/sound.cpp` `Write_IRQFM`（$4017 写 handler）

```diff
 DECLFW(Write_IRQFM)
 {
  // E-3 probe ... (保留)
  if (e3_trace_on()) { ... }  // 保留

+ // R6-2a: 在 (V&0xC0)>>6 归约前保存原始 $4017 值 —— bit6 是 IRQ inhibit 位
+ const uint8 raw = V;
  V=(V&0xC0)>>6;
  fcnt=0;
  if(V&0x2)
   FrameSoundUpdate();
  fcnt=1;
  fhcnt=fhinc;
- X6502_IRQEnd(FCEU_IQFCOUNT);
- SIRQStat&=~0x40;
+ if (raw & 0x40) {        // 仅当设置 inhibit 位时清 frame IRQ 标志
+  X6502_IRQEnd(FCEU_IQFCOUNT);
+  SIRQStat&=~0x40;
+ }
  IRQFrameMode=V;
  if (e3_trace_on()) { ... }  // 保留
 }
```

**改动范围**：+7 / -2 行（含 6 行注释）。**唯一实质改动**：无条件清 → `if (raw & 0x40)` 条件清。

**硬件依据（Nesdev）**：
- `$4017[7]` = 5-step 模式（归约后 `IRQFrameMode & 0x2`）
- `$4017[6]` = IRQ inhibit（归约后 `IRQFrameMode & 0x1`）
- 写 $4017 **不带 inhibit 位**（$00/$80）**不应**清 frame IRQ 标志
- 写 **设置 inhibit 位**（$40/$C0）才清
- `$4015` 读（StatusRead）无条件清 bit6 是标准硬件行为，**未改**

---

## 2. Oracle A 回归

```
$ do_build.ps1 -Config Release -BuildDir build-c1 (13 min)
100% tests passed, 0 tests failed out of 34
```

✅ **34/34 PASS**（构建脚本自带 ctest）

---

## 3. 全 APU ROM 回归（12 ROM × 600 帧 + 2 vbl 抽检）

### 3.1 核心结果

| ROM | 修复前 $6000 | 修复后 $6000 | 变化 | 说明 |
|---|---|---|---|---|
| **`apu_single_3_irq_flag`** | FAIL 0x06 (#6) | **PASS 0x00** | 🟢 **修复** | 缺陷 2 直接命中 |
| **`apu_test`** | FAIL 0x01 (停在 irq_flag #6) | FAIL 0x01 (**推进到 4-jitter #2**) | 🟡 **推进** | 缺陷 2 已过，卡在缺陷 1 |
| `apu_01_len_ctr` | PASS | PASS | ✅ 无回归 | 对照 |
| `apu_02_len_table` | PASS | PASS | ✅ 无回归 | 对照 |
| `apu_03_irq_flag` | PASS | PASS | ✅ 无回归 | 对照 |
| `apu_07_irq_flag_timing` | PASS | PASS | ✅ 无回归 | 对照 |
| `apu_08_irq_timing` | PASS | PASS | ✅ 无回归 | 对照 |
| `apu_reset_4017_timing` | FAIL 0x02 | FAIL 0x02 | ⚪ 缺陷 1 | 未修（预期） |
| `apu_reset_4017_written` | FAIL 0x02 | FAIL 0x02 | ⚪ 缺陷 1 | 未修（预期） |
| `apu_single_4_jitter` | FAIL 0x02 | FAIL 0x02 | ⚪ 缺陷 1 | 未修（预期） |
| `apu_single_5_len_timing` | FAIL 0x02 | FAIL 0x02 | ⚪ 缺陷 1 | 未修（预期） |
| `apu_single_6_irq_timing` | FAIL 0x02 | FAIL 0x02 | ⚪ 缺陷 1 | 未修（预期） |
| `vbl_01_basics`（PPU） | PASS | PASS | ✅ R5 无回归 | 抽检 |
| `vbl_05_nmi_timing`（PPU） | FAIL 0x01 | FAIL 0x01 | ⚪ R5 状态保持 | 抽检 |

**7 个 FAIL ROM 中**：
- 🟢 **1 个修复**（apu_single_3）
- 🟡 **1 个推进**（apu_test 越过缺陷 2 的 irq_flag #6，卡到缺陷 1 的 4-jitter #2）
- ⚪ **5 个不变**（全是缺陷 1 根因，本次未修）

**结论**：缺陷 2 的 fix **有效且无回归**。剩余 5 个 FAIL 均为缺陷 1（帧计数器相位），须 R6-2b 处理。

### 3.2 Probe 行为验证（$4017 写不再误清标志）

`apu_single_3_irq_flag` fix 后 trace：

```
W4017_IN  V=0x00  pre_sirq=0x40 → post_sirq=0x40  ✅ 保留（修复，blargg #6 断言）
W4017_IN  V=0x40  pre_sirq=0x40 → post_sirq=0x00  ✅ 清除（inhibit 位，正确）
W4017_IN  V=0x80  pre_sirq=0x40 → post_sirq=0x40  ✅ 保留（修复）
W4017_IN  V=0x40  pre_sirq=0x40 → post_sirq=0x00  ✅ 清除（inhibit 位，正确）
```

**与 Nesdev 语义完全一致**。

---

## 4. 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | `git diff --stat` 仅 `src/sound.cpp` +7/-2 | ✅ |
| 2 | ctest 34/34 PASS | ✅ |
| 3 | `apu_single_3_irq_flag` 0x06 → 0x00 | ✅ 核心 |
| 4 | `apu_test` 推进到 4-jitter #2（越过缺陷 2） | ✅ |
| 5 | 5 PASS 对照不回归 | ✅ |
| 6 | probe 保留，$4017 前后状态可验证 | ✅ |
| 7 | `$4015` StatusRead 未改 | ✅ |
| 8 | 不动缺陷 1 相关代码（FCEUSND_Reset / FrameSoundUpdate） | ✅ |
| 9 | 不开新分支（wip_1.16） | ✅ |
| 10 | vbl ROM 抽检无回归（R5 fix 完好） | ✅ |

---

## 5. 下一步

**缺陷 2 已闭合。剩余 5/7 FAIL 全为缺陷 1**（帧计数器相位太早）。

### R6-2b（缺陷 1，待用户批准）

- 目标：`FCEUSND_Reset` 的 `fcnt=0` 初值 → 让首个 IRQ 在第 4 个 quarter-frame（~29828 cyc）而非第 1 个（~7457 cyc）
- 影响：`apu_reset_4017_timing/written` + `apu_single_4/5/6` + `apu_test` 剩余部分（5/7 FAIL）
- 风险：影响所有 APU ROM（含 PASS 对照），须全回归

---

## 6. 配套文件

- `src/sound.cpp`：缺陷 2 fix（+7/-2）
- `build-c1/r6_fix_traces/*.err` / `*.out`：14 ROM 回归数据（不进 git）

---

*数据文档完。等用户批准 R6-2b。*