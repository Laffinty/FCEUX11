# FCEUX11 v1.18 — F11QA 残留精度长期演进计划

> **STATUS: OPEN**（长期演进；按 Phase 独立立项、逐步清零，不设一次性关闭日）
> **版本**：v1.18（演进计划 v1.0）
> **日期**：2026-09-27
> **基线**：GitHub Actions `f11qa.yml` run **98252605787**（logs 已本地归档）
> | 字段 | 值 |
> |---|---|
> | `git_rev` | `b0658c9` |
> | R4 gate | **passed** |
> | 迁移矩阵 | **106P / 14F**（grade **B**） |
> | 内部逻辑检测 | **42P / 0F**（全绿） |
> | 硬件一致性检测 | **64P / 14F**（用例层） |
> | blargg 全量 ROM | **145P / 32F**（177 ROM） |
> | `fail_to_pass` / `pass_to_fail` | **0 / 0** |
> | 14 项 FAIL 迁移态 | 全部 `fail_to_fail`（无回归、无进展） |
> | advisory-FAIL 占比 | 14/120 = **11.7%**（cap 15%） |
> | `vendor_state` | vendored=43 / advisory=13 / pending_vendor=22 |
>
> **前置**：v1.8 F11QA 建设已收口（`docs/history/plans/FCEUX11-v1.8_F11QA-构建计划.md` STATUS: CLOSED）；
> A-055 `cpu_reset_regs`、B-093 `bntest` 已清零；kgmqa-106 `vaus` 本基线已 **PASS**。
>
> **关联**：`docs/tech/F11QA.md`、`docs/tech/precision.md`、`docs/tech/f11qa-accuracy-backlog.md`、
> `tests/tests.json`、`tests/fixtures/blargg_known_fail.json`
>
> **文档集**（可独立领用执行）：
> [`README.md`](README.md) ·
> [`T0-指令时序与板级约束.md`](T0-指令时序与板级约束.md) ·
> [`T1-Mapper与DMA.md`](T1-Mapper与DMA.md) ·
> [`T2-中断时序.md`](T2-中断时序.md) ·
> [`T3-PPU读缓冲与外设时序.md`](T3-PPU读缓冲与外设时序.md) ·
> [`T4-聚合与FDS.md`](T4-聚合与FDS.md)
>
> **纪律**：known_fail / frozen baseline / savestate 金标更新须人工授权；AI 不得修改期望值（precision.md §3.6 / §4）。

---

## ◆ TL;DR

v1.8 把 F11QA 门槛抬到 **120 用例 / R4 四硬门禁 / grade B**。本计划只做一件事：

> **把矩阵里 14 个尚未通过的 F11QA 用例，按「解决难度从易到难」拆成可独立立项的精度分项计划，
> 供未来一段时期逐步、可回退、可验证地清零。**

| 维度 | 现状（run 98252605787） | 长期目标 |
|---|---|---|
| 矩阵 FAIL | 14（全 advisory、全 B 通道） | **0**（或全部升格有据 known_limit 并经评审） |
| advisory-FAIL 占比 | 11.7% | 持续下降，最终 < 5% |
| 发布评级 | **B** | **A**（无 blocking 缺口且 B 通道残留可控） |
| blargg 177 ROM | 145P / 32F | 随 T2/T3 攻关上移；`eventually_pass=true` 逐条清零 |
| 内部逻辑检测 | 42P / 0F | **保持全绿**（任何回退即停） |

**本计划不做的事**（避免范围膨胀）：

- 不改 R4 语义、不扩 120 清单、不做 harness 重构；
- 不把 blocking 降级为 advisory 来“刷绿”；
- 不在无探针数据时写入新的 `known_limit`。

---

## 一、筛选口径

### 1.1 什么是「尚未通过 F11QA」

以 CI 产物 `f11qa_migration_matrix.json` 的 `details[].passed == false` 为唯一口径：

| 通道 | PASS | FAIL | 说明 |
|---|---|---|---|
| 内部逻辑检测（oracle A） | 42 | **0** | CTest / 单元 / 回归，全部通过 |
| 硬件一致性检测（oracle B） | 64 | **14** | 第三方 ROM 精度缺口，全部 `failure_means=advisory` |
| **合计** | **106** | **14** | 与 Print Summary / R4 gate 一致 |

14 项 **全部** 满足：

