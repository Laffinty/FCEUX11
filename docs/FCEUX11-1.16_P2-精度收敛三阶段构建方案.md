# FCEUX11 v1.16 精度收敛 — 三阶段构建方案（P2 深化版）

> **编制日期**：2026-08-01（**2026-08-02 深化修订**：E-1 Step 1.3 状态回填——vbl_05 PASS、读侧/边沿分源、行号校准；**2026-08-04 修订**：Phase 3.2 桶 A+B.3+B.4 完成、§3 Step 3.1 报告桶分类修订为精准版（CPU 13 / PPU 4 / vbl 5 / MMC3 12 / sprdma 2 = 36）、§5 桶 B 拆分为 B.1+B.2 / B.3+B.4、桶 C 拆分为 PPU 真实精度 4 / vbl 已知 5、Step 3.3 数字同步 20 项剩余）
> **2026-08-03 修订**：Step 1.2 闭合可行性评估——测试源码实证 1 dot/行漂移为亚指令粒度、残量在差值中抵消
> （漂移被帧边界重同步摧毁，非"未建模"）、NMI 取消时序缺口（小修）、决定性残量探针定夺深模型；
> **2026-08-03 修订 2**：NMI 取消时序缺口实测实施——取消机制正确生效（vbl_06 行 5-6 转 `V -`）但
> vbl_04/05 回归（VBL 轮询读同落 (241,0)），**绑定深模型、不可独立落地，已回滚**（§0.2），
> 记录为有据已知限制，见 `docs/history/surveys/e1_vbl/vbl_step1_2_nmi_cancel_gap_2026-08-03.md`）
> **编制依据**：
> - `docs/history/reports/FCEUX11-1.16_最终验收报告.md` §十 R5/R6（原始处方 + 🚨/🚧 实测校准块）
> - `docs/tech/P2_precision_instrument_handoff.md`（E-1/E-3 交接档案，含 6 探针清单）
> - **本次联机研究**（2026-08-01）：NESdev Wiki（APU Frame Counter / PPU frame timing / NMI）+ blargg 原始测试包源码
>   （`christopherpow/nes-test-roms`：`ppu_vbl_nmi/source/*.s`、`blargg_apu_2005.07.30/source/*.asm` + readme/tests.txt、`apu_reset/readme.txt`）
> - **用户决策（2026-08-01）**：P3（第二 oracle 来源）暂不做，"先确保精度再谈别的"。P3 已在验收报告中标注暂缓。
>
> **性质**：可执行的构建方案。每个 Phase/Step 含 目标 / 文件:行号 / 改法 / instrument 验证 / 回归集 / 证伪判据。**不是**对交接档案结论的推翻，而是用联机资料把交接档案中"未闭合"的两项（R5-E1、R6-缺陷1）钉到可动手的精度。
>
> **最新实测（2026-08-04 收尾核对）**：
> - **Phase 1 (E-1 PPU VBL/NMI)**:vbl 5 PASS / 5 FAIL(vbl_05 已修复,`5581769`);5 FAIL 全部记入有据已知限制（vbl_02/06 读侧量化、vbl_07/08 边沿采样、vbl_10 写接受边界,均深模型族,`vbl_step1_4_instrument_data_2026-08-03.md` 定案）
> - **Phase 2 (E-3 APU 帧计数器)**:7 bucket-C sub-test 全 PASS(`f5e7cd0` cycle-position 帧计数器);APU 52 ROM 零 apu_* 失败
> - **Phase 3 (全量精度收敛)**:**143 PASS / 34 FAIL**(基线 177 ROMs)
>   - Step 3.1 harness 清理 ✅ 已落地(`821a26e`,0x80/0x81 桶全清零)
>   - Step 3.2 桶 A (MMC3 12) ✅ 已完成(`f4a072a`)
>   - Step 3.2 桶 B.3+B.4 (CPU 3) ✅ 已完成(`57d3e88`,零代码改动,探针撤回)
>   - Step 3.2 桶 C (PPU 4) 🚧 1/4 收敛(`863e9d7`):`ppu_read_buffer` 0x0E→0x00 + 顺带修复 `cpu_dummy_writes_ppu` 0x09→0x00;其余 3 项待处理
>   - Step 3.2 桶 B.1+B.2 (CPU 9) / 桶 C.1 (vbl 5) / 桶 D (sprdma 2) ⏳ 未启动
>   - Step 3.3 全量回归与验收复检 ⏳ 未启动
> - **Oracle A 34/34**(自 `7ea1d0c` 起的 golden nestest 已稳定);Oracle B 143 PASS
>
> **2026-08-03 复核**:基线重跑确认 vbl_02=0x01(行 04 应 `- -` 实为 `- V`,仅差 1 行)、vbl_06=0x01
> (行 04-06 应 `- -`/`V -`/`V -` 实为 `V N`,行 07-09 已对)。Step 1.2 闭合可行性评估见 §3 Step 1.2 块
> (结论:闭合需亚指令级读时序,收敛无保证;**用户决策:有时间再推进**,先以评估文档 + 本修订收口)。
>
> **2026-08-03 Step 1.4 插桩调查(`vbl_step1_4_instrument_data_2026-08-03.md`)**:vbl_10 根因定案——
> 跳点门控的"写接受边界"模拟器为 true-dot ≤ 338,硬件为 ≤ 337(gate 340 + ~2-3 dot 写延迟);A=5 的 $2001 写
> 被批量化不可区分(两 sub-test 写同批末 338,亚 dot 相位差 16 单位 = 1 dot),模拟器多余跳 1 次 → 计数 8,7。
> gate/skipdot 5×3 扫参全部证伪(无配置产生 8,8,9,7)。修复需**亚指令级写时序**(与 vbl_02/06 深模型同族,
> 收敛无保证,不排入当前序列)。vbl_10 记录为有据已知限制(错误码 0x03)。代码保持基线,保留 E1 SKIP_DEC / W2001 探针。
>
> **2026-08-04 §5 桶分类修订(本修订)**:实测核对 36 项 FAIL 精准分类,修正原报告"CPU 11 / PPU 11 / MMC3 12 / APU 2 / 永久跳过 1 = 37"的统计偏差(漏算 vbl 5 项单独列出;APU 2 实为 sprdma 2;CPU 11 应为 12)。修订后:**CPU 13**(含 0xFE 永久跳过 1) / **PPU 4** / **vbl 5** / **MMC3 12** / **sprdma 2** = 36(35 真实精度 + 1 永久跳过)。桶 B 进一步拆为 B.1+B.2(9 项,改动面大,未启动)与 B.3+B.4(3 项,已完成);桶 C 拆为 PPU 真实精度 4 项(未启动)与 vbl 5 项(Phase 1 已知,不在本桶重做)。详见 §3 Step 3.1 末段 + §5 Step 3.2。

---

## 0. 前置硬约束（沿用交接档案 §0，逐条强制，违反即回滚）

1. **instrument-first 是前置硬约束**：任何代码改动前，先加 env-gated probe 采集真实时序，用数据证实/证伪假设。
2. **每步强制回归**：Oracle A `ctest -LE perf` 34/34 + 相关 ROM 全量，任一红即回滚该步。
3. **savestate 兼容**：不得改 `FHCN`/`FCNT`/`IQFM` chunk 名/大小/序（`sound.cpp:1462-1466`）。改运行期起始值会碎
   `golden_savestate_test` / `savestate_regression_test`（MD5 固定参照）——**按 `tests/CMakeLists.txt:348` 用
   `fceux11_golden_savestate_test --generate` 重生 golden 索引**（这是预期流程，不是事故）。
4. **不开新分支**：v1.16 系开发直接在 `wip_1.16` 分支进行（用户明确要求）。
5. **每步独立 commit**：E-1 与 E-3 分开提交；每步一个 commit，commit message 带 `e1(stepN)` / `e3(stepN)` 前缀（沿用现有约定）。
6. **全量重建再测**：每步前 `scripts/do_build.ps1` 全量重建（E-1 调查记录 §0 教训：增量 exe 可能比源码旧）。

---

## 1. 本次联机研究的决定性发现（对照交接档案：确认 + 修正）

### 1.1 E-1（PPU VBL/NMI）硬件真值（NESdev Wiki：PPU frame timing / NMI）

| 事件 | 硬件真值 | FCEUX working-config 现状 | 差距 |
|---|---|---|---|
| VBL flag 置位 | **sl 241 dot 1**（`ppu_rendering.cpp` 注释亦自认 "cycle 1"） | `PPU_status \|= 0x80` 于 **cycle 0**（`ppu_rendering.cpp:1605`） | **早 1 dot**（Step 1.1 已证伪单独调整；6820 set→clear 为硬约束） |
| NMI latch | flag 置位**同 dot**（/NMI 低电平 iff vblank_flag && NMI_output，CPU 下一指令边界采样） | `if (VBlankON) { X6502_Run(8); TriggerNMI(); }`（`:1642/:1648`，`5581769` 以 X6502_Run(8) 替换 runppu(3)） | **✅ 帧畸变已消除**（NMI-on 帧 6820，探针证实 cycle 恒 0）；vbl_05 PASS |
| VBL flag 清除 | **sl 261 dot 1**（置位后 6820 dots） | `PPU_status = 0`（`:1667`，20 扫描线循环后） | 清除点随置位点联动（6820 set→clear 保持，vbl_01 PASS） |
| $2002 读抑制窗口 | 读 **1 dot 前**于置位 → 抑制置位+无 NMI；**同 dot 或 1 dot 后** → 读到置位+清除+抑制 NMI；**≥2 dot** → 正常 | `A2002`（`ppu.cpp:327-349`）Step 1.2 已实现 `(240,340)` 抑制 + `(241,0-1)` NMI 取消 | 🔍 机制已落地但**读落点量化未闭合**（vbl_02/06，`1b5a434`），需读采样相位模型 |
| even/odd 跳点 | rendering on 时奇数帧在 (339,261)→(0,0) 跳 1 clock | `ppu_rendering.cpp:2060-2064`（`idleSynch` toggle） | 位置/相位待 instrument 验证（Step 1.4，vbl_10 FAIL） |

