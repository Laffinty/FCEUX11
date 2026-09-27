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
> **当前进度**（截至 v1.18.2，含 2026-09-28 重估裁定）：
> 矩阵 **108P / 12F**（grade **B** 不变）· blargg **147P / 30F** · advisory-FAIL **10.0%**
> · `pass_to_fail = 0` / `fail_to_pass = 0`（2026-09-28 全量矩阵实测：
> 120 用例，`new_test=0`，`test_set_diff` added/removed 均为空）。
> **L1 终态分布**：**2 DONE**（① 056、② 078 —— **T0 收口 2/2**）·
> **2 abandon**（④ 097、⑤ 096，均为 survey ROM，非精度债）·
> **1 defer**（③ 038，Tier 已改判为 T3）· **9 项未开始**。
> **defer / abandon 不改动矩阵预期值** —— 矩阵仍 12F，12 个 FAIL 全部保留。
> **T1 只剩 2 项可攻关**（050 / 051）。口径与完整论证见 **§2.2**；
> 逐项完成度与排序见 **§八**。
> 上表为**原始基线**（`b0658c9` / run 98252605787），保留作对照。

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

| 难度 | Tier | 用例 | 主题 | 预估 | 建议顺序 | 状态 |
|---|---|---|---|---|---|---|
| ★☆☆☆☆ | **T0** | kgmqa-056 | 指令周期表 2 条非法指令 | 1–3 天 | 1 | **DONE** |
| ★★☆☆☆ | **T0** | kgmqa-078 | MMC1 SEROM/SHROM 板约束 | 2–4 天 | 2 | **DONE** |
| — | **移出** | kgmqa-097 | FME-7 WRAM 映射 | — | — | **abandon**（经查非精度债，见 §2.2） |
| — | **移出** | kgmqa-096 | FME-7 IRQ ack | — | — | **abandon**（survey ROM；**留有真实 ack bug 不修**，见 §2.2 ⑸） |
| ★★★☆☆ | **T1** | kgmqa-096 | FME-7 IRQ ack | 3–7 天 | 4′ | **abandon**（survey ROM，代码不动，见 §2.2 ⑸） |
| ★★★☆☆ | **T1** | kgmqa-050 | OAM DMA dummy write | 4–7 天 | 5 | TODO |
| ★★★☆☆ | **T1** | kgmqa-051 | 从 IO 空间取指 / bus dispatch | 4–7 天 | 6 | TODO |
| ★★★★☆ | **T3** | kgmqa-038 | CPU 数据总线锁存（049 前哨） | 1–2 周 | 7′ | **defer**（★2→★4 / 风险低→中，见 §2.2） |
| ★★★★☆ | **T2** | kgmqa-037 | 中断轮询下沉 + hijack | 1–2 周 | 8 | TODO |
| ★★★★★ | **T3** | kgmqa-049 | bisqwit $2007 read buffer | 2–4 周 | 9 | TODO |
| ★★★★★ | **T3** | kgmqa-099 | NTSC/PAL/Dendy TV 时序 | 2–4 周 | 10 | TODO |
| ★★★★★ | **T3** | kgmqa-107 | SNES 鼠标扫描协议 | 1–2 周 | 11 | TODO |
| ★★★★★ | **T3** | kgmqa-108 | FC 麦克风扫描协议 | 1–2 周 | 12 | TODO |
| ★★★★★（**派生**） | **T4** | kgmqa-077 | Holy Mapperel 13 mapper 聚合 | **随上游变** | **10′** | TODO |
| ★★★★★ | **T4** | kgmqa-081 | FDS 加载路径 + IRQ 子系统 | 2–4 周 | 13 | TODO |

> **独立性**：每个 Tier 内条目可并行、可单独开分支/PR；跨 Tier 建议顺序执行，
> 但 T1 四项彼此无代码耦合，可按人力切分。

### 2.1 ⑬ 077 的成本是派生的，不按 star 数排期

077 是 47 子 ROM 聚合器，**没有独立根因**。它的真实剩余成本 =
**② 078（MMC1 组）+ ④ 097 / ⑤ 096（FME-7 组）+ L2 MMC3 IRQ 组**的剩余部分。
按 star 数把它压到 ⑪⑫ 之后、押到 ε 阶段，会让一条「已经在做」的活被当成「最后的大块」而漏排。

**排期口径（取代原 ε 阶段无条件收口）**：

1. 077 **不单独立项**，随 ②④⑤ 顺带推进，不额外占窗口；
2. **β（T1）收口时立即重估** 077 剩余子 ROM 数，而非等到 ε；
3. 重估后若剩余子 ROM ≤ 10 且不含 MMC3 IRQ 组 → 提前并入 δ 尾部；
4. 若剩余仍集中在 MMC3 IRQ 组 → 归入 T2 模型级工作的副产品，单独立项；
5. **度量只看子组完成数**，不看 077 总码（47/47 是合取，滞后于任何子项）。

### 2.2 三项重估裁定：⑶ 038 **defer** / ⑷ 097 **abandon** / ⑸ 096 **abandon**

> **裁定口径**（与「DONE / TODO」并列的第三种终态）：
> **defer** = 仍属精度债，留待后续窗口做，不阻塞 release；
> **abandon** = 经查**不是精度债**，不再排期，只保留记录与探针。
> 两者都**不改动矩阵预期值** —— 矩阵仍 **108P / 12F**、grade **B**。

| # | 旧 Tier | **新 Tier** | 旧难度 | **新难度** | 旧风险 | **新风险** | 裁定 |
|---|---|---|---|---|---|---|---|
| ③ **038** | T0 | **T3**（排在 049 之前） | ★★☆☆☆ / 3–5 天 | **★★★★☆ / 1–2 周** | 低 | **中** | **defer** —— 留待以后解决 |
| ④ **097** | T1 | **移出活跃排期** | ★★☆☆☆ / 2–4 天 | **不适用（无缺陷）** | 低 | **无** | **abandon** —— 彻底放弃 |
| ⑸ **096** | T1 | **移出活跃排期** | ★★★☆☆ / 3–7 天 | **不适用（无缺陷）**<br>*（但存在真实 ack bug，见下）* | 中 | **无**（不改动即无风险） | **abandon** —— 彻底放弃，**代码不动** |

#### ⑶ 038：Tier T0 → T3，裁定 defer

**为什么换 Tier。** 原归 T0 是因为「dummy read」听上去像 CPU 时序补丁。
v1.18.3 实测已改判根因：**阻塞点是 FCEUX11 没有 CPU 数据总线模型**
（`Cpu::set_db()` 全仓库零调用点 → `DB` 永不赋值 → `::ANull` 恒返回常量 → open bus 是死的），
这与 ⑨ 049 属同一族（两者都要一个「活的总线值」）。计划本节早就写过
「038 的 S0 有可能顺带给 049 铺路」。故 **T0 → T3，且必须排在 049 之前**。