- `oracle_type = B`（硬件一致性检测）
- `failure_means = advisory`（计入 15% 护栏，不阻断 R4）
- 迁移态 `fail_to_fail`（相对冻结基线无回归，也无进展）
- 均落在 **Core / Boards** 层，无 Driver / Lua / 流水线残留

### 1.2 双层残留（避免漏项）

| 层 | 数量 | 含义 | 本计划角色 |
|---|---|---|---|
| **L1 用例层** | **14** | 矩阵 `passed=false`，直接拖累 grade | **主清单**（§三） |
| **L2 ROM 层** | **32** | blargg 177 中仍 FAIL 的 ROM；多数被 `blargg_known_fail.json` 吸收，故不抬升 L1 | **副清单**（§四，支撑 T2/T3 深度） |

> 注意：L2 吸收 ≠ 已修复。`eventually_pass=true` 的 known_fail 仍是真实精度债，
> 只是当前不计入矩阵 FAIL。清零 L2 是冲 **grade A** 与 TASVideos 级精度的必要条件。

---

## 二、难度排序总览（易 → 难）

排序原则（可复核）：

1. **根因是否已写明**（known_limit / diag_string 可定位到函数或寄存器表）
2. **改动面**（1 张表 / 1 个 mapper / 1 条 DMA 路径 vs 中断模型 vs 多区域时序）
3. **回归半径**（是否牵动 savestate 金标 / golden 帧 / 其余 mapper）
4. **是否依赖上游 cascade**（聚合项等单点突破后自然收敛）
5. **是否属于业界著名难点**（$2007 read buffer、多制式 TV、外设扫描协议）

| 难度 | Tier | 用例 | 主题 | 预估 | 建议顺序 |
|---|---|---|---|---|---|
| ★☆☆☆☆ | **T0** | kgmqa-056 | 指令周期表 2 条非法指令 | 1–3 天 | 1 |
| ★★☆☆☆ | **T0** | kgmqa-078 | MMC1 SEROM/SHROM 板约束 | 2–4 天 | 2 |
| ★★☆☆☆ | **T0** | kgmqa-038 | `LDA abs,x` dummy read | 3–5 天 | 3 |
| ★★☆☆☆ | **T1** | kgmqa-097 | FME-7 WRAM 映射 | 2–4 天 | 4 |
| ★★★☆☆ | **T1** | kgmqa-096 | FME-7 IRQ ack | 3–7 天 | 5 |
| ★★★☆☆ | **T1** | kgmqa-050 | OAM DMA dummy write | 4–7 天 | 6 |
| ★★★☆☆ | **T1** | kgmqa-051 | 从 IO 空间取指 / bus dispatch | 4–7 天 | 7 |
| ★★★★☆ | **T2** | kgmqa-037 | 中断轮询下沉 + hijack | 1–2 周 | 8 |
| ★★★★★ | **T3** | kgmqa-049 | bisqwit $2007 read buffer | 2–4 周 | 9 |
| ★★★★★ | **T3** | kgmqa-099 | NTSC/PAL/Dendy TV 时序 | 2–4 周 | 10 |
| ★★★★★ | **T3** | kgmqa-107 | SNES 鼠标扫描协议 | 1–2 周 | 11 |
| ★★★★★ | **T3** | kgmqa-108 | FC 麦克风扫描协议 | 1–2 周 | 12 |
| ★★★★★ | **T4** | kgmqa-077 | Holy Mapperel 13 mapper 聚合 | cascade | 13 |
| ★★★★★ | **T4** | kgmqa-081 | FDS 加载路径 + IRQ 子系统 | 2–4 周 | 14 |

> **独立性**：每个 Tier 内条目可并行、可单独开分支/PR；跨 Tier 建议顺序执行，
> 但 T1 四项彼此无代码耦合，可按人力切分。

---

## 三、L1 主清单 — 14 项独立分析（易 → 难）

> 每项格式固定：**现象 → 根因 → 攻关步骤 → 验收 → 风险**。
> 状态栏供后续勾选；未开始默认 `TODO`。

---

### ① T0 · kgmqa-056-instr-timing-blargg　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★☆☆☆☆（**最易**） |
| 层 / 通道 | Core / B |
| ROM | `blargg/cpu/instr_timing_instr_timing.nes`（L2：`instr_timing`、`instr_timing_v2_1`） |
| 错误码 | `0x01` / `0x03`（L2 表内 `0x80`） |
| known_limit | instruction cycle table combined test; singles pass |

