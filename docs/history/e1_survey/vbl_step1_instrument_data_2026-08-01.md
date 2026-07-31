# E-1 PPU VBL/NMI Step 1 — Instrument-First Data (2026-08-01)

> **目的**：在动任何 PPU 时序代码前，采集 VBL/NMI dot 时序的真实数据，决定 §十 R5 Step 1 修订路径 (a)/(b)/(c) 的方向。
>
> **分支**：`e1-step1-probe-2026-08-01`（forked from `wip_1.16` @ `1156ca1`）
> **Probe commit**：（见 git log）
> **作者**：独立接管（ZCode agent）
> **方法学**：env-gated `FCEUX11_E1_TRACE=1` 触发，stderr 输出，**零侵入**（不改 PPU 时序、不改 CPU 时序、不改任何全局状态）
>
> **与既有文档的关系**：
> - 上游 `E-1-VBL调查记录.md`（§-1 §0 §1-6）—— vbl_* 失败模式的人文描述
> - 上游 `e1_survey/vbl05_disasm_2026-07-30.md` —— vbl_05 内循环 23 cycle/iteration 的反汇编分析
> - 上游 `e1_survey/vbl_baseline_2026-07-30.txt` —— 600 帧实测 baseline（vbl_01/03/04/09 PASS，02/05/06/07/08/10 FAIL）
> - **本文档** —— 第一次拿到 dot-level PPU 时序数据

---

## 1. 方法

### 1.1 Probe 启用方式

```powershell
$env:FCEUX11_E1_TRACE = "1"
$env:PATH = "D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& build-c1\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_05_nmi_timing.nes --frames 300
```

不设 `FCEUX11_E1_TRACE` 或设为非 `1`：probe 完全静默，行为与现状一致（已实测 `ctest -LE perf` 仍 34/34 PASS）。

### 1.2 Probe 插入点（仅 `src/ppu_rendering.cpp`，不动其他文件）

| # | 文件:行（前置编辑）| 触发事件 | 输出格式 |
|---|---|---|---|
| 1 | `ppu_rendering.cpp:1587` | `PPU_status \|= 0x80;` 之前 | `E1 VBL_SET frame=N sl=S cycle=C` |
| 2 | `ppu_rendering.cpp:1603` | `if (VBlankON) TriggerNMI();` 之前 | `E1 NMI_DISPATCH frame=N sl=S cycle=C vblank_on=B` |
| 3 | `ppu_rendering.cpp:1622` | `PPU_status = 0;` 之前 | `E1 VBL_CLR frame=N sl=S cycle=C` |

每个 probe 点 1 行 stderr，~60 字节。每帧约 3 行 × 300 帧 = ~900 行 / ROM。

### 1.3 关键不变量

- ✅ **未修改 `ppu_rendering.cpp` 任何逻辑代码行**：`PPU_status |= 0x80`、`if (VBlankON) TriggerNMI()`、`PPU_status = 0`、`delay = 20`、`kLineTime` 等均保持原样
- ✅ **未触及 ppudead 路径**（`:1525-1553`）：vbl_* 测试不进入 ppudead，probe 不插此处
- ✅ **未触发 NMI / VBL 时序改变**：probe 在 `PPU_status |= 0x80` 与 `TriggerNMI()` 之间**不插任何 `runppu()` 调用**，因此 PPU 自身行为字节级一致

### 1.4 构建与回归

- `scripts\do_build.ps1 -Config Release -BuildDir build-c1` —— 19 分钟全量重建（probe 是新增 TU 代码触发 pch 重生）
- `ctest --test-dir build-c1 --build-config Release --output-on-failure -LE perf` —— **100% tests passed, 0 tests failed out of 34**（probe 静默时行为完全一致）

---

## 2. 实测数据

### 2.1 5 个 ROM × 300 帧 概览