**T0 由此干净收口。** `056` + `078` 两项 **2/2 DONE**，
包出口达成：矩阵 14F → **12F**，advisory-FAIL 11.7% → **10.0%**。
T0 卡的出口条件「三项均 PASS」按 **2/2** 计，038 移出后 T0 不再有未完成项。

**为什么 defer 而不是 abandon。** 038 确实**不阻塞 release**（它不改任何游戏行为），
但它的价值是**降低 049 的不确定性** —— 049 是 ★★★★★ / 2–4 周的大项，
先花 1–3 天做最小判别实验（S0+S1）能砍掉 049 的一整类不确定性。
**性价比最高的顺序是「038 的 S0+S1 → 再开 049」，而不是直接进 049。**
排期上它不占 δ 的独立窗口，只作为 049 的开工前置。

**风险为什么从「低」升到「中」。** S0 让 `DB` 活起来会改变**每一次未映射读**的返回值，
仓库里至少 6 处 mapper 保护逻辑依赖 `DB` 做 open-bus 混入
（`01-222.cpp:75`、`158B.cpp:57`、`170.cpp:43`、`178.cpp:128/130`、
`235.cpp:68/71`、`cart.cpp:113-120` 的 `CartBROB`），
必须逐项对账，不能只看 038 转绿。

**仍未修复。** 本项自 2026-09-27 调查后**一次都没动过代码**
（唯一一次 `GetABIRD` 改动已按纪律回退）。defer 期间不投入。

#### ⑷ 097：Tier 移出活跃排期，裁定 abandon

**为什么不是「改判」而是「放弃」。** 这一项**不是精度债**。
`fme7ramtest` 是 survey ROM，不实现 blargg 的 `$6000` 结果协议
（静态扫描 32 KiB PRG：对 `$6000` 绝对寻址 **0 处**，对 `$6900` 恰 **2 处**，
与上游 `main.s` 逐字吻合）；harness 判 PASS 需 `probe_addr == 0x00`，
而 `probe_addr` 恒为 `$6000` 且 `tests.json` 无 per-case 覆盖，
改读 `$6900` 得到 `$C0` —— **没有任何 mapper 改动能让它转 PASS**。
同时探针实测证明 **FME-7 WRAM 映射本身是正确的**。
继续投入即无目标投入，故裁定 abandon。

**「abandon 攻关」≠「从矩阵删用例」。** 三种动作后果不同：

| 动作 | 后果 | 本次是否做 |
|---|---|---|
| 改 `known_limit` 为有据表述 | **零机械变化**（`grade.rs::compute_grade` 从不读该字段、也不读 tag） | ✅ **已做**（人工授权 A，2026-09-28） |
| 从 `tests.json` 删除该用例 | `test_set_diff.removed` 非空；矩阵变 119 用例 / 108P-11F；**grade 仍 B**（`new_fails` 只看 `new_test` 桶，删掉的进 `removed`；且 `baseline_supplied=true`） | ❌ 未做 —— 属 `test_set_diff` 评审范围，不在本次授权内 |
| 把 `failure_means` 升为 `blocking` | grade 由 B 降 **D** | ❌ 不考虑 |

**T1 ⑷ 裁定 abandon 后**可攻关三项；**⑸ 096 也于同日裁定 abandon（代码不动）**，
故 **T1 最终只有两项可攻关**（050 / 051），包出口 4/4 → **2/2**。
**⑬ 077 的成本口径随之两次收窄**：原写「077 = ②+④+⑤ 剩余」，
现 **② 已 DONE、④⑸ 均 abandon** → **077 = 仅 L2 MMC3 IRQ 组**。

#### ⑸ 096：Tier 移出活跃排期，裁定 abandon，**代码不动**（2026-09-28）

**裁定方式与 ⑷ 不同：这一项经查「不是精度债」，但同时暴露了一个真实 ack bug，
人工选择只裁定 abandon、保留代码现状。** 下面是两件事，必须分开看。

##### ⑸-1 为什么 abandon：它也是 survey ROM，改 mapper 也不会转 PASS

矩阵报的 `value=0xA2 diag=[0x0F,0x8E,0x00]`，与
**PRG ROM bank 3 offset 0-3 逐字节相同**（`A2 0F 8E 00` 本身就是 6502 指令：
`LDX #imm` / `BPL` / `STX abs` / `BRK`）。成因链：

1. 上游 `fme7acktest/src/fme7.s` 的 `init_fme7` 把 **reg 8 写成 3**；
2. `preg[3] & 0xC0 == 0` → `Sync()` 走 `setprg8(0x6000, 3)` → `$6000-$7FFF` 是 **PRG ROM**；
3. 按硬件这**是正确的**：reg 8 的 bit7 是 6264 的 +CE、bit6 是 RAM/ROM select，
   `$03` 两位都是 0 → 选中 PRG ROM。**不是模拟器的 bug。**
4. 静态扫描 32 KiB PRG：对 `$6000` 的绝对寻址指令 **0 处**。
   测试结果写在零页 `test_results`（8 字节），只画到屏幕上。

harness 判 PASS 仍需 `probe_addr == 0x00`（`blargg.rs:316`），而 `probe_addr` 恒为 `$6000`。
**与 ⑷ 同理：没有任何 mapper 改动能让 kgmqa-096 转 PASS。**
至此 T1 的两个 FME-7 用例（⑷ 097、⑸ 096）**都是 survey ROM**。

##### ⑸-2 但确实存在一个真实精度 bug：**故意不修，留档**

上游测试作者 README 直接给了各实现的实测对照表：

| 写 $0D | PowerPak / Everdrive（**硬件**） | FCEUX11 现状 | 判定 |
|---|---|---|---|
| `$00` | Acked | Acked | ✓ |
| `$01` | **No ack** | **Acked** | **✗** |
| `$80` | Acked | Acked | ✓ |
| `$81` | **No ack** | **Acked** | **✗** |
| `$0E=$FF` | No ack | No ack | ✓ |
| `$0F=$FF` | No ack | No ack | ✓ |

**2 / 6 错。** 现状 `src/boards/69.cpp`：

```c
case 0xD: IRQa = V; X6502_IRQEnd(FCEU_IQEXT); break;   // 无条件 ack
```

**硬件规则：bit 0 == 0 才 ack。** 三方独立一致：PowerPak、Everdrive（两个硬件实现）
与 Nintendulator（Quietust 按 Oliveira 的实测改的）。

