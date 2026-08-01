# E-3 APU R6 Step 1 — Instrument-First Data (2026-08-01)

> **目的**：在动任何 APU 时序代码前，用 env-gated probe 采集帧计数器状态机（fcnt / IRQFrameMode / fhcnt / SIRQStat）与 $4017 写的真实时序，验证 §十 R6 的两个根因假设。
>
> **分支**：`wip_1.16`（HEAD `0268478`，含 R5 `runppu(3)` fix）
> **Probe commit**：（见 git log）
> **作者**：独立接管（ZCode agent）
> **方法**：env-gated `FCEUX11_E3_TRACE=1`，stderr 输出，零侵入（不改任何 APU 时序/状态）
>
> **配套文档**：`docs/history/reports/FCEUX11-1.16_最终验收报告.md` §十 R6（处方来源）、`docs/history/surveys/e1_vbl/vbl_baseline_2026-07-30.txt`

---

## 1. 方法

### 1.1 Probe 插入点（仅 `src/sound.cpp`，零逻辑改动）

| # | 文件:行 | 触发事件 | 输出格式 |
|---|---|---|---|
| 1 | `sound.cpp:~470` | `FrameSoundUpdate()` 入口 | `E3 FSU fcnt=N mode=0xM fhcnt=X sirq=0xS` |
| 2 | `sound.cpp:~539` | `FCEU_SoundCPUHook()` quarter-frame 触发 | `E3 HOOK cycles=N fhcnt_before=X fhcnt_after=Y` |
| 3 | `sound.cpp:~1022` | `Write_IRQFM($4017)` 入口 | `E3 W4017_IN V=0xV pre_mode=0xM pre_fcnt=N pre_sirq=0xS` |
| 4 | `sound.cpp:~1036` | `Write_IRQFM($4017)` 出口 | `E3 W4017_OUT post_mode=0xM post_fcnt=N post_sirq=0xS` |

> 关键修正：初版用 `namespace fceu11::e3 { static bool trace_on() ... }` 编译失败（`C2653: 'fceu11': 不是类或命名空间名`，MSVC 无法在 sound.cpp 中解析嵌套命名空间）。改为文件级 `static bool e3_trace_on()`（无命名空间），编译通过。

### 1.2 构建与回归

- `scripts\do_build.ps1 -Config Release -BuildDir build-c1` —— 14 分钟全量重建
- `ctest -LE perf` —— **33/33 PASS**（probe 静默时无影响，34 注册含 1 perf 排除）

### 1.3 数据采集

12 ROM × 600 帧（7 个 §十 R6 FAIL + 5 个 PASS 对照）：
`build-c1/r6_traces/*.err`（stderr probe 数据） + `*.out`（BLARGG_RESULT）

---

## 2. 实测数据与结果

### 2.1 7 个 FAIL ROM 的 $6000 结果

| ROM | $6000 | 诊断 | 对应缺陷 |
|---|---|---|---|
| `apu_reset_4017_timing.nes` | 0x02 | "Delay after effective $4017 write..." | 缺陷 1 |
| `apu_reset_4017_written.nes` | 0x02 | "At power, $4017 should be written with $00" | 缺陷 1 |
| `apu_single_3_irq_flag.nes` | 0x06 | "Writing $00 or $80 to $4017 shouldn't affect flag" | **缺陷 2** |
| `apu_single_4_jitter.nes` | 0x02 | "Frame irq is set too soon" | 缺陷 1 |
| `apu_single_5_len_timing.nes` | 0x02 | "First length of mode 0 is too soon" | 缺陷 1 |
| `apu_single_6_irq_timing.nes` | 0x02 | "Flag first set too soon" | 缺陷 1 |
| `apu_test.nes` | 0x01 | "Writing $00 or $80 to $4017 sh..." (停在 sub-test 3) | 缺陷 2 |

### 2.2 5 个 PASS 对照 ROM