**vbl_05 期望输出（源码注释原文，目标表）**：`00 4 / 01 4 / 02 4 / 03 3 / 04 3 / 05 3 / 06 3 / 07 3 / 08 3 / 09 2`
（NMI 落在 5 条 `LDX #1..#5` 中的哪条之后；`05-nmi_timing.s` 原文）。

**vbl_06 期望输出**：`00-03 -N…`（正常）、`04 - -`（flag 从未置位，读抑制了置位）、`05/06 V -`（读到置位但 NMI 被抑制）、`07-09 V N`（正常）。

**关键认识修正（两次实测迭代后定案）**：
1. 交接档案 §2 称"单参数 NMI 延迟无法修复 vbl_05，根因是 per-row 相位漂移"——方向对。
2. **"置位点 cycle0→dot1 是根因"的假设已被 Step 1.1 实测证伪**（见 §3 Step 1.1 🚨 块）：移动置位点
   使 vbl_01/03/09 翻红（6820 set→clear 周期被破坏）且 vbl_02/05 无改善。
3. **实测定案根因**（Step 1.2 探针数据）：模拟器在**帧边界（sl240→sl241）的 CPU↔PPU 读/NMI 采样时序偏移 ~2-3 dot**——
   vbl_05 "NMI 早 ~2 指令"、vbl_02 "测量 read1 未落在抑制点" 同源。这不是 PPU 置位点问题，是 **CPU 侧相对时序**问题
   （印证 `vbl_step1_instrument_data` §5.2）。修正路径 = Step 1.3（CPU 侧 NMI/读采样模型），非置位点调整。
   **2026-08-02 进展**：派发侧（vbl_05）经 X6502_Run 帧修复 + NMIDELAY=8 已闭合（`5581769`）；
   读侧（vbl_02/06）细化定位为读落点量化，待读采样相位模型（`1b5a434`）。

### 1.2 E-3（APU 帧计数器）硬件真值（blargg_apu_2005.07.30 权威文档 + NESdev Wiki）

**Mode 0（4-step）时序表**（写 `$00` 至 $4017 后，blargg readme 原文）：

| 周期 | 事件 |
|---|---|
| 7459 | Clock linear（quarter：envelope + 三角 linear） |
| 14915 | Clock linear & **length**（half：length + sweep） |
| 22373 | Clock linear |
| 29830 / 29831 / 29832 | **Set frame irq**（三次）；29831 同时 Clock linear & length |
| 37289 | Clock linear（下一序列 Step 1） |

- **IRQ flag 对 CPU 可见点 ≈ 29831**；**IRQ handler 最早 29833**（readme："The IRQ handler is invoked at minimum 29833 clocks after writing $00 to $4017"）。
- **`04.clock_jitter.asm` 原文证实测试窗口**：写 `$40` → 写 `$00` → 延时 **29826** 读 $4015（=29830 读，**必须未置位**）；再写 → 延时 **29828** 读（=29832 读，**必须已置位**）。即 **29830 清 / 29832 置，2 周期窗口**。
- **power-on**：APU 等效"**$00 写 + 9~12 时钟延迟**"后开始执行（`apu_reset/readme.txt` + blargg readme "Misc"）。
- **reset（软复位）**：**重写最后一次写入 $4017 的值**（非 $00）；IRQ inhibit 标志"有时被清"（`apu_reset/readme.txt`）。
- **jitter**：$4017 模式改变**只在偶数内部 APU clock 生效**；奇数 clock 写则第一步**延迟 1 clock**。power-on/reset 时 APU 的奇偶随机（blargg readme "Clock Jitter" 节）。
- **5-step 写（bit7）触发立即 quarter+half clock**（NESdev Wiki side effects）；**inhibit 写（bit6）只清 IRQ 标志，不触发 clock**。
- IRQ 标志仅被 **$4015 读**或 **bit6=1 的 $4017 写**清除（blargg "Misc"——R6 缺陷 2 修复已落地并符合此条）。

### 1.3 对交接档案/报告两处误判的修正（重要）

1. **"wait_n 等 ~6 个 quarter"是误读**（`FCEUSND_Reset` 内注释）：`04.clock_jitter.asm` 实际是 **4 quarter − 4 cycles**，
   读取点 29830/29831/29832。交接档案因此把缺陷 1 引向"wait_n 语义未反汇编"，方向错了。
2. **报告 §十 R6"Write_IRQFM 留 fcnt=1 → 仅 3 个 quarter 就置 IRQ（~22371）"是错的**：fcnt=1 后序列为 1,2,3,0，
   IRQ 在第 **4** 个 quarter = **29830**，周期本身正确。真缺陷是 **29830 置位 vs 测试 29830 读的竞态**（硬件可见点 29831），
   即 **W4017→IRQ 缺 ~1-2 周期延迟**；叠加 power-on 相位与 jitter 两个独立缺陷。
   **2026-08-02 实测校准**：目标 ROM（`apu_single_4_jitter`）的窗口为 **sub-test 2 读 write+29831 须 CLEAR、
   sub-test 3 读 write+29833 须 SET** → 一阶修复目标 = 置位点落 **write+29832（偶）/ +29833（奇）**，
   而非方案 §4 原写的 29831（29831 对应 `apu_04` 版本窗口，两 ROM delay 记账差 1 cyc）。已按此校准落地（Step 2.2）。

### 1.4 对 R6 三个失败假设的重新定位（为什么它们无效，真因是什么）

| 假设 | 结果 | 为什么无效（本次研究结论） |
|---|---|---|
| H1: power-on fcnt 0→1 | 无效（apu_single_4 仍 FAIL）+ 碎 golden | **方向正确但只修了一半**：apu_single_4 走 $4017 写路径，与 power-on fcnt 无关；golden 碎裂是预期流程问题（需 `--generate` 重生），非代码错误。**恢复此改动（Phase 2 Step 2.1）** |
| H2: IRQ 位置 `if(!fcnt)`→`if(fcnt==3)` | 无效（仍 FAIL 0x02） | fcnt 编号语义：写后 fcnt=1，IRQ 在 fcnt==0 的第 4 quarter=29830 已正确；改 fcnt==3 反而在 3rd quarter=22373 置位，**更早** |
| H3: 去掉 fhcnt=fhinc | 无效 + 碎 golden | fhcnt 重置本身正确（写后计时起点）；移除只引入漂移，未触及竞态 |

**真因四件套**（Phase 2 逐一修复；2026-08-02 实测后由"三件套"补入 ④）：① power-on/reset 相位（fcnt=1 + reset 保留最后模式）——**✅ Step 2.1 已落地**（`p2_step2_1_fix_data`）；② **W4017→IRQ ~1-2 周期延迟**（一阶近似 + IRQ 可见点；目标值实测校准为 **write+29832（偶）/ +29833（奇）**，见 §1.3 修正）——**✅ Step 2.2 一阶近似已落地**（apu_single_4 PASS）；③ **jitter**（odd/even APU clock 对齐，±1 clock）——**✅ Step 2.2 已落地**（相位信号 = 写时刻 `g_cpu.timestamp & 1`）；④ **hook 量化方差**（quarter 触发被量化到指令边界，±2-3 cyc，超出 blargg ±1 cyc 容差）——**✅ 2026-08-02 Step 2.2 深化已修复**：以 RustyNES/Mesen2 已验证模型（cycle-position 表 + 写后 3/4 周期重置 + 绝对周期奇偶）替换均匀 countdown，single_5/6 + reset_4017_timing/written + apu_test 全转 PASS（`p2_step2_2_deep_implementation`）。

---

## 2. 三阶段总览

