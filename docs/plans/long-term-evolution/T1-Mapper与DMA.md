# 分项计划 T1 · Mapper 边界 与 DMA/总线路径

> **STATUS: OPEN** · **难度** ★★★☆☆ · **预估** 2–4 周
> **基线**：run 98252605787 · `git_rev=b0658c9` · 106P/14F
> **当前**：v1.18.2 / 矩阵 **108P / 12F** / grade B · **可攻关 3 项**（④ 097 已 abandon）
> **包出口**：矩阵 FAIL **12 → 9**（三项工作；自当前实际起点 12F 起算）
> **主报告**：§三 ⑤⑥⑦ · §2.2

---

## 包目标

三条**彼此无代码耦合**的单点缺口：一个 FME-7 板级行为 + 两条 CPU 总线副作用路径。适合按人力切分并行。

| 序 | kgmqa | 主题 | 预估 | 状态 |
|---|---|---|---|---|
| 4 | **097** fme7ramtest | FME-7 WRAM 映射 | — | **abandon**（2026-09-28 裁定，非精度债，见下 ④） |
| 5 | **096** fme7acktest | FME-7 IRQ ack | 3–7 天 | TODO |
| 6 | **050** dummy_writes | OAM DMA dummy write | 4–7 天 | TODO |
| 7 | **051** exec_space | IO 空间取指 / bus dispatch | 4–7 天 | TODO |

**依赖**：无（T0 非硬依赖；050 与 defer 中的 038 同属 dummy 语义，若将来做 038-S0+S1 可先于 050）。

**本包是 ⑬ 077 的主要成本来源**：⑤ 096 直接吃掉 077 的 FME-7 组，
② 078 吃掉 MMC1 组。**β 收口时必须回头重估 077 的剩余子 ROM 数**（主报告 §2.1）——
不要等到 ε 阶段才看 077，47/47 是合取判定，滞后于任何单点进展。

> **2026-09-28 更新**：④ 097 裁定 abandon 后，077 的派生成本由
> 「② 078 + ④ 097 + ⑤ 096 + L2 MMC3 IRQ 组」收窄为
> **「⑤ 096 剩余 + L2 MMC3 IRQ 组」**（主报告 §2.2）。

**切分建议**：
- 线 A（Boards）：096
- 线 B（Core/bus）：050 → 051

---

## ④ kgmqa-097-fme7ramtest-tepples

| 字段 | 值 |
|---|---|
| ROM | `tepples/fme7/fme7ramtest.nes` |
| 错误码 | `0x01` |
| known_limit | FME-7 WRAM mapping edge　**← 已过时，见下** |

> **STATUS: 根因改判（2026-09-27）。本项不是 mapper 缺陷，也不可修。**
> 完整证据在主报告 [`FCEUX11-v1.18_F11QA残留精度长期演进计划.md`](FCEUX11-v1.18_F11QA残留精度长期演进计划.md) §三④
> 「④ 调查记录」—— 按证据归属规则，本卡只留指针。
>
> 要点三条：
>
> 1. `fme7ramtest` 是 **survey ROM**，不使用 blargg 的 `$6000` 结果协议。
>    静态扫描：PRG 里对 `$6000` 的绝对寻址指令 **0 处**，对 `$6900` **恰 2 处**
>    （`STA $6900` / `LDA $6900`），与上游 `main.s` 的 `check_bank_numbers` 逐字吻合。
>    `$6000` 里的 `0x01` 是 `check_for_wram` 写的 9 字节 RAM 图样首字节。
> 2. **FME-7 WRAM 映射本身是对的**：`FCEUX11_FME7_PROBE=1` 实测 bank tag
>    `C0 C1 C2 C3 C0 C1 C2 C3`（两个 8 字节组），与上游 README
>    「With 62256 (32Kx8)」期望输出逐字节相同；`WRAMSIZE=32768` 是 ROM 头部
>    有意声明（`64 << header[10]`，FCEUX 遗留布局 `ram_size` 在 offset 10）。
> 3. harness 判 PASS 的唯一条件是 `probe_addr` 处读到 `0x00`
>    （`blargg.rs:316`），`probe_addr` 恒为 `0x6000` 且 `tests.json` 无 per-case 覆盖。
>    **换 `$6900` 得到 `0xC0`，也不是 `0x00`** —— 没有可行地址。
>
> 顺带发现 `reg8 = 40-7F` 档 `Sync()`（映 PRG ROM）与读 handler（open bus）不自洽，
> **本 ROM 测不出，故不改**（同 038 纪律：无法验证的核心时序改动不合项目纪律）。