| ROM | $6000 | 状态 |
|---|---|---|
| `apu_01_len_ctr.nes` | 0x00 | PASS |
| `apu_02_len_table.nes` | 0x00 | PASS |
| `apu_03_irq_flag.nes` | 0x00 | PASS |
| `apu_07_irq_flag_timing.nes` | 0x00 | PASS |
| `apu_08_irq_timing.nes` | 0x00 | PASS |

> 注：`apu_reset_len_ctrs` / `apu_reset_works_imm` 报 0x81 "Press RESET"（需 `--reset-after` 处理，非本次缺陷 ROM）。

---

## 3. 缺陷 1 实证（帧计数器相位太早 → "IRQ/length too soon"）

### 3.1 上电后首个 quarter-frame 就置 IRQ

`apu_single_4_jitter.nes` trace（前 18 行）：

```
E3 FSU fcnt=0 mode=0x0 fhcnt=-24 sirq=0x0    ← 上电后第 1 个 FSU：fcnt=0, mode=0x0 (IRQ enabled)
E3 FSU fcnt=1 mode=0x0 fhcnt=-96 sirq=0x40   ← 第 2 个 FSU：sirq=0x40 已置位！
E3 FSU fcnt=2 mode=0x0 fhcnt=-24 sirq=0x40
E3 FSU fcnt=3 mode=0x0 fhcnt=-96 sirq=0x40
E3 FSU fcnt=0 mode=0x0 fhcnt=-72 sirq=0x40   ← 第 4 个 FSU：仍置位
```

**解读**：
- `FCEUSND_Reset` 设 `fcnt=0` + `IRQFrameMode=0x0`（sound.cpp:1105/1102）
- 第一个 `FrameSoundUpdate`（fcnt=0）命中 `if(!fcnt && !(IRQFrameMode&0x3))`（sound.cpp:448）
- → 上电后 **~7457 cyc（首个 quarter-frame）就置 IRQ 标志**，而硬件应在 **~29828 cyc（第 4 个 quarter-frame）**
- probe 显示第二个 FSU 时 `sirq=0x40` 已可见（第一个 FSU 内部置位，probe 在入口看不到置位本身，但在后续 FSU 可见）

### 3.2 与 §十 R6 处方完全吻合

> §十 R6 缺陷 1："`FCEUSND_Reset` 设 `fcnt=0` + `FrameSoundUpdate` 在 `fcnt==0` 就置 IRQ → 上电后第一个 quarter-frame 就置 IRQ、clock length，而硬件在 ~29828 / ~22371"

**probe 实测证实该处方**：上电后 fcnt=0 的 FSU 即置位 IRQ。

### 3.3 影响

- `apu_single_4_jitter` / `apu_single_6_irq_timing`：IRQ "too soon"
- `apu_single_5_len_timing`：length clock "too soon"（FrameSoundStuff(fcnt) 在 fcnt=0 就 clock length）
- `apu_reset_4017_timing` / `apu_reset_4017_written`：Reset 后相位错 → $4017 写时机断言失败

---

## 4. 缺陷 2 实证（$4017 写无条件清 IRQ 标志）

### 4.1 `apu_single_3_irq_flag.nes` 核心证据

```
E3 W4017_IN  V=0x0  pre_mode=0x0  pre_fcnt=1  pre_sirq=0x40   ← 写 $4017=$00 前，IRQ 标志已置位
E3 W4017_OUT post_mode=0x0 post_fcnt=1 post_sirq=0x0           ← 写后 IRQ 标志被清掉！
```

**解读**：
- `Write_IRQFM`（sound.cpp:991-992）**无条件**执行 `X6502_IRQEnd(FCEU_IQFCOUNT); SIRQStat&=~0x40;`
- 即使 V=$00（4-step 模式，IRQ 保持使能，按硬件不应清标志）
- blargg 要求"Writing $00 or $80 to $4017 shouldn't affect flag"——**当前行为直接违反**

