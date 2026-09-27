# 分项计划 T1 · Mapper 边界 与 DMA/总线路径

> **STATUS: OPEN** · **难度** ★★★☆☆ · **预估** 2 周
> **基线**：run 98252605787 · `git_rev=b0658c9` · 106P/14F
> **当前**：v1.18.2 / 矩阵 **108P / 12F** / grade B · **仅 2 项可攻关**（④ 097、⑤ 096 已 abandon）
> **包出口**：矩阵 FAIL **12 → 10**（两项工作；自当前实际起点 12F 起算）
> **主报告**：§三 ⑥⑦ · §2.2

---

## 包目标

两条**彼此无代码耦合**的 CPU 总线副作用路径。适合按人力切分并行。

| 序 | kgmqa | 主题 | 预估 | 状态 |
|---|---|---|---|---|
| 4 | **097** fme7ramtest | FME-7 WRAM 映射 | — | **abandon**（2026-09-28 裁定，非精度债，见下 ④） |
| 5 | **096** fme7acktest | FME-7 IRQ ack | — | **abandon**（2026-09-28 裁定，survey ROM + ack bug 故意不修，见下 ⑤） |
| 6 | **050** dummy_writes | OAM DMA dummy write | 4–7 天 | TODO |
| 7 | **051** exec_space | IO 空间取指 / bus dispatch | 4–7 天 | TODO |

**依赖**：无（050 与 defer 中的 038 同属 dummy 语义，若将来做 038-S0+S1 可先于 050）。

> **T1 的两个 FME-7 用例（④ 097 / ⑤ 096）都是 survey ROM，均不实现 blargg 的
> `$6000` 结果协议** —— 无论 mapper 怎么改都不会转 PASS。详见下 ④ ⑤ 两段与主报告 §2.2。
> 因此 **T1 只剩 050 / 051 两项可攻关**，包出口 4/4 → **2/2**。
> **⑬ 077 的派生成本随之两次收窄为「仅 L2 MMC3 IRQ 组」**（主报告 §2.2）。

**切分建议**：线 B（Core/bus）：050 → 051。

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

## ⑤ kgmqa-096-fme7acktest-tepples　**已移出本卡（abandon，代码不动）**

> **2026-09-28 裁定**：096 裁定 abandon，**`src/boards/69.cpp` 一行未改**。
> 完整论证见主报告 **§2.2 ⑸**，此处只留结论，避免日后误以为「查过了没问题」。

**两件必须分开看的事**：

1. **它也是 survey ROM**（同 ④）。矩阵报的 `value=0xA2 diag=[0x0F,0x8E,0x00]`
   与 **PRG ROM bank 3 offset 0-3 逐字节相同**（那 4 个字节本身就是 6502 指令）。
   成因：上游 `fme7.s` 的 `init_fme7` 把 reg 8 写成 3 → `preg[3]&0xC0==0` →
   `setprg8(0x6000, 3)` → `$6000` 是 PRG ROM。**按硬件这是对的**（reg 8 的
   bit7=+CE、bit6=RAM/ROM select，`$03` 两位皆 0 → 选 PRG ROM）。
   ROM 对 `$6000` 的绝对寻址指令数 = **0**，结果只写零页并画到屏幕上。
   → **改 mapper 也不会转 PASS。**

2. **但确实存在一个真实 ack bug，本轮明确选择不修。** 硬件（PowerPak / Everdrive
   两个独立实现）与 Nintendulator 三方一致：**bit 0 == 0 才 ack**。
   现状 `case 0xD: IRQa = V; X6502_IRQEnd(FCEU_IQEXT); break;` 无条件 ack →
   6 个用例里 **`$0D=$01` 与 `$0D=$81` 两项错**（应为 No ack）。

   > ⚠️ nesdev wiki 现写着「All writes to this register acknowledge an active IRQ」，
   > 那是照 FCEUX 行为改的，与硬件实测相反。**以硬件为准。**

   **不修的三个理由**：
   - 该 ROM 报不了 pass/fail → 改完**验证不了**（038 纪律：无法验证的改动不合项目纪律）；
   - 要验证需**新增 ctest 用例**（`AWrite[]` 驱动 `$8000`/`$A000`、
     读 `X.IRQlow & FCEU_IQEXT` 断言 ack 表），属独立工作项；
   - `IRQa = V` 用整字节当使能位，按规范应是 bit 0 = IRQ enable、bit 7 = counter enable，
     但 **bit 7 语义本身有争议**（wiki 说 0 = 停计数器，但与 PowerPak 实测自相矛盾），
     改它会动到 Batman: Return of the Joker / Gimmick! 的真实 IRQ 时序。

   **若将来要修，最小改动 1 行**（只门控 ack，不碰使能位）：
   ```c
   case 0xD: IRQa = V; if (!(V & 1)) X6502_IRQEnd(FCEU_IQEXT); break;
   ```

**留档结论**：ack bug 已知、已定位、有三方硬件证据、一行可修，
**但本轮明确不修**。它影响真实 FME-7 游戏的 IRQ 应答行为，
**不体现在任何 F11QA 指标上**（096 修不修都是 FAIL）。将来碰 mapper 69 请先读主报告 §2.2 ⑸。

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

- [ ] **两项**矩阵均 PASS（050 / 051）　·　**2/2**
      （原 4/4 口径作废：④ 097 与 ⑤ 096 于 2026-09-28 双双裁定 abandon，见 §2.2）
- [ ] L1 FAIL 降至 **10**（= 038 defer + 037 + T3 四项 + T4 两项；自当前 12F 起算）
- [ ] `pass_to_fail = 0`（尤其 MMC3 / dma_sync 哨兵）
- [ ] 内部逻辑检测全绿
- [ ] **β 收口时重估 ⑬ 077 剩余子 ROM 数**（成本已两次收窄为**仅 L2 MMC3 IRQ 组**）

## 复验命令

同 T0（ctest + blargg 177 + f11qa-runner 矩阵）。
