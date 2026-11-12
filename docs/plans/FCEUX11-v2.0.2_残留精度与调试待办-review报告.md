# FCEUX11 v2.0.2 残留精度与调试待办 · Review 报告

> **STATUS: OPEN → 已被承接**（2026-11-12）
>
> ⚠️ **本文件是 2026-11-08 的快照，不是「当前状态」的事实源。**
> 它的 17 个待核点已在 **v2.0.3 收口之后逐条回源码复核**（`main` @ `101026b`）⇒
> **13 项仍存在 / 1 项部分存在 / 2 项已消失**，并**外加 1 条本报告未列、但实测仍在的项**
> （F-1：Lua `emu.frameadvance()` 仍 abort `0xC0000409`，且门禁对它全盲）。
> 复核结论、**性价比裁决**与 v2.0.4 的执行清单见
> **`docs/plans/FCEUX11-v2.0.4_残留精度与调试待办-遗留项计划.md`**。
> ⇒ **接手人请以 v2.0.4 那份为准**；本文件保留作历史取证与原始证据坐标。
> **来源**：`docs/plans/FCEUX11-v2.0.2_DEBUG与性能优化计划.md` 与
> `docs/plans/long-term-evolution/`（README / 主报告 / T0–T4 五张执行卡）——**两份均已删除**。
> **已完成部分**见 `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md`。
> **本文件是 review 报告**：只陈述「什么还没做完、卡在哪、证据是什么」，
> **不含解决流程**（不写攻关步骤、不排期、不给方案选项）。
> **产出时点**：2026-11-08 · HEAD `2960998` · 门禁读数见归档文件 §一。

---

## 〇、判定口径

一条工作项进本文件，当且仅当它**没被归档文件收掉**，即以下任一条成立：

| # | 情形 | 典型 |
|---|---|---|
| R1 | **代码/文档已落地，但验收判据未满足** | A1（代码全好，手工 GUI 验收从未执行） |
| R2 | **主体未动工**，或只做了部分子步骤 | A7、B2(d)、D10、D1 的 S3–S5 |
| R3 | **用例仍 FAIL，且该 FAIL 是精度缺口本身**（不是业主裁决的已知限制） | 037、049、107、108、077、081、099 |
| R4 | **被否决 / 被关闭，但护栏或替代路径的落地没跟上** | A4-1 的否决注释未写进 `src/video.cpp` |
| R5 | **文档或清单的描述与事实相反**（review 发现，非工作项） | §三 |

> **R1–R4 是「未完成的工作项」；R5 是「review 发现的事实偏差」**，
> R5 不需要有人去做，但**必须有人去改**，否则下一个接手的人会按错的描述动手。

---

## 一、v2.0.2 线 —— 未完成的工作项

### 1.1 A1 —— 手工 GUI 验收从未执行（R1）

- **代码状态**：**100% 落地并已验证**。快门在 `src/debug.cpp:918-919`（11 路 OR 早退），
  `e1_last_pc` 写入已移入门内（`src/x6502.cpp:1066`），`map_irq_hook` 热路径 load 已改 relaxed
  （`src/cpu.h:141`，store 仍 release `:144`）。提交 `55d82d0`。
- **验收状态**：**未闭合**。`docs/tech/debug-regression-checklist.md` 的勾选表
  （口径为 **21 行**：H 3 / G 18 / C 0）**从未被任何人执行**——ctest 抓不到这类回归；
  业主走查单 `docs/tech/a1_gui_walkthrough.md`（18 项，估一次 GUI 会话 2–3 小时）同样未执行。
- **已被自动闭合的部分**：L-2 核心路径**已由无头验证自动闭合**（`FCEUX11_TRACE` 全链路：
  两 ROM 格式 0 错位、首行 PC == 复位向量、操作码字节 == ROM 字节、助记符抽查 94650/94650、
  阴性对照无文件）。
- **被缺陷阻断的部分**：LUA-1 / LUA-2 被 **F-1**（Lua `emu.frameadvance()` ⇒ Rust 回调内跨 C 边界
  yield ⇒ `panic in a function that cannot unwind` ⇒ abort `0xC0000409`；绑定自 2026-06-21
  `d119514` 未改，**非 A1 回归**）阻断，走查时按「因 F-1 阻断」标注，不计为本项失败。