> **⚠️ nesdev wiki 现在写着「All writes to this register acknowledge an active IRQ」——
> 那是照 FCEUX 的行为改的，与硬件实测相反。以硬件为准。**
> 该页脚注：`Test performed in 2015 by Oliveira using IRQ acknowledge test ROM on NESdev BBS`。
> 另注：wiki 把 bit 7 描述为「IRQ Counter Enable，0 = Disable Counter Decrement」，
> 但这与 PowerPak 实测**自相矛盾**（若 bit7=0 真停计数器，PowerPak 上 `$0D=$01`
> 就不可能出现第二次 IRQ，与实测 "No ack" 冲突）。**故 bit 7 语义无证据，本项不动。**

**若将来要修，最小改动是 1 行**（只门控 ack，不碰使能位）：

```c
case 0xD: IRQa = V; if (!(V & 1)) X6502_IRQEnd(FCEU_IQEXT); break;
```

**为什么这轮不修（人工裁定）**：
1. 该 survey ROM 无法报 pass/fail → **改完验证不了**。按 038 纪律
   （「无法验证的核心时序改动不合项目纪律」），本应回退。
2. 修它需要一个**新的 ctest 用例**（用 `AWrite[]` 驱动 `$8000`/`$A000`、
   读 `X.IRQlow & FCEU_IQEXT` 断言 ack 行为）—— 基础设施齐备
   （`bus_test.cpp` 已证 `AWrite[]` 可用、`mapper_reset_test.cpp` 已证
   `LoadGame` 可用、`cpu_test.cpp` 已证 `X` 状态可读），
   但那是**新增测试基建**，属独立工作项，不在本轮范围。
3. `IRQa = V` 用整字节当使能位，按规范应是 bit 0 = IRQ enable、bit 7 = counter enable。
   但**改它会动到真实 FME-7 游戏（Batman: Return of the Joker / Gimmick!）的 IRQ 时序**，
   而 bit 7 语义本身有争议（见上）—— 典型的不可验证改动。

**留档结论**：ack bug **已知、已定位、有三方硬件证据、一行可修**，
但在本轮**明确选择不修**。它影响真实 FME-7 游戏的 IRQ 应答行为，
**不体现在任何 F11QA 指标上**（096 无论修不修都是 FAIL）。
将来若有人碰 mapper 69，请先读本节。

---

## 三、L1 主清单 — 14 项独立分析（易 → 难）

> 每项格式固定：**现象 → 根因 → 攻关步骤 → 验收 → 风险**。
> 状态栏供后续勾选；未开始默认 `TODO`。

---

### ① T0 · kgmqa-056-instr-timing-blargg　`DONE`　`8010a3f`

> **已交付**（2026-09-27，v1.18.1）。CI run **98267393461** 实测：
> kgmqa-056 `exit=0 / 0x00 / PASS`；L2 `instr_timing` / `instr_timing_v2_1` FAIL→PASS
> （`instr_timing_v2_2` 保持 PASS）；blargg 全量 **145/32 → 147/30**；
> 矩阵 **106P/14F → 107P/13F**；`pass_to_fail=0` / `fail_to_pass=1`；
> 内部逻辑检测 42P/0F 不变；`nestest` / `cpu_dummy_reads` 哨兵 PASS。
>
> **实际改动**（与原计划的两处偏差，均已验证）：
> - `CycTable[0xE2]` 3→2 —— 非法 NOP imm，与计划一致；
> - `0xBB` 计划写的是「周期 4→5」，**实际根因更深一层**：该 opcode 用的是
>   `RMW_ABY`（带假写回），既周期语义错（缺 `GetABIRD` 的跨页 +1）又有多余写回。
>   改为 `LD_ABY`（只读）后一次解决两个问题；
> - 附带修正 `tests/tests.json` 的 `kgmqa-056 --frames` 300→3000 ——
>   原值与 manifest 不符，300 帧跑不完测试、报「仍在运行」的假码，**掩盖了真实周期表结果**。
>   即：只改周期表不足以让该用例转 PASS，帧数是并行的第二个必要条件。
>
> **对计划的影响**：★☆☆☆☆ 的真实成本是「周期表 2 处 + opcode 语义 1 处 + manifest 帧数 1 处」。
> 后续 T0-③（038）若也卡在「明明改了却不转 PASS」，先查 `tests/tests.json` 的 `--frames` 与 manifest 是否一致。

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
→ **已达成**：`fail_to_pass=2`（L2 两项）、`pass_to_fail=0`；blargg 147/30。

**风险**：低。周期表变更可能影响帧对齐类 golden（若 runner 按帧截取）；出现 `pass_to_fail` 则回滚并查 frame budget。
→ **未触发**：`nestest` / `instr_test_v5` / `cpu_dummy_reads` 均 PASS，goldens 无需重生成。

#### ① 实际执行记录（v1.18.1，commit `8010a3f`）

CI run **98267393461** 实测：

```
kgmqa-056-instr-timing-blargg    exit=0  value=0x00  status=PASS
L2 instr_timing.nes              FAIL -> PASS
L2 instr_timing_v2_1.nes         FAIL -> PASS
L2 instr_timing_v2_2.nes         PASS (unchanged)
blargg 全量 177 ROM              145/32 -> 147/30
迁移矩阵                          106P/14F -> 107P/13F
pass_to_fail / fail_to_pass       0 / 2
内部逻辑检测                       42P/0F (unchanged)
nestest.nes (CPU 金标)             PASS
cpu_dummy_reads.nes (哨兵)        PASS
grade                             B (unchanged)
advisory-FAIL 占比                11.7% -> 10.8%
```

**实际改动 vs 原计划的偏差（两条，都已验证，勿照原计划重做）**：

1. **`0xBB` 的根因比计划写得更深一层。**原计划判断是「跨页少 1 个周期，补 +1 即可」；
   实测该 opcode 走的是 `RMW_ABY` —— 读-改-写宏，**既周期语义错（缺 `GetABIRD` 的跨页 +1）
   又带一次多余写回**。改为 `LD_ABY`（只读）后两个问题一并解决。
   `src/ops_table.inc` 由 `scripts/generate_x6502_dispatch.py` 重新生成，不要手改。
2. **周期表改对了，用例仍不会转 PASS —— 帧数是第二个必要条件。**
   `tests/tests.json` 里 `kgmqa-056` 的 `--frames` 原为 `300`，而 manifest 规定 `3000`。
   300 帧跑不完整个测试，runner 报「仍在运行」的假错误码，**把真实的周期表结果掩盖了**。已改为 3000。

> **对后续项的可复用教训**：若某项「代码明显改对了但矩阵仍 FAIL」，先核对 `tests/tests.json` 的
> `--frames` 与 `manifest` 是否一致 —— 这是零成本的一步，且症状（假错误码）与真实失败无法区分。

