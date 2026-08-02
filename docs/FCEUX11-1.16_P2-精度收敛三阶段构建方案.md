# FCEUX11 v1.16 精度收敛 — 三阶段构建方案（P2 深化版）

> **编制日期**：2026-08-01
> **编制依据**：
> - `docs/history/reports/FCEUX11-1.16_最终验收报告.md` §十 R5/R6（原始处方 + 🚨/🚧 实测校准块）
> - `docs/tech/P2_precision_instrument_handoff.md`（E-1/E-3 交接档案，含 6 探针清单）
> - **本次联机研究**（2026-08-01）：NESdev Wiki（APU Frame Counter / PPU frame timing / NMI）+ blargg 原始测试包源码
>   （`christopherpow/nes-test-roms`：`ppu_vbl_nmi/source/*.s`、`blargg_apu_2005.07.30/source/*.asm` + readme/tests.txt、`apu_reset/readme.txt`）
> - **用户决策（2026-08-01）**：P3（第二 oracle 来源）暂不做，"先确保精度再谈别的"。P3 已在验收报告中标注暂缓。
>
> **性质**：可执行的构建方案。每个 Phase/Step 含 目标 / 文件:行号 / 改法 / instrument 验证 / 回归集 / 证伪判据。**不是**对交接档案结论的推翻，而是用联机资料把交接档案中"未闭合"的两项（R5-E1、R6-缺陷1）钉到可动手的精度。

---

## 0. 前置硬约束（沿用交接档案 §0，逐条强制，违反即回滚）

1. **instrument-first 是前置硬约束**：任何代码改动前，先加 env-gated probe 采集真实时序，用数据证实/证伪假设。
2. **每步强制回归**：Oracle A `ctest -LE perf` 34/34 + 相关 ROM 全量，任一红即回滚该步。
3. **savestate 兼容**：不得改 `FHCN`/`FCNT`/`IQFM` chunk 名/大小/序（`sound.cpp:1303-1307`）。改运行期起始值会碎
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
| VBL flag 置位 | **sl 241 dot 1**（`ppu_rendering.cpp` 注释亦自认 "cycle 1"） | `PPU_status \|= 0x80` 于 **cycle 0**（`ppu_rendering.cpp:1560`） | **早 1 dot** |
| NMI latch | flag 置位**同 dot**（/NMI 低电平 iff vblank_flag && NMI_output，CPU 下一指令边界采样） | `if (VBlankON) { runppu(3); TriggerNMI(); }`（`:1580`，R5 Step 3 保留的 hack） | runppu(3) 只移 PPU 相位，不修正 flag 置位点；且使 VBL 周期 +3 |
| VBL flag 清除 | **sl 261 dot 1**（置位后 6820 dots） | `PPU_status = 0`（`:1597`，20 扫描线循环后） | 清除点随置位点联动，需同步对齐 |
| $2002 读抑制窗口 | 读 **1 dot 前**于置位 → 抑制置位+无 NMI；**同 dot 或 1 dot 后** → 读到置位+清除+抑制 NMI；**≥2 dot** → 正常 | `A2002`（`ppu.cpp:327-349`）仅 `PPU_status &= 0x7F`，**无抑制逻辑** | 缺窗口逻辑 |
| even/odd 跳点 | rendering on 时奇数帧在 (339,261)→(0,0) 跳 1 clock | `ppu_rendering.cpp:1979-1994` | 位置/相位待 instrument 验证 |

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

### 1.4 对 R6 三个失败假设的重新定位（为什么它们无效，真因是什么）

| 假设 | 结果 | 为什么无效（本次研究结论） |
|---|---|---|
| H1: power-on fcnt 0→1 | 无效（apu_single_4 仍 FAIL）+ 碎 golden | **方向正确但只修了一半**：apu_single_4 走 $4017 写路径，与 power-on fcnt 无关；golden 碎裂是预期流程问题（需 `--generate` 重生），非代码错误。**恢复此改动（Phase 2 Step 2.1）** |
| H2: IRQ 位置 `if(!fcnt)`→`if(fcnt==3)` | 无效（仍 FAIL 0x02） | fcnt 编号语义：写后 fcnt=1，IRQ 在 fcnt==0 的第 4 quarter=29830 已正确；改 fcnt==3 反而在 3rd quarter=22373 置位，**更早** |
| H3: 去掉 fhcnt=fhinc | 无效 + 碎 golden | fhcnt 重置本身正确（写后计时起点）；移除只引入漂移，未触及竞态 |