- **仍无任何一层验证的**：A1 的 `break_asap` 门腿、A1-b 反向项（F-1 修复前）。
- **另有一处已知行为变化，走查时看到不应判红**：`vblankScanLines` / `vblankPixel` 的计算
  **位于门内**（`src/debug.cpp:921-929`）⇒ 零断点时调试器 PPU 页签的 vblank 读数停在旧值。

### 1.2 A4-1 —— 否决结论的护栏未落地（R4）

- **工作项状态**：**正确地未实施**（已被盘点否决）。A4-1 = 帧级内存流量收窄的消费者计数门控，
  否决理由 = savestate chunk-8 任意帧异步消费 ⇒ 金标漂移，谓词恒真。
- **残留**：计划 §十 r6 承诺「结论存档进 `src/video.cpp` 注释防后人实施」——
  **该注释不存在**。`src/video.cpp`（681 行）全文 grep `A4|否决|盘点|chunk|谓词|消费`
  仅命中 `:502` 一条无关的 PNG chunk 注释；`git log --all -S '消费计数'` 显示该字符串
  **只出现在 `CHANGELOG.md` 与计划正文，从未进入任何 `src/` 文件**。
- **为什么这算未完成**：否决理由目前**只活在文档里**；`src/video.cpp:254` 仍是无条件
  `memcpy(XBackBuf, XBuf, 256*256)`（64 KB/帧），**看起来该被优化**，而计划 §二 A4
  仍以「收益低-中、风险低-中」描述它。

### 1.3 A7 —— 指令 handler 别名/溢出障碍（R2）

- **状态**：**未动工**。`src/ops_table.inc` 的 **256 个 handler 全部** `(void)xp;` 丢弃参数；
  `src/x6502abbrev.h:11` `#define _X g_cpu.native_layout().X` ⇒ 编译器仍须假定每次
  RdMem/WrMem 可写 `g_cpu`，寄存器值全部反复重载。
- **约束**：`layout_` 是 savestate `static_assert` 对象（`src/cpu.cpp:14-24`），**布局不得动**。
- **权重变化**：A6 被业主裁决关闭时（r25），把它与 PGO 并列为「**从未动过的两个更便宜替代**」
  ⇒ A6 关闭后 A7 的槽位价值上升（它是 A6 关闭后剩下的唯一性能替代项之一）。

### 1.4 B2 —— profiler 残缺两处（R2）

| 子项 | 状态 | 证据 |
|---|---|---|
| (a) `FCEU_PROFILER_ENABLE` 做成 CMake option | ✅ | `CMakeLists.txt:137`，默认 **OFF** |
| (b) 关键函数探点 | ⚠️ **4/5** | ✅ `Emulate`（`src/fceu.cpp:804`）、`DoLine`（`src/ppu_rendering.cpp:686`）、`RefreshLine`（`src/ppu_rendering.cpp:301`）、`WriteSound`（`src/drivers/Qt/sdl-sound.cpp:376`）；❌ **`FCEU_SoundCPUHook` 探点不存在**（`src/sound.cpp` 内零 `FCEU_PROFILE_FUNC`；该函数定义在 `src/sound.cpp:767`） |
| (c) 文本报告（min/max/sum/numCalls 表） | ✅ | `src/profiler.cpp:332-394`，表头 `sum(s) %total avg(us) min(us) max(us) calls`，按 sum 降序；`src/fceu.cpp:707` 收尾打印 |
| (d) Qt 菜单项 | ❌ | Qt 侧只有 `profiler.h` include 与 `FCEU_profiler_log_thread_activity()`（`ConsoleWindow.cpp:743`、`fceuWrapper.cpp:1536/1552`），**无菜单动作** |

- **注意 (b) 的省略是「有意」的**：提交说明自陈「`FCEU_SoundCPUHook` 每指令热路径按 R4 裁决不上」，
  **但计划 §三 与 §十 r6 均未记录这一省略** ⇒ 计划 §三 仍列着 5 个探点，
  下一个接手的人会去找一个被刻意否掉的探点。
- (d) 在计划中标为「可选」，但**未登记为已知限制**。

### 1.5 D1 / 037 —— 中断时序模型：S3–S5 未做，round-2 两章程转长尾（R2 + R3）