### 4.2 多次复现

`apu_single_3_irq_flag` trace 中 L37/L88 两处均见 `pre_sirq=0x40 → post_sirq=0x0`（V=$00）。
`apu_reset_4017_written` L61 亦见同型（V=$00 清掉 0x40）。

### 4.3 与 §十 R6 处方吻合

> §十 R6 缺陷 2："`Write_IRQFM` `:991-992` 对每次 $4017 写都 `X6502_IRQEnd(...); SIRQStat&=~0x40;`，包括写 $00。blargg `3-irq_flag #6` 要求写 $00/$80 不扰动标志——清标志须以 5-step(bit6)/inhibit(bit7) 位为条件"

**probe 实测证实该处方**。

---

## 5. 关键结论

### 5.1 两个缺陷假设均被 probe 数据证实

| 缺陷 | §十 R6 假设 | probe 实测 | 判定 |
|---|---|---|---|
| 缺陷 1 | 上电后 fcnt=0 首个 FSU 置 IRQ（早 ~22371 cyc） | 上电后第 2 个 FSU 已见 `sirq=0x40` | ✅ 证实 |
| 缺陷 2 | $4017 写无条件清 IRQ 标志 | `W4017_IN pre_sirq=0x40 → W4017_OUT post_sirq=0x0`（V=$00） | ✅ 证实 |

### 5.2 处方可信度评估

**§十 R6 Priority 1 + Priority 2 处方的根因分析均获数据支撑**。但具体改法（fcnt 初始值、IRQ 清标志条件）**仍须独立设计**，不直接照抄处方——因为：
- 处方 Priority 1 "方案 A/B" 未给精确数值（fcnt 应初始为几？）
- 缺陷 1 的 fix 可能影响 PASS ROM（apu_01/02/03/07/08），须回归验证

### 5.3 下一步（须用户决策）

| 选项 | 内容 | 风险 |
|---|---|---|
| **R6-2a** | 直接修缺陷 2（改 `Write_IRQFM`：仅在 V 的 bit6/7 设置时清 IRQ） | 低（范围小，独立于相位） |
| **R6-2b** | 先修缺陷 1（改 `FCEUSND_Reset` 的 fcnt 初始值，如 fcnt=3 → 首个 IRQ 在第 4 个 quarter-frame） | 中（影响所有 APU ROM，须全回归） |
| **R6-2c** | 两个一起修 | 高（相互作用难隔离） |

**建议先 R6-2a（缺陷 2）**：改动最小、blargg 有明确断言（$00/$80 不清标志）、且 `apu_single_3` 和 `apu_test`（2/7 FAIL）直接由它导致。

---

## 6. 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | `git diff --stat` 仅 `src/sound.cpp` + 数据文档 | ✅ |
| 2 | `ctest -LE perf` 33/33 PASS | ✅ |
| 3 | `FCEUX11_E3_TRACE=1` probe 有输出（E3 FSU/HOOK/W4017） | ✅ |
| 4 | 7 个 FAIL ROM $6000 与 §十 R6 一致 | ✅ |
| 5 | 5 个 PASS ROM 对照采集 | ✅ |
| 6 | 缺陷 1 证据（上电后 fcnt=0 → sirq=0x40） | ✅ |
| 7 | 缺陷 2 证据（V=$00 清掉 pre_sirq=0x40） | ✅ |
| 8 | 不动任何 APU 逻辑代码（仅 probe） | ✅ |
| 9 | R5 `runppu(3)` fix 未改 | ✅ |
| 10 | 不开新分支（仍在 wip_1.16） | ✅ |

---

## 7. 配套文件

- `src/sound.cpp`：probe 代码（+48/-0 行）
- `build-c1/r6_traces/*.err` / `*.out`：14 ROM × 600 帧 probe 数据（不进 git）

---

*数据文档完。等用户决策 R6-2a / R6-2b / R6-2c。*