**现象（CI diag 已点名）**

```
Official instructions...
NOPs and alternate SBC...
E2 was 3, should be 2

Unofficial instructions...
BB was 4, should be 5 (cross)

NOPs and alternate SBC timing is wrong
1-instr_timing / Failed #3
```

**根因（可直接改表）**

- 非法 NOP `E2`：周期数被记为 3，硬件为 **2**。
- 非法 `BB`（LAX/XAA 变体）：跨页时应为 **5**，当前为 4（page-cross 未加 1）。
- 不是“组合项玄学”，就是 **指令周期表两条数据 + 跨页加成**。

**攻关步骤**

1. 在 `ops_table.inc` / `x6502.cpp` 周期表定位 `E2`、`BB`。
2. 先上 env-gated 探针打印这两条指令的实际耗时（Instrument-first）。
3. 改周期数；跑 `instr_timing` / `instr_timing_v2_1` 单 ROM。
4. 全量硬件一致性检测对账（145/32 不得回退）。

**验收**：kgmqa-056 矩阵 PASS；L2 中 `instr_timing*.nes` 转 PASS；`fail_to_pass≥1`、`pass_to_fail=0`。

**风险**：低。周期表变更可能影响帧对齐类 golden（若 runner 按帧截取）；出现 `pass_to_fail` 则回滚并查 frame budget。

---

### ② T0 · kgmqa-078-serom-lidnariq　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★☆☆☆ |
| 层 / 通道 | Boards / B |
| ROM | `lidnariq/serom/serom.nes` |
| 错误码 | `0xC3`（diag 恒 `[C3,C3,C3]`） |
| known_limit | MMC1 SEROM/SHROM bank-size constraint not modeled |

**现象**：握手寄存器 `$6000` 读出稳定 `0xC3`，测试在板级约束检查处失败。

**根因**：MMC1 的 **SEROM/SHROM 板型** 对 CHR/PRG bank 宽度有硬件约束（bank-size 译码约束），
当前 MMC1 实现未建模该约束，测试 ROM 在约束探测分支判定失败。

**攻关步骤**

1. 读 `src/boards/` 下 MMC1 实现，标出 CHR/PRG bank 写路径。
2. 对照 lidnariq 测试说明实现 SEROM/SHROM 约束（board type 判定）。
3. 单 ROM 验证 `$6000 == 0x00`。
4. 回归：mmc1 相关 L2 / Holy Mapperel 中 M1 系列 ROM 观察连带变化。

**验收**：kgmqa-078 PASS；`serom.nes` L2 PASS。

**风险**：低–中。约束过严可能影响普通 MMC1 游戏 ROM 的 bank 切换；用 mapper_byte_diff + 商业 ROM smoke 兜底。

---

### ③ T0 · kgmqa-038-instr-misc-blargg　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★☆☆☆ |
| 层 / 通道 | Core / B |
| ROM | `blargg/cpu/instr_misc.nes`（L2：`instr_misc_03_dummy`） |
| 错误码 | `0x01` / `0x03` |
| known_limit | instr_misc combined opcode group; singles pass, combo fails |

**现象（CI diag，比 known_limit 更精确）**

```
LDA abs,x
03-dummy_reads
Failed #3
While running test 3 of 4
```

- 子项 `01_abs_x` / `02_branch` / `04_dummy_apu`：**PASS**
- 子项 `03_dummy` 与组合 ROM：**FAIL**

**根因**：不是单纯“组合必挂”，而是 **`LDA abs,x` 的 dummy read**（abs,X 寻址在不跨页时仍发生的数据总线假读）行为不完整。
`instr_misc_03_dummy` 单项也挂，证明根因在 dummy read 语义，而非聚合调度。

**攻关步骤**

1. 对照 `cpu_dummy_reads`（已 PASS）与 `instr_misc_03_dummy` 的差异，锁定 `abs,x` 路径。
2. 实现/修正 abs,X 的 dummy read（含跨页与不跨页两种）。
3. 先过 `03_dummy`，再过组合 `instr_misc`。
4. 与 ① 同属 CPU 周期/dummy 语义，建议紧邻提交。

**验收**：kgmqa-038 PASS；`instr_misc*.nes` L2 全 PASS。

**风险**：低。dummy read 影响 open-bus / 外设副作用，注意不要连带改动 $2007/$4016 路径。

---

