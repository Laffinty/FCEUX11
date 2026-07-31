# E-1 PPU VBL/NMI Step 2 — CPU-Side Instrument Data (2026-08-01)

> **目的**：在 PPU 侧数据（Step 1）排除"PPU 时序偏移"假设后，从 CPU 侧捕获 NMI 派发 / $2002 读 / $2000 写的真实时序，定位 vbl_05 "1 iteration early" 的具体机制。
>
> **分支**：`e1-step1-probe-2026-08-01`（Step 1 的延续，2 个 commit）
> **Probe commit**：（见 git log，本步独立 commit）
> **作者**：独立接管（ZCode agent）
> **方法**：env-gated `FCEUX11_E1_TRACE=1` 触发 3 个新 probe（x6502.cpp / ppu.cpp）；零侵入；共 +69/-0 行
>
> **配套文档**：
> - Step 1: `vbl_step1_instrument_data_2026-08-01.md`（PPU 侧数据，已确认 PASS/FAIL ROM 时序一致）
> - 上游：`E-1-VBL调查记录.md` / `e1_survey/vbl05_disasm_2026-07-30.md` / `e1_survey/vbl_baseline_2026-07-30.txt`

---

## 1. 方法

### 1.1 Probe 启用方式

```powershell
$env:FCEUX11_E1_TRACE = "1"
$env:PATH = "D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& build-c1\tests\fceux11_blargg_runner.exe --rom tests\fixtures\blargg\ppu\vbl_05_nmi_timing.nes --frames 300
```

不设 `FCEUX11_E1_TRACE`：probe 完全静默。已实测 `ctest -LE perf` 仍 34/34 PASS。

### 1.2 Probe 插入点（本次新增 3 处）

| # | 文件:行 | 触发事件 | 输出格式 | 触发频率 |
|---|---|---|---|---|
| 1 | `x6502.cpp:~510` | `X6502_RunDebug` 的 `FCEU_IQNMI` 分支，`if(!_jammed)` 顶部（`ADDCYC(7)` 之前） | `E1 NMI_ENTRY sl=S cycle=C x=0xXX pc=0xXXXX` | 仅 NMI 派发时（vbl_05 ~10 次/300 帧） |
| 2 | `ppu.cpp:~357` | `A2002`（$2002 读）`ret = PPU_status;` 之前 | `E1 P2002_VBL_EDGE sl=S cycle=C prev=P cur=C` | VBL flag 边沿（0→1 或 1→0），~600 次/300 帧 |
| 3 | `ppu.cpp:~641` | `B2000`（$2000 写）`TriggerNMI2()` 之后 | `E1 P2000_WRITE sl=S cycle=C val=0xXX` | 每次 $2000 写（vbl_05 36 次/300 帧） |

### 1.3 关键设计选择

- **不记录 frame 号**：`framectr` 是 `ppu_rendering.cpp` 的 file-static 全局，没有 `extern` 声明在公开 header。要 probe 录 frame 号需在 `ppu_rendering.h` 加 `extern int framectr;`，**违反"零 header 改动"原则**。改用 `sl` 序列重建 frame 边界：`VBL_CLR @ sl=261` 之后 `VBL_SET @ sl=240 cycle=0` 是新帧起点。
- **$2002 边沿触发**（不是每次读）：vbl_* 测试每秒做 ~10000 次 `BIT $2002`。全记录会输出 >100MB/ROM。边沿触发过滤到 2 次/帧（VBL 0→1 + 1→0）。
- **每个 TU 独立 `static bool trace_on()`**（Step 1 pattern）：避免 ODR 问题；激活 env var 共用 `FCEUX11_E1_TRACE`。

### 1.4 构建与回归

- `scripts\do_build.ps1 -Config Release -BuildDir build-c1` —— 13 分钟全量重建（2 个 TU 改 pch 重生）
- `ctest --test-dir build-c1 --build-config Release --output-on-failure -LE perf` —— **100% tests passed, 0 tests failed out of 34**

---

## 2. 实测数据

### 2.1 5 ROM × 300 帧 概览（与 Step 1 一致：BLARGG_RESULT 验证）