**真因三件套**（Phase 2 逐一修复）：① power-on/reset 相位（fcnt=1 + reset 保留最后模式）；② **W4017→IRQ ~1-2 周期延迟**（含 3-4 周期计时器重置延迟的一阶近似 + IRQ 可见点 29831）；③ **jitter**（odd/even APU clock 对齐，±1 clock）。

---

## 2. 三阶段总览

> **🚨 执行现状与优先级修订（2026-08-01 实测后）**：
> - **Phase 1（E-1）已实测两轮**：Step 1.1 证伪（置位点假设，已回滚）；Step 1.2 抑制机制落地（**零回归**，但 vbl_02 未闭合）。
>   剩余 6 ROM 全部指向**同一深层根因：CPU 侧 NMI/读采样时序 vs PPU 帧边界对齐**（Step 1.3 专项，改动面在 x6502 采样模型，较深）。
> - **Phase 2（R6/APU）处方已由联机源码实证**（`04.clock_jitter.asm` 的 29830/29831/29832 窗口、power-on/reset 语义、
>   jitter 规则），根因明确、5 ROM 共享单根因，**单次收益高**。
> - **建议执行顺序修订**：**Phase 2（R6）→ Phase 1 Step 1.3 → Phase 1 Step 1.4 → Phase 3**（E-1 置位点/抑制的
>   剩余收尾并入 Step 1.3 专项）。是否按此顺序执行**待用户确认**（2026-08-01）。

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

**原证伪判据（未达成，保留为 Step 1.3 目标）**：`vbl_06` 输出 `-N,-N,-N,-N, - -, V-, V-, VN, VN, VN`（$6000=0x00）；
`vbl_02` 输出与 readme 模式一致；`vbl_01` 不回归。

### Step 1.3 — CPU 侧 NMI/读采样时序模型（vbl_05 主修复 + vbl_02/06/07/08 残余）【**E-1 深层根因专项**】

> **范围修订（2026-08-01 实测后）**：本步从"NMI on/off timing 微调"升格为 **E-1 深层根因专项**。
> Step 1.1/1.2 实测确认：剩余 6 ROM（02/05/06/07/08/10）同源——**CPU 侧 NMI 采样与 $2002 读在帧边界的相对时序偏移 ~2-3 dot**
> （vbl_05 "NMI 早 ~2 指令"、vbl_02 "测量 read1 未落抑制点"、vbl_06 rows 05-06 "NMI 抑制未达成" 三表同证）。
> 目标输出：vbl_05 `4,4,4,3,3,3,3,3,3,2`；vbl_06 全模式；vbl_02/07/08 与 readme 一致。

**依赖**：**不依赖** Step 1.1 置位点（已证伪）。vbl_07（NMI-enable 在 VBL 清除边界）、vbl_08（NMI-disable 在 VBL 置位边界）
的边沿采样窗口随 NMI 采样模型修正自动对齐。
**文件**：`src/x6502.cpp:395-403`（`TriggerNMI`/`TriggerNMI2`，立即/延迟 NMI 两路 + CPU 指令边界 NMI 检查）、
`src/ppu.cpp:327-349`（`A2002`，已含抑制）、`src/ppu_rendering.cpp:1580`（runppu(3) hack 的去留评估）。
**改法（instrument-first）**：先插桩记录（a）NMI latch 点 vs CPU 指令边界采样点（b）$2002 读点 vs NMI dispatch 顺序，
量化 ~2-3 dot 偏移的精确来源（CPU 采样提前？NMI 7-cycle 进入延迟缺失？指令边界相位？），再按数据建模——
候选：NMI 边沿 latch + 指令边界采样相位修正、runppu(3) hack 替换为 CPU 侧等效延迟。
**证伪判据**：vbl_05 输出 `4,4,4,3,3,3,3,3,3,2`（$6000=0x00）；vbl_02/06/07/08 $6000=0x00；vbl_04（当前 PASS）不回归。

### Step 1.4 — even/odd 跳点（vbl_10）