### ④ T1 · kgmqa-097-fme7ramtest-tepples　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★☆☆☆ |
| 层 / 通道 | Boards / B |
| ROM | `tepples/fme7/fme7ramtest.nes` |
| 错误码 | `0x01` |
| known_limit | FME-7 WRAM mapping edge |

**现象**：`$6000` 窗口 WRAM 映射边界失败（diag `[02,04,08]` 系列为分项偏移）。

**根因**：FME-7（Sunsoft 5B）WRAM 映射寄存器边界未完整建模（bank 号 / 使能位 / 窗口）。

**攻关步骤**

1. 读 FME-7 mapper 实现的 WRAM 映射寄存器写逻辑。
2. 对照 tepples 测试期望补全边界（含 bank 使能关闭时的总线行为）。
3. 与 ⑤ 同文件改动，可合并分支但 **分 commit**。

**验收**：kgmqa-097 PASS。

**风险**：低。

---

### ⑤ T1 · kgmqa-096-fme7acktest-tepples　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★☆☆ |
| 层 / 通道 | Boards / B |
| ROM | `tepples/fme7/fme7acktest.nes` |
| 错误码 | `0xA2`（diag `[0F,8E,00]`） |
| known_limit | FME-7 IRQ acknowledge edge |

**现象**：IRQ **应答（ack）** 边沿行为不符合硬件。

**根因**：FME-7 IRQ 状态/应答寄存器写时的清除边沿未精确建模（ack 后 IRQ 线撤销时机）。

**攻关步骤**

1. 探针记录 IRQ assert / ack / deassert 相对写时钟的时序。
2. 修正 ack 边沿；确认不影响 MMC3 IRQ 路径（独立模块，但共享 CPU IRQ 线）。
3. 全量 mapper IRQ 相关 ROM 对账。

**验收**：kgmqa-096 PASS。

**风险**：中。IRQ 共享路径可能影响 MMC3；发现 `pass_to_fail` 立即回滚。

---

### ⑥ T1 · kgmqa-050-cpu-dummy-writes-bisqwit　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★☆☆ |
| 层 / 通道 | Core / B |
| ROM | `bisqwit/cpu_dummy_writes_oam.nes` |
| 错误码 | `0x06` |
| known_limit | dummy writes to OAM DMA / PPU region; RMW macro OK, DMA path not |

**现象**：OAM DMA 路径的 dummy write 失败；**RMW 宏路径已正确**；`cpu_dummy_writes_ppu` **已 PASS**。

**根因**：OAM DMA（$4014）触发的假写路径未模拟；只修了 RMW 与 PPU 区域。

**攻关步骤**

1. 对比 `cpu_dummy_writes_ppu`（PASS）与 `_oam`（FAIL）的总线轨迹。
2. 在 OAM DMA 状态机补 dummy write（注意 DMA 与 DMC 的交互，L2 `sprdma_dmc_dma` 可能连带）。
3. 与 ③ 的 dummy read 属同一“CPU 总线副作用”主题，建议连续攻坚。

**验收**：kgmqa-050 PASS；观察 L2 `sprdma_dmc_dma*.nes` 是否连带改善（允许仍 FAIL，但不得回退）。

**风险**：中。DMA 路径与 DMC 交互复杂；`dma_sync_test_v2`（kgmqa-110，现 PASS）是回归哨兵，不得回退。

---

### ⑦ T1 · kgmqa-051-cpu-exec-space-bisqwit　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★☆☆ |
| 层 / 通道 | Core / B |
| ROM | `bisqwit/test_cpu_exec_space_apu.nes` |
| 错误码 | （矩阵 exit 1） |
| known_limit | execute-from-IO-space edge; bus dispatch coverage gap |

**现象**：从 **IO/APU 空间取指执行** 的边缘行为失败。blargg 侧 `cpu_exec_space_apu` 已 PASS，
bisqwit 变体更严格。

**根因**：bus dispatch 对“取指 vs 数据访问”未区分；从 $4000–$401F 取指时的映射/副作用不完整。

**攻关步骤**

1. 在 bus dispatch 打探针，区分 opcode fetch 与 data read/write。
2. 补全 IO 空间取指的读映射（与数据路径可能不同）。
3. 同步观察 `cpu_exec_space_ppuio`（L2，`0x05`）是否连带改善。

**验收**：kgmqa-051 PASS。

**风险**：中。bus dispatch 是热路径；改动后跑 `fceux11_bench_bus_dispatch` 性能哨兵。

