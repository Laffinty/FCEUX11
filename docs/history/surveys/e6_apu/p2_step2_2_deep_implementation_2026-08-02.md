# E-3 APU P2 Phase 2 Step 2.2 深化 — cycle-position 帧计数器 实现记录 (2026-08-02)

> **目的**：闭合 `p2_step2_2_data` §2.3 记录的**有据已知限制**（hook 量化方差 ±2-3 cyc
> 使 `apu_single_5/6` 参数级不可收敛）。按方案 §4 Step 2.2 深化路径落地
> **cycle-accurate quarter crossing**，用已验证参考实现（Mesen2/RustyNES）替换
> 均匀 countdown 模型。
>
> **依据**：
> - `blargg_apu_2005.07.30/readme.txt`（Mode 0/1 时序表、clock jitter、IRQ ≥29833、power/reset 9-12 延迟）
> - `04.clock_jitter.asm`（sub-test 读点：29830 CLEAR / 29831 get_jitter / 29832 SET）
> - `05.len_timing_mode0.asm`（第一次 length 可见于 14916、第二次于 29832、第三次于 44746）
> - `07.irq_flag_timing` / `tests.txt`（flag 于 29831 起连续三次置位）
> - `apu_reset/` 测试源码（power/reset 相位、4017_written 需两次复位）
> - RustyNES `frame_counter.rs`（NTSC 4-step 位置 7457/14913/22371/29828/29829/29830，
>   quarter@7457/14913/22371/29829，half@14913/29829，IRQ@29828-30；写后 3/4 周期重置延迟；
>   通过全部 blargg apu 测试）
>
> **分支**：`wip_1.16`（工作树直接改，未开新分支）
> **作者**：独立执行（2026-08-02）

---

## 1. 根因（为什么旧模型参数级不可收敛）

旧模型每 quarter 固定 7457.5 周期（`fhcnt` 48ths countdown）。但 blargg 测试要求：

| 事件 | 测试读点 | 可见边界 |
|---|---|---|
| 第一次 length clock | 14915 未触发 / 14916 已触发 | (14915, 14916] |
| flag 首次可见 | 29830 未置 / 29831 置（get_jitter） | (29830, 29831] |
| 第二次 length clock | 29831 未触发 / 29832 已触发 | (29831, 29832] |
| 第三次 length clock | 44745 未触发 / 44746 已触发 | (44745, 44746] |

length#1 与 length#2 的间距必须是 **14916** 周期（29832−14916），而均匀模型固定
2×7457.5 = **14915**——任何单一 offset 只能命中其一（实测 offset 0-4 全部证伪）。
硬件位置非均匀（7457/14913/22371/29828-30，间隔 7456/7458/7457/1/1），源于
写后 3/4 周期重置 + APU 相位对齐。

## 2. 实现（src/sound.cpp）

### 2.1 帧计数器改为 cycle-position 表驱动

- `fhcnt` 语义从"48ths countdown"改为"**距序列起点的 CPU 周期位置**"（0..29830），
  FHCN savestate chunk 名称/大小/序不变（值语义变化，golden 按预期流程重生）。
- `FrameCounterTick()`：每周期推进，按表触发事件：
  - NTSC 4-step：7457 q / 14913 q+h / 22371 q / 29828 IRQ / 29829 IRQ+q+h / 29830 IRQ·wrap
  - NTSC 5-step：7457 q / 14913 q+h / 22371 q / 37281 q+h / 37282 wrap（无 IRQ）
  - PAL 4-step：8313 / 16627 / 24939 / 33252 / 33253 / 33254
  - PAL 5-step：8313 / 16627 / 24939 / 41565 / 41566
- `FrameSoundEvent(q,h)`：quarter=FrameSoundStuff(1)（envelope+linear），
  half=FrameSoundStuff(0)（length+sweep+envelope+linear）；`fcnt` 每步自增（FCNT chunk 兼容）。
- **IRQ flag 与 IRQ line 分离**：`FrameIRQSet` 无条件置 `SIRQStat|0x40`（$4015 可见），
  仅非 inhibit 时 `X6502_IRQBegin`；29830 处 inhibit 分支清 flag+line（Mesen2/RustyNES 语义）。

### 2.2 $4017 写 → 3/4 周期重置延迟（clock jitter）

- `Write_IRQFM` 不再立即 `fcnt=1; fhcnt=fhinc+offset*48`，改为调度
  `fc_reset_in = ((abs_ts & 1) == 0) ? 3 : 4`（偶数 CPU 周期=APU 对齐→3，奇数→4）。
- hook 中逐周期递减 `fc_reset_in`，到期后 `fhcnt=0; fcnt=0`；5-step 写（bit7）在
  重置到期时立即 quarter+half clock（`apu_01` sub-test 4 语义）。