**文件**：`src/ppu_rendering.cpp:1979-1994`（even/odd 跳点，pre-render 行末 (339,261)→(0,0)）
**改法**：先插桩（env-gated，仿已 revert 的 `FCEUX11_E1_TRACE`）记录跳点 dot 与 BG-enable/disable 事件的相对位置，确认"跳点偏晚/偏早"后按数据移动。`vbl_09` 当前 PASS 且依赖跳点位置，**盲目移动必回归**。
**注意**：`idleSynch` 存于 savestate（`ppu_state.cpp:69` tag "IDLS"），改 toggle 时机会 invalidate `golden_savestate_test` 哈希 → 需 `--generate` 重生。
**证伪判据**：`vbl_10` 输出 `08 08 09 07`（$6000=0x00）；`vbl_09` $6000 保持 `0x00`；Oracle A 34/34。

### Phase 1 强制回归集（每步后必跑，任一红即回滚该步）

- `vbl_01_basics` / `vbl_03_clear_time` / `vbl_04_nmi_control` / `vbl_09_even_odd_frames`（PASS 基线）
- `fceux11_rom_regression_test`（Oracle A，13 ROM × 60 帧 CRC32，`tests/tests.json:38`，blocking）
- `fceux11_golden_savestate_test` + `fceux11_savestate_regression_test`（Oracle A，blocking）
- 全量 `ctest -LE perf`（34/34）
- 每步前 `scripts/do_build.ps1` 全量重建

**禁忌**：不动 ppudead 路径（`:1526-1554`，带 P4-bridge Super Donkey Kong 修复）；不在 newppu 下让 `FCEUPPU_LineUpdate` 非 no-op（`:234-236`）；`blargg_ppu_vbl_nmi` 在全 10 ROM PASS 前不升 blocking、不动 `failure_means`。

---

## 4. Phase 2 — E-3 APU 帧计数器收敛（独立 PR，每步独立 commit）【**处方已由联机源码实证，建议优先执行**】

> **✅ 2026-08-01 处方实证**：本 Phase 全部改法的硬件依据已从 blargg 原始源码逐行确认——
> `04.clock_jitter.asm`（29830/29831/29832 读取窗口、jitter 检测逻辑）、`apu_reset/readme.txt`（power/reset 语义）、
> `blargg_apu_2005.07.30/readme.txt`（Mode 0/1 时序表、IRQ ≥29833）。交接档案 R6 缺陷 1 的"wait_n 语义未反汇编"阻塞
> **已解除**（真因 = power-on 相位 + W4017→IRQ ~1-2 周期延迟 + jitter，见 §1.3/§1.4）。
> 与 Phase 1（E-1 CPU 侧深层模型）相比，本 Phase 根因明确、5 ROM 共享单根因，**建议先执行**（2026-08-01，待用户确认）。

> 目标：`apu_single_3_irq_flag`（已修，复验）、`apu_single_4_jitter`、`apu_single_5_len_timing`、`apu_single_6_irq_timing`、
> `apu_reset_4017_timing`、`apu_reset_4017_written`、`apu_test`（组合套件，停在 sub-test 3）全转 PASS。
> 跑法：`fceux11_blargg_runner.exe --manifest fixtures/blargg_manifest.json --frames 600`（`apu_reset_*` 加 `--reset-after 60`）

### Step 2.1 — power-on / reset 相位（fcnt=1 + reset 保留最后模式）

**文件**：`src/sound.cpp:1195-1209`（`FCEUSND_Reset`）、`src/fceu.cpp:896-901`（软复位调用点）

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

**文件**：`src/sound.cpp:1050-1090`（`Write_IRQFM`）、`src/sound.cpp:543-579`（`FCEU_SoundCPUHook` 的 fhcnt 递减）

**改法（一阶近似，instrument 校准）**：
1. **计时器重置延迟**：$4017 写生效有 3-4 周期延迟（blargg readme；NESdev wiki "After 3 or 4 CPU clock cycles, the timer is reset"）。
   实现：`Write_IRQFM` 不再立即 `fcnt=1; fhcnt=fhinc`，而是记录 pending 状态，在 `FCEU_SoundCPUHook` 中延后 ~2 周期再应用
   （或等价地：应用时 `fhcnt = fhinc - (2×48)` 并核对 IRQ 可见点落到 **29831**，即 29830 读清 / 29832 读置）。
