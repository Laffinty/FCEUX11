# 分项计划 T1 · Mapper 边界 与 DMA/总线路径

> **STATUS: OPEN** · **难度** ★★☆☆☆–★★★☆☆ · **预估** 2–4 周
> **基线**：run 98252605787 · `git_rev=b0658c9` · 106P/14F
> **当前**：v1.18.2 / 矩阵 **108P / 12F** / grade B / **① 056、② 078 已 DONE**，余 12 项
> **包出口**：矩阵 FAIL **13 → 9**（自当前实际起点 13F 起算；α 全额收口则为 11 → 7）
> **主报告**：§三 ④⑤⑥⑦

---

## 包目标

四条**彼此无代码耦合**的单点缺口：两个 FME-7 板级行为 + 两条 CPU 总线副作用路径。适合按人力切分并行。

| 序 | kgmqa | 主题 | 预估 | 状态 |
|---|---|---|---|---|
| 4 | **097** fme7ramtest | FME-7 WRAM 映射 | 2–4 天 | TODO |
| 5 | **096** fme7acktest | FME-7 IRQ ack | 3–7 天 | TODO |
| 6 | **050** dummy_writes | OAM DMA dummy write | 4–7 天 | TODO |
| 7 | **051** exec_space | IO 空间取指 / bus dispatch | 4–7 天 | TODO |

**依赖**：无（T0 非硬依赖，但 050 与 T0-038 同属 dummy 语义，做完 038 再做 050 更顺）。

**本包同时是 ⑬ 077 的主要成本来源**：④ 097 / ⑤ 096 直接吃掉 077 的 FME-7 组，
② 078 吃掉 MMC1 组。**β 收口时必须回头重估 077 的剩余子 ROM 数**（主报告 §2.1）——
不要等到 ε 阶段才看 077，47/47 是合取判定，滞后于任何单点进展。

**切分建议**：
- 线 A（Boards）：097 → 096
- 线 B（Core/bus）：050 → 051

---

## ④ kgmqa-097-fme7ramtest-tepples

| 字段 | 值 |
|---|---|
| ROM | `tepples/fme7/fme7ramtest.nes` |
| 错误码 | `0x01` |
| known_limit | FME-7 WRAM mapping edge |

### 根因

FME-7（Sunsoft 5B）**WRAM 映射寄存器**边界未完整建模（bank 号 / 使能 / $6000 窗口）。

### 步骤

- [ ] 读 FME-7 WRAM 映射写逻辑
- [ ] 补全边界（含 bank 使能关闭时的总线行为）
- [ ] 单 ROM 验证

### 验收

kgmqa-097 PASS。  
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

- [ ] 四项矩阵均 PASS
- [ ] L1 FAIL 降至 **9**（= T0 余 2 + 037 + T3 四项 + T4 两项；自当前 13F 起算）
- [ ] `pass_to_fail = 0`（尤其 MMC3 / dma_sync 哨兵）
- [ ] 内部逻辑检测全绿
- [ ] 097/096 可同分支但 **分 commit**

## 复验命令

同 T0（ctest + blargg 177 + f11qa-runner 矩阵）。