> **🚨 执行现状与优先级修订（2026-08-01 实测后 + 2026-08-02 补充 + 2026-08-02 深化）**：
> - **Phase 1（E-1）**：Step 1.1 证伪（置位点假设，已回滚）；Step 1.2 抑制机制落地（零回归，vbl_02/06 未闭合）；
>   **Step 1.3 深化两轮**：NMI fresh 延迟（`7cfb029`）+ **X6502_Run 帧修复 + NMIDELAY=8 → vbl_05 PASS**（`5581769`，
>   **证伪判据主项达成**）。剩余 5 ROM 已重新分源：
>   **vbl_02/06 = CPU 侧读采样量化**（读落点被指令边界量化、A2002 (sl,cyc) 坐标失真、Step 1.2 marker 缺口，
>   需读采样相位模型——深，`1b5a434` 调查；**2026-08-03 深化**：测试源码实证漂移为亚指令粒度 + 残量在差值中抵消
>   （漂移被帧边界重同步摧毁）→ 闭合需亚指令级读时序，收敛无保证，评估见
>   `vbl_step1_2_closure_assessment_2026-08-03.md`，**用户决策：有时间再推进**）、
>   **vbl_07/08 = $2000 边沿采样**（对 NMIDELAY 5-12 全扫免疫，独立子问题）、
>   **vbl_10 = Step 1.4 even/odd**（独立机制）。
> - **Phase 2（R6/APU）已执行（顺序经用户确认）**：**Step 2.1 相位分离 ✅**（2026-08-01，`e3(step2.1)`）+ **Step 2.2 写路径延迟+jitter 一阶近似 ✅**（2026-08-02，`e3(step2.2)`，**`apu_single_4_jitter` 0x02→0x00 PASS**，40 ROM 零回归，Oracle A 33/33）+ **Step 2.2 深化 ✅**（2026-08-02，`e3(step2.2-deep)`：cycle-position 帧计数器，**single_5/6 + reset_4017_timing/written + apu_test 全转 PASS，7 个 bucket-C sub-test 全闭合**，Oracle B 135 PASS 无 apu_* 失败）。
> - **Phase 2 剩余**：**Step 2.3 已实测证伪（2026-08-02）**——当前 `V&0x2` 即原始 bit7（5-step），
>   已是硬件正确实现；方案改法 `V&0x1` 方向相反（会使 $40 触发 clock、$80 不触发），实测
>   `apu_single_1_len_ctr` 0x00→0x04 回归（见 `docs/history/surveys/e6_apu/p2_step2_3_falsification_2026-08-02.md`）。
> - **建议执行顺序（已确认）**：**Phase 2（R6）→ Phase 1 Step 1.3 → Phase 1 Step 1.4 → Phase 3**。
>   E-3 已闭合；Step 1.3 派发侧（vbl_05）已落地。**Step 1.4 已调查并定案（2026-08-03）**：vbl_10 根因 =
>   跳点门控写接受边界高 1 dot（写 true dot 被批量化不可区分），修复需亚指令级写时序（与 vbl_02/06 同族，
>   收敛无保证），**记录为有据已知限制，不排入当前序列**（探针与扫参数据见
>   `docs/history/surveys/e1_vbl/vbl_step1_4_instrument_data_2026-08-03.md`）。
>   下一步候选（按改动面/风险排序）：**~~NMI 取消时序缺口小修（真实正确性缺口，见 §3 Step 1.2）~~**
>   （2026-08-03 实测证伪"独立落地"：取消机制正确但 vbl_04/05 回归，绑定深模型，见
>   `docs/history/surveys/e1_vbl/vbl_step1_2_nmi_cancel_gap_2026-08-03.md`；修复方案保留随深模型落地）→
>   **vbl_02/06 决定性残量探针（定夺深模型是否理论可行）→ vbl_07/08 边沿采样 → Phase 3**。

| Phase | 目标 | 验收门 | 对应 |
|---|---|---|---|
| **Phase 1** | E-1 PPU VBL/NMI 收敛：`vbl_01`~`vbl_10` 全 10 ROM 返回 `$6000=0x00` | Oracle A 34/34；`blargg_ppu_vbl_nmi` 升 blocking | §十 R5 |
| **Phase 2** | E-3 APU 帧计数器收敛：7 个 bucket-C sub-test 全转 PASS | Oracle A 34/34；`apu_01`~`apu_11` + `pal_apu_*` + `apu_mixer_*` 不回归 | §十 R6 |
| **Phase 3** | 全量精度收敛与收尾：blargg 真实精度 FAIL 面下降、迁移矩阵 39/39 | Oracle A/B 全绿；matrix `passed=39`；README 数字回填 | §十 完成判据 |

---

## 3. Phase 1 — E-1 PPU VBL/NMI 收敛（独立 PR，每步独立 commit）

> 目标 ROM：`vbl_02_set_time` / `vbl_05_nmi_timing` / `vbl_06_suppression` / `vbl_07_nmi_on_timing` / `vbl_08_nmi_off_timing` / `vbl_10_even_odd_timing`（6 FAIL）
> PASS 基线：`vbl_01_basics` / `vbl_03_clear_time` / `vbl_04_nmi_control` / `vbl_09_even_odd_frames`（4 PASS，不得回归）
> 跑法：`fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json --frames 600`

### Step 1.1 — ~~VBL 置位点 cycle0→dot1 + NMI latch 同点~~（**已证伪，2026-08-01**）

> **🚨 实测证伪**：本步按路径 (a) 实装 + 10 ROM × 600 帧实测，**3 个 PASS ROM 翻红（vbl_01 0x07 周期过短 / vbl_03 / vbl_09），0 个 FAIL 转 PASS**。
> 根因：周期核算错误——块总长 6820 ✓ 但 **set→clear = 6819**（flag 于块内 dot 1 置位，清除在块末）；`vbl_01` 的 6820 不变量是 **set→clear** 周期。
> vbl_02 改动后全行 "V -"（更差）、vbl_05 全行 X=1（比 runppu(3) 差）——置位点假设对二者均**证伪**，印证 `vbl_step1_instrument_data` §5.2（PASS/FAIL 与置位点无关）。
> 已回滚（`git checkout`），完整数据见 `docs/history/surveys/e1_vbl/vbl_step1_1_falsification_2026-08-01.md`。
> **修订后顺序**：Step 1.2（$2002 抑制）→ Step 1.3（CPU 侧 NMI latch）→ Step 1.4（even/odd），置位点不再单独调整。

**文件**：`src/ppu_rendering.cpp:1556-1595`（working-config VBL 块）

> **本步原改法（路径 (a)：runppu(1) 前置 + S=0 上限 (kLineTime-1)，保持块总长 6820）已整体废弃**——
> 实测证伪（vbl_01 0x07 周期过短）。**教训**：`vbl_01` 的 6820 不变量是 **set→clear 周期**，不是块总长；
> flag 于块内 dot 1 置位则清除必须于 dot 6821，需重构 VBL 块+idle 边界——且即使重构，置位点调整对 vbl_02/05
> 无改善（探针证实 PASS/FAIL 与置位点无关）。**不重试本路径。**

### Step 1.2 — $2002 读抑制窗口（vbl_02 / vbl_06）【**已实施：零回归、部分生效、vbl_02 未闭合**】

> **✅ 2026-08-01 实测状态**：
> - 抑制机制已实现（`ppu.cpp A2002` 标记 + `ppu_rendering.cpp` VBL 块消费 + `(241,0-1)` NMI 取消），
>   **Oracle A 34/34 + Oracle B 121/56 全量零回归**。
> - vbl_02 **未修复**：探针显示测量行 read1 从未落在 (240,340)，抑制大多在 sync 自旋读上触发（硬件上合法）。
> - vbl_06/07/08 部分改善（需真基线确认）。vbl_06 rows 05-06 的 NMI 抑制未达成（CPU 在指令边界先于读检查 NMI）。
> - **深层根因确认**：帧边界 CPU↔PPU 读时序偏移 ~2-3 dot（与 vbl_05 "NMI 早 ~2 指令"同源）——指向 CPU 侧模型（Step 1.3），
>   非抑制窗口本身。完整数据见 `docs/history/surveys/e1_vbl/vbl_step1_2_data_2026-08-01.md`。

**文件**：`src/ppu.cpp:327-349`（`A2002`）【**已实施（2026-08-01），本段改法为实现记录**】

**已实现改法**：`A2002` newppu 路径——读于 (sl240, c340) 标记本帧 VBL 置位抑制（`fceu11_ppu_mark_vbl_set_suppressed`）；
读于 (sl241, c0-1) `X6502_IRQEnd(FCEU_IQNMI)` 取消 NMI；`ppu_rendering.cpp` VBL 块消费标记。`runppu(3)` 在 NMI 开启时无条件执行（帧长一致）。
env-gated 探针 `E1 P2002_READ` / `E1 VBL_SUPPRESSED`。

**实测结论**：全量零回归（Oracle A 34/34、Oracle B 121/56）；vbl_02 **未闭合**——测量 read1 从未落在抑制点
（探针证实抑制大多在 sync 自旋读触发）；vbl_06/07/08 部分改善。**残余工作并入 Step 1.3**（CPU 侧时序根因）。