2. **jitter**：按写时刻 CPU/APU 时钟奇偶，第一步延迟 1 clock（`fhcnt` 初始多计 48 单位或延迟应用 1 周期）。
   需先 instrument 确定 FCEUX 中"写发生在偶数/奇数 APU clock"的可判定信号（`g_cpu.timestamp` 奇偶 + APU 半速时钟相位）。
3. 目标表：写 `$00` 后 IRQ flag 可见点 **29831**（jitter 时 29832）；handler ≥29833。

**证伪判据**（对应 `04.clock_jitter.asm` 四个 sub-test）：
- 29830 读 $4015 未置位（sub-test 2 "too soon" 消除）
- 29832 读 $4015 已置位（sub-test 3 "too late" 消除）

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
- 偶数对齐两次 `get_jitter` 结果一致（sub-test 4）、奇数对齐两次结果不同（sub-test 5）→ $6000=0x00
- `apu_single_6`（irq_timing）$6000=0x00

### Step 2.3 — 5-step 立即 clock 触发条件修正（V&0x1 而非 V&0x2）

**文件**：`src/sound.cpp:1070`（`if(V&0x2) FrameSoundUpdate();`）

**改法**：硬件语义是 **bit7（5-step mode）写触发立即 quarter+half clock**（NESdev wiki side effects；blargg Mode 1 时序 step 0 @ cycle 1），
非 bit6（inhibit）。当前 `V=(V&0xC0)>>6` 后 `V&0x2` 是 inhibit——**改 `V&0x1`**。
注意：`$80` 写同时置 bit6+bit7，两条件均触发，`apu_01` sub-test 4（"Writing $80 should clock length immediately"）不受影响；
`$40` 写（仅 inhibit）将**不再**立即 clock——此为正确行为，需确认无现有用例依赖错误行为。

**证伪判据**：`apu_01`~`apu_11` 全 PASS（`apu_01` sub-test 4/5 尤其）；`pal_apu_*` 10 个 + `apu_mixer_*` 4 个不回归；
`apu_single_3` 保持 PASS（缺陷 2 修复不回退）。

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

### Step 3.1 — harness 清理重测（零模拟精度改动，先拿掉非精度 FAIL）

现状 56 FAIL = 18 harness + 38 真实精度（Stage-2 分桶）。其中：
- **12 项 `0x80`（帧预算不足）**：`--frames 600` → 按需提高（这些 ROM 需更多帧完成），重测
- **6 项 `0x81`（"Press RESET"，批处理未传 `--reset-after`）**：runner 批量路径补 `--reset-after`，重测
- 重测后重新分桶，确认真实精度 FAIL 面收敛到 38 以下（harness 修正不应改变精度项）

**证伪判据**：Oracle B 重跑后 0x80/0x81 类 FAIL 归零或显著下降；精度项清单逐条可核对。

### Step 3.2 — 剩余真实精度 FAIL 重分桶 + 逐个收敛

- 重跑全量 Oracle B（Phase 1/2 落地后基线），把剩余真实精度 FAIL 按子系统分桶（CPU 时序 / PPU 渲染 / OAM / DMA / APU 残余 / 其他）
- 每桶一个独立 PR，沿用 instrument-first + 强制回归纪律；每个 FAIL 保留 $6000 码 + 诊断串 + 根因结论
- 无法在不回归前提下收敛的项 → 记录为**有据已知限制**（带错误码、诊断、根因、已尝试方案），符合 §十·五"精确知道什么失败"原则

### Step 3.3 — 全量回归 + 验收复检（100% 完美交付判据）

- [ ] Oracle A：`ctest -LE perf` 34/34；cargo test 40/40
- [ ] Oracle B：全量重跑，PASS 数 ≥ 当前 121，FAIL 全带码+分类
- [ ] 迁移矩阵：`total=39, passed=39`（4 FAIL 清零；`lua_joypad_test`/`lua_memory_test` 视实现进度转 PASS 或保留有据 advisory）
- [ ] README CN/EN + `docs/tech/KagamiQA.md` 三处数字与锚 commit 由 CI 产物回填（R2 路径 A）
- [ ] CI 实跑一轮全绿（R4 Gate 通过）
- [ ] `blargg_ppu_vbl_nmi` 升 blocking 且 PASS
- [ ] P2 交接档案状态摘要更新（R6 缺陷 1 由"暂停"转"已修复"或"有据已知限制"）

---

## 6. 风险、禁忌与资源

### 主要风险