- **已落地**：S0 调查页（`docs/history/surveys/cpu_interrupt_hijack/README.md`）、
  S1 探针 `FCEUX11_INTLEDGER`、S2 两拍轮询 + **向量级 hijack**、S2.5 第一轮
  （per-line `g_intl_assert_at` / `TriggerNMIAt` / `X6502_IRQBeginAt` / `intl_stamp_legacy` /
  `fceu11_cpu_abs_ts`）、S2.5 第二轮 **3/4** 阶段
  （① OAM DMA 513/514 奇偶 `e35b748` ② APU frame IRQ 精确戳 `81f6631`
  ③ poll 采样点 override `a720db9`）。
- **残留**：
  - **S3**（`cpu_int_3` NMI/IRQ）、**S4**（`cpu_int_4` IRQ/DMA）、**S5**（`cpu_int_5`
    branch-delay）—— 三个模型扩展全部未做；
  - **round-2 两章程**：**(C) DMC 计费范围界定**、**VBL 锚 K 扫参** —— 业主裁决 r26 **转长尾**；
  - **S2.5 阶段 4（VBL 锚 K 首切）已回退**：K=13 在新登记结构下过早 ~3 周期，被 G-2 金丝雀
    咬住（派发 PC 落在同步例程而非 marker 区）；K 为 `constexpr` 无法免构建扫描 ⇒ 回退。
- **当前读数**：037 仍 **FAIL 0x01**（S2 期 7/10 行）；`cpu_int_4` = **16/18**（round-2 后）；
  `cpu_int_3` 收敛 +1 行；`cpu_int_5` 校验和 `79064A81` 不变。
- **未收敛的模型分歧**：capture 形态 `cpu_int_4` = 18/18 ROM PASS 但 `cpu_int_5` −1 行；
  override 形态零回退但 `cpu_int_4` 差 2 行 ⇒ 差异机理 = **DMCDMA 的 `X6502_DMR` 计费算不算
  某条指令**（18/18 需要计入，与「DMA 停摆不属于任何指令」模型**矛盾**）—— 即 (C) 章程。
- **终态**：`kgmqa-037-cpu-int-2-nmi-brk-blargg` = **advisory** + 冻结基线 **False**
  ⇒ 不判红、不影响 grade。
- **一处已纠正的模型误读（纪律 12 第四例，供后人对照）**：S1 调查页与首版 hijack 的
  「压栈 B=0」判读**有误**——blargg README 该行实为 "with B bit set on stack"（期望行 `36 00 00`）
  ⇒ 正确模型是**向量级劫持**（push 不重压、只改向量）。调查页 §10 已记录。

### 1.6 D3 / 049 —— `$2007` 读缓冲：M1–M3 未做（R2 + R3）

- **已落地**：**M0 状态机文档**（`docs/tech/ppu_2007_read_buffer_state_machine.md`，
  R1–R16 迁移表；性质 = **纯文档、零代码改动**）。
- **未做**：**M1**（非渲染期/渲染期读探针）、**M2**（缓冲刷新语义）、**M3**（open-bus 位衰减）。
  文档 §4 明列两项已知未实现：**U1** 渲染期隐式填充 VRAMBuffer（实测零赋值点）、
  **U2** per-bit 衰减（defer）。
- **⚠️ 用例 PASS 的来源必须写清**：kgmqa-049 现在的 PASS 来自**帧预算伪 FAIL 修正**
  （600 → 2000，`tests/tests.json` 的 `provenance` 明写「600 was a harness pseudo-fail
  (0x80 = still running)」），**不是精度修复**。
  ⇒ **「kgmqa-049 PASS」不等于「读缓冲做对了」**；bisqwit 分段全过（M4）并未达成。
- **终态**：**advisory** + 冻结基线 **False**。

### 1.7 D7 / 081 —— Path B（FDS IRQ 状态机）未做（R3）

- **Path A（0xFE 加载失败）已关闭**：根因 = headless **从不调 `SetBaseDirectory`**
  （唯一调用点在 `src/drivers/Qt/config.cpp:509`）⇒ BaseDirectory 空 ⇒ BIOS 查找落到盘根
  ⇒ 加载失败。已重分类为 **BIOS 门控 known-limit**（提交 `c714a72`，**零代码改动**；
  `src/fds.cpp` 最后提交 `94df0ce`（2026-07-18）**早于**本轮全部 D 工作）。