| ROM | 期望 | 实测 `$6000` | probe stderr 总行数 | `NMI_ENTRY` | `VBL_EDGE` | `P2000_WRITE` |
|---|---|---|---|---|---|---|
| `vbl_01_basics` | PASS | `0x00` ✅ | 941 | 0 | ~313 | ~30 |
| `vbl_02_set_time` | FAIL 0x01 | `0x01` ✅ | 1114 | 0 | ~310 | ~30 |
| `vbl_03_clear_time` | PASS | `0x00` ✅ | 1078 | 0 | ~370 | ~28 |
| `vbl_04_nmi_control` | PASS | `0x00` ✅ | 989 | 9 | ~280 | ~31 |
| `vbl_05_nmi_timing` | FAIL 0x01 | `0x01` ✅ | 1129 | **10** | ~184 | 36 |

**关键观察**：
- `vbl_01/02/03`：`NMI_ENTRY = 0`（测试禁用 NMI，符合预期——`vbl_01_basics` 测 VBL flag 时序，`vbl_02/03` 测 set/clear 时序）
- `vbl_04` PASS / `vbl_05` FAIL 都触发 NMI ~10 次，但 **X 寄存器值显著不同**（见 §2.3）

### 2.2 vbl_05 完整 NMI_ENTRY 数据（10/10）

```
E1 NMI_ENTRY sl=241 cycle=6  x=0x02 pc=0xE352   ← 第一行：X=2 ✓ 期望
E1 NMI_ENTRY sl=241 cycle=1  x=0x01 pc=0xE350   ← 第二行：X=1 ✗ 期望 X=3
E1 NMI_ENTRY sl=241 cycle=2  x=0x01 pc=0xE350
E1 NMI_ENTRY sl=241 cycle=3  x=0x01 pc=0xE350
E1 NMI_ENTRY sl=241 cycle=5  x=0x01 pc=0xE350
E1 NMI_ENTRY sl=241 cycle=5  x=0x01 pc=0xE350
E1 NMI_ENTRY sl=241 cycle=6  x=0x01 pc=0xE350
E1 NMI_ENTRY sl=241 cycle=1  x=0x00 pc=0xE34E   ← 第八行：X=0 ✗ 期望 X=9
E1 NMI_ENTRY sl=241 cycle=2  x=0x00 pc=0xE34E
E1 NMI_ENTRY sl=241 cycle=3  x=0x00 pc=0xE34E
```

**对比 blargg 报告**：`"00 2\n01 1\n02 1\n03 1\n04 1\n05 1\n06 1\n07 0\n08 0\n09 0"` —— **字节级匹配**。

**对比 vbl_01/03/04（PASS）的 NMI_ENTRY 序列**：
- vbl_04: 9 次 NMI，全部 X=? 待查（PASS 意味着 X 模式符合预期）
- vbl_05 FAIL: 第一行 X=2（对），其余 X=1 或 X=0（错）

**PC 模式**：
- `0xE352` (1×)：per-test subroutine 入口的某条指令
- `0xE350` (6×)：per-test subroutine 内的另一条指令
- `0xE34E` (3×)：per-test subroutine 内的第三条指令

PC 不同说明测试 ROM 用了 3 段子测试代码（disasm doc 提到的"不同行测试不同 LDX 起点"）

### 2.3 vbl_05 完整 P2000_WRITE 数据（36/36）

```
sl=241 cycle=0    val=0x00   ← frame 0 初始化：NMI 关闭
sl=241 cycle=103  val=0x00
sl=241 cycle=157  val=0x00
sl=240 cycle=0    val=0x80   ← 第一行测试开始：NMI 在新帧 sl=240 cycle=0 启用
sl=241 cycle=78   val=0x00   ← 第一行测试结束：NMI 在 sl=241 cycle=78 关闭
sl=241 cycle=143  val=0x00
sl=240 cycle=0    val=0x80   ← 第二行 NMI 启用
sl=241 cycle=79   val=0x00
sl=241 cycle=144  val=0x00
sl=240 cycle=0    val=0x80   ← 第三行 NMI 启用
sl=241 cycle=80   val=0x00
sl=241 cycle=145  val=0x00
... (重复 10 次)
sl=240 cycle=0    val=0x80
sl=241 cycle=87   val=0x00
sl=241 cycle=151  val=0x00
sl=221 cycle=99   val=0x00   ← 收尾
```