---

### ② T0 · kgmqa-078-serom-lidnariq　`DONE`　`v1.18.2`

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

#### ② 实际执行记录（v1.18.2）

**原计划的根因判断是错的。**计划写「MMC1 SEROM/SHROM bank 宽度约束未建模」——
实测后发现**有两个独立缺陷，且第一个完全掩盖了第二个**。

##### 缺陷 1（真正让矩阵变红的那一个）：`$6000` 根本没被映射

ROM 头部（NES 2.0）：`PRG=32K CHR=8K mapper=1 submapper=5 **PRG-RAM=0**`。
`DetectMMC1WRAMSize()` 只对 **NES 2.0** 板信头部字段，
所以本板拿到 **0 KB WRAM**；而**同一颗 ROM 若写成 iNES 1.0**，
该函数会落到函数开头的 `int ws = 8;` 默认值、拿到 8 KB。

后果：`$6000` 无读/写 handler，blargg 的结果寄存器写不进去，
runner 读回一个常量 —— 这正是 `value=0xC3 diag=[0xC3,0xC3,0xC3]` 的由来。
**它与 mapper 行为无关**：无论 MMC1 怎么译码，矩阵都报同一个 0xC3。

**证明**（零成本 header patch，无需改代码）：把 byte10 低半字节改成 7
（= 8 KB PRG-RAM），其余字节不动 ——

```
ORIGINAL  (PRG-RAM=0) : exit=1  value=0xC3  diag=[0xC3,0xC3,0xC3]  FAIL
EXPERIMENT(PRG-RAM=8K): exit=0  value=0x00  diag=[0x00,0x00,0x00]  PASS
```

##### 缺陷 2：submapper 5「Fixed PRG」语义缺失（规范正确，但本 ROM 测不出来）

ROM 自带的字符串就是答案：

```
Submapper 5 has been allocated for the SEROM, SHROM, and SH1ROM PCBs,
which do not support PRG banking at all.
```

NES 2.0 规范：mapper 001 submapper 5 = Fixed PRG，
**PRG ROM A14 直接接 CPU A14，不经 MMC1**。而 `bmap[]` 只按 mapper 号索引，
submapper 从来到不了 mapper 层。

**诚实结论：这一项不是让 078 转 PASS 的原因。**A/B 实测——
同一 ROM 内容、同样 8 KB PRG-RAM，只改 submapper 字节：

| submapper | isFixedPRG | 结果 |
|---|---|---|
| 0（plain MMC1） | 否 | **PASS** |
| 5（Fixed PRG） | 是 | **PASS** |

两者都 PASS。原因：PRG 只有 32 KB 时 `PRGmask16=1`，
32K 模式下 `(prg_reg & ~1) & 1` 恒为 0，与固定映射**数值上完全相同**。
探针可见 fixed-PRG 在 9 次同步中确实改掉了 2 次映射，
但本 ROM 无法据此判负。保留该实现是因为它符合规范、且改动面为零；
**不要用「重跑这个测试」去验证它** —— 验证要换一个 >32K PRG 的板。

##### 实际改动

仅 `src/boards/mmc1.cpp`：

1. `Mapper1_Init()`：NES 2.0 + submapper 5 且头部 PRG-RAM 为空时，
   `ws` 回退到 8 KB（**承载项**）。submapper 5 不约束 PRG RAM，
   真实 SEROM/SHROM 板（Dr. Mario / Tetris / Boulder Dash…）也都带 8K。
2. 新增 `static int isFixedPRG` + `MMC1PRG()` 开头的固定分支：
   恒 `setprg16(0x8000, 0)` / `setprg16(0xC000, PRGmask16[0])`，
   忽略 `DRegs[0]` 模式位、`DRegs[3]` bank 寄存器、`DRegs[1]` bit4 别名。
   寄存器仍正常锁存，CHR / mirroring / savestate 形状不变。
3. 新增 env-gated 探针 `FCEUX11_MMC1_PROBE=1`（写 **stderr**，
   因为 `FCEU_printf` → `FCEUD_Message` → driver 回调，
   headless runner 从不安装 message 回调，探针会被静默吞掉）。

**影响面**：仅 mapper 001 submapper 5 且 PRG-RAM 字段为空的镜像。
整个 fixture 集里**只有 `serom.nes` 一个**。
`GenMMC1Init` 用 `&=` 与 loader 按真实尺寸算出的 mask 取交，只能收窄不能放宽，
因此不存在越界读。

##### 验证（全部实测）

```
serom.nes 单 ROM      exit=0  value=0x00  diag=[0x00,0x00,0x00]  PASS
ctest 内部逻辑检测     34/34 PASS（含 mapper_byte_diff、golden_savestate）
blargg 全量 177 ROM    147P / 30F（与 ① 后基线完全一致，零回归）
迁移矩阵               107P/13F -> 108P/12F   grade B 不变
                       fail_to_pass=1 (kgmqa-078)  pass_to_pass=107
                       pass_to_fail=0            new_test=0
                       Oracle A 42P/0F          Oracle B 66P/12F
advisory-FAIL 占比      10.8% -> 10.0%（cap 15%）
f11qa_baseline_frozen  kgmqa-078  false -> true（同 PR 更新，见 §7.1）
```

**无金标重生成**：`mapper_byte_diff` 与 `golden_savestate` 均通过，
`Mmc1Cart::save_mapper_state()` 未加入 `isFixedPRG`
（它由 ROM 头确定性推导，入 savestate 反而会污染跨镜像比对）。

> **可复用教训**：`value` 是一个**恒定**的字节、且 256 字节 diag 区全是同一个值时，
> 先怀疑**探针寄存器本身没被映射**（`$6000` 在无 RAM 的板上不可写），
> 而不是去改被测逻辑。改 ROM 头部一个字节做 A/B 是零成本分辨手段。

---

### ③ T3 · kgmqa-038-instr-misc-blargg　**defer**（**Tier 已由 T0 改判为 T3；2026-09-27 调查后未修复，见下 + §2.2**）

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

#### ③ 调查记录（2026-09-27，v1.18.3 任务）—— **未修复，根因改判**

本轮没有交出修复。计划里的一行根因（"`LDA abs,x` dummy read 不完整"）经实测**不成立**，
以下全部是量出来的，不是推断。

##### 复现（矩阵同一路径）