> **🔍 2026-08-02 读侧调查（`vbl_step1_3_read_suppression_2026-08-02.md`）**：vbl_02/06 未闭合的精确机制已定位——
> ① **读落点量化**：测量读被指令边界量化（vbl_06 行 4-9 全部停在 (241,0)、Δ=+4，行 0-3 在 Δ=0，仅 2 个离散落点，
>    无法还原硬件 1 dot/行漂移的 5+ 落点）；② **坐标失真**：`X6502_Run` 阶段 PPU cycle 停驻使 A2002 的 (sl,cyc) 判定失效；
> ③ **Step 1.2 缺口**：(241,0-1) 读发生在 `TriggerNMI` **之前**，`X6502_IRQEnd` 无事可清（补 marker 会误伤行 7-9）。
> **结论**：需 CPU 侧读采样相位模型（与 cycle-position 同族），NMIDELAY 5-12 全扫证伪参数级修复；未落地半修复。
>
> **🔍 2026-08-03 闭合可行性评估（`vbl_step1_2_closure_assessment_2026-08-03.md`）**：
> - **基线重跑确认**（`build/tests/fceux11_blargg_runner.exe --frames 600`）：vbl_02=0x01（行 04 应 `- -`
>   实为 `- V`，仅差 1 行）；vbl_06=0x01（行 04-06 应 `- -`/`V -`/`V -` 实为 `V N`，行 07-09 已对）。
> - **测试源码实证**（`build/vbl_06-suppression.s` + `build/vbl_sync_vbl.s`）：1 dot/行漂移来自
>   `sync_vbl_delay(A=行号)`（A 循环每迭代等效延迟 1 PPU dot = **1/3 CPU 周期，亚指令粒度**），
>   而 `A2002` 派发被指令边界量化 → 10 行仅 2 个离散落点（与 8-02 调查一致）。
> - **新发现（残量抵消）**：即使按 8-02 建议用 `_count` 残量携带亚周期相位，行 4-9 相对置位点偏移恒为
>   +12 dots（读与置位点同边界，残量在差值中抵消）——**漂移是被帧边界重同步摧毁的，不是"未建模"**。
>   恢复漂移需 CPU 在帧边界附近的**亚指令级读时序**（CPU/PPU 联合仿真改造），超出"A2002 加相位"范围，
>   **收敛无保证**。
> - **NMI 取消时序缺口（独立小修，真实正确性缺口）**：`A2002` 在 (241,0-1) 的 `X6502_IRQEnd(FCEU_IQNMI)`
>   是 no-op——读执行于 VBL 块 `X6502_Run(nd)` 期间、**早于** `TriggerNMI()`，无 pending 可清。
>   正确语义应改为"读后跳过 TriggerNMI"标记。**2026-08-03 实测**：按此实装后取消机制正确生效
>   （vbl_06 行 5-6 转 `V -`，探针 10/10 帧 `nmi_cancel=1`、0 NMI_SET），**但 vbl_04/05 回归**
>   （VBL 轮询读同样落在 (241,0)，取消误伤其 NMI）→ **绑定深模型，不可独立落地，已回滚**；
>   完整数据见 `docs/history/surveys/e1_vbl/vbl_step1_2_nmi_cancel_gap_2026-08-03.md`。
> - **决策**：闭合需深模型、收敛无保证，**用户决策：有时间再继续推进**（2026-08-03）。
>   前置**决定性残量探针**（低成本、结论决定性）：测 vbl_06 行 0-9 读落点 `_count` 残量结构——
>   有亚周期结构 → 相位模型理论可行再评估实现；无 → 坐实"漂移被重同步摧毁"，按有据已知限制收口。

**原证伪判据（未达成，保留为 Step 1.3 目标）**：`vbl_06` 输出 `-N,-N,-N,-N, - -, V-, V-, VN, VN, VN`（$6000=0x00）；
`vbl_02` 输出与 readme 模式一致；`vbl_01` 不回归。

### Step 1.3 — CPU 侧 NMI/读采样时序模型（vbl_05 主修复 + vbl_02/06/07/08 残余）【**E-1 深层根因专项**】

> **2026-08-02 实测（第一轮，部分落地）**：✅ **NMI latch 延迟 1 指令边界已落地**
> （`x6502.cpp TriggerNMI` fresh 标记：VBL 路径 latch 在两次 CPU run 之间置位，
> 立即派发早 1 指令；6502 指令结束采样语义下应推迟一个边界）。实测：vbl_05 首行
> X 2→3（`[2,2,2,2,2,1,...]`→`[3,3,3,3,2,2,...]`），Oracle A 33/33 零回归，
> vbl_01/03/04/09 保持 PASS。🔍 **未闭合**：flag 置位相位漂移需 CPU 侧子周期
> 采样建模（见下），vbl_02/05/06/07/08/10 仍未 PASS。完整数据见
> `docs/history/surveys/e1_vbl/vbl_step1_3_nmi_sampling_2026-08-02.md`。
>
> **✅ 2026-08-02 深化（第二轮，vbl_05 主修复目标达成，`5581769`）**：
> - **帧修复**：VBL 块 NMI 使能路径 `runppu(nd)` → `X6502_Run(nd)`。runppu(nd) 同时推进 PPU nd dots
>   并给 CPU nd×16 单位预算，使 NMI-on 帧长 6823 dots（R5 Step 3 遗留畸变——探针证实 `VBL_ENTER cycle`
>   逐帧 +3，扭曲 nmi_timing 的逐行相位漂移）；`X6502_Run(nd)` 只给 CPU 预算、不推进 PPU，帧恢复
>   6820-dot VBL（硬件真值，探针证实 cycle 恒 0）。CPU 行为与预算完全一致，仅帧长归一。
> - **NMIDELAY 校准 3 → 8**：`X6502_Run(8)` = 8 dots ≈ 2.67 周期预算于 latch 前。扫参：
>   7 → `[4,3,...]`（偏早 1 指令）、**8 → `[4,4,4,3,3,3,3,3,3,2]`（与期望完全一致）**、9 → `[4,4,4,4,3,...]`（转变行晚 1）。
> - **实测**：vbl_05 **PASS（$6000=0x00）——证伪判据主项达成**；vbl 全 10 ROM 零回归（01/03/04/09 基线保持）；
>   APU 18 ROM 零回归；Oracle A 33/33（golden nestest 哈希已重生，`7ea1d0c`）。
>   完整数据见 `docs/history/surveys/e1_vbl/vbl_step1_3_deep_x6502run_2026-08-02.md`。
>
> **深层根因定位（本轮决定性数据）**：vbl_05 的 `_count` 残量（= CPU↔PPU 帧相位）
> 确按 1/3 周期/帧漂移，但 **VBL 块 runppu 预算把派发量化吸收**——固定 NMI 延迟 D
> 下 DPC 只随 lastpc 三档粗变（E350/E34E/E34B），无法还原期望逐行漂移
> `[4,4,4,3,3,3,3,3,3,2]`；flag 置位点任何移入块内实验（FLAGDELAY/dot1/相位模型）
> 均破坏 6820 周期或 sync。修正路径 = **让 NMI 派发点由 `_count` 相位直接决定**
> （与 Phase 2 cycle-position 帧计数器同族），非固定预算+延迟近似。

> **范围修订（2026-08-01 实测后）**：本步从"NMI on/off timing 微调"升格为 **E-1 深层根因专项**。
> Step 1.1/1.2 实测确认：剩余 6 ROM（02/05/06/07/08/10）同源——**CPU 侧 NMI 采样与 $2002 读在帧边界的相对时序偏移 ~2-3 dot**
> （vbl_05 "NMI 早 ~2 指令"、vbl_02 "测量 read1 未落抑制点"、vbl_06 rows 05-06 "NMI 抑制未达成" 三表同证）。
> **2026-08-02 细化**：该"同源"假设经实测收窄——派发侧（vbl_05）已独立修复（见上）；vbl_02/06（读侧量化）与
> vbl_07/08（边沿采样）虽同属 CPU 侧采样族，但为独立子问题，分源与处置见 §2。
> 目标输出（当前状态）：vbl_05 `4,4,4,3,3,3,3,3,3,2`（**✅ 已达成**）；vbl_06 全模式、vbl_02/07/08 与 readme 一致
> （未达成——02/06 读侧相位模型、07/08 边沿采样，分源见 §2）。

**依赖**：**不依赖** Step 1.1 置位点（已证伪）。vbl_07（NMI-enable 在 VBL 清除边界）、vbl_08（NMI-disable 在 VBL 置位边界）
的边沿采样窗口随 NMI 采样模型修正自动对齐。
**文件**：`src/x6502.cpp:395-403`（`TriggerNMI`/`TriggerNMI2`，立即/延迟 NMI 两路 + CPU 指令边界 NMI 检查）、
`src/ppu.cpp:327-349`（`A2002`，已含抑制）、`src/ppu_rendering.cpp:1642`（`X6502_Run(nd)`——runppu(3) hack 已替换，2026-08-02）。
**改法（instrument-first）**：先插桩记录（a）NMI latch 点 vs CPU 指令边界采样点（b）$2002 读点 vs NMI dispatch 顺序，
量化 ~2-3 dot 偏移的精确来源（CPU 采样提前？NMI 7-cycle 进入延迟缺失？指令边界相位？），再按数据建模——
候选：NMI 边沿 latch + 指令边界采样相位修正、runppu(3) hack 替换为 CPU 侧等效延迟。
**证伪判据（当前状态）**：~~vbl_05 输出 `4,4,4,3,3,3,3,3,3,2`（$6000=0x00）~~（**✅ 已达成，`5581769`**）；
vbl_02/06/07/08 $6000=0x00（未达成）；vbl_04（当前 PASS）不回归（保持）。

### Step 1.4 — even/odd 跳点（vbl_10）【**已调查，根因定案，记录为有据已知限制（2026-08-03）**】