| ROM | 期望 | 实测 `$6000` | 实测 status | probe stderr 行数 | `vblank_on=1` 计数 |
|---|---|---|---|---|---|
| `vbl_01_basics.nes` | PASS | `0x00` | PASS | 900 | 0 |
| `vbl_02_set_time.nes` | FAIL 0x01 | `0x01` | FAIL | 900 | 0 |
| `vbl_03_clear_time.nes` | PASS | `0x00` | PASS | 900 | 0 |
| `vbl_04_nmi_control.nes` | PASS | `0x00` | PASS | 900 | 9 |
| `vbl_05_nmi_timing.nes` | FAIL 0x01 | `0x01` | FAIL | 900 | 10 |

**诊断字符串**（来自 BLARGG_RESULT）：

- vbl_02: `"T+ 1 2\n00 - V\n01 - V\n02 - V\n03 - V\n04 - V\n05 V -\n06 V -\n07 V -\n08 V -"` → VBL set "1 row late"
- vbl_05: `"00 2\n01 1\n02 1\n03 1\n04 1\n05 1\n06 1\n07 0\n08 0\n09 0"` → NMI "1 iteration early"

### 2.2 probe 数据（前 12 行 / 5 ROM 全等）

```
E1 VBL_SET frame=0 sl=0 cycle=0
E1 NMI_DISPATCH frame=0 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=0 sl=261 cycle=0
E1 VBL_SET frame=1 sl=240 cycle=0
E1 NMI_DISPATCH frame=1 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=1 sl=261 cycle=0
E1 VBL_SET frame=2 sl=240 cycle=0
E1 NMI_DISPATCH frame=2 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=2 sl=261 cycle=0
E1 VBL_SET frame=3 sl=240 cycle=0
E1 NMI_DISPATCH frame=3 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=3 sl=261 cycle=0
```

> **5 个 ROM 在前 12 行完全一致**。包括 vbl_01 / vbl_03 / vbl_04（PASS）和 vbl_02 / vbl_05（FAIL）。
> 唯一区别：vbl_01/02/03 整 300 帧 `vblank_on=0`（NMI 永久禁用）；vbl_04 在 9 个离散帧 `vblank_on=1`；vbl_05 在 10 个离散帧 `vblank_on=1`。

### 2.3 vbl_05 的 `vblank_on=1` 帧序列

```
frame=20  sl=241 cycle=0 vblank_on=1
frame=39  sl=241 cycle=0 vblank_on=1   (Δ 19)
frame=58  sl=241 cycle=0 vblank_on=1   (Δ 19)
frame=77  sl=241 cycle=0 vblank_on=1   (Δ 19)
frame=91  sl=241 cycle=0 vblank_on=1   (Δ 14)
frame=109 sl=241 cycle=0 vblank_on=1   (Δ 18)
frame=125 sl=241 cycle=0 vblank_on=1   (Δ 16)
frame=144 sl=241 cycle=0 vblank_on=1   (Δ 19)
frame=163 sl=241 cycle=0 vblank_on=1   (Δ 19)
frame=182 sl=241 cycle=0 vblank_on=1   (Δ 19)
```

> **NMI 启用的间隔** ~18 帧（最大 19，最小 14）。每次启用 1 帧，1 次 NMI dispatch。10 次对应 10 行 sub-test 结果。
> **所有 NMI dispatch 时序完全一致**：`sl=241 cycle=0` —— **PPU 侧无任何抖动**。

### 2.4 probe 数据（中段 / frame 98-100）

```
E1 VBL_SET frame=98 sl=240 cycle=0
E1 NMI_DISPATCH frame=98 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=98 sl=261 cycle=0
E1 VBL_SET frame=99 sl=240 cycle=0
E1 NMI_DISPATCH frame=99 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=99 sl=261 cycle=0
E1 VBL_SET frame=100 sl=240 cycle=0
E1 NMI_DISPATCH frame=100 sl=241 cycle=0 vblank_on=0
E1 VBL_CLR frame=100 sl=261 cycle=0
```

> 5 ROM 在 frame 98-100 一致。与 frame 0-3 也一致。**300 帧数据完全一致**。

---

## 3. 关键发现