---

### ⑧ T2 · kgmqa-037-cpu-int-2-nmi-brk-blargg　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★★☆ |
| 层 / 通道 | Core / B |
| ROM | `blargg/cpu/cpu_interrupts_v2_2-nmi_and_brk.nes` |
| 错误码 | `0x01` |
| known_limit | CPU interrupt polling only at instruction boundary (`x6502.cpp:515-579`); hijack/branch-delay not modeled |

**现象（diag）**

```
NMI BRK 00
27  36  00
26  36  00
...
```

NMI 与 BRK 交互的周期对齐差 1；L2 中 `cpu_int_3/4/5` 同族失败（NMI/IRQ、IRQ/DMA、branch-delay IRQ）。

**根因（结构性）**

- 中断仅在 **指令边界** 轮询（`x6502.cpp:515-579`）。
- 未建模 **中断劫持（hijack）** 与 **分支延迟窗口** 内的 IRQ/NMI 采样。
- 这是 CPU 时序模型级改动，不是补丁级。

**攻关步骤（必须分阶段）**

1. **调查页**：先开独立调查记录（参考 `docs/history/surveys/cpu_bucketB/`），列出 hijack 状态机。
2. **探针**：在 `x6502.cpp:515-579` 采样“每条指令内周期级的 NMI/IRQ 线”。
3. **最小模型**：先支持 NMI hijack BRK，验证 `cpu_int_2`。
4. **扩展**：`cpu_int_3`（NMI/IRQ）、`cpu_int_4`（IRQ/DMA）、`cpu_int_5`（branch delay）。
5. 每步全量对账；`fail_to_pass` 递增、`pass_to_fail` 必须为 0。

**验收**：kgmqa-037 PASS；L2 `cpu_int_*.nes` 尽可能清零；`nestest` / `instr_test_v5` **不得回退**（它们是 CPU 金标）。

**风险**：**高**。改动面大，最容易引入 `pass_to_fail`。建议独占分支 + 小步提交 + 每步跑 177 ROM。

---

### ⑨ T3 · kgmqa-049-ppu-read-buffer-bisqwit　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★★★ |
| 层 / 通道 | Core / B |
| ROM | `bisqwit/test_ppu_read_buffer.nes` |
| 错误码 | （矩阵 exit 1） |
| known_limit | bisqwit read-buffer monster test; open-bus/decay path incomplete |

**现象**：`$2007` **读缓冲**（PPU read buffer）行为不符。业界公认高难度精度测试。

**根因**：`$2007` 读缓冲 + 总线 **open-bus / 衰减** 路径不完整；与 VRAM 增量、镜像、渲染期读交织。

**攻关步骤（长期）**

1. 单独立项调查页；先做 `$2007` 读缓冲状态机文档（缓冲何时填、何时透传、何时衰减）。
2. 分子问题打探针：非渲染期读、渲染期读、缓冲刷新、open-bus 位衰减。
3. 按 bisqwit 测试分段推进，允许长期多 PR。
4. 可参考 L2 `ppu_vbl_nmi` / `ppu_open_bus` 调研（`docs/history/surveys/e1_vbl/`）。

**验收**：最终 kgmqa-049 PASS；过程中允许拆成子里程碑。

**风险**：**很高**。易牵动 PPU 渲染金标（golden frame / savestate）。**任何 PPU 变更后按纪律重生成金标前必须人工授权。**

---

### ⑩ T3 · kgmqa-099-240pee-damianyerrick　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★★★ |
| 层 / 通道 | Core / B |
| ROM | `240pee/240pee.nes` |
| 错误码 | （矩阵 exit 1） |
| known_limit | NTSC/PAL/Dendy TV timing display test |

**现象**：多制式（NTSC / PAL / Dendy）时序显示测试失败。

**根因**：当前核心以 NTSC 为主；PAL/Dendy 的 CPU/PPU 时钟比、扫描线数、帧长未完整建模。
测试可能在检测制式自动识别或时序显示时挂。

**攻关步骤**

1. 明确产品目标制式（是否正式支持 PAL/Dendy）。若只支持 NTSC，可申请有据 `known_limit` 并文档化——**需评审**。
2. 若支持：建制式表（CPU Hz / PPU 点 / 扫描线 / 帧）并接到 core 配置。
3. 先过 240pee 的单制式子项，再过全制式。

**验收**：kgmqa-099 PASS，或经评审的制式范围声明 + 分制式子测试 PASS。