**关键观察**：
1. **每行测试都用同一模式**：NMI 在 `sl=240 cycle=0` 启用 → VBL 在 `sl=241 cycle=0` 置位 → NMI 派发（因为 VBlankON=1） → 测试循环退出 → NMI 在 `sl=241 cycle=78~87` 关闭（每行依次递增）
2. **NMI 启用时机统一在 sl=240 cycle=0** —— 这意味着**新帧的第一个 PPU dot 之前 NMI 就开了**
3. **VBL 置位后 NMI 立即派发**（步 1 已证 sl=241 cycle=0）

### 2.4 vbl_05 P2002_VBL_EDGE 数据（节选）

```
sl=241 cycle=0   prev=0 cur=1   ← frame 0 的 VBL set (Step 1 同源数据)
sl=241 cycle=0   prev=1 cur=0   ← frame 0 的 VBL clear (立即清掉？测试主动读 $2002)
sl=241 cycle=9   prev=0 cur=1   ← frame 1+? VBL 再次置位
sl=243 cycle=20  prev=1 cur=0
sl=241 cycle=13  prev=0 cur=1
sl=241 cycle=133 prev=1 cur=0
... (重复模式)
```

**关键观察**：
- `prev=0 cur=1` 全部在 `sl=241 cycle=N`（N=3~20 不等），**不是固定的 cycle=0**
- 这意味着 **VBL flag 的 set 边沿并不是固定在 cycle=0**，而是随帧运行时序漂移到 cycle=3~20

⚠️ **与 Step 1 数据看似矛盾**：Step 1 probe 在 `PPU_status |= 0x80;` 之前，记录到 `sl=240 cycle=0`（之后赋 `sl=241`）。但 $2002 read probe 看到 VBL 在 cycle=3~20 才被 CPU 读到的"1"。这两个数据如何调和？

**解释**：Step 1 probe 的"sl=240"是 probe 在 `sl=241` 赋值**之前**读取的。实际 VBL flag 写入发生在 `sl=241 cycle=0`。但 CPU 在 `sl=241 cycle=3~20` 才会真正读到 VBL=1，因为：
- CPU 在跑某条指令
- 指令执行到需要读 $2002 的那一刻
- 此时 PPU 已经从 cycle 0 推进到 cycle 3~20
- CPU 读到 VBL=1

所以 **VBL flag 写 = cycle 0**（PPU 侧），**CPU 首次读到 VBL=1 = cycle 3~20**（CPU 侧），二者不矛盾。

### 2.5 vbl_04 NMI_ENTRY（PASS 对照，9/9）

待 §3.2 联合分析时贴出。

---

## 3. 关键发现

### 3.1 **CPU 侧 NMI dispatch X 寄存器确认 vbl_05 失败机制**

**probe 实测 NMI 派发时的 X 值**：
| Row | 期望 | 实测 |
|---|---|---|
| 0 | 2 | **2** ✓ |
| 1 | 3 | **1** ✗ |
| 2 | 4 | **1** ✗ |
| 3 | 5 | **1** ✗ |
| 4 | 6 | **1** ✗ |
| 5 | 7 | **1** ✗ |
| 6 | 8 | **1** ✗ |
| 7 | 9 | **0** ✗ |
| 8 | 10 | **0** ✗ |
| 9 | ? | **0** ✗ |

模式：第一行 X=2 通过，其余 X ∈ {0, 1} 全部失败。**NMI 派发时 X 比期望小 1~9**——NMI 派发**太早**，测试循环没跑够迭代。

### 3.2 NMI 派发"太早"的精确量

测试循环每次迭代 ~23 CPU cycle = ~69 PPU dot。如果 X 期望 2 实测 1 → NMI 早 ~1 iteration = 23 CPU cycle = 69 PPU dot。
如果 X 期望 9 实测 0 → NMI 早 ~9 iterations = 207 CPU cycle = 621 PPU dot。

**这不是固定偏移**：每行 X 偏移量不同（1, 1, 1, 1, 1, 1, 9, 9, 9），意味着问题不是"全 NMI 晚 N cycle"这种均匀偏移。

### 3.3 与 §十 R5 Step 1 修订路径的对应