- **残留**：**Path B** —— FDS IRQ 状态机（磁盘传输 / 定时器建模）**未做**。
- **附带**：原调查记录提到「旁证 082/083/084 PASS 也被证伪」（ROM 不在镜像里，advisory skip 路径），
  且 `$6000` 判据对「从未跑起来」也是假绿（`--frames 1` 即 0x00）。
- **终态**：FAIL，**advisory** + 冻结基线 **False**（裁决明确「baseline 不翻」）。

### 1.8 D9 / 107 —— SNES 鼠标：端到端 0xFC 未达成（R3）

- **已落地**：协议四缺陷修复（`src/input/snesmouse.cpp`：① byte0 签名 `0xFF`/`0x00` 交替
  `:94-95` ② dy 对称倍率补齐 `:75-80` ③ ×1/×2/×4 `:77` ④ 6-bit 幅度 + bit6 溢出 `:85-88/:101-102`）；
  测试桥注入面（`src/f11qa_bridge.h:60` / `src/f11qa_bridge.cpp:78` `kagami_bridge_set_input`、
  `tests/tests.json` 的 `sut_input: [8,0]`、`src/rust/crates/f11qa/src/adapter/direct.rs:139-147`）；
  锁测试 `tests/f11qa/core/input_snesmouse_test.cpp`（T1–T6，`:126-250`）。提交 `05bfc1f`。
- **残留**：端到端 **0xFC 未达成** —— 需真实鼠标交互；业主裁决 r11④ **defer 记为已知限制**。
- **终态**：FAIL，**advisory** + 冻结基线 **False** ⇒ 判红体系零影响。

### 1.9 D10 / 108 —— FC 麦克风协议：**完全未实现**（R2）

- **计划状态 vs 实际**：计划 §四 D10 判「**入选**」，但**零实现**。
- **证据**：`src/input.cpp:131-140` 仍是**原始粗实现**，逐字保留自注释
  「Not verified against hardware.」；逻辑 = 按 `joy[1]&8` 翻转一个布尔量后 `ret|=4`
  （**单一 bit2**，非硬件采样位 2/3 协议）。
  `src/input.cpp` 自 2026-09-28 起唯一提交 `05bfc1f` **不含麦克风改动**
  （其 diff 为 snesmouse.cpp / f11qa_bridge / rust runner / tests）。
  **全仓无麦克风锁测试**（`tests/f11qa/core/` 下只有 `input_snesmouse_test.cpp`）。
- **终态**：FAIL，**advisory** + 冻结基线 **False**。

### 1.10 D8 / 099 —— PAL/Dendy：等业主产品决策（R3）

- **状态**：业主裁决 r11⑤ **defer**（主题外扩）。计划 §四 D8 判「卡产品决策」。
- **残留**：产品是否正式支持 PAL/Dendy **未决** ⇒ 制式表（CPU Hz / PPU 点 / 扫描线 / 帧）、
  接到 core 配置、单制式子项 → 全制式，全部未做。
- **证据**：2026-09-28 后**无任何** PAL/Dendy/region/vidsys 文件改动（排他性检查）。
- **终态**：FAIL，**advisory** + 冻结基线 **False**。

### 1.11 PGO 开关 —— A6 关闭后的替代槽位，从未评估（R2）

- A6 关闭裁决把 **PGO** 与 **A7** 并列为「从未动过的两个更便宜替代」；
  `CMakeLists.txt:95-96` 的 `FCEUX11_PGO` / `FCEUX11_PGO_USE` 现成可用。
- **状态：从未评估**（无任何读数、无任何结论）。

---

## 二、v1.18 长期演进 —— 14 项逐项校准后的真实状态

原 `long-term-evolution/` 的状态快照停在 **v1.18.2（2026-09-28）**，与实际状态已严重脱节。
下表是**逐项校准后的真实状态**（14 项全覆盖）：