**风险**：高。多制式影响所有时序测试的默认假设；必须作为特性级变更走。

---

### ⑪ T3 · kgmqa-107-mset-rainwarrior　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★★★ |
| 层 / 通道 | Core / B |
| ROM | `rainwarrior/mset/mset.nes` |
| 错误码 | （矩阵 exit 1） |
| known_limit | SNES mouse input scan protocol |

**现象**：SNES 鼠标输入扫描协议测试失败。

**根因**：$4016/$4017 手柄串行协议中的 **SNES 鼠标** 扩展协议未实现/不完整（分辨率位、溢出、移位时序）。

**攻关步骤**

1. 对照 rainwarrior mset 文档列出协议状态机。
2. 在输入扫描路径加鼠标设备类型；与标准手柄 / Zapper（已 PASS）共存。
3. 用 `porttest` / `allpads`（已 PASS）做回归哨兵。

**验收**：kgmqa-107 PASS。

**风险**：中–高。属外设协议，回归面在输入子系统；不碰 PPU/CPU 金标。

---

### ⑫ T3 · kgmqa-108-mict-rainwarrior　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★★★ |
| 层 / 通道 | Core / B |
| ROM | `rainwarrior/mict/mict.nes` |
| 错误码 | （矩阵 exit 1） |
| known_limit | Famicom microphone input scan protocol |

**现象**：FC 麦克风输入扫描协议测试失败。

**根因**：$4016 麦克风位（通常 bit 2/3 采样）未实现；与标准手柄移位交织。

**攻关步骤**

1. 实现麦克风采样位；提供测试夹具（恒定电平 / 噪声）。
2. 与 ⑪ 同属输入协议，可连续立项。

**验收**：kgmqa-108 PASS。

**风险**：中–高。同 ⑪。

---

### ⑬ T4 · kgmqa-077-holy-mapperel-tepples　`TODO`（cascade）

| 字段 | 值 |
|---|---|
| 难度 | ★★★★★（**结构**） / 实际取决于 mapper 单点 |
| 层 / 通道 | Boards / B |
| 协议 | `aggregate-mapperel`（13 mapper / 47 ROM 内部循环） |
| known_limit | Holy Mapperel aggregate 13 mappers / 47 ROMs; partial mapper coverage |

**现象**：47 个子 ROM 聚合，**全过才 PASS**。CI 本地 accuracy 表显示大量 `M0_*/M1_*/M4_*/M11_*` 等 FAIL。

**根因**：多个 mapper 边界缺口的 **合成结果**，不是单一 bug。

**攻关步骤（严禁整包硬攻）**

1. 把 47 子 ROM 按 mapper 归组（M0/M1/M3/M4/M9/M10/M11/M18/M34/M66/M69/M78/M118…）。
2. 按子组单独复现（runner 循环内日志），每个 mapper 一个小 PR。
3. **优先级**：先清与 ②④⑤ 同类的 MMC1 / FME-7 / 简单 UNROM 组，再啃 MMC3 IRQ 相关（与 L2 mmc3_* 呼应）。
4. 每清一组，观察 077 是否仍 FAIL；最终自然收敛。

**验收**：kgmqa-077 PASS（47/47）。

**风险**：中（单项）/ 高（整包）。禁止一次性大改 13 个 mapper。

---

### ⑭ T4 · kgmqa-081-fds-irq-tests-sour　`TODO`

| 字段 | 值 |
|---|---|
| 难度 | ★★★★★ |
| 层 / 通道 | Core / B |
| ROM | `sour/fdsirqtests/fdsirqtestsV7_patched.fds` |
| 错误码 | **`0xFE` = ROM load failure**（duration 55ms） |
| known_limit | FDS IRQ timing subsystem; not v1.8 focus |

**现象（双故障）**

1. **加载失败**：`.fds` 未能 `LoadGame`（`0xFE`）。
2. **IRQ 时序**：即便加载成功，FDS IRQ 子系统仍是已知缺口。

**根因**

- 路径 A：FDS 镜像加载对 patched `.fds` 的格式/头处理不完整（`kgmqa-011 fds_load` 单测是绿的，说明基础 FDS 加载可用，缺口在该 ROM 的具体形态）。
- 路径 B：FDS IRQ（磁盘传输 / 定时器）时序未建模完整。

**攻关步骤（先 A 后 B）**