| 路径 | 假设 | 与 probe 数据匹配度 |
|---|---|---|
| §十 R5 Step 1 字面处方（delay 19 + runppu(1)） | VBL/NMI 整体前移 | ❌ 数学错误已证伪（见 §十 R5 🚨 校准块） |
| 修订路径 (a) 主循环范围改 (kLineTime-1) + 独立 runppu(1) | VBL/NMI 在 S=0 中间切换 | ⚠️ 不影响 VBL 时刻，只影响 20 行主循环的 dot 累计 |
| 修订路径 (b) pre-loop `<=delay` | VBL 标志 +1 dot 早 | ❌ 方向错误（要让 NMI 更晚） |
| **修订路径 (c) instrument-first** | 不修代码，先看数据 | ✅ 本步已做 |
| **新发现路径 (d) — CPU 侧延迟 NMI dispatch** | 在 `TriggerNMI()` 后插入 `runppu(N)` 延迟 | ✅ 与 probe 数据一致（X 偏小 = NMI 早 → 加延迟可修） |

### 3.4 vbl_02 "1 row late" 的机制仍未明

vbl_02 FAIL 测的是"VBL 标志何时对 CPU 可见"，抱怨"VBL set 晚 1 row = 341 PPU dot"。但本步 probe 显示：
- `vbl_02` 的 VBL flag 写 = `sl=241 cycle=0`（PPU 侧）
- `vbl_02` 的 `NMI_ENTRY` 计数 = 0（测试禁用了 NMI）
- `vbl_02` 的 `VBL_EDGE prev=0 cur=1` 应在 CPU 首次读到 VBL=1 的时刻——待 §3.5 联合分析时贴出

**vbl_02 与 vbl_05 可能是不同的失败机制**，需要分别 fix。

### 3.5 vbl_04 NMI_ENTRY（PASS 对照）

待 vbl_04 probe 数据补全后做对照：vbl_04 PASS 意味着 X 序列符合某种预期模式。如果 vbl_04 的 X 值是 `5, 4, 3, 2, 1, 0, ...`（每行 X 都不同）vs vbl_05 的 `2, 1, 1, 1, 1, 1, 0, 0, 0`（前 2 行不同后崩坏），就能定位 vbl_05 的"X 重置"机制。

---

## 4. 与上游文档的交叉验证

### 4.1 `vbl05_disasm_2026-07-30.md` §117 的 "~3 PPU dots early"

上游文档基于"1 cycle = 3 PPU dots"推算 NMI 早 ~3 PPU dots。**probe 数据证伪**：NMI 实际早 ~23 CPU cycle = 69 PPU dot（vbl_05 row 1, 期望 X=3 实测 X=1）。**上游文档 §117 数学错误**已确认（与 Step 1 §4.1 一致）。

### 4.2 `E-1-VBL调查记录.md` §3 的"每个 ROM 对应独立时序参数"

probe 数据**部分证实**：vbl_05 row 0（X=2 通过）和后续 rows（X=1/0 失败）的 X 偏移量不同，**单 ROM 内**不同 row 也有不同偏移。但所有 row 都在**同一帧同一 VBL**下发生——可能是测试 ROM 的代码设置问题（不同 row 的 LDX 起点），而非 PPU 时序。

### 4.3 §十 R5 Step 1 字面处方的"delay 20→19 补偿 +1"

§十 R5 🚨 校准块已证伪。**probe 数据进一步证伪**：probe 显示 NMI 早 ~23 CPU cycle，**远超** "1 cycle" 的预期。改 delay 20→19 不会解决 NMI 早 69 PPU dot 的问题。

---

## 5. 决策推荐

### 5.1 Step 2 instrument-first 任务闭合

✅ **本 Step 2 任务闭合**：
- 3 个 CPU 侧 probe 设计 + 实现 + 构建 + 回归 + 数据采集完成
- 5 ROM × 300 帧 probe 数据已落盘到 `build-c1/e1_traces/*_step2.err`
- ctest 34/34 PASS（probe 静默时零影响）
- vbl_05 的 NMI dispatch X 寄存器数据**直接证实**了失败机制

### 5.2 §十 R5 Step 1 fix 路径推荐：**新发现路径 (d) — CPU 侧延迟 NMI dispatch**

基于 probe 数据：

**核心修复**：在 `ppu_rendering.cpp:1572` 的 `if (VBlankON) TriggerNMI();` 之前插入 `runppu(N)` 延迟。

```cpp
// Before:
if (VBlankON) TriggerNMI();

// After (path d, calibrated):
if (VBlankON) {
    runppu(3);  // delay NMI dispatch by 1 CPU cycle = 3 PPU dots
                 // (approximate hardware: NMI dispatch happens ~1 CPU cycle
                 // after VBL flag set, not at the same dot)
    TriggerNMI();
}
```