| 风险 | 缓解 |
|---|---|
| E-1 修好一个 vbl ROM 弄坏另一个（经典互耦） | 每步只动一个机制 + 全 10 ROM 回归；`vbl_01/03/04/09` 是硬基线 |
| **E-1 深层根因是 CPU 侧 NMI/读采样模型（改动面大、风险高）** | **已实测确认**（Step 1.1/1.2）；转为 Step 1.3 专项，单独评估；不因 E-1 阻塞 R6 |
| VBL 周期 6820 不变量被破坏 → vbl_01 翻红 | **Step 1.1 已证伪此路径并回滚**；set→clear 周期是硬约束，置位点不再单独调整 |
| golden savestate 哈希碎裂被误判为回归 | 预期流程：`fceux11_golden_savestate_test --generate` 重生，diff 确认仅起始值变化（R6 Step 2.1 必走） |
| APU 时钟奇偶（jitter）在 FCEUX 架构中无现成信号 | Step 2.2 instrument 先行，必要时在 hook 侧引入 APU 半速时钟相位跟踪 |
| R6 改 $4017 写路径影响现有 apu_* PASS ROM | Step 2.3 已标 V&0x1 改法的回归风险；`apu_01` sub-test 4/5、`apu_06` 为关键哨兵 |
| Phase 3 剩余 38 项精度面大、易摊薄 | 分桶后按子系统逐个 PR，宁缺毋滥，保留"有据已知限制"出口 |

### 禁忌清单（汇总）

1. 不动 ppudead 路径（`ppu_rendering.cpp:1526-1554`，P4-bridge Super Donkey Kong 修复依赖）
2. 不在 newppu 下让 `FCEUPPU_LineUpdate` 非 no-op（`:234-236`，会重引入旧 PPU glitch）
3. 不重复 P4-2 length reload（commit `562f0e8`/revert `cda40fe`）
4. 不"顺手修" $4017 bit 映射 swap（`apu_06` PASS 依赖）
5. 不改 savestate chunk 结构（`sound.cpp:1303-1307`）
6. 不用 runppu(N) 盲调 NMI 相位（已证伪两次）
7. `blargg_ppu_vbl_nmi` 全 PASS 前不升 blocking

### 文件:行号总索引

**E-1**：VBL 置位 `ppu_rendering.cpp:1560`；NMI latch `:1580`；`delay` 旋钮 `:1567`；VBL 清除 `:1595`；$2002 读+清 `ppu.cpp:327-349`；
$2000 NMI-enable 边沿 `ppu.cpp:601-615`；`TriggerNMI`/`TriggerNMI2` `x6502.cpp:395-403`；even/odd 跳点 `ppu_rendering.cpp:1979-1994`；`runppu` `:1361-1377`

**E-3**：帧 IRQ 置位 `sound.cpp:488-492`；5-step 额外周期 `:494-498`；length/sweep 半帧 clock `FrameSoundStuff :382-461`；
$4017 写 `Write_IRQFM :1050-1090`（`fcnt=0` `:1069`、`if(V&0x2)` `:1070`、`fcnt=1` `:1072`、`fhcnt=fhinc` `:1080`、条件清 `raw & 0x40` `:1081-1084`）；
fhcnt 递减 `FCEU_SoundCPUHook :543-579`；reset 状态 `FCEUSND_Reset :1195-1209`（`fhcnt=fhinc` `:1200`、`fcnt=0` `:1209`）；power `:1280-1284`；savestate chunks `:1303-1307`

### 参考来源（联机研究，2026-08-01 抓取）

- NESdev Wiki：APU Frame Counter、PPU frame timing、NMI（https://www.nesdev.org/wiki/...）
- `christopherpow/nes-test-roms`（GitHub）：
  - `ppu_vbl_nmi/source/05-nmi_timing.s`、`06-suppression.s`（+ readme.txt 期望输出表）
  - `blargg_apu_2005.07.30/readme.txt`（Mode 0/1 时序、jitter、power-on/reset、IRQ ≥29833）、`tests.txt`（11 个 ROM sub-test 码表）、`source/04.clock_jitter.asm`（29830/29831/29832 读取窗口原文）
  - `apu_reset/readme.txt`（power/reset 语义）

---

*方案完。Phase 1/2 每步落地后，按交接档案 §5 更新数据文档索引与状态摘要。*
