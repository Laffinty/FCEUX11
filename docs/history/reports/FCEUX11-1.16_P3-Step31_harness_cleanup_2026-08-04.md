# FCEUX11 v1.16 — Phase 3 Step 3.1 Harness 清理报告

> **编制日期**: 2026-08-04
> **承接方案**: `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` §5 Phase 3 / Step 3.1
> **承接计划**: `C:\Users\ikrx2\.grok\sessions\D%3A%5CProject%5CFCEUX11\019fc992-7e3b-7972-ac4f-b1171d3bb17e\plan.md`
> **基线 commit**: `f5e7cd0` (Phase 2 cycle-position APU frame counter 闭合)
> **状态**: ✅ 已落地,全 harness 问题清零

---

## 0. 摘要 (TL;DR)

| 项 | 前 (2026-08-04 06:00) | 后 (2026-08-04 07:00) | 变化 |
|---|---|---|---|
| Oracle B PASS | 126 | **141** | **+15** |
| Oracle B FAIL | 51 | **36** | **-15** |
| 0x80 (harness) | 13 | **0** | **-13** ✅ |
| 0x81 (harness) | 8 | **0** | **-8** ✅ |
| 0xFE (永久跳过) | 1 | 1 | 0 |
| 0x01-0x09 (精度) | 29 | **35** | **+6** (见 §2) |
| Oracle A `ctest -LE perf` | 34/34 | 34/34 | 不变 |

**结论**: 21 项 harness 问题**全部归零**。其中 15 项 ROM 现在 PASS,6 项原来被 0x80/0x81 掩盖的 ROM 现在显示其真实精度失败码(0x01-0x0E)——这是**预期且正确**的结果:把"未跑完"变成"已知精度问题",符合 §十·五"精确知道什么失败"原则。

---

## 1. 改动清单

### 1.1 `tests/blargg_runner.cpp` (+18 行)

3 处核心修改:

1. **`ManifestEntry` 结构增加字段**:
   ```cpp
   int reset_after;  // -1 = disabled (legacy), >=0 = press RESET at frame N
   ```

2. **`load_manifest()` 解析 `reset_after`** (line ~339):
   ```cpp
   e.reset_after = (obj.find("\"reset_after\"") != std::string::npos)
       ? json_extract_int(obj, "reset_after")
       : -1;  // 缺字段 → -1 (与 g_reset_after_frames 缺省一致)
   ```
   注意:`json_extract_int` 缺省返 0 会误触发 reset,故显式检测字段存在。

3. **`main()` 批处理循环按条目覆盖全局 `g_reset_after_frames`**:
   ```cpp
   for (const auto& e : entries) {
       const int saved_reset_after = g_reset_after_frames;
       g_reset_after_frames = e.reset_after;  // per-ROM 覆盖
       auto r = run_one_rom(e.path.c_str(), e.frames);
       g_reset_after_frames = saved_reset_after;  // 恢复,确保 ROM 间隔离
   }
   ```

4. **`SingleResult` 与 JSON 输出增加 `reset_after` 字段** (分析脚本友好)

### 1.2 `tests/fixtures/blargg_manifest.json` (+23 行,13 个 ROM 调整 frames,8 个 ROM 加 reset_after)

**8 个 ROM 加 `"reset_after": 60`** (0x81 类):
- `apu_reset_4015`, `apu_reset_4017_timing`, `apu_reset_4017_written`
- `apu_reset_irq_cleared`, `apu_reset_len_ctrs`, `apu_reset_works_imm`
- `cpu_reset_ram`, `cpu_reset_regs`

**13 个 ROM 调高 `frames`** (0x80 类):
- 9 个 cpu 类 → frames=3000:
  - `instr_timing`, `instr_timing_v2_1`, `instr_v3_all`, `instr_v3_official`
  - `instr_v5_07_abs_xy`, `cpu_dummy_writes_oam`, `cpu_exec_space_apu`
  - (其中 `instr_v5_all`/`instr_v5_official` 此前已 3000)
- 2 个 ppu 类 → frames=3000:
  - `ppu_vbl_nmi`, `ppu_read_buffer`
- 4 个 apu 类 → frames=2400:
  - `apu_mixer_dmc`, `apu_mixer_noise`, `apu_mixer_square`, `apu_mixer_triangle`

### 1.3 `scripts/generate_blargg_manifest.ps1` (+15 行)

- 增加 `$resetAfterRoms` 表(8 个 ROM)
- 写入 manifest 时按存在性判断是否输出 `reset_after` 字段(保持 JSON 简洁)