| # | kgmqa | 原卡片状态 | **当前真实状态** | 残留 | 判红通道（实测 `tests/tests.json` + 冻结基线） |
|---|---|---|---|---|---|
| 1 | 056 instr_timing | DONE | **DONE** | — | advisory / 基线 **True**（PASS）→ **已归档** |
| 2 | 078 serom | DONE | **DONE** | — | advisory / 基线 **True**（PASS）→ **已归档** |
| 3 | 038 instr_misc | defer | **defer** | S2 需**重新定义**；候选 `GetIX`（`LDA (z,x)`/`STA (z,x)` 目前**完全无 dummy read、也无跨页周期**，而它们在 ROM 声明的 8 种被测形式里）；真根因 `set_db()` **零调用点**未动 | advisory / 基线 **False** |
| 4 | 097 fme7ram | abandon | **abandon** | — | advisory / 基线 **False** → **已归档** |
| 5 | 096 fme7ack | abandon | **abandon** | **留一个已知不修的 ack bug**（`src/boards/69.cpp:130` 无条件 ack） | advisory / 基线 **False** → **已归档** |
| 6 | 050 dummy_writes | TODO | **已完成（PASS）** | — | advisory / 基线 **False**（**fail_to_pass 漂移，未授权翻基线**）→ **已归档** |
| 7 | 051 exec_space | TODO | **已完成（PASS）** | — | advisory / 基线 **False**（同上）→ **已归档** |
| 8 | 037 cpu_int_2 | TODO | **部分** | S3 / S4 / S5 三个模型扩展；round-2 两章程（转长尾） | advisory / 基线 **False** |
| 9 | 049 ppu_read_buffer | TODO | **部分** | M1 / M2 / M3 | advisory / 基线 **False**（**PASS 来自预算修正**） |
| 10 | 099 240pee | TODO | **defer** | 等 PAL/Dendy 产品决策 | advisory / 基线 **False** |
| 11 | 107 mset | TODO | **部分** | 端到端 0xFC（需真实鼠标交互） | advisory / 基线 **False** |
| 12 | 108 mict | TODO | **未开始** | 全部 | advisory / 基线 **False** |
| 13 | 077 holy_mapperel | TODO（cascade） | **未开始** | 47 子 ROM 聚合；成本已两次收窄为「**仅 L2 MMC3 IRQ 组**」⇒ **完全依赖 D1**。⚠️ **度量只看子组完成数**，47/47 是合取判定、滞后于任何单点进展 | advisory / 基线 **False** |
| 14 | 081 fds_irq | TODO | **部分** | Path A 已关闭（前提证伪）；**Path B** 未做 | advisory / 基线 **False** |

**完成度**：**4 / 14 已终结并归档**（056、078、097、096）＋ **2 / 14 已完成且 PASS**（050、051）＋
**2 / 14 defer**（038、099）＋ **4 / 14 部分**（037、049、107、081）＋ **2 / 14 未开始**（108、077）= **14**。
> ⚠️ 049 **归在「部分」**：它的用例虽已 PASS，但那是**帧预算放宽**的结果，M1–M3 的实体工作没做 ⇒
> 「PASS」不等于「读缓冲做对了」（见 §1.6）。
> 原卡片的「**2 / 14 DONE（14%）、9 项未开始**」已完全过时。

---

## 三、review 发现：文档与清单的过期 / 错误描述（R5）

> 这些**不是工作项**，但每一处都会误导下一个接手的人。改动量都是纯文本。

### 3.1 `tests/tests.json` 的 `known_limit` 文本（4 处与事实相反）

| 用例 | 现文本要点 | 与事实的差异 |
|---|---|---|
| `kgmqa-037-cpu-int-2-nmi-brk-blargg` | 「polling only at instruction boundary (`x6502.cpp:515-579`); **hijack/branch-delay not modeled**」 | **两拍轮询与向量级 hijack 已落地**（`src/x6502.cpp:400` 的 `g_intl_pending`、`:848` 的 `X6502_NmiHijacked()`）⇒ 该描述**不成立**（branch-delay 仍未建模，但 hijack 已建模） |
| `kgmqa-107-mset-rainwarrior` | 「SNES mouse input scan protocol」 | 协议四缺陷**已修**；遗留实为**端到端 0xFC** |
| `kgmqa-038-instr-misc-blargg` | 「combined opcode group; singles pass, combo fails」 | 与 defer 状态一致，但**未记真根因**「`set_db()` 零调用点 ⇒ open bus 是死的」 |
| `kgmqa-056` / `kgmqa-078` | 「instruction cycle table combined test; singles pass」/「MMC1 SEROM/SHROM bank-size constraint **not modeled**」 | 两项**都已 DONE 且现已 PASS** ⇒ 仍是**修复前**的旧文本 |

> `known_limit` 字段**不参与任何机械判定**（`grade.rs::compute_grade` 只读 `failure_means` /
> `pass_to_fail` / `new_test` / `test_set_diff` / `summary.failed`）
> ⇒ **不影响矩阵与 grade**，但 manifest 里有与事实相反的描述。