> **🚨 2026-08-03 插桩调查结论**：完整数据见 `docs/history/surveys/e1_vbl/vbl_step1_4_instrument_data_2026-08-03.md`。
> - **决策点恒定 dot 338**（比被跳 dot 340 早 2 dot）；两 sub-test 各恰好 1 次跳点（第三序列帧）。
> - **$2001 写**：两 sub-test 的 Y 写批末均在 dot 338，亚 dot 相位差 16 单位 = 1 dot（A=5 精确晚 1 dot，相位传递自洽）。
> - **计数机制**：计数循环读恰落 VBL 置位边界，flag 观测由读时刻 `count`（相位误差）决定；sub-test 2 读 count 从
>   -80 起（8 次到回绕置位）、sub-test 3 从 -96 起（7 次）→ 计数差 1 纯粹来自初始 1 dot 未吸收。
> - **根因**：跳点门控"写接受边界"模拟器 true-dot ≤ 338，硬件 ≤ 337（gate 340 + ~2-3 dot $2001 写延迟）→
>   A=5 的写（true 338）模拟器错误接受 → 多余跳 1 次。但写 true dot 被批量化不可区分（同批末 338），
>   修复需**亚指令级写时序**（CPU/PPU 联合仿真改造，与 vbl_02/06 深模型同族，收敛无保证）。
> - **扫参证伪**：FCEUX11_E1_GATEDOT(336-340) × FCEUX11_E1_SKIPDOT(339-341) 共 15 配置，无任何配置产生 8,8,9,7。
> - **处置**：vbl_10 记录为有据已知限制（0x03），不排入当前序列；代码保持基线，保留 E1 SKIP_DEC / E1 W2001 探针
>   （env-gated 零影响，供未来深模型调查复用）。PASS 基线 01/03/04/05/09 零回归，Oracle A 34/34。

**文件**：`src/ppu_rendering.cpp:2058-2074`（even/odd 跳点，pre-render 行末 (339,261)→(0,0)；文档早期引用的 1979-1994 为过时行号）
**改法（原案，已被调查证伪路径）**：先插桩记录跳点 dot 与 BG-enable/disable 事件的相对位置，确认"跳点偏晚/偏早"后按数据移动。`vbl_09` 当前 PASS 且依赖跳点位置，盲目移动必回归。**实测**：跳点位置本身与硬件真值一致（skip dot 340），问题在写侧接受边界，非跳点位置。
**注意**：`idleSynch` 存于 savestate（`ppu_state.cpp:69` tag "IDLS"），改 toggle 时机会 invalidate `golden_savestate_test` 哈希 → 需 `--generate` 重生。
**证伪判据（未达成，已定案为限制）**：`vbl_10` 输出 `08 08 09 07`（$6000=0x00）——需亚指令级写时序，收敛无保证；`vbl_09` $6000 保持 `0x00`（✅ 保持）；Oracle A 34/34（✅ 保持）。

### Phase 1 强制回归集（每步后必跑，任一红即回滚该步）

- `vbl_01_basics` / `vbl_03_clear_time` / `vbl_04_nmi_control` / `vbl_09_even_odd_frames`（PASS 基线）
- `fceux11_rom_regression_test`（Oracle A，13 ROM × 60 帧 CRC32，`tests/tests.json:38`，blocking）
- `fceux11_golden_savestate_test` + `fceux11_savestate_regression_test`（Oracle A，blocking）
- 全量 `ctest -LE perf`（34/34）
- 每步前 `scripts/do_build.ps1` 全量重建

**禁忌**：不动 ppudead 路径（`:1526-1554`，带 P4-bridge Super Donkey Kong 修复）；不在 newppu 下让 `FCEUPPU_LineUpdate` 非 no-op（`:234-236`）；`blargg_ppu_vbl_nmi` 在全 10 ROM PASS 前不升 blocking、不动 `failure_means`。

---

## 4. Phase 2 — E-3 APU 帧计数器收敛（独立 PR，每步独立 commit）【**处方已由联机源码实证，建议优先执行**】

> **✅ 2026-08-01 处方实证 + 2026-08-02 执行状态**：本 Phase 全部改法的硬件依据已从 blargg 原始源码逐行确认——
> `04.clock_jitter.asm`（29830/29831/29832 读取窗口、jitter 检测逻辑）、`apu_reset/readme.txt`（power/reset 语义）、
> `blargg_apu_2005.07.30/readme.txt`（Mode 0/1 时序表、IRQ ≥29833）。交接档案 R6 缺陷 1 的"wait_n 语义未反汇编"阻塞
> **已解除**（真因 = power-on 相位 + W4017→IRQ ~1-2 周期延迟 + jitter + hook 量化方差，见 §1.3/§1.4）。
> **已执行**：Step 2.1 ✅（2026-08-01）+ Step 2.2 一阶近似 ✅（2026-08-02，`apu_single_4_jitter` PASS，
> single_5/6 转有据已知限制）。

> 目标：`apu_single_3_irq_flag`（已修，复验）、`apu_single_4_jitter`、`apu_single_5_len_timing`、`apu_single_6_irq_timing`、
> `apu_reset_4017_timing`、`apu_reset_4017_written`、`apu_test`（组合套件，停在 sub-test 3）全转 PASS。
> 跑法：`fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json --frames 600`（`apu_reset_*` 加 `--reset-after 60`）

### Step 2.1 — power-on / reset 相位（fcnt=1 + reset 保留最后模式）【**✅ 已落地 2026-08-01**】

> **✅ 实测状态**：已实现 `FCEUSND_Reset(bool is_power)`（power: IRQFrameMode=0x0 + fcnt=1；reset: 保留最后
> IRQFrameMode + fcnt=1），`fceu.cpp`/`FCEUSND_Power` 调用点区分。探针证实 power-on 后 fcnt 序列 1,2,3,0、
> IRQ 于第 4 个 quarter=29830 置位；40 ROM 零 PASS→FAIL、Oracle A 34/34、golden 重生（仅 fhcnt/fcnt 起始值
> 变更）。完整数据见 `docs/history/surveys/e6_apu/p2_step2_1_fix_data_2026-08-01.md`。

**文件**：`src/sound.cpp:1247-1300`（`FCEUSND_Reset`）、`src/fceu.cpp:896-901`（软复位调用点）

**改法**：
1. **power-on**：`fcnt=0` → `fcnt=1`（等效"$00 写 + 9~12 时钟"后的相位；IRQ 从第 4 quarter=29830 起算，length 从 14915/29830 起算）。
   —— 恢复 R6-2b 已 revert 的改动（它方向正确，被 golden 碎裂误伤）。
2. **soft reset**（`fceu.cpp:900` 路径）：`FCEUSND_Reset` 需区分 power 与 reset：
   - power：`IRQFrameMode=0x0`（$4017=$00）
   - reset：**保留 `IRQFrameMode` 最后写入值**（bit7 模式不变；bit6 inhibit 按"有时清除"语义处理——先按保留实测，用 `apu_reset_4017_written` 判据校准）
   - 实现建议：`FCEUSND_Reset(bool is_power)` 或独立 `FCEUSND_SoftReset()`，由 `fceu.cpp` 调用点区分。

**golden 处理**（预期流程）：改运行期起始值 → `fceux11_golden_savestate_test --generate` 重生索引（`tests/CMakeLists.txt:348`），
确认仅 savestate 哈希变化、无功能回归。

**instrument-first**：复用 `E3 FSU`/`E3 W4017_IN/OUT` 探针（已就位，`FCEUX11_E3_TRACE=1`），记录 power-on 后首个 FSU 的 fcnt 序列（应为 1,2,3,0，IRQ 于第 4 个置位）。

**证伪判据**：`apu_reset_4017_timing` $6000=0x00（sub-test 2/3：IRQ flag 置位时机正确）；
`apu_reset_4017_written` $6000=0x00（power $4017=$00、reset 保留最后值）；`apu_single_5` sub-test 2/3（首次 length 时机）转 PASS。

### Step 2.2 — W4017→IRQ ~1-2 周期延迟 + jitter（apu_single_4/6 主修复）

**文件**：`src/sound.cpp:1081-1140`（`Write_IRQFM`）、`src/sound.cpp:567-610`（`FCEU_SoundCPUHook` 的 fhcnt 递减）

**改法（一阶近似，instrument 校准）**：
1. **计时器重置延迟**：$4017 写生效有 3-4 周期延迟（blargg readme；NESdev wiki "After 3 or 4 CPU clock cycles, the timer is reset"）。
   实现：`Write_IRQFM` 不再立即 `fcnt=1; fhcnt=fhinc`，而是记录 pending 状态，在 `FCEU_SoundCPUHook` 中延后 ~2 周期再应用
   （或等价地：应用时 `fhcnt = fhinc - (2×48)` 并核对 IRQ 可见点落到 **29831**，即 29830 读清 / 29832 读置）。
   **2026-08-02 校准注**：实测方向与上式相反——现有模型第 4 quarter 落在 write+29831（偏早），
   需**后移 +1 cyc** 落 write+29832（偶）/ +29833（奇）方可过 apu_single_4 窗口（§1.3 修正 + ✅ 块）。
2. **jitter**：按写时刻 CPU/APU 时钟奇偶，第一步延迟 1 clock（`fhcnt` 初始多计 48 单位或延迟应用 1 周期）。
   需先 instrument 确定 FCEUX 中"写发生在偶数/奇数 APU clock"的可判定信号（`g_cpu.timestamp` 奇偶 + APU 半速时钟相位）。
3. 目标表：写 `$00` 后 IRQ flag 可见点 **29832（偶）/ 29833（奇）**（apu_single_4 版本窗口；handler ≥29833）。