| ROM | mapper | 结果 |
|---|---|---|
| `instr_misc.nes`（组合） | **1 / MMC1 / 64K PRG** | FAIL `0x01` |
| `instr_misc_03_dummy.nes` | 0 / NROM / 32K PRG | FAIL `0x03` |
| `instr_misc_03-dummy_reads.nes` | 0 / NROM | FAIL `0x03` |
| `instr_misc_01_abs_x.nes` / `_01-abs_x_wrap.nes` | 0 | PASS |
| `instr_misc_04_dummy_apu.nes` / `_04-dummy_reads_apu.nes` | 0 | PASS |

`--frames 600` 与 `3000` 结果一致（**本项不是 ① 那种帧数陷阱**）。

##### 计划需要修正的三处

1. **`02-branch` / `02_branch_wrap` 两个 ROM 在 fixture 集里根本不存在**
   （`_mirror_snapshot` 里也没有）。"四项子测试"只有 3 组可本地验证。
2. **组合 ROM 是 mapper 1（MMC1），子项是 mapper 0（NROM）**。
   组合 ROM 靠 MMC1 在子测试间切 bank，所以组合的失败并不等价于"03 的同一份代码"。
3. **它查的不是一件事，而是 8 种指令形式**。ROM 自带字符串：

```
03-dummy_reads
Test requires $2002 mirroring every 8 bytes to $3FFA
LDA abs,x     STA abs,x     LDA (z),y     STA (z),y
LDA (z,x)     STA (z,x)     ROL abs       ROL abs,x
```

##### 前提条件已满足，不是阻塞点

`ppu.cpp:1185` 已经在 `for (x = 0x2000; x < 0x4000; x += 8)` 里把每个 8 字节的 +2 偏移
都指向 `A2002`，即 `$2002 … $3FFA` 全部命中。ROM 要求的 "$2002 mirroring" 早就有了。

##### CPU 里确实存在 dummy read 不对称 —— 但不是阻塞点

| 形式 | 宏 | dummy read |
|---|---|---|
| `LDA abs,x` / `abs,y` | `GetABIRD` | **仅跨页时** |
| `STA abs,x` / `ROL abs,x` | `GetABIWR` | 无条件 ✓ 已正确 |
| `LDA (z),y` | `GetIYRD` | **仅跨页时**（同样的 bug） |
| `LDA (z,x)` / `STA (z,x)` | `GetIX` | **完全没有**，也没有跨页周期 |

本轮把 `GetABIRD` 改成硬件行为（未修正地址无条件读，周期数不变），
**测试结果没有任何变化**（仍 `0x03`）。探针（env-gated、400 事件预算）实测到 33 次
abs 索引读，**全部 no-cross**，此时未修正地址 == 真实地址，多读一次放到总线上的值完全相同
→ 观察上为零。该改动**已回退**（无法验证的核心时序改动不合项目纪律）。

##### 真正的阻塞点：FCEUX11 没有 CPU 数据总线模型

- `Cpu::set_db()` 在 `cpu.h:61` 声明、`cpu.cpp:51` 定义，
  **全仓库零调用点**；`DB` 字段从未被赋值。
- `::ANull`（`fceu.cpp:264`）未映射读返回 `DB`，因此恒为常量。
- `PPUGenLatch` 只由 PPU 寄存器写、`$2002` / `$2007` / `$2000` 读更新
  （`ppu.cpp:642/778/786/874/906` 等），**不跟随通用 CPU 总线访问**。

一个"观察 dummy read 往总线上放了什么值"的测试，在当前实现里**没有任何东西可观察** ——
加多少 dummy read 都不可能让它通过。

这也解释了 **04_dummy_reads_apu 为什么 PASS**：它经 APU 自己的寄存器路径观察 dummy read，
那条路径 FCEUX 建模了；03 走 $2000–$3FFF 镜像路径，需要裸总线。

##### 结论与后续

**（本段结论已被下一节部分修正：数据总线锁存这一层仍需做，但「与 049 同族、需 PPU 耦合」的说法过重了。）**
并让该值可经 $2000–$3FFF 镜像路径读出。这是**结构性总线改动**，
与 ⑨ `kgmqa-049`（PPU 读缓冲，★★★★★，2–4 周）同一族，**不是周期表/语义补丁**。

因此本项的 **★★☆☆☆ / 3–5 天 / 风险低** 预估**明显偏低**，建议按 ⑨ 同量级重估。
在数据总线模型落地前，038 属于"已定位阻塞点、待立项"，不是"照计划改两行即可"。

#### ③ 难度重估（2026-09-27，v1.18.3 任务后）

##### 结论：★★☆☆☆ / 3–5 天 / 风险低 → **★★★★☆ / 1–2 周 / 风险中**

**并且上一节末尾那句「需要实现 CPU 数据总线锁存…与 049 同族」我下调了一档。**
硬件参考（nesquick 的 2C02 描述，与 FCEUX 自身实现一致）确认：

> The PPU has an internal data bus... Writing any value to any PPU port, even to
> read-only PPUSTATUS, will fill this latch. Reading any readable port
> (PPUSTATUS, OAMDATA or PPUDATA) also fills the latch with the bits read.
> Reading a nominally "write-only" register returns the latch's current value,
> as do the unused bits of PPUSTATUS.

即 `$2002` 低位回的是 **PPU latch**，由**端口访问**填充 —— 这正是 FCEUX 的
`PPUGenLatch` 现有模型。**所以「要做 CPU↔PPU 总线耦合」不是已证实的阻塞点**，
那是上一轮我推过头了，这里收回。

##### 已确证 / 未确证，分清楚

| 结论 | 强度 |
|---|---|
| `$2002` 每 8 字节镜像到 `$3FFA` 的前提**已满足**（`ppu.cpp:1185`） | **实测确证** |
| `GetABIRD` 改为硬件行为后测试**毫无变化**；33 次 abs 索引读全不跨页、dummy 地址 == 真实地址 | **实测确证** |
| `Cpu::set_db()` **全仓库零调用点**，`DB` 永不赋值，`::ANull`（`fceu.cpp:264`）因此返回常量 | **实测确证** |
| 主假设：测试经**未映射读的 open bus**（如 `lda $4018` 回读总线上一个值）观察 dummy read，因此需要一个活的 `DB` | **未验证** |
| 备选假设：经 `$2002` 的 PPU latch 路径观察，需要把 CPU 总线耦合进 latch | **未验证，且被上一条硬件描述削弱** |

**我特意没有把主假设写成结论。**区分「测出来的」和「推出来的」是这一节的重点 ——
上一轮就是因为把推理当结论，才需要事后收回。

##### 工作量分解（两条假设的公共前半段是确定的）