### 3.2 v2.0.2 计划 §十 r26 的一条论证被证伪（**影响「两项章程转长尾」的依据措辞**）

- **原文**：「int_4 / 037 / 050 / 107 在 `tests/tests.json` 里**全是 `failure_means: advisory`
  且 frozen baseline 为 `false`**」。
- **实测**（本次主会话独立复核）：
  - `kgmqa-037-cpu-int-2-nmi-brk-blargg` = advisory，基线 **False** ✅
  - `kgmqa-050-cpu-dummy-writes-bisqwit` = advisory，基线 **False** ✅
  - `kgmqa-107-mset-rainwarrior` = advisory，基线 **False** ✅
  - **int_4 —— 不成立**：`tests/tests.json` 里**根本没有 int_4 这个 kgmqa id**
    （全文检索无命中）。它对应的是 blargg **L2 ROM** `cpu_int_4_irq_dma.nes`，登记在
    `tests/fixtures/blargg_known_fail.json:76-81`（**无 `failure_means` 字段**、
    **不在 kgmqa 冻结基线内**、`eventually_pass: true`）。
    kgmqa 层面覆盖中断族的 `kgmqa-054-cpu-interrupts-v2-blargg` 是 **blocking** + 基线 **True**。
- **结论的净效果**：「修它判红体系零收益」这个**结论在 L2 blargg 通道上仍成立**
  （它在 known-fail 名单里），但**给出的证据被错置到了 kgmqa 矩阵通道**。
  ⇒ 措辞需改，**结论不必翻**。

### 3.3 冻结基线与实测存在三条未授权漂移

- `tests/fixtures/f11qa_baseline_frozen.json` 的 `generated_at` 仍是 **2026-09-26**，
  `results` 只有 **121** 条（`kgmqa-121-apu-frame-timing` 在基线外，走 `new_test` 桶），
  而 `tests/tests.json` 已有 **122** 条用例。
- **049 / 050 / 051** 三条的 FAIL→PASS 漂移，`baseline_drift` 中 `approved: false`
  ⇒ **基线尚未经人工授权翻转**（属期望值变更，按 r15 口径需同级评审）。
- ⚠️ **049 尤其不宜翻**：它的 PASS 建立在**帧预算放宽**之上而非精度修复之上，
  翻基线会让 049 看起来像「精度已达标」——这正是它实际不具备的。

### 3.4 「内部逻辑检测保持 42P / 0F」这条不变式**当前不成立**（本次实跑发现）

- 原 `long-term-evolution/README.md:187` 把「**内部逻辑检测保持 42P / 0F**，红灯即停」
  列为每个分项计划的通用不变式。
- **本次实跑读数**：`oracle_breakdown.A_regression` = **42 pass / 1 fail**，
  `B_hardware` = **70 pass / 9 fail**。
- 那 1 个 FAIL 是 **kgmqa-115-bench-tolerance** —— 墙钟类 bench，其 PASS/FAIL **不具跨机可比性**
  （基线采于 dev box，本次跑在更慢的机器上，超 +10% 容差）。该用例已在业主裁决 r15 中
  **移出精度回归判定**（`failure_means` blocking → advisory、冻结基线 true → false），
  故**不判红、不影响 grade**（`grade_reasons` = 「10 advisory known-limit failure(s),
  all within frozen baseline」）。
- ⇒ **不是回归，是不变式本身过时了**：该条的「0F」是在 kgmqa-115 尚为 blocking 时写的。
  接手人若按字面执行「红灯即停」，会对着一个**设计上就不该判红**的用例反复排查。

### 3.5 其它过期描述