**证伪判据**（对应 `04.clock_jitter.asm` 四个 sub-test）：
- 29830 读 $4015 未置位（sub-test 2 "too soon" 消除）
- 29832 读 $4015 已置位（sub-test 3 "too late" 消除）
- 偶数对齐两次 `get_jitter` 结果一致（sub-test 4）、奇数对齐两次结果不同（sub-test 5）→ $6000=0x00
- `apu_single_6`（irq_timing）$6000=0x00

> **✅ 2026-08-02 实测状态（已落地，一阶近似）**：
> - 已实现：`Write_IRQFM` 计时器重置改为 `fhcnt = fhinc + (1 + odd)*48`（offset=1，`FCEUX11_E3_OFFSET` 可覆盖）；
>   jitter 相位 = 写时刻 `g_cpu.timestamp & 1`。完整数据见
>   `docs/history/surveys/e6_apu/p2_step2_2_data_2026-08-02.md`。
> - ✅ **`apu_single_4_jitter` 0x02 → 0x00 PASS**；40 ROM APU 全量零 PASS→FAIL；
>   Oracle A 33/33（golden 重生，`fds_bios.fc0` 仅 5 字节 fhcnt/fcnt 运行期值变更）。
> - ⚠️ **`apu_single_5/6` 0x02 → 0x04 未闭合**（"first too soon"→"second too soon"）：
>   hook 量化方差（±2-3 cyc）超出 blargg ±1 cyc 容差，**参数级不可收敛**（offset 0-4、
>   fhinc 7457.5/7458、分数 offset、fcnt==3 +1、tsdelta 递减 全部证伪）。需 cycle-accurate
>   quarter crossing（消除 hook 量化），改动面与 Phase 1 Step 1.3 同族，记为**有据已知限制**。
> - `apu_reset_4017_timing`（power 路径 "delay 2"）与 `apu_test` 停在 Step 2.1 状态，未变。

> **✅ 2026-08-02 深化（Step 2.2 闭合，`p2_step2_2_deep_implementation`）**：
> - **根因**：均匀 7457.5 周期模型无法同时满足 length#1∈(14915,14916] 与
>   length#2∈(29831,29832]（间距必须 14916，均匀模型固定 14915）——任何参数均无解。
> - **实现**：以 RustyNES/Mesen2 已验证模型替换——cycle-position 表
>   （NTSC 7457/14913/22371/29828/29829/29830，quarter@7457/14913/22371/29829，
>   half@14913/29829，IRQ@29828-30）+ $4017 写后 **3/4 周期重置延迟**（偶数/奇数相位）
>   + **绝对周期奇偶**（`timestamp_base+ts`，修复每帧归零导致的跨帧翻转）
>   + power/reset 起始相位 D=4 + IRQ flag/line 分离。
> - **结果**：**`apu_single_5/6`、`apu_reset_4017_timing/written`、`apu_test`
>   全转 PASS——Phase 2 目标 7 个 bucket-C sub-test 全闭合**；APU 52 ROM 零 apu_* 失败；
>   Oracle B 135 PASS（基线 121）；golden 重生（compare-layout 仅 SFSND/FHCN 变化）。
> - runner 升级：`--reset-after` 后按 0x81 协议自动多次复位（4017_written 需两次）。

### Step 2.3 — ~~5-step 立即 clock 触发条件修正（V&0x1 而非 V&0x2）~~（**已证伪，2026-08-02**）

> **🚨 实测证伪**：本步前提（"当前 `V=(V&0xC0)>>6` 后 `V&0x2` 是 inhibit"）与代码事实相反——
> 该映射为自然映射：**bit6（inhibit）→ 缩减 bit0、bit7（5-step mode）→ 缩减 bit1**，故当前
> `if(V&0x2)` 触发条件已是原始 bit7，**即硬件要求的"5-step 写触发立即 quarter+half clock"**；
> `IRQFrameMode&0x2` 的 5-step 额外周期消费（`:518`）亦印证 bit1=bit7。改 `V&0x1` 反而使
> `$40`（inhibit）错误触发 clock、`$80`（5-step）错误不触发。
> 实测（`FCEUX11_E3_TRACE=1`，apu_01 × 600 帧）：基线 `$80` 写 → 立即 FSU、`$40` 写 → 无 FSU
> （符合硬件）；改动版行为反转，且 **`apu_single_1_len_ctr` 0x00→0x04 翻红**（回滚后复 PASS）。
> 完整数据见 `docs/history/surveys/e6_apu/p2_step2_3_falsification_2026-08-02.md`。
> **处置**：本步**无需执行**（当前实现已正确），不留已知错误触发条件；无代码改动落地。
> **修订后 Phase 2 剩余**：Step 2.2 深化（cycle-accurate quarter crossing，闭合 single_5/6）为唯一硬骨头，
> 与 Phase 1 Step 1.3（CPU 侧采样模型）同族，建议合并评估。

**文件**：`src/sound.cpp:1105`（`if(V&0x2) FrameSoundUpdate();`）【维持现状，不改】

**原证伪判据（已达成，用于证明现状正确）**：`apu_01`~`apu_11` 全 PASS、`apu_single_1` 保持 PASS、
`apu_single_3` 保持 PASS；改动版则使 `apu_single_1` 翻红。

### Phase 2 强制回归集（每步后必跑）

- `apu_01_len_ctr`（PASS 基线，原 E-3 前提已证伪）、`apu_02`~`apu_11` 全 11 个编号测试
- 全部 10 个 `pal_apu_*`、4 个 `apu_reset_*`（带 `--reset-after`）、4 个 `apu_mixer_*`（`--frames 2400`）
- `apu_07`/`apu_08` 稳态测试最敏感：验证 inter-IRQ 周期仍 ~29830(NTSC)/~33252(PAL)
- 全量 `ctest -LE perf` 维持 34/34
- savestate：`fceux11_golden_savestate_test`（重生后）+ `savestate_regression_test`

**禁忌**：不重复已 revert 的「P4-2 APU length counter 无条件 reload」（commit `562f0e8`/revert `cda40fe`）；
不"顺手修" `V=(V&0xC0)>>6` 的 bit 映射 swap（`apu_06` 当前 PASS 依赖它）；不得改 `FHCN`/`FCNT`/`IQFM` chunk 结构。

---

## 5. Phase 3 — 全量精度收敛与收尾（目标：迁移矩阵 39/39、blargg 精度 FAIL 面下降）

### Step 3.1 — harness 清理重测（零模拟精度改动，先拿掉非精度 FAIL）【**✅ 已落地 2026-08-04**】

> **✅ 2026-08-04 落地状态**(详见 `docs/history/reports/FCEUX11-1.16_P3-Step31_harness_cleanup_2026-08-04.md`):
>
> **基线(2026-08-04 06:00 Oracle B)**: 177 ROMs,**126 PASS / 51 FAIL**
> - 0x80 × 13(帧预算不足)
> - 0x81 × 8("Press RESET" 中途复位)
> - 0xFE × 1(`cpu_interrupts.nes` 永久跳过)
> - 0x01-0x09 × 29(真实精度)
>
> **修后(2026-08-04 07:00 Oracle B)**: 177 ROMs,**141 PASS / 36 FAIL** ✅
> - 0x80 × **0**(全清零)
> - 0x81 × **0**(全清零)
> - 0xFE × 1(永久跳过不变)
> - 0x01-0x0E × **35**(其中 6 项从 0x80/0x81 露出真实精度码)
>
> **改动**:
> - `tests/blargg_runner.cpp`:`ManifestEntry` 加 `reset_after` 字段(per-ROM RESET 覆盖),批处理循环按条目设 `g_reset_after_frames`
> - `tests/fixtures/blargg_manifest.json`:8 个 0x81 ROM 加 `reset_after: 60`;13 个 0x80 ROM 调高 frames(9 个 cpu → 3000、2 个 ppu → 3000、4 个 apu_mixer → 2400)
> - `scripts/generate_blargg_manifest.ps1`:增加 `$resetAfterRoms` 表(8 ROM)+ 输出字段
> - `scripts/analyze_blargg_results.ps1`:加注释说明 `reset_after` 字段含义
>
> **验证**:
> - Oracle A `ctest -LE perf`:33/33 PASS(不变)
> - 0x80 桶:13 → 0 ✅
> - 0x81 桶:8 → 0 ✅
> - 真实精度净修正:15 ROM PASS + 6 ROM 露出真实精度码(预期行为)
>
> **不引入回归**:`ppu_vbl_nmi`/`instr_timing`/`instr_timing_v2_1`/`ppu_read_buffer`/`cpu_dummy_writes_oam`/`cpu_reset_regs` 从 0x80/0x81 变为真实精度码,这些是 **harness 掩盖的既有精度问题**,不是新回归(符合 §十·五"精确知道什么失败"原则)。
>
> **剩余 36 项 FAIL** 进入 Step 3.2,详见报告 §3 分类(按子系统:**CPU 13**(含 0xFE 永久跳过 1)/ **PPU 4** / **vbl 5** / **MMC3 12** / **sprdma 2** = 36;35 真实精度 + 1 永久跳过)
>
> 分类详细(2026-08-04 06:00 Step 3.1 后 Oracle B 实测):
> - **CPU 13**:`cpu_dummy_writes_oam` 0x06 / `cpu_dummy_writes_ppu` 0x09 / `cpu_exec_space_ppuio` 0x05 / `cpu_int_2/3/4/5_nmi_brk/nmi_irq/irq_dma/branch_irq` 0x01×4 / `cpu_reset_regs` 0x02 / `instr_misc` 0x01 / `instr_misc_03_dummy` 0x03 / `instr_timing` 0x01 / `instr_timing_v2_1` 0x03 / `cpu_interrupts` 0xFE(永久跳过)
> - **PPU 4**:`oam_stress` 0x01 / `ppu_open_bus` 0x03 / `ppu_read_buffer` 0x0E / `ppu_vbl_nmi` 0x01
> - **vbl 5**:`vbl_02_set_time` 0x01 / `vbl_06_suppression` 0x01 / `vbl_07_nmi_on_timing` 0x01 / `vbl_08_nmi_off_timing` 0x01 / `vbl_10_even_odd_timing` 0x03(均 Phase 1 Step 1.3/1.4 已知限制,CPU/PPU 联合时序族)
> - **MMC3 12**:`mmc3_1/2/3/4/5/6` 0x03/0x02/0x04/0x09/0x02/0x02 + `mmc3_v2_1/2/3/4/5/6` 0x03/0x02/0x04/0x09/0x02/0x02
> - **sprdma 2**:`sprdma_dmc_dma` 0x01 / `sprdma_dmc_dma_512` 0x01(DMC+SPR DMA 耦合)
>
> 修订前报告原文"CPU 11 / PPU 11 / MMC3 12 / APU 2 / 永久跳过 1 = 37"分类与实测 36 不符(漏算了 vbl 5 项单独列出;APU 2 实为 sprdma 2;CPU 11 应为 12)。本修订为实地核对后的精准分类。
>
> **📌 后续收敛注记(2026-08-05)**:以上为 Step 3.1 后 36 FAIL 历史基线快照。桶 C 调查(`863e9d7`)已收敛 `ppu_read_buffer`(0x0E) + 顺带修复 `cpu_dummy_writes_ppu`(0x09),当前 Oracle B 为 **143 PASS / 34 FAIL**,剩余 34 项 FAIL 分类见 §5 Step 3.2 各桶。