1. **A 加载**：单独立项修 `.fds` 加载（对比 `kgmqa-011` 覆盖的样例）；`0xFE` 消失即阶段性成功。
2. **B IRQ**：FDS IRQ 状态机调查页（可参考 `docs/history/surveys/` 风格）；探针记录 IRQ 边沿。
3. 注意 082–085 TakuikaNinja FDS 族为 **advisory LICENSE**，不要求本阶段一并解决。

**验收**：kgmqa-081 从 `0xFE` 变为可运行并最终 PASS；或 A 完成后拆出独立 FAIL 项并更新 provenance（需评审）。

**风险**：高。FDS 子系统与存档/磁盘状态耦合；savestate 金标可能受影响。

---

## 四、L2 副清单 — blargg 32 ROM 残留（支撑深度）

> 权威文件：`tests/fixtures/blargg_known_fail.json`（P5.2，32 条）。
> 这些 ROM 不一定抬升 L1 用例 FAIL（known_fail 吸收），但 `eventually_pass=true` 的都是真实精度债。

| 组 | ROM | 码 | 与 L1 关系 | eventually_pass |
|---|---|---|---|---|
| 指令时序 | `instr_timing`, `instr_timing_v2_1` | 0x80/0x01/0x03 | → ① 056 | true |
| 中断 | `cpu_int_2/3/4/5` | 0x01 | → ⑧ 037 | true |
| dummy/IO | `cpu_dummy_writes_oam`, `cpu_exec_space_ppuio`, `instr_misc*` | 0x06/0x05/0x01/0x03 | → ③⑥⑦ | true |
| MMC3 | `mmc3_1..6`, `mmc3_v2_1..6`（12） | 0x02/0x03/0x04/0x09 | 支撑 ⑬ | true |
| VBL/NMI | `ppu_vbl_nmi`, `vbl_02/06/07/08/10` | 0x01/0x03 | 支撑 ⑨；已证伪深模型处方 | true |
| OAM | `oam_stress` | 0x01 | 部分 PPU 时序 | true |
| DMA | `sprdma_dmc_dma`, `sprdma_dmc_dma_512` | 0x01 | 可能被 ⑥ 连带 | true |
| 加载 | `cpu_interrupts` | **0xFE** | 永久 skip | **false** |

**建议**：T2/T3 攻关时 **按组清 L2**，每组结束更新 known_fail（人工授权），并在矩阵上确认 `fail_to_pass`。

> runppu 相关 7 项（`ppu_vbl_nmi` / `vbl_02/06/07/08/10` / `oam_stress`）是 PPU 深模型方向的探针，
> 与 ⑨ 合并评估；v1.16/v1.17 已证伪部分“深模型处方”，新方案必须带探针数据（见 surveys）。

---

## 五、阶段编排（供独立、逐步优化）

| Phase | 窗口 | 包含 | 出口指标 |
|---|---|---|---|
| **α 指令时序与板级约束** | 1–2 周 | ① 056 → ② 078 → ③ 038 | 14F → **11F**；advisory ≤ 9.2% |
| **β Mapper/DMA** | 2–4 周 | ④ 097 → ⑤ 096 → ⑥ 050 → ⑦ 051 | 14F → **7F**；L2 中 DMA 组观察 |
| **γ 中断模型** | 1–2 月 | ⑧ 037（+ L2 cpu_int_*） | 14F → **6F**；L2 中断组清零 |
| **δ 长周期精度项** | 1–2 季 | ⑨ 049、⑩ 099、⑪ 107、⑫ 108 | 每项独立里程碑；不强制同窗完成 |
| **ε 收敛** | 随 δ | ⑬ 077 cascade、⑭ 081（先加载后 IRQ） | 14F → **0F** 或有据 known_limit |
| **ζ 评级** | 末 | L2 eventually_pass 清零、advisory&lt;5% | grade **A** 评审 |

**工作方式（每项通用）**

1. 开调查/实现分支；先探针后改逻辑（precision.md §4 Instrument-first）。
2. 单点 commit；PR 描述含：根因假设、探针数据、回归命令。
3. 每次合并前：内部逻辑检测全绿 + 硬件一致性检测全量对账 + `pass_to_fail=0`。
4. 有意 PPU/CPU 语义变更才允许刷 golden（人工授权）。
5. 完成后把 `docs/tech/f11qa-accuracy-backlog.md` 对应条目划掉，并回填本文件状态栏。

---