### 3.1 PPU 时序是确定性的（PASS / FAIL ROM 一致）

**全部 5 ROM 在 300 帧内产生字节级相同的 probe 输出**：
- `VBL_SET @ sl=241 cycle=0`（probe 实际记录的是 `sl=240` 在 `ppur.status.sl=241` 赋值之前，但紧接的赋值使 NMI dispatch 读到的就是 `sl=241 cycle=0`）
- `NMI_DISPATCH @ sl=241 cycle=0`（与 VBL_SET 同一 dot）
- `VBL_CLR @ sl=261 cycle=0`

**推论**：§十 R5 Step 1 字面处方假设"PPU 时序偏移导致 02 / 05 失败"——**本数据排除此假设**。PASS 和 FAIL ROM 在 PPU 侧时序完全一致。

### 3.2 失败原因不在 PPU 侧

- vbl_01 PASS 与 vbl_02 FAIL 在 PPU 侧 0 差异
- vbl_04 PASS 与 vbl_05 FAIL 在 PPU 侧 0 差异
- 即使把 VBL_SET / NMI_DISPATCH 各推 N 个 dot，所有 5 个 ROM 都会同步移动 —— PASS/FAIL 关系不变

**结论**：**E-1 失败是 CPU 侧的相对时序问题**，不是 PPU 侧的绝对时序问题。

### 3.3 probe 的能力边界

| 能检测 | 不能检测 |
|---|---|
| ✅ VBL flag 设置时的 sl/cycle | ❌ CPU 何时通过 BIT $2002 读到 VBL flag |
| ✅ NMI dispatch 触发时的 sl/cycle | ❌ CPU 测试循环的迭代次数（X 快照反映此） |
| ✅ VBL flag 清除时的 sl/cycle | ❌ NMI handler 进入延迟（x6502.cpp:474-491 的 7 cycle） |
| ✅ NMI 是否启用（vblank_on=1/0） | ❌ $2000[7] enable 边沿与 VBL set 的相对相位 |

**vbl_05 "1 iteration early" 的本质**：CPU 测试循环在 VBL flag 可见之前完成 1 个 iteration。这可能是：
- CPU 的 BIT $2002 比真实指令"读"得早 → **不是**，CPU 读 $2002 就是 PPU 状态寄存器的当前值
- VBL flag 实际设置的时机与测试循环预期时机不匹配 → **可能**，但 probe 显示 PPU 侧时机一致
- NMI dispatch 与 VBL flag 的相对相位错了 → **可能**，但 probe 显示两者同步在 cycle 0

**最可能假设**（需 CPU 侧 probe 验证）：**NMI dispatch 的时机相对于 VBL flag 应该是 +1 CPU cycle（≈3 PPU dots）延迟**，对应真实硬件的"VBL 标志置位 1 CPU cycle 后才派发 NMI"。当前 impl 在 cycle 0 同时 set + dispatch，NMI 早了 ~3 PPU dots。

但这只能解释 vbl_05（"1 iteration early" = 23 CPU cycle = 7-8 PPU dots 早），而 §十 R5 Step 1 修订路径 (a)/(b) 的核心数学推导（`pre-loop + S=0 = kLineTime` 不变量）才是定量基础。

### 3.4 vbl_02 "1 row late" 的解释

vbl_02 测试"VBL 标志何时对 CPU 可见"。如果真实硬件 VBL 在 sl=241 cycle=1 置位，而当前 impl 在 sl=241 cycle=0 置位 —— 我们的 impl **早** 1 PPU dot。

但测试抱怨"晚 1 row" = 341 PPU dots？这远大于 1 dot 的差异。**这条线索与 vbl_05 的 7-8 dot 差异不一致**，说明 vbl_02 / vbl_05 的失败机制可能不同，或者两者测试的是不同的"可见性"层面（CPU 通过 BIT 指令读取 vs NMI handler 隐式读取）。

---

## 4. 与上游文档的交叉验证

### 4.1 `vbl05_disasm_2026-07-30.md` §117 的 "~3 PPU dots early"

