# F11QA v1.8 精度攻关 backlog（2026-09-26 启动）

> 实测矩阵 **106P / 14F**（v1.8 收口时点；内部逻辑检测 42P/0F）。
> 流水线与 baseline 已稳：`pass_to_fail=0`、内部逻辑检测 全绿。
> **当前进度（2026-09-27，v1.18.2）**：矩阵 **108P / 12F**，grade **B**，
> 已清 **① 056**（`8010a3f`）与 **② 078**（v1.18.2），blargg 147P/30F，advisory 10.0%。

## 0. 本批已收掉的非精度项（对照）

| kgmqa | 症状 | 处置 |
|---|---|---|
| 117 | CI python 编码/路径 | `-X utf8` + 相对路径 |
| 113 | i18n working_dir 误为 tests | CTest 是仓库根 |
| 114 | 伪二进制 fceux11_menu_slot_check | 改 python + check_menu_slots.py |
| 030 | blargg_runner 无参 usage 失败 | 加 nestest smoke 参数 |
| 043 | 全量 batch 合法 exit1 被判 FAIL | 新增 `run_blargg_suite_oracle.py`（known_fail 覆盖则 0） |
| 046 | lua memory 未加载 ROM | 加 `--rom nestest.nes` |

## 1. 精度 backlog（按可攻性）

### A. CPU 复位 / 指令（4）— 有 known_fail 根因

| id | ROM | code | 根因（known_fail / precision.md） |
|---|---|---|---|
| 055 | cpu_reset_registers | 0x81 | 复位后 A/X/Y/P/S 初值；诊断串 `A  X  Y  P  S` |
| 038 | instr_misc | 0x01 / 0x03 | **已重估 ★★★★☆ / 1–2 周 / 风险中**（原 ★★☆☆☆ / 3–5 天）：阻塞点为 open bus 恒定（`set_db` 零调用）；先做 S0+S1（1–3 天）拿判别结论 |
| ~~056~~ | instr_timing | ~~0x80~~ | **已清零**（v1.18.1）：`CycTable[0xE2]` 3→2；`0xBB` 由 `RMW_ABY` 改 `LD_ABY`（含跨页 +1，去多余写回） |
| 037 | cpu_int_2_nmi_brk | 0x01 | 中断仅在指令边界轮询（x6502.cpp:515-579） |

**优先 055**：复位寄存器是小表，可对照 nestest / blargg 期望逐项改 PowerNES/ResetNES 初值，回归面窄。

### B. Mapper 边界（4）

| id | 套件 | 现象 |
|---|---|---|
| ~~078~~ | lidnariq serom | **已清零**（v1.18.2）：submapper 5 + PRG-RAM=0 → `$6000` 未映射 → 矩阵 108P/12F |
| 093 | tepples bntest-aorom | BxROM/BNROM 边界，$6000=0x27 |
| 077 | holy_mapperel | 13 mapper 聚合，47 ROM 循环 |
| 096/097 | fme7 | FME-7 IRQ ack / WRAM |

**优先 093**：单 mapper 边界，比聚合套件好定位。

### C. 第三方边缘 / 输入 / 音频（7）

049/050/051 bisqwit、081 FDS IRQ、099 240pee、106 vaus、107 mset、108 mict。
多为「怪兽级」时序/输入扫描（bisqwit ppu_read_buffer 为 $2007 读缓冲著名难点）。

### D. 不建议本阶段攻（2）

- 043 已由 oracle 收口（见上）
- 其余聚合项等单点突破后自然收敛

## 2. 攻关纪律

- **Instrument-first**（precision.md §4）：改时序前先上 env-gated 探针
- 每项必须带「探针数据 + 根因」才能标 known_limit，禁止无据 FAIL
- 单点提交 + 硬件一致性检测 全量对账（**147/30 不得回退**；原记 144/33 为 v1.8 早期数字）
- golden 金标仅在有意 PPU 变更时重生成

## 3. 阶段出口

R4 grade B 需要 blocking FAIL 明显下降。建议路径：
1. **Phase A-055** 复位寄存器 → **已清零**（145/32）
2. **Phase B-093** BNROM/AxROM → **已清零**
3. **Phase T0-056** 指令周期表 E2/BB → **已清零**（147/30，见长期演进计划 ①）
4. 再评估 037 中断轮询下沉（工程量大，单独立项）

> 后续 13 项的逐项完成度、易→难排序与两处纪律修正，见
> `docs/plans/long-term-evolution/README.md`（§完成度速查）。