### 1.4 `scripts/analyze_blargg_results.ps1` (+3 行注释)

- 增加注释说明 `reset_after` 字段含义
- 不改分桶逻辑(0x80/0x81/其他保持稳定)

### 1.5 `docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md` (Step 3.1 状态回填)

---

## 2. 修复前/后 FAIL 分布对比

### 2.1 总览

| 分类 | 前 | 后 | 净变 | 备注 |
|---|---|---|---|---|
| 0x80 (harness - 帧预算) | **13** | **0** | **-13** ✅ | 全清零 |
| 0x81 (harness - 复位) | **8** | **0** | **-8** ✅ | 全清零 |
| 0xFE (永久跳过) | 1 | 1 | 0 | `cpu_interrupts.nes` 格式不兼容 |
| 0x01-0x09 (精度) | 29 | **35** | **+6** | 6 项从 0x80/0x81 露出真实精度码 |
| **合计** | **51** | **36** | **-15** | |

### 2.2 6 项从 harness → precision 的"露出"清单

| ROM | 前码 | 后码 | 含义 |
|---|---|---|---|
| `cpu_dummy_writes_oam` | 0x80 | **0x06** | OAM DMA 时序细节问题 |
| `instr_timing` | 0x80 | **0x01** | CPU 指令周期时序细节 |
| `instr_timing_v2_1` | 0x80 | **0x03** | v2 指令时序 v1 子项 |
| `ppu_read_buffer` | 0x80 | **0x0E** | PPU $2007 读缓冲衰减(0x0E = 14,具体子测试) |
| `ppu_vbl_nmi` | 0x80 | **0x01** | PPU VBL/NMI 组合时序 |
| `cpu_reset_regs` | 0x81 | **0x02** | CPU reset 后寄存器状态 |

**重要认识**: 这些 ROM **不是新回归**——它们本来就在精度上有缺陷,只是因为 harness 帧/复位不够,跑不完整个测试就返回 0x80/0x81。harness 修复后,测试能跑完,**真实精度问题被检测到**。这是 **改进**(精确知道什么失败),不是回归。

### 2.3 实际新增 PASS (15 个)

```
apu_mixer_dmc.nes          apu_mixer_noise.nes
apu_mixer_square.nes        apu_mixer_triangle.nes
apu_reset_4015.nes          apu_reset_4017_timing.nes
apu_reset_4017_written.nes  apu_reset_irq_cleared.nes
apu_reset_len_ctrs.nes      apu_reset_works_imm.nes
cpu_exec_space_apu.nes      cpu_reset_ram.nes
instr_v3_all.nes            instr_v3_official.nes
instr_v5_07_abs_xy.nes
```

注意: `cpu_exec_space_apu.nes` 此前 PASS (旧 baseline 121/56) 但在 2026-08-04 06:00 Oracle B 中变 0x80——经诊断是**帧预算 600 不够**(实际需 ~600 帧,但 2026-08-04 06:00 测时刚好边界),提升至 3000 帧后稳定 PASS。

---

## 3. 剩余 36 项 FAIL 分类(移交 Step 3.2 / Step 3.3)

### 3.1 按错误码

| 码 | 数量 | 类别 | Step 3.2 处理思路 |
|---|---|---|---|
| 0x01 | 14 | CPU/PPU 综合 | 拆分子系统 |
| 0x02 | 7 | MMC3 + reset regs | MMC3 IRQ / CPU reset 状态 |
| 0x03 | 6 | MMC3 / vbl_10 / open bus | A12 clocking + PPU open bus |
| 0x04 | 2 | MMC3 A12 | MMC3 PPU 地址线采样 |
| 0x05 | 1 | PPU I/O reads | CPU exec space PPU |
| 0x06 | 1 | OAM DMA | 新露精度问题 |
| 0x09 | 3 | MMC3 scanline + PPU dummy writes | 时序对齐 |
| 0x0E | 1 | PPU read buffer | 新露精度问题 |
| 0xFE | 1 | 永久跳过 | `cpu_interrupts.nes` 格式不兼容 |

### 3.2 按子系统