| 阶段 | 内容 | 估时 |
|---|---|---|
| S0 | 让 `DB` 活起来：每次总线访问记录它放到总线上的值，`::ANull` 回读它 | 1–3 天 |
| S1 | 跑 `03_dummy`，让**测试自己**分辨走的是哪条通道 | 0.5 天 |
| S2 | 按 S1 的结果补齐 8 种指令形式的 dummy read 时序/取值 | 3–7 天 |
| S3 | 全量对账：ctest 34 项 + blargg 177 ROM + 矩阵 `pass_to_fail=0` | 1–2 天 |

S0+S1 是两种假设的公共部分，也是「最小可判别实验」—— 一次构建就能确认主假设死活。
**建议先只做 S0+S1**（1–3 天），拿到结论再决定是否投 S2。这是本重估最主要的可执行建议。

##### 为什么不是 ★★★★★

比 049（`$2007` 读缓冲状态机 + open-bus 衰减，2–4 周）轻：038 要的是**一个总线锁存**，
不涉及 PPU 读缓冲的状态机与逐位衰减。两者共用「活的总线值」这一层，
所以 **038 的 S0 有可能顺带给 049 铺路**（先做 038 可降低 049 的不确定性）。

##### 风险为什么从「低」升到「中」

S0 会改变**每一次未映射读的返回值**。仓库里至少 6 个 mapper 的保护逻辑依赖 `DB` 做
open-bus 混入（`01-222.cpp:75`、`158B.cpp:57`、`170.cpp:43`、`178.cpp:128/130`、
235.cpp:68/71`、`cart.cpp:113-120` 的 `CartBROB`），这些行为会随之变化，
可能影响既有金标与 mapper byte-diff。必须逐项对账，不能只看 038 转绿。

##### 排期建议 → **已被 §2.2 裁定取代（2026-09-28）**

原文此处是一个 A/B 选择题（「先做 T1 的 ④ 097/⑤ 096 再做 038 的 S0+S1」
vs「直接做 038 的 S0+S1」）。现在两个前提都变了，该选择已作废：

- **④ 097 裁定 abandon**（§2.2）—— 不再是可攻的单点，A 方案失去意义；
- **038 的 Tier 由 T0 改为 T3**，且**排在 ⑨ 049 之前**；
- **T0 随之收口为 2/2 DONE**（`056` + `078`），不再等 038。

**现行结论**：038 **defer** —— 不占独立窗口，不阻塞 release；
真正值得做的动作是 **S0+S1（1–3 天）最小判别实验**，且它的价值在于
**给 ⑨ 049 铺路**（砍掉 049 的一整类不确定性），而不是为了让 038 自己转绿。
**性价比最高的顺序：038-S0+S1 → 再开 049。**

---

### ④ （已移出活跃排期）· kgmqa-097-fme7ramtest-tepples　**abandon**（**2026-09-27 调查后根因改判：不是 mapper 缺陷；2026-09-28 裁定彻底放弃，见下 + §2.2**）

| 字段 | 值 |
|---|---|
| 难度 | ★★☆☆☆ → **不适用（非 mapper 缺陷）** |
| 层 / 通道 | Boards / B |
| ROM | `tepples/fme7/fme7ramtest.nes` |
| 错误码 | `0x01` |
| known_limit | FME-7 WRAM mapping edge　**← 已过时，见下** |

**现象（计划原文的判断）**：`$6000` 窗口 WRAM 映射边界失败（diag `[02,04,08]` 系列为分项偏移）。

**原根因**：FME-7（Sunsoft 5B）WRAM 映射寄存器边界未完整建模（bank 号 / 使能位 / 窗口）。

**攻关步骤（原计划）**

1. 读 FME-7 mapper 实现的 WRAM 映射寄存器写逻辑。
2. 对照 tepples 测试期望补全边界（含 bank 使能关闭时的总线行为）。
3. 与 ⑤ 同文件改动，可合并分支但 **分 commit**。

**验收**：kgmqa-097 PASS。

**风险**：低。

#### ④ 调查记录（2026-09-27，v1.18.3 任务）—— **未修复；根因改判**

本轮没有交出修复，因为**没有缺陷可修**。以下全部是量出来的，不是推断。

##### 结论先行：`$6000 = 0x01` 是 ROM 自己写的**测试图样字节**，不是结果码

`fme7ramtest` 是一条 **survey ROM（勘测式测试）**，**不使用 blargg 的 `$6000` 结果协议**。
静态扫描 fixture（`tests/fixtures/fme7ramtest.nes`，40976 B）的 32 KiB PRG：

| 目标 | 绝对寻址指令数 | 指令（PRG 相对偏移） |
|---|---|---|
| `$6000` | **0** | —— |
| `$6900` | **2** | `PRG+06138 STA abs $6900` / `PRG+06146 LDA abs $6900` |

与上游 `pinobatch/little-things-nes/fme7ramtest/src/main.s` 逐字吻合：
`check_bank_numbers` 用 `sta $6900` 暂存 bank tag、用 `lda $6900` 读回，
**全文件没有任何 `sta $6000`**。结果文字（"No WRAM found at $6000-$7FFF" /
"WRAM banks at $6000"）只画在屏幕上。

`$6000` 里的值来自 `check_for_wram` 的 9 字节 RAM 图样：
`lda #$C0 / sta $A000 / asl a / asl a` 后 `rol a` 循环写入，首字节恒为 **`0x01`**。
实测 `value=0x01 diag=[0x02,0x04,0x08]` 正是该图样的前 4 字节
（`01 02 04 08 10 20 40 80 00`）—— 一一对应，不是巧合。

harness 侧判定只有一条（`src/rust/crates/f11qa/src/runner/blargg.rs:316`）：
`let passed = value == 0x00;`，`probe_addr` 恒为 `0x6000`
（`tests.json` 无 per-case probe 地址字段）。
**换任何地址都不成立**：改读 `$6900` 得到的是 `0xC0`（bank 0 的 tag），也不是 `0x00`。

> **因此 kgmqa-097 在当前 harness 下结构上不可能转 PASS。**
> 这不是 mapper 的问题，也不是「精度缺口」。

##### FME-7 WRAM 映射本身是对的（探针实测，非推理）

新增 env-gated 探针 `FCEUX11_FME7_PROBE=1`（`src/boards/69.cpp`，写 stderr——
`FCEU_printf` 走 driver 回调，headless runner 从不安装，会被静默吞掉）：

```
M69_INIT ines2=1 wram=32768 battery_wram=0 -> WRAMSIZE=32768 submapper=0 CRC32=9F5C1791
```

`WRAMSIZE=32768` 是 **ROM 有意声明**的，不是 loader 走偏：
header `4E 45 53 1A 02 01 50 48 00 00 09 00`，
FCEUX 遗留 `FceuInesHeader` 布局把 `ram_size` 放在 **offset 10**，
`64 << (0x09 & 0x0F) = 32768`。这与上游 bug 731 里 tepples 报的
「FCEUX r3218 识别出 NES 2.0 头：Total WRAM size: 32768」完全相同 ——
ROM 头部就是照 FCEUX 的约定造的，cah4e3 在 r3220 让 mapper 认这个值。