## 六、本地复验命令

```powershell
# 1) 构建（含 f11qa-runner / blargg_runner）
cmake --build build --config Release

# 2) 内部逻辑检测
ctest --test-dir build -C Release --output-on-failure

# 3) 硬件一致性检测（177 ROM）
& build\tests\f11qa_blargg_runner.exe --manifest tests\fixtures\blargg_manifest.json

# 4) 迁移矩阵 + R4 等价物
& build\tests\f11qa-runner.exe `
  --manifest tests\tests.json --bin-dir build\tests `
  --output build\f11qa_migration_matrix.json `
  --known-fail tests\fixtures\blargg_known_fail.json `
  --baseline tests\fixtures\f11qa_baseline_frozen.json `
  --save-baseline build\f11qa_baseline_next.json
```

> CI 唯一可信来源为 `f11qa.yml` artifact；本地数字仅作迭代反馈。

---

## 七、风险与回退策略

| 风险 | 信号 | 处置 |
|---|---|---|
| `pass_to_fail > 0` | 矩阵迁移 | **立即回滚**该 PR，禁止带回归合并 |
| 内部逻辑检测红灯 | ctest | 同上，blocking |
| advisory 超 15% | R4 gate | 门禁失败；禁止用改 `failure_means` 绕过 |
| golden 抖动 | frame/savestate hash 变 | 停；确认是否“有意变更”；否则回滚 |
| L2 静默回退 | 145/32 数字下降 | 对照 `blargg_full_baseline.json` 查漂移 |
| 聚合项假进展 | 077 仍 FAIL | 按 mapper 组核对子 ROM，不看总码 |

---

## 八、状态总表（执行时勾选）

| # | kgmqa | 难度 | Tier | 状态 | PR / commit | 矩阵结果 |
|---|---|---|---|---|---|---|
| 1 | 056-instr-timing | ★☆☆☆☆ | T0 | TODO | | FAIL |
| 2 | 078-serom | ★★☆☆☆ | T0 | TODO | | FAIL |
| 3 | 038-instr-misc | ★★☆☆☆ | T0 | TODO | | FAIL |
| 4 | 097-fme7ram | ★★☆☆☆ | T1 | TODO | | FAIL |
| 5 | 096-fme7ack | ★★★☆☆ | T1 | TODO | | FAIL |
| 6 | 050-dummy-writes | ★★★☆☆ | T1 | TODO | | FAIL |
| 7 | 051-exec-space | ★★★☆☆ | T1 | TODO | | FAIL |
| 8 | 037-cpu-int-2 | ★★★★☆ | T2 | TODO | | FAIL |
| 9 | 049-ppu-read-buffer | ★★★★★ | T3 | TODO | | FAIL |
| 10 | 099-240pee | ★★★★★ | T3 | TODO | | FAIL |
| 11 | 107-mset | ★★★★★ | T3 | TODO | | FAIL |
| 12 | 108-mict | ★★★★★ | T3 | TODO | | FAIL |
| 13 | 077-holy-mapperel | ★★★★★ | T4 | TODO | | FAIL |
| 14 | 081-fds-irq | ★★★★★ | T4 | TODO | | FAIL |

**基线快照**：`b0658c9` / run 98252605787 / 106P-14F / grade B。
任何一行从 FAIL→PASS，先更新矩阵数字，再更新 backlog，最后更新本表。

---

## 九、附录 — 与 v1.8 backlog 的差分

| v1.8 backlog 条目 | 本基线状态 |
|---|---|
| A-055 cpu_reset_regs | **已清零**（PASS） |
| B-093 bntest | **已清零**（PASS） |
| A-038 / 056 / 037 | 仍在 FAIL，分别对应本计划 ③①⑧ |
| B-078 / 077 / 096 / 097 | 仍在 FAIL，分别对应 ②⑬⑤④ |
| C-049/050/051/099/107/108 | 仍在 FAIL，对应 ⑨⑥⑦⑩⑪⑫ |
| C-106 vaus | **已 PASS**（本基线） |
| C-081 fds_irq | 仍在 FAIL（0xFE 加载），对应 ⑭ |
| D-043 blargg_suite oracle | 仍由 oracle 收口，不在 L1 |

---

*本文件为 v1.18 残留精度的长期演进主计划。单项展开到实现级时，另开
`docs/plans/` 子页或 `docs/history/surveys/` 调查页，并回链到本文 §3 对应编号。*