### 步骤（原计划，已作废）

- [x] 读 FME-7 WRAM 映射写逻辑
- [x] 静态扫描 ROM + 上游源码比对
- [x] env-gated 探针实测 reg8 轨迹与 bank tag
- [x] ~~补全边界~~ —— 无缺陷可补
- [x] **处置：2026-09-28 人工授权 A（有据 known_limit）**，已改 `tests/tests.json`
      的 `known_limit` + `provenance`。`failure_means` 仍 `advisory`，用例数仍 120

### 验收

kgmqa-097 PASS。
→ **不可达**（见上方证据）。已于 2026-09-28 以「有据 known_limit」处置，
   矩阵仍 108P/12F、grade 仍 B —— **`known_limit` 字段不参与任何机械判定**，
   本次改动的价值是语义，不是数字。

### 风险：低。

---

## ⑤ kgmqa-096-fme7acktest-tepples

| 字段 | 值 |
|---|---|
| ROM | `tepples/fme7/fme7acktest.nes` |
| 错误码 | `0xA2`（diag `[0F,8E,00]`） |
| known_limit | FME-7 IRQ acknowledge edge |

### 根因

IRQ **应答（ack）** 写边沿 → 线撤销时机未精确建模。

### 步骤

- [ ] 探针记录 assert / ack / deassert 相对写时钟
- [ ] 修正 ack 边沿
- [ ] 确认 MMC3 IRQ 路径无 `pass_to_fail`（共享 CPU IRQ 线）

### 验收

kgmqa-096 PASS。  
### 风险：中。IRQ 共享路径；回退哨兵 = mmc3irqtest / mmc3 L2。

---

## ⑥ kgmqa-050-cpu-dummy-writes-bisqwit

| 字段 | 值 |
|---|---|
| ROM | `bisqwit/cpu_dummy_writes_oam.nes` |
| 错误码 | `0x06` |
| known_limit | dummy writes to OAM DMA; RMW OK, DMA path not |

### 根因

OAM DMA（$4014）假写路径未模拟。**RMW 宏已正确**；`cpu_dummy_writes_ppu` **已 PASS**。

### 步骤

- [ ] 对比 `_ppu`（PASS）vs `_oam`（FAIL）总线轨迹
- [ ] OAM DMA 状态机补 dummy write
- [ ] 观察 L2 `sprdma_dmc_dma*` 连带（允许仍 FAIL，不得回退）

### 验收

kgmqa-050 PASS；`dma_sync_test_v2`（kgmqa-110）保持 PASS。  
### 风险：中。DMA×DMC 交互；110 是回归哨兵。

---

## ⑦ kgmqa-051-cpu-exec-space-bisqwit

| 字段 | 值 |
|---|---|
| ROM | `bisqwit/test_cpu_exec_space_apu.nes` |
| known_limit | execute-from-IO-space; bus dispatch gap |

### 根因

bus dispatch 未区分 **取指 vs 数据访问**；从 $4000–$401F 取指的映射/副作用不完整。
（blargg 侧 `cpu_exec_space_apu` 已 PASS，bisqwit 变体更严。）

### 步骤

- [ ] 探针区分 opcode fetch 与 data r/w
- [ ] 补全 IO 空间取指读映射
- [ ] 观察 L2 `cpu_exec_space_ppuio`（`0x05`）

### 验收

kgmqa-051 PASS；`fceux11_bench_bus_dispatch` 无明显回退。  
### 风险：中。bus dispatch 为热路径。

---

## 包出口检查

- [ ] **三项**矩阵均 PASS（096 / 050 / 051）　·　**3/3**
      （原 4/4 口径作废：④ 097 于 2026-09-28 裁定 abandon，见 §2.2）
- [ ] L1 FAIL 降至 **9**（= 038 defer + 037 + T3 四项 + T4 两项；自当前 12F 起算）
- [ ] `pass_to_fail = 0`（尤其 MMC3 / dma_sync 哨兵）
- [ ] 内部逻辑检测全绿
- [ ] **β 收口时重估 ⑬ 077 剩余子 ROM 数**（成本已收窄为 ⑤ 096 + L2 MMC3 IRQ 组）

## 复验命令

同 T0（ctest + blargg 177 + f11qa-runner 矩阵）。