ROM 读回每个 bank 的 tag（`$6900`，`check_bank_numbers` 的 check 循环，X 从 15 递减）：

```
reg8=CF→C3  CE→C2  CD→C1  CC→C0  CB→C3  CA→C2  C9→C1  C8→C0
reg8=C7→C3  C6→C2  C5→C1  C4→C0  C3→C3  C2→C2  C1→C1  C0→C0
```

即屏幕 hexdump `C0: C0 C1 C2 C3 C0 C1 C2 C3` / `C8: C0 C1 C2 C3 C0 C1 C2 C3`，
与上游 README「With 62256 (32Kx8)」的期望输出**逐字节相同**
（README 原文：`00: C0 C1 C2 C3 C0 C1 C2 C3` / `08: C0 C1 C2 C3 C0 C1 C2 C3`）。

另外 `check_for_wram` 能跑完 `check_bank_numbers`（探针抓到 16+16 次 reg8 写）
即证明 8 KiB 图样的写入-回读校验**已通过** —— 8 KiB 窗口映射本身没问题。

##### 顺带发现一处真实不一致：**本 ROM 测不出，故不改**

`src/boards/69.cpp` 的 reg 8 模式表（上游 `main.s` 注释给定）：
`00-3F: ROM; 40-7F: open bus; C0-FF: RAM`，bit6 = RAM/ROM select，bit7 = WRAM +CE。

| reg8 | `Sync()` 映射 | `M69WRAMRead` | `M69WRAMWrite` | 是否自洽 |
|---|---|---|---|---|
| `00-3F` | PRG ROM | `CartBR`（ROM） | 丢弃 | ✓ |
| `40-7F` | **PRG ROM** | 返回 `DB`（open bus） | 丢弃 | **✗ 读/映射打架** |
| `80-BF` | PRG ROM | `CartBR`（ROM） | 丢弃 | ✓ |
| `C0-FF` | WRAM banked | `CartBR`（RAM） | `CartBW` | ✓ |

`40-7F` 档 `Sync()` 把 PRG ROM 映到 `$6000-$7FFF`，而读 handler 声称是 open bus；
读 handler 优先，所以实际可观察行为是 open bus（与硬件一致），
但映射表自相矛盾。**按 038 的纪律（无法验证的核心时序改动不合项目纪律）本轮不改** ——
`main.s` 自己写明「This ROM does not currently test open bus behavior」，
没有 fixture 能证伪；且本项目 open bus 本身就是死的（见 ③：`set_db()` 零调用点）。

##### 处置：**A 已授权执行（2026-09-28）**

人工在 2026-09-28 选定 **A：记为有据 known_limit**。

已改 `tests/tests.json` 的 `kgmqa-097-fme7ramtest-tepples` 条目：
`known_limit` 换成带证据的完整表述，`provenance` 追加
`known-limit (2026-09-28 human-authorized): survey ROM does not implement the
$6000 result protocol ... NOT an accuracy debt`。
`failure_means` 仍为 `advisory`，**未加 `known-limit` tag**（`grade.rs` 不读 tag，
加了只会误导），用例数仍 120。

> **`known_limit` 字段是纯文档，不参与任何机械判定。**
> `report/grade.rs::compute_grade` 只读 `failure_means`、`pass_to_fail`、
> `new_test`、`test_set_diff` 是否存在、`summary.failed/skipped`；
> **从不读 `known_limit` 字符串，也从不读 tag**。
> 因此本次改动：矩阵仍 **108P / 12F**、grade 仍 **B**、
> `pass_to_fail = 0`、`fail_to_pass = 0`、`new_test = 0`。
> `test_set_diff` 评审也未被触发 —— 没有增删任何 `test_id`，
> `new_test` 桶按构造为空（`report/matrix.rs:257-259`）。
> 这次改动的价值是**语义**：把「不是 mapper 缺陷」写进机器可读的清单，
> 避免下一个会话再按精度缺口去查一遍。

| 备选 | 内容 | 状态 |
|---|---|---|
| **A（已选）** | 有据 known_limit，写入 `tests.json` | ✅ 2026-09-28 授权执行 |
| B | 仅 backlog 标注，不动 `tests.json` | 未选 |
| C | 更换为能报告 pass/fail 的 FME-7 用例 | 未选（上游该族只有 survey 与 IRQ = ⑤） |

---

### ⑤ （已移出活跃排期）· kgmqa-096-fme7acktest-tepples　**abandon**（**2026-09-28 裁定：survey ROM + 真实 ack bug 故意不修**，见 §2.2 ⑸）

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

| Phase | 窗口 | 包含 | 出口指标 | 进度 |
|---|---|---|---|---|
| **α 指令时序与板级约束** | ~~1–2 周~~→~~3–4 周~~ | ① 056 → ② 078 | ~~14F → 11F~~ | **2/2 DONE · 14F → 12F · advisory 11.7%→10.0%** ✅ 收口（③ 038 已按 §2.2 移出本阶段） |
| **β Mapper/DMA** | 2 周 | ⑥ 050 → ⑦ 051 | 12F → **10F**；L2 中 DMA 组观察 | 未开始（**仅 2 项可攻关**，④ 097 与 ⑤ 096 均已 abandon） |
| **γ 中断模型** | 1–2 月 | ⑧ 037（+ L2 cpu_int_*） | 10F → **9F**；L2 中断组清零 | 未开始 |
| **δ 长周期精度项** | 1–2 季 | **③ 038（S0+S1 前置）**、⑨ 049、⑩ 099、⑪ 107、⑫ 108 | 每项独立里程碑；不强制同窗完成 | 未开始（**038 为 defer，见 §2.2**） |
| **ε 收敛** | 随 β/δ | ⑬ 077（成本已收窄为**仅 L2 MMC3 IRQ 组**，见 §2.2）、⑭ 081 | 9F → **0F** 或有据 known_limit | 未开始 |
| **ζ 评级** | 末 | L2 eventually_pass 清零、advisory<5% | grade **A** 评审 | 未开始 |