| 位置 | 问题 |
|---|---|
| v2.0.2 计划 §二 第 56 行 | 节状态头仍写「A1/A2/A3/A4 已落地；**A5、A6、A7 未动**」⇒ **A5 行已过时**（A5 已在 r7/r8 落地）。AGENTS.md 已对 A6 发出警告（「那是旧文，别照着做」），**但 A5 行同样已过时且无对应警告** |
| v2.0.2 计划 §二 第 134 行 | A6 标题仍是「单独立项，排最后」⇒ 已被 r25 裁决关闭 |
| v2.0.2 计划 §四 C3 | `#[ignore]` 计数写「**20** 条（frame.rs 5）」⇒ **实为 21 条（frame.rs 6）**；同文件 §十 r6 写的是 21 ⇒ **计划内部自相矛盾**。**AGENTS.md 的 21 是正确的** |
| v2.0.2 计划 §十 r7 / 提交 `34ce329` | 「test_memory.lua **+13** 用例」⇒ **实为 +10**（18 → 28），其中 2 条是恒真空断言 |
| `long-term-evolution/README.md:142` 与主报告 `:314` | 「108P-11F」是**预期值**而非实测（实测 122 total / 112 passed / 10 failed / grade B） |
| `src/tests/CMakeLists.txt:65` | 仍声明 `add_test(NAME rom_regression_test ...)`，而 `tests/CMakeLists.txt:106` 写「C++ harness `rom_regression_test.cpp` was deleted」。事实：`src/CMakeLists.txt` 只 `add_subdirectory(rust)` ⇒ **`src/tests/` 从未进构建树**，该夹具是**死代码**，其 `add_test` 不在 36 名单里。**不是门禁缺口** —— 同批 780 条金标由 Rust 夹具 `rom_regression_rust_smoke`（ctest #35）覆盖且在门禁路径上 |

---

## 四、复验命令

```powershell
$vc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Auxiliary\Build\vcvars64.bat"

# 门禁（不变式 1）—— 必须等构建结束再跑，不要与构建/测试并行
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11 && cmake --build build --config Release"
$env:PATH="D:\Project\FCEUX11\vcpkg_installed\x64-windows\bin;$env:PATH"
& "D:\Program Files\CMake\bin\ctest.exe" --test-dir D:\Project\FCEUX11\build -j 4 -LE perf

# Rust 锁测试（四条都要跑：cargo test -p fceux11-rust 不跑依赖 crate 的测试）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-rust --no-default-features --features gba --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p gba-core --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-debug --lib"
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo test -p fceux11-media"

# F11QA 迁移矩阵（122 用例；pass_to_fail 必须为空）
cmd /c "call `"$vc`" >nul 2>&1 && cd /d D:\Project\FCEUX11\src\rust && cargo build --release -p f11qa --bin f11qa-runner"
& src\rust\target\x86_64-pc-windows-msvc\release\f11qa-runner.exe --manifest tests\tests.json --bin-dir build\tests --output build\f11qa_migration_matrix.json --accuracy-table build\f11qa_accuracy_table.md --known-fail tests\fixtures\blargg_known_fail.json --baseline tests\fixtures\f11qa_baseline_frozen.json --save-baseline build\f11qa_baseline_next.json

# 单个长期难题用例（示例：037 / 050 / 051 / 107 / 108）
& src\rust\target\x86_64-pc-windows-msvc\debug\f11qa-rom-runner.exe --kgmqa-id kgmqa-037-cpu-int-2-nmi-brk-blargg --frames 3000
```

> `f11qa-rom-runner` 是 f11qa crate 的 Cargo `[[bin]]`（`src/rust/crates/f11qa/Cargo.toml:15`），
> **不是 CMake 目标**，产物在 `src\rust\target\…\debug\` 而非 `build\tests\`。

---

## 五、本报告的边界

- **已实跑**：`ctest -LE perf`（36/36）、四条 Rust 锁测试（249 / 242 / 93 / 52）、
  Release 构建（exit 0）、F11QA 迁移矩阵（见归档文件 §一）。
- **未实跑**：各长期难题 ROM 的**逐行**读数（`cpu_int_4` = 16/18 等）来自计划 §十 r25 的记录，
  本次**未逐行复验**；`cpu_int_3` / `cpu_int_5` 的校验和同理。
- **不可得的读数**：077 的 47 子 ROM **子组分解**——矩阵产物只给聚合级 FAIL（43.8 s），
  不给子组明细；T4 卡要求的「用子组完成数度量」在当前产物里**不可得**。
- **CI 远端 run 读数**（r16 记的 `42352471017` 等）：本地不可复现，**未验证**。
- **未做**：`cargo fmt`、任何 git 写操作（commit / branch / push）——本次只做 review 与文档重组。

---

*本文件由独立 code review 产生，与 `docs/history/plans/FCEUX11-v2.0.2_DEBUG与性能优化-已完成项归档.md`
共同构成被删除的两份计划的唯一继任者。*