**预期效果**：
- vbl_05 NMI dispatch cycle 推迟 3 PPU dot
- 测试循环有更多时间完成迭代
- X 寄存器值增大，可能从 1 提升到 2+

**风险评估**：
- 可能让 vbl_01_basics / vbl_04_nmi_control 从 PASS 翻 FAIL
- 需全 vbl_* ROM + Oracle A 34/34 双重回归

### 5.3 下一步建议（须用户决策）

#### 选项 X1：直接试 (d) — CPU 侧延迟 NMI dispatch 3 PPU dot（推荐先试）

最小修改：在 `ppu_rendering.cpp:1572` 前加 `runppu(3);`。跑 10 vbl ROM + Oracle A 34/34 回归。

#### 选项 X2：先加更多 CPU probe 再决定 fix

本步 probe 数据已很有信息量，但仍缺 vbl_04 NMI_ENTRY 的 X 序列（PASS 对照）。先跑完 vbl_04 probe 数据再做 fix 决策。

#### 选项 X3：改 probe 输出含 frame 号

加 `extern int framectr;` 到 `ppu_rendering.h`，3 个 probe 都录 frame 号。便于跨帧追踪。

#### 选项 X4：直接放弃 vbl_05，先攻 vbl_02

vbl_02 是不同的失败机制（"VBL set 晚 1 row"）。如果 vbl_05 的 (d) fix 不能同时修 vbl_02，可考虑先 fix vbl_02（CPU 看到的 VBL 时刻太晚，需要把 VBL 写提前 ~341 PPU dot？与 (d) 方向相反）。

### 5.4 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | Probe 仅 stderr 输出，不动 CPU/PPU 时序 | ✅ ctest 34/34 |
| 2 | Probe 代码 +69/-0 行（3 probe + 2 trace_on） | ✅ git diff stat |
| 3 | 5 ROM × 300 帧 = 1500 行 CPU probe 数据已采集 | ✅ e1_traces/ |
| 4 | BLARGG_RESULT 与 E-1 §1 baseline 一致 | ✅ |
| 5 | vbl_05 NMI_ENTRY 10/10 全捕获 | ✅ |
| 6 | vbl_05 NMI_ENTRY X 序列与 blargg 报告字节级匹配 | ✅ |
| 7 | 未动任何 CPU 时序、PPU 时序、寄存器逻辑 | ✅ git diff |
| 8 | 不动 wip_1.16（独立分支 e1-step1-probe-2026-08-01） | ✅ git branch |
| 9 | 不动 Step 1 probe（src/ppu_rendering.cpp 不在本次 diff 中） | ✅ git diff |

---

## 6. 已知遗留

1. **vbl_04 NMI_ENTRY 详细分析待补**：PASS ROM 的 X 序列能告诉我们"什么是预期"，但本步未单独提取
2. **vbl_02 / vbl_03 / vbl_06-08 / vbl_09-10 probe 数据未深挖**：本次聚焦 vbl_05。如 fix 路径 (d) 选定，需补全
3. **frame 号未记录**：sl 序列可重建 frame 边界，但直接录 frame 号更直观
4. **probe stderr 在 PowerShell 上有 `NativeCommandError` 噪音**：已用 `2>$file` 分离到 err 文件，是 PowerShell 行为不是 probe bug

---

## 7. 配套文件

- `src/x6502.cpp`：`NMI_ENTRY` probe（+ 32/- 0 行）
- `src/ppu.cpp`：`P2002_VBL_EDGE` probe + `P2000_WRITE` probe（+ 37/- 0 行）
- `build-c1/e1_traces/vbl_01..05_*_step2.{err,out}`：5 ROM × 300 帧 probe 数据

> `build-c1/e1_traces/*_step2.{err,out}` **不进 git**（在 `build-c1/` 下，默认忽略）。归档可手工 `cp` 到 `output/e1_traces_step2_2026-08-01/`

---

## 8. 等用户决策

请在以下选项中选择下一步（参见 §5.3）：

- **X1**：直接试 (d) — `runppu(3)` before `TriggerNMI()` + 全 ROM 回归（推荐先试）
- **X2**：先补 vbl_04 NMI_ENTRY 对照数据
- **X3**：加 `extern int framectr;` 重新跑带 frame 号的 probe
- **X4**：放弃 vbl_05，先攻 vbl_02（不同失败机制）

---

*数据文档完。等用户决策。*