> **Phase 出口数字一律以实际起点为准，不以计划值倒推。**
> 原表曾按 ① 达成的 13F 起算；2026-09-28 §2.2 裁定后，
> α 收口为 2/2（14F → **12F**），β 出口 **12F → 9F**，γ 出口 **9F → 8F**。
> ④ 097 已 abandon（不再计入 β 的可攻关项），③ 038 已 defer 并移入 δ。

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
| `fail_to_pass > 0` | 矩阵迁移 | **不是风险信号**，是精度进展（见 §7.1） |
| 内部逻辑检测红灯 | ctest | 同上，blocking |
| advisory 超 15% | R4 gate | 门禁失败；禁止用改 `failure_means` 绕过 |
| golden 抖动 | frame/savestate hash 变 | 停；确认是否“有意变更”；否则回滚 |
| L2 静默回退 | 145/32 数字下降 | 对照 `blargg_full_baseline.json` 查漂移 |
| 聚合项假进展 | 077 仍 FAIL | 按 mapper 组核对子 ROM，不看总码 |

### 7.1 R4 gate 的方向：`fail_to_pass` 是进展，不是违规

**已修（`b6fe92a`）**：R4 gate 原先守 `fail_to_pass != 0`，等于「**修好任何一项残留 FAIL 都会被门禁拦红**」——
与本计划的目的正好相反。首次应用就撞上了：① 056 修完后 run 98267393461 报
`fail_to_pass=1`、全绿之外唯此一项红。

现语义（`.github/workflows/f11qa.yml`）：

| 指标 | 含义 | 门禁 |
|---|---|---|
| `pass_to_fail` | 基线 PASS → 当前 FAIL，**回归** | **硬门禁，非 0 即红** |
| `fail_to_pass` | 基线 FAIL → 当前 PASS，**精度进展** | 信息行，不阻断 |
| `new_test` | 清单扩项 | 走 `test_set_diff` 评审，非门禁 |

**为什么可以不再重复守 `fail_to_pass`**：防扩项作弊已由 runner 结构性保证 ——
`report/matrix.rs:257-259` 把基线中不存在的 `test_id` 路由进 `new_test` 桶，
构造上就落不到 `fail_to_pass` 里。

**对剩余 13 项的意义**：每修好一项，`fail_to_pass` 必然 +1，CI 不再拦。
但 `tests/fixtures/f11qa_baseline_frozen.json` 仍需**随每项修复同步更新**（把该 id 由 false 改 true），
否则下一项的 `fail_to_pass` 会把已修项重复计入。**每清一项，冻结基线必须同 PR 更新** —— 这已是一条隐含纪律，补在此处。

---

## 八、状态总表（执行时勾选）

| # | kgmqa | 难度 | Tier | 状态 | PR / commit | 矩阵结果 |
|---|---|---|---|---|---|---|
| 1 | 056-instr-timing | ★☆☆☆☆ | T0 | **DONE** | `8010a3f`（run 98267393461） | **PASS** |
| 2 | 078-serom | ★★☆☆☆ | T0 | **DONE** | v1.18.2（MMC1 submapper-5） | **PASS** |
| 3 | 038-instr-misc | ★★★★☆（★2→★4） | **T3**（原 T0） | **defer** —— 风险低→中，1–2 周，不占独立窗口；S0+S1 为 049 铺路，见 §2.2 | | FAIL（defer） |
| 4 | 097-fme7ram | —（无缺陷） | **移出**（原 T1） | **abandon** —— 2026-09-28 裁定彻底放弃；非 mapper 缺陷，survey ROM 无 `$6000` 协议，见 §2.2 / §三④ | | FAIL（结构性，abandon） |
| 5 | 096-fme7ack | —（无缺陷） | **移出**（原 T1） | **abandon**（2026-09-28 裁定，**代码不动**）—— survey ROM，`$6000` = PRG ROM bank 3 字节；**留有真实 ack bug 明确不修**，见 §2.2 ⑸ | | FAIL（结构性，abandon） |
| 6 | 050-dummy-writes | ★★★☆☆ | T1 | TODO | | FAIL |
| 7 | 051-exec-space | ★★★☆☆ | T1 | TODO | | FAIL |
| 8 | 037-cpu-int-2 | ★★★★☆ | T2 | TODO | | FAIL |
| 9 | 049-ppu-read-buffer | ★★★★★ | T3 | TODO | | FAIL |
| 10 | 099-240pee | ★★★★★ | T3 | TODO | | FAIL |
| 11 | 107-mset | ★★★★★ | T3 | TODO | | FAIL |
| 12 | 108-mict | ★★★★★ | T3 | TODO | | FAIL |
| 13 | 077-holy-mapperel | ★★★★★（**派生**） | T4 | TODO | | FAIL |
| 14 | 081-fds-irq | ★★★★★ | T4 | TODO | | FAIL |

**原始基线快照**：`b0658c9` / run 98252605787 / 106P-14F / grade B。
**当前进度快照**：v1.18.2 / **108P-12F** / grade B / 已完成 2、剩余 12。
（矩阵 108P/**12**F —— §八 状态表的 13 行中 ② 078 已转 PASS。）
任何一行从 FAIL→PASS，先更新矩阵数字，再更新 backlog，最后更新本表。

**完成度汇总（截至当前进度快照）**：**2 / 14 DONE（14%）**；
**2 项 abandon**（④ 097、⑤ 096，均非精度债，§2.2）、**1 项 defer**（③ 038，§2.2）、
**9 项未开始**。按易→难排序的逐项完成度见 [README §完成度速查](README.md#完成度速查)。

两处口径修正：

1. **⑬ 077 的成本是派生的** —— 已随 §2.2 **两次收窄**，现为
   **仅 L2 MMC3 IRQ 组**（② 078 已 DONE、④ 097 与 ⑤ 096 均 abandon），见 §2.1。
2. **`defer` / `abandon` 是与 DONE / TODO 并列的第三、第四种终态**，
   口径见 §2.2。defer 项仍计入未清零数，abandon 项**不再计入精度债**。
   **T1 因此只剩 2 项可攻关**（050 / 051）。

---

## 九、附录 — 与 v1.8 backlog 的差分

| v1.8 backlog 条目 | 本基线状态 |
|---|---|
| A-055 cpu_reset_regs | **已清零**（PASS） |
| B-093 bntest | **已清零**（PASS） |
| A-038 / 056 / 037 | 仍在 FAIL，分别对应本计划 ③①⑧ |
| B-078 | **已清零**（v1.18.2，矩阵 108P/12F） |
| C-049/050/051/099/107/108 | 仍在 FAIL，对应 ⑨⑥⑦⑩⑪⑫ |
| C-106 vaus | **已 PASS**（本基线） |
| C-081 fds_irq | 仍在 FAIL（0xFE 加载），对应 ⑭ |
| D-043 blargg_suite oracle | 仍由 oracle 收口，不在 L1 |

---

*本文件为 v1.18 残留精度的长期演进主计划。单项展开到实现级时，另开
`docs/plans/` 子页或 `docs/history/surveys/` 调查页，并回链到本文 §3 对应编号。*