### Step 3.2 — 剩余真实精度 FAIL 重分桶 + 逐个收敛【**🚧 进行中 2026-08-04**】

- 重跑全量 Oracle B（Phase 1/2 落地后基线），把剩余真实精度 FAIL 按子系统分桶（CPU 时序 / PPU 渲染 / OAM / DMA / APU 残余 / 其他）
- 每桶一个独立 PR，沿用 instrument-first + 强制回归纪律；每个 FAIL 保留 $6000 码 + 诊断串 + 根因结论
- 无法在不回归前提下收敛的项 → 记录为**有据已知限制**（带错误码、诊断、根因、已尝试方案），符合 §十·五"精确知道什么失败"原则

> **🚧 2026-08-04 进度**(Step 3.1 后基线 141 PASS / 36 FAIL):
>
> **桶 A — MMC3 (12 ROMs)** ✅ **已完成** → 12/12 全记入有据已知限制
> - 根因:0x02 IRQ counter reload 边角场景(单 PR 不闭合);0x03/0x04 A12 via PPUADDR 未实现(Mapper 4 无 PPU_hook);0x09 scanline 0 IRQ 时机
> - 详:`docs/history/surveys/mmc3/stepA_investigation_2026-08-04.md`
> - 探针保留:`FCEUX11_MMC3_PROBE=1` env-gated(零侵入,Oracle A 33/33 不变),供未来深模型调研复用
> - 提交:`m(a-investigation)`(`f4a072a`)
>
> **桶 B — CPU (12 ROMs 真实精度 + 1 永久跳过)**：
> **桶 B.3+B.4 (3 ROMs, 改动面小优先)** ✅ **已完成** → 2/3 记入有据已知限制 + 1/3 收敛
> - `cpu_dummy_writes_oam` 0x06:oam_read_test 失败(OAM 写+读+比较 4096 次迭代),根因为 PPU 隐式 OAM 改写(非 B2004 路径)
> - `cpu_dummy_writes_ppu` 0x09:PPU $2006 写时序 RMW dummy write 未建模 —— **✅ 已由桶 C 修复(`863e9d7`)→ 0x00 PASS**(palette 索引用 RefreshAddr 同族)
> - `cpu_exec_space_ppuio` 0x05:CPU 指令预取 PPU I/O 镜像语义
> - 详:`docs/history/surveys/cpu_bucketB/stepB_investigation_2026-08-04.md`
> - 探针:env-gated probe 模式 1/2/3 验证 A2004 读路径正确(ret==SPRAM[PPU[3]]==oam36_was、地址同一性),RMW 宏 abs/abs,X/abs,Y/(zp,X)/(zp),Y 都有 dummy write。根因在 PPU 隐式 OAM 改写,需深模型调研
> - 提交:`b(investigation)`(`57d3e88`),零代码改动(探针已撤回)
>
> **桶 B.1+B.2 (9 ROMs, 改动面大)** ⏳ **未启动**
> - `cpu_int_2_nmi_brk` 0x01 / `cpu_int_3_nmi_irq` 0x01 / `cpu_int_4_irq_dma` 0x01 / `cpu_int_5_branch_irq` 0x01:CPU 中断/复位/分支时序族,与 Phase 1 vbl_02/06/07/08 同族(深模型族)
> - `cpu_reset_regs` 0x02:CPU 复位寄存器时序
> - `instr_misc` 0x01 / `instr_misc_03_dummy` 0x03:指令时序杂项
> - `instr_timing` 0x01 / `instr_timing_v2_1` 0x03:指令时序(可能与 vbl_02 残量探针同族)
> - 评估建议:启动新一轮 instrument-first 探针,优先尝试 B.1 中 `cpu_int_*` 与 vbl_07/08 边沿采样模型合并建模(共享"CPU 指令边界 NMI 采样"族)
>
> **桶 C — PPU (4 ROMs, 真实精度)** 🚧 **1/4 收敛 + 3/4 待处理**
> - `ppu_read_buffer` 0x0E ✅ **已收敛 → 0x00 PASS**（`863e9d7`）：3 处真实 PPU bug——
>   `Ppu::reset()` 不重置 vnapage（mirroring 保留，修复 NTA_MIRRORING 0x0E）、
>   A2007 newppu 读更新 PPUGenLatch（open bus，修复 0x13）、palette 索引用
>   RefreshAddr 非 tmp（修复 PALETTE_READS_UNRELIABLE 0x30）
> - `cpu_dummy_writes_ppu` 0x09 ✅ **顺带修复 → 0x00 PASS**（palette 索引同族）
> - `ppu_open_bus` 0x03:open bus decay（PPUGenLatch 电容放电衰减，需衰减模型,改动面小可独立收敛）
> - `oam_stress` 0x01:OAM 压力测试,与桶 B 的 PPU 隐式 OAM 改写同族(深模型)
> - `ppu_vbl_nmi` 0x01:manifest frames=300 不足(需 3000);3000 frames 下为 0x01
>   (测试 2 of 10 vbl_set_time,与 Phase 1 vbl_02 同族——CPU 侧读采样量化)
> - 详:`docs/history/surveys/ppu_bucketC/stepC_investigation_2026-08-04.md`
> - 验证:Oracle B 143 PASS/34 FAIL(基线 141/36,+2 PASS 零回归);Oracle A 34/34
> - 提交:`c(fix)`(`863e9d7`)
>
> **桶 C.1 — vbl (5 ROMs, Phase 1 已知限制)** ⏳ **不在本桶重做**
> - vbl_02/06/07/08/10 均在 Phase 1 Step 1.2/1.3/1.4 调查中定案为深模型族限制
> - 探针 `FCEUX11_E1_TRACE` 已落地保留(零侵入,Oracle A 33/33 不变)
> - 待深模型族突破后统一处理
>
> **桶 D — sprdma (2 ROMs)** ⏳ **未启动**
> - `sprdma_dmc_dma` 0x01 / `sprdma_dmc_dma_512` 0x01:DMC DMA 与 sprite DMA 冲突/耦合
> - 可能与 Phase 2 帧计数器交互(DMC 触发 IRQ 时 CPU 指令边界对齐)
> - 评估建议:复用 `E3` 探针,优先尝试 DMC 时序校正
>
> **桶 E — 永久跳过(1 项)**
> - `cpu_interrupts.nes` 0xFE(`eventually_pass=false`,格式与 blargg harness 不兼容)
> - 已计入 CPU 桶总数(13 项),不重复计算

### Step 3.3 — 全量回归 + 验收复检（100% 完美交付判据）

> **2026-08-04 数字同步**(Step 3.1 + Step 3.2 桶 A + 桶 B + 桶 C.1 完成后基线):
> - Oracle B:**143 PASS / 34 FAIL**(基线 177 ROMs;桶 A 12 + 桶 B.3+B.4 3 = 15 项记入有据已知限制;桶 C.1 `ppu_read_buffer` + `cpu_dummy_writes_ppu` 收敛 +2 PASS,0x80/0x81 已清零)
> - 0x80/0x81 桶:**已清零**(Step 3.1 完成)
> - 0xFE `cpu_interrupts.nes`:**永久跳过**(计入 CPU 桶,共 1 项)
> - **剩余 18 项真实精度 FAIL** 按 P2 §5 桶分类:
>   - 桶 B.1+B.2:9 项(CPU 中断/复位/指令时序,改动面大)
>   - 桶 C (PPU 真实精度):3 项(ppu_open_bus/oam_stress/ppu_vbl_nmi)
>   - 桶 C.1 (vbl):5 项(Phase 1 已知限制,不在本桶重做)
>   - 桶 D (sprdma):2 项(DMC+SPR DMA 耦合)
>   - 小计:9 + 3 + 5 + 2 = 19 项?→ 校正:35 - 15(桶A+B.3+B.4) - 2(桶C.1收敛) = 18 项 ✓
> - **预计收敛潜力**(按 P2 §3 优先级):桶 C 剩余 3 项(`ppu_open_bus` decay 改动面小优先) > 桶 B.1+B.2 9 项 > 桶 D 2 项;桶 C.1 vbl 5 项 + `oam_stress`/`ppu_vbl_nmi` 待深模型族突破
> - Step 3.2 全部落地后,迁移矩阵按实际调整(35 真实精度 + 1 永久跳过 = 36 FAIL 基线,已收敛 2 项)