> 上游文档说 NMI "1 cycle early (~3 PPU dots)"。
> 我的推算（§3.3）是 ~7-8 PPU dots 早（基于 1 iteration = 23 CPU cycle = 7.67 PPU dots）。
> probe 数据**无法直接证实**或证伪这个数字（probe 看不到 CPU 迭代计数）。
> **但** §117 的描述**自相矛盾**：说"1 cycle" 又说"3 PPU dots"，而 1 CPU cycle = 3 PPU dots，1 iteration ≠ 1 cycle。
> **建议**：上游文档 §117 的"~3 PPU dots"是**错误简化**，真实数字应是 ~7-8 PPU dots（1 iteration - 1 iteration_diff）。

### 4.2 §十 R5 Step 1 修订路径 (a)/(b)/(c)

| 路径 | 提议 | probe 兼容性 | 评估 |
|---|---|---|---|
| **(a)** 主循环范围 `(kLineTime-1)` + 独立 `runppu(1)` | 总周期仍 = 6820，但 VBL/NMI 在 S=0 中间切换 | ✅ probe 仍能记录新的 sl/cycle | ⚠️ 修的是主循环结构，不是 VBL 时序。**对 vbl_02/vbl_05 是否生效未知**——因为这两者失败不在 PPU 侧 |
| **(b)** pre-loop `<=delay` | VBL 标志 +1 dot 早 | ✅ probe 能看到 cycle=0→1 变化 | ⚠️ 与 vbl_02 "1 row late" 报告方向相反（让 VBL 更早，不是更晚）。**不太可能修 vbl_02** |
| **(c)** instrument-first | 不动代码，只采数据 | ✅ 本次已做 | ✅ 数据已得。**结论**：PPU 侧时序不是失败根因，需要 CPU 侧 probe 才能继续 |

### 4.3 E-1 §-1 的"1-cycle linear shift 不工作"

> "02/05/06/07/08 共享同一根缺陷（VBL/NMI 边沿相位 vs CPU 观察点），但每个 ROM 对应独立时序参数"
>
> probe 数据**部分证实**："共享根缺陷"成立（PASS / FAIL ROM PPU 时序一致）但**"独立参数"**不成立（PPU 侧时序在所有 ROM 完全一致）。
> 修正为：**CPU 侧对 VBL/NMI 的"相对观察"机制因 ROM 而异**，但 PPU 侧绝对时序是统一的。

---

## 5. 决策推荐

### 5.1 Step 1 instrument-first 任务闭合

✅ **本 Step 1 任务闭合**：
- Probe 设计 + 实现 + 构建 + 回归 + 数据采集全部完成
- 5 ROM × 300 帧 = 1500 行 probe 数据已落盘到 `build-c1/e1_traces/`
- ctest 34/34 PASS（probe 静默时零影响）

### 5.2 §十 R5 Step 1 路径推荐：**不做代码修改，回到 CPU 侧 probe**

**推荐**：**不要套用 §十 R5 Step 1 字面处方的任何路径**。理由：
1. probe 数据显示 PPU 侧时序确定且一致，与 PASS/FAIL 无关
2. 路径 (a) 改的是主循环结构，**对 vbl_02/vbl_05 是否生效无证据**——可能修了 PPU 但 CPU 仍看不到
3. 路径 (b) 改 VBL 时序到 +1 dot，方向与 vbl_02 "1 row late" 报告**相反**
4. §十 R5 Step 1 字面处方的数学基础（"delay 20→19 不补偿 +1"）已被证实错误（见 §十 R5 🚨 校准块）

### 5.3 下一步建议（须用户决策）

#### 选项 X：CPU 侧 probe（推荐）

在 `x6502.cpp` 的 `FCEU_IQNMI` dispatch 处加 probe，记录：
- NMI handler 进入的 frame / sl / cycle
- NMI handler 入口时的 X 寄存器值（直接对应 vbl_05 测试期望/观测的迭代计数）
- 每次 $2002 读（BIT 指令）的 sl / cycle