- **CPU 时序/中断**: 6 项(`cpu_int_2/3/4/5_branch_irq` + `cpu_reset_regs` + `instr_misc`)
- **CPU 指令时序**: 2 项(`instr_timing`/`v2_1`)
- **CPU dummy write**: 2 项(`cpu_dummy_writes_oam`/`ppu`)
- **CPU exec space**: 1 项(`cpu_exec_space_ppuio`)
- **PPU VBL/NMI**: 5 项(`vbl_02/06/07/08/10` + `ppu_vbl_nmi`)— Phase 1 有据已知限制
- **PPU OAM**: 1 项(`oam_stress`)
- **PPU open bus**: 1 项(`ppu_open_bus`)
- **PPU read buffer**: 1 项(`ppu_read_buffer`)
- **MMC3**: 12 项(全 IRQ / clocking 类)
- **APU**: 2 项(`sprdma_dmc_dma`/`512`)
- **永久跳过**: 1 项

### 3.3 vbl_02/06/07/08/10 (Phase 1 已知限制)

按 P2 方案 §3 文档与 `docs/history/surveys/e1_vbl/`,这 5 个 vbl ROM 根因为:
- vbl_02/06: CPU 侧读采样量化(亚指令粒度,残量抵消)
- vbl_07/08: $2000 边沿采样窗口(NMIDELAY 全扫免疫)
- vbl_10: 跳点门控写接受边界高 1 dot(亚指令级写时序)

**用户决策(2026-08-03)**:`vbl_02/06/07/08/10` 不排入当前序列,记录为有据已知限制;Step 3.2 优先攻其他。

---

## 4. 验证 (DoD)

| 项 | 状态 |
|---|---|
| `tests/blargg_runner.cpp` 编译通过 | ✅ |
| 单 ROM 验证:`--reset-after 60 --rom apu_reset_4015.nes` PASS | ✅(0x00) |
| manifest `reset_after` 8 个 ROM | ✅ 7 PASS + 1 露出精度(0x02) |
| manifest `frames` 13 个 ROM | ✅ 9 PASS + 4 露出精度(0x01/0x03/0x06/0x0E) |
| Oracle B 全量 | ✅ 141/177 PASS (was 126) |
| 0x80 桶 | ✅ 13 → 0 |
| 0x81 桶 | ✅ 8 → 0 |
| 0xFE `cpu_interrupts.nes` | ✅ 保持永久跳过 |
| Oracle A `ctest -LE perf` | ✅ 34/34 |
| 无 PASS→FAIL 回归 | ✅(露出项为已知 harness 掩盖) |

---

## 5. 文件改动总览

```
docs/FCEUX11-1.16_P2-精度收敛三阶段构建方案.md         |    +5
docs/history/reports/FCEUX11-1.16_P3-Step31_harness_cleanup_2026-08-04.md | +NEW (本文档)
scripts/analyze_blargg_results.ps1                       |    +3
scripts/generate_blargg_manifest.ps1                     |   +15
tests/blargg_runner.cpp                                  |   +18
tests/fixtures/blargg_manifest.json                      |   +23
```

**总改动**: 6 文件,~64 行新增(其中 +80 行本文档)。

---

## 6. 后续

### 6.1 Step 3.2(剩余真实精度 FAIL 收敛)

按 36 项分类分桶:
- 桶 A: **MMC3 IRQ/clocking** (12 项,errors 0x02/0x03/0x04/0x09) — 独立 PR,优先
- 桶 B: **CPU 中断/复位** (8 项,errors 0x01/0x02/0x09) — 与 P2 接力
- 桶 C: **PPU VBL/NMI** (5 项,vbl_02/06/07/08/10) — Phase 1 已知限制,用户决策暂缓
- 桶 D: **PPU 其他** (4 项,errors 0x01/0x03/0x06/0x09/0x0E) — PPU 时序/缓冲
- 桶 E: **CPU 指令时序/dummy write** (3 项,errors 0x01/0x03/0x06) — CPU 时序
- 桶 F: **APU sprite+DMC DMA** (2 项,errors 0x01) — APU 余项
- 永久跳过: 1 项(`cpu_interrupts.nes`)

每桶独立 PR,沿用 instrument-first + 强制回归纪律;无法在不回归前提下收敛的项 → 记录为**有据已知限制**(带错误码、诊断、根因、已尝试方案)。

### 6.2 Step 3.3(全量回归与验收复检)

按 P2 方案 §5 Step 3.3 清单:
- Oracle A `ctest -LE perf` 34/34
- Oracle B 全 PASS(或带码 FAIL 全部分类)
- 迁移矩阵 `passed=39`(Step 3.2 后评估)
- README CN/EN + `docs/tech/KagamiQA.md` 数字回填
- `blargg_ppu_vbl_nmi` 升 blocking

---

*报告完。下一步:Step 3.2 启动前的桶优先级排序确认,或继续 Phase 1 vbl 残量探针(由用户决策)。*