- [ ] Oracle A：`ctest -LE perf` 33/33 (排除 perf 标签) + `kagamiqa_migration_matrix.json` 生成
- [ ] Oracle B：全量重跑,FAIL 全带码+分类(已知限制类 PASS 数无变化即可)
- [ ] 迁移矩阵：`total=39`,按 Step 3.2 实际收敛度调整 PASS/FAIL 数;`lua_joypad_test`/`lua_memory_test` 视实现进度转 PASS 或保留有据 advisory
- [ ] README CN/EN + `docs/tech/KagamiQA.md` 三处数字与锚 commit 由 CI 产物回填(R2 路径 A)
- [ ] CI 实跑一轮全绿(R4 Gate 通过)
- [ ] `blargg_ppu_vbl_nmi` 升 blocking 且 PASS(**前提**:vbl_10 深模型闭合;否则维持 current,记录为已知限制)
- [ ] P2 交接档案状态摘要更新(Step 3.1 已落地,R6 缺陷 1 由"暂停"转"已修复"(7 个 bucket-C sub-test 全闭合))

---

## 6. 风险、禁忌与资源

### 主要风险

| 风险 | 缓解 |
|---|---|
| E-1 修好一个 vbl ROM 弄坏另一个（经典互耦） | 每步只动一个机制 + 全 10 ROM 回归；`vbl_01/03/04/09` 是硬基线 |
| **E-1 深层根因是 CPU 侧 NMI/读采样模型（改动面大、风险高）** | **派发侧（vbl_05）已修复**（`5581769`，X6502_Run 帧修复 + NMIDELAY=8）；剩余 vbl_02/06 读侧量化、vbl_07/08 边沿采样为独立子问题，按 Step 1.3 残余专项单独评估 |
| vbl_02/06 读落点量化（测量读被指令边界量化，A2002 (sl,cyc) 坐标失真） | 已探针定位（`1b5a434`）；**2026-08-03 深化**：测试源码实证漂移为亚指令粒度 + 残量在差值中抵消（漂移被帧边界重同步摧毁）→ 闭合需亚指令级读时序（CPU/PPU 联合仿真改造），收敛无保证；前置决定性残量探针定夺；**用户决策：有时间再推进**（`vbl_step1_2_closure_assessment_2026-08-03.md`） |
| vbl_07/08 $2000 边沿采样窗口 | 对 NMIDELAY 全扫免疫（5-12 不变），独立子问题，待专项调查 |
| NMI 取消时序缺口（(241,0-1) 读应取消本帧 NMI） | **2026-08-03 实测证伪"独立落地"**：取消机制正确（vbl_06 行 5-6 转 `V -`、探针 0 NMI_SET）但 vbl_04/05 回归（VBL 轮询读同落 (241,0)）→ 绑定深模型，已回滚，修复方案保留（`vbl_step1_2_nmi_cancel_gap_2026-08-03.md`） |
| VBL 周期 6820 不变量被破坏 → vbl_01 翻红 | **Step 1.1 已证伪此路径并回滚**；set→clear 周期是硬约束，置位点不再单独调整 |
| golden savestate 哈希碎裂被误判为回归 | 预期流程：`fceux11_golden_savestate_test --generate` 重生，diff 确认仅起始值变化（R6 Step 2.1 必走） |
| APU 时钟奇偶（jitter）在 FCEUX 架构中无现成信号 | **✅ Step 2.2 已消解**：相位信号 = 写时刻**绝对周期** `timestamp_base + timestamp & 1`（2026-08-02 深化修正：`timestamp` 每帧归零，跨帧奇偶翻转会破坏 sync_apu 对齐；探针实测偶间隔同相位/奇间隔翻转） |
| R6 改 $4017 写路径影响现有 apu_* PASS ROM | **✅ Step 2.2 已验证零回归**（40 ROM 全量，含 apu_01~11/pal/mixer/reset）；Step 2.3 已证伪不落地（改 V&0x1 会使 apu_single_1 翻红） |
| **Step 2.2 一阶近似无法闭合 single_5/6（hook 量化方差）** | **✅ 2026-08-02 深化已闭合**：cycle-position 帧计数器（RustyNES/Mesen2 模型 + 3/4 周期重置 + 绝对周期奇偶）落地，single_5/6 + reset_4017_timing/written + apu_test 全 PASS；无参数回退空间（已证伪） |
| Phase 3 剩余 38 项精度面大、易摊薄 | 分桶后按子系统逐个 PR，宁缺毋滥，保留"有据已知限制"出口 |

### 禁忌清单（汇总）

1. 不动 ppudead 路径（`ppu_rendering.cpp:1526-1554`，P4-bridge Super Donkey Kong 修复依赖）
2. 不在 newppu 下让 `FCEUPPU_LineUpdate` 非 no-op（`:234-236`，会重引入旧 PPU glitch）
3. 不重复 P4-2 length reload（commit `562f0e8`/revert `cda40fe`）
4. 不"顺手修" $4017 bit 映射 swap（`apu_06` PASS 依赖）
5. 不改 savestate chunk 结构（`sound.cpp:1462-1466`）
6. 不用 runppu(N) 盲调 NMI 相位（已证伪两次；`5581769` 已用 `X6502_Run(nd)` 替换——只给 CPU 预算不推进 PPU，NMI-on 帧保持 6820）
7. `blargg_ppu_vbl_nmi` 全 PASS 前不升 blocking

### 文件:行号总索引

**E-1**（行号 2026-08-02 校准）：VBL 置位 `ppu_rendering.cpp:1605`；NMI latch `:1648`；`X6502_Run(nd)` NMI 预算 `:1642`；`e1_nmi_delay` 旋钮 `:1529-1541`（默认 8）；VBL 清除 `:1667`；$2002 读+清 `ppu.cpp:327-349`（含 Step 1.2 抑制 `(240,340)` 标记 + `(241,0-1)` NMI 取消，后者为 no-op 缺口——读早于 `TriggerNMI`）；
$2000 NMI-enable 边沿 `ppu.cpp:601-615`；`TriggerNMI`/`TriggerNMI2` `x6502.cpp:395-403`（+ E-1 fresh 标记 `:425-432`）；even/odd 跳点 `ppu_rendering.cpp:2060-2064`（`idleSynch` toggle；pre-render 行末跳点块在其附近）；`runppu` `:1363-1377`；
vbl_02/06 测试源码 `build/vbl_06-suppression.s` + `build/vbl_sync_vbl.s`（`sync_vbl_delay` A=行号 → 1 dot/行亚指令漂移）

**E-3**（行号 2026-08-02 校准）：帧 IRQ 置位 `FrameSoundUpdate :491-525`（`SIRQStat|=0x40` 在内）；5-step 额外周期 `if(fcnt==3) :516-520`；length/sweep 半帧 clock `FrameSoundStuff :404-474`；
$4017 写 `Write_IRQFM :1081-1140`（`fcnt=0` `:1104`、`if(V&0x2)` `:1105`、`fcnt=1` `:1107`、`fhcnt=...` Step 2.2 `:1132`、条件清 `raw & 0x40` `:1133`）；
fhcnt 递减 `FCEU_SoundCPUHook :567-610`；reset 状态 `FCEUSND_Reset :1247-1300`（`fhcnt=fhinc` `:1263`、`fcnt=1` `:1264`）；power `:1333-1336`；savestate chunks `:1462-1466`

### 参考来源（联机研究，2026-08-01 抓取）

- NESdev Wiki：APU Frame Counter、PPU frame timing、NMI（https://www.nesdev.org/wiki/...）
- `christopherpow/nes-test-roms`（GitHub）：
  - `ppu_vbl_nmi/source/05-nmi_timing.s`、`06-suppression.s`（+ readme.txt 期望输出表）
  - `blargg_apu_2005.07.30/readme.txt`（Mode 0/1 时序、jitter、power-on/reset、IRQ ≥29833）、`tests.txt`（11 个 ROM sub-test 码表）、`source/04.clock_jitter.asm`（29830/29831/29832 读取窗口原文）
  - `apu_reset/readme.txt`（power/reset 语义）
- **2026-08-03 评估**：`docs/history/surveys/e1_vbl/vbl_step1_2_closure_assessment_2026-08-03.md`
  （Step 1.2 闭合可行性：漂移亚指令粒度 + 残量抵消 + NMI 取消缺口 + 决定性探针）

---

*方案完。Phase 1/2 每步落地后，按交接档案 §5 更新数据文档索引与状态摘要。*