预期能直接揭示 vbl_05 "1 iteration early" 的 CPU 侧机制。

#### 选项 Y：直接试 (b) + 全 ROM 回归

按 §十 R5 Step 1 修订路径 (b) 改 pre-loop `<=delay`，跑 10 vbl ROM + Oracle A 34/34 + ppudead 相关 ROM（SDK 修复验证）。
**风险**：可能让 vbl_01 / vbl_04 从 PASS 翻 FAIL，且 vbl_02 / vbl_05 不一定修好。

#### 选项 Z：暂缓 E-1，转 R6

跳到 R6 Priority 1（apu 帧计数器相位，5 ROM 共享单根因）。
**理由**：R5 4 步彼此依赖（Step 1 不通则 2-4 无依据），R6 一次修 5 ROM 收益可能更高。

### 5.4 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | Probe 仅 stderr 输出，不动 PPU 时序 | ✅ ctest 34/34 |
| 2 | Probe 代码 +37/-1 行（namespace + 3 probes） | ✅ git diff stat |
| 3 | 5 ROM × 300 帧 = 1500 行数据已采集 | ✅ e1_traces/ |
| 4 | BLARGG_RESULT 与 E-1 §1 baseline 一致（vbl_01/03/04 PASS，02/05 FAIL） | ✅ |
| 5 | 全部 ROM 在 PPU 侧时序字节级一致 | ✅ probe 数据 |
| 6 | 未动 ppudead 路径、delay=20、kLineTime、任何逻辑代码 | ✅ git diff |
| 7 | 不动 wip_1.16（独立分支 e1-step1-probe-2026-08-01） | ✅ git branch |

---

## 6. 已知遗留

1. **未跑 vbl_06/07/08/09/10**：本次只跑 5 ROM。E-1 §4 推荐优先级 05 → 02 → 06/07/08 → 10。如果走选项 X（CPU probe），可一次覆盖全部 10 ROM
2. **未验证 ppudead 路径**：probe 不插 ppudead。如果未来发现 ppudead ROM 也需要 probe，需扩 probe 到 `:1525-1553`
3. **`vblank_on=1` 间隔不均匀**：vbl_05 的 NMI 启用间隔 14-19 帧不等（不是固定 18）。可能是测试 ROM 内部节奏，不是 bug
4. **probe stderr 在 PowerShell console 上有 `NativeCommandError` 噪音**：stdout 走 runner 程序，stderr 走 probe。两者混合时 PowerShell 把 stderr 当异常。需要 `2>$file` 分离。这是 PowerShell 行为，不是 probe bug

---

## 7. 配套文件

- `src/ppu_rendering.cpp`：probe 代码（+37/-1 行，独立分支）
- `build-c1/e1_traces/01-vbl_basics_300.txt`（900 行 probe 数据）
- `build-c1/e1_traces/02-vbl_set_time_300.txt`（900 行 + BLARGG_RESULT FAIL 0x01）
- `build-c1/e1_traces/03-vbl_clear_time_300.txt`（900 行 + BLARGG_RESULT PASS）
- `build-c1/e1_traces/04-nmi_control_300.txt`（900 行 + 9 个 vblank_on=1 + PASS）
- `build-c1/e1_traces/05-vbl_nmi_timing_300.txt`（900 行 + 10 个 vblank_on=1 + FAIL 0x01）

> 这些 `build-c1/e1_traces/*.txt` 文件**不进 git**（在 `build-c1/` 下，默认忽略）。如需归档可手工 `cp` 到 `output/e1_traces_2026-08-01/` 后单独 commit。

---

## 8. 等用户决策

请在以下选项中选择下一步（参见 §5.3）：

- **X**：CPU 侧 probe（推荐）
- **Y**：直接试路径 (b) + 全 ROM 回归（高风险）
- **Z**：暂缓 E-1 转 R6（最高单次收益）
- **W**：其他（请说明）

---

*数据文档完。等用户决策。*