### 2.3 绝对周期奇偶（本次另一关键修复）

`g_cpu.timestamp_ref()` **每帧被归零**（`fceu.cpp` FCEU_Emulate），NTSC 帧长 29780.67
周期 → 帧内奇偶跨帧翻转。改用单调绝对周期
`g_cpu.timestamp_base() + g_cpu.timestamp_ref()` 判定写相位。修复前 `sync_apu` 的
`bne +/-1` 对齐分支在跨帧后选错，使 05.len_timing M4 的写入落在奇相位
（length#2 至 29833，"too late"）。

### 2.4 power/reset 相位

- `FCEUSND_Reset`：`fhcnt=4`（等效"$00 写 + 4 周期"后开始；实测 count=5+D，
  D=4 → count=9 ∈ [6,12] 窗口，4017_timing PASS；D∈[1,7] 均可行，取中值）。
- power：`IRQFrameMode=0x0`；reset：保留最后写入值（Step 2.1 逻辑不变）。

### 2.5 runner 支持多次软复位（tests/blargg_runner.cpp）

`apu_reset_4017_written` 需要**两次**软复位（power→reset→reset）。`--reset-after` 后
每 6 帧轮询 $6000，按 blargg 协议（0x81=等待 RESET，延迟≥100ms）自动再按；
加 20 帧冷却避免把复位后测量窗口内的陈旧 0x81 误判为再次等待。

## 3. 验证

### 3.1 APU bucket-C（原 7 个 FAIL sub-test 全转 PASS）

| ROM | 旧（Step 2.2） | 新 |
|---|---|---|
| `apu_single_3_irq_flag` | PASS | PASS |
| `apu_single_4_jitter` | PASS | PASS |
| **`apu_single_5_len_timing`** | **FAIL 0x04** | **PASS** |
| **`apu_single_6_irq_timing`** | **FAIL 0x04** | **PASS** |
| **`apu_reset_4017_timing`** | **FAIL 0x03** | **PASS** |
| **`apu_reset_4017_written`** | **FAIL 0x02** | **PASS** |
| **`apu_test`（组合套件）** | **FAIL 0x01** | **PASS** |

### 3.2 全量回归

- **APU 52 ROM**：全部 PASS（mixer 需 `--frames 2400`；reset 需 `--reset-after`；
  sprdma_dmc_dma×2 为既有 DMA 时序失败，旧模型同样 FAIL，与本次无关）。
- **Oracle A**（`ctest -LE perf`，全量重建后）：32/33；唯一失败
  `config_store_test` 仅链接 Qt6::Core、不链接模拟器核心，为沙箱 `.config` 权限环境问题。
- **Oracle B**（177 ROM，mixer 2400f + reset-after）：**PASS 135**（基线 121，+14），
  FAIL 42 全为既有 CPU/mapper/PPU/harness/E-1 项，**无任何 apu_* 失败**。
- **golden savestate**：重生 8 个 .fc0 + golden_index.json + savestate 12 哈希；
  `--compare-layout` 确认 diff 仅 **SFSND/FHCN** 字段（fhcnt 值语义），无 chunk 结构漂移。

## 4. 结论

1. **Phase 2 验收目标（7 个 bucket-C sub-test 全转 PASS）达成**，E-3 APU 帧计数器收敛闭合。
2. 方案 §1.4 真因四件套 ④（hook 量化方差）由**有据已知限制**转**已修复**；
   3/4 周期重置延迟与绝对周期奇偶（① 已落地于 Step 2.1/2.2，②③ 语义被新模型吸收）。
3. 遗留：`sprdma_dmc_dma*`（DMA×DMC 时序，非帧计数器）、E-1 六 ROM（Phase 1 专项）、
   Phase 3 harness 项（0x80/0x81 分桶）。

## 5. 自检清单

| # | 检查 | 状态 |
|---|---|---|
| 1 | instrument-first：先采集旧模型单 ROM trace + blargg 源码定义 | ✅ |
| 2 | 参考实现对照（RustyNES/Mesen2 frame_counter.rs） | ✅ |
| 3 | 关键 ROM 逐一闭合（single_4/5/6、reset×2、apu_test） | ✅ |
| 4 | APU 52 ROM 全量零 PASS→FAIL | ✅ |
| 5 | Oracle A 32/33（config_store 环境问题，与核心无关） | ✅ |
| 6 | Oracle B 135 PASS，无 apu_* 失败 | ✅ |
| 7 | golden 重生 + compare-layout 仅 FHCN 变化 | ✅ |
| 8 | 未开新分支（wip_1.16）、savestate chunk 结构未变 | ✅ |

---

*实现记录完。Step 2.2 深化落地，E-3 收敛；下一步按方案回到 Phase 1 Step 1.3（CPU 侧采样模型）合并评估。*
